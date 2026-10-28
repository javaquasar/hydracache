use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const FILES: [&str; 11] = [
    "status.json",
    "operation-contract.json",
    "resource-bounds.json",
    "surface-equivalence.json",
    "mutation-stages.json",
    "retry-idempotency.json",
    "failure-consistency-matrix.json",
    "rpo-rto-contract.json",
    "security-contract.json",
    "canonical-key-vectors.json",
    "test-registry.json",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn fixture() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let source = root().join("docs/testing/imap/0.75");
    for file in FILES {
        fs::copy(source.join(file), temp.path().join(file)).unwrap();
    }
    temp
}

fn edit(path: &Path, mutate: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    mutate(&mut value);
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn problems(temp: &TempDir) -> Vec<String> {
    xtask::imap_contract::check_contract_dir(temp.path()).unwrap()
}

#[test]
fn repository_contract_is_complete_and_provisional() {
    let problems = xtask::imap_contract::check_at_root(&root(), "0.75").unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn command_accepts_release_075_contracts() {
    let status = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(root())
        .args(["imap-contract-check", "--release", "0.75"])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn canary_missing_test_id_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        value["operations"][0]["test_ids"] = Value::Array(Vec::new());
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("missing test id")));
}

#[test]
fn canary_unmapped_and_ambiguous_test_claims_fail_closed() {
    let temp = fixture();
    edit(&temp.path().join("test-registry.json"), |value| {
        value["mappings"][0]["id_prefix"] = Value::String("unmatched::".into());
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("exactly one executable mapping")));

    let temp = fixture();
    edit(&temp.path().join("test-registry.json"), |value| {
        let duplicate = value["mappings"][0].clone();
        value["mappings"].as_array_mut().unwrap().push(duplicate);
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("duplicate test registry prefix")));
}

#[test]
fn registry_targets_must_name_real_test_functions() {
    let temp = tempfile::tempdir().unwrap();
    let contract_dir = temp.path().join("docs/testing/imap/0.75");
    fs::create_dir_all(&contract_dir).unwrap();
    let source = root().join("docs/testing/imap/0.75");
    for file in FILES {
        fs::copy(source.join(file), contract_dir.join(file)).unwrap();
    }
    let registry: Value =
        serde_json::from_slice(&fs::read(contract_dir.join("test-registry.json")).unwrap())
            .unwrap();
    let mut by_path = std::collections::BTreeMap::<String, String>::new();
    for mapping in registry["mappings"].as_array().unwrap() {
        by_path
            .entry(mapping["path"].as_str().unwrap().to_owned())
            .or_default()
            .push_str(&format!(
                "fn {}() {{}}\n",
                mapping["test"].as_str().unwrap()
            ));
    }
    for (path, contents) in by_path {
        let path = temp.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    assert!(xtask::imap_contract::check_at_root(temp.path(), "0.75")
        .unwrap()
        .is_empty());
    edit(&contract_dir.join("test-registry.json"), |value| {
        value["mappings"][0]["test"] = Value::String("missing_test_function".into());
    });
    assert!(xtask::imap_contract::check_at_root(temp.path(), "0.75")
        .unwrap()
        .iter()
        .any(|problem| problem.contains("does not contain test")));
}

#[test]
fn canary_missing_surface_cell_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("surface-equivalence.json"), |value| {
        value["surfaces"][0]["cells"].as_array_mut().unwrap().pop();
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("missing operation cell")));
}

#[test]
fn collection_surface_matrix_is_explicit_and_fail_closed() {
    assert_surface_matrix(&[
        "size",
        "is_empty",
        "contains_value_bounded",
        "clear_detailed",
        "destroy",
        "evict",
        "evict_all_detailed",
        "key_set_page",
        "values_page",
        "entry_set_page",
    ]);
}

#[test]
fn extended_surface_matrix_is_explicit_and_fail_closed() {
    assert_surface_matrix(&[
        "get_or_default",
        "contains_entry",
        "set",
        "get_entry_view",
        "try_put",
        "try_remove",
        "replace_all_detailed",
        "remove_entry_listener",
        "capability_descriptor",
    ]);
}

fn assert_surface_matrix(operations: &[&str]) {
    let contract: Value = serde_json::from_slice(
        &fs::read(root().join("docs/testing/imap/0.75/surface-equivalence.json")).unwrap(),
    )
    .unwrap();
    for surface in contract["surfaces"].as_array().unwrap() {
        let id = surface["id"].as_str().unwrap();
        let cells = surface["cells"].as_array().unwrap();
        for operation in operations {
            let matches = cells
                .iter()
                .filter(|cell| cell["operation"] == *operation)
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 1, "{id}:{operation}");
            let cell = matches[0];
            if matches!(id, "hc2-rust" | "hc2-java" | "hazelcast-facade") {
                assert_eq!(cell["disposition"], "enabled", "{id}:{operation}");
                assert!(cell["unsupported_error"].is_null(), "{id}:{operation}");
            } else {
                assert_eq!(cell["disposition"], "unsupported", "{id}:{operation}");
                assert!(
                    cell["unsupported_error"]
                        .as_str()
                        .is_some_and(|error| !error.is_empty()),
                    "{id}:{operation}"
                );
            }
        }
    }
}

