use hydracache::{ClusterCandidate, ClusterGeneration, InMemoryCluster, TopologyFence};

#[test]
fn stamp_changes_when_members_change() {
    let cluster = InMemoryCluster::new("stamp");
    let initial = cluster.ownership_diagnostics().stamp;

    cluster
        .join_member(ClusterCandidate::member("member-a"))
        .unwrap();
    let after_join = cluster.ownership_diagnostics().stamp;

    cluster
        .leave(
            &hydracache::ClusterNodeId::from("member-a"),
            ClusterGeneration::default(),
        )
        .unwrap();
    let after_leave = cluster.ownership_diagnostics().stamp;

    assert!(after_join > initial);
    assert!(after_leave > after_join);
}

#[test]
fn stamp_is_monotonic_nondecreasing() {
    let cluster = InMemoryCluster::new("stamp");
    let mut last = cluster.ownership_diagnostics().stamp;

    for index in 0..5 {
        cluster
            .join_member(ClusterCandidate::member(format!("member-{index}")))
            .unwrap();
        let current = cluster.ownership_diagnostics().stamp;
        assert!(current >= last);
        last = current;
    }
}

#[test]
fn client_with_stale_stamp_can_detect_refresh_needed() {
    let cluster = InMemoryCluster::new("stamp");
    cluster
        .join_member(ClusterCandidate::member("member-a"))
        .unwrap();
    let client_stamp = cluster.ownership_diagnostics().stamp;

    cluster
        .join_member(ClusterCandidate::member("member-b"))
        .unwrap();
    let refreshed = cluster.ownership_diagnostics();

    assert_ne!(client_stamp, refreshed.stamp);
    assert_eq!(cluster.owner_for_key("user:42").member_count, 2);
}

#[test]
fn topology_fence_keeps_epoch_authority_separate_from_stamp() {
    let mut fence = TopologyFence::default();
    assert!(fence.admit(hydracache::ClusterEpoch::new(0)));

    fence.commit(hydracache::ClusterEpoch::new(3));

    assert!(!fence.admit(hydracache::ClusterEpoch::new(2)));
    assert!(fence.admit(hydracache::ClusterEpoch::new(3)));
    assert!(fence.admit(hydracache::ClusterEpoch::new(4)));
}
