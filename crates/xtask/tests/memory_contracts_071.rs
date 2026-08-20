use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn load(relative: &str) -> TomlValue {
    toml::from_str(&fs::read_to_string(root().join(relative)).expect("contract file"))
        .expect("valid TOML")
}

#[test]
fn checked_in_decision_statistics_allocator_compat_and_release_contracts_pass() {
    let decisions = load("docs/testing/memory/0.71/decision-gates.toml");
    assert!(
        xtask::memory_contracts::check_decisions(&decisions, "0.71").is_empty(),
        "decision contract"
    );
    let statistics = load("docs/testing/memory/0.71/memory-statistics-v1.toml");
    assert!(
        xtask::memory_contracts::check_statistics(&statistics, "0.71").is_empty(),
        "statistics contract"
    );
    let allocators = load("docs/testing/memory/0.71/allocator-capabilities.toml");
    assert!(
        xtask::memory_contracts::check_allocators(&allocators, "0.71").is_empty(),
        "allocator contract"
    );
    assert!(xtask::memory_contracts::check_allocator_source(&root()).is_empty());
    let compat = load("docs/testing/compat/memory-071.toml");
    assert!(
        xtask::memory_contracts::check_compat(&root(), &compat, "0.71").is_empty(),
        "compatibility contract"
    );
    let policy = load("docs/testing/memory/0.71/release-policy.toml");
    assert!(
        xtask::memory_contracts::check_release_policy(&policy, "0.71", false).is_empty(),
        "release policy"
    );
}

#[test]
fn decision_gate_rejects_candidate_baseline_and_skipped_transitions() {
    let mut decisions = load("docs/testing/memory/0.71/decision-gates.toml");
    let proposal = decisions["proposal"][0]
        .as_table_mut()
        .expect("proposal table");
    proposal.insert(
        "candidate_derived_baseline".to_owned(),
        TomlValue::Boolean(true),
    );
    proposal.insert("state".to_owned(), TomlValue::String("D3".to_owned()));
    proposal.insert(
        "classification".to_owned(),
        TomlValue::String("live ownership".to_owned()),
    );
    let problems = xtask::memory_contracts::check_decisions(&decisions, "0.71");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate-derived")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("ordered transition receipts")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("without frozen baselines")));
}

#[test]
fn statistics_gate_rejects_candidate_thresholds_and_flattering_contracts() {
    let mut statistics = load("docs/testing/memory/0.71/memory-statistics-v1.toml");
    statistics["candidate_may_amend"] = TomlValue::Boolean(true);
    statistics["minimum_repetitions"] = TomlValue::Integer(1);
    statistics["missing_row_policy"] = TomlValue::String("interpolate".to_owned());
    statistics["metric"][0]["relative_minimum_effect"] = TomlValue::Float(0.0);
    let problems = xtask::memory_contracts::check_statistics(&statistics, "0.71");
    for expected in [
        "candidate_may_amend",
        "minimum_repetitions",
        "missing_row_policy",
        "positive practical effects",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(expected)),
            "missing {expected}: {problems:?}"
        );
    }
}

#[test]
fn allocator_gate_rejects_rss_only_and_hidden_capabilities() {
    let mut allocators = load("docs/testing/memory/0.71/allocator-capabilities.toml");
    allocators["allocator"][1]["fields"] =
        TomlValue::Array(vec![TomlValue::String("resident".to_owned())]);
    allocators["allocator"][1]["unavailable_fields"] =
        TomlValue::Array(vec![TomlValue::String("active".to_owned())]);
    allocators["allocator"][1]["default"] = TomlValue::Boolean(true);
    let problems = xtask::memory_contracts::check_allocators(&allocators, "0.71");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sole portable default")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("unavailable(reason)")));
}

#[test]
fn host_fingerprint_is_order_stable_and_drift_sensitive() {
    let first = json!({"kernel": "6.8", "governor": "performance", "thp": "never"});
    let reordered = json!({"thp": "never", "kernel": "6.8", "governor": "performance"});
    assert_eq!(
        xtask::memory_contracts::canonical_json_digest(&first),
        xtask::memory_contracts::canonical_json_digest(&reordered)
    );
    for drift in [
        json!({"kernel": "6.9", "governor": "performance", "thp": "never"}),
        json!({"kernel": "6.8", "governor": "powersave", "thp": "never"}),
        json!({"kernel": "6.8", "governor": "performance", "thp": "always"}),
    ] {
        assert_ne!(
            xtask::memory_contracts::canonical_json_digest(&first),
            xtask::memory_contracts::canonical_json_digest(&drift)
        );
    }
    let profile: serde_json::Value = serde_json::from_slice(
        &fs::read(root().join("docs/testing/perf-host-profiles/memory-reference-071-v1.json"))
            .expect("profile"),
    )
    .expect("profile JSON");
    assert!(xtask::memory_contracts::check_host_profile(
        &profile,
        "0.71",
        "memory-reference-071-v1"
    )
    .is_empty());
}

#[test]
fn compatibility_gate_rejects_mixed_baseline_identity_and_missing_rollback() {
    let mut compat = load("docs/testing/compat/memory-071.toml");
    compat["baseline_commit"] = TomlValue::String("candidate-derived".to_owned());
    compat["row"]
        .as_array_mut()
        .expect("rows")
        .retain(|row| row["id"].as_str() != Some("candidate-to-baseline-compatible-rollback"));
    let problems = xtask::memory_contracts::check_compat(&root(), &compat, "0.71");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("resolves to")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate-to-baseline-compatible-rollback")));
}

#[test]
fn ship_policy_rejects_pending_work_and_unqualified_deferral() {
    let mut policy = load("docs/testing/memory/0.71/release-policy.toml");
    let problems = xtask::memory_contracts::check_release_policy(&policy, "0.71", true);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("pending optional work")));

    policy["optional_work"][0]["disposition"] = TomlValue::String("deferred".to_owned());
    policy["optional_work"][0]["reason"] = TomlValue::String("schedule pressure".to_owned());
    let problems = xtask::memory_contracts::check_release_policy(&policy, "0.71", false);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("bounded and correct")));
}

#[test]
fn allocator_selection_contract_is_fail_loud_in_source() {
    let manifest = fs::read_to_string(root().join("crates/hydracache/Cargo.toml")).unwrap();
    let source = fs::read_to_string(root().join("crates/hydracache/src/lib.rs")).unwrap();
    for feature in [
        "allocator-explicit",
        "allocator-system",
        "allocator-jemalloc",
        "allocator-mimalloc",
    ] {
        assert!(manifest.contains(feature));
    }
    assert!(source.contains("allocator-explicit requires exactly one"));
    assert!(source.contains("allocator features are mutually exclusive"));
    assert!(source.contains("HYDRACACHE_JEMALLOC"));
    assert!(source.contains("HYDRACACHE_MIMALLOC"));
}