#[test]
fn canary_missing_mutation_stage_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("mutation-stages.json"), |value| {
        value["stages"].as_array_mut().unwrap().remove(5);
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("mutation stages missing required id")));
}

#[test]
fn canary_timed_api_requires_all_certainties_and_retained_identity() {
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        value["api_extensions"]["deadline_outcomes"] =
            serde_json::json!(["applied", "not_applied"]);
        for operation in value["operations"].as_array_mut().unwrap() {
            if operation["id"] == "try_put" {
                operation["retry_class"] = Value::String("new_identity_allowed".into());
            }
        }
    });
    let errors = problems(&temp);
    assert!(errors.iter().any(|problem| problem
        .contains("deadline certainty outcomes missing required id outcome_unknown")));
    assert!(errors
        .iter()
        .any(|problem| problem.contains("timed operation try_put must retain certainty")));
}

#[test]
fn canary_replacement_callbacks_and_unbounded_work_are_rejected() {
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        value["api_extensions"]["replacement_mode"] = Value::String("remote_callback".into());
        for operation in value["operations"].as_array_mut().unwrap() {
            if operation["id"] == "replace_all_detailed" {
                operation["bound_ids"] = serde_json::json!(["response_bytes"]);
                operation["ttl_policy"] = serde_json::json!(["eternal", "expire_after"]);
                operation["errors"] = serde_json::json!(["invalid_argument"]);
                operation["retry_class"] = serde_json::json!("new_identity_allowed");
            }
        }
    });
    let errors = problems(&temp);
    assert!(errors
        .iter()
        .any(|problem| problem.contains("replacement mode must be explicit_bounded_entries")));
    assert!(errors
        .iter()
        .any(|problem| problem
            .contains("replace_all_detailed must bound explicit replacement entries")));
    assert!(errors
        .iter()
        .any(|problem| problem.contains("replace_all_detailed must preserve existing expiry")));
    assert!(errors.iter().any(|problem| problem.contains(
        "replace_all_detailed must expose retained-identity conflict and capacity errors"
    )));
    assert!(errors.iter().any(|problem| problem.contains(
        "replace_all_detailed must retry the unchanged request with the retained identity"
    )));
}

#[test]
fn canary_bulk_replacement_requires_each_aggregate_and_replay_bound() {
    for missing in [
        "request_entries",
        "request_bytes",
        "response_bytes",
        "dedup_entries_per_tenant",
    ] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            let operation = value["operations"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|operation| operation["id"] == "replace_all_detailed")
                .unwrap();
            operation["bound_ids"]
                .as_array_mut()
                .unwrap()
                .retain(|bound| bound != missing);
        });
        assert!(problems(&temp).iter().any(|problem| problem.contains(
            "replace_all_detailed must bound explicit replacement entries, bytes and retained outcomes"
        )), "missing {missing}");
    }
}

#[test]
fn canary_async_capabilities_cannot_enable_production_or_hide_cancellation() {
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        value["api_extensions"]["production_enabled"] = Value::Bool(true);
        value["api_extensions"]["interface_features"][0]["cancellation"] =
            Value::String("always_not_applied".into());
        value["api_extensions"]["interface_features"][1]["test_ids"] = serde_json::json!([]);
        value["api_extensions"]["unsupported_families"] = serde_json::json!([]);
    });
    let errors = problems(&temp);
    assert!(errors
        .iter()
        .any(|problem| problem.contains("production capability must remain disabled")));
    assert!(errors
        .iter()
        .any(|problem| problem.contains("unsafe cancellation semantics")));
    assert!(errors
        .iter()
        .any(|problem| problem.contains("java_async_facade has a missing test id")));
    assert!(errors
        .iter()
        .any(|problem| problem
            .contains("API unsupported families missing required id unbounded_scan")));
}

