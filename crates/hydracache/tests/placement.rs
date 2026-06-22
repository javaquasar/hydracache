use hydracache::{
    ClusterCandidate, ClusterNodeId, ClusterReplicationStrategy, EffectiveReplicationMap,
    InMemoryCluster, RendezvousClusterOwnership, Replicas, ReplicationConfig,
    ReplicationConfigError,
};

fn admitted_members() -> Vec<hydracache::ClusterMember> {
    let cluster = InMemoryCluster::new("placement");
    for node in ["member-a", "member-b", "member-c"] {
        cluster
            .join_member(ClusterCandidate::member(node))
            .expect("member admitted");
    }
    cluster.members()
}

#[test]
fn placement_deterministic_for_same_member_set() {
    let members = admitted_members();
    let resolver = RendezvousClusterOwnership;

    let first = resolver
        .replicas_for_key("user:42", &members, 3)
        .expect("replicas");
    let second = resolver
        .replicas_for_key("user:42", &members, 3)
        .expect("replicas");

    assert_eq!(first, second);
}

#[test]
fn no_duplicate_backup_owners() {
    let members = admitted_members();
    let resolver = RendezvousClusterOwnership;

    let replicas = resolver
        .replicas_for_key("user:42", &members, 10)
        .expect("replicas");

    assert!(!replicas.backups.contains(&replicas.primary));
    let mut sorted = replicas.backups.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), replicas.backups.len());
}

#[test]
fn replication_factor_exceeding_members_degrades_clearly() {
    let members = admitted_members().into_iter().take(2).collect::<Vec<_>>();
    let resolver = RendezvousClusterOwnership;

    let replicas = resolver
        .replicas_for_key("user:42", &members, 5)
        .expect("replicas");

    assert_eq!(replicas.copy_count(), 2);
    assert_eq!(replicas.backups.len(), 1);
}

#[test]
fn pending_map_reads_both_during_move() {
    let natural = Replicas::new("member-a", vec![ClusterNodeId::from("member-b")]);
    let pending = Replicas::new("member-c", vec![ClusterNodeId::from("member-b")]);

    let map = EffectiveReplicationMap::with_pending(natural, pending);

    assert!(map.is_readable_from(&ClusterNodeId::from("member-a")));
    assert!(map.is_readable_from(&ClusterNodeId::from("member-b")));
    assert!(map.is_readable_from(&ClusterNodeId::from("member-c")));
}

#[test]
fn quorum_validation_rejects_bad_config() {
    assert_eq!(
        ReplicationConfig {
            replication_factor: 0,
            ..ReplicationConfig::default()
        }
        .validate(),
        Err(ReplicationConfigError::ReplicationFactorZero)
    );
    assert_eq!(
        ReplicationConfig {
            replication_factor: 2,
            read_quorum: 3,
            write_quorum: 1,
            ..ReplicationConfig::default()
        }
        .validate(),
        Err(ReplicationConfigError::QuorumExceedsReplicationFactor)
    );
    assert_eq!(
        ReplicationConfig {
            replication_factor: 2,
            sync_backups: 1,
            async_backups: 1,
            ..ReplicationConfig::default()
        }
        .validate(),
        Err(ReplicationConfigError::BackupCountExceedsReplicationFactor)
    );
}

#[test]
fn placement_distribution_is_even() {
    let members = admitted_members();
    let resolver = RendezvousClusterOwnership;
    let mut counts = std::collections::BTreeMap::<String, usize>::new();

    for index in 0..3_000 {
        let replicas = resolver
            .replicas_for_key(&format!("key:{index}"), &members, 2)
            .expect("replicas");
        *counts.entry(replicas.primary.to_string()).or_default() += 1;
    }

    assert_eq!(counts.len(), 3);
    for count in counts.values() {
        assert!(
            (600..=1_800).contains(count),
            "primary distribution should stay close enough for 3k keys: {counts:?}"
        );
    }
}
