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
        xtask::performance_contract_074::check_release_admission(&contract(
            "release-admission-contract.toml"
        ))
        .is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_terminal_dispositions(
            &root(),
            &contract("terminal-disposition-ledger.toml")
        )
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
        (
            "role_overhead_attempt_schema",
            "role_overhead_attempt_schema_sha256",
        ),
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
        "host_observation_socket_export_complete",
        "installed_source_receipt_binding_complete",
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
        "privileged_start_bundle_host_rehearsal_complete",
        "protected_start_attach_abort_rehearsal_complete",
        "signed_socket_start_host_rehearsal_complete",
        "live_attach_host_rehearsal_complete",
        "live_abort_host_rehearsal_complete",
        "supervisor_restart_rehearsal_complete",
        "live_role_reboot_rehearsal_complete",
        "live_seal_complete",
        "live_mutating_operations_complete",
        "role_overhead_analyzer_complete",
        "role_overhead_collector_complete",
    ] {
        assert_eq!(implementation[field].as_bool(), Some(true), "{field}");
    }
    assert_eq!(
        implementation["live_service_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["release_admission_allowed"].as_bool(),
        Some(false)
    );
}

#[test]
fn w11_live_role_reboot_is_fail_closed_and_non_promotable() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    let relative = implementation["live_role_reboot_rehearsal"]
        .as_str()
        .unwrap();
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(root().join(relative)).unwrap()).unwrap();

    assert_eq!(
        evidence["source_commit"].as_str(),
        Some("b83fd1b792cd610de9cb4817d178b5c8c9635914")
    );
    assert_eq!(
        evidence["provisioning"]["run_id"].as_u64(),
        Some(37453559280)
    );
    assert_eq!(
        evidence["start_bundle"]["run_id"].as_u64(),
        Some(37453956080)
    );
    assert_eq!(evidence["start"]["run_id"].as_u64(), Some(37454199519));
    assert_ne!(
        evidence["reboot"]["old_boot_id"].as_str(),
        evidence["reboot"]["new_boot_id"].as_str()
    );
    assert_eq!(
        evidence["reboot"]["measurement_loss_reason"].as_str(),
        Some("host-identity-drift")
    );
    assert_eq!(
        evidence["final_state"]["campaign_state"].as_str(),
        Some("FAILED_INCOMPLETE")
    );
    for field in [
        "recorded_failure",
        "harness_identity_cleared",
        "daemon_identity_cleared",
        "checkpoint_identity_cleared",
        "controller_lease_cleared",
        "active_campaign_released",
        "role_unit_absent",
        "role_process_absent",
        "supervisor_active",
        "runner_inactive",
        "runner_disabled",
    ] {
        assert_eq!(
            evidence["final_state"][field].as_bool(),
            Some(true),
            "{field}"
        );
    }
    assert_eq!(
        evidence["final_state"]["replacement_role_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["forbidden_work"]["product_candidate_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["live_role_reboot_rehearsal_complete"].as_bool(),
        Some(true)
    );
    assert_eq!(
        implementation["live_service_complete"].as_bool(),
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
fn w11_non_product_role_rehearsal_is_complete_but_role_budget_stays_closed() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    let relative = implementation["supervisor_idle_overhead_evidence"]
        .as_str()
        .unwrap();
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(root().join(relative)).unwrap()).unwrap();

    assert_eq!(
        evidence["tooling_source_commit"].as_str(),
        Some("1bc4823b9f0585cd7ad4f767d0fcf093a7f7576b")
    );
    assert_eq!(evidence["github_run"]["id"].as_u64(), Some(37388038717));
    assert_eq!(
        evidence["observation"]["product_candidate_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["observation"]["cpu_budget_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["rss_budget_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["io_budget_evaluated"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["io_budget_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["idle_screen_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["decision"]["idle_overhead_budget_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["decision"]["release_admission_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["supervisor_idle_cpu_rss_screen_complete"].as_bool(),
        Some(true)
    );
    for field in [
        "supervisor_idle_io_screen_complete",
        "supervisor_idle_overhead_screen_complete",
        "io_accounting_instrumentation_staged",
        "io_accounting_instrumentation_host_rehearsal_complete",
    ] {
        assert_eq!(implementation[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "idle_overhead_budget_complete",
        "role_overhead_qualification_complete",
    ] {
        assert_eq!(implementation[field].as_bool(), Some(false), "{field}");
    }
    assert_eq!(
        implementation["non_product_role_overhead_rehearsal_complete"].as_bool(),
        Some(true)
    );
    let role_evidence: Value = serde_json::from_slice(
        &std::fs::read(
            root().join(
                implementation["non_product_role_overhead_rehearsal"]
                    .as_str()
                    .unwrap(),
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        role_evidence["evidence_class"].as_str(),
        Some("non-product-role-overhead-rehearsal")
    );
    assert_eq!(role_evidence["promotable"].as_bool(), Some(false));
    assert_eq!(
        role_evidence["source_commit"].as_str(),
        Some("543108f1ccd206ae670803c2617f07e7fa91ae62")
    );
    assert_eq!(
        role_evidence["github_run"]["id"].as_u64(),
        Some(37467960862)
    );
    assert_eq!(
        role_evidence["workload_identity"]["attempt_count"].as_u64(),
        Some(20)
    );
    assert_eq!(
        role_evidence["budgets"]["non_product_rehearsal_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        role_evidence["decision"]["role_overhead_qualification_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        role_evidence["decision"]["release_admission_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["role_overhead_analyzer_complete"].as_bool(),
        Some(true)
    );
    let analyzer = std::fs::read_to_string(
        root().join(implementation["role_overhead_analyzer"].as_str().unwrap()),
    )
    .unwrap();
    assert!(analyzer.contains("non-product-role-overhead-rehearsal"));
    assert!(analyzer.contains("\"role_overhead_qualification_complete\": False"));
    assert!(analyzer.contains("\"release_admission_allowed\": False"));
    let collector = std::fs::read_to_string(
        root().join(implementation["role_overhead_collector"].as_str().unwrap()),
    )
    .unwrap();
    assert!(collector.contains("non-product-role-overhead-rehearsal"));
    assert!(collector.contains("PAIRS = 5"));
    assert!(collector.contains("CHECKPOINT_BYTES = 4_096"));
    assert!(collector.contains("active_campaign_absent"));
    assert!(collector.contains("shell=False"));
    assert!(collector.contains("RLIMIT_AS"));
    assert!(!collector.contains("/bin/sh"));
    let fixture = std::fs::read_to_string(
        root().join(implementation["role_overhead_fixture"].as_str().unwrap()),
    )
    .unwrap();
    assert!(fixture.contains("const PAIRS: u32 = 5"));
    assert!(fixture.contains("const CHECKPOINT_BYTES: usize = 4_096"));
    assert!(fixture.contains("role-overhead fixture invocation is invalid"));

    let provisioning: Value = serde_json::from_slice(
        &std::fs::read(
            root().join(
                implementation["host_provisioning_receipt"]
                    .as_str()
                    .unwrap(),
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(provisioning["github_run"]["id"].as_u64(), Some(37387670191));
    assert_eq!(
        provisioning["host"]["service_io_accounting"].as_bool(),
        Some(true)
    );
    assert_eq!(
        provisioning["decision"]["release_admission_allowed"].as_bool(),
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
fn w3_delivery_frontier_review_preserves_semantics_without_authorizing_a_candidate() {
    let review = contract("w3-delivery-frontier-review.toml");
    assert_eq!(review["schema_version"].as_integer(), Some(1));
    assert_eq!(review["release"].as_str(), Some("0.74"));
    assert_eq!(
        review["state"].as_str(),
        Some("local-feasibility-review-no-candidate")
    );
    assert_eq!(review["candidate_source_sha"].as_str(), Some("UNRESOLVED"));
    for flag in [
        "product_mutation_allowed",
        "numerical_measurement_started",
        "numerical_claims_allowed",
        "promotable",
        "prior_rejections_reopened",
    ] {
        assert_eq!(review[flag].as_bool(), Some(false), "{flag}");
    }
    let boundary = &review["delivery_boundary"];
    for flag in [
        "previous_response_write_all_complete_required",
        "previous_response_flush_complete_required",
    ] {
        assert_eq!(boundary[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "first_byte_authorizes_next_mutation",
        "socket_writable_hint_authorizes_next_mutation",
        "complete_write_without_flush_authorizes_next_mutation",
        "execute_then_batch_before_write_allowed",
        "transaction_staging_or_early_ack_allowed",
    ] {
        assert_eq!(boundary[flag].as_bool(), Some(false), "{flag}");
    }
    let scope = &review["guard_scope"];
    assert_eq!(scope["first_reply_bytes"].as_integer(), Some(5));
    assert_eq!(
        scope["accepted_prefixes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_integer().unwrap())
            .collect::<Vec<_>>(),
        (0..=5).collect::<Vec<_>>()
    );
    assert_eq!(scope["mutations_before_release"].as_integer(), Some(1));
    assert_eq!(
        scope["mutations_after_successful_release"].as_integer(),
        Some(2)
    );
    for flag in [
        "complete_reply_pending_flush",
        "flush_error_stops_next_command",
        "other_connection_reads_exact_committed_state",
        "correctness_uses_explicit_future_poll",
    ] {
        assert_eq!(scope[flag].as_bool(), Some(true), "{flag}");
    }
    assert_eq!(
        scope["sleep_based_correctness_allowed"].as_bool(),
        Some(false)
    );
    let previous = contract("w3-adaptive-coalescing-contract.toml");
    for section in ["deep_pipeline_acceptance", "pipeline_one_non_regression"] {
        assert_eq!(review.get(section), previous.get(section), "{section}");
    }
    assert!(root().join(review["design"].as_str().unwrap()).is_file());
    let source =
        std::fs::read_to_string(root().join(review["semantic_tests"].as_str().unwrap())).unwrap();
    for function in [
        "every_partial_reply_and_pending_flush_preserve_the_mutation_frontier",
        "failed_flush_after_complete_reply_does_not_execute_the_next_set",
    ] {
        assert!(source.contains(&format!("async fn {function}()")));
    }
}

#[test]
fn w3_staged_execution_assessment_does_not_authorize_product_or_threshold_changes() {
    let review = contract("w3-staged-execution-review.toml");
    assert_eq!(review["schema_version"].as_integer(), Some(1));
    assert_eq!(review["release"].as_str(), Some("0.74"));
    assert_eq!(
        review["state"].as_str(),
        Some("architecture-review-complete-not-admitted")
    );
    assert_eq!(review["candidate_source_sha"].as_str(), Some("UNRESOLVED"));
    assert_eq!(
        review["architecture_assessment_allowed"].as_bool(),
        Some(true)
    );
    for flag in [
        "product_implementation_allowed",
        "numerical_measurement_started",
        "numerical_claims_allowed",
        "promotable",
        "prior_rejections_reopened",
        "changes_external_semantics",
    ] {
        assert_eq!(review[flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "first_mutation_visible_before_its_response_completion",
        "previous_response_complete_write_and_flush_before_next_execution",
        "commit_before_exposing_success_response",
        "native_operations_progress_while_resp_writer_pending",
        "no_store_lock_across_io",
        "no_early_tenant_quota_reservation",
        "request_time_and_conditional_result_revalidated_at_execution",
        "no_early_audit_event_or_mutation_publication",
        "no_speculative_shared_store_mutation_and_rollback",
        "no_native_cost_transfer",
        "no_acknowledgement_or_durability_change",
        "no_distributed_transactions",
    ] {
        assert_eq!(review["invariants"][flag].as_bool(), Some(true), "{flag}");
    }
    let verification = &review["verification"];
    assert_eq!(
        verification["accepted_first_reply_prefix_bytes"],
        contract("w3-delivery-frontier-review.toml")["guard_scope"]["accepted_prefixes"]
    );
    for flag in [
        "native_invalidate_changes_queued_nx_result",
        "native_put_changes_queued_get_result",
        "expiry_changes_queued_nx_result",
        "new_ttl_starts_at_queued_command_execution",
        "native_put_consumes_quota_before_queued_set",
        "quota_failure_emits_one_audit_and_no_set_mutation",
        "canonical_separate_write_positive_control",
    ] {
        assert_eq!(verification[flag].as_bool(), Some(true), "{flag}");
    }
    assert_eq!(verification["model_permutations"].as_integer(), Some(6));
    assert_eq!(
        verification["chronological_fixed_batch_schedules"].as_integer(),
        Some(3)
    );
    assert_eq!(
        verification["admitted_fixed_batch_schedules"].as_integer(),
        Some(0)
    );
    let previous = contract("w3-adaptive-coalescing-contract.toml");
    for section in ["deep_pipeline_acceptance", "pipeline_one_non_regression"] {
        assert_eq!(review.get(section), previous.get(section), "{section}");
    }
    assert!(root().join(review["design"].as_str().unwrap()).is_file());
    let source =
        std::fs::read_to_string(root().join(review["semantic_tests"].as_str().unwrap())).unwrap();
    for function in [
        "queued_set_nx_revalidates_after_intervening_native_invalidate",
        "queued_get_observes_intervening_native_put_without_blocking_native",
        "queued_set_nx_uses_expiry_and_ttl_at_its_execution_boundary",
        "queued_set_does_not_reserve_quota_ahead_of_intervening_native_put",
    ] {
        assert!(source.contains(&format!("async fn {function}()")));
    }
    let model =
        std::fs::read_to_string(root().join(review["order_model"].as_str().unwrap())).unwrap();
    assert!(model.contains(
        "fn fixed_two_reply_batch_cannot_preserve_both_commit_frontier_and_response_order()"
    ));
}

#[test]
fn w9b_large_response_attribution_is_bounded_and_does_not_reopen_product_candidates() {
    let review = contract("w9b-response-buffer-attribution-contract.toml");
    assert_eq!(
        review["state"].as_str(),
        Some("preregistered-local-d1-only")
    );
    assert_eq!(review["candidate_source_sha"].as_str(), Some("UNRESOLVED"));
    for flag in [
        "product_mutation_allowed",
        "promotable",
        "product_numeric_claims_allowed",
        "prior_w3_or_w4_rejections_reopened",
    ] {
        assert_eq!(review[flag].as_bool(), Some(false), "{flag}");
    }
    let screen = &review["screen"];
    assert_eq!(screen["seed"].as_integer(), Some(740074));
    assert_eq!(
        screen["warmup_operations_per_canonical_control"].as_integer(),
        Some(100)
    );
    assert_eq!(screen["fresh_process_repeats"].as_integer(), Some(3));
    assert_eq!(screen["maximum_attempts"].as_integer(), Some(15));
    for flag in [
        "independent_identically_initialized_canonical_controls",
        "exact_results_dispatch_mutations_and_final_cardinality_required",
    ] {
        assert_eq!(screen[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "socket_io_included",
        "counting_allocator_timing_is_product_cpu_evidence",
        "expected_payload_allocation_inside_validator_allowed",
    ] {
        assert_eq!(screen[flag].as_bool(), Some(false), "{flag}");
    }
    let cells: Vec<_> = review["cell"]
        .as_array()
        .unwrap()
        .iter()
        .map(|cell| {
            (
                cell["operation"].as_str().unwrap(),
                cell["payload_bytes"].as_integer().unwrap(),
                cell["iterations"].as_integer().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        cells,
        vec![
            ("get", 256, 10000),
            ("set", 256, 10000),
            ("get", 4096, 10000),
            ("set", 4096, 10000),
            ("get", 1048576, 500)
        ]
    );
    let boundary = &review["future_candidate_boundary"];
    for flag in [
        "previous_complete_write_and_flush_before_next_command",
        "pipeline_one_retains_canonical_path",
        "release_scratch_before_waiting_for_another_read",
        "no_cross_connection_pool",
        "no_larger_input_output_or_idle_retention_bound",
        "native_surfaces_remain_separate_required_guards",
    ] {
        assert_eq!(boundary[flag].as_bool(), Some(true), "{flag}");
    }
    assert_eq!(
        boundary["minimum_affected_end_to_end_gross_allocation_reduction"].as_float(),
        Some(0.20)
    );
    assert_eq!(
        boundary["unaffected_minimum_goodput_ratio"].as_float(),
        Some(0.98)
    );
    assert_eq!(
        boundary["unaffected_maximum_cpu_or_p99_ratio"].as_float(),
        Some(1.03)
    );
    assert_eq!(
        boundary["minimum_counterbalanced_independent_pairs_for_d3"].as_integer(),
        Some(5)
    );
    let source = std::fs::read_to_string(root().join(review["source"].as_str().unwrap())).unwrap();
    assert!(source.contains(review["profile_id"].as_str().unwrap()));
    for function in [
        "canonical_controls_reconcile_dispatch_mutation_and_cardinality",
        "canonical_result_validator_adds_no_gross_allocation",
        "canonical_validator_rejects_changed_values_errors_and_frame_bytes",
    ] {
        assert!(source.contains(&format!("fn {function}()")));
    }
}

#[test]
fn w9b_owner_receipts_preserve_all_registered_cells_without_product_claims() {
    let report: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join(
            "docs/testing/performance/0.74/local-runs/w9b-serial-encoder-owner-cc0b2fbf.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        report["source_commit"],
        "cc0b2fbfc0b7d9c4c9f37e96089f37134fdf546f"
    );
    assert_eq!(report["candidate_source_commit"], "UNRESOLVED");
    for flag in ["promotable", "product_changed", "product_performance_claim"] {
        assert_eq!(report[flag], false, "{flag}");
    }
    let review = contract("w9b-response-buffer-attribution-contract.toml");
    let receipts = report["raw_receipts"].as_array().unwrap();
    assert_eq!(receipts.len(), 15);
    let paths: std::collections::BTreeSet<_> = receipts
        .iter()
        .map(|entry| entry["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths.len(), 15);
    let summaries = report["cells"].as_array().unwrap();
    assert_eq!(summaries.len(), 5);
    for cell in review["cell"].as_array().unwrap() {
        let operation = cell["operation"].as_str().unwrap();
        let payload_bytes = cell["payload_bytes"].as_integer().unwrap() as u64;
        let iterations = cell["iterations"].as_integer().unwrap() as u64;
        let summary = summaries
            .iter()
            .find(|entry| {
                entry["operation"] == operation && entry["payload_bytes"] == payload_bytes
            })
            .unwrap();
        assert_eq!(summary["iterations"], iterations);
        let matches: Vec<_> = receipts
            .iter()
            .filter(|entry| {
                entry["operation"] == operation && entry["payload_bytes"] == payload_bytes
            })
            .collect();
        assert_eq!(matches.len(), 3);
        let mut previous: Option<Value> = None;
        for repeat in 1..=3 {
            let entry = matches
                .iter()
                .find(|entry| entry["repeat"] == repeat)
                .unwrap();
            let text = std::fs::read_to_string(root().join(entry["path"].as_str().unwrap()))
                .unwrap()
                .replace("\r\n", "\n");
            let original = text.strip_suffix('\n').unwrap_or(&text);
            let digest = Sha256::digest(original.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert_eq!(entry["original_tool_output_sha256"], digest);
            let receipt: Value = serde_json::from_str(original).unwrap();
            for field in ["source_commit", "profile_id", "binary_sha256"] {
                assert_eq!(receipt[field], report[field], "{field}");
            }
            assert_eq!(receipt["operation"], operation);
            assert_eq!(receipt["payload_bytes"], payload_bytes);
            assert_eq!(receipt["iterations"], iterations);
            assert_eq!(receipt["seed"], 740074);
            assert_eq!(receipt["canonical_warmup_operations"], 100);
            assert_eq!(receipt["promotable"], false);
            assert_eq!(receipt["exact_result_validation"], true);
            for (summary_field, receipt_field) in [
                (
                    "canonical_encode_incremental_bytes_per_operation",
                    "canonical_encode_incremental_allocated_bytes_per_operation",
                ),
                (
                    "isolated_encode_incremental_bytes_per_operation",
                    "encode_incremental_allocated_bytes_per_operation",
                ),
                ("workload_sha256", "workload_sha256"),
                ("request_sha256", "request_sha256"),
                ("key_corpus_sha256", "key_corpus_sha256"),
                ("payload_corpus_sha256", "payload_corpus_sha256"),
            ] {
                if receipt[receipt_field].is_number() {
                    assert_eq!(
                        summary[summary_field].as_f64().unwrap(),
                        receipt[receipt_field].as_f64().unwrap(),
                        "{summary_field}"
                    );
                } else {
                    assert_eq!(
                        summary[summary_field], receipt[receipt_field],
                        "{summary_field}"
                    );
                }
            }
            let control = receipt["canonical_execution"]["gross_allocated_bytes_per_operation"]
                .as_f64()
                .unwrap();
            let encoded = receipt["canonical_execution_and_encode"]
                ["gross_allocated_bytes_per_operation"]
                .as_f64()
                .unwrap();
            assert_eq!(
                summary["canonical_execution_gross_bytes_per_operation"]
                    .as_f64()
                    .unwrap(),
                control
            );
            assert_eq!(
                summary["canonical_execution_and_encode_gross_bytes_per_operation"]
                    .as_f64()
                    .unwrap(),
                encoded
            );
            let increment = receipt["canonical_encode_incremental_allocated_bytes_per_operation"]
                .as_f64()
                .unwrap();
            assert!((increment - (encoded - control)).abs() < 1e-9);
            assert!(
                (summary["canonical_encode_share"].as_f64().unwrap() - increment / encoded).abs()
                    < 1e-12
            );
            for field in [
                "canonical_control_dispatches",
                "canonical_encoded_dispatches",
            ] {
                assert_eq!(receipt[field], 2 * iterations + 102);
            }
            for stage in ["canonical_execution", "canonical_execution_and_encode"] {
                assert_eq!(receipt[stage]["checksum"], iterations);
                if let Some(previous) = &previous {
                    assert_eq!(
                        receipt[stage]["gross_allocated_bytes"],
                        previous[stage]["gross_allocated_bytes"]
                    );
                }
            }
            if let Some(previous) = &previous {
                for field in [
                    "workload_sha256",
                    "request_sha256",
                    "key_corpus_sha256",
                    "payload_corpus_sha256",
                ] {
                    assert_eq!(receipt[field], previous[field], "{field}");
                }
            }
            previous = Some(receipt);
        }
    }
    assert_eq!(report["assessment"]["accepted_product_proposals"], 0);
    assert_eq!(
        report["assessment"]["end_to_end_twenty_percent_reduction_proven"],
        false
    );
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
        ["W11", "W12"]
    );
    assert!(
        xtask::canary_check::check_canary_registry_for_release(&root(), "0.74")
            .unwrap()
            .is_empty()
    );
    let report = xtask::release_evidence::build_report(&root(), "0.74", None).unwrap();
    for item in report
        .work_items
        .iter()
        .filter(|item| !matches!(item.id.as_str(), "W11" | "W12"))
    {
        assert!(
            item.reasons
                .iter()
                .all(|reason| !reason.contains("dynamic canary")),
            "explicit W11/W12 canary selection must not block {}: {:?}",
            item.id,
            item.reasons
        );
    }
}

#[test]
fn w12_release_admission_contract_is_fail_closed() {
    let mut value = contract("release-admission-contract.toml");
    assert!(xtask::performance_contract_074::check_release_admission(&value).is_empty());

    value["candidate_source_sha"] = toml::Value::String("1".repeat(40));
    value["release_admission_allowed"] = toml::Value::Boolean(true);
    value["claims"]["numerical_product_performance_allowed"] = toml::Value::Boolean(true);
    value["authorization"]["expensive_runs_enabled"] = toml::Value::Boolean(true);
    value["completion"]["immutable_archive_complete"] = toml::Value::Boolean(true);
    value["unresolved"] = toml::Value::Array(Vec::new());

    let problems = xtask::performance_contract_074::check_release_admission(&value);
    for field in [
        "candidate_source_sha",
        "release_admission_allowed",
        "numerical_product_performance_allowed",
        "expensive_runs_enabled",
        "immutable_archive_complete",
        "unresolved blockers",
    ] {
        assert!(
            has(&problems, field),
            "missing problem for {field}: {problems:?}"
        );
    }
}

#[test]
fn release_admission_expected_red_canary() {
    let mut value = contract("release-admission-contract.toml");
    let defect_enabled = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("PERF74-W12");
    if defect_enabled {
        value["authorization"]["expensive_runs_enabled"] = toml::Value::Boolean(true);
    }
    let problems = xtask::performance_contract_074::check_release_admission(&value);
    assert!(
        problems.is_empty(),
        "{}: injected unsafe release admission flag was rejected: {problems:?}",
        if defect_enabled {
            "HC-CANARY-RED:PERF74-W12"
        } else {
            "checked-in release admission contract is invalid"
        }
    );
}

#[test]
fn w12_release_note_retains_draft_claim_rollback_and_gate_boundaries() {
    let note = std::fs::read_to_string(root().join("docs/releases/0.74.0.md")).unwrap();
    for marker in [
        "pre-release draft; no product candidate is accepted or frozen",
        "no numerical product-performance",
        "no exact C74 D4 artifact",
        "Activation and rollback",
        "Remaining admission work",
        "performance-contract-check --require-ship",
        "release-evidence --require-ship",
    ] {
        assert!(note.contains(marker), "release note must retain {marker:?}");
    }
}

#[test]
fn w12_generic_evidence_report_retains_closed_ship_admission() {
    for release in ["0.74", "0.74.0"] {
        let report = xtask::release_evidence::build_report(&root(), release, None).unwrap();
        assert!(
            has(&report.reasons, "0.74 ship admission is closed"),
            "ordinary green test receipts must not substitute for qualification: {:?}",
            report.reasons
        );
        assert!(report
            .work_items
            .iter()
            .all(|item| { item.stage <= xtask::release_evidence::EvidenceStage::FastGreen }));
    }
}

#[test]
fn w2_through_w9_terminal_dispositions_are_exact_and_non_promotable() {
    let mut value = contract("terminal-disposition-ledger.toml");
    assert!(
        xtask::performance_contract_074::check_terminal_dispositions(&root(), &value).is_empty()
    );

    let receipt_path = root().join(
        "docs/testing/performance/0.74/local-runs/w2-w9-terminal-dispositions-local-20261006.json",
    );
    let receipt: Value = serde_json::from_slice(&std::fs::read(receipt_path).unwrap()).unwrap();
    let ledger_bytes = std::fs::read(
        root().join("docs/testing/performance/0.74/terminal-disposition-ledger.toml"),
    )
    .unwrap();
    let digest = Sha256::digest(ledger_bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        receipt["terminal_disposition_ledger_sha256"].as_str(),
        Some(digest.as_str())
    );
    assert_eq!(
        receipt["accepted_product_candidate_count"].as_u64(),
        Some(0)
    );
    assert_eq!(receipt["promotable"].as_bool(), Some(false));
    assert_eq!(receipt["release_admission_allowed"].as_bool(), Some(false));

    value["disposition"]
        .as_array_mut()
        .unwrap()
        .first_mut()
        .unwrap()["accepted_product_change"] = toml::Value::Boolean(true);
    let problems = xtask::performance_contract_074::check_terminal_dispositions(&root(), &value);
    assert!(has(&problems, "cannot accept a product change"));
}

#[test]
fn w12_workspace_fast_gate_covers_every_work_item_without_budget_drift() {
    let registry = xtask::fast_suite::load_registry(&root()).unwrap();
    assert_eq!(registry.aggregate_budget_seconds, 1_680);
    assert_eq!(
        registry
            .suite
            .iter()
            .map(|suite| suite.budget_seconds)
            .sum::<u64>(),
        1_680
    );
    let gate = registry
        .suite
        .iter()
        .find(|suite| suite.id == "fast.workspace-nextest")
        .unwrap();
    assert_eq!(gate.timeout_seconds, 2_400);
    assert_eq!(gate.budget_seconds, 840);
    assert_eq!(gate.command.program, "cargo");
    assert_eq!(
        gate.command.args,
        [
            "nextest",
            "run",
            "--workspace",
            "--profile",
            "ci",
            "--locked"
        ]
    );

    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("docs/testing/release-evidence/0.74.toml")).unwrap(),
    )
    .unwrap();
    let gate_items = gate
        .work_items
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for item in manifest["work_item"].as_array().unwrap() {
        let id = item["id"].as_str().unwrap();
        assert!(gate_items.contains(id), "workspace gate must cover {id}");
        assert_eq!(
            item["fast_gate_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<Vec<_>>(),
            ["fast.workspace-nextest"],
            "release evidence must bind {id} to the unchanged workspace gate"
        );
    }

    let receipt: Value = serde_json::from_slice(
        &std::fs::read(root().join(
            "docs/testing/performance/0.74/local-runs/w12-fast-gate-registration-local-20261006.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["gate_id"].as_str(), Some("fast.workspace-nextest"));
    assert_eq!(receipt["work_items_bound"].as_u64(), Some(28));
    assert_eq!(receipt["command_changed"].as_bool(), Some(false));
    assert_eq!(receipt["budget_changed"].as_bool(), Some(false));
    assert_eq!(receipt["gate_executed"].as_bool(), Some(false));
    assert_eq!(
        receipt["exact_commit_receipt_present"].as_bool(),
        Some(false)
    );
    assert_eq!(receipt["promotable"].as_bool(), Some(false));

    let timeout: Value = serde_json::from_slice(
        &std::fs::read(root().join(
            "docs/testing/performance/0.74/local-runs/w12-workspace-fast-gate-cold-timeout-05e7c6e9.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        timeout["source_commit"].as_str(),
        Some("05e7c6e957ca7c1705f22977164f1576e358f88c")
    );
    assert_eq!(timeout["outcome"].as_str(), Some("timeout"));
    assert_eq!(timeout["execution_phase"].as_str(), Some("compile"));
    assert_eq!(timeout["duration_ms"].as_u64(), Some(2_400_607));
    assert_eq!(timeout["timeout_seconds"].as_u64(), Some(2_400));
    assert_eq!(timeout["budget_seconds"].as_u64(), Some(840));
    assert_eq!(timeout["tests_started"].as_bool(), Some(false));
    assert_eq!(timeout["junit_present"].as_bool(), Some(false));
    assert_eq!(timeout["test_failure_observed"].as_bool(), Some(false));
    assert_eq!(timeout["command_changed"].as_bool(), Some(false));
    assert_eq!(timeout["budget_changed"].as_bool(), Some(false));
    assert_eq!(timeout["timeout_changed"].as_bool(), Some(false));
    assert_eq!(timeout["promotable"].as_bool(), Some(false));
    assert_eq!(timeout["release_admission_allowed"].as_bool(), Some(false));
    assert_eq!(
        timeout["command_digest"].as_str(),
        Some("c5dcee3e2ec9668589f44d012e3b0a902fe342bbb44402f14cfda5ded4283c3c")
    );
    assert_eq!(
        timeout["registry_digest"].as_str(),
        Some("1edc459e489c1c7e1f6f292e9d1fef2eb00f46f9d1b385e9100748ea9e333d60")
    );
    assert_eq!(
        timeout["input_digest"].as_str(),
        Some("32b2acb938379beafeac9ad5b476b218d77bd977b268bf3d862ef457589ed6c8")
    );

    let warm_timeout: Value = serde_json::from_slice(
        &std::fs::read(root().join(
            "docs/testing/performance/0.74/local-runs/w12-workspace-fast-gate-warm-timeout-415b01fc.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        warm_timeout["source_commit"].as_str(),
        Some("415b01fc59fdf0829197d50832f0963a77bf3b5b")
    );
    assert_eq!(warm_timeout["outcome"].as_str(), Some("timeout"));
    assert_eq!(warm_timeout["execution_phase"].as_str(), Some("tests"));
    assert_eq!(warm_timeout["duration_ms"].as_u64(), Some(2_626_352));
    assert_eq!(
        warm_timeout["termination_overrun_ms"].as_u64(),
        Some(226_352)
    );
    assert_eq!(warm_timeout["compile_duration_seconds"].as_u64(), Some(684));
    assert_eq!(warm_timeout["tests_started"].as_bool(), Some(true));
    assert_eq!(warm_timeout["tests_scheduled"].as_u64(), Some(3_655));
    assert_eq!(warm_timeout["test_binaries"].as_u64(), Some(477));
    assert_eq!(warm_timeout["tests_skipped"].as_u64(), Some(82));
    assert_eq!(warm_timeout["junit_present"].as_bool(), Some(false));
    assert_eq!(warm_timeout["test_failure_reported"].as_bool(), Some(false));
    assert_eq!(
        warm_timeout["test_completion_proven"].as_bool(),
        Some(false)
    );
    assert_eq!(warm_timeout["timeout_seconds"].as_u64(), Some(2_400));
    assert_eq!(warm_timeout["budget_seconds"].as_u64(), Some(840));
    assert_eq!(warm_timeout["command_changed"].as_bool(), Some(false));
    assert_eq!(warm_timeout["budget_changed"].as_bool(), Some(false));
    assert_eq!(warm_timeout["timeout_changed"].as_bool(), Some(false));
    assert_eq!(warm_timeout["promotable"].as_bool(), Some(false));
    assert_eq!(
        warm_timeout["release_admission_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        warm_timeout["retry_count_after_this_result"].as_u64(),
        Some(0)
    );
}

#[test]
fn w11_linux_fixture_harness_cli_is_platform_gated() {
    let source =
        std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src/main.rs")).unwrap();
    assert!(source.contains(
        "#[cfg(target_os = \"linux\")]\n        [command] if command == \"campaign-fixture-harness\" => campaign_fixture_harness(),"
    ));
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
    assert!(workflow.contains("workflow_call:"));
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

    let entry = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-capability-074.yml"),
    )
    .unwrap();
    for required in [
        "- controller",
        "- controller-start-abort-rehearsal",
        "inputs.mode == 'controller'",
        "inputs.mode == 'controller-start-abort-rehearsal'",
        "uses: ./.github/workflows/performance-long-run-qualification-074.yml",
        "source_sha: ${{ inputs.controller_source_sha }}",
        "operation: ${{ inputs.controller_operation }}",
        "approval_nonce_sha256: ${{ inputs.approval_nonce_sha256 }}",
        "start_bundle_run_id: ${{ inputs.start_bundle_run_id }}",
        "start_bundle_artifact_name: ${{ inputs.start_bundle_artifact_name }}",
        "secrets: inherit",
    ] {
        assert!(
            entry.contains(required),
            "controller entrypoint omitted {required}"
        );
    }
    for required in [
        "needs: controller-rehearsal-start",
        "needs: controller-rehearsal-attach",
        "operation: start",
        "operation: attach",
        "operation: abort",
        "expected_state_revision: \"0\"",
        "expected_state_revision: \"2\"",
        "expected_state_revision: \"3\"",
        "request_id: ${{ inputs.attach_request_id }}",
        "request_id: ${{ inputs.abort_request_id }}",
    ] {
        assert!(
            entry.contains(required),
            "same-run controller rehearsal omitted {required}"
        );
    }
    assert_eq!(
        entry
            .matches("uses: ./.github/workflows/performance-long-run-qualification-074.yml")
            .count(),
        12
    );
}

#[test]
fn host_observation_workflow_is_read_only_fixed_scope_and_socket_bound() {
    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-observation-074.yml"),
    )
    .unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    for required in [
        "workflow_call:",
        "workflow_dispatch:",
        "group: long-run-074-${{ inputs.host_id }}",
        "cancel-in-progress: false",
        "runs-on: [self-hosted, linux, x64, hydracache-release]",
        "environment: performance-reference-074",
        "operation\": \"host_observation",
        "2dd3f4aa960289c385aad3988d988533197269368f851e5834f535a753cd92ba",
        "baa6e451a7248643e7a96fc5908ac07e0c97d38255e31b1bb1494ea5ac26c6ec",
        "/run/hydracache-perf/supervisor-v1.sock",
        "/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074",
        "installed_source_commit",
        "active_campaign_absent",
        "installed_fixture_binary",
        "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture",
        "installed_supervisor_binary_sha256",
        "peer_admission_required",
        "product_candidate_started",
        "promotable\": False",
    ] {
        assert!(workflow.contains(required), "workflow omitted {required}");
    }
    for forbidden in [
        "sudo -n",
        "systemctl stop",
        "request-start",
        "performance-long-run-qualification-074",
        "sleep ",
    ] {
        assert!(
            !workflow.contains(forbidden),
            "read-only host observation workflow contains {forbidden}"
        );
    }
    let collector = workflow
        .split_once("- name: Collect and independently bind the host receipt")
        .unwrap()
        .1;
    assert!(
        collector.contains("import re"),
        "fixture digest validation must import its regular-expression dependency"
    );

    let entry = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-capability-074.yml"),
    )
    .unwrap();
    assert!(entry.contains("- host-observation"));
    assert!(entry.contains("inputs.mode == 'host-observation'"));
    assert!(
        entry.contains("uses: ./.github/workflows/performance-long-run-host-observation-074.yml")
    );
}

#[test]
fn non_product_start_bundle_workflow_is_observation_bound_and_does_not_start() {
    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-start-bundle-074.yml"),
    )
    .unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    for required in [
        "workflow_call:",
        "workflow_dispatch:",
        "uses: ./.github/workflows/performance-long-run-host-observation-074.yml",
        "runs-on: ubuntu-latest",
        "performance_non_product_start_bundle_074.py",
        "test_performance_non_product_start_bundle_074",
        "--verify-start-bundle",
        "long-run-074-start-bundle-${{ steps.bundle.outputs.bundle_sha256 }}",
        "long-run-074-start-bundle-evidence-${{ steps.bundle.outputs.bundle_sha256 }}",
    ] {
        assert!(workflow.contains(required), "workflow omitted {required}");
    }
    for forbidden in [
        "request-start",
        "HYDRACACHE_074_AUTH_SIGNING_KEY_HEX",
        "sudo -n",
        "systemctl stop",
    ] {
        assert!(
            !workflow.contains(forbidden),
            "bundle-only workflow contains {forbidden}"
        );
    }

    let assembler = std::fs::read_to_string(
        root().join("scripts/perf/performance_non_product_start_bundle_074.py"),
    )
    .unwrap();
    for required in [
        "non-product-start-fixture.toml",
        "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture",
        "campaign-start-rehearsal-harness",
        "signed_start_dispatched\": False",
        "campaign_state_mutated\": False",
        "product_candidate_started\": False",
        "promotable\": False",
    ] {
        assert!(assembler.contains(required), "assembler omitted {required}");
    }
    let fixture = std::fs::read_to_string(
        root().join("docs/testing/performance/0.74/non-product-start-fixture.toml"),
    )
    .unwrap();
    for required in [
        "product_candidate = false",
        "promotable = false",
        "role_started = \"i74\"",
        "cleanup = \"explicit-signed-abort\"",
    ] {
        assert!(
            fixture.contains(required),
            "fixture contract omitted {required}"
        );
    }

    let entry = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-capability-074.yml"),
    )
    .unwrap();
    assert!(entry.contains("- start-bundle"));
    assert!(entry.contains("inputs.mode == 'start-bundle'"));
    assert!(entry.contains("performance-long-run-start-bundle-074.yml"));
}

#[test]
fn w11_host_observation_evidence_is_exact_non_mutating_and_non_promotable() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    let relative = implementation["host_observation_socket_evidence"]
        .as_str()
        .unwrap();
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(root().join(relative)).unwrap()).unwrap();

    let source = "127ebc6cb24191d567772966586bdb827a0ae380";
    assert_eq!(evidence["source_commit"].as_str(), Some(source));
    assert_eq!(
        evidence["provisioning"]["run_id"].as_u64(),
        Some(37394441292)
    );
    assert_eq!(
        evidence["observation"]["run_id"].as_u64(),
        Some(37394832233)
    );
    assert_eq!(
        evidence["observation"]["installed_source_commit"].as_str(),
        Some(source)
    );
    assert_eq!(
        evidence["observation"]["campaign_state_mutated"].as_bool(),
        Some(false)
    );
    assert_eq!(evidence["product_candidate_started"].as_bool(), Some(false));
    assert_eq!(
        evidence["expensive_qualification_started"].as_bool(),
        Some(false)
    );
    assert_eq!(evidence["promotable"].as_bool(), Some(false));
    assert_eq!(
        evidence["decision"]["host_observation_socket_export_complete"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["decision"]["privileged_start_bundle_host_rehearsal_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["post_run_host_state"]["runner_active"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["post_run_host_state"]["supervisor_active"].as_bool(),
        Some(true)
    );
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
    assert!(
        workflow.contains("if: github.event_name == 'workflow_dispatch' && inputs.mode == 'probe'")
    );
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
        "supervisor-overhead-smoke",
        "role-overhead-smoke",
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

    let overhead = std::fs::read_to_string(
        root().join("scripts/perf/performance_long_run_supervisor_overhead_074.py"),
    )
    .unwrap();
    assert!(overhead.contains("hydracache-w11-supervisor-idle-overhead-v1"));
    assert!(overhead.contains("\"role_overhead_qualification_complete\": False"));
    assert!(overhead.contains("\"release_admission_allowed\": False"));
    assert!(workflow.contains("--duration-seconds 30"));
    assert!(workflow.contains("--maximum-cpu-percent 0.5"));
    assert!(workflow.contains("--maximum-rss-bytes 67108864"));
    assert!(workflow.contains("--maximum-io-bytes-per-second 1048576"));
    assert!(workflow.contains("performance_long_run_role_overhead_collect_074.py"));
    assert!(workflow.contains("--supervisor-pid \"$SUPERVISOR_PID\""));
    assert!(workflow.contains("--control-group \"$SUPERVISOR_CONTROL_GROUP\""));
    assert!(!workflow
        .contains("sudo -n /usr/local/sbin/hydracache-provision-host-074 --role-overhead-smoke"));
    assert!(workflow.contains("--expected-i74-source \"$INSTALLED_SOURCE_COMMIT\""));
    assert!(workflow.contains("--expected-c74-source \"$INSTALLED_SOURCE_COMMIT\""));
    assert!(workflow.contains(
        "assert receipt[\"decision\"][\"role_overhead_qualification_complete\"] is False"
    ));
    let overhead_job = workflow
        .split("\n  supervisor-overhead-smoke:")
        .nth(1)
        .expect("isolated supervisor overhead job");
    for forbidden in [
        "systemctl start",
        "systemctl stop",
        "systemctl restart",
        "--campaign-lifecycle-smoke-start",
        "performance-long-run-qualification-074",
    ] {
        assert!(
            !overhead_job.contains(forbidden),
            "overhead screen must remain read-only: {forbidden}"
        );
    }
}

#[test]
fn w12_ordinary_ci_preserves_explicit_infrastructure_boundary() {
    let read = |name: &str| {
        let text = std::fs::read_to_string(root().join(".github/workflows").join(name)).unwrap();
        serde_yaml::from_str::<serde_yaml::Value>(&text).unwrap()
    };
    let ci = read("ci.yml");
    let inputs = &ci["on"]["workflow_dispatch"]["inputs"];
    for flag in [
        "run_nightly",
        "run_reference_performance",
        "run_memory_diagnostic",
        "run_retention_soak_070",
        "run_management_candidate_soak_072",
        "run_management_ship_soak_072",
        "run_management_mixed_072",
        "run_redis_compat_release_proof",
    ] {
        assert_eq!(inputs[flag]["default"].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(
        inputs["performance_0671_mode"]["default"].as_str(),
        Some("off")
    );
    for (file, job) in [
        ("performance-kernel-attribution-074.yml", "attribute"),
        ("performance-long-run-host-capability-074.yml", "probe"),
    ] {
        let workflow = read(file);
        let condition = workflow["jobs"][job]["if"].as_str().unwrap();
        assert!(condition.contains("github.event_name == 'workflow_dispatch'"));
        assert!(
            !condition.contains("||"),
            "push must not select {file}#{job}"
        );
    }
    let bundle = read("performance-long-run-start-bundle-074.yml");
    let group = bundle["concurrency"]["group"].as_str().unwrap();
    assert!(group.starts_with("long-run-074-start-bundle-"));
    for marker in ["inputs.host_id", "inputs.source_sha", "github.run_id"] {
        assert!(group.contains(marker));
    }
    assert_eq!(
        bundle["concurrency"]["cancel-in-progress"].as_bool(),
        Some(false)
    );
}

#[test]
fn w12_internal_workspace_tools_inherit_reviewed_license() {
    let workspace: toml::Value =
        toml::from_str(&std::fs::read_to_string(root().join("Cargo.toml")).unwrap()).unwrap();
    assert_eq!(
        workspace["workspace"]["package"]["license"].as_str(),
        Some("Apache-2.0")
    );
    let mut checked = 0;
    for member in workspace["workspace"]["members"].as_array().unwrap() {
        let member = member.as_str().unwrap();
        if !member.starts_with("tools/") {
            continue;
        }
        let manifest: toml::Value = toml::from_str(
            &std::fs::read_to_string(root().join(member).join("Cargo.toml")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest["package"]["publish"].as_bool(),
            Some(false),
            "{member}"
        );
        assert_eq!(
            manifest["package"]
                .get("license")
                .and_then(|license| license.get("workspace"))
                .and_then(toml::Value::as_bool),
            Some(true),
            "internal tool must inherit the reviewed workspace license: {member}"
        );
        checked += 1;
    }
    assert!(checked >= 2, "both 0.74 workspace tools must be checked");
}

#[test]
fn w12_live_registries_preserve_frozen_baseline_inputs() {
    let identities: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("docs/testing/memory/0.71/baseline-identities.toml"))
            .unwrap(),
    )
    .unwrap();
    for input in identities["scenario"]["input"].as_array().unwrap() {
        let path = input["path"].as_str().unwrap();
        let bytes = std::fs::read(root().join(path)).unwrap();
        let text = String::from_utf8(bytes).unwrap().replace("\r\n", "\n");
        assert_eq!(
            Sha256::digest(text.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            input["sha256"].as_str().unwrap(),
            "historical prerequisite must not be rewritten to register live CI: {path}"
        );
    }
}
