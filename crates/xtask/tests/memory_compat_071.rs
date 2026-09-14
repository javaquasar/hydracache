use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn contract() -> Value {
    toml::from_str(
        &fs::read_to_string(root().join("docs/testing/compat/memory-071.toml"))
            .expect("compatibility contract"),
    )
    .expect("compat TOML")
}

#[test]
fn checked_in_upgrade_rollback_and_mixed_version_matrix_is_valid() {
    let problems = xtask::memory_contracts::check_compat(&root(), &contract(), "0.71");
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn missing_rollback_and_wire_generation_are_rejected() {
    let mut value = contract();
    value["row"]
        .as_array_mut()
        .expect("rows")
        .retain(|row| row["id"].as_str() != Some("candidate-to-baseline-compatible-rollback"));
    value["row"][0]["wire_generations"] = Value::Array(vec![Value::String("HC/1".to_owned())]);
    let problems = xtask::memory_contracts::check_compat(&root(), &value, "0.71");
    assert!(problems.iter().any(|problem| problem.contains("rollback")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("HC/1 and HC/2")));
}
