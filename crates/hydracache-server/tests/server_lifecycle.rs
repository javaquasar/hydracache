use std::{
    env,
    ffi::OsString,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use hydracache_server::{
    AdminApiConfig, BackupConfig, ClientApiConfig, ClusterAuthConfig, ClusterStartMode,
    ServerConfig, ServerConfigError, ServerRole, ServerRuntime, ServerState, TlsConfig,
};

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

static CONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

const CONFIG_ENV_VARS: &[&str] = &[
    "HYDRACACHE_ROLE",
    "HYDRACACHE_LISTEN_ADDR",
    "HYDRACACHE_CLUSTER_ADDR",
    "HYDRACACHE_CLUSTER_START",
    "HYDRACACHE_CLUSTER_ADVERTISE_ADDR",
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
