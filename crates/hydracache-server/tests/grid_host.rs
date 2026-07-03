use std::path::PathBuf;

use hydracache_server::{
    AdminApiConfig, BackupConfig, ClientApiConfig, ServerConfig, ServerConfigError, ServerRole,
    ServerRuntime, StatusSource, TlsConfig,
};
use serde_json::json;

fn member_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:17057".parse().unwrap(),
        seeds: vec!["127.0.0.1:17057".to_owned()],
        storage_dir: Some(PathBuf::from("target/test-hydracache-grid-host")),
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
        ..member_config()
    }
}

fn client_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Client,
        storage_dir: None,
        ..member_config()
    }
}

#[test]
fn member_role_reports_live_source_with_real_member_table() {
    let runtime = ServerRuntime::new(member_config()).unwrap().start();

    let status = runtime.admin_status();
    assert_eq!(status.source, StatusSource::Live);
    assert_eq!(status.leader, None);
    assert_eq!(status.term, 1);
    assert!(status.quorum_ok);
    assert_eq!(status.members, 1);

    let overview = serde_json::to_value(runtime.cluster_overview()).unwrap();
    assert_eq!(overview["source"], "live");
    assert_eq!(overview["leader"], json!(null));
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
    let mut missing_storage = member_config();
    missing_storage.storage_dir = None;
    assert!(matches!(
        ServerRuntime::new(missing_storage),
        Err(ServerConfigError::MissingStorageDir)
    ));

    let mut missing_seeds = member_config();
    missing_seeds.seeds.clear();
    assert!(matches!(
        ServerRuntime::new(missing_seeds),
        Err(ServerConfigError::MissingSeeds)
    ));
}

#[test]
#[ignore = "W6b deferred: requires networked raft/chitchat daemon wiring"]
fn multi_node_members_form_a_cluster_and_elect_one_leader() {}
