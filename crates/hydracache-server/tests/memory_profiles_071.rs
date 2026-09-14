use std::path::{Path, PathBuf};

use hydracache_server::{ClientApiConfig, RedisApiConfig, ServerConfig, ServerRuntime};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

#[test]
fn disabled_client_and_resp_services_allocate_no_dispatch_surface() {
    let config = ServerConfig {
        client_api: ClientApiConfig {
            enabled: false,
            ..ClientApiConfig::default()
        },
        redis_api: RedisApiConfig {
            enabled: false,
            ..RedisApiConfig::default()
        },
        ..ServerConfig::default()
    };
    let runtime = ServerRuntime::new(config).expect("runtime").start();
    assert!(!runtime.client_surface_ready());
    assert!(!runtime.redis_surface_ready());
    assert!(runtime.client_dispatch_state().is_none());
    assert_eq!(runtime.client_active_subscriptions(), 0);
    assert_eq!(runtime.redis_active_connections(), 0);
}

#[test]
fn enabling_client_or_resp_materializes_only_the_requested_surface() {
    let client = ServerRuntime::new(ServerConfig {
        client_api: ClientApiConfig {
            enabled: true,
            ..ClientApiConfig::default()
        },
        ..ServerConfig::default()
    })
    .expect("client runtime")
    .start();
    assert!(client.client_surface_ready());
    assert!(!client.redis_surface_ready());
    assert!(client.client_dispatch_state().is_some());

    let redis = ServerRuntime::new(ServerConfig {
        redis_api: RedisApiConfig {
            enabled: true,
            ..RedisApiConfig::default()
        },
        ..ServerConfig::default()
    })
    .expect("RESP runtime")
    .start();
    assert!(!redis.client_surface_ready());
    assert!(redis.redis_surface_ready());
    assert!(redis.client_dispatch_state().is_some());
}

#[test]
fn profile_changes_remain_deferred_without_one_factor_qualification() {
    let policy =
        std::fs::read_to_string(root().join("docs/testing/memory/0.71/release-policy.toml"))
            .expect("policy");
    let value: toml::Value = toml::from_str(&policy).expect("TOML");
    let row = value["optional_work"]
        .as_array()
        .expect("optional work")
        .iter()
        .find(|row| row["id"].as_str() == Some("W9-W11"))
        .expect("W9-W11");
    assert_eq!(row["disposition"].as_str(), Some("deferred"));
    assert!(row["next_evidence"]
        .as_str()
        .expect("next evidence")
        .contains("one-factor"));
}
