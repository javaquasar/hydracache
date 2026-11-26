use std::future::{ready, Future};
use std::pin::pin;
use std::time::Duration;

use hydracache::{ClusterCandidate, ClusterControlPlane};
use hydracache_cluster_raft::{RaftReadIndexError, RaftWireMessage};
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use raft::eraftpb::MessageType;
use tokio::time::timeout;

async fn assert_pending<T>(future: std::pin::Pin<&mut impl Future<Output = T>>) {
    tokio::select! {
        biased;
        _ = future => panic!("ReadIndex completed prematurely"),
        _ = ready(()) => {}
    }
}

#[tokio::test]
async fn follower_requires_its_own_current_leader_response_and_applied_metadata() {
    let mut cluster = RuntimeRaftCluster::three_node();
    let follower = cluster.node(2);
    assert!(matches!(
        follower.begin_read_index(),
        Err(RaftReadIndexError::Unavailable)
    ));
    cluster.campaign(1);
    let leader = cluster.node(1);

    let (first, first_messages) = follower.begin_read_index().unwrap();
    let (second, second_messages) = follower.begin_read_index().unwrap();
    let mut first = pin!(first.complete());
    let mut second = pin!(second.complete());
    assert_pending(first.as_mut()).await;

    // Commit a client record without delivering the entry to the reader.
    // Client membership leaves the placement epoch unchanged.
    let filters = cluster.filters();
    filters.add_filter(
        RaftPacketFilter::new()
            .from(1)
            .to(2)
            .message_type(MessageType::MsgAppend)
            .action(RaftFilterAction::Drop),
    );
    let epoch = follower.partition_snapshot().unwrap().epoch;
    let join = tokio::spawn({
        let leader = leader.clone();
        async move {
            leader
                .join_client(ClusterCandidate::client("reader-test"))
                .await
                .unwrap()
        }
    });
    timeout(Duration::from_secs(2), async {
        while !join.is_finished() {
            cluster.drain_until_idle(leader.take_outbound_messages());
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    join.await.unwrap();
    assert!(follower.snapshot().applied_index < leader.snapshot().commit_index);

    cluster.drain_until_idle(first_messages);
    let reply = cluster
        .delivered()
        .iter()
        .rev()
        .find(|message| {
            message.to == 2
                && message.decode().unwrap().get_msg_type() == MessageType::MsgReadIndexResp
        })
        .unwrap()
        .clone();
    assert_pending(first.as_mut()).await;
    assert_pending(second.as_mut()).await;

    // Even a matching context from a different peer or term cannot advance
    // the follower. A duplicate of the first proof cannot complete the second.
    let mut decoded = reply.decode().unwrap();
    decoded.entries = second_messages[0].decode().unwrap().entries;
    let term = decoded.term;
    decoded.from = 3;
    follower
        .step(RaftWireMessage::encode(&decoded).unwrap())
        .unwrap();
    decoded.from = 1;
    decoded.term = term + 1;
    follower
        .step(RaftWireMessage::encode(&decoded).unwrap())
        .unwrap();
    assert_eq!(follower.snapshot().term, term);
    assert_pending(second.as_mut()).await;

    filters.recover();
    cluster.drain_until_idle(leader.tick().unwrap());
    let proof = timeout(Duration::from_secs(2), first.as_mut())
        .await
        .unwrap()
        .unwrap();
    proof
        .with_snapshot(|snapshot, index| {
            assert_eq!(snapshot.epoch, epoch);
            assert!(snapshot.applied_index >= index.get());
            assert_eq!(index.get(), reply.decode().unwrap().index);
        })
        .unwrap();
    follower.step(reply).unwrap();
    assert_pending(second.as_mut()).await;
    cluster.drain_until_idle(second_messages);
    timeout(Duration::from_secs(2), second.as_mut())
        .await
        .unwrap()
        .unwrap()
        .with_snapshot(|_, _| ())
        .unwrap();
}

#[tokio::test]
async fn isolated_leader_bounds_waiters_and_releases_cancelled_contexts() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader = cluster.node(1);
    let filters = cluster.filters();
    filters.isolate(1, [2, 3]);

    let mut pending = (0..256)
        .map(|_| {
            let (pending, messages) = leader.begin_read_index().unwrap();
            cluster.drain_until_idle(messages);
            pending
        })
        .collect::<Vec<_>>();
    assert!(matches!(
        leader.begin_read_index(),
        Err(RaftReadIndexError::Busy)
    ));
    {
        let mut cancelled = pin!(pending.pop().unwrap().complete());
        assert_pending(cancelled.as_mut()).await;
    }
    // The in-flight future released both local and leader-side capacity.
    let (replacement, _) = leader.begin_read_index().unwrap();
    drop(replacement);
    drop(pending);
    let (fresh, messages) = leader.begin_read_index().unwrap();
    filters.recover();
    cluster.drain_until_idle(messages);
    let proof = timeout(Duration::from_secs(2), fresh.complete())
        .await
        .unwrap()
        .unwrap();
    proof
        .with_snapshot(|snapshot, index| assert!(snapshot.applied_index >= index.get()))
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn forwarded_reads_expire_at_the_leader_after_the_requester_cancels() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader = cluster.node(1);
    let follower = cluster.node(2);
    let filters = cluster.filters();
    for peer in [2, 3] {
        filters.add_filter(
            RaftPacketFilter::new()
                .from(peer)
                .to(1)
                .message_type(MessageType::MsgHeartbeatResponse)
                .action(RaftFilterAction::Drop),
        );
    }
    for _ in 0..256 {
        let (pending, messages) = follower.begin_read_index().unwrap();
        cluster.drain_until_idle(messages);
        drop(pending);
    }
    assert!(matches!(
        leader.begin_read_index(),
        Err(RaftReadIndexError::Busy)
    ));

    // No release message is required from the requester. A regular leader
    // tick expires the forwarded contexts even with no heartbeat replies.
    tokio::time::advance(Duration::from_secs(1)).await;
    cluster.drain_until_idle(leader.tick().unwrap());
    filters.recover();
    let (fresh, messages) = follower.begin_read_index().unwrap();
    cluster.drain_until_idle(messages);
    timeout(Duration::from_secs(2), fresh.complete())
        .await
        .unwrap()
        .unwrap()
        .with_snapshot(|_, _| ())
        .unwrap();
}

#[tokio::test]
async fn a_completed_proof_cannot_cross_leadership_transition() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader = cluster.node(1);
    let (pending, messages) = leader.begin_read_index().unwrap();
    cluster.drain_until_idle(messages);
    let proof = timeout(Duration::from_secs(2), pending.complete())
        .await
        .unwrap()
        .unwrap();
    cluster.request_leadership_transfer(1, 2).unwrap();
    assert_eq!(cluster.leader_id(), Some(2));
    assert!(matches!(
        proof.with_snapshot(|_, _| panic!("stale proof reached the lookup")),
        Err(RaftReadIndexError::Unavailable)
    ));
}

#[tokio::test]
async fn a_completed_proof_cannot_cross_a_voter_change_in_the_same_term() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader = cluster.node(1);
    let before = leader.partition_snapshot().unwrap();
    let (pending, messages) = leader.begin_read_index().unwrap();
    cluster.drain_until_idle(messages);
    let proof = timeout(Duration::from_secs(2), pending.complete())
        .await
        .unwrap()
        .unwrap();
    cluster.drain_until_idle(leader.propose_remove_voter(3).unwrap());
    assert_eq!(leader.voter_ids().unwrap(), vec![1, 2]);
    let after = leader.partition_snapshot().unwrap();
    assert_eq!((before.term, before.epoch), (after.term, after.epoch));
    assert!(matches!(
        proof.with_snapshot(|_, _| panic!("proof from the previous voter set reached lookup")),
        Err(RaftReadIndexError::Unavailable)
    ));
}

