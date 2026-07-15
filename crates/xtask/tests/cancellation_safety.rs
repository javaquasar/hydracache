use std::fs;

#[test]
fn w39_contract_registry_requires_all_three_cancellation_surfaces() {
    let root = workspace_root();
    let required = [
        (
            "crates/hydracache/tests/cancellation_safety.rs",
            "cache_drop_at_registered_boundaries_preserves_state_and_permit_baseline",
        ),
        (
            "crates/hydracache/tests/cancellation_safety.rs",
            "dropped_singleflight_loader_does_not_poison_the_slot",
        ),
        (
            "crates/hydracache-client-transport-axum/tests/cancellation_safety.rs",
            "client_drop_at_registered_boundaries_preserves_lock_token_and_ttl",
        ),
        (
            "crates/hydracache-client-transport-axum/tests/cancellation_safety.rs",
            "client_drop_does_not_leak_subscription_or_inflight_budget",
        ),
        (
            "crates/hydracache-cluster-raft/tests/cancellation_safety.rs",
            "raft_dropped_proposal_has_explicit_unknown_outcome_and_retry_is_idempotent",
        ),
        (
            "crates/hydracache-cluster-raft/tests/cancellation_safety.rs",
            "runtime_shutdown_with_inflight_ops_recovers_consistent_metadata",
        ),
    ];

    for (path, function) in required {
        let source = fs::read_to_string(root.join(path)).unwrap_or_else(|error| {
            panic!("W39 cancellation source {path} is not readable: {error}")
        });
        assert!(
            source.contains(&format!("fn {function}("))
                || source.contains(&format!("async fn {function}(")),
            "W39 cancellation source {path} is missing {function}"
        );
    }

    let plan = fs::read_to_string(
        root.join("docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md"),
    )
    .unwrap();
    assert!(plan.contains("## W39."), "W39 must remain in the 0.64 plan");
    assert!(plan.contains("W39a") && plan.contains("W39b") && plan.contains("W39c"));
}

#[test]
fn canary_noncancelsafe_fixture_leaks_a_permit_on_drop() {
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W39") {
        panic!("HC-CANARY-RED:W39 non-cancellation-safe fixture leaked a permit on drop");
    }
}

fn workspace_root() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|path| path.parent())
        .expect("xtask crate must live under workspace crates/")
        .to_path_buf()
}
