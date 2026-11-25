//! Golden key placement.
//!
//! Owners are a durable cross-version contract: two releases in one cluster must
//! agree on which member owns a key, or ownership reshuffles during a rolling
//! upgrade. These values were recorded before the placement and consistency
//! types moved to an always-built module (TD-0014); any change to the
//! rendezvous hash or tie-break fails here.

use hydracache::{
    ClusterCandidate, ClusterOwnershipResolver, ClusterReplicationStrategy, InMemoryCluster,
    RendezvousClusterOwnership,
};

const KEYS: [&str; 6] = [
    "user:42",
    "profiles:p",
    "orders:1",
    "orders:2",
    "session:abc",
    "",
];

fn members() -> Vec<hydracache::ClusterMember> {
    let cluster = InMemoryCluster::new("golden");
    for node in ["node-0", "node-1", "node-2", "node-3", "node-4"] {
        cluster
            .join_member(ClusterCandidate::member(node))
            .expect("member admitted");
    }
    cluster.members()
}

fn placement() -> Vec<String> {
    let members = members();
    KEYS.iter()
        .map(|key| {
            let owner = RendezvousClusterOwnership
                .resolve_owner(key, &members)
                .owner_node_id()
                .expect("owner")
                .to_string();
            let replicas = RendezvousClusterOwnership
                .replicas_for_key(key, &members, 3)
                .expect("replicas")
                .all_nodes()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("{key}|{owner}|{replicas}")
        })
        .collect()
}

#[test]
fn rendezvous_placement_matches_recorded_owners() {
    let expected = [
        "user:42|node-1|node-1,node-0,node-3",
        "profiles:p|node-2|node-2,node-3,node-0",
        "orders:1|node-4|node-4,node-2,node-3",
        "orders:2|node-4|node-4,node-3,node-2",
        "session:abc|node-4|node-4,node-2,node-3",
        "|node-2|node-2,node-3,node-0",
    ];
    assert_eq!(placement(), expected);
}

#[test]
fn single_owner_and_replica_primary_agree() {
    // The ownership resolver and the replication strategy carry separate
    // copies of the hash; their first choice must stay the same.
    for line in placement() {
        let mut parts = line.split('|');
        let _key = parts.next();
        let owner = parts.next().expect("owner");
        let primary = parts.next().expect("replicas").split(',').next();
        assert_eq!(Some(owner), primary, "{line}");
    }
}
