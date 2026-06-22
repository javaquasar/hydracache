use std::collections::BTreeMap;

use hydracache::{
    resolve_live_split_brain, ClusterEpoch, HigherVersionWins, PartitionId, ReplicatedValueRecord,
};

#[test]
fn split_brain_live_partition_then_heal_resolves_to_higher_epoch() {
    let mut left = BTreeMap::new();
    left.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::value(PartitionId::new(1), 3, ClusterEpoch::new(7), b"left"),
    );
    let mut right = BTreeMap::new();
    right.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::value(PartitionId::new(1), 4, ClusterEpoch::new(6), b"right"),
    );

    let resolution = resolve_live_split_brain(
        ClusterEpoch::new(7),
        left,
        ClusterEpoch::new(6),
        right,
        &HigherVersionWins,
    );

    assert_eq!(resolution.winner_epoch, ClusterEpoch::new(7));
    assert_eq!(resolution.loser_epoch, ClusterEpoch::new(6));
    assert_eq!(resolution.outcome.records["user:42"].version, 4);
}

#[test]
fn split_brain_live_tombstone_on_winner_beats_value_on_loser() {
    let mut winner = BTreeMap::new();
    winner.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::tombstone(PartitionId::new(1), 9, ClusterEpoch::new(8), None),
    );
    let mut loser = BTreeMap::new();
    loser.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::value(PartitionId::new(1), 9, ClusterEpoch::new(8), b"stale"),
    );

    let resolution = resolve_live_split_brain(
        ClusterEpoch::new(8),
        winner,
        ClusterEpoch::new(7),
        loser,
        &HigherVersionWins,
    );

    assert!(resolution.outcome.records["user:42"].is_tombstone());
}

#[test]
#[ignore = "chaos gate: run with -- --ignored when exercising partition heal under churn"]
fn split_brain_live_split_then_heal_under_churn_converges() {
    let resolution = resolve_live_split_brain(
        ClusterEpoch::new(2),
        BTreeMap::new(),
        ClusterEpoch::new(1),
        BTreeMap::new(),
        &HigherVersionWins,
    );
    assert_eq!(resolution.outcome.report.unresolved_conflicts, 0);
}
