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

fn release_canary_enabled() -> Option<String> {
    std::env::var("HYDRACACHE_CANARY_DEFECT")
        .ok()
        .filter(|value| value.starts_with("PERF73-"))
}

#[test]
fn release_073_governance_contract_is_fail_closed() {
    for path in [
        "docs/testing/canary-registry-0.73.json",
        "docs/testing/release-evidence/0.73.toml",
        "docs/testing/performance/0.73/release-coverage.toml",
        "docs/testing/performance/0.73/w10-long-run-v2-qualification-passed-36532416869.toml",
        "docs/testing/perf-artifacts/0.73/long-run-qualification-36532416869/manifest.json",
    ] {
        assert!(
            root().join(path).is_file(),
            "missing 0.73 release evidence {path}"
        );
    }
    assert!(
        xtask::performance_contract::check_at_root(&root(), "0.73", None)
            .expect("check 0.73 performance closure")
            .is_empty(),
        "the checked-in 0.73 evidence chain must remain internally consistent"
    );
}

#[test]
fn canary_release_073_rejects_missing_work_item_evidence() {
    let Some(defect) = release_canary_enabled() else {
        return;
    };
    let work_item = defect
        .strip_prefix("PERF73-")
        .expect("release canary prefix");
    assert!(
        matches!(
            work_item,
            "W0" | "W1" | "W2" | "W3" | "W4" | "W5" | "W6" | "W7" | "W8" | "W9" | "W10" | "W11"
        ),
        "unknown 0.73 release canary {defect}"
    );

    let evidence: TomlValue = toml::from_str(
        &fs::read_to_string(root().join("docs/testing/release-evidence/0.73.toml"))
            .expect("0.73 release evidence manifest"),
    )
    .expect("valid 0.73 release evidence TOML");
    let registered = evidence["work_item"]
        .as_array()
        .expect("release work items")
        .iter()
        .any(|item| {
            item["id"].as_str() == Some(work_item)
                && item["required_sources"]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
                && item["required_tests"]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
                && item["required_artifacts"]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
                && item["ship_required"].as_bool() == Some(true)
        });
    assert!(
        registered,
        "{work_item} must have complete release evidence wiring"
    );
    panic!("HC-CANARY-RED:{defect}: removing {work_item} evidence must block Release 0.73");
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
    value["distribution"]["version_requirement"] = TomlValue::String("^0.12.15-hydra.1".to_owned());
    value["distribution"]["registry_sha256"] = TomlValue::String("wrong".to_owned());
    value["distribution"]["runtime_source_revision"] =
        TomlValue::String("616473ee923f4cd1429b3d8eb3be7df3eb9906b1".to_owned());
    value["distribution"]["yanked"] = TomlValue::Boolean(true);
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
        .any(|problem| problem.contains("pending upstream review")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("distribution version_requirement")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("distribution registry_sha256")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("distribution runtime_source_revision")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("distribution is not available")));
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

#[test]
fn w5_connection_census_cannot_claim_memory_or_skip_cardinalities() {
    let mut value = manifest("w5-hc2-connection-census-contract.toml");
    value["allocation_claims_allowed"] = TomlValue::Boolean(true);
    value["rss_claims_allowed"] = TomlValue::Boolean(true);
    value["cardinalities"] = TomlValue::Array(vec![TomlValue::Integer(1)]);
    let problems =
        xtask::performance_contract::check_w5_hc2_connection_census_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("allocation_claims_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("rss_claims_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("cardinalities changed")));
}

#[test]
fn w6_management_profile_cannot_mutate_product_promote_or_skip_owner_cells() {
    let mut value = manifest("w6-management-overhead-profile-contract.toml");
    value["product_mutation_allowed"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    value["scenarios"] = TomlValue::Array(vec![TomlValue::String("management-on-idle".to_owned())]);
    value["polling_requests"] = TomlValue::Integer(1);
    let problems =
        xtask::performance_contract::check_w6_management_overhead_profile_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("product_mutation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sampling matrix changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("scenario or metric set changed")));
}

#[test]
fn w6_management_evidence_cannot_promote_hide_outlier_or_rewrite_result() {
    let mut value = manifest("w6-management-overhead-316961e0.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["idle_working_set_outlier_retained"] = TomlValue::String(String::new());
    value["aggregate_cache_hit_transport_calls"] = TomlValue::Integer(60);
    value["raw_results"] = TomlValue::Array(Vec::new());
    let problems =
        xtask::performance_contract::check_w6_management_overhead_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("outlier_retained is missing")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("raw result manifest changed")));
}

#[test]
fn w3_tag_index_profile_cannot_mutate_product_or_drop_fanout_cells() {
    let mut value = manifest("w3-tag-index-profile-contract.toml");
    value["product_mutation_allowed"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    value["tag_cardinalities"] = TomlValue::Array(vec![TomlValue::Integer(1)]);
    value["event_subscriber_counts"] = TomlValue::Array(vec![TomlValue::Integer(0)]);
    let problems = xtask::performance_contract::check_w3_tag_index_profile_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("product_mutation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sampling matrix changed")));
}

