use hydracache::{
    tombstone_crdt_decision, ClusterEpoch, ClusterNodeId, ConflictFreeValue, CrdtMergeStats,
    GCounter, HigherVersionWins, HybridLogicalClock, LwwRegister, MergePolicy, OrSet, OrSetTag,
    PartitionId, PnCounter, ReplicatedValueRecord, TombstoneCrdtDecision,
};

fn merged<T: ConflictFreeValue>(mut left: T, right: &T) -> T {
    left.merge(right);
    left
}

#[test]
fn crdt_gcounter_merge_is_associative_commutative_idempotent() {
    let mut a = GCounter::new();
    a.increment("a", 2);
    let mut b = GCounter::new();
    b.increment("b", 3);
    let mut c = GCounter::new();
    c.increment("a", 5);

    let left_assoc = merged(merged(a.clone(), &b), &c);
    let right_assoc = merged(a.clone(), &merged(b.clone(), &c));
    assert_eq!(left_assoc, right_assoc);

    assert_eq!(merged(a.clone(), &b), merged(b.clone(), &a));
    assert_eq!(merged(a.clone(), &a), a);
    assert_eq!(left_assoc.value(), 8);
}

#[test]
fn crdt_pn_counter_converges_across_regions() {
    let mut eu = PnCounter::new();
    eu.increment("eu", 10);
    let mut us = PnCounter::new();
    us.decrement("us", 3);

    eu.merge(&us);
    us.merge(&eu);

    assert_eq!(eu, us);
    assert_eq!(eu.value(), 7);
}

#[test]
fn crdt_or_set_add_remove_converges() {
    let mut first = OrSet::new();
    let mut second = OrSet::new();
    let add_v1 = OrSetTag::new("eu", 1);
    let add_v2 = OrSetTag::new("us", 1);

    first.add("profile:42", add_v1.clone());
    second.merge(&first);
    second.remove(&"profile:42");
    first.add("profile:42", add_v2);

    first.merge(&second);
    second.merge(&first);

    assert_eq!(first, second);
    assert!(first.contains(&"profile:42"));
    assert_eq!(first.values(), vec!["profile:42"]);
    assert!(first.metadata_bytes() >= 2 * std::mem::size_of::<u64>() as u64);
}

#[test]
fn crdt_lww_register_uses_hlc_not_wall_clock() {
    let mut left = LwwRegister::new("old", HybridLogicalClock::new(1000, 0), "eu");
    let right = LwwRegister::new("new", HybridLogicalClock::new(1, 2), "us");

    left.merge(&right);
    assert_eq!(left.value(), &"old");

    let newer = LwwRegister::new("newer", HybridLogicalClock::new(1000, 1), "us");
    left.merge(&newer);
    assert_eq!(left.value(), &"newer");
}

#[test]
fn crdt_tombstone_beats_concurrent_crdt_update() {
    let tombstone =
        ReplicatedValueRecord::tombstone(PartitionId::new(1), 9, ClusterEpoch::new(4), None);

    assert_eq!(
        tombstone_crdt_decision(&tombstone, 9, ClusterEpoch::new(4)),
        TombstoneCrdtDecision::KeepTombstone
    );
    assert_eq!(
        tombstone_crdt_decision(&tombstone, 10, ClusterEpoch::new(4)),
        TombstoneCrdtDecision::ApplyUpdate
    );
}

#[test]
fn crdt_non_crdt_value_still_uses_merge_policy() {
    let winner = ReplicatedValueRecord::value(
        PartitionId::new(1),
        3,
        ClusterEpoch::new(1),
        b"winner".to_vec(),
    );
    let loser = ReplicatedValueRecord::value(
        PartitionId::new(1),
        4,
        ClusterEpoch::new(1),
        b"loser".to_vec(),
    );
    let merged = HigherVersionWins
        .merge(Some(&winner), &loser)
        .expect("merged");

    assert_eq!(merged.version, 4);
}

#[test]
fn crdt_merge_stats_use_bounded_type_labels() {
    let mut stats = CrdtMergeStats::default();
    let mut counter = GCounter::new();
    counter.increment("eu", 1);

    stats.record_merge("gcounter", true, counter.metadata_bytes());
    stats.record_merge("gcounter", false, counter.metadata_bytes());

    assert_eq!(stats.merge_total["gcounter"], 2);
    assert_eq!(stats.conflict_resolved_total["gcounter"], 1);
    assert_eq!(stats.metadata_bytes["gcounter"], counter.metadata_bytes());
}

#[test]
fn crdt_lww_register_ties_break_by_writer() {
    let mut left = LwwRegister::new(
        "left",
        HybridLogicalClock::new(5, 1),
        ClusterNodeId::from("a"),
    );
    let right = LwwRegister::new(
        "right",
        HybridLogicalClock::new(5, 1),
        ClusterNodeId::from("z"),
    );

    left.merge(&right);

    assert_eq!(left.value(), &"right");
}
