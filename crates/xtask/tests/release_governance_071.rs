use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn policy() -> Value {
    toml::from_str(
        &fs::read_to_string(root().join("docs/testing/memory/0.71/release-policy.toml"))
            .expect("release policy"),
    )
    .expect("policy TOML")
}

#[test]
fn complete_foundation_with_evidenced_deferrals_allows_no_win_ship() {
    let problems = xtask::memory_contracts::check_release_policy(&policy(), "0.71", true);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn missing_foundation_or_pending_optional_work_blocks_ship() {
    let mut value = policy();
    value["mandatory_foundation"]
        .as_array_mut()
        .expect("foundation")
        .retain(|item| item.as_str() != Some("W13-governance"));
    value["optional_work"][0]["disposition"] = Value::String("pending-evidence".to_owned());
    let problems = xtask::memory_contracts::check_release_policy(&value, "0.71", true);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("W13-governance")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("refuses pending optional work")));
}

#[test]
fn safety_defect_cannot_be_deferred_or_hidden_as_a_schedule_choice() {
    let mut value = policy();
    value["deferred_safety_defects_allowed"] = Value::Boolean(true);
    value["optional_work"][0]["reason"] = Value::String("schedule pressure".to_owned());
    let problems = xtask::memory_contracts::check_release_policy(&value, "0.71", true);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("deferred_safety_defects_allowed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("bounded and correct")));
}