#[test]
fn canary_entry_view_and_interface_metadata_are_required() {
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        value["api_extensions"]["entry_view_fields"] =
            serde_json::json!(["value", "version", "last_access_timestamp"]);
        value["api_extensions"]["interface_features"] = serde_json::json!([]);
    });
    let errors = problems(&temp);
    assert!(errors
        .iter()
        .any(|problem| problem.contains("entry view must contain only")));
    assert!(errors
        .iter()
        .any(|problem| problem
            .contains("API interface features missing required id rust_async_facade")));

    edit(&temp.path().join("operation-contract.json"), |value| {
        value.as_object_mut().unwrap().remove("api_extensions");
    });
    assert!(xtask::imap_contract::check_contract_dir(temp.path()).is_err());
}

#[test]
fn canary_extension_identity_clock_and_duplicate_features_are_rejected() {
    for (field, wrong, expected) in [
        ("deadline_clock", "wall_clock", "deadline clock"),
        ("mutation_identity", "optional", "timed mutation identity"),
        (
            "retry_after_admission",
            "new_identity_allowed",
            "retry after admission",
        ),
    ] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            value["api_extensions"][field] = Value::String(wrong.into());
        });
        assert!(problems(&temp)
            .iter()
            .any(|problem| problem.contains(expected)));
    }
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        let duplicate = value["api_extensions"]["interface_features"][0].clone();
        value["api_extensions"]["interface_features"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        value["api_extensions"]["interface_features"][2]["cancellation"] =
            Value::String("silently_restart".into());
    });
    let errors = problems(&temp);
    assert!(errors
        .iter()
        .any(|problem| problem.contains("duplicate API interface feature rust_async_facade")));
    assert!(errors
        .iter()
        .any(|problem| problem
            .contains("cursor_iterator_adapters has unsafe cancellation semantics")));
}

#[test]
fn canary_timed_remove_cannot_drop_unknown_outcomes_or_dedup_bound() {
    for field in ["outcomes", "bound_ids"] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            let operation = value["operations"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|operation| operation["id"] == "try_remove")
                .unwrap();
            operation[field] = if field == "outcomes" {
                serde_json::json!(["applied", "not_applied"])
            } else {
                serde_json::json!(["key_bytes", "request_bytes"])
            };
        });
        assert!(problems(&temp)
            .iter()
            .any(|problem| problem.contains("timed operation try_remove must retain certainty")));
    }
}

#[test]
fn canary_missing_bound_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("resource-bounds.json"), |value| {
        value["bounds"].as_array_mut().unwrap().remove(0);
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("bounds missing required id")));
}

#[test]
fn canary_missing_proof_id_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("rpo-rto-contract.json"), |value| {
        value["cells"][0]["proof_id"] = Value::String(String::new());
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("proof_id must not be empty")));
}

#[test]
fn canary_finalized_w0_claim_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("status.json"), |value| {
        value["status"] = Value::String("finalized".into());
        value["w0_disposition"] = Value::String("closed".into());
    });
    let problems = problems(&temp);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("provisional")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("w0_disposition=open")));
}

#[test]
fn canary_missing_safe_foundation_slice_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("status.json"), |value| {
        value["safe_foundation"]["implemented"]
            .as_array_mut()
            .unwrap()
            .retain(|slice| slice != "linearizability_oracle");
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("missing implemented slice linearizability_oracle")));
}

#[test]
fn canary_missing_security_threat_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("security-contract.json"), |value| {
        value["threats"]
            .as_array_mut()
            .unwrap()
            .retain(|threat| threat["id"] != "false_backup_ack");
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("security threats missing required id false_backup_ack")));
}

#[test]
fn canary_assigning_production_key_identity_fails_closed() {
    let temp = fixture();
    edit(&temp.path().join("canonical-key-vectors.json"), |value| {
        value["production_partition_hash"] = Value::String("fnv1a-final".into());
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("must not allocate production identities")));
}

#[test]
fn unified_surface_matrix_is_explicit_and_fail_closed() {
    let contract: Value = serde_json::from_slice(
        &fs::read(root().join("docs/testing/imap/0.75/surface-equivalence.json")).unwrap(),
    )
    .unwrap();
    for surface in contract["surfaces"].as_array().unwrap() {
        for operation in [
            "replace_if_version",
            "remove_if_version",
            "set_ttl_if_version",
            "query_operation_outcome",
            "local_statistics",
        ] {
            let cells = surface["cells"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|cell| cell["operation"] == operation)
                .collect::<Vec<_>>();
            assert_eq!(cells.len(), 1);
            let unsupported = matches!(surface["id"].as_str().unwrap(), "resp" | "hc1");
            assert_eq!(
                cells[0]["disposition"],
                if unsupported {
                    "unsupported"
                } else {
                    "enabled"
                }
            );
            assert_eq!(cells[0]["unsupported_error"].is_string(), unsupported);
        }
    }
}