#[tokio::test]
async fn committed_value_prefix_requires_exact_predecessor_and_recovers_from_snapshot() {
    use hydracache::{ClusterEpoch, PartitionId, PartitionValuePrefix};
    use hydracache_cluster_raft::RaftMetadataRuntime;

    let raft = RaftMetadataRuntime::single_node("w10c", 1).unwrap();
    raft.join_member(ClusterCandidate::member("owner"))
        .await
        .unwrap();
    let epoch = raft.partition_snapshot().unwrap().epoch;
    let partition = PartitionId::new(6);
    let first = PartitionValuePrefix {
        partition,
        epoch,
        version: 1,
        previous: [0; 32],
        digest: [1; 32],
    };
    raft.propose_value_prefix(first).unwrap();
    for invalid in [
        PartitionValuePrefix {
            version: 3,
            previous: [1; 32],
            digest: [3; 32],
            ..first
        },
        PartitionValuePrefix {
            version: 2,
            previous: [2; 32],
            digest: [4; 32],
            ..first
        },
        PartitionValuePrefix {
            epoch: ClusterEpoch::new(0),
            version: 2,
            previous: [1; 32],
            digest: [5; 32],
            ..first
        },
    ] {
        raft.propose_value_prefix(invalid).unwrap();
        assert_eq!(
            raft.partition_snapshot().unwrap().value_prefixes[&partition],
            first
        );
    }
    let second = PartitionValuePrefix {
        version: 2,
        previous: [1; 32],
        digest: [2; 32],
        ..first
    };
    raft.propose_value_prefix(second).unwrap();
    let recovered = RaftMetadataRuntime::from_snapshot(raft.export_snapshot()).unwrap();
    assert_eq!(
        recovered.partition_snapshot().unwrap().value_prefixes[&partition],
        second
    );
}

