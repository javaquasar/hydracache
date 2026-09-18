use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn matrix() -> Value {
    toml::from_str(
        &fs::read_to_string(root().join("docs/testing/memory/0.71/allocator-capabilities.toml"))
            .expect("allocator matrix"),
    )
    .expect("allocator TOML")
}

#[test]
fn checked_in_allocator_capability_and_source_matrix_is_closed() {
    let problems = xtask::memory_contracts::check_allocators(&matrix(), "0.71");
    assert!(problems.is_empty(), "{problems:?}");
    let source_problems = xtask::memory_contracts::check_allocator_source(&root());
    assert!(source_problems.is_empty(), "{source_problems:?}");
}

#[test]
fn rss_only_candidate_and_second_default_are_rejected() {
    let mut value = matrix();
    value["allocator"][1]["default"] = Value::Boolean(true);
    value["allocator"][1]["fields"] = Value::Array(vec![Value::String("resident".to_owned())]);
    let problems = xtask::memory_contracts::check_allocators(&value, "0.71");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sole portable default")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("allocator jemalloc") && problem.contains("active")));
}

#[test]
fn allocator_change_remains_deferred_without_same_host_ab() {
    let policy: Value = toml::from_str(
        &fs::read_to_string(root().join("docs/testing/memory/0.71/release-policy.toml"))
            .expect("policy"),
    )
    .expect("policy TOML");
    let row = policy["optional_work"]
        .as_array()
        .expect("optional work")
        .iter()
        .find(|row| row["id"].as_str() == Some("W8"))
        .expect("W8");
    assert_eq!(row["disposition"].as_str(), Some("deferred"));
    assert!(row["next_evidence"]
        .as_str()
        .expect("next evidence")
        .contains("same-host"));
}
