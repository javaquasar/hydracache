use std::collections::BTreeMap;

use hydracache::{
    merge_split_brain_records, split_brain_winner, ClusterEpoch, DiscardLoser, HigherVersionWins,
    PartitionId, PutIfAbsent, ReplicatedValueRecord,
};

#[test]
fn split_brain_lower_epoch_side_loses_topology() {
    let (winner, loser) = split_brain_winner(ClusterEpoch::new(7), ClusterEpoch::new(5));

    assert_eq!(winner, ClusterEpoch::new(7));
    assert_eq!(loser, ClusterEpoch::new(5));
}

#[test]
fn split_brain_higher_version_wins_merges_values() {
    let mut winner = BTreeMap::new();
    winner.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::value(
            PartitionId::new(1),
            4,
            ClusterEpoch::new(4),
            b"old".to_vec(),
        ),
    );
    let mut loser = BTreeMap::new();
    loser.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::value(
            PartitionId::new(1),
            6,
            ClusterEpoch::new(3),
            b"new".to_vec(),
        ),
    );

    let outcome = merge_split_brain_records(
        ClusterEpoch::new(4),
        ClusterEpoch::new(3),
        winner,
        loser,
        &HigherVersionWins,
    );

    assert_eq!(outcome.records["user:42"].version, 6);
    assert_eq!(outcome.report.merged_entries, 1);
}

#[test]
fn split_brain_tombstone_on_winner_beats_value_on_loser() {
    let mut winner = BTreeMap::new();
    winner.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::tombstone(PartitionId::new(1), 9, ClusterEpoch::new(5), None),
    );
    let mut loser = BTreeMap::new();
    loser.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::value(
            PartitionId::new(1),
            9,
            ClusterEpoch::new(5),
            b"stale".to_vec(),
        ),
    );

    let outcome = merge_split_brain_records(
        ClusterEpoch::new(5),
        ClusterEpoch::new(4),
        winner,
        loser,
        &HigherVersionWins,
    );

    assert!(outcome.records["user:42"].is_tombstone());
}

#[test]
fn split_brain_merge_runs_as_topology_op_not_hot_path() {
    let winner = BTreeMap::new();
    let mut loser = BTreeMap::new();
    loser.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::value(
            PartitionId::new(1),
            1,
            ClusterEpoch::new(1),
            b"value".to_vec(),
        ),
    );

    let outcome = merge_split_brain_records(
        ClusterEpoch::new(2),
        ClusterEpoch::new(1),
        winner,
        loser,
        &PutIfAbsent,
    );

    assert_eq!(outcome.records.len(), 1);
    assert_eq!(outcome.report.merged_entries, 1);
}

#[test]
fn split_brain_discard_policy_counts_loser_entries() {
    let winner = BTreeMap::new();
    let mut loser = BTreeMap::new();
    loser.insert(
        "user:42".to_owned(),
        ReplicatedValueRecord::value(
            PartitionId::new(1),
            1,
            ClusterEpoch::new(1),
            b"value".to_vec(),
        ),
    );

    let outcome = merge_split_brain_records(
        ClusterEpoch::new(2),
        ClusterEpoch::new(1),
        winner,
        loser,
        &DiscardLoser,
    );

    assert!(outcome.records.is_empty());
    assert_eq!(outcome.report.discarded_entries, 1);
}
