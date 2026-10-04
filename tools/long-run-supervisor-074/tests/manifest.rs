#![recursion_limit = "256"]

use hydracache_long_run_supervisor_074::manifest::{
    parse_and_validate, ManifestError, MAX_MANIFEST_BYTES,
};
use hydracache_long_run_supervisor_074::protocol::{ControllerIdentity, Operation, Request};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn manifest() -> Value {
    json!({
        "schema_version": 1,
        "repository_id": 10,
        "authorization_identity": "protected-performance-074",
        "contract_sha256": "a".repeat(64),
        "tooling_sha": "b".repeat(40),
        "i74_source_sha": "c".repeat(40),
        "c74_source_sha": "d".repeat(40),
        "i74_tree_sha": "1".repeat(40),
        "c74_tree_sha": "2".repeat(40),
        "i74_cargo_lock_sha256": "3".repeat(64),
        "c74_cargo_lock_sha256": "4".repeat(64),
        "i74_dirty": false,
        "c74_dirty": false,
        "scenario_sha256": "e".repeat(64),
        "workload_sha256": "5".repeat(64),
        "offered_load_sha256": "6".repeat(64),
        "estimator_sha256": "7".repeat(64),
        "thresholds_sha256": "8".repeat(64),
        "host_receipt_sha256": "f".repeat(64),
        "lease_id": "123e4567-e89b-42d3-a456-426614174000",
        "machine_id": "machine-a",
        "boot_id": "boot-a",
        "mount_identity": "dev=1;opts=rw",
        "isolated_cpuset": "2-7",
        "housekeeping_cpuset": "0-1",
        "seed": 740074,
        "checkpoint_cadence_seconds": 30,
        "progress_warning_gap_seconds": 90,
        "progress_rejection_gap_seconds": 180,
        "diagnostic_grace_seconds": 30,
        "product_lease_deadline_unix_seconds": 2_000,
        "maximum_campaign_bytes": 21_474_836_480_u64,
        "maximum_campaign_files": 20_000,
        "installed_binaries": [
            {"role": "i74", "path": "/opt/hydracache-performance/0.74/i74/hydracache", "sha256": "9".repeat(64), "size": 1, "inode": 2, "device": 3, "uid": 1001, "gid": 1001, "mode": 365},
            {"role": "c74", "path": "/opt/hydracache-performance/0.74/c74/hydracache", "sha256": "a".repeat(64), "size": 1, "inode": 4, "device": 3, "uid": 1001, "gid": 1001, "mode": 365}
        ],
        "argv_templates": {
            "i74": ["/opt/hydracache-performance/0.74/i74/hydracache", "--role", "i74"],
            "c74": ["/opt/hydracache-performance/0.74/c74/hydracache", "--role", "c74"]
        },
        "command_environment_sha256": "b".repeat(64),
        "role_order": ["i74", "c74"],
        "phase_durations_seconds": {"warmup": 60, "measured": 300, "drain": 30, "durable_companion": 30, "post_work_idle": 60, "reconciliation": 30},
        "output_limits": {"stdout_bytes": 1_048_576, "stderr_bytes": 1_048_576, "diagnostic_bytes": 1_048_576, "final_artifact_bytes": 1_073_741_824, "files": 2_000},
        "expected_output_schema_sha256s": {"checkpoint": "c".repeat(64), "measurement": "d".repeat(64), "reconciliation": "e".repeat(64), "packet_manifest": "f".repeat(64)},
        "required_final_guards": ["semantic", "native-non-regression", "retention"],
        "secret_identifiers": ["github-environment-key-v1"],
        "release": "0.74",
        "campaign_id": "1".repeat(64),
        "nonce_sha256": "2".repeat(64),
        "dirty": false,
        "controller_history": [],
        "state": "PREPARED"
    })
}

fn encoded(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn fixture_request(bytes: &[u8]) -> Request {
    Request {
        schema_version: 1,
        request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Start,
        campaign_id: "1".repeat(64),
        expected_state_revision: 0,
        manifest_path: Some(format!(
            "/var/lib/hydracache-performance/staging/{}/campaign-start.json",
            "1".repeat(64)
        )),
        manifest_sha256: hex(&Sha256::digest(bytes)),
        controller: ControllerIdentity {
            repository_id: 10,
            run_id: 20,
            run_attempt: 1,
            actor_id: 30,
            authorization_sha256: "3".repeat(64),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    }
}

#[test]
fn exact_canonical_prepared_manifest_is_accepted() {
    let bytes = encoded(&manifest());
    let request = fixture_request(&bytes);
    assert!(parse_and_validate(&bytes, &request, 1_000).is_ok());
    let mut newline = bytes.clone();
    newline.push(b'\n');
    assert!(parse_and_validate(&newline, &request, 1_000).is_ok());
}

#[test]
fn digest_identity_and_frozen_field_drift_fail_closed() {
    let bytes = encoded(&manifest());
    let mut request = fixture_request(&bytes);
    request.controller.repository_id = 11;
    assert_eq!(
        parse_and_validate(&bytes, &request, 1_000),
        Err(ManifestError::Binding)
    );

    let mut changed = manifest();
    changed["progress_rejection_gap_seconds"] = json!(181);
    let bytes = encoded(&changed);
    let request = fixture_request(&bytes);
    assert_eq!(
        parse_and_validate(&bytes, &request, 1_000),
        Err(ManifestError::Invariant)
    );

    let mut request = fixture_request(&bytes);
    request.manifest_sha256 = "4".repeat(64);
    assert_eq!(
        parse_and_validate(&bytes, &request, 1_000),
        Err(ManifestError::Digest)
    );
}

#[test]
fn unknown_float_secret_value_and_oversize_fail_closed() {
    let mut unknown = manifest();
    unknown["command"] = json!("sh -c id");
    let bytes = encoded(&unknown);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Document)
    );

    let mut secret = manifest();
    secret["secret_identifiers"] = json!(["TOKEN=plaintext"]);
    let bytes = encoded(&secret);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Invariant)
    );

    let oversized = vec![b' '; MAX_MANIFEST_BYTES + 1];
    let request = fixture_request(&encoded(&manifest()));
    assert_eq!(
        parse_and_validate(&oversized, &request, 1_000),
        Err(ManifestError::Document)
    );
}

#[test]
fn execution_identity_and_nested_unknown_fields_fail_closed() {
    let mut dirty = manifest();
    dirty["i74_dirty"] = json!(true);
    let bytes = encoded(&dirty);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Invariant)
    );

    let mut argv = manifest();
    argv["argv_templates"]["c74"][0] = json!("/tmp/unbound");
    let bytes = encoded(&argv);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Invariant)
    );

    let mut nested_unknown = manifest();
    nested_unknown["output_limits"]["unbounded"] = json!(true);
    let bytes = encoded(&nested_unknown);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Document)
    );
}
