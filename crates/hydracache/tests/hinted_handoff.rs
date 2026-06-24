use std::collections::BTreeMap;
use std::time::Duration;

use hydracache::{
    apply_hint, replay_hints, ClusterEpoch, ClusterNodeId, Hint, HintBudget, HintOutcome,
    HintReplayDecision, HintStore, InMemoryHintStore, PartitionId, ReplicatedValueRecord,
};

fn hint(key: &str, version: u64, created_at_millis: u64) -> Hint {
    Hint::new(
        ClusterNodeId::new("replica-b"),
        key,
        PartitionId::new(7),
        version,
        ClusterEpoch::new(1),
        vec![version as u8],
        created_at_millis,
    )
}

#[test]
fn hinted_handoff_brief_outage_hint_replays_on_return() {
    let mut store = InMemoryHintStore::new(HintBudget::new(10, 1024, Duration::from_secs(30)));
    let target = ClusterNodeId::new("replica-b");

    assert_eq!(
        store.store(hint("user:1", 1, 0), false, 1).unwrap(),
        HintOutcome::Stored
    );

    let drained = store.drain_for(&target).unwrap();
    let decisions = replay_hints(drained, &BTreeMap::new()).unwrap();

    assert!(matches!(
        decisions.as_slice(),
        [HintReplayDecision::Replayed { .. }]
    ));
}

#[test]
fn hinted_handoff_hint_never_resurrects_tombstone() {
    let tombstone =
        ReplicatedValueRecord::tombstone(PartitionId::new(7), 2, ClusterEpoch::new(1), None);
    let stale_hint = hint("user:1", 2, 0);

    let decision = apply_hint(Some(&tombstone), &stale_hint).unwrap();

    assert!(matches!(
        decision,
        HintReplayDecision::SuppressedByTombstone { .. }
    ));
}

#[test]
fn hinted_handoff_over_budget_hint_dropped_and_marked_for_repair() {
    let mut store = InMemoryHintStore::new(HintBudget::new(1, 1024, Duration::from_secs(30)));

    assert_eq!(
        store.store(hint("user:1", 1, 0), false, 1).unwrap(),
        HintOutcome::Stored
    );
    assert_eq!(
        store.store(hint("user:2", 1, 0), false, 1).unwrap(),
        HintOutcome::DroppedOverBudget
    );

    assert_eq!(store.len(), 1);
    assert!(store.is_marked_for_repair("user:2"));
    assert_eq!(store.metrics().hints_dropped_total, 1);
}

#[test]
fn hinted_handoff_expired_hint_dropped_after_window() {
    let mut store = InMemoryHintStore::new(HintBudget::new(10, 1024, Duration::from_millis(10)));

    assert_eq!(
        store.store(hint("user:1", 1, 0), false, 11).unwrap(),
        HintOutcome::DroppedExpired
    );

    assert!(store.is_empty());
    assert!(store.is_marked_for_repair("user:1"));
}

#[test]
fn hinted_handoff_required_replica_miss_still_fails_the_write() {
    let mut store = InMemoryHintStore::new(HintBudget::new(10, 1024, Duration::from_secs(30)));

    assert_eq!(
        store.store(hint("user:1", 1, 0), true, 1).unwrap(),
        HintOutcome::RequiredReplicaMiss
    );

    assert!(store.is_empty());
    assert!(!store.is_marked_for_repair("user:1"));
}

#[test]
fn hinted_handoff_replay_suppresses_newer_target_value() {
    let current =
        ReplicatedValueRecord::value(PartitionId::new(7), 3, ClusterEpoch::new(1), vec![3]);
    let stale = hint("user:1", 2, 0);

    let decision = apply_hint(Some(&current), &stale).unwrap();

    assert!(matches!(
        decision,
        HintReplayDecision::SuppressedByNewerValue { .. }
    ));
}

#[test]
#[ignore = "chaos marker: outage beyond hint window falls back to Merkle repair"]
fn hinted_handoff_replica_recovers_after_hint_window_falls_back_to_repair() {
    let mut store = InMemoryHintStore::new(HintBudget::new(10, 1024, Duration::from_millis(1)));

    assert_eq!(
        store.store(hint("user:1", 1, 0), false, 2).unwrap(),
        HintOutcome::DroppedExpired
    );
    assert!(store.is_marked_for_repair("user:1"));
}
