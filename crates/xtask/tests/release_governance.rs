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
fn release_governance_check_accepts_the_explicit_0_66_fast_wiring() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let report = xtask::release_governance::check(&root, "0.66").unwrap();
    assert!(report.problems.is_empty(), "{:#?}", report.problems);
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
    let problems =
        xtask::release_governance::release_execution_wiring_problems(&workflow, "0.66").unwrap();
    assert!(problems.is_empty(), "{problems:#?}");

    for required in [
        "operator-controller-live.log",
        "cargo build -p hydracache-operator --locked",
        "operator_binary=\"$(pwd)/target/debug/hydracache-operator\"",
        "operator_log=\"target/test-evidence/0.66/operator-controller-live.log\"",
        "operator_pid_file=\"target/test-evidence/0.66/operator-controller.pid\"",
        "operator_nonce=\"release-066-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}-${GITHUB_SHA}\"",
        "export HYDRACACHE_OPERATOR_EVIDENCE_NONCE=\"$operator_nonce\"",
        "echo \"HYDRACACHE_OPERATOR_EVIDENCE_NONCE=$operator_nonce\" >> \"$GITHUB_ENV\"",
        "HC-OPERATOR-CONTROLLER-START nonce=%s binary=%s",
        "\"$operator_nonce\" \"$operator_binary\" > \"$operator_log\"",
        "printf '%s\\n' \"$BASHPID\" > \"$operator_pid_file\"",
        "exec \"$operator_binary\" >> \"$operator_log\" 2>&1",
        "kill -0 \"$operator_pid\"",
        "HC-OPERATOR-CONTROLLER-RUNTIME nonce=$operator_nonce",
        "operator-kind-pod-logs-post.txt",
    ] {
        let broken = workflow.replace(required, "operator-evidence-was-removed");
        assert_ne!(
            broken, workflow,
            "operator fixture marker was not found: {required}"
        );
        let problems =
            xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
        assert!(
            problems.iter().any(|problem| problem.contains(required)),
            "missing operator evidence wiring was accepted: {required}: {problems:#?}"
        );
    }

    for (current, replacement, expected_problem) in [
        (
            "id: operator-controller",
            "id: unsupervised-operator-controller",
            "background step id operator-controller",
        ),
        (
            "background: true",
            "background: false",
            "background step id operator-controller",
        ),
        (
            "cancel: operator-controller",
            "cancel: unsupervised-operator-controller",
            "must be explicitly canceled",
        ),
    ] {
        let broken = workflow.replacen(current, replacement, 1);
        assert_ne!(
            broken, workflow,
            "operator lifecycle marker was not found: {current}"
        );
        let problems =
            xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains(expected_problem)),
            "broken operator lifecycle was accepted: {current}: {problems:#?}"
        );
    }

    for required in [
        "canary-check --release 0.66",
        "canary-sweep --release 0.66 --tier fast",
        "canary-sweep --release 0.66 --tier all",
        "evidence-run --release \"$HYDRACACHE_CANDIDATE_RELEASE\" --gate fast.fuzz-corpus-regression",
        "evidence-run --release 0.66 --gate env.hydracache-run-066-daemon-process-e2e",
        "evidence-run --release 0.66 --gate env.hydracache-operator-kind-066",
        "evidence-run --release 0.66 --gate tool.cargo-fuzz.raft-wire-frame-066",
    ] {
        let broken = workflow.replacen(required, "candidate-release-command-was-removed", 1);
        let problems =
            xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
        assert!(
            problems.iter().any(|problem| problem.contains(required)),
            "missing requested-release command was accepted: {required}: {problems:#?}"
        );
    }

    for (current, stale, expected_problem) in [
        (
            "default: \"0.66\"",
            "default: \"0.65\"",
            "workflow_dispatch input candidate_release",
        ),
        (
            "${{ inputs.candidate_release || '0.66' }}",
            "${{ inputs.candidate_release || '0.65' }}",
            "global HYDRACACHE_CANDIDATE_RELEASE",
        ),
        (
            r#"evidence-run --release "$HYDRACACHE_CANDIDATE_RELEASE" --gate fast.workspace-nextest"#,
            "evidence-run --release 0.65 --gate fast.workspace-nextest",
            "fast workspace receipt",
        ),
        (
            r#"release-governance-check --release "$HYDRACACHE_CANDIDATE_RELEASE""#,
            "release-governance-check --release 0.65",
            "candidate governance",
        ),
        (
            r#"evidence-run --release "$HYDRACACHE_CANDIDATE_RELEASE" --gate "${{ inputs.gated_gate_id }}""#,
            r#"evidence-run --release 0.64 --gate "${{ inputs.gated_gate_id }}""#,
            "manually dispatched gate receipt",
        ),
    ] {
        let broken = workflow.replacen(current, stale, 1);
        assert_ne!(broken, workflow, "fixture command was not found: {current}");
        let problems =
            xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains(expected_problem)),
            "hardcoded older candidate release was accepted for {expected_problem}: {problems:#?}"
        );
    }

    let broken = workflow.replacen(
        r#"evidence-run --release "$HYDRACACHE_CANDIDATE_RELEASE" --gate fast.raft-sled-snapshot"#,
        "sled-compaction-proof-was-replaced",
        1,
    );
    let problems =
        xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exact \"Raft compaction control 0.66\" commands")));

    let broken = workflow.replacen(
        "evidence-run --release 0.64 --gate env.hydracache-grid-scope",
        "coverage-command-was-silently-removed",
        1,
    );
    let problems =
        xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("env.hydracache-grid-scope")));

    let broken = workflow.replacen(
        "evidence-run --release 0.65 --gate env.hydracache-run-redis-resp-multinode-e2e",
        "redis-multinode-evidence-was-silently-removed",
        1,
    );
    let problems =
        xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("env.hydracache-run-redis-resp-multinode-e2e")));

    let broken = workflow.replacen(
        "cargo +nightly fuzz run fuzz_config_parse -- -max_total_time=60",
        "cargo +nightly fuzz run fuzz_config_parse --manifest-path fuzz/Cargo.toml -- -max_total_time=60",
        1,
    );
    let problems =
        xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
    assert!(problems.iter().any(|problem| {
        problem.contains("--manifest-path after the target")
            || problem.contains("fuzz_config_parse -- -max_total_time=60")
    }));

    for required in [
        "cargo test -p hydracache-server --test scheduler_tick_process --locked",
        "git merge-base --is-ancestor refs/tags/v0.65.0 HEAD",
        "disableDefaultCNI: true",
        "kubectl get crd iochaos.chaos-mesh.org",
    ] {
        let broken = workflow.replacen(required, "release-066-proof-was-silently-removed", 1);
        assert_ne!(
            broken, workflow,
            "fixture command was not found: {required}"
        );
        let problems =
            xtask::release_governance::release_execution_wiring_problems(&broken, "0.66").unwrap();
        assert!(
            problems.iter().any(|problem| problem.contains(required)),
            "missing 0.66 proof marker was accepted: {required}: {problems:#?}"
        );
    }

    let skip_green = workflow.replacen(
        "rustup toolchain install nightly",
        "set +e\n          rustup toolchain install nightly\n          echo available=false",
        1,
    );
    let problems =
        xtask::release_governance::release_execution_wiring_problems(&skip_green, "0.66").unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("fail loud") || problem.contains("skip-green")));
}

