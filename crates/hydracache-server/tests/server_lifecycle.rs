use std::{
    env,
    ffi::OsString,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use hydracache_redis_compat::{RedisCommand, RespValue};
use hydracache_server::{
    serve_redis_listener, AdminApiConfig, BackupConfig, ClientApiConfig, ClusterAuthConfig,
    ClusterStartMode, RedisApiConfig, ServerConfig, ServerConfigError, ServerRole, ServerRuntime,
    ServerState, TlsConfig,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

fn member_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:0".parse().unwrap(),
        node_id: None,
        seeds: vec!["127.0.0.1:0".to_owned()],
        storage_dir: Some(PathBuf::from("target/test-hydracache-server")),
        drain_timeout_ms: 1_000,
        tls: TlsConfig::default(),
        cluster_auth: ClusterAuthConfig::default(),
        backup: BackupConfig::default(),
        client_api: ClientApiConfig::default(),
        admin_api: AdminApiConfig::default(),
        ..ServerConfig::default()
    }
}

fn member_config_with_redis_surface() -> ServerConfig {
    ServerConfig {
        redis_api: RedisApiConfig {
            enabled: true,
            listen_addr: "127.0.0.1:16379".parse().unwrap(),
        },
        ..member_config()
    }
}

static CONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

const CONFIG_ENV_VARS: &[&str] = &[
    "HYDRACACHE_ROLE",
    "HYDRACACHE_LISTEN_ADDR",
    "HYDRACACHE_CLUSTER_ADDR",
    "HYDRACACHE_CLUSTER_START",
    "HYDRACACHE_CLUSTER_ADVERTISE_ADDR",
    "HYDRACACHE_CLUSTER_HEADLESS_SERVICE",
    "HYDRACACHE_BOOTSTRAP_REPLICAS",
    "HYDRACACHE_NODE_ID",
    "HYDRACACHE_STORAGE_DIR",
    "HYDRACACHE_SEEDS",
    "HYDRACACHE_JOIN_TIMEOUT_MS",
    "HYDRACACHE_TLS_ACK_INSECURE",
    "HYDRACACHE_TLS_ENABLED",
    "HYDRACACHE_TLS_CERT_PATH",
    "HYDRACACHE_TLS_KEY_PATH",
    "HYDRACACHE_TLS_CA_PATH",
    "HYDRACACHE_CLUSTER_AUTH_KEY_ID",
    "HYDRACACHE_CLUSTER_AUTH_TOKEN_FILE",
    "HYDRACACHE_CLUSTER_AUTH_PREVIOUS_KEY_ID",
    "HYDRACACHE_CLUSTER_AUTH_PREVIOUS_TOKEN_FILE",
    "HYDRACACHE_BACKUP_ENABLED",
    "HYDRACACHE_BACKUP_LOCATION",
    "HYDRACACHE_CLIENT_API_ENABLED",
    "HYDRACACHE_ADMIN_API_ENABLED",
    "HYDRACACHE_ADMIN_ADDR",
    "HYDRACACHE_REDIS_API_ENABLED",
    "HYDRACACHE_REDIS_ADDR",
    "HOSTNAME",
];

struct ConfigEnvGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
    _lock: MutexGuard<'static, ()>,
}

impl ConfigEnvGuard {
    fn new(overrides: &[(&'static str, &'static str)]) -> Self {
        let lock = CONFIG_ENV_LOCK.lock().unwrap();
        let saved = CONFIG_ENV_VARS
            .iter()
            .map(|name| (*name, env::var_os(name)))
            .collect::<Vec<_>>();
        for name in CONFIG_ENV_VARS {
            env::remove_var(name);
        }
        for (name, value) in overrides {
            env::set_var(name, value);
        }
        Self { saved, _lock: lock }
    }
}

impl Drop for ConfigEnvGuard {
    fn drop(&mut self) {
        for name in CONFIG_ENV_VARS {
            env::remove_var(name);
        }
        for (name, value) in &self.saved {
            if let Some(value) = value {
                env::set_var(name, value);
            }
        }
    }
}

fn config_error_from_env(overrides: &[(&'static str, &'static str)]) -> ServerConfigError {
    let _guard = ConfigEnvGuard::new(overrides);
    ServerConfig::from_env().unwrap_err()
}

