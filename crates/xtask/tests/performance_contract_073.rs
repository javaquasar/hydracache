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
fn baseline_identity_rejects_a_changed_i73_freeze() {
    let mut value = manifest("baseline-identities.toml");
    value["instrumented_baseline"]["source_sha"] = TomlValue::String("f".repeat(40));
    value["instrumented_baseline"]["frozen_d3_rates_per_second"] =
        TomlValue::Array(vec![TomlValue::Integer(5_000)]);
    let problems = xtask::performance_contract::check_baseline_identities(&root(), &value, "0.73")
        .expect("check baseline identities");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exact-source freeze")));
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
fn scenario_matrix_rejects_reopening_the_admitted_i73_freeze() {
    let mut value = manifest("scenario-matrix.toml");
    value["state"] = TomlValue::String("pilot".to_owned());
    value["candidate_measurement_allowed"] = TomlValue::Boolean(false);
    value["baseline_source_sha"] = TomlValue::String(String::new());
    value["stable_rates_state"] = TomlValue::String("unmeasured".to_owned());
    let problems = xtask::performance_contract::check_scenario_matrix(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("admitted I73 freeze")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("bind frozen I73")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("calibration, window, or rates")));
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
fn instrumentation_overhead_cannot_freeze_i73_before_production_qualification() {
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

#[test]
fn local_overhead_isolation_cannot_claim_correctness_or_promotion() {
    let mut value = manifest("local-overhead-isolation-5264d96c.toml");
    value["counter_correctness_eligible"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_local_overhead_isolation(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("diagnostic and non-promotable")));
}

#[test]
fn local_noop_listener_screen_cannot_claim_correctness_or_promotion() {
    let mut value = manifest("local-overhead-listener-noop-5e101e69.toml");
    value["counter_correctness_eligible"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_local_overhead_listener_noop(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("diagnostic and non-promotable")));
}

#[test]
fn notification_feasibility_cannot_authorize_product_semantics_or_promotion() {
    let mut value = manifest("notification-feasibility-bf9f1382.toml");
    value["product_semantics_eligible"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    value["decision"] = TomlValue::String("migrate-to-sync".to_owned());
    let problems = xtask::performance_contract::check_notification_feasibility(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("must remain diagnostic")));
}

#[test]
fn observer_requirements_cannot_detach_from_d2_or_drop_ordering_falsifiers() {
    let mut value = manifest("notification-observer-requirements.toml");
    value["d2_authorized"] = TomlValue::Boolean(false);
    value["candidate_measurements_allowed"] = TomlValue::Boolean(true);
    value["required_falsifiers"] = TomlValue::Array(Vec::new());
    let problems =
        xtask::performance_contract::check_notification_observer_requirements(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("must bind D2 to the pinned fork")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("incomplete required_falsifiers")));
}

#[test]
fn observer_prototype_cannot_be_promoted_as_a_moka_result() {
    let mut value = manifest("notification-observer-prototype-9a2ca114.toml");
    value["product_semantics_eligible"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    value["decision"] = TomlValue::String("moka-seam-proven".to_owned());
    let problems =
        xtask::performance_contract::check_notification_observer_prototype(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("must remain diagnostic")));
}

#[test]
fn moka_observer_spike_cannot_hide_unresolved_removal_cost() {
    let mut value = manifest("moka-observer-spike-5d560170.toml");
    value["product_semantics_eligible"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    value["verified_causes"] = TomlValue::Array(Vec::new());
    let problems = xtask::performance_contract::check_moka_observer_spike(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("must remain diagnostic")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("omits explicit cause")));
}

#[test]
fn direct_moka_observer_cannot_self_authorize_d2_or_drop_ordering_proof() {
    let mut value = manifest("moka-observer-direct-779849b6.toml");
    value["d2_authorized"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    value["versioned_replacement_ordering_verified"] = TomlValue::Boolean(false);
    let problems = xtask::performance_contract::check_moka_observer_direct(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("must remain lab-only")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("versioned replacement ordering")));
}

#[test]
fn d2_review_cannot_enable_measurement_or_use_candidate_thresholds() {
    let mut value = manifest("notification-observer-d2-review.toml");
    value["reviewer"] = TomlValue::String("proposal-author".to_owned());
    value["reviewer_independent"] = TomlValue::Boolean(true);
    value["d2_authorized"] = TomlValue::Boolean(false);
    value["candidate_measurements_allowed"] = TomlValue::Boolean(true);
    value["threshold_freeze_commit"] = TomlValue::String("rewritten".to_owned());
    value["candidate_data_used_for_thresholds"] = TomlValue::Boolean(true);
    let candidate = value["candidate_evidence_excluded_from_threshold_derivation"][0].clone();
    value["baseline_evidence"]
        .as_array_mut()
        .expect("baseline evidence")
        .push(candidate);
    value["required_correctness_falsifiers"] = TomlValue::Array(
        (0..10)
            .map(|index| TomlValue::String(format!("generic falsifier {index}")))
            .collect(),
    );
    let problems =
        xtask::performance_contract::check_notification_observer_d2_review(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("authorize only product integration")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("threshold_freeze_commit")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("mixes candidate evidence")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("omits panic falsifier")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("omits reentrancy falsifier")));
}