#[test]
fn release_066_registered_heavy_gates_are_mandatory_and_fail_closed() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let problems = xtask::release_governance::release_066_gate_contract_problems(&registry.gate);
    assert!(problems.is_empty(), "{problems:#?}");

    let mut missing_daemon_target = registry.gate.clone();
    let daemon = missing_daemon_target
        .iter_mut()
        .find(|gate| gate.id == "env.hydracache-run-066-daemon-process-e2e")
        .unwrap();
    daemon
        .command
        .args
        .retain(|arg| arg != "scheduler_tick_process");
    let problems =
        xtask::release_governance::release_066_gate_contract_problems(&missing_daemon_target);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("scheduler_tick_process")));

    let mut missing_process_nemesis_artifact = registry.gate.clone();
    let daemon = missing_process_nemesis_artifact
        .iter_mut()
        .find(|gate| gate.id == "env.hydracache-run-066-daemon-process-e2e")
        .unwrap();
    daemon
        .artifacts
        .retain(|artifact| !artifact.contains("process-control-plane-nemesis.json"));
    let problems = xtask::release_governance::release_066_gate_contract_problems(
        &missing_process_nemesis_artifact,
    );
    assert!(problems
        .iter()
        .any(|problem| problem.contains("process-control-plane-nemesis.json")));

    let mut optional_iochaos = registry.gate.clone();
    let operator = optional_iochaos
        .iter_mut()
        .find(|gate| gate.id == "env.hydracache-operator-kind-066")
        .unwrap();
    operator
        .command
        .env
        .remove("HYDRACACHE_OPERATOR_REQUIRE_IOCHAOS");
    let problems = xtask::release_governance::release_066_gate_contract_problems(&optional_iochaos);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("HYDRACACHE_OPERATOR_REQUIRE_IOCHAOS")));

    let mut missing_operator_logs = registry.gate.clone();
    let operator = missing_operator_logs
        .iter_mut()
        .find(|gate| gate.id == "env.hydracache-operator-kind-066")
        .unwrap();
    operator
        .artifacts
        .retain(|artifact| !artifact.ends_with("operator-kind-pod-logs.txt"));
    let problems =
        xtask::release_governance::release_066_gate_contract_problems(&missing_operator_logs);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("operator-kind-pod-logs.txt")));

    let mut missing_operator_nonce = registry.gate.clone();
    let operator = missing_operator_nonce
        .iter_mut()
        .find(|gate| gate.id == "env.hydracache-operator-kind-066")
        .unwrap();
    operator
        .required_env
        .retain(|required| required != "HYDRACACHE_OPERATOR_EVIDENCE_NONCE");
    let problems =
        xtask::release_governance::release_066_gate_contract_problems(&missing_operator_nonce);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("HYDRACACHE_OPERATOR_EVIDENCE_NONCE")));

    let mut unbounded_fuzz = registry.gate.clone();
    let fuzz = unbounded_fuzz
        .iter_mut()
        .find(|gate| gate.id == "tool.cargo-fuzz.raft-wire-frame-066")
        .unwrap();
    fuzz.command.args.pop();
    let problems = xtask::release_governance::release_066_gate_contract_problems(&unbounded_fuzz);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exact bounded")));
}

#[test]
fn canary_release_governance_accepts_a_missing_mandatory_gate() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let mut missing_daemon_gate = registry.gate.clone();
    missing_daemon_gate.retain(|gate| gate.id != "env.hydracache-run-066-daemon-process-e2e");

    let problems =
        xtask::release_governance::release_066_gate_contract_problems(&missing_daemon_gate);
    let rejected = problems.iter().any(|problem| {
        problem.contains("missing mandatory gate env.hydracache-run-066-daemon-process-e2e")
    });

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W13") {
        assert!(
            !rejected,
            "HC-CANARY-RED:W13 release governance accepted a missing mandatory gate"
        );
    }
    assert!(
        rejected,
        "release 0.66 governance did not reject a missing mandatory daemon gate: {problems:#?}"
    );
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
  release-066-daemon-process:
    steps:
      - uses: actions/checkout@v5
"#;
    let problems = xtask::release_governance::release_history_checkout_problems(shallow).unwrap();
    assert_eq!(problems.len(), 6, "{problems:#?}");
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
    assert!(problems
        .iter()
        .any(|problem| problem.contains("job release-066-daemon-process")));
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