#[test]
fn canary_local_service_cannot_split_authority_or_invent_absent_outcomes() {
    for (field, replacement) in [
        ("value_owner", serde_json::json!("second_shadow_map")),
        ("outcome_owner", serde_json::json!("independent_history")),
        ("missing_outcome", serde_json::json!("not_applied")),
        ("version_scope", serde_json::json!(["entry_version"])),
        ("query_outcomes", serde_json::json!(["completed", "absent"])),
    ] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            value["api_extensions"]["local_service"][field] = replacement;
        });
        assert!(
            problems(&temp)
                .iter()
                .any(|problem| problem.starts_with("local IMap service")),
            "{field}"
        );
    }
}

#[test]
fn canary_stream_listener_stats_and_differential_cannot_weaken_their_contract() {
    for (field, replacement) in [
        ("include_value", serde_json::json!("unbounded_default")),
        ("stream_buffer_pages", serde_json::json!(0)),
        ("stream_terminal", serde_json::json!("drop_silently")),
        ("statistics_scope", serde_json::json!("cluster_aggregate")),
        ("listener_kinds", serde_json::json!(["added"])),
        (
            "differential_observations",
            serde_json::json!(["command_hash"]),
        ),
        ("test_ids", serde_json::json!([])),
    ] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            value["api_extensions"]["local_service"][field] = replacement;
        });
        assert!(
            problems(&temp)
                .iter()
                .any(|problem| problem.starts_with("local IMap service")),
            "{field}"
        );
    }
}

#[test]
fn canary_retirement_budgets_and_repair_cannot_drop_owner_guards() {
    for (field, replacement) in [
        ("retirement_scope", serde_json::json!(["client"])),
        (
            "retirement_scope",
            serde_json::json!(["client", "namespace_generation", "client"]),
        ),
        ("retired_retry", serde_json::json!("reexecute_if_missing")),
        ("pending_retirement", serde_json::json!("evict_pending")),
        ("accounting_scope", serde_json::json!("unmeasured_rss")),
        ("resource_owners", serde_json::json!(["values"])),
        ("admission", serde_json::json!("check_after_mutation")),
        (
            "listener_repair",
            serde_json::json!("register_after_snapshot"),
        ),
        (
            "repair_bounds",
            serde_json::json!("clear_gap_then_check_size"),
        ),
    ] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            value["api_extensions"]["local_lifecycle"][field] = replacement;
        });
        assert!(
            problems(&temp)
                .iter()
                .any(|problem| problem.starts_with("local IMap lifecycle")),
            "{field}"
        );
    }
}

#[test]
fn canary_bulk_expiry_checkpoint_and_concurrency_preserve_local_limits() {
    for (field, replacement) in [
        ("version_bulk", serde_json::json!("cross_key_transaction")),
        (
            "batch_outcomes",
            serde_json::json!("missing_means_not_applied"),
        ),
        ("expiry_work", serde_json::json!("full_sweep_per_request")),
        (
            "expired_visibility",
            serde_json::json!("visible_until_cleanup"),
        ),
        ("checkpoint", serde_json::json!("production_durable_format")),
        (
            "qualification",
            serde_json::json!("identical_winner_required"),
        ),
        ("test_ids", serde_json::json!([])),
    ] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            value["api_extensions"]["local_lifecycle"][field] = replacement;
        });
        assert!(
            problems(&temp)
                .iter()
                .any(|problem| problem.starts_with("local IMap lifecycle")),
            "{field}"
        );
    }
}

#[test]
fn lifecycle_contract_requires_both_implementations_and_unique_proofs() {
    for field in ["local_lifecycle", "local_service"] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            value["api_extensions"]
                .as_object_mut()
                .unwrap()
                .remove(field);
        });
        assert!(xtask::imap_contract::check_contract_dir(temp.path()).is_err());
    }
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        value["api_extensions"]["local_lifecycle"]["test_ids"]
            .as_array_mut()
            .unwrap()
            .pop();
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("each unique Rust/Java")));
    let temp = fixture();
    edit(&temp.path().join("operation-contract.json"), |value| {
        let tests = value["api_extensions"]["local_lifecycle"]["test_ids"]
            .as_array_mut()
            .unwrap();
        tests.push(tests[0].clone());
    });
    assert!(problems(&temp)
        .iter()
        .any(|problem| problem.contains("each unique Rust/Java")));
    for error in [
        "retired_identity",
        "pending_retirement",
        "resource_budget_exhausted",
        "active_owners",
        "expiry_backlog",
    ] {
        let temp = fixture();
        edit(&temp.path().join("operation-contract.json"), |value| {
            value["errors"]
                .as_array_mut()
                .unwrap()
                .retain(|item| item["id"] != error);
        });
        assert!(
            problems(&temp)
                .iter()
                .any(|problem| problem.contains("missing typed error")),
            "{error}"
        );
    }
}
