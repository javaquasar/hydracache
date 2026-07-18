#[test]
fn release_067_governance_contract_is_fail_closed() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let problems = xtask::release_governance::release_067_gate_contract_problems(&registry.gate);
    assert!(problems.is_empty(), "{problems:#?}");

    let workflow = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let wiring =
        xtask::release_governance::release_execution_wiring_problems(&workflow, "0.67").unwrap();
    assert!(wiring.is_empty(), "{wiring:#?}");
}
