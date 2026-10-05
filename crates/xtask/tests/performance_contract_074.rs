use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn contract(name: &str) -> toml::Value {
    let path = root().join("docs/testing/performance/0.74").join(name);
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn trace() -> Value {
    json!({
        "trace_sha256": "trace-a",
        "payload_corpus_sha256": "payload-a",
        "key_corpus_sha256": "keys-a",
        "seed": 740074,
        "warmup_seconds": 1,
        "measurement_seconds": 3,
        "operation_count": 1000,
        "offered_schedule_sha256": "schedule-a",
        "concurrency": 8,
        "pipeline_depth": 10,
        "security": "plaintext",
        "persistence": "off",
        "semantic_contract": "get-hit-v1"
    })
}

fn role(id: &str) -> Value {
    json!({
        "source_sha": id,
        "binary_sha256": format!("binary-{id}"),
        "binary_sha256s": [format!("binary-{id}")],
        "redis_tool_sha256": "redis-benchmark-7.2.5-a",
        "trace": trace(),
        "outcomes": {
            "completed": 1000,
            "errors": 0,
            "timeouts": 0,
            "rejections": 0,
            "late": 0,
            "incomplete": 0,
            "final_cardinality": 128,
            "final_state_sha256": "state-a"
        },
        "metrics": {
            "goodput_operations_per_second": 1000.0,
            "latency_p99_microseconds": 100.0,
            "cpu_seconds_per_operation": 0.00001
        }
    })
}

fn receipt() -> Value {
    let surfaces = [
        "resp-api",
        "native-api-hc1",
        "native-api-hc2",
        "client-surface-state",
        "embedded-hydracache-raw",
        "embedded-hydracache-typed",
    ]
    .into_iter()
    .map(|surface| {
        json!({
            "surface": surface,
            "trace_sha256": "trace-a",
            "baseline_goodput": 1000.0,
            "candidate_goodput": 1000.0
        })
    })
    .collect::<Vec<_>>();
    json!({
        "release": "0.74",
        "candidate_derived_thresholds": false,
        "final_sample_present": true,
        "declared_block_order": "baseline_candidate",
        "block_order": ["baseline", "candidate"],
        "integrated_native_guard": true,
        "baseline": role("baseline-source"),
        "candidate": role("candidate-source"),
        "surface_results": surfaces
    })
}

fn has(problems: &[String], needle: &str) -> bool {
    problems.iter().any(|problem| problem.contains(needle))
}

#[test]
fn checked_in_w0_contract_is_valid_and_non_promotable() {
    assert!(xtask::performance_contract_074::check_identities(&contract(
        "baseline-identities.toml"
    ))
    .is_empty());
    assert!(
        xtask::performance_contract_074::check_matrix(&contract("scenario-matrix.toml")).is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_statistics(&contract("statistics.toml")).is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_registry(&contract("proposal-registry.toml"))
            .is_empty()
    );
    assert!(xtask::performance_contract_074::check_host(&contract("host-profile.toml")).is_empty());
    assert!(
        xtask::performance_contract_074::check_local_harness(&contract("local-harness.toml"))
            .is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_composition(&contract("composition-ledger.toml"))
            .is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_at_root(Path::new(&root()), None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn w11_schema_hashes_and_local_completion_flags_are_exact() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    for (path_field, digest_field) in [
        ("packet_manifest_schema", "packet_manifest_schema_sha256"),
        (
            "seal_input_inventory_schema",
            "seal_input_inventory_schema_sha256",
        ),
        (
            "checkpoint_envelope_schema",
            "checkpoint_envelope_schema_sha256",
        ),
        ("raw_manifest_schema", "raw_manifest_schema_sha256"),
        ("start_manifest_schema", "start_manifest_schema_sha256"),
        ("host_observation_schema", "host_observation_schema_sha256"),
        ("start_bundle_schema", "start_bundle_schema_sha256"),
    ] {
        let path = root().join(implementation[path_field].as_str().unwrap());
        let digest = Sha256::digest(std::fs::read(path).unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(implementation[digest_field].as_str(), Some(digest.as_str()));
    }
    for field in [
        "raw_file_set_verification_complete",
        "campaign_manifest_binding_complete",
        "continuation_packet_digest_binding_complete",
        "strict_seal_input_resolver_complete",
        "live_seal_dispatch_complete",
        "guard_evidence_binding_complete",
        "deterministic_archive_creation_complete",
        "phase_progress_watchdog_complete",
        "supervisor_progress_loss_maintenance_complete",
        "supervisor_measurement_loss_maintenance_complete",
        "full_design_manifest_fields_complete",
        "host_receipt_revalidation_complete",
        "live_start_host_receipt_revalidation_complete",
        "start_evidence_import_complete",
        "live_attach_lease_admission_complete",
        "live_attach_checkpoint_refresh_complete",
        "start_bundle_builder_complete",
        "start_bundle_transport_verifier_complete",
        "supervisor_owned_start_upload_complete",
        "privileged_start_bundle_staging_complete",
        "connection_transport_failure_survival_complete",
        "startup_checkpoint_absence_maintenance_complete",
        "campaign_lifecycle_controller_loss_rehearsal_complete",
        "progress_loss_host_rehearsal_complete",
        "measurement_loss_host_rehearsal_complete",
        "lease_expiry_host_rehearsal_complete",
        "real_systemd_spawn_backend_host_rehearsal_complete",
        "systemd_confinement_complete",
    ] {
        assert_eq!(implementation[field].as_bool(), Some(true), "{field}");
    }
    assert_eq!(
        implementation["live_service_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["privileged_start_bundle_host_rehearsal_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["release_admission_allowed"].as_bool(),
        Some(false)
    );
}

#[test]
fn w11_lease_expiry_host_rehearsal_is_exact_and_non_promotable() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    let relative = implementation["campaign_lease_expiry_rehearsal"]
        .as_str()
        .unwrap();
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(root().join(relative)).unwrap()).unwrap();

    assert_eq!(
        evidence["source_commit"].as_str(),
        Some("187b63fb20555743f8b220ac60273e0e8e8663cc")
    );
    assert_eq!(evidence["github_run"]["id"].as_u64(), Some(37380414415));
    assert_eq!(
        evidence["observation"]["terminal_state"].as_str(),
        Some("LEASE_EXPIRED_INCOMPLETE")
    );
    assert_eq!(
        evidence["observation"]["product_candidate_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["observation"]["terminal_execution_fields_cleared"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["fixture_unit_stopped"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["active_campaign_released"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["decision"]["scheduled_drift_window_independently_sampled"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["decision"]["qualification_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["decision"]["release_admission_allowed"].as_bool(),
        Some(false)
    );
}

#[test]
fn w10_cannot_freeze_a_candidate_or_hide_an_accepted_proposal() {
    let mut value = contract("composition-ledger.toml");
    value["freeze_c74_allowed"] = toml::Value::Boolean(true);
    value["accepted_candidate_count"] = toml::Value::Integer(1);
    value["proposal"]
        .as_array_mut()
        .unwrap()
        .first_mut()
        .unwrap()["disposition"] = toml::Value::String("accepted".to_owned());
    let problems = xtask::performance_contract_074::check_composition(&value);
    assert!(has(&problems, "freeze_c74_allowed"));
    assert!(has(&problems, "accepted_candidate_count"));
    assert!(has(&problems, "cannot contain an accepted proposal"));
}

#[test]
fn w12_evidence_skeleton_is_exact_and_fail_closed() {
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("docs/testing/release-evidence/0.74.toml")).unwrap(),
    )
    .unwrap();
    let expected = [
        "W0", "W1", "W2", "W3", "W4", "W4a", "W4b", "W4c", "W4d", "W4e", "W5", "W6", "W6a", "W6b",
        "W7", "W8", "W8a", "W8b", "W8c", "W9", "W9a", "W9b", "W9c", "W9d", "W9e", "W10", "W11",
        "W12",
    ];
    let actual = manifest["work_item"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(
        manifest["dynamic_canary_work_items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap())
            .collect::<Vec<_>>(),
        ["W11"]
    );
    assert!(
        xtask::canary_check::check_canary_registry_for_release(&root(), "0.74")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn published_b73_requires_exact_tag_archive_and_runtime_relationship() {
    let mut value = contract("baseline-identities.toml");
    value["predecessor_candidate"]["annotated_tag_commit_sha"] =
        toml::Value::String("0".repeat(40));
    value["predecessor_candidate"]["release_archive_verified"] = toml::Value::Boolean(false);
    value["predecessor_closure"]["tag_and_product_relationship_verified"] =
        toml::Value::Boolean(false);

    let problems = xtask::performance_contract_074::check_identities(&value);
    assert!(has(&problems, "annotated_tag_commit_sha"));
    assert!(has(&problems, "release_archive_verified"));
    assert!(has(&problems, "tag_and_product_relationship_verified"));
}

#[test]
fn local_harness_freezes_placement_abba_and_noise_policy() {
    let mut value = contract("local-harness.toml");
    value["cpu_affinity_required"] = toml::Value::Boolean(false);
    value["pairs"] = toml::Value::Integer(3);
    value["noise"]["minimum_p99_effect"] = toml::Value::Float(0.0);
    let problems = xtask::performance_contract_074::check_local_harness(&value);
    assert!(has(&problems, "cpu_affinity_required"));
    assert!(has(&problems, "pairs"));
    assert!(has(&problems, "minimum_p99_effect"));
}

#[test]
fn proposal_admission_requires_w1_attribution_and_evidence() {
    let mut value = contract("proposal-registry.toml");
    value["work_items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"].as_str() == Some("W1"))
        .unwrap()["decision"] = toml::Value::String("in-progress".to_owned());
    let problems = xtask::performance_contract_074::check_registry(&value);
    assert!(has(&problems, "requires locally attributed W1"));

    let mut value = contract("proposal-registry.toml");
    let w3 = value["work_items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"].as_str() == Some("W3"))
        .unwrap();
    w3["decision"] = toml::Value::String("authorized-local-candidate".to_owned());
    w3.as_table_mut().unwrap().remove("evidence");
    let problems = xtask::performance_contract_074::check_registry(&value);
    assert!(has(&problems, "requires evidence"));
}

#[test]
fn w3_adaptive_retry_preserves_pipeline_one_and_freezes_deep_thresholds() {
    let value = contract("w3-adaptive-coalescing-contract.toml");
    assert_eq!(
        value["state"].as_str(),
        Some("rejected-semantic-backpressure-before-measurement")
    );
    assert_eq!(value["pairs"].as_integer(), Some(5));
    assert_eq!(
        value["activation"]["minimum_complete_frames_in_current_read"].as_integer(),
        Some(2)
    );
    assert_eq!(
        value["activation"]["pipeline_one_uses_existing_write_response"].as_bool(),
        Some(true)
    );
    assert_eq!(value["activation"]["timer_allowed"].as_bool(), Some(false));
    assert_eq!(
        value["deep_pipeline_acceptance"]["minimum_goodput_ratio"].as_float(),
        Some(1.20)
    );
    assert_eq!(
        value["pipeline_one_non_regression"]["minimum_goodput_ratio"].as_float(),
        Some(0.98)
    );
    assert_eq!(
        value["pipeline_one_non_regression"]["minimum_passing_pairs"].as_integer(),
        Some(5)
    );
    assert_eq!(
        value["semantic_guards"]["native_surfaces_unchanged"].as_bool(),
        Some(true)
    );
    assert_eq!(
        value["negative_evidence"].as_str(),
        Some("local-runs/w3-adaptive-semantic-rejection.json")
    );
}

#[test]
fn socket_boundary_attribution_keeps_local_product_work_admitted() {
    let value = contract("proposal-registry.toml");
    assert!(
        xtask::performance_contract_074::check_registry(&value).is_empty(),
        "the checked-in open kernel gate is a terminal local W1 attribution state"
    );
}

#[test]
fn matched_receipt_is_accepted() {
    assert!(xtask::performance_contract_074::check_receipt(&receipt()).is_empty());
}

#[test]
fn trace_workload_security_and_persistence_drift_are_rejected() {
    for field in [
        "trace_sha256",
        "payload_corpus_sha256",
        "key_corpus_sha256",
        "seed",
        "measurement_seconds",
        "concurrency",
        "pipeline_depth",
        "security",
        "persistence",
        "semantic_contract",
    ] {
        let mut value = receipt();
        value["candidate"]["trace"][field] = json!(format!("changed-{field}"));
        let problems = xtask::performance_contract_074::check_receipt(&value);
        assert!(
            has(&problems, &format!("mismatch for {field}")),
            "{problems:#?}"
        );
    }
}

#[test]
fn redis_tool_mismatch_and_mixed_binary_are_rejected() {
    let mut value = receipt();
    value["candidate"]["redis_tool_sha256"] = json!("redis-other");
    value["candidate"]["binary_sha256s"] = json!(["a", "b"]);
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "mismatched Redis tool"));
    assert!(has(&problems, "mixes more than one binary"));
}

#[test]
fn missing_outcomes_final_sample_and_nonfinite_metric_are_rejected() {
    let mut value = receipt();
    value["baseline"]["outcomes"]
        .as_object_mut()
        .unwrap()
        .remove("errors");
    value["candidate"]["metrics"]["cpu_seconds_per_operation"] = json!("NaN");
    value["final_sample_present"] = json!(false);
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "missing the final sample"));
    assert!(has(&problems, "outcomes are missing errors"));
    assert!(has(&problems, "is not finite"));
}

