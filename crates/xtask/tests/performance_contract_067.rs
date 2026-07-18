#[test]
fn release_067_governance_contract_is_fail_closed() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let problems = xtask::release_governance::release_067_gate_contract_problems(&registry.gate);
    assert!(problems.is_empty(), "{problems:#?}");
}
