use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use xtask::imap_qualification_075::*;

const COMPARISON: &str =
    include_str!("../../../docs/testing/imap/0.75/hazelcast-comparison-contract.toml");
const POLICY: &str =
    include_str!("../../../docs/testing/imap/0.75/release-admission-contract.toml");
const DIR: &str = "docs/testing/imap/0.75";
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn contract() -> ComparisonContract {
    toml::from_str(COMPARISON).unwrap()
}
fn policy() -> ReleasePolicy {
    toml::from_str(POLICY).unwrap()
}
fn write(root: &Path, path: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        &format!("{DIR}/hazelcast-comparison-contract.toml"),
        COMPARISON,
    );
    write(
        temp.path(),
        &format!("{DIR}/release-admission-contract.toml"),
        POLICY,
    );
    write(temp.path(), &format!("{DIR}/status.json"), br#"{"release":"0.75","contract_version":1,"production_capability":"disabled_fail_closed","w0_disposition":"open"}"#);
    temp
}

#[test]
fn comparison_contract_is_valid_but_unpinned_products_cannot_be_ready() {
    let blockers = check_comparison_at_root(&root()).unwrap();
    assert_eq!(blockers.len(), 6);
    let mut candidate = contract();
    for product in &mut candidate.products {
        product.source_sha = "a".repeat(40);
        product.artifact_sha256 = "b".repeat(64);
        product.dependency_sha256 = "c".repeat(64);
    }
    assert!(validate_comparison(&candidate).unwrap().is_empty());
    let duplicate = candidate.products[0].clone();
    candidate.products[1] = duplicate;
    assert!(validate_comparison(&candidate).is_err());
}

#[test]
fn comparison_canaries_reject_semantic_mismatch_and_weakened_measurement() {
    for field in [
        "backups",
        "persistence",
        "tls",
        "ttl",
        "previous_value",
        "retry",
        "listeners",
        "acknowledgement",
        "key_codec",
    ] {
        let mut value = serde_json::to_value(contract()).unwrap();
        let target = &mut value["cells"][1]["hazelcast"][field];
        *target = match field {
            "backups" => serde_json::json!(0),
            "tls" | "previous_value" | "listeners" => serde_json::json!(!target.as_bool().unwrap()),
            "persistence" => serde_json::json!("durable"),
            "ttl" => serde_json::json!("eternal"),
            "retry" => serde_json::json!("new_identity"),
            "acknowledgement" => serde_json::json!("owner"),
            _ => serde_json::json!("java_hash"),
        };
        assert!(
            validate_comparison(&serde_json::from_value(value).unwrap()).is_err(),
            "{field}"
        );
    }
    for field in [
        "minimum_pairs",
        "minimum_goodput_advantage_percent",
        "maximum_p99_regression_percent",
        "require_balanced_order",
        "retain_negative_results",
        "production_enabled",
        "claims_enabled",
    ] {
        let mut value = serde_json::to_value(contract()).unwrap();
        value[field] = match field {
            "maximum_p99_regression_percent" => serde_json::json!(6),
            "production_enabled" | "claims_enabled" => serde_json::json!(true),
            "require_balanced_order" | "retain_negative_results" => serde_json::json!(false),
            _ => serde_json::json!(1),
        };
        assert!(
            validate_comparison(&serde_json::from_value(value).unwrap()).is_err(),
            "{field}"
        );
    }
    let mut candidate = contract();
    candidate.cells[1].members = 2;
    assert!(validate_comparison(&candidate).is_err());
    let mut candidate = contract();
    candidate.cells[1].hydra.backups = 0;
    candidate.cells[1].hazelcast.backups = 0;
    candidate.cells[1].hydra.acknowledgement = "owner".into();
    candidate.cells[1].hazelcast.acknowledgement = "owner".into();
    assert!(validate_comparison(&candidate).is_err());
    let mut candidate = contract();
    candidate.products[0].source_sha = "mutable-branch".into();
    assert!(validate_comparison(&candidate).is_err());
    let mut candidate = contract();
    candidate.cells.push(candidate.cells[0].clone());
    assert!(validate_comparison(&candidate).is_err());
    let mut candidate = contract();
    candidate.cells.pop();
    assert!(validate_comparison(&candidate).is_err());
    let mut candidate = contract();
    let mut mismatch = candidate.cells[0].clone();
    mismatch.id = "excluded".into();
    mismatch.hazelcast.tls = false;
    mismatch.disposition = "not_comparable".into();
    mismatch.reason = "different TLS".into();
    candidate.cells.push(mismatch);
    assert!(validate_comparison(&candidate)
        .unwrap()
        .iter()
        .any(|v| v.contains("not comparable")));
}