#[test]
fn reordered_blocks_and_candidate_thresholds_are_rejected() {
    let mut value = receipt();
    value["block_order"] = json!(["candidate", "baseline"]);
    value["candidate_derived_thresholds"] = json!(true);
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "reordered"));
    assert!(has(&problems, "candidate-derived thresholds"));
}

#[test]
fn pooled_or_missing_native_surface_is_rejected() {
    let mut value = receipt();
    let results = value["surface_results"].as_array_mut().unwrap();
    results.retain(|row| row["surface"] != "native-api-hc2");
    results.push(json!({
        "surface": "native",
        "trace_sha256": "trace-a",
        "baseline_goodput": 1000.0,
        "candidate_goodput": 1000.0
    }));
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "pooled"));
    assert!(has(&problems, "omits native-api-hc2"));
}

#[test]
fn unmatched_surface_trace_and_three_percent_native_regression_are_rejected() {
    let mut value = receipt();
    let row = value["surface_results"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["surface"] == "client-surface-state")
        .unwrap();
    row["trace_sha256"] = json!("other-trace");
    row["candidate_goodput"] = json!(970.0);
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "does not use the matched trace"));
    assert!(has(&problems, "regresses goodput beyond 2%"));
}

#[test]
fn long_run_controller_workflow_is_manual_serialized_and_signs_off_host() {
    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-qualification-074.yml"),
    )
    .unwrap();
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(!workflow.contains("pull_request:"));
    assert!(!workflow.contains("schedule:"));
    assert!(workflow.contains("group: long-run-074-${{ inputs.host_id }}"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("environment: performance-reference-074"));
    assert!(workflow.contains("runs-on: ubuntu-latest"));
    assert!(workflow.contains("runs-on: [self-hosted, linux, x64, hydracache-release]"));
    assert_eq!(
        workflow
            .matches("secrets.HYDRACACHE_074_AUTH_SIGNING_KEY_HEX")
            .count(),
        1
    );
    let signing_key = workflow
        .find("secrets.HYDRACACHE_074_AUTH_SIGNING_KEY_HEX")
        .unwrap();
    let self_hosted = workflow
        .find("runs-on: [self-hosted, linux, x64, hydracache-release]")
        .unwrap();
    assert!(signing_key < self_hosted);
    assert!(workflow.contains("build-request"));
    assert!(workflow.contains("request.sha256"));
    assert!(workflow.contains("actions: read"));
    assert!(workflow.contains("start_bundle_run_id"));
    assert!(workflow.contains("start_bundle_artifact_name"));
    assert!(workflow.contains("--verify-start-bundle"));
    assert!(workflow.contains("request-start"));
    assert!(workflow.contains("inputs.expected_state_revision == '0'"));
    assert!(workflow.contains("Capture a best-effort read-only status snapshot"));
    assert!(!workflow.contains("continue-on-error: true"));
}

