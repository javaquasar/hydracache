#![cfg(target_os = "linux")]

use ed25519_dalek::SigningKey;
use hydracache_long_run_supervisor_074::config::{ConfigError, ServerConfig};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn config(socket: &str, campaign_root: &str) -> String {
    let key = SigningKey::from_bytes(&[7; 32]);
    format!(
        r#"schema_version = 1
socket_path = "{socket}"
campaign_root = "{campaign_root}"
socket_mode = 432
expected_repository_id = 10
allowed_actor_ids = [30]
allowed_client_uids = [1000]
required_client_gid = 2000
verification_key_hex = "{}"
"#,
        hex(key.verifying_key().as_bytes())
    )
}

#[test]
fn strict_config_builds_the_frozen_service_policy() {
    let parsed = ServerConfig::parse(
        config("/tmp/supervisor.sock", "/tmp/campaigns").as_bytes(),
        false,
    )
    .unwrap();
    let policy = parsed.policy().unwrap();
    assert_eq!(policy.expected_repository_id, 10);
    assert_eq!(policy.allowed_actor_ids, [30]);
    assert_eq!(policy.required_client_gid, 2000);
}

#[test]
fn unknown_duplicate_relaxed_and_nonproduction_values_fail_closed() {
    let mut unknown = config("/tmp/supervisor.sock", "/tmp/campaigns");
    unknown.push_str("shell = \"sh\"\n");
    assert_eq!(
        ServerConfig::parse(unknown.as_bytes(), false).unwrap_err(),
        ConfigError::Document
    );

    let duplicate = config("/tmp/supervisor.sock", "/tmp/campaigns")
        .replace("allowed_actor_ids = [30]", "allowed_actor_ids = [30, 30]");
    assert_eq!(
        ServerConfig::parse(duplicate.as_bytes(), false).unwrap_err(),
        ConfigError::Invariant
    );

    let relaxed = config("/tmp/supervisor.sock", "/tmp/campaigns")
        .replace("socket_mode = 432", "socket_mode = 438");
    assert_eq!(
        ServerConfig::parse(relaxed.as_bytes(), false).unwrap_err(),
        ConfigError::Invariant
    );

    assert_eq!(
        ServerConfig::parse(
            config("/tmp/supervisor.sock", "/tmp/campaigns").as_bytes(),
            true
        )
        .unwrap_err(),
        ConfigError::Invariant
    );
}
