use serde_json::{json, Value as JsonValue};
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn load_toml(relative: &str) -> toml::Value {
    toml::from_str(&fs::read_to_string(root().join(relative)).expect("contract file"))
        .expect("valid TOML")
}

fn valid_historical_receipt() -> JsonValue {
    json!({
        "schema_version": 1,
        "release": "0.71",
        "branch": "explore/0.67-telemetry-hazelcast",
        "tag": "explore-0.67-telemetry-20260803",
        "commit": "dbc2f82f7f303528b3cca7842818730c82232b9c",
        "checkout_clean": true,
        "files": [{
            "path": "raw/memory.jsonl",
            "bytes": 123,
            "sha256": format!("sha256:{}", "1".repeat(64))
        }],
        "mirror": {
            "provider": "protected-object-store",
            "object_id": "memory-067/raw.tar.zst",
            "archive_sha256": format!("sha256:{}", "2".repeat(64)),
            "byte_length": 456,
            "manifest_sha256": format!("sha256:{}", "3".repeat(64)),
            "restored_manifest_sha256": format!("sha256:{}", "3".repeat(64)),
            "verified_at": "2026-08-20T12:00:00Z",
            "retention_deadline": "2027-08-20T12:00:00Z"
        }
    })
}

#[test]
fn b0_b1_are_distinct_cohorts() {
    let identities = load_toml("docs/testing/memory/0.71/baseline-identities.toml");
    assert_ne!(
        identities["b0_release"]["source_sha"],
        identities["b1_instrumented"]["source_sha"]
    );
    assert_eq!(identities["cohort_pooling"].as_bool(), Some(false));
    assert_eq!(
        identities["candidate_derived_thresholds"].as_bool(),
        Some(false)
    );
    let problems = xtask::memory_baseline::check_contract(&root(), false).expect("contract check");
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn corrected_ttl_requires_final_checkpoint() {
    let text = fs::read_to_string(
        root().join("docs/testing/perf-scenarios/0.71/memory-efficiency-v1.toml"),
    )
    .expect("scenario");
    assert!(xtask::memory_baseline::validate_scenario_text(&text).is_empty());
    let mutated = text.replace(
        "final_checkpoint = \"post_idle\"",
        "final_checkpoint = \"expire_or_delete\"",
    );
    let problems = xtask::memory_baseline::validate_scenario_text(&mutated);
    assert!(problems.iter().any(|problem| problem.contains("post-idle")));
}

#[test]
fn dirty_identity_and_reused_process_are_rejected() {
    let expected = "795f9493bcbb7a56aa229c59e4a717f60c654cdb";
    let problems = xtask::memory_baseline::validate_worktree_identity(
        expected,
        "0000000000000000000000000000000000000000",
        &[" M product.rs".to_owned()],
    );
    assert!(problems
        .iter()
        .any(|problem| problem.contains("source SHA")));
    assert!(problems.iter().any(|problem| problem.contains("dirty")));
    let problems =
        xtask::memory_baseline::validate_fresh_processes(&[(41, true), (41, true), (42, false)]);
    assert_eq!(problems.len(), 2);
}

#[test]
fn archive_commit_and_restored_mirror_mismatch_are_rejected() {
    let mut receipt = valid_historical_receipt();
    assert!(xtask::memory_baseline::validate_historical_receipt(&receipt).is_empty());
    receipt["commit"] = JsonValue::String("0000000000000000000000000000000000000000".to_owned());
    receipt["mirror"]["restored_manifest_sha256"] =
        JsonValue::String(format!("sha256:{}", "4".repeat(64)));
    let problems = xtask::memory_baseline::validate_historical_receipt(&receipt);
    assert!(problems.iter().any(|problem| problem.contains("commit")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("manifest mismatch")));
}

#[test]
fn typed_report_rejects_identity_unique_key_allocator_and_unavailable_lies() {
    let report = xtask::memory_baseline::diagnostic_fixture_report();
    assert!(
        xtask::memory_baseline::validate_baseline_report(&report).is_empty(),
        "valid fixture"
    );

    let mut mutated = report.clone();
    mutated["source_sha"] =
        JsonValue::String("75719b0bf5de2250cf4eb16a30073dd7429538e3".to_owned());
    mutated["unique_key_verification"]["observed"] = json!(9_999);
    mutated["allocator"]["name"] = json!("jemalloc");
    mutated["checkpoints"][0]["allocator"]["retained_bytes"] =
        json!({"value": null, "unavailable_reason": null});
    let problems = xtask::memory_baseline::validate_baseline_report(&mutated);
    for expected in [
        "frozen cohort",
        "independently verified",
        "frozen build identity",
        "schema violation",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(expected)),
            "missing {expected}: {problems:#?}"
        );
    }
}

#[test]
fn d0_admission_fails_closed_without_external_receipts() {
    let problems = xtask::memory_baseline::check_contract(&root(), true).expect("D0 check");
    assert!(
        problems.iter().any(|problem| problem.contains("receipt")),
        "D0 unexpectedly admitted: {problems:#?}"
    );
}
