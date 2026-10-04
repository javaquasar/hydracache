use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const FILES: [&str; 9] = [
    "status.json",
    "operation-contract.json",
    "resource-bounds.json",
    "surface-equivalence.json",
    "mutation-stages.json",
    "retry-idempotency.json",
    "failure-consistency-matrix.json",
    "rpo-rto-contract.json",
    "security-contract.json",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn fixture() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let source = root().join("docs/testing/imap/0.75");
    for file in FILES {
        fs::copy(source.join(file), temp.path().join(file)).unwrap();
    }
    temp
}

fn edit(path: &Path, mutate: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    mutate(&mut value);
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn problems(temp: &TempDir) -> Vec<String> {
    xtask::imap_contract::check_contract_dir(temp.path()).unwrap()
}

#[test]
fn repository_contract_is_complete_and_provisional() {
    let problems = xtask::imap_contract::check_at_root(&root(), "0.75").unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn command_accepts_release_075_contracts() {
    let status = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(root())
        .args(["imap-contract-check", "--release", "0.75"])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn canary_missing_test_id_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        value["operations"][0]["test_ids"] = Value::Array(Vec::new());
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("missing test id")));
}

#[test]
fn canary_missing_surface_cell_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("surface-equivalence.json"), |value| {
        value["surfaces"][0]["cells"].as_array_mut().unwrap().pop();
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("missing operation cell")));
}

#[test]
fn canary_missing_mutation_stage_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("mutation-stages.json"), |value| {
        value["stages"].as_array_mut().unwrap().remove(5);
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("mutation stages missing required id")));
}

#[test]
fn canary_missing_bound_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("resource-bounds.json"), |value| {
        value["bounds"].as_array_mut().unwrap().remove(0);
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("bounds missing required id")));
}

#[test]
fn canary_missing_proof_id_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("rpo-rto-contract.json"), |value| {
        value["cells"][0]["proof_id"] = Value::String(String::new());
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("proof_id must not be empty")));
}

#[test]
fn canary_finalized_w0_claim_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("status.json"), |value| {
        value["status"] = Value::String("finalized".into());
        value["w0_disposition"] = Value::String("closed".into());
    });
    let problems = problems(&temp);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("provisional")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("w0_disposition=open")));
}

#[test]
fn canary_missing_safe_foundation_slice_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("status.json"), |value| {
        value["safe_foundation"]["implemented"]
            .as_array_mut()
            .unwrap()
            .retain(|slice| slice != "linearizability_oracle");
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("missing implemented slice linearizability_oracle")));
}

#[test]
fn canary_missing_security_threat_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("security-contract.json"), |value| {
        value["threats"]
            .as_array_mut()
            .unwrap()
            .retain(|threat| threat["id"] != "false_backup_ack");
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("security threats missing required id false_backup_ack")));
}