#[test]
fn server_lifecycle_server_starts_serves_health_ready_and_shuts_down_cleanly() {
    let mut runtime = ServerRuntime::new(member_config()).unwrap().start();

    assert_eq!(runtime.health().status, "ok");
    assert_eq!(runtime.health().state, ServerState::Running);
    assert!(runtime.ready().ready);
    assert!(runtime.begin_request());

    let drain = runtime.shutdown();

    assert_eq!(drain.started_with, 1);
    assert_eq!(drain.remaining, 0);
    assert!(!drain.timed_out);
    assert!(runtime.flushed());
    assert_eq!(runtime.health().state, ServerState::Stopped);
    assert!(!runtime.ready().ready);
    assert!(!runtime.begin_request());
}

#[test]
fn server_lifecycle_invalid_config_fails_loud() {
    let mut missing_storage = member_config();
    missing_storage.storage_dir = None;
    assert!(matches!(
        missing_storage.validate(),
        Err(ServerConfigError::MissingStorageDir)
    ));

    let mut external_without_tls = member_config();
    external_without_tls.listen_addr = "0.0.0.0:18080".parse().unwrap();
    assert!(matches!(
        external_without_tls.validate(),
        Err(ServerConfigError::NonLoopbackWithoutTls)
    ));

    let mut backup_without_location = member_config();
    backup_without_location.backup.enabled = true;
    assert!(matches!(
        backup_without_location.validate(),
        Err(ServerConfigError::MissingBackupLocation)
    ));

    let mut auth_without_token = member_config();
    auth_without_token.cluster_auth.key_id = Some("k1".to_owned());
    assert!(matches!(
        auth_without_token.validate(),
        Err(ServerConfigError::IncompleteClusterAuth { .. })
    ));

    let mut auth_with_missing_file = member_config();
    auth_with_missing_file.cluster_auth.key_id = Some("k1".to_owned());
    auth_with_missing_file.cluster_auth.token_file =
        Some(PathBuf::from("target/test-hydracache-server/missing-token"));
    assert!(matches!(
        auth_with_missing_file.validate(),
        Err(ServerConfigError::ClusterAuthTokenRead { .. })
    ));

    let mut empty_node_id = member_config();
    empty_node_id.node_id = Some("   ".to_owned());
    assert!(matches!(
        empty_node_id.validate(),
        Err(ServerConfigError::InvalidNodeId)
    ));

    let mut join_with_local_role = member_config();
    join_with_local_role.cluster_start = ClusterStartMode::Join;
    join_with_local_role.role = ServerRole::Local;
    assert!(matches!(
        join_with_local_role.validate(),
        Err(ServerConfigError::JoinRequiresMemberRole)
    ));

    let mut join_without_seeds = member_config();
    join_without_seeds.cluster_start = ClusterStartMode::Join;
    join_without_seeds.seeds.clear();
    assert!(matches!(
        join_without_seeds.validate(),
        Err(ServerConfigError::JoinRequiresSeeds)
    ));

    let mut zero_join_timeout = member_config();
    zero_join_timeout.join_timeout_ms = 0;
    assert!(matches!(
        zero_join_timeout.validate(),
        Err(ServerConfigError::JoinTimeoutZero)
    ));

    let mut empty_advertise_addr = member_config();
    empty_advertise_addr.cluster_advertise_addr = Some("   ".to_owned());
    assert!(matches!(
        empty_advertise_addr.validate(),
        Err(ServerConfigError::InvalidClusterAdvertiseAddr)
    ));

    let mut bind_addr_as_advertise_addr = member_config();
    bind_addr_as_advertise_addr.cluster_advertise_addr = Some("0.0.0.0:7000".to_owned());
    assert!(matches!(
        bind_addr_as_advertise_addr.validate(),
        Err(ServerConfigError::InvalidClusterAdvertiseAddr)
    ));

    let mut admin_conflict = member_config();
    admin_conflict.admin_api.listen_addr = admin_conflict.listen_addr;
    assert!(matches!(
        admin_conflict.validate(),
        Err(ServerConfigError::AdminAddressConflicts)
    ));

    let mut redis_listen_conflict = member_config();
    redis_listen_conflict.redis_api.enabled = true;
    redis_listen_conflict.redis_api.listen_addr = redis_listen_conflict.listen_addr;
    assert!(matches!(
        redis_listen_conflict.validate(),
        Err(ServerConfigError::RedisAddressConflicts {
            surface: "listen_addr"
        })
    ));

    let mut redis_cluster_conflict = member_config();
    redis_cluster_conflict.redis_api.enabled = true;
    redis_cluster_conflict.redis_api.listen_addr = redis_cluster_conflict.cluster_addr;
    assert!(matches!(
        redis_cluster_conflict.validate(),
        Err(ServerConfigError::RedisAddressConflicts {
            surface: "cluster_addr"
        })
    ));

    let mut redis_admin_conflict = member_config();
    redis_admin_conflict.redis_api.enabled = true;
    redis_admin_conflict.redis_api.listen_addr = redis_admin_conflict.admin_api.listen_addr;
    assert!(matches!(
        redis_admin_conflict.validate(),
        Err(ServerConfigError::RedisAddressConflicts {
            surface: "admin_api.listen_addr"
        })
    ));

    let mut client_limit_zero = member_config();
    client_limit_zero.client_api.enabled = true;
    client_limit_zero.client_api.limits.max_frame_bytes = 0;
    assert!(matches!(
        client_limit_zero.validate(),
        Err(ServerConfigError::InvalidClientApi(error)) if error.contains("max_frame_bytes")
    ));

    let mut previous_auth_without_token = member_config();
    previous_auth_without_token.cluster_auth.previous_key_id = Some("old".to_owned());
    assert!(matches!(
        previous_auth_without_token.validate(),
        Err(ServerConfigError::IncompleteClusterAuth {
            section: "cluster_auth.previous"
        })
    ));

    std::fs::create_dir_all("target/test-hydracache-server").unwrap();
    std::fs::write("target/test-hydracache-server/empty-token", "\n").unwrap();
    let mut empty_auth_token = member_config();
    empty_auth_token.cluster_auth.key_id = Some("k1".to_owned());
    empty_auth_token.cluster_auth.token_file =
        Some(PathBuf::from("target/test-hydracache-server/empty-token"));
    assert!(matches!(
        empty_auth_token.validate(),
        Err(ServerConfigError::EmptyClusterAuthToken { .. })
    ));

    let mut incomplete_tls = member_config();
    incomplete_tls.tls.enabled = true;
    incomplete_tls.tls.cert_path = Some(PathBuf::from("cert.pem"));
    assert!(matches!(
        incomplete_tls.validate(),
        Err(ServerConfigError::IncompleteTlsMaterial)
    ));

    let mut external_admin_without_tls = member_config();
    external_admin_without_tls.admin_api.listen_addr = "0.0.0.0:19091".parse().unwrap();
    assert!(matches!(
        external_admin_without_tls.validate(),
        Err(ServerConfigError::NonLoopbackWithoutTls)
    ));
}