#[test]
fn single_maintainer_policy_cannot_claim_independence_or_unpin_dependency() {
    let mut value = manifest("single-maintainer-review-policy.toml");
    value["independent_review_claim_allowed"] = TomlValue::Boolean(true);
    value["dependency_decision"] = TomlValue::String("pending".to_owned());
    value["d2_authorized"] = TomlValue::Boolean(false);
    value["threshold_freeze_commit"] = TomlValue::String("main".to_owned());
    let problems =
        xtask::performance_contract::check_single_maintainer_review_policy(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("bind D2 to the reviewed dependency")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("threshold_freeze_commit")));
}

#[test]
fn frozen_i73_instrumentation_cannot_be_reopened_as_a_candidate() {
    let mut value = manifest("proposal-registry.toml");
    value["proposals"][0]["candidate_measurements_allowed"] = TomlValue::Boolean(true);
    value["proposals"][0]["d3_measurement_required"] = TomlValue::Boolean(true);
    value["proposals"][0]["dependency_decision"] =
        TomlValue::String("hydracache/post-removal-observer-0.12.15".to_owned());
    let problems = xtask::performance_contract::check_proposal_registry(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("inside frozen I73")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("must reference the pinned fork decision")));
}

#[test]
fn moka_fork_decision_requires_exact_revision_and_keeps_measurement_closed() {
    let mut value = manifest("moka-fork-decision-352e53fa.toml");
    value["candidate_measurements_allowed"] = TomlValue::Boolean(true);
    value["source"]["revision"] =
        TomlValue::String("hydracache/post-removal-observer-0.12.15".to_owned());
    value["source"]["remote_revision_verified"] = TomlValue::Boolean(false);
    value["upstream"]["submission_state"] = TomlValue::String("submitted".to_owned());
    let problems = xtask::performance_contract::check_moka_fork_decision(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("not candidate measurement")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("source revision must be")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("source integrity")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("absent upstream review")));
}

#[test]
fn product_admission_cannot_promote_local_results_or_hide_scope_and_falsifier_gaps() {
    let mut value = manifest("notification-observer-product-73fc38a1.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["dedicated_candidate_measurement_allowed"] = TomlValue::Boolean(true);
    value["thresholds_changed"] = TomlValue::Boolean(true);
    value["changed_files"] = TomlValue::Array(Vec::new());
    value["falsifier"]
        .as_array_mut()
        .expect("falsifier array")
        .retain(|item| item["id"].as_str() != Some("rollback"));
    let problems = xtask::performance_contract::check_notification_observer_product(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("only non-promotable local screening")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("changed-file ledger")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("rollback falsifier")));
}

#[test]
fn local_product_screen_cannot_be_promoted_or_weaken_the_frozen_fill_gate() {
    let mut value = manifest("local-observer-product-screening-06a8bd95.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["thresholds_changed"] = TomlValue::Boolean(true);
    value["candidate_screening_sha256"] = TomlValue::String("f".repeat(64));
    let fill = value["metric"]
        .as_array_mut()
        .expect("metric array")
        .iter_mut()
        .find(|metric| metric["name"].as_str() == Some("fill_allocated_bytes_per_operation"))
        .expect("fill metric");
    fill["relative_change"] = TomlValue::Float(-0.10);
    fill["minimum_relative_improvement"] = TomlValue::Float(0.10);
    let peak = value["metric"]
        .as_array_mut()
        .expect("metric array")
        .iter_mut()
        .find(|metric| metric["name"].as_str() == Some("peak_rss_delta_bytes"))
        .expect("peak RSS metric");
    peak["candidate_production_over_off"] = TomlValue::Float(0.10);
    peak["candidate_absolute_over_off"] = TomlValue::Float(2_097_152.0);
    let problems =
        xtask::performance_contract::check_local_observer_product_screening(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("must remain non-promotable")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("does not match retained evidence")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("frozen 15% fill gate")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("peak_rss_delta_bytes guard")));
}

