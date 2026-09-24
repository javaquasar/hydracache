use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn profile() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("docs/testing/perf-host-profiles/memory-reference-071-v1.json"))
            .expect("host profile"),
    )
    .expect("profile JSON")
}

fn performance_profile_073() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("docs/testing/perf-host-profiles/performance-reference-073-v1.json"))
            .expect("0.73 host profile"),
    )
    .expect("0.73 profile JSON")
}

#[test]
fn checked_in_host_profile_is_complete() {
    let problems =
        xtask::memory_contracts::check_host_profile(&profile(), "0.71", "memory-reference-071-v1");
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn checked_in_performance_profile_073_is_complete_and_fresh() {
    let profile = performance_profile_073();
    let problems = xtask::memory_contracts::check_host_profile(
        &profile,
        "0.73",
        "performance-reference-073-v1",
    );
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(
        profile["completed_bootstrap_admission"],
        "fresh-0.73-admission-required"
    );
    assert!(profile["ineligible_platforms"]
        .as_array()
        .expect("ineligible platforms")
        .iter()
        .any(|value| value == "shared-self-hosted"));
    assert!(profile["required_tools"]
        .as_array()
        .expect("required tools")
        .iter()
        .any(|value| value == "pidstat"));

    let workflow =
        fs::read_to_string(root().join(".github/workflows/performance-host-admission-073.yml"))
            .expect("0.73 host admission workflow");
    for required in [
        "runs-on: [self-hosted, linux, x64, hydracache-release]",
        "environment: performance-reference-073",
        "group: performance-reference-073-host",
        "preflight.json",
        "postflight.json",
        "host fingerprint drifted across admission",
        "candidate_measurement_authorized\": False",
    ] {
        assert!(workflow.contains(required), "workflow omitted {required}");
    }
}

#[test]
fn fingerprint_is_order_stable_and_sensitive_to_mutable_drift() {
    let before = json!({"kernel":"6.8", "governor":"performance", "thp":"never"});
    let reordered = json!({"thp":"never", "kernel":"6.8", "governor":"performance"});
    let drifted = json!({"kernel":"6.8", "governor":"powersave", "thp":"never"});
    assert_eq!(
        xtask::memory_contracts::canonical_json_digest(&before),
        xtask::memory_contracts::canonical_json_digest(&reordered)
    );
    assert_ne!(
        xtask::memory_contracts::canonical_json_digest(&before),
        xtask::memory_contracts::canonical_json_digest(&drifted)
    );
}

#[test]
fn profile_without_lease_or_bootstrap_is_rejected() {
    let mut value = profile();
    value["lease_required"] = Value::Bool(false);
    value
        .as_object_mut()
        .expect("object")
        .remove("completed_bootstrap_admission");
    let problems =
        xtask::memory_contracts::check_host_profile(&value, "0.71", "memory-reference-071-v1");
    assert!(problems.iter().any(|problem| problem.contains("lease")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("completed_bootstrap_admission")));
}