#[test]
fn redis_api_is_off_by_default_and_env_gated() {
    {
        let _guard = ConfigEnvGuard::new(&[]);
        let default = ServerConfig::default();
        assert!(!default.redis_api.enabled);
        assert_eq!(
            default.redis_api.listen_addr,
            "127.0.0.1:6379".parse().unwrap()
        );
    }

    let _guard = ConfigEnvGuard::new(&[
        ("HYDRACACHE_REDIS_API_ENABLED", "true"),
        ("HYDRACACHE_REDIS_ADDR", "127.0.0.1:6380"),
    ]);
    let config = ServerConfig::from_env().unwrap();
    assert!(config.redis_api.enabled);
    assert_eq!(
        config.redis_api.listen_addr,
        "127.0.0.1:6380".parse().unwrap()
    );
}

#[test]
fn redis_api_addr_conflicting_with_client_or_admin_is_rejected_loud() {
    let mut client_conflict = member_config();
    client_conflict.redis_api.enabled = true;
    client_conflict.redis_api.listen_addr = client_conflict.listen_addr;
    assert!(matches!(
        client_conflict.validate(),
        Err(ServerConfigError::RedisAddressConflicts {
            surface: "listen_addr"
        })
    ));

    let mut admin_conflict = member_config();
    admin_conflict.redis_api.enabled = true;
    admin_conflict.redis_api.listen_addr = admin_conflict.admin_api.listen_addr;
    assert!(matches!(
        admin_conflict.validate(),
        Err(ServerConfigError::RedisAddressConflicts {
            surface: "admin_api.listen_addr"
        })
    ));
}

#[test]
fn daemon_serves_redis_resp_listener_only_when_enabled_and_drains_gracefully() {
    let disabled = ServerRuntime::new(member_config()).unwrap().start();
    assert!(!disabled.ready().redis_surface_ready);
    assert!(!disabled.redis_surface_ready());
    assert_eq!(disabled.redis_active_connections(), 0);

    let mut runtime = ServerRuntime::new(member_config_with_redis_surface())
        .unwrap()
        .start();

    assert!(runtime.ready().ready);
    assert!(runtime.ready().redis_surface_ready);
    assert!(runtime.redis_surface_ready());
    assert!(runtime.begin_redis_connection());
    assert_eq!(runtime.redis_active_connections(), 1);

    runtime.shutdown();

    assert!(!runtime.redis_surface_ready());
    assert_eq!(runtime.redis_active_connections(), 0);
    assert_eq!(runtime.redis_surface_drain().unwrap().started_with, 1);
    assert_eq!(runtime.redis_surface_drain().unwrap().remaining, 0);
}

