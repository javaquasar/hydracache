use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use hydracache_server::{
    AdminApiConfig, BackupConfig, ClientApiConfig, ServerConfig, ServerConfigError, ServerRole,
    ServerRuntime, StatusSource, TlsConfig,
};
use serde_json::json;

fn member_config(name: &str) -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:0".parse().unwrap(),
        seeds: vec!["127.0.0.1:0".to_owned()],
        storage_dir: Some(PathBuf::from(format!(
            "target/test-hydracache-grid-host/{name}"
        ))),
        drain_timeout_ms: 1_000,
        tls: TlsConfig::default(),
        backup: BackupConfig::default(),
        client_api: ClientApiConfig::default(),
        admin_api: AdminApiConfig::default(),
    }
}

fn local_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Local,
        seeds: Vec::new(),
        storage_dir: None,
        ..member_config("local")
    }
}

fn client_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Client,
        storage_dir: None,
        ..member_config("client")
    }
}

fn grid_env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn configured_tls() -> TlsConfig {
    TlsConfig {
        enabled: true,
        cert_path: Some(PathBuf::from(
            "target/test-hydracache-grid-host/tls/cert.pem",
        )),
        key_path: Some(PathBuf::from(
            "target/test-hydracache-grid-host/tls/key.pem",
        )),
        ca_path: Some(PathBuf::from("target/test-hydracache-grid-host/tls/ca.pem")),
        acknowledge_insecure: false,
    }
}

#[test]
fn member_builds_networked_triad_with_shared_raft_control_plane() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let config = member_config("networked-shared-raft");
    let storage_dir = config.storage_dir.clone().unwrap();
    let _ = std::fs::remove_dir_all(&storage_dir);

    let runtime = ServerRuntime::new(config).unwrap().start();

    let status = runtime.admin_status();
    assert_eq!(status.source, StatusSource::Live);
    let leader = status.leader.clone().expect("networked raft leader");
    assert!(leader.starts_with("member-"));
    assert_eq!(status.term, 1);
    assert!(status.quorum_ok);
    assert_eq!(status.members, 1);
    assert!(storage_dir.join("raft-log").is_dir());
    assert_eq!(
        runtime.cache().cluster_diagnostics().unwrap().member_count,
        status.members as usize
    );

    let overview = serde_json::to_value(runtime.cluster_overview()).unwrap();
    assert_eq!(overview["source"], "live");
    assert_eq!(overview["leader"]["node_id"], json!(leader));
    assert_eq!(overview["members"].as_array().unwrap().len(), 1);
    assert_eq!(overview["members"][0]["role"], "member");
    assert_eq!(overview["members"][0]["reachable"], true);
    assert_eq!(overview["members"][0]["reachability"], "reachable");
    assert_eq!(overview["members"][0]["generation"], 1);
}

#[test]
fn local_and_client_roles_stay_modeled() {
    let local = ServerRuntime::new(local_config()).unwrap().start();
    let client = ServerRuntime::new(client_config()).unwrap().start();

    assert_eq!(local.admin_status().source, StatusSource::Modeled);
    assert_eq!(client.admin_status().source, StatusSource::Modeled);
    assert_eq!(local.admin_status().members, 0);
    assert_eq!(client.admin_status().members, 0);
}

#[test]
fn member_without_storage_or_seeds_is_rejected_loud() {
    let mut missing_storage = member_config("missing-storage");
    missing_storage.storage_dir = None;
    assert!(matches!(
        ServerRuntime::new(missing_storage),
        Err(ServerConfigError::MissingStorageDir)
    ));

    let mut missing_seeds = member_config("missing-seeds");
    missing_seeds.seeds.clear();
    assert!(matches!(
        ServerRuntime::new(missing_seeds),
        Err(ServerConfigError::MissingSeeds)
    ));
}

#[test]
fn inproc_fallback_still_builds_under_env_flag() {
    let _env = grid_env_lock();
    std::env::set_var("HYDRACACHE_GRID_INPROC", "1");
    let runtime = ServerRuntime::new(member_config("inproc-fallback"))
        .unwrap()
        .start();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");

    let status = runtime.admin_status();
    assert_eq!(status.source, StatusSource::Live);
    assert_eq!(status.leader, None);
    assert_eq!(status.term, 1);
    assert!(status.quorum_ok);
    assert_eq!(status.members, 1);
}

#[test]
fn draining_member_leaves_raft_config_cleanly() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let mut runtime = ServerRuntime::new(member_config("draining-leaves-raft"))
        .unwrap()
        .start();
    assert_eq!(runtime.admin_status().members, 1);

    let drain = runtime.shutdown();

    assert!(!drain.timed_out);
    assert_eq!(
        runtime.cache().cluster_diagnostics().unwrap().member_count,
        0
    );
    let status = runtime.admin_status();
    assert_eq!(status.members, 0);
    assert!(!status.quorum_ok);
}

#[test]
fn non_loopback_member_without_tls_is_rejected_loud() {
    let mut config = member_config("non-loopback-without-tls");
    config.cluster_addr = "0.0.0.0:17057".parse().unwrap();
    config.seeds = vec!["0.0.0.0:17057".to_owned()];

    assert!(matches!(
        ServerRuntime::new(config),
        Err(ServerConfigError::NonLoopbackWithoutTls)
    ));
}

#[test]
fn member_cluster_listener_uses_configured_tls() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let mut config = member_config("configured-tls");
    config.tls = configured_tls();

    let runtime = ServerRuntime::new(config).unwrap().start();

    let status = runtime.admin_status();
    assert_eq!(status.source, StatusSource::Live);
    assert!(status.leader.is_some());
    assert!(status.quorum_ok);
}

#[test]
#[ignore = "W6b deferred: requires networked raft/chitchat daemon wiring"]
fn multi_node_members_form_a_cluster_and_elect_one_leader() {}
