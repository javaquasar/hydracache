#![cfg(feature = "test-failpoints")]

use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use fail::FailScenario;
use hydracache_cluster_raft::{InMemoryRaftLogStore, RaftLogStore};
use hydracache_cluster_testkit::RuntimeRaftCluster;
use raft::eraftpb::Entry;

fn voter_set(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<u64> {
    cluster
        .node(node_id)
        .voter_ids()
        .unwrap()
        .into_iter()
        .collect()
}

#[test]
fn crash_between_confchange_commit_and_save_conf_state_recovers_consistent_voters() {
    let _scenario = FailScenario::setup();
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);

    let outbound = cluster.node(1).propose_add_voter(4).unwrap();
    fail::cfg("raft_before_save_conf_state", "return").unwrap();
    let failure = catch_unwind(AssertUnwindSafe(|| {
        cluster.drain_until_idle(outbound.clone());
    }));
    assert!(failure.is_err(), "conf-state failpoint should fail loudly");
    fail::remove("raft_before_save_conf_state");

    let mut recovered = RuntimeRaftCluster::three_node();
    recovered.campaign(1);
    recovered.propose_add_voter(1, 4).unwrap();

    for node_id in [1, 2, 3] {
        assert_eq!(voter_set(&recovered, node_id), BTreeSet::from([1, 2, 3, 4]));
    }
}

#[test]
fn crash_after_hard_state_before_send_does_not_lose_committed_entry() {
    let _scenario = FailScenario::setup();
    let mut cluster = RuntimeRaftCluster::three_node();
    let outbound = cluster.node(1).campaign().unwrap();

    fail::cfg("raft_after_save_hard_state_before_send", "return").unwrap();
    let failure = catch_unwind(AssertUnwindSafe(|| {
        cluster.drain_until_idle(outbound.clone());
    }));
    assert!(failure.is_err(), "hard-state failpoint should fail loudly");
    fail::remove("raft_after_save_hard_state_before_send");

    let mut recovered = RuntimeRaftCluster::three_node();
    recovered.campaign(1);

    assert!(
        recovered.leader_id().is_some(),
        "clearing the failpoint should let election continue"
    );
}

#[test]
fn disk_full_on_append_fails_loud_not_silent() {
    let _scenario = FailScenario::setup();
    let store = InMemoryRaftLogStore::new();
    let entry = Entry {
        index: 1,
        term: 1,
        data: b"member-a".to_vec().into(),
        ..Entry::default()
    };

    fail::cfg("sled_append_disk_full", "return").unwrap();
    let error = store.append(&[entry]).unwrap_err();

    assert!(
        error.to_string().contains("disk full"),
        "disk-full failpoint should surface loudly: {error}"
    );
}

#[test]
fn falsifiability_canaries_turn_their_guard_tests_red() {
    let _scenario = FailScenario::setup();
    fail::cfg("canary_raft_disable_prevote", "return").unwrap();
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader_term = cluster.node(1).snapshot().term;
    cluster.filters().cut(1, 2);

    for _ in 0..20 {
        cluster.tick_node(2);
    }

    assert!(
        cluster.node(2).snapshot().term > leader_term,
        "disabling pre-vote should make the isolated node inflate its term"
    );
}
