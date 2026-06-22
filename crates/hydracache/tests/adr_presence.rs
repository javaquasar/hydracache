use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn adr_presence_files_exist_and_are_linked() {
    let root = repo_root();
    let readiness =
        fs::read_to_string(root.join("docs/cluster/readiness.md")).expect("cluster readiness doc");
    let adr_files = [
        "0001-gossip-liveness-vs-raft-topology.md",
        "0002-raft-log-store-durability-contract.md",
        "0003-replication-strategy-and-effective-map.md",
        "0004-rebalance-plan-as-data.md",
        "0005-tombstone-gc-vs-repair-boundary.md",
    ];

    for file in adr_files {
        assert!(
            root.join("docs/adr").join(file).exists(),
            "missing ADR file {file}"
        );
        assert!(
            readiness.contains(file),
            "readiness doc does not link {file}"
        );
    }
}