fn evidence(root: &Path) -> ReleaseManifest {
    let candidate = "a".repeat(40);
    let tooling = "b".repeat(40);
    let schema = "c".repeat(64);
    let sdk = "d".repeat(64);
    let config = "e".repeat(64);
    let mut refs = Vec::new();
    for kind in &policy().required_evidence {
        let artifact_path = format!("evidence/{kind}.log");
        let payload = format!("synthetic validation fixture for {kind}");
        write(root, &artifact_path, payload.as_bytes());
        let receipt = ProductionReceipt {
            release: "0.75".into(),
            kind: kind.clone(),
            scope: "production".into(),
            source_sha: candidate.clone(),
            tooling_sha: tooling.clone(),
            predecessor_sha: "f".repeat(40),
            schema_sha256: schema.clone(),
            sdk_sha256: sdk.clone(),
            config_sha256: config.clone(),
            producer_command: expected_producer(kind),
            coverage: required_coverage(kind),
            passed: true,
            complete: true,
            fixture: false,
            total: 3,
            executed: 3,
            failed: 0,
            skipped: 0,
            artifacts: vec![ArtifactRef {
                path: artifact_path,
                sha256: digest(payload.as_bytes()),
            }],
        };
        let path = format!("evidence/{kind}.json");
        let bytes = serde_json::to_vec(&receipt).unwrap();
        write(root, &path, &bytes);
        refs.push(ArtifactRef {
            path,
            sha256: digest(&bytes),
        });
    }
    ReleaseManifest {
        release: "0.75".into(),
        contract_version: 1,
        candidate_sha: candidate,
        tooling_sha: tooling,
        predecessor_sha: "f".repeat(40),
        predecessor_tag: "v0.74.0".into(),
        schema_sha256: schema,
        sdk_sha256: sdk,
        config_sha256: config,
        comparison_sha256: digest(COMPARISON.as_bytes()),
        receipts: refs,
    }
}

#[test]
fn complete_exact_source_evidence_is_structurally_valid_without_admitting_the_repository() {
    let temp = fixture();
    let manifest = evidence(temp.path());
    validate_release_manifest(temp.path(), &policy(), &manifest).unwrap();
    write(
        temp.path(),
        "evidence/manifest.json",
        serde_json::to_vec(&manifest).unwrap(),
    );
    let report = check_release_at_root(temp.path(), Some("evidence/manifest.json")).unwrap();
    assert!(!report.admitted);
    assert!(report.blockers.iter().any(|v| v.contains("capability")));
    assert!(report.blockers.iter().any(|v| v.contains("predecessor")));
    let report = check_release_at_root(&root(), None).unwrap();
    assert!(!report.admitted);
    assert!(report.blockers.iter().any(|v| v.contains("manifest")));
}

