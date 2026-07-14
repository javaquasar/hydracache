#[test]
fn release_governance_check_accepts_current_structural_meta_gates() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let report = xtask::release_governance::check(&root, "0.64").unwrap();
    assert!(report.problems.is_empty(), "{:#?}", report.problems);
    assert!(report
        .todos
        .iter()
        .any(|todo| todo.contains("TODO-W32-COMPAT-CHECK")));
    assert!(report
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
    assert!(problems
        .iter()
        .any(|problem| problem.contains("missing job")));

    let mut gate = registry.gate[0].clone();
    gate.ci.step = "Missing step".to_owned();
    let problems = xtask::release_governance::ci_wiring_problems(&root, &[gate]).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("missing step")));
}
