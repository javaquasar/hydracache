use std::path::PathBuf;

use hydracache_server::{
    AdminApiConfig, BackupConfig, ClientApiConfig, ClusterAuthConfig, ServerConfig,
    ServerConfigError, ServerRole, ServerRuntime, ServerState, TlsConfig,
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
node_id = "member-configured"
seeds = ["127.0.0.1:17000"]
storage_dir = "target/test-hydracache-server"
drain_timeout_ms = 1000

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
    assert_eq!(config.node_id.as_deref(), Some("member-configured"));
    assert_eq!(config.drain_timeout().as_millis(), 1_000);
}