#[test]
fn w9e_allocator_attribution_is_linux_only_complete_and_non_promotable() {
    let allocator = contract("w9e-linux-allocator-profile-contract.toml");
    assert_eq!(
        allocator["state"].as_str(),
        Some("preregistered-before-dedicated-linux-execution")
    );
    assert_eq!(allocator["operating_system"].as_str(), Some("linux"));
    assert_eq!(allocator["architecture"].as_str(), Some("x86_64"));
    assert_eq!(allocator["minimum_attempts"].as_integer(), Some(20));
    assert_eq!(
        allocator["counterbalanced_allocator_order_required"].as_bool(),
        Some(true)
    );
    assert_eq!(allocator["product_mutation_allowed"].as_bool(), Some(false));
    assert_eq!(
        allocator["allocator_default_change_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        allocator["acceptance_decision_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(allocator["promotable"].as_bool(), Some(false));
    assert_eq!(
        allocator["owner_materiality_floor_percent"].as_float(),
        Some(5.0)
    );

    let allocators = allocator["allocators"].as_array().unwrap();
    assert_eq!(allocators.len(), 3);
    for expected in ["system", "mimalloc", "jemalloc"] {
        assert!(allocators
            .iter()
            .any(|value| value.as_str() == Some(expected)));
    }

    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-allocator-attribution-074.yml"),
    )
    .unwrap();
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("feat/0.74-resp-native-throughput"));
    assert!(workflow.contains("group: performance-reference-074-host"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("runs-on: [self-hosted, linux, x64, hydracache-release]"));
    assert!(workflow.contains("--features \"allocator-$allocator\""));
    assert!(workflow.contains("--repeats 5"));
    assert!(workflow.contains("acceptance_decision_allowed"));
    assert!(workflow.contains("promotable"));
    assert!(!workflow.contains("performance-long-run-qualification-074"));
}

#[test]
fn w9c_kernel_attribution_is_linux_only_counterbalanced_and_non_promotable() {
    let kernel = contract("w9c-linux-kernel-attribution-contract.toml");
    assert_eq!(
        kernel["state"].as_str(),
        Some("preregistered-before-dedicated-linux-execution")
    );
    assert_eq!(kernel["operating_system"].as_str(), Some("linux"));
    assert_eq!(kernel["architecture"].as_str(), Some("x86_64"));
    assert_eq!(kernel["minimum_attempts"].as_integer(), Some(30));
    assert_eq!(kernel["minimum_processes"].as_integer(), Some(60));
    assert_eq!(kernel["repeats_per_cell"].as_integer(), Some(5));
    assert_eq!(
        kernel["paired_untraced_control_required"].as_bool(),
        Some(true)
    );
    assert_eq!(
        kernel["measurement_gate_after_warmup_required"].as_bool(),
        Some(true)
    );
    assert_eq!(kernel["product_mutation_allowed"].as_bool(), Some(false));
    assert_eq!(kernel["runtime_tuning_allowed"].as_bool(), Some(false));
    assert_eq!(kernel["acceptance_decision_allowed"].as_bool(), Some(false));
    assert_eq!(kernel["promotable"].as_bool(), Some(false));

    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-kernel-attribution-074.yml"),
    )
    .unwrap();
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("feat/0.74-resp-native-throughput"));
    assert!(workflow.contains("group: performance-reference-074-host"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("runs-on: [self-hosted, linux, x64, hydracache-release]"));
    assert!(workflow.contains("command -v strace"));
    assert!(workflow.contains("command -v ss"));
    assert!(workflow.contains("--repeats 5"));
    assert!(workflow.contains("candidate_data_present"));
    assert!(workflow.contains("promotable"));
    assert!(!workflow.contains("performance-long-run-qualification-074"));
}

