use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterEpoch, ClusterGeneration, InMemoryCluster,
    InMemoryClusterDiscovery, RaftMetadataCommand, RaftStyleMetadataControlPlane, TopologyFence,
};

#[test]
fn stale_epoch_message_dropped() {
    let fence = TopologyFence::new(ClusterEpoch::new(5));

    assert!(!fence.admit(ClusterEpoch::new(4)));
    assert!(fence.admit(ClusterEpoch::new(5)));
}

#[tokio::test]
async fn gossip_suspect_does_not_change_owner_for_key() {
    let cluster = InMemoryCluster::new("fence");
    let discovery = InMemoryClusterDiscovery::new();
    cluster
        .join_member(ClusterCandidate::member("member-a"))
        .unwrap();
    let owner_before = cluster.owner_for_key("user:42").owner_node_id().cloned();

    discovery.announce(ClusterCandidate::member("member-b"));

    let owner_after = cluster.owner_for_key("user:42").owner_node_id().cloned();
    assert_eq!(owner_before, owner_after);
    assert_eq!(cluster.members().len(), 1);
}

#[tokio::test]
async fn owner_set_deterministic_after_commit() {
    let control_plane = RaftStyleMetadataControlPlane::new("fence");
    control_plane
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();
    control_plane
        .join_member(ClusterCandidate::member("member-b").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();

    let snapshot = control_plane.commit_topology();
    let commands = control_plane.commands();

    assert!(matches!(
        snapshot.last_command,
        Some(RaftMetadataCommand::CommitTopology { .. })
    ));
    assert!(matches!(
        commands.last(),
        Some(RaftMetadataCommand::CommitTopology { members, .. }) if members.len() == 2
    ));
    assert_eq!(
        control_plane.ownership_diagnostics().stamp,
        control_plane.snapshot().epoch.value()
    );
}

#[test]
fn late_packet_from_old_leader_does_not_resurrect_topology() {
    let mut fence = TopologyFence::new(ClusterEpoch::new(2));
    fence.commit(ClusterEpoch::new(7));
    fence.commit(ClusterEpoch::new(3));

    assert_eq!(fence.committed_epoch(), ClusterEpoch::new(7));
    assert!(!fence.admit(ClusterEpoch::new(6)));
}