#[test]
fn w3_tag_index_evidence_cannot_promote_shrink_or_hide_event_owner() {
    let mut value = manifest("w3-tag-index-profile-f8b90ef0.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(5);
    value["event_s8_tag_churn_bytes_per_delivery_tag"] = TomlValue::Float(0.0);
    value["decision"] = TomlValue::String("close-w3".to_owned());
    let problems = xtask::performance_contract::check_w3_tag_index_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("event owner result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w3_shared_event_tags_contract_cannot_weaken_matrix_or_thresholds() {
    let mut value = manifest("w3-shared-event-tags-contract.toml");
    value["candidate_attempts"] = TomlValue::Integer(5);
    value["promotable"] = TomlValue::Boolean(true);
    value["minimum_primary_total_gross_reduction_fraction"] = TomlValue::Float(0.0);
    value["event_content_equality_required"] = TomlValue::Boolean(false);
    let problems = xtask::performance_contract::check_w3_shared_event_tags_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sampling matrix changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("thresholds changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("event_content_equality_required must be true")));
}

#[test]
fn w3_shared_event_tags_evidence_cannot_promote_or_rewrite_acceptance() {
    let mut value = manifest("w3-shared-event-tags-accepted-82f46245.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(5);
    value["candidate_primary_median_gross_allocated_bytes"] = TomlValue::Integer(1);
    value["invalidation_removed_keys"] = TomlValue::Array(Vec::new());
    let problems = xtask::performance_contract::check_w3_shared_event_tags_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("primary result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("control result changed")));
}

#[test]
fn w4_resp_profile_cannot_mutate_product_or_drop_copy_cells() {
    let mut value = manifest("w4-resp-translation-profile-contract.toml");
    value["product_mutation_allowed"] = TomlValue::Boolean(true);
    value["promotable"] = TomlValue::Boolean(true);
    value["scenario_count"] = TomlValue::Integer(1);
    value["key_bytes"] = TomlValue::Array(vec![TomlValue::Integer(16)]);
    let problems =
        xtask::performance_contract::check_w4_resp_translation_profile_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("product_mutation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sampling matrix changed")));
}

#[test]
fn w4_resp_profile_evidence_cannot_hide_rejected_attempt_or_rewrite_owner() {
    let mut value = manifest("w4-resp-translation-profile-f8c968a5.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["rejected_attempts"] = TomlValue::Integer(0);
    value["resp2_bulk_median_gross_bytes_per_operation"] = TomlValue::Array(Vec::new());
    value["decision"] = TomlValue::String("close-w4".to_owned());
    let problems =
        xtask::performance_contract::check_w4_resp_translation_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("attempt ledger changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("encode result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w4_zero_copy_encode_contract_cannot_expand_scope_or_weaken_thresholds() {
    let mut value = manifest("w4-zero-copy-encode-contract.toml");
    value["candidate_attempts"] = TomlValue::Integer(5);
    value["promotable"] = TomlValue::Boolean(true);
    value["encode_wire_bytes_identical_required"] = TomlValue::Boolean(false);
    value["minimum_large_response_gross_reduction_fraction"] = TomlValue::Float(0.0);
    let problems = xtask::performance_contract::check_w4_zero_copy_encode_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("scope or matrix changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("encode_wire_bytes_identical_required must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("thresholds changed")));
}

#[test]
fn w4_zero_copy_encode_evidence_cannot_promote_or_rewrite_acceptance() {
    let mut value = manifest("w4-zero-copy-encode-accepted-d98f2afc.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(5);
    value["candidate_resp2_bulk_median_gross_bytes_per_operation"] = TomlValue::Array(Vec::new());
    value["minimum_observed_large_response_gross_reduction_fraction"] = TomlValue::Float(0.0);
    value["decision"] = TomlValue::String("close-w4".to_owned());
    let problems = xtask::performance_contract::check_w4_zero_copy_encode_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume or controls changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("allocation result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("threshold result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w7_durability_profile_cannot_invent_file_residency_or_skip_modes() {
    let mut value = manifest("w7-durability-page-cache-profile-contract.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(5);
    value["durability_modes"] = TomlValue::Array(vec![TomlValue::String("sync".to_owned())]);
    value["unsupported_anon_file_split_must_be_explicit"] = TomlValue::Boolean(false);
    let problems = xtask::performance_contract::check_w7_durability_page_cache_profile_contract(
        &value, "0.73",
    );
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sampling matrix changed")));
    assert!(problems.iter().any(
        |problem| problem.contains("unsupported_anon_file_split_must_be_explicit must be true")
    ));
}

#[test]
fn w7_durability_evidence_cannot_promote_or_hide_platform_limit() {
    let mut value = manifest("w7-durability-page-cache-profile-b5a327ce.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(5);
    value["page_cache_residency_claim_allowed"] = TomlValue::Boolean(true);
    value["payload64_fill_median_gross_bytes_per_operation"] = TomlValue::Array(Vec::new());
    value["decision"] = TomlValue::String("close-w7".to_owned());
    let problems = xtask::performance_contract::check_w7_durability_page_cache_profile_evidence(
        &value, "0.73",
    );
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("page_cache_residency_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("store result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w7_cached_budget_contract_cannot_replace_validation_or_weaken_thresholds() {
    let mut value = manifest("w7-cached-budget-total-contract.toml");
    value["candidate_attempts"] = TomlValue::Integer(5);
    value["promotable"] = TomlValue::Boolean(true);
    value["public_total_bytes_validation_scan_required"] = TomlValue::Boolean(false);
    value["minimum_primary_gross_reduction_fraction"] = TomlValue::Float(0.0);
    let problems =
        xtask::performance_contract::check_w7_cached_budget_total_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("scope or matrix changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems.iter().any(
        |problem| problem.contains("public_total_bytes_validation_scan_required must be true")
    ));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("thresholds changed")));
}

#[test]
fn w7_cached_budget_evidence_cannot_promote_or_rewrite_acceptance() {
    let mut value = manifest("w7-cached-budget-total-accepted-8bcd55e1.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["attempts"] = TomlValue::Integer(5);
    value["candidate_payload64_fill_median_gross_bytes_per_operation"] =
        TomlValue::Array(Vec::new());
    value["minimum_observed_primary_gross_reduction_fraction"] = TomlValue::Float(0.0);
    value["decision"] = TomlValue::String("close-w7-measured-no-win".to_owned());
    let problems =
        xtask::performance_contract::check_w7_cached_budget_total_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("allocation result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("threshold result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w8_allocator_contract_cannot_substitute_rss_or_change_defaults() {
    let mut value = manifest("w8-allocator-profile-contract.toml");
    value["rss_substitution_for_native_fields_allowed"] = TomlValue::Boolean(true);
    value["allocator_default_change_allowed"] = TomlValue::Boolean(true);
    value["windows_applicable_allocators"] =
        TomlValue::Array(vec![TomlValue::String("system".to_owned())]);
    value["required_native_concepts"] = TomlValue::Array(Vec::new());
    let problems = xtask::performance_contract::check_w8_allocator_profile_contract(&value, "0.73");
    assert!(problems.iter().any(
        |problem| problem.contains("rss_substitution_for_native_fields_allowed must be false")
    ));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("allocator_default_change_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("matrix changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("required telemetry changed")));
}

#[test]
fn w8_allocator_evidence_cannot_promote_hide_retention_or_claim_completion() {
    let mut value = manifest("w8-allocator-profile-deferred-ea12f68a.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["allocator_default_changed"] = TomlValue::Boolean(true);
    value["mimalloc_to_system_delete_working_set_ratio"] = TomlValue::Float(1.0);
    value["mimalloc_purge_calls_delta_median"] = TomlValue::Integer(0);
    value["terminal_disposition"] = TomlValue::String("accepted".to_owned());
    value["decision"] = TomlValue::String("switch-default-to-mimalloc".to_owned());
    let problems = xtask::performance_contract::check_w8_allocator_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("allocator_default_changed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("comparison changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("purge result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w9_decision_contract_cannot_authorize_without_pressure_or_change_legacy_semantics() {
    let mut value = manifest("w9-retained-byte-admission-decision-contract.toml");
    value["product_mutation_allowed"] = TomlValue::Boolean(true);
    value["required_pressure_evidence"] = TomlValue::Boolean(false);
    value["legacy_max_capacity_reinterpretation_allowed"] = TomlValue::Boolean(true);
    value["ineligible_substitutes"] = TomlValue::Array(Vec::new());
    let problems = xtask::performance_contract::check_w9_retained_byte_admission_decision_contract(
        &value, "0.73",
    );
    assert!(problems
        .iter()
        .any(|problem| problem.contains("product_mutation_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("required_pressure_evidence must be true")));
    assert!(problems.iter().any(|problem| {
        problem.contains("legacy_max_capacity_reinterpretation_allowed must be false")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("audit matrix changed")));
}

#[test]
fn w9_evidence_cannot_invent_pressure_or_add_a_hidden_limit() {
    let mut value = manifest("w9-retained-byte-admission-not-applicable-49ae52e8.toml");
    value["terminal_disposition"] = TomlValue::String("accepted".to_owned());
    value["eligible_unbounded_pressure_owners"] = TomlValue::Integer(1);
    value["legacy_max_capacity_changed"] = TomlValue::Boolean(true);
    value["owner_finding"] = TomlValue::Array(Vec::new());
    value["decision"] = TomlValue::String("add-global-retained-limit".to_owned());
    let problems =
        xtask::performance_contract::check_w9_retained_byte_admission_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("audit result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("legacy_max_capacity_changed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("owner audit changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w10_integration_contract_cannot_skip_local_gates_or_add_isolated_gains() {
    let mut value = manifest("w10-integrated-candidate-contract.toml");
    value["candidate_is_final_d4"] = TomlValue::Boolean(true);
    value["host_qualification_allowed_before_local_pass"] = TomlValue::Boolean(true);
    value["isolated_gains_may_be_added"] = TomlValue::Boolean(true);
    value["composition"] = TomlValue::Array(Vec::new());
    value["interaction"] = TomlValue::Array(Vec::new());
    let problems =
        xtask::performance_contract::check_w10_integrated_candidate_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_is_final_d4 must be false")));
    assert!(problems.iter().any(|problem| {
        problem.contains("host_qualification_allowed_before_local_pass must be false")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("isolated_gains_may_be_added must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("composition changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("interactions changed")));
}

#[test]
fn w10_local_amendment_cannot_hide_zero_tests_or_open_the_host_gate() {
    let mut value = manifest("w10-local-qualification-amendment-6c107abf.toml");
    value["invalid_attempt_retained"] = TomlValue::Boolean(false);
    value["invalid_attempt_passed"] = TomlValue::Boolean(true);
    value["host_run_allowed"] = TomlValue::Boolean(true);
    value["minimum_replacement_tests"] = TomlValue::Integer(0);
    let problems =
        xtask::performance_contract::check_w10_local_qualification_amendment(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("invalid_attempt_retained must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("invalid_attempt_passed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("correction changed")));
}

#[test]
fn w10_static_amendment_cannot_hide_failure_or_drop_targets() {
    let mut value = manifest("w10-static-qualification-amendment-a725077d.toml");
    value["failed_attempt_retained"] = TomlValue::Boolean(false);
    value["failed_attempt_passed"] = TomlValue::Boolean(true);
    value["target_set_changed"] = TomlValue::Boolean(true);
    value["host_run_allowed"] = TomlValue::Boolean(true);
    value["replacement_command"] = TomlValue::String("cargo clippy".to_owned());
    let problems =
        xtask::performance_contract::check_w10_static_qualification_amendment(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("failed_attempt_retained must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("failed_attempt_passed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("target_set_changed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("host_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("correction changed")));
}

#[test]
fn w10_local_evidence_cannot_hide_attempts_or_open_expensive_tiers() {
    let mut value = manifest("w10-local-qualification-passed-132917fd.toml");
    value["tests_failed"] = TomlValue::Integer(1);
    value["failed_attempt_hidden"] = TomlValue::Boolean(true);
    value["host_dispatch_allowed"] = TomlValue::Boolean(true);
    value["integrated_process_contract_present"] = TomlValue::Boolean(true);
    value["invalid_attempt"] = TomlValue::Array(Vec::new());
    value["decision"] = TomlValue::String("dispatch-host".to_owned());
    let problems =
        xtask::performance_contract::check_w10_local_qualification_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("failed_attempt_hidden must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("host_dispatch_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| { problem.contains("integrated_process_contract_present must be false") }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("invalid attempts changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w10_integrated_smoke_cannot_replace_real_surfaces_or_open_host_claims() {
    let mut value = manifest("w10-integrated-process-smoke-contract.toml");
    value["real_mtls_hc2_required"] = TomlValue::Boolean(false);
    value["real_resp_required"] = TomlValue::Boolean(false);
    value["host_dispatch_allowed"] = TomlValue::Boolean(true);
    value["performance_threshold_allowed"] = TomlValue::Boolean(true);
    value["mixed_weights_percent"] = TomlValue::Array(vec![TomlValue::Integer(100)]);
    let problems =
        xtask::performance_contract::check_w10_integrated_process_smoke_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("real_mtls_hc2_required must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("real_resp_required must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("host_dispatch_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("performance_threshold_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("matrix changed")));
}

#[test]
fn w10_integrated_smoke_evidence_cannot_hide_outcomes_or_authorize_host() {
    let mut value = manifest("w10-integrated-process-smoke-passed-e192e615.toml");
    value["success_total"] = TomlValue::Integer(3_999);
    value["incomplete_total"] = TomlValue::Integer(1);
    value["working_tree_dirty"] = TomlValue::Boolean(true);
    value["canary_red"] = TomlValue::Boolean(false);
    value["host_dispatch_allowed"] = TomlValue::Boolean(true);
    value["invalid_attempt"] = TomlValue::Array(Vec::new());
    value["decision"] = TomlValue::String("dispatch-host".to_owned());
    let problems =
        xtask::performance_contract::check_w10_integrated_process_smoke_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("working_tree_dirty must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("canary_red must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("host_dispatch_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("invalid attempts changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w10_focused_host_contract_cannot_shrink_or_patch_product_roles() {
    let mut value = manifest("w10-focused-host-comparison-contract.toml");
    value["candidate_source_commit"] = TomlValue::String("0".repeat(40));
    value["pairs_per_rate"] = TomlValue::Integer(1);
    value["mixed_weights_percent"] = TomlValue::Array(vec![TomlValue::Integer(100)]);
    value["overlay_may_change_product_sources"] = TomlValue::Boolean(true);
    value["host_dispatch_allowed_before_harness_local_receipt"] = TomlValue::Boolean(true);
    value["push_trigger_allowed"] = TomlValue::Boolean(true);
    value["allocation_or_rss_promotion_allowed"] = TomlValue::Boolean(true);
    value["regression_budget"][0]["maximum_relative_regression"] = TomlValue::Float(0.20);
    let problems =
        xtask::performance_contract::check_w10_focused_host_comparison_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("matrix changed")));
    assert!(problems
        .iter()
        .any(|problem| { problem.contains("overlay_may_change_product_sources must be false") }));
    assert!(problems.iter().any(|problem| {
        problem.contains("host_dispatch_allowed_before_harness_local_receipt must be false")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("push_trigger_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| { problem.contains("allocation_or_rss_promotion_allowed must be false") }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("budgets changed")));
}

#[test]
fn w10_focused_host_tooling_cannot_promote_local_data_or_relax_dispatch() {
    let mut value = manifest("w10-focused-host-tooling-passed-514183a9.toml");
    value["overlay_sha256"] = TomlValue::String("not-a-digest".to_owned());
    value["working_tree_dirty"] = TomlValue::Boolean(true);
    value["full_host_volume_reduction_allowed"] = TomlValue::Boolean(true);
    value["local_results_promotable"] = TomlValue::Boolean(true);
    value["release_numerical_claim_allowed"] = TomlValue::Boolean(true);
    value["manual_workflow_only"] = TomlValue::Boolean(false);
    value["host_dispatch_allowed"] = TomlValue::Boolean(false);
    value["local_hc2_success"] = TomlValue::Integer(349);
    value["invalid_attempt"] = TomlValue::Array(Vec::new());
    value["decision"] = TomlValue::String("promote-local-result".to_owned());
    let problems =
        xtask::performance_contract::check_w10_focused_host_tooling_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("overlay_sha256 is not SHA-256")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("working_tree_dirty must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("full_host_volume_reduction_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("local_results_promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("release_numerical_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("manual_workflow_only must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("host_dispatch_allowed must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("result changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("invalid attempts changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w10_focused_host_comparison_only_opens_compatibility_and_rollback() {
    let mut value = manifest("w10-focused-host-comparison-passed-7e307089.toml");
    value["attempt_count"] = TomlValue::Integer(29);
    value["all_primary_guards_passed"] = TomlValue::Boolean(false);
    value["published_072_compatibility_satisfied"] = TomlValue::Boolean(true);
    value["long_run_allowed"] = TomlValue::Boolean(true);
    value["release_numerical_claim_allowed"] = TomlValue::Boolean(true);
    value["allocation_or_rss_promotion_allowed"] = TomlValue::Boolean(true);
    value["decision"] = TomlValue::String("finalize-c73".to_owned());
    let problems =
        xtask::performance_contract::check_w10_focused_host_comparison_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("volume or outcome changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("all_primary_guards_passed must be true")));
    assert!(problems.iter().any(|problem| {
        problem.contains("published_072_compatibility_satisfied must be false")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("long_run_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("release_numerical_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("allocation_or_rss_promotion_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision changed")));
}

#[test]
fn w10_published_072_compatibility_cannot_substitute_or_shrink_the_matrix() {
    let mut value = manifest("w10-published-072-compatibility-contract.toml");
    value["published_source_substitute_allowed"] = TomlValue::Boolean(true);
    value["long_run_allowed_before_pass"] = TomlValue::Boolean(true);
    value["wire_cells_required"] = TomlValue::Integer(3);
    value["rolling_scenarios"] = TomlValue::Array(Vec::new());
    value["same_disk_rollback_required"] = TomlValue::Boolean(false);
    value["unsupported_cross_surface_substitution_allowed"] = TomlValue::Boolean(true);
    value["wire_case_applicability"] = TomlValue::Array(Vec::new());
    value["orchestrator"] = TomlValue::String("manual-command".to_owned());
    value["rolling_driver"] = TomlValue::String("manual-command".to_owned());
    value["ci_runner"] = TomlValue::String("manual-command".to_owned());
    value["canary_marker"] = TomlValue::String("green".to_owned());
    let problems =
        xtask::performance_contract::check_w10_published_072_compatibility_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("matrix changed")));
    assert!(problems
        .iter()
        .any(|problem| { problem.contains("published_source_substitute_allowed must be false") }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("long_run_allowed_before_pass must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("same_disk_rollback_required must be true")));
    assert!(problems.iter().any(|problem| {
        problem.contains("unsupported_cross_surface_substitution_allowed must be false")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("case coverage changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("canary changed")));
}

#[test]
fn w10_published_072_compatibility_evidence_cannot_hide_attempts_or_promote_claims() {
    let mut value = manifest("w10-published-072-compatibility-passed-9508330b.toml");
    value["rolling_scenarios_passed"] = TomlValue::Integer(5);
    value["all_nested_hashes_verified_after_download"] = TomlValue::Boolean(false);
    value["host_performance_claim_allowed"] = TomlValue::Boolean(true);
    value["candidate_is_final_c73"] = TomlValue::Boolean(true);
    value["silent_retry_allowed"] = TomlValue::Boolean(true);
    value["attempt"] = TomlValue::Array(Vec::new());
    value["decision"] = TomlValue::String("ship".to_owned());
    let problems =
        xtask::performance_contract::check_w10_published_072_compatibility_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("matrix changed")));
    assert!(problems.iter().any(|problem| {
        problem.contains("all_nested_hashes_verified_after_download must be true")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("host_performance_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_is_final_c73 must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("silent_retry_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("attempt chain changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("decision or scenarios changed")));
}

#[test]
fn w10_long_run_contract_cannot_shorten_roles_or_use_candidate_only_growth() {
    let mut value = manifest("w10-long-run-qualification-contract.toml");
    value["qualification_duration_seconds_per_role"] = TomlValue::Integer(3_600);
    value["confirmation_operations_per_role"] = TomlValue::Integer(1);
    value["candidate_only_slope_claim_allowed"] = TomlValue::Boolean(true);
    value["threshold_change_after_baseline_start_allowed"] = TomlValue::Boolean(true);
    value["equal_duration_roles_required"] = TomlValue::Boolean(false);
    value["final_checkpoint_required"] = TomlValue::Boolean(false);
    value["automatic_retry_allowed"] = TomlValue::Boolean(true);
    value["runner"] = TomlValue::String("manual-command".to_owned());
    value["regression_budget"] = TomlValue::Array(Vec::new());
    let problems =
        xtask::performance_contract::check_w10_long_run_qualification_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("duration or artifact budget changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("candidate_only_slope_claim_allowed must be false")));
    assert!(problems.iter().any(|problem| {
        problem.contains("threshold_change_after_baseline_start_allowed must be false")
    }));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("equal_duration_roles_required must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("final_checkpoint_required must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("automatic_retry_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("regression budgets changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity changed")));
}

#[test]
fn w10_long_run_qualification_workflow_is_serial_bounded_and_fail_closed() {
    let workflow = fs::read_to_string(
        root().join(".github/workflows/performance-long-run-qualification-073.yml"),
    )
    .expect("long-run qualification workflow");
    serde_yaml::from_str::<serde_yaml::Value>(&workflow).expect("valid workflow YAML");

    for required in [
        "workflow_call:",
        "runs-on: [self-hosted, linux, x64, hydracache-release]",
        "timeout-minutes: ${{ inputs.phase == 'confirmation' && 1740 || 900 }}",
        "HYDRACACHE_PERFORMANCE_COLLECTOR_CPUSET:",
        "--mode canary --phase qualification",
        "--mode role --role I73 --phase \"$LONG_RUN_PHASE\"",
        "--mode role --role C73 --phase \"$LONG_RUN_PHASE\"",
        "--mode seal --phase \"$LONG_RUN_PHASE\"",
        "w10-long-run-v2-qualification-passed-36532416869.toml",
        "test \"$lease_seconds\" -ge $((3480 * 60))",
        "accept-six-hour-qualification-and-hold-before-explicit-confirmation",
        "if: always()",
        "actions/upload-artifact@v4",
        "retention-days: 30",
        "C73_SHA: 16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b",
        "C73_TREE_OID: 92336607f21a68f563e65dc0fccccd8efaa14f7b",
        "w10-long-run-qualification-v2-contract.toml",
        "Cargo.lock.c73",
        "35b7ab4f62cee2ce41aa0547cc632a11d90e0d89861b87158bbcf7b184767630",
        "cp \"$C73_OVERLAY/Cargo.lock.c73\" \"$C73_OVERLAY/Cargo.lock\"",
        "cp \"$I73_OVERLAY/Cargo.lock\" \"$C73_OVERLAY/Cargo.lock\"",
        "diff -qr --exclude=target \"$I73_OVERLAY\" \"$C73_OVERLAY\"",
        "inputs.role == 'I73'",
        "inputs.role == 'C73'",
        "continuation-SHA256SUMS",
        "sha256sum --check continuation-SHA256SUMS",
        "performance-long-run-confirmation-i73-stage-073-",
        "I73 continuation attempt is not valid",
        "I73 continuation observation is incomplete",
    ] {
        assert!(workflow.contains(required), "workflow omitted {required}");
    }
    assert!(
        !workflow.contains("workflow_dispatch:")
            && !workflow.contains("push:")
            && !workflow.contains("schedule:"),
        "the reusable long-run worker must only be reachable through the approved entry workflow"
    );
    for forbidden in [
        "--mode role --role I73 --phase qualification",
        "--mode role --role C73 --phase qualification",
        "--mode role --role I73 --phase confirmation",
        "--mode role --role C73 --phase confirmation",
    ] {
        assert!(
            !workflow.contains(forbidden),
            "long-run workflow hard-coded a role phase: {forbidden}"
        );
    }
    assert_eq!(
        workflow.matches("--mode role --role I73").count(),
        1,
        "I73 must run exactly once"
    );
    assert_eq!(
        workflow.matches("--mode role --role C73").count(),
        1,
        "C73 must run exactly once"
    );
    for calibration in ["pre-i73", "post-i73", "pre-c73", "post-c73"] {
        assert!(
            workflow.contains(&format!("calibration/{calibration}.json")),
            "workflow omitted {calibration} calibration"
        );
    }

    let baseline = workflow
        .find("--mode role --role I73")
        .expect("I73 role command");
    let sealed_bounds = workflow
        .find("test -f \"$EVIDENCE_DIR/i73/baseline-bounds.json\"")
        .expect("sealed baseline bound guard");
    let candidate = workflow
        .find("--mode role --role C73")
        .expect("C73 role command");
    let seal = workflow
        .find("--mode seal --phase \"$LONG_RUN_PHASE\"")
        .expect("campaign seal command");
    assert!(
        baseline < sealed_bounds && sealed_bounds < candidate && candidate < seal,
        "workflow must seal I73 bounds before C73 and seal only after both roles"
    );

    let entry =
        fs::read_to_string(root().join(".github/workflows/performance-host-admission-073.yml"))
            .expect("registered host admission entry");
    serde_yaml::from_str::<serde_yaml::Value>(&entry).expect("valid entry workflow YAML");
    for required in [
        "authorize-long-run:",
        "startsWith(inputs.lease_owner, 'long-run-073@')",
        "environment: performance-reference-073",
        "long-run-confirmation-i73:",
        "long-run-confirmation-c73:",
        "needs: long-run-confirmation-i73",
        "if: needs.long-run-confirmation-i73.result == 'success'",
        "uses: ./.github/workflows/performance-long-run-qualification-073.yml",
        "tooling_sha: ${{ inputs.source_sha }}",
        "lease_owner: ${{ inputs.lease_owner }}",
        "lease_end: ${{ inputs.lease_end }}",
        "phase: ${{ inputs.long_run_phase }}",
        "role: pair",
        "role: I73",
        "role: C73",
        "entry_serializes_host: true",
    ] {
        assert!(
            entry.contains(required),
            "registered entry omitted {required}"
        );
    }
    assert_eq!(
        entry.matches("performance-reference-073-host").count(),
        1,
        "the registered entry must remain the sole owner of the shared host concurrency group"
    );
    assert_eq!(
        entry
            .matches("uses: ./.github/workflows/performance-long-run-qualification-073.yml")
            .count(),
        3,
        "qualification and the two serialized confirmation roles must share one worker definition"
    );
    assert_eq!(
        entry.matches("environment: performance-reference-073").count(),
        2,
        "one approval gates the long-run chain while the independent fresh-admission job retains its gate"
    );
}

#[test]
fn w10_integrated_workflows_pin_role_specific_harness_locks_and_restore_overlay_identity() {
    for relative in [
        ".github/workflows/performance-integrated-host-073.yml",
        ".github/workflows/performance-long-run-qualification-073.yml",
    ] {
        let workflow = fs::read_to_string(root().join(relative)).expect("integrated workflow");
        serde_yaml::from_str::<serde_yaml::Value>(&workflow).expect("valid workflow YAML");
        for required in [
            "3c2e8a20a26c6105fec6f449ce6cad43ba4a9a65f78516f5c0110ff0a9ca4953",
            "35b7ab4f62cee2ce41aa0547cc632a11d90e0d89861b87158bbcf7b184767630",
            "cp \"$C73_OVERLAY/Cargo.lock.c73\" \"$C73_OVERLAY/Cargo.lock\"",
            "cargo build --manifest-path \"$C73_OVERLAY/Cargo.toml\" --release --locked",
            "cp \"$I73_OVERLAY/Cargo.lock\" \"$C73_OVERLAY/Cargo.lock\"",
            "diff -qr --exclude=target \"$I73_OVERLAY\" \"$C73_OVERLAY\"",
        ] {
            assert!(workflow.contains(required), "{relative} omitted {required}");
        }
        let select = workflow
            .find("cp \"$C73_OVERLAY/Cargo.lock.c73\"")
            .expect("C73 lock selection");
        let build = workflow
            .find("cargo build --manifest-path \"$C73_OVERLAY/Cargo.toml\"")
            .expect("C73 harness build");
        let restore = workflow
            .find("cp \"$I73_OVERLAY/Cargo.lock\" \"$C73_OVERLAY/Cargo.lock\"")
            .expect("canonical overlay restoration");
        let compare = workflow
            .find("diff -qr --exclude=target \"$I73_OVERLAY\" \"$C73_OVERLAY\"")
            .expect("overlay identity guard");
        assert!(
            select < build && build < restore && restore < compare,
            "{relative} must select, build, restore, then compare overlays"
        );
    }
}

#[test]
fn w10_final_candidate_harness_identity_and_canary_are_fail_closed() {
    let harness = fs::read_to_string(root().join("tools/performance-integrated-073/src/main.rs"))
        .expect("integrated harness source");
    assert!(harness.contains("const C73_SHA: &str = \"16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b\";"));
    assert!(
        !harness.contains("const C73_SHA: &str = \"7e3070894aa51af96cdcb3e350eff923a309e1fa\";")
    );

    let runner = fs::read_to_string(root().join("scripts/perf/performance_long_run_073.py"))
        .expect("long-run runner source");
    for required in [
        "canary process failed with exit code",
        "receipt_value.get(\"final_checkpoint_present\") is not False",
        "checkpoint_values[-1].get(\"kind\") != \"final-work\"",
        "item.get(\"kind\") == \"post-idle-reconciled\"",
        "missing-post-idle-reconciled-checkpoint",
    ] {
        assert!(runner.contains(required), "runner omitted {required}");
    }
}

#[test]
fn w10_long_run_tooling_cannot_promote_local_data_or_hide_failed_attempts() {
    let mut value = manifest("w10-long-run-tooling-passed-c3769d3e.toml");
    value["local_results_promotable"] = TomlValue::Boolean(true);
    value["host_dispatch_route_registered"] = TomlValue::Boolean(true);
    value["local_resources_available"] = TomlValue::Boolean(true);
    value["local_operations_per_role"] = TomlValue::Integer(259_200_000);
    value["invalid_attempt"] = TomlValue::Array(Vec::new());
    let problems = xtask::performance_contract::check_w10_long_run_tooling_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("local_results_promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("host_dispatch_route_registered must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("local_resources_available must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("local screen changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("invalid attempt ledger changed")));
}

#[test]
fn w10_failed_six_hour_qualification_keeps_confirmation_and_claims_closed() {
    let value = manifest("w10-long-run-qualification-failed-8f8d4457.toml");
    assert_eq!(value["workflow_run"].as_integer(), Some(36_278_780_653));
    assert_eq!(
        value["tooling_commit"].as_str(),
        Some("8f8d44570eb984a37f38a6b8dafeaebbd54d3d11")
    );
    assert_eq!(value["operations_per_role"].as_integer(), Some(259_200_000));
    assert_eq!(value["checkpoint_count_per_role"].as_integer(), Some(362));
    for field in [
        "nested_sha256_verified",
        "backlog_drained",
        "reconciliation_exact",
        "management_truth_zero",
        "goodput_guard_passed",
        "cpu_guard_passed",
        "p99_guard_passed",
    ] {
        assert_eq!(value[field].as_bool(), Some(true), "{field} changed");
    }
    for field in [
        "rss_slope_guard_passed",
        "anonymous_pss_slope_guard_passed",
        "automatic_retry_allowed",
        "confirmation_allowed",
        "performance_claim_allowed",
        "candidate_is_final_c73",
        "tooling_or_orchestration_fault_found",
    ] {
        assert_eq!(value[field].as_bool(), Some(false), "{field} changed");
    }
    assert_eq!(value["result"].as_str(), Some("failed"));
    assert_eq!(
        value["decision"].as_str(),
        Some("retain-failed-attempt-and-stop-before-confirmation")
    );
    assert!(
        value["c73_rss_upper_95_bytes_per_second"]
            .as_float()
            .expect("candidate upper bound")
            > value["i73_rss_upper_95_bytes_per_second"]
                .as_float()
                .expect("baseline upper bound")
    );
}

#[test]
fn w10_analyzer_correction_cannot_change_raw_data_or_open_confirmation() {
    let mut value = manifest("w10-long-run-analyzer-correction-36278780653.toml");
    assert!(
        xtask::performance_contract::check_w10_long_run_analyzer_correction(&value, "0.73")
            .is_empty()
    );
    value["raw_observations_changed"] = TomlValue::Boolean(true);
    value["thresholds_changed"] = TomlValue::Boolean(true);
    value["rerun_performed"] = TomlValue::Boolean(true);
    value["confirmation_allowed"] = TomlValue::Boolean(true);
    value["bootstrap_method"] = TomlValue::String("randomized-levels".to_owned());
    value["c73_rss_upper_95_bytes_per_second"] = TomlValue::Float(200.0);
    let problems =
        xtask::performance_contract::check_w10_long_run_analyzer_correction(&value, "0.73");
    for required in [
        "raw_observations_changed must be false",
        "thresholds_changed must be false",
        "rerun_performed must be false",
        "confirmation_allowed must be false",
        "method or boundary changed",
        "slope comparison failed",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(required)),
            "missing problem containing {required:?}: {problems:?}"
        );
    }
}

#[test]
fn w10_registry_transition_cannot_transfer_old_evidence_or_skip_new_long_runs() {
    let mut value = manifest("w10-d4-registry-transition-9f13ee15.toml");
    assert!(
        xtask::performance_contract::check_w10_d4_registry_transition(&value, "0.73").is_empty()
    );
    value["binary_identity_proven_equal"] = TomlValue::Boolean(true);
    value["old_six_hour_evidence_transfer_allowed"] = TomlValue::Boolean(true);
    value["old_confirmation_allowed"] = TomlValue::Boolean(true);
    value["new_six_hour_qualification_required"] = TomlValue::Boolean(false);
    value["new_twenty_four_hour_confirmation_required"] = TomlValue::Boolean(false);
    value["server_dispatch_allowed_before_final_candidate_freeze"] = TomlValue::Boolean(true);
    let problems = xtask::performance_contract::check_w10_d4_registry_transition(&value, "0.73");
    for required in [
        "binary_identity_proven_equal must be false",
        "old_six_hour_evidence_transfer_allowed must be false",
        "old_confirmation_allowed must be false",
        "new_six_hour_qualification_required must be true",
        "new_twenty_four_hour_confirmation_required must be true",
        "server_dispatch_allowed_before_final_candidate_freeze must be false",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(required)),
            "missing problem containing {required:?}: {problems:?}"
        );
    }
}

#[test]
fn w10_long_run_v2_cannot_drift_from_the_final_registry_candidate() {
    let mut value = manifest("w10-long-run-qualification-v2-contract.toml");
    assert!(
        xtask::performance_contract::check_w10_long_run_qualification_v2_contract(&value, "0.73")
            .is_empty()
    );
    value["candidate_source_commit"] = TomlValue::String("0".repeat(40));
    value["candidate_root_lock_sha256"] = TomlValue::String("1".repeat(64));
    value["hydra_moka_registry_checksum"] = TomlValue::String("2".repeat(64));
    value["qualification_duration_seconds_per_role"] = TomlValue::Integer(3_600);
    let problems =
        xtask::performance_contract::check_w10_long_run_qualification_v2_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("final registry identity changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("duration or artifact budget changed")));
}

#[test]
fn w10_long_run_v2_qualification_cannot_open_final_release_or_hide_a_failed_guard() {
    let mut value = manifest("w10-long-run-v2-qualification-passed-36532416869.toml");
    assert!(
        xtask::performance_contract::check_w10_long_run_qualification_v2_evidence(&value, "0.73")
            .is_empty()
    );
    value["nested_sha256_verified"] = TomlValue::Boolean(false);
    value["rss_slope_guard_passed"] = TomlValue::Boolean(false);
    value["confirmation_started"] = TomlValue::Boolean(true);
    value["final_c73_allowed"] = TomlValue::Boolean(true);
    value["c73_rss_upper_95_bytes_per_second"] = TomlValue::Float(100.0);
    value["cpu_relative_change"] = TomlValue::Float(0.04);
    let problems =
        xtask::performance_contract::check_w10_long_run_qualification_v2_evidence(&value, "0.73");
    for required in [
        "nested_sha256_verified must be true",
        "rss_slope_guard_passed must be true",
        "confirmation_started must be false",
        "final_c73_allowed must be false",
        "regression budgets failed",
        "c73_rss_upper_95_bytes_per_second exceeds sealed baseline",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(required)),
            "missing problem containing {required:?}: {problems:?}"
        );
    }
}

#[test]
fn w10_interrupted_confirmation_cannot_admit_final_candidate_or_reuse_partial_roles() {
    let mut value = manifest("w10-long-run-v2-confirmation-interrupted-36622527013.toml");
    assert!(
        xtask::performance_contract::check_w10_long_run_confirmation_interrupted_evidence(
            &value, "0.73"
        )
        .is_empty()
    );

    value["automatic_retry_performed"] = TomlValue::Boolean(true);
    value["confirmation_passed"] = TomlValue::Boolean(true);
    value["final_c73_allowed"] = TomlValue::Boolean(true);
    value["product_failure_observed"] = TomlValue::Boolean(true);
    value["c73_role_completed"] = TomlValue::Boolean(true);
    value["correction_changes_duration"] = TomlValue::Boolean(true);
    value["c73_completed_at_last_checkpoint"] = TomlValue::Integer(1_036_800_000);
    value["decision"] =
        TomlValue::String("accept-twenty-four-hour-confirmation-and-admit-final-c73".to_owned());
    let problems =
        xtask::performance_contract::check_w10_long_run_confirmation_interrupted_evidence(
            &value, "0.73",
        );
    for required in [
        "automatic_retry_performed must be false",
        "confirmation_passed must be false",
        "final_c73_allowed must be false",
        "product_failure_observed must be false",
        "c73_role_completed must be false",
        "correction_changes_duration must be false",
        "partial C73 observation changed",
        "disposition changed",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(required)),
            "missing problem containing {required:?}: {problems:?}"
        );
    }
}

fn synthetic_w10_long_run_v2_confirmation() -> TomlValue {
    let mut value = manifest("w10-long-run-v2-qualification-passed-36532416869.toml");
    value["evidence_id"] =
        TomlValue::String("w10-long-run-v2-confirmation-passed-36622527013-v1".to_owned());
    value["state"] =
        TomlValue::String("twenty-four-hour-confirmation-passed-final-c73-admitted".to_owned());
    value["workflow_run"] = TomlValue::Integer(36_622_527_013);
    value["workflow_url"] = TomlValue::String(
        "https://github.com/javaquasar/hydracache/actions/runs/36622527013".to_owned(),
    );
    value["artifact_id"] = TomlValue::Integer(1);
    value["artifact_name"] = TomlValue::String(
        "performance-long-run-confirmation-073-b9f621a5b4f50c5277600fff9a8278fbd2154e28-36622527013-1"
            .to_owned(),
    );
    value["immutable_archive_path"] = TomlValue::String(
        "docs/testing/perf-artifacts/0.73/long-run-confirmation-36622527013".to_owned(),
    );
    value["tooling_sha"] = TomlValue::String("b9f621a5b4f50c5277600fff9a8278fbd2154e28".to_owned());
    value["phase"] = TomlValue::String("confirmation".to_owned());
    value.as_table_mut().expect("confirmation table").insert(
        "qualification_precondition_verified".to_owned(),
        TomlValue::Boolean(true),
    );
    value["confirmation_allowed"] = TomlValue::Boolean(false);
    value["confirmation_started"] = TomlValue::Boolean(true);
    value["performance_claim_allowed"] = TomlValue::Boolean(false);
    value["final_c73_allowed"] = TomlValue::Boolean(true);
    value["operations_per_role"] = TomlValue::Integer(1_036_800_000);
    value["completed_per_role"] = TomlValue::Integer(1_036_800_000);
    value["checkpoint_count_per_role"] = TomlValue::Integer(1_442);
    value["periodic_resource_samples_per_role"] = TomlValue::Integer(1_439);
    value["decision"] =
        TomlValue::String("accept-twenty-four-hour-confirmation-and-admit-final-c73".to_owned());
    value
}

#[test]
fn w10_long_run_v2_confirmation_accepts_only_complete_frozen_ship_evidence() {
    let mut value = synthetic_w10_long_run_v2_confirmation();
    assert!(
        xtask::performance_contract::check_w10_long_run_confirmation_v2_evidence(&value, "0.73")
            .is_empty()
    );

    value["candidate_source_commit"] = TomlValue::String("0".repeat(40));
    value["nested_sha256_verified"] = TomlValue::Boolean(false);
    value["qualification_precondition_verified"] = TomlValue::Boolean(false);
    value["automatic_retry_performed"] = TomlValue::Boolean(true);
    value["performance_claim_allowed"] = TomlValue::Boolean(true);
    value["operations_per_role"] = TomlValue::Integer(1_036_799_999);
    value["periodic_resource_samples_per_role"] = TomlValue::Integer(1_438);
    value["bootstrap_seed"] = TomlValue::Integer(1);
    value["cpu_relative_change"] = TomlValue::Float(0.04);
    value["c73_rss_upper_95_bytes_per_second"] = TomlValue::Float(100.0);
    let problems =
        xtask::performance_contract::check_w10_long_run_confirmation_v2_evidence(&value, "0.73");
    for required in [
        "confirmation identity changed",
        "nested_sha256_verified must be true",
        "qualification_precondition_verified must be true",
        "automatic_retry_performed must be false",
        "performance_claim_allowed must be false",
        "workload or outcomes changed",
        "frozen method changed",
        "regression budgets failed",
        "c73_rss_upper_95_bytes_per_second exceeds sealed baseline",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(required)),
            "missing problem containing {required:?}: {problems:?}"
        );
    }
}

#[test]
fn w5_connection_profile_cannot_promote_claim_server_bytes_or_skip_samples() {
    let mut value = manifest("w5-hc2-connection-profile-contract.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["per_server_connection_claim_allowed"] = TomlValue::Boolean(true);
    value["cardinalities"] = TomlValue::Array(vec![TomlValue::Integer(1)]);
    value["repeats_per_cardinality"] = TomlValue::Integer(1);
    let problems =
        xtask::performance_contract::check_w5_hc2_connection_profile_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("per_server_connection_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sampling matrix changed")));
}

#[test]
fn w5_connection_profile_evidence_cannot_promote_or_rewrite_results() {
    let mut value = manifest("w5-hc2-connection-profile-ff657fc5.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["per_server_connection_claim_allowed"] = TomlValue::Boolean(true);
    value["cardinality"][3]["median_working_set_delta_bytes"] = TomlValue::Integer(1);
    let problems =
        xtask::performance_contract::check_w5_hc2_connection_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("per_server_connection_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("1000-connection result changed")));
}

#[test]
fn w5_split_connection_profile_cannot_promote_or_merge_endpoints() {
    let mut value = manifest("w5-hc2-split-connection-profile-contract.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["universal_per_connection_claim_allowed"] = TomlValue::Boolean(true);
    value["server_scope"] = TomlValue::String("combined".to_owned());
    value["cardinalities"] = TomlValue::Array(vec![TomlValue::Integer(1)]);
    let problems =
        xtask::performance_contract::check_w5_hc2_split_connection_profile_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("universal_per_connection_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("identity changed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("sampling matrix changed")));
}

#[test]
fn w5_split_connection_evidence_cannot_promote_or_rewrite_endpoint_results() {
    let mut value = manifest("w5-hc2-split-connection-profile-19a440d4.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["universal_per_connection_claim_allowed"] = TomlValue::Boolean(true);
    value["cardinality"][3]["median_server_working_set_delta_bytes"] = TomlValue::Integer(1);
    let problems =
        xtask::performance_contract::check_w5_hc2_split_connection_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("universal_per_connection_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("1000-connection result changed")));
}

#[test]
fn w5_slow_consumer_profile_cannot_promote_or_weaken_the_workload() {
    let mut value = manifest("w5-hc2-slow-consumer-profile-contract.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["server_queued_byte_claim_allowed"] = TomlValue::Boolean(true);
    value["connections"] = TomlValue::Integer(1);
    value["total_mutations"] = TomlValue::Integer(1_100);
    let problems =
        xtask::performance_contract::check_w5_hc2_slow_consumer_profile_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("server_queued_byte_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("workload changed")));
}

#[test]
fn w5_slow_consumer_evidence_cannot_promote_or_move_retention_to_server() {
    let mut value = manifest("w5-hc2-slow-consumer-profile-ff432801.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["server_queued_byte_claim_allowed"] = TomlValue::Boolean(true);
    value["median_paired_server_working_set_delta_bytes"] = TomlValue::Integer(21_204_992);
    let problems =
        xtask::performance_contract::check_w5_hc2_slow_consumer_profile_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("server_queued_byte_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("result changed")));
}

#[test]
fn w5_raw_transport_stall_cannot_promote_or_weaken_the_matrix() {
    let mut value = manifest("w5-hc2-raw-transport-stall-contract.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["exact_server_queued_byte_claim_allowed"] = TomlValue::Boolean(true);
    value["connections"] = TomlValue::Integer(1);
    value["value_bytes"] = TomlValue::Array(vec![TomlValue::Integer(4_096)]);
    let problems =
        xtask::performance_contract::check_w5_hc2_raw_transport_stall_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exact_server_queued_byte_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("workload changed")));
}

#[test]
fn w5_raw_transport_stall_evidence_cannot_promote_or_rewrite_retention() {
    let mut value = manifest("w5-hc2-raw-transport-stall-25cdfb61.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["exact_server_queued_byte_claim_allowed"] = TomlValue::Boolean(true);
    value["median_paired_server_working_set_delta_262144_bytes"] = TomlValue::Integer(9_216_000);
    let problems =
        xtask::performance_contract::check_w5_hc2_raw_transport_stall_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exact_server_queued_byte_claim_allowed must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("result changed")));
}

#[test]
fn w5_outbound_byte_admission_cannot_change_scope_or_skip_progress() {
    let mut value = manifest("w5-hc2-outbound-byte-admission-contract.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["frame_rejection_added"] = TomlValue::Boolean(true);
    value["oversize_progress_required"] = TomlValue::Boolean(false);
    value["application_queue_byte_budget"] = TomlValue::Integer(8 * 1024 * 1024);
    let problems =
        xtask::performance_contract::check_w5_hc2_outbound_byte_admission_contract(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("frame_rejection_added must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("oversize_progress_required must be true")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("design changed")));
}

#[test]
fn w5_outbound_byte_admission_evidence_cannot_promote_or_rewrite_result() {
    let mut value = manifest("w5-hc2-outbound-byte-admission-2846e936.toml");
    value["promotable"] = TomlValue::Boolean(true);
    value["frame_rejection_added"] = TomlValue::Boolean(true);
    value["candidate_median_unpolled_dispatch_262144"] = TomlValue::Integer(128);
    value["candidate_median_paired_server_working_set_delta_262144_bytes"] =
        TomlValue::Integer(22_667_264);
    let problems =
        xtask::performance_contract::check_w5_hc2_outbound_byte_admission_evidence(&value, "0.73");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("promotable must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("frame_rejection_added must be false")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("result changed")));
}
