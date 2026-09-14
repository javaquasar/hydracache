use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn decisions() -> Value {
    toml::from_str(
        &fs::read_to_string(root().join("docs/testing/memory/0.71/decision-gates.toml"))
            .expect("decision gates"),
    )
    .expect("decision TOML")
}

#[test]
fn checked_in_d0_d3_decision_chain_is_valid() {
    let problems = xtask::memory_contracts::check_decisions(&decisions(), "0.71");
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn mixed_identity_and_candidate_derived_baseline_are_rejected() {
    let mut value = decisions();
    value["proposal"][0]["candidate_derived_baseline"] = Value::Boolean(true);
    value["proposal"][0]["transition"][1]["source_sha"] =
        Value::String("different-candidate-sha".to_owned());
    let problems = xtask::memory_contracts::check_decisions(&value, "0.71");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate-derived")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("source_sha")));
}

#[test]
fn skipped_transition_and_unattributed_candidate_result_are_rejected() {
    let mut value = decisions();
    value["proposal"][0]["state"] = Value::String("D3".to_owned());
    value["proposal"][0]["transition"]
        .as_array_mut()
        .expect("transitions")
        .remove(1);
    value["proposal"][0]["owner_ids"] = Value::Array(Vec::new());
    let problems = xtask::memory_contracts::check_decisions(&value, "0.71");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("ordered transition receipts")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("owner/stack")));
}
