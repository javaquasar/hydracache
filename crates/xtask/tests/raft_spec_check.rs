#[test]
fn raft_spec_structural_contract_maps_every_invariant_to_rust_evidence() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let problems = xtask::raft_spec_check::structural_check(&root).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn main_model_cannot_import_the_negative_canary() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let main = std::fs::read_to_string(root.join("docs/specs/RaftElection.tla")).unwrap();
    assert!(!main.contains("RaftElectionCanary"));
    assert!(!main.contains("UnsafeSecondLeader"));
}
