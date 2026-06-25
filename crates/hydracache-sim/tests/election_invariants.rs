use hydracache::ClusterNodeId;
use hydracache_sim::{
    ElectionTopologyNode, ElectionTopologyState, InvariantChecker, NodeFsmState,
    SubscriberDeliveryObservation,
};

#[test]
fn election_safety_holds_on_single_leader() {
    assert_ok(good_topology());
}

#[test]
fn election_safety_fires_on_split_brain() {
    let topology = ElectionTopologyState::new(
        3,
        vec![
            leader("node-0", 2, 2),
            leader("node-1", 2, 2),
            follower("node-2", 2),
        ],
    );

    assert_violation(topology, "election_safety");
}

#[test]
fn leader_requires_quorum_holds_when_leader_has_majority() {
    assert_ok(good_topology());
}

#[test]
fn leader_requires_quorum_fires_on_minority_leader() {
    let topology = ElectionTopologyState::new(
        5,
        vec![
            leader("node-0", 3, 2),
            follower("node-1", 3),
            follower("node-2", 3),
            follower("node-3", 3),
            follower("node-4", 3),
        ],
    );

    assert_violation(topology, "leader_requires_quorum");
}

#[test]
fn no_stale_leader_writes_holds_when_only_authoritative_leader_writes() {
    assert_ok(good_topology());
}

#[test]
fn no_stale_leader_writes_fires_when_isolated_leader_writes() {
    let topology = ElectionTopologyState::new(
        3,
        vec![
            leader("node-0", 4, 2).stale_leader_writes(1),
            leader("node-1", 5, 2),
            follower("node-2", 5),
        ],
    )
    .leader(Some(ClusterNodeId::from("node-1")));

    assert_violation(topology, "no_stale_leader_writes");
}

#[test]
fn index_monotonicity_holds_for_monotonic_commit_and_apply() {
    assert_ok(good_topology());
}

#[test]
fn index_monotonicity_fires_on_log_rollback() {
    let topology = ElectionTopologyState::new(
        3,
        vec![
            leader("node-0", 2, 2).index_history(vec![(1, 1), (2, 2), (1, 1)]),
            follower("node-1", 2),
            follower("node-2", 2),
        ],
    );

    assert_violation(topology, "index_monotonicity");
}

#[test]
fn catchup_no_skip_holds_when_rejoin_applies_contiguous_commits() {
    let topology = ElectionTopologyState::new(
        3,
        vec![
            leader("node-0", 2, 2),
            follower("node-1", 2),
            follower("node-2", 2).applied_commits(vec![1, 2, 3, 4]),
        ],
    );

    assert_ok(topology);
}

#[test]
fn catchup_no_skip_fires_when_rejoin_skips_commit() {
    let topology = ElectionTopologyState::new(
        3,
        vec![
            leader("node-0", 2, 2),
            follower("node-1", 2),
            follower("node-2", 2).applied_commits(vec![1, 2, 4]),
        ],
    );

    assert_violation(topology, "catchup_no_skip");
}

#[test]
fn event_after_commit_holds_when_delivery_follows_commit() {
    let topology = good_topology().subscriber_delivery(SubscriberDeliveryObservation {
        subscriber_id: "sub-1".to_owned(),
        key: "profile:42".to_owned(),
        commit_index: 7,
        delivered_after_commit_index: 7,
    });

    assert_ok(topology);
}

#[test]
fn event_after_commit_fires_when_delivery_precedes_commit() {
    let topology = good_topology().subscriber_delivery(SubscriberDeliveryObservation {
        subscriber_id: "sub-1".to_owned(),
        key: "profile:42".to_owned(),
        commit_index: 7,
        delivered_after_commit_index: 6,
    });

    assert_violation(topology, "event_after_commit");
}

fn good_topology() -> ElectionTopologyState {
    ElectionTopologyState::new(
        3,
        vec![
            leader("node-0", 2, 3).index_history(vec![(1, 1), (2, 2), (3, 3)]),
            follower("node-1", 2).index_history(vec![(1, 1), (2, 2), (3, 3)]),
            follower("node-2", 2)
                .index_history(vec![(1, 1), (2, 2), (3, 3)])
                .applied_commits(vec![1, 2, 3]),
        ],
    )
}

fn leader(node_id: &str, term: u64, votes_received: usize) -> ElectionTopologyNode {
    ElectionTopologyNode::new(node_id)
        .role(NodeFsmState::Leader, term)
        .vote(node_id, votes_received)
}

fn follower(node_id: &str, term: u64) -> ElectionTopologyNode {
    ElectionTopologyNode::new(node_id).role(NodeFsmState::Follower, term)
}

fn assert_ok(topology: ElectionTopologyState) {
    let report = InvariantChecker.check_election_topology(&topology);
    assert!(
        report.is_ok(),
        "expected topology to hold, got {:?}",
        report.violations
    );
    assert_eq!(report.checked, 6);
}

fn assert_violation(topology: ElectionTopologyState, invariant: &str) {
    let report = InvariantChecker.check_election_topology(&topology);
    assert!(
        report
            .violations
            .iter()
            .any(|violation| violation.name == invariant),
        "expected {invariant}, got {:?}",
        report.violations
    );
}
