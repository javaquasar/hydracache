use std::fs;

#[test]
fn fast_suite_registry_has_pinned_nextest_and_bounded_unmeasured_baselines() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let registry = xtask::fast_suite::load_registry(&root).unwrap();
    let problems = xtask::fast_suite::validate_registry(&root, &registry, "0.64", None).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
    assert_eq!(registry.nextest_version, "0.9.137");
    assert!(registry
        .suite
        .iter()
        .all(|suite| suite.baseline.status == xtask::fast_suite::BaselineStatus::Unmeasured));
}

#[test]
fn fast_suite_check_rejects_invented_baseline_and_aggregate_budget_overrun() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let mut registry = xtask::fast_suite::load_registry(&root).unwrap();
    registry.suite[0].baseline.commit = "f".repeat(40);
    registry.suite[0].budget_seconds = registry.aggregate_budget_seconds + 1;
    let problems = xtask::fast_suite::validate_registry(&root, &registry, "0.64", None).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("invented measurements")));
    assert!(problems.iter().any(|problem| problem.contains("above")));
}

#[test]
fn fast_suite_check_rejects_slow_receipt_without_reclassifying_the_suite() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let registry = xtask::fast_suite::load_registry(&root).unwrap();
    let suite = &registry.suite[0];
    let directory = root.join("target/fast-suite-test-receipts");
    fs::create_dir_all(&directory).unwrap();
    let receipt = xtask::evidence_run::EvidenceReceipt {
        schema_version: 1,
        release: "0.64.0".to_owned(),
        gate_id: suite.id.clone(),
        source_commit: "0".repeat(40),
        dirty_worktree: false,
        command_digest: "0".repeat(64),
        registry_digest: "0".repeat(64),
        input_digest: "0".repeat(64),
        toolchain: "test".to_owned(),
        container_identity: Default::default(),
        platform: "test".to_owned(),
        started_at: "2026-07-14T00:00:00Z".to_owned(),
        ended_at: "2026-07-14T00:02:00Z".to_owned(),
        duration_ms: (suite.budget_seconds + 1) * 1_000,
        outcome: xtask::evidence_run::EvidenceOutcome::Pass,
        exit_code: Some(0),
        normalized_result: xtask::evidence_run::NormalizedResult {
            outcome: xtask::evidence_run::EvidenceOutcome::Pass,
            exit_code: Some(0),
            stdout_sha256: String::new(),
            stderr_sha256: String::new(),
        },
        stdout: String::new(),
        stderr: String::new(),
        artifacts: vec![],
        missing_artifacts: vec![],
    };
    fs::write(
        directory.join("slow.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    let problems = xtask::fast_suite::validate_registry(
        &root,
        &registry,
        "0.64",
        Some(std::path::Path::new("target/fast-suite-test-receipts")),
    )
    .unwrap();
    assert!(problems.iter().any(|problem| problem.contains("above its")));
    assert_eq!(suite.id, "fast.fuzz-corpus-regression");
    fs::remove_dir_all(directory).unwrap();
}