#[tokio::test]
async fn promotion_requires_exact_committed_cut_and_fences_delayed_certificate_after_recovery() {
    use hydracache::{
        ClusterEpoch, ClusterGeneration, ClusterReplicationStrategy, PartitionId,
        PartitionPromotion, PartitionReplicaIdentity, PartitionReplicaRepair, PartitionValuePrefix,
        RendezvousClusterOwnership,
    };
    use hydracache_cluster_raft::RaftMetadataRuntime;

    let raft = RaftMetadataRuntime::single_node("w10e", 1).unwrap();
    for node in ["old", "potential-backup", "spare"] {
        raft.join_member(ClusterCandidate::member(node))
            .await
            .unwrap();
    }
    let before = raft.partition_snapshot().unwrap();
    let partition = PartitionId::new(6);
    let ranked = RendezvousClusterOwnership
        .replicas_for_key("partition-6", &before.members, 2)
        .unwrap();
    let identity = |node| {
        let member = before
            .members
            .iter()
            .find(|member| member.node_id == node)
            .unwrap();
        PartitionReplicaIdentity {
            node,
            generation: member.generation,
        }
    };
    let old_owner = identity(ranked.primary);
    let new_owner = identity(ranked.backups[0].clone());
    let first = PartitionValuePrefix {
        partition,
        epoch: before.epoch,
        version: 1,
        previous: [0; 32],
        digest: [1; 32],
    };
    raft.propose_value_prefix(first).unwrap();
    let plan = PartitionPromotion {
        partition,
        old_epoch: before.epoch,
        new_epoch: ClusterEpoch::new(before.epoch.value() + 1),
        old_owner: old_owner.clone(),
        new_owner: new_owner.clone(),
        backups: vec![old_owner],
        required: Some(first),
    };
    let wrong_backup = PartitionReplicaIdentity {
        node: plan.old_owner.node.clone(),
        generation: ClusterGeneration::new(plan.old_owner.generation.value() + 1),
    };
    for invalid in [
        PartitionPromotion {
            required: None,
            ..plan.clone()
        },
        PartitionPromotion {
            required: Some(PartitionValuePrefix {
                digest: [9; 32],
                ..first
            }),
            ..plan.clone()
        },
        PartitionPromotion {
            new_epoch: before.epoch,
            ..plan.clone()
        },
        PartitionPromotion {
            backups: vec![wrong_backup],
            ..plan.clone()
        },
        PartitionPromotion {
            backups: vec![new_owner],
            ..plan.clone()
        },
    ] {
        raft.propose_value_promotion(invalid).unwrap();
        let snapshot = raft.partition_snapshot().unwrap();
        assert_eq!(snapshot.epoch, before.epoch);
        assert!(snapshot.value_promotions.is_empty());
    }
    let (pending, _) = raft.begin_read_index().unwrap();
    let old_proof = pending.complete().await.unwrap();
    raft.propose_value_promotion(plan.clone()).unwrap();
    assert!(matches!(
        old_proof.with_snapshot(|_, _| panic!("old proof crossed the cut")),
        Err(RaftReadIndexError::Unavailable)
    ));
    let recovered = RaftMetadataRuntime::from_snapshot(raft.export_snapshot()).unwrap();
    for runtime in [&raft, &recovered] {
        let view = runtime.partition_snapshot().unwrap();
        assert_eq!(view.epoch, plan.new_epoch);
        assert_eq!(view.value_promotions[&partition], plan);
        assert_eq!(view.value_prefixes[&partition], first);
    }
    let delayed = PartitionValuePrefix {
        version: 2,
        previous: first.digest,
        digest: [2; 32],
        ..first
    };
    raft.propose_value_prefix(delayed).unwrap();
    assert_eq!(
        raft.partition_snapshot().unwrap().value_prefixes[&partition],
        first
    );
    let next = PartitionValuePrefix {
        epoch: plan.new_epoch,
        digest: [3; 32],
        ..delayed
    };
    raft.propose_value_prefix(next).unwrap();
    assert_eq!(
        raft.partition_snapshot().unwrap().value_prefixes[&partition],
        next
    );

    let target = identity(
        before
            .members
            .iter()
            .find(|member| {
                member.node_id != plan.old_owner.node && member.node_id != plan.new_owner.node
            })
            .unwrap()
            .node_id
            .clone(),
    );
    let repair = PartitionReplicaRepair {
        partition,
        old_epoch: plan.new_epoch,
        new_epoch: ClusterEpoch::new(plan.new_epoch.value() + 1),
        owner: plan.new_owner.clone(),
        replaced: plan.old_owner.clone(),
        target: target.clone(),
        required: Some(next),
        snapshot_digest: [8; 32],
    };
    for invalid in [
        PartitionReplicaRepair {
            required: Some(first),
            ..repair.clone()
        },
        PartitionReplicaRepair {
            old_epoch: before.epoch,
            ..repair.clone()
        },
        PartitionReplicaRepair {
            replaced: target.clone(),
            ..repair.clone()
        },
        PartitionReplicaRepair {
            target: PartitionReplicaIdentity {
                generation: ClusterGeneration::new(target.generation.value() + 1),
                ..target.clone()
            },
            ..repair.clone()
        },
    ] {
        raft.propose_value_repair(invalid).unwrap();
        let snapshot = raft.partition_snapshot().unwrap();
        assert_eq!(snapshot.epoch, plan.new_epoch);
        assert_eq!(snapshot.value_promotions[&partition], plan);
        assert!(snapshot.value_repairs.is_empty());
    }
    let (pending, _) = raft.begin_read_index().unwrap();
    let proof = pending.complete().await.unwrap();
    raft.propose_value_repair(repair.clone()).unwrap();
    assert!(matches!(
        proof.with_snapshot(|_, _| panic!("old proof crossed the replica replacement")),
        Err(RaftReadIndexError::Unavailable)
    ));
    raft.propose_value_prefix(PartitionValuePrefix {
        version: 3,
        previous: next.digest,
        digest: [7; 32],
        ..next
    })
    .unwrap();
    let restored = RaftMetadataRuntime::from_snapshot(raft.export_snapshot()).unwrap();
    for runtime in [&raft, &restored] {
        runtime
            .with_applied_partition_snapshot(|snapshot| {
                assert_eq!(snapshot.epoch, repair.new_epoch);
                assert_eq!(snapshot.value_repairs[&partition], repair);
                assert_eq!(snapshot.value_prefixes[&partition], next);
                assert_eq!(
                    snapshot.value_promotions[&partition].backups,
                    vec![target.clone()]
                );
                assert_eq!(
                    snapshot.value_promotions[&partition].new_owner,
                    plan.new_owner
                );
            })
            .unwrap();
    }
}
