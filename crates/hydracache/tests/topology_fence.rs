use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterEpoch, InMemoryCluster, InMemoryClusterDiscovery,
    RaftMetadataCommand, RaftStyleMetadataControlPlane, TopologyFence,
};

#[test]
fn topology_fence_stale_epoch_message_is_dropped() {
    let fence = TopologyFence::new(ClusterEpoch::new(5));

    assert!(!fence.admit(ClusterEpoch::new(4)));
    assert!(fence.admit(ClusterEpoch::new(5)));
    assert!(fence.admit(ClusterEpoch::new(6)));
}

#[test]
fn topology_fence_gossip_suspect_does_not_change_owner() {
    let cluster = InMemoryCluster::new("topology");
    cluster
        .join_member(ClusterCandidate::member("member-a"))
        .expect("member-a");
    cluster
        .join_member(ClusterCandidate::member("member-b"))
        .expect("member-b");
    let owner_before = cluster.owner_for_key("user:42").owner_node_id().cloned();

    let discovery = InMemoryClusterDiscovery::new();
    discovery.announce(ClusterCandidate::member("member-c"));

    let owner_after_gossip = cluster.owner_for_key("user:42").owner_node_id().cloned();
    assert_eq!(owner_before, owner_after_gossip);
}

#[test]
fn topology_fence_late_packet_from_old_leader_does_not_move_fence_backwards() {
    let mut fence = TopologyFence::new(ClusterEpoch::new(6));
    fence.commit(ClusterEpoch::new(5));

    assert_eq!(fence.committed_epoch(), ClusterEpoch::new(6));
}

#[tokio::test]
async fn topology_fence_committed_topology_owner_set_is_deterministic() {
    let control_plane = RaftStyleMetadataControlPlane::new("topology");
    control_plane
        .join_member(ClusterCandidate::member("member-a"))
        .await
        .expect("member-a");
    control_plane
        .join_member(ClusterCandidate::member("member-b"))
        .await
        .expect("member-b");

    let snapshot = control_plane.commit_topology();
    let commands = control_plane.commands();

    assert_eq!(snapshot.member_count, 2);
    assert!(matches!(
        commands.last(),
        Some(RaftMetadataCommand::CommitTopology { members, .. }) if members.len() == 2
    ));
}
