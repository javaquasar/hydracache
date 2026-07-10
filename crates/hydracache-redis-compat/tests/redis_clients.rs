use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer, DEFAULT_REDIS_NAMESPACE};
use redis::Value;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;

const CLIENT_MATRIX_ENV: &str = "HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS";
const PINNED_REDIS_IMAGES: [&str; 2] = ["redis:6.2.14", "redis:7.2.5"];

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
    assert!(manifest.contains("redis_oracle_del_exists_counts_match_real_redis"));
    assert!(manifest.contains("redis_oracle_mget_nil_and_order_match_real_redis"));
    assert!(manifest.contains("redis_oracle_unsupported_divergence_is_documented"));
    assert!(manifest.contains("redis_oracle_hc_extensions_are_hydracache_only"));
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
    for image in PINNED_REDIS_IMAGES {
        assert!(manifest.contains(&format!(r#""{image}""#)));
    }
    assert!(!manifest.contains("redis:latest"));
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_supported_subset_matches_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_supported_subset_scenario(oracle.addr, "oracle").await;
    let hydracache_replies = run_supported_subset_scenario(hydracache_addr, "oracle").await;

    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_del_exists_counts_match_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_count_scenario(oracle.addr, "counts").await;
    let hydracache_replies = run_count_scenario(hydracache_addr, "counts").await;

    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_mget_nil_and_order_match_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_mget_order_scenario(oracle.addr, "mget").await;
    let hydracache_replies = run_mget_order_scenario(hydracache_addr, "mget").await;

    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_unsupported_divergence_is_documented() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_reply = query_reply(oracle.addr, "HSET", &["hash", "field", "value"]).await;
    let hydracache_reply = query_reply(hydracache_addr, "HSET", &["hash", "field", "value"]).await;

    assert_eq!(redis_reply, OracleReply::Int(1));
    assert!(matches!(
        hydracache_reply,
        OracleReply::ErrorClass(ref class) if class == "ERR"
    ));

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_hc_extensions_are_hydracache_only() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_reply = query_reply(oracle.addr, "HC.STATS", &[]).await;
    let hydracache_reply = query_reply(hydracache_addr, "HC.STATS", &[]).await;

    assert!(matches!(
        redis_reply,
        OracleReply::ErrorClass(ref class) if class == "ERR"
    ));
    assert!(matches!(hydracache_reply, OracleReply::Array(_)));

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

fn env_gate_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OracleReply {
    Status(String),
    Bulk(Vec<u8>),
    Int(i64),
    Array(Vec<OracleReply>),
    Nil,
    ErrorClass(String),
}

impl OracleReply {
    fn from_value(value: Value) -> Self {
        match value {
            Value::Nil => Self::Nil,
            Value::Int(value) => Self::Int(value),
            Value::BulkString(value) => Self::Bulk(value),
            Value::Array(values) => Self::Array(values.into_iter().map(Self::from_value).collect()),
            Value::SimpleString(value) => Self::Status(value),
            Value::Okay => Self::Status("OK".to_owned()),
            Value::ServerError(error) => Self::ErrorClass(error.code().to_owned()),
            other => Self::Status(format!("{other:?}")),
        }
    }
}

async fn run_supported_subset_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let key = format!("{prefix}:k");
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "PING", &[]).await,
        query_reply(addr, "ECHO", &["hello"]).await,
        query_reply(addr, "SET", &[&key, "v"]).await,
        query_reply(addr, "GET", &[&key]).await,
        query_reply(addr, "MGET", &[&key, &missing]).await,
        query_reply(addr, "EXISTS", &[&key, &missing]).await,
        query_reply(addr, "DEL", &[&key, &missing]).await,
        query_reply(addr, "GET", &[&key]).await,
    ]
}

async fn run_count_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let first = format!("{prefix}:first");
    let second = format!("{prefix}:second");
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "SET", &[&first, "1"]).await,
        query_reply(addr, "SET", &[&second, "2"]).await,
        query_reply(addr, "EXISTS", &[&first, &second, &missing]).await,
        query_reply(addr, "DEL", &[&first, &second, &missing]).await,
        query_reply(addr, "EXISTS", &[&first, &second, &missing]).await,
    ]
}

async fn run_mget_order_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let first = format!("{prefix}:first");
    let second = format!("{prefix}:second");
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "SET", &[&first, "1"]).await,
        query_reply(addr, "SET", &[&second, "2"]).await,
        query_reply(addr, "MGET", &[&second, &missing, &first]).await,
    ]
}

async fn query_reply(addr: SocketAddr, command: &str, args: &[&str]) -> OracleReply {
    let client = redis::Client::open(format!("redis://{addr}/")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();
    let mut cmd = redis::cmd(command);
    for arg in args {
        cmd.arg(arg);
    }
    match cmd.query_async::<Value>(&mut connection).await {
        Ok(value) => OracleReply::from_value(value),
        Err(error) => OracleReply::ErrorClass(error_class(&error)),
    }
}

fn error_class(error: &redis::RedisError) -> String {
    error
        .to_string()
        .split_whitespace()
        .next()
        .unwrap_or("ERR")
        .trim_start_matches('-')
        .to_owned()
}

struct RedisOracle {
    container_id: String,
    addr: SocketAddr,
}

impl RedisOracle {
    async fn start_first_available() -> Option<Self> {
        if !docker_available() {
            eprintln!("skipping real Redis oracle; docker CLI is not available");
            return None;
        }
        let mut last_error = None;
        for image in PINNED_REDIS_IMAGES {
            match Self::start(image).await {
                Ok(oracle) => return Some(oracle),
                Err(error) => {
                    last_error = Some(error);
                    eprintln!("redis oracle image {image} did not start: {last_error:?}");
                }
            }
        }
        panic!(
            "none of the pinned Redis oracle images could start: {:?}",
            last_error
        );
    }

    async fn start(image: &str) -> Result<Self, String> {
        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-d",
                "-p",
                "127.0.0.1::6379",
                image,
                "redis-server",
                "--save",
                "",
                "--appendonly",
                "no",
            ])
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        let container_id = String::from_utf8(output.stdout)
            .map_err(|error| error.to_string())?
            .trim()
            .to_owned();
        let addr = docker_published_redis_addr(&container_id)?;
        wait_for_redis(addr).await?;
        Ok(Self { container_id, addr })
    }
}

impl Drop for RedisOracle {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container_id])
            .output();
    }
}

fn docker_available() -> bool {
    Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn docker_published_redis_addr(container_id: &str) -> Result<SocketAddr, String> {
    let output = Command::new("docker")
        .args(["port", container_id, "6379/tcp"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let port_line = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let port = port_line
        .lines()
        .next()
        .and_then(|line| line.rsplit(':').next())
        .ok_or_else(|| format!("docker port did not return host port: {port_line}"))?;
    format!("127.0.0.1:{port}")
        .parse::<SocketAddr>()
        .map_err(|error| error.to_string())
}

async fn wait_for_redis(addr: SocketAddr) -> Result<(), String> {
    for _ in 0..50 {
        let client = redis::Client::open(format!("redis://{addr}/")).map_err(|e| e.to_string())?;
        if let Ok(mut connection) = client.get_multiplexed_async_connection().await {
            let pong = redis::cmd("PING")
                .query_async::<String>(&mut connection)
                .await;
            if pong.as_deref() == Ok("PONG") {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(format!("redis oracle at {addr} did not become ready"))
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
