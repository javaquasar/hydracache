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
fn nextest_serializes_trybuild_harnesses_with_a_bounded_compile_timeout() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let config = fs::read_to_string(root.join(".config/nextest.toml")).unwrap();
    let config: toml::Value = toml::from_str(&config).unwrap();
    assert_eq!(
        config["test-groups"]["trybuild"]["max-threads"].as_integer(),
        Some(1)
    );
    let compile_override = config["profile"]["ci"]["overrides"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["test-group"].as_str() == Some("trybuild"))
        .unwrap();
    let filter = compile_override["filter"].as_str().unwrap();
    assert!(filter.contains("cacheable_macro_compile_tests"));
    assert!(filter.contains("proc_macro_compile_tests"));
    assert_eq!(
        compile_override["slow-timeout"]["period"].as_str(),
        Some("120s")
    );
    assert_eq!(
        compile_override["slow-timeout"]["terminate-after"].as_integer(),
        Some(3)
    );
}

#[test]
fn fast_suite_check_rejects_invented_baseline_and_aggregate_budget_overrun() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let mut registry = xtask::fast_suite::load_registry(&root).unwrap();
    registry.suite[0].baseline.commit = "f".repeat(40);
    registry.suite[0].budget_seconds = registry.aggregate_budget_seconds + 1;
    let problems = xtask::fast_suite::validate_registry(&root, &registry, "0.64", None).unwrap();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W6") {
        assert!(
            problems.is_empty(),
            "HC-CANARY-RED:W6 unreviewed fast-suite budget regression was accepted"
        );
    }
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

#[test]
fn fast_suite_registry_rejects_missing_timeout_budget_or_command() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let mut registry = xtask::fast_suite::load_registry(&root).unwrap();
    registry.suite[0].timeout_seconds = 0;
    registry.suite[1].budget_seconds = 0;
    registry.suite[2].command.program.clear();
    let problems = xtask::fast_suite::validate_registry(&root, &registry, "0.64", None).unwrap();
    assert_eq!(
        problems
            .iter()
            .filter(|problem| problem.contains("incomplete execution contract"))
            .count(),
        3
    );
}

#[test]
fn fast_suite_budget_rejects_an_unreviewed_runtime_regression() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let mut registry = xtask::fast_suite::load_registry(&root).unwrap();
    registry.suite[0].budget_seconds = registry.aggregate_budget_seconds + 1;
    let problems = xtask::fast_suite::validate_registry(&root, &registry, "0.64", None).unwrap();
    assert!(problems.iter().any(|problem| problem.contains("above")));
}
