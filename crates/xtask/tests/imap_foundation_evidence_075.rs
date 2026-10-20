use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use xtask::imap_foundation_evidence::check_at_root;
use xtask::imap_foundation_generate::generate_at_root;
use xtask::imap_value_plane_model::{execute_for_test, git_head, hex_digest};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn write_receipt(value: &Value) -> (tempfile::TempDir, PathBuf) {
    let temp = tempdir().unwrap();
    let path = temp.path().join("receipt.json");
    fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    (temp, path)
}

fn canonical_digest(value: &Value) -> String {
    hex_digest(Sha256::digest(serde_json::to_vec(value).unwrap()).as_slice())
}

#[test]
fn authority_model_receipt_is_exact_source_and_schema_valid() {
    let workspace = root();
    let receipt = execute_for_test(&workspace, 117, 8, 20_000).unwrap();
    assert_eq!(receipt.result, "passed");
    assert!(!receipt.truncated);
    let value = serde_json::to_value(receipt).unwrap();
    let (_temp, path) = write_receipt(&value);
    assert_eq!(
        check_at_root(&workspace, "0.75", Some(&path)).unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn tampered_source_sha_is_rejected() {
    let workspace = root();
    let mut receipt =
        serde_json::to_value(execute_for_test(&workspace, 117, 8, 20_000).unwrap()).unwrap();
    receipt["source_sha"] = json!("0000000000000000000000000000000000000000");
    let (_temp, path) = write_receipt(&receipt);
    let problems = check_at_root(&workspace, "0.75", Some(&path)).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("current HEAD")));
}

#[test]
fn unknown_fault_checkpoint_is_rejected() {
    let workspace = root();
    let checkpoints = json!(["after_replica_proof", "invented_checkpoint"]);
    let receipt = json!({
        "schema": "hydracache.imap.fault-schedule-receipt.v1",
        "release": "0.75",
        "source_sha": git_head(&workspace).unwrap(),
        "seed": 117,
        "chaos_steps": 128,
        "chaos_trace_fingerprint": "0000000000000075",
        "schedule_sha256": canonical_digest(&checkpoints),
        "checkpoints": checkpoints,
        "result": "passed"
    });
    let (_temp, path) = write_receipt(&receipt);
    let problems = check_at_root(&workspace, "0.75", Some(&path)).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("schema violation")));
}

#[test]
fn truncated_history_checksum_is_rejected() {
    let workspace = root();
    let full_history = json!([{"id": 1}, {"id": 2}]);
    let receipt = json!({
        "schema": "hydracache.imap.linearizability-receipt.v1",
        "release": "0.75",
        "source_sha": git_head(&workspace).unwrap(),
        "history": [{"id": 1}],
        "history_sha256": canonical_digest(&full_history),
        "complete": true,
        "result": "passed"
    });
    let (_temp, path) = write_receipt(&receipt);
    let problems = check_at_root(&workspace, "0.75", Some(&path)).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("canonical history")));
}

#[test]
fn tampered_chaos_fingerprint_is_rejected_by_seed_replay() {
    let workspace = root();
    let temp = tempdir().unwrap();
    let output = temp.path().join("evidence");
    generate_at_root(&workspace, &output, 117).unwrap();
    let path = output.join("fault-schedule.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt["chaos_trace_fingerprint"] = json!("0000000000000000");
    fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();

    let problems = check_at_root(&workspace, "0.75", Some(&path)).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("does not replay from seed")));
}