#[test]
fn w11_host_capability_probe_is_read_only_and_serialized() {
    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-capability-074.yml"),
    )
    .unwrap();
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("group: performance-reference-074-host"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("runs-on: [self-hosted, linux, x64, hydracache-release]"));
    assert!(workflow.contains("Collect read-only host capability receipt"));
    assert!(workflow.contains("if: github.event_name == 'push' || inputs.mode == 'probe'"));
    let probe_job = workflow
        .split("\n  provision:")
        .next()
        .expect("probe job precedes mutating dispatch jobs");
    for forbidden in [
        "sudo ",
        "systemctl start",
        "systemctl stop",
        "systemctl restart",
        "systemd-run --unit",
        "performance-long-run-qualification-074",
    ] {
        assert!(
            !probe_job.contains(forbidden),
            "forbidden mutation: {forbidden}"
        );
    }
    for mode in [
        "provision",
        "systemd-smoke",
        "controller-loss-smoke",
        "campaign-lifecycle-smoke",
        "campaign-progress-loss-smoke",
        "campaign-measurement-loss-smoke",
        "campaign-lease-expiry-smoke",
    ] {
        assert!(
            workflow.contains(&format!("inputs.mode == '{mode}'")),
            "missing isolated dispatch condition for {mode}"
        );
    }

    let probe = std::fs::read_to_string(
        root().join("scripts/perf/performance_long_run_host_capability_074.py"),
    )
    .unwrap();
    assert!(probe.contains("\"read_only\": True"));
    assert!(probe.contains("\"mutation_performed\": False"));
}
