use std::num::NonZeroU32;

use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterEpoch, ClusterGeneration, ClusterRole,
    PartitionId, ReplicationConfig,
};
use hydracache_cluster_raft::{RaftMetadataRuntime, RaftPartitionSnapshot};
use hydracache_server::imap_topology::{PartitionTopology, PartitionTopologyError};

const PARTITIONS: NonZeroU32 = NonZeroU32::new(271).unwrap();

fn replication() -> ReplicationConfig {
    ReplicationConfig {
        replication_factor: 2,
        sync_backups: 1,
        ..ReplicationConfig::local_first()
    }
}

async fn roster() -> RaftMetadataRuntime {
    let raft = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    for id in ["a", "b", "c"] {
        raft.join_member(ClusterCandidate::member(id).generation(ClusterGeneration::new(1)))
            .await
            .unwrap();
    }
    raft
}

fn topology(snapshot: RaftPartitionSnapshot) -> Result<PartitionTopology, PartitionTopologyError> {
    PartitionTopology::from_snapshot(snapshot, PARTITIONS, replication())
}

#[tokio::test]
async fn placement_retains_epoch_and_generation_across_owner_restart() {
    let raft = roster().await;
    let partition = PartitionId::new(42);
    let before = topology(raft.partition_snapshot().unwrap()).unwrap();
    let old = before.placement(partition).unwrap();
    assert_eq!(old.partition, partition);
    assert_eq!(old.backups.len(), 1);
    assert_ne!(old.primary.node, old.backups[0].node);

    raft.join_member(
        ClusterCandidate::member(old.primary.node.clone()).generation(ClusterGeneration::new(2)),
    )
    .await
    .unwrap();
    let updated = raft.partition_snapshot().unwrap();
    let current = topology(updated.clone())
        .unwrap()
        .placement(partition)
        .unwrap();
    assert_eq!(current.epoch, updated.epoch);
    assert!(current.epoch > old.epoch);
    assert_eq!(current.primary.node, old.primary.node);
    assert_eq!(current.primary.generation.value(), 2);
    assert_eq!(current.backups, old.backups);
    assert_eq!(before.placement(partition).unwrap(), old);

    let mut reversed = updated;
    reversed.members.reverse();
    assert_eq!(
        topology(reversed).unwrap().placement(partition).unwrap(),
        current
    );
}

#[tokio::test]
async fn placement_refuses_unapplied_metadata_and_inconsistent_rosters() {
    let raft = roster().await;
    let snapshot = raft.partition_snapshot().unwrap();
    let mut lagging = snapshot.clone();
    lagging.commit_index += 1;
    assert_eq!(
        topology(lagging).unwrap_err(),
        PartitionTopologyError::Unapplied {
            applied: snapshot.applied_index,
            committed: snapshot.commit_index + 1,
        },
    );

    let mut duplicate = snapshot.clone();
    duplicate.members.push(duplicate.members[0].clone());
    assert_eq!(
        topology(duplicate).unwrap_err(),
        PartitionTopologyError::InvalidRoster
    );
    let mut future = snapshot.clone();
    future.members[0].epoch = ClusterEpoch::new(snapshot.epoch.value() + 1);
    assert_eq!(
        topology(future).unwrap_err(),
        PartitionTopologyError::InvalidRoster
    );
    let mut includes_client = snapshot.clone();
    includes_client.members[0].role = ClusterRole::Client;
    assert_eq!(
        topology(includes_client).unwrap_err(),
        PartitionTopologyError::InvalidRoster
    );

    let mut one_member = snapshot.clone();
    one_member.members.truncate(1);
    assert_eq!(
        topology(one_member).unwrap_err(),
        PartitionTopologyError::InsufficientMembers {
            available: 1,
            required: 2
        },
    );
    assert_eq!(
        topology(snapshot)
            .unwrap()
            .placement(PartitionId::new(PARTITIONS.get()))
            .unwrap_err(),
        PartitionTopologyError::UnknownPartition(PARTITIONS.get()),
    );
}