#[test]
fn statistics_cannot_weaken_pairs_confidence_or_goodput_guard() {
    let mut value = manifest("statistics.toml");
    value["minimum_admitted_pairs"] = TomlValue::Integer(3);
    value["regression_budget"][0]["maximum_relative_regression"] = TomlValue::Float(0.05);
    let problems = xtask::performance_contract::check_statistics(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sample or confidence")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("goodput_operations_per_second")));
}

#[test]
fn admitted_host_cannot_drop_the_i73_freeze_or_reuse_an_old_identity() {
    let mut value = manifest("host-profile.toml");
    value["candidate_measurements_allowed"] = TomlValue::Boolean(false);
    value["baseline_freeze_evidence"] = TomlValue::String("pending".to_owned());
    value["identity_reuse_from_071_allowed"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_host_profile(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("bind the I73 freeze")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity_reuse_from_071_allowed must be false")));
}

#[test]
fn host_admission_cannot_hide_failed_attempts_or_authorize_candidate_measurement() {
    let mut value = manifest("host-admission-3ba09fcc.toml");
    value["candidate_measurement_authorized"] = TomlValue::Boolean(true);
    value["preflight_sha256"] = TomlValue::String("f".repeat(64));
    value["attempt"].as_array_mut().expect("attempts").remove(0);
    let problems = xtask::performance_contract::check_host_admission(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_measurement_authorized must remain false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("retained packet")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("retain both failed attempts")));
}

#[test]
fn baseline_pilot_cannot_observe_candidate_or_move_the_rate_grid() {
    let mut value = manifest("baseline-pilot-contract.toml");
    value["candidate_data_allowed"] = TomlValue::Boolean(true);
    value["offered_rates_per_second"] = TomlValue::Array(vec![TomlValue::Integer(20_000)]);
    value["maximum_goodput_regression"] = TomlValue::Float(0.10);
    let problems = xtask::performance_contract::check_baseline_pilot_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_data_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sample or rate grid")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("maximum_goodput_regression")));
}

#[test]
fn insufficient_baseline_cannot_freeze_i73_or_hide_the_retained_attempts() {
    let mut value = manifest("baseline-pilot-insufficient-4ba93a1a.toml");
    value["i73_freeze_eligible"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(20);
    value["artifact_sha256"] = TomlValue::String("missing".to_owned());
    let problems = xtask::performance_contract::check_baseline_pilot_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("i73_freeze_eligible must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("24 attempts and zero stable rates")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("artifact_sha256 is not SHA-256")));
}

#[test]
fn cpu_attribution_cannot_be_promoted_or_change_its_modes() {
    let mut value = manifest("cpu-attribution-contract.toml");
    value["acceptance_decision_allowed"] = TomlValue::Boolean(true);
    value["instrumentation_modes"] =
        TomlValue::Array(vec![TomlValue::String("production".to_owned())]);
    let problems = xtask::performance_contract::check_cpu_attribution_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("acceptance_decision_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("modes or sample grid")));
}

#[test]
fn cpu_attribution_evidence_cannot_authorize_measurement_or_rewrite_the_result() {
    let mut value = manifest("cpu-attribution-ed339846.toml");
    value["candidate_measurement_authorized"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(19);
    value["comparison"][3]["cpu_relative_delta"] = TomlValue::Float(0.0);
    let problems = xtask::performance_contract::check_cpu_attribution_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| { problem.contains("candidate_measurement_authorized must be false") }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("complete 20-attempt block")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("off_to_production")));
}

#[test]
fn removal_drain_fast_path_can_only_authorize_the_unchanged_baseline_repeat() {
    let mut value = manifest("removal-drain-fast-path-affe4390.toml");
    value["candidate_measurement_authorized"] = TomlValue::Boolean(true);
    value["baseline_only_repeat_allowed"] = TomlValue::Boolean(false);
    value["falsifier"]
        .as_array_mut()
        .expect("falsifiers")
        .remove(0);
    let problems = xtask::performance_contract::check_removal_drain_fast_path(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| { problem.contains("candidate_measurement_authorized must be false") }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("bounded repeat decision")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("empty-drain-lock-elision")));
}

#[test]
fn fast_path_baseline_cannot_freeze_i73_with_only_two_rates() {
    let mut value = manifest("baseline-pilot-insufficient-9bba762c.toml");
    value["i73_freeze_eligible"] = TomlValue::Boolean(true);
    value["stable_rates"] = TomlValue::Array(vec![TomlValue::Integer(20_000)]);
    let problems =
        xtask::performance_contract::check_baseline_pilot_fast_path_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("i73_freeze_eligible must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exactly two stable rates")));
}

#[test]
fn removal_queue_contract_cannot_block_grow_or_skip_local_gates() {
    let mut value = manifest("removal-queue-contract.toml");
    value["queue_capacity"] = TomlValue::Integer(8_192);
    value["callback_may_block"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed_before_local_gates"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_removal_queue_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity or bound changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("callback_may_block must be false")));
    assert!(problems.iter().any(|problem| {
        problem.contains("dedicated_host_run_allowed_before_local_gates must be false")
    }));
}

#[test]
fn removal_queue_product_cannot_claim_performance_or_relax_exactness() {
    let mut value = manifest("removal-queue-product-daffd71b.toml");
    value["queue_capacity"] = TomlValue::Integer(8_192);
    value["accepted_acknowledged_barrier_preserved"] = TomlValue::Boolean(false);
    value["numerical_claim_eligible"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_removal_queue_product(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity or bound changed")));
    assert!(problems.iter().any(|problem| {
        problem.contains("accepted_acknowledged_barrier_preserved must be true")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("numerical_claim_eligible must be false")));
}

#[test]
fn array_queue_baseline_cannot_freeze_i73_or_hide_the_cancelled_duplicate() {
    let mut value = manifest("baseline-pilot-insufficient-90a40510.toml");
    value["i73_freeze_eligible"] = TomlValue::Boolean(true);
    value["duplicate_dispatch_state"] = TomlValue::String("completed".to_owned());
    let problems =
        xtask::performance_contract::check_baseline_pilot_array_queue_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("i73_freeze_eligible must be false")));
}

#[test]
fn removal_sequence_contract_cannot_hide_overflow_or_skip_local_gates() {
    let mut value = manifest("removal-sequence-contract.toml");
    value["overflow_may_be_silently_accepted"] = TomlValue::Boolean(true);
    value["overflow_must_mark_observer_dirty"] = TomlValue::Boolean(false);
    value["dedicated_host_run_allowed_before_local_gates"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_removal_sequence_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("overflow_may_be_silently_accepted must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("overflow_must_mark_observer_dirty must be true")));
    assert!(problems.iter().any(|problem| {
        problem.contains("dedicated_host_run_allowed_before_local_gates must be false")
    }));
}

#[test]
fn removal_sequence_product_cannot_claim_performance_or_weaken_overflow_recovery() {
    let mut value = manifest("removal-sequence-product-549fbaeb.toml");
    value["overflow_recovery_requires_reconciliation"] = TomlValue::Boolean(false);
    value["numerical_claim_eligible"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_removal_sequence_product(&value, "0.73");
    assert!(problems.iter().any(|problem| {
        problem.contains("overflow_recovery_requires_reconciliation must be true")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("numerical_claim_eligible must be false")));
}

#[test]
fn sequence_baseline_cannot_claim_a_code_effect_or_hide_its_single_stable_rate() {
    let mut value = manifest("baseline-pilot-insufficient-862c9015.toml");
    value["code_effect_claimed"] = TomlValue::Boolean(true);
    value["stable_rates"] =
        TomlValue::Array(vec![TomlValue::Integer(10_000), TomlValue::Integer(20_000)]);
    let problems =
        xtask::performance_contract::check_baseline_pilot_sequence_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("code_effect_claimed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exactly one stable rate")));
}

#[test]
fn baseline_pilot_v2_cannot_reduce_volume_or_relax_the_original_ceilings() {
    let mut value = manifest("baseline-pilot-v2-contract.toml");
    value["repeats_per_rate_and_mode"] = TomlValue::Integer(3);
    value["window_seconds"] = TomlValue::Integer(5);
    value["maximum_cpu_per_operation_regression"] = TomlValue::Float(0.10);
    let problems = xtask::performance_contract::check_baseline_pilot_v2_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume, rates, or unchanged ceilings changed")));
}

#[test]
fn baseline_pilot_v2_product_cannot_restore_push_or_independent_medians() {
    let mut value = manifest("baseline-pilot-v2-product-4979e2e1.toml");
    value["push_trigger_present"] = TomlValue::Boolean(true);
    value["independent_mode_medians_used_for_decision"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_baseline_pilot_v2_product(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("push_trigger_present must be false")));
    assert!(problems.iter().any(|problem| {
        problem.contains("independent_mode_medians_used_for_decision must be false")
    }));
}

#[test]
fn baseline_pilot_v2_evidence_cannot_freeze_i73_with_two_rates() {
    let mut value = manifest("baseline-pilot-v2-insufficient-72d491ac.toml");
    value["i73_freeze_eligible"] = TomlValue::Boolean(true);
    value["stable_rates"] = TomlValue::Array(vec![
        TomlValue::Integer(2_500),
        TomlValue::Integer(5_000),
        TomlValue::Integer(20_000),
    ]);
    let problems = xtask::performance_contract::check_baseline_pilot_v2_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("i73_freeze_eligible must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exactly two stable rates")));
}

#[test]
fn memory_counter_contract_cannot_touch_epoch_or_hide_underflow() {
    let mut value = manifest("memory-counter-atomic-contract.toml");
    value["epoch_algorithm_changed"] = TomlValue::Boolean(true);
    value["underflow_must_fault"] = TomlValue::Boolean(false);
    value["dedicated_host_run_allowed_before_local_gates"] = TomlValue::Boolean(true);
    let problems =
        xtask::performance_contract::check_memory_counter_atomic_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("epoch_algorithm_changed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("underflow_must_fault must be true")));
    assert!(problems.iter().any(|problem| {
        problem.contains("dedicated_host_run_allowed_before_local_gates must be false")
    }));
}

#[test]
fn memory_counter_product_cannot_claim_performance_or_weaken_faults() {
    let mut value = manifest("memory-counter-atomic-product-a205ce0a.toml");
    value["numerical_claim_eligible"] = TomlValue::Boolean(true);
    value["underflow_faults_exact_capture"] = TomlValue::Boolean(false);
    value["version_algorithm_changed"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_memory_counter_atomic_product(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("numerical_claim_eligible must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("underflow_faults_exact_capture must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("version_algorithm_changed must be false")));
}

#[test]
fn counter_fast_path_baseline_cannot_freeze_or_authorize_a_repeat() {
    let mut value = manifest("baseline-pilot-v2-insufficient-2daccb47.toml");
    value["i73_freeze_eligible"] = TomlValue::Boolean(true);
    value["manual_repeat_authorized"] = TomlValue::Boolean(true);
    value["stable_rates"] = TomlValue::Array(vec![
        TomlValue::Integer(5_000),
        TomlValue::Integer(10_000),
        TomlValue::Integer(20_000),
    ]);
    let problems =
        xtask::performance_contract::check_baseline_pilot_v2_counter_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("i73_freeze_eligible must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("manual_repeat_authorized must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exactly one stable rate")));
}

#[test]
fn allocation_attribution_cannot_reduce_volume_or_claim_cpu() {
    let mut value = manifest("observer-allocation-attribution-contract.toml");
    value["repeats_per_scenario_and_mode"] = TomlValue::Integer(3);
    value["cpu_claims_allowed"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed"] = TomlValue::Boolean(true);
    let problems =
        xtask::performance_contract::check_observer_allocation_attribution_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("cpu_claims_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume or interpretation changed")));
}

#[test]
fn allocation_attribution_evidence_cannot_promote_or_authorize_host_repeat() {
    let mut value = manifest("observer-allocation-attribution-2c37d2f1.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["dedicated_host_repeat_authorized"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(119);
    let problems =
        xtask::performance_contract::check_observer_allocation_attribution_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_repeat_authorized must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("evidence volume changed")));
}

#[test]
fn shared_entry_tags_cannot_change_estimator_or_authorize_host() {
    let mut value = manifest("shared-entry-tags-contract.toml");
    value["retained_estimator_values_changed"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed"] = TomlValue::Boolean(true);
    value["cleanup_ticket_shares_same_tags"] = TomlValue::Boolean(false);
    let problems = xtask::performance_contract::check_shared_entry_tags_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("retained_estimator_values_changed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("cleanup_ticket_shares_same_tags must be true")));
}

#[test]
fn shared_entry_tags_product_cannot_promote_or_weaken_local_evidence() {
    let mut value = manifest("shared-entry-tags-product-947e624d.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["cpu_claims_allowed"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(119);
    let problems = xtask::performance_contract::check_shared_entry_tags_product(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("cpu_claims_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("allocation evidence changed")));
}

#[test]
fn baseline_v2_freeze_cannot_admit_candidate_data_or_drop_stable_rates() {
    let mut value = manifest("baseline-pilot-v2-passed-e757556d.toml");
    value["candidate_data_present"] = TomlValue::Boolean(true);
    value["stable_rates"] =
        TomlValue::Array(vec![TomlValue::Integer(5_000), TomlValue::Integer(10_000)]);
    value["silent_retry_allowed"] = TomlValue::Boolean(true);
    let problems =
        xtask::performance_contract::check_baseline_pilot_v2_freeze_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_data_present must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("silent_retry_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume or rate decision changed")));
}

#[test]
fn w1_owner_classification_cannot_skip_a_surface_or_authorize_product_work() {
    let mut value = manifest("w1-owner-classification-contract.toml");
    value["product_mutation_allowed"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed"] = TomlValue::Boolean(true);
    value["surface"]
        .as_array_mut()
        .expect("W1 surface array")
        .retain(|surface| surface["work_item"].as_str() != Some("W8"));
    let problems =
        xtask::performance_contract::check_w1_owner_classification_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("product_mutation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("all W2--W9 surfaces")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exactly one W8/allocator")));
}

#[test]
fn w2_expiry_sweep_profile_cannot_enable_product_or_host_work() {
    let mut value = manifest("w2-expiry-sweep-profile-contract.toml");
    value["product_mutation_allowed"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed"] = TomlValue::Boolean(true);
    value["scan_limit"] = TomlValue::Integer(512);
    let problems =
        xtask::performance_contract::check_w2_expiry_sweep_profile_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("product_mutation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume or metrics changed")));
}

#[test]
fn w2_expiry_sweep_evidence_cannot_promote_or_change_copy_volume() {
    let mut value = manifest("w2-expiry-sweep-profile-e4a61d9f.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["candidate_implementation_allowed"] = TomlValue::Boolean(true);
    value["scenario"][0]["gross_allocated_bytes"] = TomlValue::Integer(1);
    let problems =
        xtask::performance_contract::check_w2_expiry_sweep_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_implementation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("none-expired result changed")));
}

#[test]
fn w2_borrowed_expiry_scan_contract_cannot_admit_candidate_data_or_host_run() {
    let mut value = manifest("w2-borrowed-expiry-scan-contract.toml");
    value["candidate_data_present"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed"] = TomlValue::Boolean(true);
    value["repeats_per_scenario"] = TomlValue::Integer(1);
    let problems =
        xtask::performance_contract::check_w2_borrowed_expiry_scan_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_data_present must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate bounds changed")));
}

#[test]
fn w2_borrowed_expiry_scan_evidence_cannot_promote_or_change_result() {
    let mut value = manifest("w2-borrowed-expiry-scan-a80839fd.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed"] = TomlValue::Boolean(true);
    value["scenario"][0]["candidate_gross_allocated_bytes"] = TomlValue::Integer(97);
    let problems =
        xtask::performance_contract::check_w2_borrowed_expiry_scan_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("none-expired result changed")));
}

#[test]
fn w5_hc2_event_copy_profile_cannot_enable_product_or_host_work() {
    let mut value = manifest("w5-hc2-event-copy-profile-contract.toml");
    value["product_mutation_allowed"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed"] = TomlValue::Boolean(true);
    value["fanouts"] = TomlValue::Array(vec![TomlValue::Integer(1)]);
    let problems =
        xtask::performance_contract::check_w5_hc2_event_copy_profile_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("product_mutation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume changed")));
}

#[test]
fn w5_hc2_event_copy_evidence_cannot_promote_or_change_result() {
    let mut value = manifest("w5-hc2-event-copy-profile-02777da6.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["candidate_implementation_allowed"] = TomlValue::Boolean(true);
    value["scenario"][3]["gross_allocated_bytes"] = TomlValue::Integer(1);
    let problems =
        xtask::performance_contract::check_w5_hc2_event_copy_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_implementation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("fanout-16-value-4096 changed")));
}

#[test]
fn w5_shared_event_bytes_contract_cannot_admit_data_or_host_run() {
    let mut value = manifest("w5-shared-event-bytes-contract.toml");
    value["candidate_data_present"] = TomlValue::Boolean(true);
    value["dedicated_host_run_allowed"] = TomlValue::Boolean(true);
    value["forbidden_changes"] = TomlValue::Array(Vec::new());
    let problems =
        xtask::performance_contract::check_w5_shared_event_bytes_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_data_present must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated_host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate bounds changed")));
}
