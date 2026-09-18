use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

#[test]
fn allocation_provider_contract_has_count_bytes_phase_and_redaction_fields() {
    let provider =
        fs::read_to_string(root().join("scripts/perf/memory-providers/provider_common.py"))
            .expect("provider contract");
    for marker in [
        "allocation_count",
        "allocated_bytes",
        "\"phase\"",
        "file_digest",
        "[redacted]",
    ] {
        assert!(
            provider.contains(marker),
            "provider contract omits {marker}"
        );
    }
}

#[test]
fn copy_optimization_is_not_promoted_without_paired_evidence() {
    let policy: Value = toml::from_str(
        &fs::read_to_string(root().join("docs/testing/memory/0.71/release-policy.toml"))
            .expect("policy"),
    )
    .expect("policy TOML");
    let row = policy["optional_work"]
        .as_array()
        .expect("optional work")
        .iter()
        .find(|row| row["id"].as_str() == Some("W5-W7"))
        .expect("W5-W7");
    assert_eq!(row["disposition"].as_str(), Some("deferred"));
    assert!(row["reason"]
        .as_str()
        .expect("reason")
        .contains("does not isolate"));
}

#[test]
fn archived_receipts_are_secret_free_by_contract() {
    let archive = fs::read_to_string(
        root().join("docs/testing/perf-artifacts/0.71/ax42/d0-baseline/README.md"),
    )
    .expect("archive README");
    for marker in ["sanitized", "redaction", "SHA256SUMS"] {
        assert!(archive.contains(marker), "archive contract omits {marker}");
    }
}
