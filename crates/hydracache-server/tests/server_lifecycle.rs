use std::path::PathBuf;

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
}
