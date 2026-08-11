use std::error::Error;
use std::sync::Arc;

use hydracache_client_transport_axum::AxumClientSurface;
use hydracache_server::{
    serve_hc2_listener, serve_redis_listener, AdminHttpSurface, Hc2ClientPlaneService,
    Hc2ListenerTls, ServerConfig, ServerRuntime,
};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::from_env()?;
    let admin_enabled = config.admin_api.enabled;
    let admin_addr = config.admin_api.listen_addr;
    let client_enabled = config.client_api.enabled;
    let client_addr = config.listen_addr;
    let redis_enabled = config.redis_api.enabled;
    let redis_addr = config.redis_api.listen_addr;
    let hc2_enabled = config.hc2_client_plane.enabled;
    let hc2_addr = config.hc2_client_plane.listen_addr;
    let hc2_cluster_id = config.hc2_client_plane.cluster_id.clone();
    let drain_timeout = config.drain_timeout();
    let hc2_tls = hc2_enabled
        .then(|| Hc2ListenerTls::from_server_config(&config.tls))
        .transpose()?;

    // Bind every enabled public socket before any surface begins accepting.
    // A conflict therefore fails the process without a partially started daemon.
    let client_listener = if client_enabled {
        Some(TcpListener::bind(client_addr).await?)
    } else {
        None
    };
    let redis_listener = if redis_enabled {
        Some(TcpListener::bind(redis_addr).await?)
    } else {
        None
    };
    let hc2_listener = if hc2_enabled {
        Some(TcpListener::bind(hc2_addr).await?)
    } else {
        None
    };
    let admin_listener = if admin_enabled {
        Some(TcpListener::bind(admin_addr).await?)
    } else {
        None
    };

    let runtime = ServerRuntime::new(config)?.start();
    let mut admin_surface = AdminHttpSurface::new(runtime);
    let shared_runtime = admin_surface.runtime();
    let dispatch_state = shared_runtime
        .lock()
        .expect("server runtime mutex")
        .client_dispatch_state();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (failure_tx, mut failure_rx) = mpsc::unbounded_channel::<String>();
    let mut listener_tasks = Vec::new();
    let mut hc2_observer = None;

    if let Some(listener) = client_listener {
        let state = dispatch_state
            .as_ref()
            .map(Arc::clone)
            .expect("client API has shared dispatch state");
        let routes = AxumClientSurface::from_state(state).routes();
        let mut shutdown = shutdown_rx.clone();
        let failure = failure_tx.clone();
        listener_tasks.push(tokio::spawn(async move {
            let result = axum::serve(listener, routes)
                .with_graceful_shutdown(async move {
                    while shutdown.changed().await.is_ok() {
                        if *shutdown.borrow() {
                            break;
                        }
                    }
                })
                .await;
            if let Err(error) = result {
                let _ = failure.send(format!("HC/1 client listener failed: {error}"));
            }
        }));
    }

    if let Some(listener) = hc2_listener {
        let state = dispatch_state
            .as_ref()
            .map(Arc::clone)
            .expect("HC/2 has shared dispatch state");
        let service = Hc2ClientPlaneService::new(state, hc2_cluster_id);
        hc2_observer = Some(service.clone());
        admin_surface = admin_surface.with_hc2_metrics(service.clone());
        let shutdown = shutdown_rx.clone();
        let failure = failure_tx.clone();
        let tls = hc2_tls.expect("enabled HC/2 has eagerly validated TLS");
        listener_tasks.push(tokio::spawn(async move {
            if let Err(error) = serve_hc2_listener(listener, service, tls, shutdown).await {
                let _ = failure.send(error.to_string());
            }
        }));
    }

    let redis_server = {
        shared_runtime
            .lock()
            .expect("server runtime mutex")
            .redis_resp_server()?
    };
    if let (Some(redis_server), Some(listener)) = (redis_server, redis_listener) {
        let redis_tls = {
            let runtime = shared_runtime.lock().expect("server runtime mutex");
            runtime.redis_tls_acceptor()?
        };
        let runtime = Arc::clone(&shared_runtime);
        let redis_shutdown_rx = shutdown_rx.clone();
        let failure = failure_tx.clone();
        listener_tasks.push(tokio::spawn(async move {
            if let Err(error) = serve_redis_listener(
                listener,
                Arc::new(redis_server),
                runtime,
                redis_tls,
                redis_shutdown_rx,
            )
            .await
            {
                let _ = failure.send(error.to_string());
            }
        }));
    }
    println!(
        "{}",
        serde_json_like_health(
            shared_runtime
                .lock()
                .expect("server runtime mutex")
                .health()
                .status
        )
    );
    let admin = async move {
        if let Some(listener) = admin_listener {
            axum::serve(listener, admin_surface.routes()).await?;
        } else {
            std::future::pending::<Result<(), std::io::Error>>().await?;
        }
        Ok::<(), std::io::Error>(())
    };
    let drain_runtime = Arc::clone(&shared_runtime);
    let drain_requested = async move {
        loop {
            if !drain_runtime
                .lock()
                .expect("server runtime mutex")
                .can_serve()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    };
    let stop_error = tokio::select! {
        result = admin => result.err().map(|error| format!("admin listener failed: {error}")),
        result = tokio::signal::ctrl_c() => result.err().map(|error| format!("signal listener failed: {error}")),
        () = drain_requested => None,
        failure = failure_rx.recv() => failure,
    };
    let _ = shutdown_tx.send(true);
    let listener_drain = async {
        for task in &mut listener_tasks {
            let _ = task.await;
        }
    };
    let listener_drain_timed_out = tokio::time::timeout(drain_timeout, listener_drain)
        .await
        .is_err();
    if listener_drain_timed_out {
        for task in &listener_tasks {
            task.abort();
        }
        for task in &mut listener_tasks {
            let _ = task.await;
        }
    }
    let hc2_accounting = hc2_observer.map(|service| service.accounting());
    shared_runtime
        .lock()
        .expect("server runtime mutex")
        .graceful_shutdown();
    if listener_drain_timed_out {
        return Err("client listener drain exceeded configured deadline".into());
    }
    if hc2_accounting.is_some_and(|snapshot| {
        snapshot.active_connections != 0
            || snapshot.active_subscriptions != 0
            || snapshot.active_sessions != 0
            || snapshot.pending_invocations != 0
    }) {
        return Err("HC/2 listener retained resources after drain".into());
    }
    if let Some(error) = stop_error {
        return Err(error.into());
    }
    Ok(())
}

fn serde_json_like_health(status: &str) -> String {
    format!(r#"{{"status":"{status}"}}"#)
}
