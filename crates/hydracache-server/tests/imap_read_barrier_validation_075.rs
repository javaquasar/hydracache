use std::cell::Cell;
use std::num::{NonZeroU32, NonZeroU64};

use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterGeneration, PartitionId, ReplicationConfig,
};
use hydracache_cluster_raft::{RaftMetadataRuntime, RaftPartitionSnapshot};
use hydracache_server::imap_read_barrier::{OwnerReadBarrier, ReadBarrierError, ReadInvocation};
use hydracache_server::imap_topology::{PartitionMember, PartitionTopology};

fn invocation() -> ReadInvocation<'static> {
    ReadInvocation {
        invocation_id: NonZeroU64::new(1).unwrap(),
        tenant: "tenant-a",
        namespace: "orders",
        namespace_generation: NonZeroU64::new(1).unwrap(),
        partition: PartitionId::new(1),
    }
}

async fn runtime() -> RaftMetadataRuntime {
    let raft = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    for node in ["a", "b"] {
        raft.join_member(ClusterCandidate::member(node).generation(ClusterGeneration::new(1)))
            .await
            .unwrap();
    }
    raft
}

fn topology(snapshot: RaftPartitionSnapshot) -> PartitionTopology {
    PartitionTopology::from_snapshot(
        snapshot,
        NonZeroU32::new(271).unwrap(),
        ReplicationConfig {
            replication_factor: 2,
            sync_backups: 1,
            ..ReplicationConfig::local_first()
        },
    )
    .unwrap()
}

fn barrier(snapshot: &RaftPartitionSnapshot, owner: &PartitionMember) -> OwnerReadBarrier<'static> {
    OwnerReadBarrier {
        invocation: invocation(),
        term: snapshot.term,
        epoch: snapshot.epoch,
        owner: owner.clone(),
        quorum_index: NonZeroU64::new(snapshot.commit_index).unwrap(),
        required_value_prefix: 8,
    }
}

#[tokio::test]
async fn wrong_invocation_scope_and_unapplied_prefixes_do_not_lookup() {
    let snapshot = runtime().await.partition_snapshot().unwrap();
    let topology = topology(snapshot.clone());
    let owner = topology.placement(invocation().partition).unwrap().primary;
    let lookups = Cell::new(0);
    let lookup = || lookups.set(lookups.get() + 1);

    for wrong in [
        ReadInvocation {
            invocation_id: NonZeroU64::new(2).unwrap(),
            ..invocation()
        },
        ReadInvocation {
            tenant: "tenant-b",
            ..invocation()
        },
        ReadInvocation {
            namespace_generation: NonZeroU64::new(2).unwrap(),
            ..invocation()
        },
        ReadInvocation {
            partition: PartitionId::new(2),
            ..invocation()
        },
    ] {
        assert_eq!(
            barrier(&snapshot, &owner).lookup(&wrong, &topology, &owner, 9, 9, lookup),
            Err(ReadBarrierError::WrongInvocation),
        );
    }
    let pending_index = OwnerReadBarrier {
        quorum_index: NonZeroU64::new(snapshot.applied_index + 1).unwrap(),
        ..barrier(&snapshot, &owner)
    };
    assert_eq!(
        pending_index.lookup(&invocation(), &topology, &owner, 9, 9, lookup),
        Err(ReadBarrierError::MetadataBehind {
            applied: snapshot.applied_index,
            required: snapshot.applied_index + 1
        }),
    );
    for (applied, requested, required) in [(7, 6, 8), (8, 9, 9)] {
        assert_eq!(
            barrier(&snapshot, &owner).lookup(
                &invocation(),
                &topology,
                &owner,
                applied,
                requested,
                lookup
            ),
            Err(ReadBarrierError::ValueBehind { applied, required }),
        );
    }
    assert_eq!(lookups.get(), 0);
    barrier(&snapshot, &owner)
        .lookup(&invocation(), &topology, &owner, 9, 9, lookup)
        .unwrap();
    assert_eq!(lookups.get(), 1);
}

#[tokio::test]
async fn a_barrier_cannot_cross_owner_restart_term_or_local_owner_generation() {
    let raft = runtime().await;
    let snapshot = raft.partition_snapshot().unwrap();
    let partition = invocation().partition;
    let previous_topology = topology(snapshot.clone());
    let previous_owner = previous_topology.placement(partition).unwrap().primary;
    raft.join_member(
        ClusterCandidate::member(previous_owner.node.clone()).generation(ClusterGeneration::new(2)),
    )
    .await
    .unwrap();
    let current = raft.partition_snapshot().unwrap();
    let current_topology = topology(current.clone());
    let owner = current_topology.placement(partition).unwrap().primary;
    assert_eq!(owner.node, previous_owner.node);
    assert_ne!(owner.generation, previous_owner.generation);

    let forbidden_lookup = || -> u8 { panic!("invalid read reached the store") };
    assert_eq!(
        barrier(&snapshot, &previous_owner).lookup(
            &invocation(),
            &current_topology,
            &owner,
            9,
            9,
            forbidden_lookup
        ),
        Err(ReadBarrierError::StaleAuthority),
    );
    let wrong_term = OwnerReadBarrier {
        term: current.term - 1,
        ..barrier(&current, &owner)
    };
    assert_eq!(
        wrong_term.lookup(
            &invocation(),
            &current_topology,
            &owner,
            9,
            9,
            forbidden_lookup
        ),
        Err(ReadBarrierError::StaleAuthority),
    );
    assert_eq!(
        barrier(&current, &owner).lookup(
            &invocation(),
            &current_topology,
            &previous_owner,
            9,
            9,
            forbidden_lookup
        ),
        Err(ReadBarrierError::StaleOwner),
    );
    assert_eq!(
        barrier(&current, &previous_owner).lookup(
            &invocation(),
            &current_topology,
            &owner,
            9,
            9,
            forbidden_lookup
        ),
        Err(ReadBarrierError::StaleOwner),
    );
}
