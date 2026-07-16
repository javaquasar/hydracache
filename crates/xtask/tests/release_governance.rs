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
  coverage-ratchet:
    steps:
      - uses: actions/checkout@v5
  msrv:
    steps:
      - uses: actions/checkout@v5
  gated-proof-registry:
    steps:
      - uses: actions/checkout@v5
"#;
    let problems = xtask::release_governance::release_history_checkout_problems(shallow).unwrap();
    assert_eq!(problems.len(), 5, "{problems:#?}");
    assert!(problems.iter().any(|problem| problem.contains("job rust")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("job dynamic-canary-sweep")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("job coverage-ratchet")));
    assert!(problems.iter().any(|problem| problem.contains("job msrv")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("job gated-proof-registry")));
}

#[test]
fn crates_io_probe_identifies_itself_and_retries_transient_responses() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/publish-crates.yml")).unwrap();
    let problems = xtask::release_governance::publish_workflow_problems(&workflow);
    assert!(problems.is_empty(), "{problems:#?}");

    let anonymous = workflow.replacen("--user-agent", "--anonymous-probe", 1);
    let problems = xtask::release_governance::publish_workflow_problems(&anonymous);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("--user-agent")));

    let no_retry = workflow.replacen("429|5??)", "429)", 1);
    let problems = xtask::release_governance::publish_workflow_problems(&no_retry);
    assert!(problems.iter().any(|problem| problem.contains("429|5??)")));
}

#[test]
fn publish_order_keeps_workspace_dev_and_build_dependencies() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/publish-crates.yml")).unwrap();

    let normal_only = workflow.replacen(
        "if dependency_id in publishable_ids:",
        "if (\n                      dependency_id in publishable_ids\n                      and any(kind.get(\"kind\") is None for kind in dependency.get(\"dep_kinds\", []))\n                  ):",
        1,
    );
    let problems = xtask::release_governance::publish_workflow_problems(&normal_only);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dev/build dependencies")));

    let client_manifest =
        std::fs::read_to_string(root.join("crates/hydracache-client/Cargo.toml")).unwrap();
    assert!(client_manifest.contains("[dev-dependencies]"));
    assert!(client_manifest.contains("hydracache-client-transport-axum.workspace = true"));
}

#[test]
fn post_publish_consumer_tracks_the_current_public_api() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/post-publish.yml")).unwrap();
    let fixture =
        std::fs::read_to_string(root.join("tests/post-publish-consumer/src/lib.rs")).unwrap();
    let problems = xtask::release_governance::post_publish_contract_problems(&workflow, &fixture);
    assert!(problems.is_empty(), "{problems:#?}");

    let stale = fixture
        .replacen(".diesel_one(", ".diesel_first(", 1)
        .replacen(".sea_one(", ".sea_value(", 1)
        .replacen(
            "ownership_diagnostics.resolutions",
            "cluster_diagnostics.ownership_resolutions",
            1,
        );
    let problems = xtask::release_governance::post_publish_contract_problems(&workflow, &stale);
    assert!(problems
        .iter()
        .any(|problem| problem.contains(".diesel_first(")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains(".sea_value(")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("ownership_resolutions")));

    let unwired = workflow.replacen(
        "tests/post-publish-consumer/src/lib.rs",
        "missing-consumer-fixture.rs",
        1,
    );
    let problems = xtask::release_governance::post_publish_contract_problems(&unwired, &fixture);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("fixture wiring")));
}
