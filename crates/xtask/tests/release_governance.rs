#[test]
fn release_governance_check_accepts_current_structural_meta_gates() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let report = xtask::release_governance::check(&root, "0.64").unwrap();
    assert!(report.problems.is_empty(), "{:#?}", report.problems);
    assert!(!report
        .todos
        .iter()
        .any(|todo| todo.contains("TODO-W32-COMPAT-CHECK")));
    assert!(!report
        .todos
        .iter()
        .any(|todo| todo.contains("TODO-W38-RAFT-SPEC-CHECK")));
}

#[test]
fn release_governance_check_rejects_an_unwired_or_missing_meta_gate() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let mut gate = registry.gate[0].clone();
    gate.ci.job = "missing-job".to_owned();
    let problems = xtask::release_governance::ci_wiring_problems(&root, &[gate]).unwrap();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W6b") {
        assert!(
            problems.is_empty(),
            "HC-CANARY-RED:W6b release meta-gate was not wired into CI"
        );
    }
    assert!(problems
        .iter()
        .any(|problem| problem.contains("missing job")));

    let mut gate = registry.gate[0].clone();
    gate.ci.step = "Missing step".to_owned();
    let problems = xtask::release_governance::ci_wiring_problems(&root, &[gate]).unwrap();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W6b") {
        assert!(
            problems.is_empty(),
            "HC-CANARY-RED:W6b release meta-gate step was not wired into CI"
        );
    }
    assert!(problems
        .iter()
        .any(|problem| problem.contains("missing step")));
}

#[test]
fn ci_wires_fast_and_raft_corner_case_tiers_to_declared_commands() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let workflow = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let problems = xtask::release_governance::release_execution_wiring_problems(&workflow).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");

    let broken = workflow.replacen(
        "evidence-run --release 0.64 --gate env.hydracache-grid-scope",
        "coverage-command-was-silently-removed",
        1,
    );
    let problems = xtask::release_governance::release_execution_wiring_problems(&broken).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("env.hydracache-grid-scope")));

    let broken = workflow.replacen(
        "cargo +nightly fuzz run fuzz_config_parse -- -max_total_time=60",
        "cargo +nightly fuzz run fuzz_config_parse --manifest-path fuzz/Cargo.toml -- -max_total_time=60",
        1,
    );
    let problems = xtask::release_governance::release_execution_wiring_problems(&broken).unwrap();
    assert!(problems.iter().any(|problem| {
        problem.contains("--manifest-path after the target")
            || problem.contains("fuzz_config_parse -- -max_total_time=60")
    }));
}

#[test]
fn release_compatibility_jobs_fetch_the_baseline_tag_and_ancestry() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let workflow = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let problems = xtask::release_governance::release_history_checkout_problems(&workflow).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");

    let shallow = r#"
jobs:
  rust:
    steps:
      - uses: actions/checkout@v5
  dynamic-canary-sweep:
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 1
"#;
    let problems = xtask::release_governance::release_history_checkout_problems(shallow).unwrap();
    assert_eq!(problems.len(), 2, "{problems:#?}");
    assert!(problems.iter().any(|problem| problem.contains("job rust")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("job dynamic-canary-sweep")));
}
