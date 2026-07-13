use std::error::Error;
use std::sync::Arc;

use hydracache_server::{serve_redis_listener, AdminHttpSurface, ServerConfig, ServerRuntime};
use tokio::net::TcpListener;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::from_env()?;
    let admin_enabled = config.admin_api.enabled;
    let admin_addr = config.admin_api.listen_addr;
    let runtime = ServerRuntime::new(config)?.start();
    let admin_surface = AdminHttpSurface::new(runtime);
    let shared_runtime = admin_surface.runtime();
    let redis_server = {
        shared_runtime
            .lock()
            .expect("server runtime mutex")
            .redis_resp_server()?
    };
    if let Some(redis_server) = redis_server {
        let (redis_addr, redis_tls) = {
            let runtime = shared_runtime.lock().expect("server runtime mutex");
            (
                runtime
                    .redis_listener_addr()
                    .expect("redis server exists only when redis_api is enabled"),
                runtime.redis_tls_acceptor()?,
            )
        };
        let listener = TcpListener::bind(redis_addr).await?;
        let (redis_shutdown_tx, redis_shutdown_rx) = watch::channel(false);
        let runtime = Arc::clone(&shared_runtime);
        tokio::spawn(async move {
            let _redis_shutdown_tx = redis_shutdown_tx;
            if let Err(error) = serve_redis_listener(
                listener,
                Arc::new(redis_server),
                runtime,
                redis_tls,
                redis_shutdown_rx,
            )
            .await
            {
                eprintln!("{error}");
            }
        });
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
    if admin_enabled {
        let listener = TcpListener::bind(admin_addr).await?;
        axum::serve(listener, admin_surface.routes()).await?;
    } else {
        std::future::pending::<()>().await;
    }
    Ok(())
}

fn serde_json_like_health(status: &str) -> String {
    format!(r#"{{"status":"{status}"}}"#)
}