#[test]
fn release_canaries_reject_foundation_mixed_source_partial_and_tampered_receipts() {
    for field in [
        "scope",
        "source_sha",
        "tooling_sha",
        "predecessor_sha",
        "schema_sha256",
        "sdk_sha256",
        "config_sha256",
        "producer_command",
        "coverage",
        "passed",
        "complete",
        "fixture",
        "total",
        "executed",
        "failed",
        "skipped",
        "artifacts",
    ] {
        let temp = fixture();
        let mut manifest = evidence(temp.path());
        let first = &manifest.receipts[0];
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(temp.path().join(&first.path)).unwrap()).unwrap();
        value[field] = match field {
            "scope" => serde_json::json!("local_foundation"),
            "source_sha" | "tooling_sha" | "predecessor_sha" => serde_json::json!("0".repeat(40)),
            "schema_sha256" | "sdk_sha256" | "config_sha256" => serde_json::json!("0".repeat(64)),
            "producer_command" => serde_json::json!("cargo test"),
            "passed" | "complete" => serde_json::json!(false),
            "fixture" => serde_json::json!(true),
            "failed" | "skipped" => serde_json::json!(1),
            "artifacts" | "coverage" => serde_json::json!([]),
            _ => serde_json::json!(0),
        };
        let bytes = serde_json::to_vec(&value).unwrap();
        write(temp.path(), &first.path, &bytes);
        manifest.receipts[0].sha256 = digest(&bytes);
        assert!(
            validate_release_manifest(temp.path(), &policy(), &manifest).is_err(),
            "{field}"
        );
    }
    let temp = fixture();
    let mut manifest = evidence(temp.path());
    manifest.receipts.pop();
    assert!(validate_release_manifest(temp.path(), &policy(), &manifest).is_err());
    let mut manifest = evidence(temp.path());
    manifest.receipts.push(manifest.receipts[0].clone());
    assert!(validate_release_manifest(temp.path(), &policy(), &manifest).is_err());
    let manifest = evidence(temp.path());
    write(temp.path(), "evidence/compatibility.log", b"changed");
    assert!(validate_release_manifest(temp.path(), &policy(), &manifest).is_err());
    let mut manifest = evidence(temp.path());
    manifest.comparison_sha256 = "0".repeat(64);
    assert!(validate_release_manifest(temp.path(), &policy(), &manifest).is_err());
    let mut p = policy();
    p.required_evidence.pop();
    assert!(validate_release_policy(&p).is_err());
}

#[test]
fn evidence_paths_and_cli_boundaries_fail_before_loading_external_artifacts() {
    let temp = fixture();
    for path in ["../secret", "/absolute", "C:\\secret", "docs/../secret", ""] {
        assert!(read_relative(temp.path(), path, 100).is_err(), "{path}");
    }
    write(temp.path(), "large", vec![0; 101]);
    assert!(read_relative(temp.path(), "large", 100).is_err());
    assert!(read_relative(temp.path(), "large", 0).is_err());
    assert!(read_relative(temp.path(), "large", usize::MAX).is_err());
    assert!(!hex(&"A".repeat(64), 64));
    assert!(!hex("aa", 40));
    for command in ["imap-comparison-check", "imap-release-admission"] {
        for args in [
            vec!["--release", "0.74"],
            vec!["--release"],
            vec!["--release", "0.75", "--unknown", "x"],
            vec!["--release", "0.75", "--release", "0.75"],
        ] {
            assert!(!Command::new(env!("CARGO_BIN_EXE_xtask"))
                .current_dir(root())
                .arg(command)
                .args(args)
                .output()
                .unwrap()
                .status
                .success());
        }
    }
    for (command, flag) in [
        ("imap-comparison-check", "--require-ready"),
        ("imap-release-admission", "--require-ship"),
    ] {
        assert!(!Command::new(env!("CARGO_BIN_EXE_xtask"))
            .current_dir(root())
            .args([command, "--release", "0.75", flag])
            .output()
            .unwrap()
            .status
            .success());
    }
}

#[cfg(unix)]
#[test]
fn symlinked_evidence_is_refused() {
    let temp = fixture();
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "secret", b"data");
    std::os::unix::fs::symlink(outside.path(), temp.path().join("escape")).unwrap();
    assert!(read_relative(temp.path(), "escape/secret", 100).is_err());
}
