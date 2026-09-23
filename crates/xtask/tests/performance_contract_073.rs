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

fn contract() -> TomlValue {
    toml::from_str(
        &fs::read_to_string(root().join("docs/testing/performance/0.73/local-screening.toml"))
            .expect("local screening contract"),
    )
    .expect("valid contract TOML")
}

fn receipt() -> serde_json::Value {
    serde_json::from_slice(
        &fs::read(
            root().join("docs/testing/performance/0.73/local-screening-receipt.example.json"),
        )
        .expect("example receipt"),
    )
    .expect("valid receipt JSON")
}

fn manifest(name: &str) -> TomlValue {
    toml::from_str(
        &fs::read_to_string(root().join("docs/testing/performance/0.73").join(name))
            .expect("performance manifest"),
    )
    .expect("valid performance TOML")
}

#[test]
fn checked_in_local_screening_contract_and_receipt_pass() {
    assert!(xtask::performance_contract::check_contract(&contract(), "0.73").is_empty());
    assert!(xtask::performance_contract::check_receipt(&receipt(), &contract()).is_empty());
    assert!(
        xtask::performance_contract::check_at_root(&root(), "0.73", None)
            .expect("check contract")
            .is_empty()
    );
}

#[test]
fn local_receipt_can_never_be_promoted_or_claim_eligible() {
    let mut value = receipt();
    value["promotable"] = json!(true);
    value["numerical_claim_eligible"] = json!(true);
    let problems = xtask::performance_contract::check_receipt(&value, &contract());
    assert!(problems
        .iter()
        .any(|problem| problem.contains("never be promotable")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("must not be numerical-claim eligible")));
}

#[test]
fn local_receipt_requires_exact_identity_and_complete_outcomes() {
    let mut value = receipt();
    value["source_sha"] = json!("main");
    value["scenario_sha256"] = json!("ABC");
    value["environment_class"] = json!("dedicated_reference");
    value["instrumentation_mode"] = json!("mystery");
    value["outcomes"]["incomplete"] = json!(1);
    let problems = xtask::performance_contract::check_receipt(&value, &contract());
    for expected in [
        "environment_class",
        "source_sha",
        "scenario_sha256",
        "instrumentation_mode",
        "account for every attempted operation",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(expected)),
            "missing {expected}: {problems:?}"
        );
    }
}

#[test]
fn contract_rejects_weakened_pair_load_and_promotion_rules() {
    let mut value = contract();
    value["promotable"] = TomlValue::Boolean(true);
    value["minimum_pairs"] = TomlValue::Integer(1);
    value["offered_load_fractions"] = TomlValue::Array(vec![TomlValue::Float(0.25)]);
    value["allowed_promotion_targets"] =
        TomlValue::Array(vec![TomlValue::String("ship".to_owned())]);
    value["allowed_instrumentation_modes"] =
        TomlValue::Array(vec![TomlValue::String("anything".to_owned())]);
    let problems = xtask::performance_contract::check_contract(&value, "0.73");
    for expected in [
        "promotable",
        "at least three pairs",
        "offered_load_fractions",
        "must not declare promotion targets",
        "allowed_instrumentation_modes",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(expected)),
            "missing {expected}: {problems:?}"
        );
    }
}

#[test]
fn receipt_schema_rejects_unregistered_fields() {
    let mut value = receipt();
    value["ship_evidence_eligible"] = json!(true);
    let path = std::env::temp_dir().join(format!(
        "hydracache-performance-073-schema-canary-{}.json",
        std::process::id()
    ));
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let problems = xtask::performance_contract::check_at_root(&root(), "0.73", Some(&path))
        .expect("check mutated receipt");
    let _ = fs::remove_file(path);
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("schema violation")),
        "unexpected schema result: {problems:?}"
    );
}

#[test]
fn baseline_identity_rejects_a_wrong_annotated_tag_object() {
    let mut value = manifest("baseline-identities.toml");
    value["published_baseline"]["tag_object_sha"] = TomlValue::String("f".repeat(40));
    let problems = xtask::performance_contract::check_baseline_identities(&root(), &value, "0.73")
        .expect("check baseline identities");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("tag object SHA")));
}

#[test]
fn post_tag_delta_rejects_an_unclassified_path() {
    let mut value = manifest("post-tag-delta.toml");
    value["paths"]
        .as_array_mut()
        .expect("path ledger")
        .remove(0);
    let problems = xtask::performance_contract::check_post_tag_delta(&root(), &value, "0.73")
        .expect("check post-tag delta");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("unclassified")));
}

#[test]
fn scenario_matrix_requires_every_w2_through_w9_surface() {
    let mut value = manifest("scenario-matrix.toml");
    value["surfaces"]
        .as_array_mut()
        .expect("surface matrix")
        .retain(|surface| surface["work_item"].as_str() != Some("W9"));
    let problems = xtask::performance_contract::check_scenario_matrix(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("omits mandatory W9")));
}

#[test]
fn scenario_matrix_rejects_candidate_data_before_i73_freeze() {
    let mut value = manifest("scenario-matrix.toml");
    value["state"] = TomlValue::String("frozen".to_owned());
    value["candidate_measurement_allowed"] = TomlValue::Boolean(true);
    value["stable_rates_state"] = TomlValue::String("selected".to_owned());
    let problems = xtask::performance_contract::check_scenario_matrix(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate-blocking")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("stable_rates_state")));
}

#[test]
fn scenario_matrix_rejects_reweighted_mixed_workload() {
    let mut value = manifest("scenario-matrix.toml");
    value["mixed_runtime"]["weights_percent"]["resp"] = TomlValue::Integer(25);
    let problems = xtask::performance_contract::check_scenario_matrix(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("weight resp must be 30%")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("do not total 100%")));
}

#[test]
fn instrumentation_overhead_rejects_weakened_limits() {
    let mut value = manifest("instrumentation-overhead.toml");
    value["throughput_regression_limit"] = TomlValue::Float(0.05);
    value["minimum_qualification_pairs"] = TomlValue::Integer(2);
    let problems = xtask::performance_contract::check_instrumentation_overhead(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("weakens inherited regression limits")));
}

#[test]
fn instrumentation_overhead_cannot_freeze_i73_with_unmeasured_limits() {
    let mut value = manifest("instrumentation-overhead.toml");
    value["state"] = TomlValue::String("qualified".to_owned());
    value["i73_freeze_allowed"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_instrumentation_overhead(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate-blocking pilot")));
}

#[test]
fn local_overhead_screening_cannot_be_promoted_or_erase_the_blocker() {
    let mut value = manifest("local-overhead-screening-307b3500.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["decision"] = TomlValue::String("accepted".to_owned());
    let problems = xtask::performance_contract::check_local_overhead_screening(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("non-promotable blocker")));
}