#[test]
fn redis_resp_server_uses_client_surface_state_without_enabling_client_api_routes() {
    let mut config = member_config_with_redis_surface();
    config.client_api.enabled = false;
    let runtime = ServerRuntime::new(config).unwrap().start();
    let server = runtime.redis_resp_server().unwrap().unwrap();

    assert!(!runtime.client_surface_ready());
    assert_eq!(
        server.execute_command(RedisCommand::Set {
            key: b"k".to_vec(),
            value: b"v".to_vec(),
            options: Vec::new(),
        }),
        RespValue::SimpleString("OK")
    );
    assert_eq!(
        server.execute_command(RedisCommand::Get { key: b"k".to_vec() }),
        RespValue::BulkString(b"v".to_vec())
    );
}

#[tokio::test]
async fn redis_tcp_listener_accepts_real_socket_and_honors_drain_gate() {
    let runtime = Arc::new(Mutex::new(
        ServerRuntime::new(member_config_with_redis_surface())
            .unwrap()
            .start(),
    ));
    let server = Arc::new(
        runtime
            .lock()
            .expect("server runtime mutex")
            .redis_resp_server()
            .unwrap()
            .unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let serving = tokio::spawn(serve_redis_listener(
        listener,
        Arc::clone(&server),
        Arc::clone(&runtime),
        shutdown_rx,
    ));

    let mut socket = TcpStream::connect(addr).await.unwrap();
    socket
        .write_all(
            b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await
        .unwrap();
    let mut output = Vec::new();
    socket.read_to_end(&mut output).await.unwrap();
    assert_eq!(output, b"+OK\r\n$1\r\nv\r\n+OK\r\n");

    wait_for_no_redis_connections(&runtime).await;
    runtime.lock().expect("server runtime mutex").shutdown();

    let mut rejected = TcpStream::connect(addr).await.unwrap();
    rejected.write_all(b"*1\r\n$4\r\nPING\r\n").await.unwrap();
    let mut byte = [0; 1];
    match rejected.read(&mut byte).await {
        Ok(0) => {}
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
        other => panic!("drained Redis listener should close without RESP output: {other:?}"),
    }

    drop(shutdown_tx);
    serving.await.unwrap().unwrap();
}

#[test]
fn server_lifecycle_env_config_parses_join_mode_and_advertise_endpoint() {
    let _guard = ConfigEnvGuard::new(&[
        ("HYDRACACHE_ROLE", "member"),
        ("HYDRACACHE_LISTEN_ADDR", "127.0.0.1:18081"),
        ("HYDRACACHE_CLUSTER_ADDR", "0.0.0.0:7000"),
        ("HYDRACACHE_CLUSTER_START", "join"),
        (
            "HYDRACACHE_CLUSTER_ADVERTISE_ADDR",
            "demo-3.demo-headless:7000",
        ),
        ("HYDRACACHE_NODE_ID", "demo-3"),
        (
            "HYDRACACHE_STORAGE_DIR",
            "target/test-hydracache-server/env",
        ),
        (
            "HYDRACACHE_SEEDS",
            "demo-0.demo-headless:7000, ,demo-1.demo-headless:7000",
        ),
        ("HYDRACACHE_JOIN_TIMEOUT_MS", "4321"),
        ("HYDRACACHE_TLS_ACK_INSECURE", "true"),
        ("HYDRACACHE_ADMIN_API_ENABLED", "false"),
    ]);

    let config = ServerConfig::from_env().unwrap();

    assert_eq!(config.role, ServerRole::Member);
    assert_eq!(config.cluster_start, ClusterStartMode::Join);
    assert_eq!(config.node_id.as_deref(), Some("demo-3"));
    assert_eq!(
        config.seeds,
        vec![
            "demo-0.demo-headless:7000".to_owned(),
            "demo-1.demo-headless:7000".to_owned()
        ]
    );
    assert_eq!(config.join_timeout().as_millis(), 4_321);
    assert_eq!(
        config.cluster_advertise_endpoint(),
        "demo-3.demo-headless:7000"
    );
    assert!(config.tls.acknowledge_insecure);
    assert!(!config.admin_api.enabled);
}

async fn wait_for_no_redis_connections(runtime: &Arc<Mutex<ServerRuntime>>) {
    for _ in 0..10 {
        if runtime
            .lock()
            .expect("server runtime mutex")
            .redis_active_connections()
            == 0
        {
            return;
        }
        tokio::task::yield_now().await;
    }
}

#[test]
fn server_lifecycle_env_config_derives_statefulset_identity_without_shell_wrapper() {
    let _guard = ConfigEnvGuard::new(&[
        ("HYDRACACHE_ROLE", "member"),
        ("HYDRACACHE_LISTEN_ADDR", "127.0.0.1:18081"),
        ("HYDRACACHE_CLUSTER_ADDR", "0.0.0.0:7000"),
        ("HYDRACACHE_CLUSTER_HEADLESS_SERVICE", "demo-headless"),
        ("HYDRACACHE_BOOTSTRAP_REPLICAS", "3"),
        (
            "HYDRACACHE_STORAGE_DIR",
            "target/test-hydracache-server/env-statefulset",
        ),
        (
            "HYDRACACHE_SEEDS",
            "demo-0.demo-headless:7000,demo-1.demo-headless:7000,demo-2.demo-headless:7000",
        ),
        ("HYDRACACHE_TLS_ACK_INSECURE", "true"),
        ("HOSTNAME", "demo-3"),
    ]);

    let config = ServerConfig::from_env().unwrap();

    assert_eq!(config.cluster_start, ClusterStartMode::Join);
    assert_eq!(config.node_id.as_deref(), Some("demo-3"));
    assert_eq!(
        config.cluster_advertise_endpoint(),
        "demo-3.demo-headless:7000"
    );
}

#[test]
fn server_lifecycle_env_config_errors_are_specific() {
    assert!(matches!(
        config_error_from_env(&[("HYDRACACHE_ROLE", "worker")]),
        ServerConfigError::InvalidRole(value) if value == "worker"
    ));
    assert!(matches!(
        config_error_from_env(&[("HYDRACACHE_LISTEN_ADDR", "not-an-addr")]),
        ServerConfigError::InvalidAddress(value) if value == "not-an-addr"
    ));
    assert!(matches!(
        config_error_from_env(&[("HYDRACACHE_CLUSTER_START", "sideways")]),
        ServerConfigError::InvalidClusterStart(value) if value == "sideways"
    ));
    assert!(matches!(
        config_error_from_env(&[("HYDRACACHE_JOIN_TIMEOUT_MS", "forever")]),
        ServerConfigError::InvalidJoinTimeoutMs(value) if value == "forever"
    ));
    assert!(matches!(
        config_error_from_env(&[
            ("HYDRACACHE_BOOTSTRAP_REPLICAS", "zeroish"),
            ("HOSTNAME", "demo-0")
        ]),
        ServerConfigError::InvalidBootstrapReplicas(value) if value == "zeroish"
    ));
    assert!(matches!(
        config_error_from_env(&[
            ("HYDRACACHE_BOOTSTRAP_REPLICAS", "3"),
            ("HOSTNAME", "demo")
        ]),
        ServerConfigError::InvalidStatefulSetHostname(value) if value == "demo"
    ));
}

#[test]
fn server_lifecycle_toml_config_roundtrip_validates() {
    std::fs::create_dir_all("target/test-hydracache-server").unwrap();
    std::fs::write("target/test-hydracache-server/token", "secret\n").unwrap();

    let config = ServerConfig::from_toml_str(
        r#"
role = "member"
listen_addr = "127.0.0.1:18080"
cluster_addr = "127.0.0.1:17000"
cluster_start = "join"
cluster_advertise_addr = "member-configured.hydracache:17000"
node_id = "member-configured"
seeds = ["127.0.0.1:17000"]
storage_dir = "target/test-hydracache-server"
drain_timeout_ms = 1000
join_timeout_ms = 2500

[cluster_auth]
key_id = "k1"
token_file = "target/test-hydracache-server/token"

[admin_api]
enabled = true
listen_addr = "127.0.0.1:19091"
"#,
    )
    .unwrap();

    assert_eq!(config.role, ServerRole::Member);
    assert_eq!(config.cluster_start, ClusterStartMode::Join);
    assert_eq!(
        config.cluster_advertise_addr.as_deref(),
        Some("member-configured.hydracache:17000")
    );
    assert_eq!(config.node_id.as_deref(), Some("member-configured"));
    assert_eq!(config.drain_timeout().as_millis(), 1_000);
    assert_eq!(config.join_timeout().as_millis(), 2_500);
    assert_eq!(
        config.cluster_advertise_endpoint(),
        "member-configured.hydracache:17000"
    );

    let default_advertise = ServerConfig::default();
    assert_eq!(
        default_advertise.cluster_advertise_endpoint(),
        "127.0.0.1:7000"
    );
}
