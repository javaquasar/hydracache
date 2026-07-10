use std::net::SocketAddr;
use std::sync::Arc;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer, DEFAULT_REDIS_NAMESPACE};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;

const CLIENT_MATRIX_ENV: &str = "HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS";

#[test]
fn redis_client_gate_manifest_and_docs_are_wired() {
    let gates = include_str!("../../../docs/GATES.md");
    let testing = include_str!("../../../docs/TESTING.md");
    let manifest = include_str!("../../../docs/integrations/redis_compat_conformance.json");

    assert!(gates.contains(CLIENT_MATRIX_ENV));
    assert!(gates.contains("--test redis_clients"));
    assert!(testing.contains("--test redis_clients"));
    assert!(manifest.contains(r#""images": ["redis:6.2.14", "redis:7.2.5"]"#));
    assert!(manifest.contains("redis_oracle_supported_subset_matches_real_redis"));
    assert!(
        manifest.contains("nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset")
    );
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; uses mainstream redis-rs client path"]
async fn mainstream_redis_client_can_talk_to_the_facade() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_resp_facade().await;
    let client = redis::Client::open(format!("redis://{addr}/")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();

    let pong: String = redis::cmd("PING")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(pong, "PONG");

    let _: () = redis::cmd("SET")
        .arg("k")
        .arg("v")
        .query_async(&mut connection)
        .await
        .unwrap();
    let value: String = redis::cmd("GET")
        .arg("k")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(value, "v");

    let values: Vec<Option<String>> = redis::cmd("MGET")
        .arg("k")
        .arg("missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(values, vec![Some("v".to_owned()), None]);

    let exists: i64 = redis::cmd("EXISTS")
        .arg("k")
        .arg("missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(exists, 1);

    let deleted: i64 = redis::cmd("DEL")
        .arg("k")
        .arg("missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(deleted, 1);

    drop(connection);
    drop(shutdown);
    serving.await.unwrap();
}

#[test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
fn redis_oracle_uses_pinned_redis_versions() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let manifest = include_str!("../../../docs/integrations/redis_compat_conformance.json");
    assert!(manifest.contains(r#""redis:6.2.14""#));
    assert!(manifest.contains(r#""redis:7.2.5""#));
    assert!(!manifest.contains("redis:latest"));
}

fn env_gate_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

async fn spawn_resp_facade() -> (watch::Sender<bool>, SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let server = Arc::new(
        RedisRespServer::new(
            state,
            RedisListenerConfig {
                tenant: DEFAULT_REDIS_NAMESPACE.to_owned(),
                ..RedisListenerConfig::default()
            },
        )
        .unwrap(),
    );
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let serving = tokio::spawn(async move {
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return;
                    }
                }
                accepted = listener.accept() => {
                    let (stream, _) = accepted.unwrap();
                    let server = Arc::clone(&server);
                    tokio::spawn(async move {
                        let _ = server.serve_connection(stream).await;
                    });
                }
            }
        }
    });
    (shutdown_tx, addr, serving)
}
