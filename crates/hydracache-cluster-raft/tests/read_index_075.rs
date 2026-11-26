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
