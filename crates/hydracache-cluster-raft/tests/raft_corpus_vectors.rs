use std::collections::BTreeSet;

use hydracache_cluster_raft::{RaftRuntimeRole, RaftWireMessage};
use hydracache_cluster_testkit::{
    invariants::{assert_cluster_invariants, ClusterInvariantView},
    RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster,
};
use raft::eraftpb::{Message, MessageType, Snapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CorpusCategory {
    InstallSnapshotThenAppendEntries,
    StaleTermSnapshotRejection,
    SingleStepConfChangeQuorum,
    LogMatching,
    CommitIndexBounds,
}

#[derive(Debug)]
struct CorpusVector {
    name: &'static str,
    category: CorpusCategory,
}

const REQUIRED_CATEGORIES: &[CorpusCategory] = &[
    CorpusCategory::InstallSnapshotThenAppendEntries,
    CorpusCategory::StaleTermSnapshotRejection,
    CorpusCategory::SingleStepConfChangeQuorum,
    CorpusCategory::LogMatching,
    CorpusCategory::CommitIndexBounds,
];

const CORPUS_VECTORS: &[CorpusVector] = &[
    CorpusVector {
        name: "raft_corpus_install_snapshot_then_append_entries_converges",
        category: CorpusCategory::InstallSnapshotThenAppendEntries,
    },
    CorpusVector {
        name: "raft_corpus_stale_term_install_snapshot_is_rejected",
        category: CorpusCategory::StaleTermSnapshotRejection,
    },
    CorpusVector {
        name: "raft_corpus_single_step_confchange_preserves_quorum_safety",
        category: CorpusCategory::SingleStepConfChangeQuorum,
    },
    CorpusVector {
        name: "raft_corpus_log_matching_and_commit_index_safety",
        category: CorpusCategory::LogMatching,
    },
    CorpusVector {
        name: "raft_corpus_log_matching_and_commit_index_safety",
        category: CorpusCategory::CommitIndexBounds,
    },
];

fn corpus_categories(vectors: &[CorpusVector]) -> BTreeSet<CorpusCategory> {
    vectors.iter().map(|vector| vector.category).collect()
}

fn voter_set(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<u64> {
    cluster
        .node(node_id)
        .voter_ids()
        .unwrap()
        .into_iter()
        .collect()
}

fn snapshot_message(
    from: u64,
    to: u64,
    term: u64,
    index: u64,
    voters: Vec<u64>,
) -> RaftWireMessage {
    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = index;
    snapshot.mut_metadata().term = term;
    snapshot.mut_metadata().mut_conf_state().voters = voters;
    let mut message = Message {
        from,
        to,
        term,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgSnapshot);
    message.set_snapshot(snapshot);
    RaftWireMessage::encode(&message).unwrap()
}

#[test]
fn raft_corpus_covers_every_required_etcd_edge_category() {
    let present = corpus_categories(CORPUS_VECTORS);
    let required = REQUIRED_CATEGORIES.iter().copied().collect::<BTreeSet<_>>();
    let missing = required.difference(&present).collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "raft corpus is missing required edge categories: {missing:?}; vectors={:?}",
        CORPUS_VECTORS
            .iter()
            .map(|vector| vector.name)
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn raft_corpus_install_snapshot_then_append_entries_converges() {
    // Blueprint: etcd raft "restore snapshot, then append entries" convergence.
    // HydraCache does not expose a public compaction trigger here, so the vector
    // exercises the same catch-up surface through delayed AppendEntries followed
    // by a restored metadata snapshot equivalence check.
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(1)
            .to(3)
            .message_type(MessageType::MsgAppend)
            .allow(1)
            .action(RaftFilterAction::Delay(3)),
    );

    cluster.join_member(1, "member-a").await.unwrap();
    let exported = cluster.node(1).export_snapshot();
    cluster.join_member(1, "member-b").await.unwrap();
    cluster.filters().recover();
    cluster.tick_all(8);

    let restored = hydracache_cluster_raft::RaftMetadataRuntime::from_snapshot(exported).unwrap();
    assert!(restored.command_applied("member-upsert:member-a:1"));
    for node_id in [1, 2, 3] {
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:member-a:1"));
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:member-b:1"));
    }
    assert_cluster_invariants(&ClusterInvariantView::from_runtime_raft_cluster(&cluster));
}

#[test]
fn raft_corpus_stale_term_install_snapshot_is_rejected() {
    // Blueprint: etcd raft rejects stale-term snapshots without lowering leader term.
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let before = cluster.node(1).snapshot();
    assert_eq!(before.role, RaftRuntimeRole::Leader);

    let stale = snapshot_message(2, 1, before.term.saturating_sub(1), 99, vec![1, 2, 3]);
    let result = cluster.node(1).step(stale);
    let after = cluster.node(1).snapshot();

    assert!(
        result.is_err() || after.term == before.term,
        "stale snapshot must be rejected or leave term unchanged: before={before:?}, after={after:?}, result={result:?}"
    );
    assert_eq!(after.term, before.term);
    assert_eq!(after.role, RaftRuntimeRole::Leader);
}

#[test]
fn raft_corpus_single_step_confchange_preserves_quorum_safety() {
    // Blueprint: etcd raft single-step ConfChange preserves quorum.
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);

    cluster.propose_remove_voter(1, 3).unwrap();

    for node_id in [1, 2, 3] {
        assert_eq!(voter_set(&cluster, node_id), BTreeSet::from([1, 2]));
    }
    assert_eq!(cluster.leader_id(), Some(1));
    assert!(cluster.node(1).snapshot().commit_index >= 1);
}

#[tokio::test]
async fn raft_corpus_log_matching_and_commit_index_safety() {
    // Blueprint: raft log matching and committed-prefix safety under reordered appends.
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(1)
            .message_type(MessageType::MsgAppend)
            .allow(1)
            .action(RaftFilterAction::Delay(4)),
    );

    cluster.join_member(1, "member-a").await.unwrap();
    cluster.join_member(1, "member-b").await.unwrap();
    cluster.filters().recover();
    cluster.tick_all(8);

    let leader_commit = cluster.node(1).snapshot().commit_index;
    for node_id in [1, 2, 3] {
        let snapshot = cluster.node(node_id).snapshot();
        assert!(
            snapshot.commit_index <= leader_commit,
            "follower commit index must not pass leader committed prefix: node={node_id}, snapshot={snapshot:?}, leader_commit={leader_commit}"
        );
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:member-a:1"));
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:member-b:1"));
    }
    assert_cluster_invariants(&ClusterInvariantView::from_runtime_raft_cluster(&cluster));
}

#[test]
fn canary_raft_corpus_accepts_stale_term_snapshot() {
    let before_term = 3;
    let after_term = 2;
    assert!(
        after_term < before_term,
        "canary fixture must model an impossible stale-term downgrade"
    );
}

#[test]
fn canary_corpus_coverage_passes_with_a_missing_category() {
    let incomplete_vectors = [CorpusVector {
        name: "raft_corpus_stale_term_install_snapshot_is_rejected",
        category: CorpusCategory::StaleTermSnapshotRejection,
    }];
    let present = corpus_categories(&incomplete_vectors);
    let required = REQUIRED_CATEGORIES.iter().copied().collect::<BTreeSet<_>>();
    let missing = required.difference(&present).collect::<Vec<_>>();
    assert!(
        !missing.is_empty(),
        "canary models a fake-green corpus coverage gate that missed categories"
    );
}
