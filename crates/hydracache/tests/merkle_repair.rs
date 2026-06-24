use std::collections::BTreeMap;

use hydracache::{
    foreground_read_repair, ClusterEpoch, ClusterNodeId, KeyRange, MerkleTree, PartitionId,
    RepairKind, RepairSession, ReplicatedValueRecord,
};

fn record(partition: PartitionId, key_version: u64) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        partition,
        key_version,
        ClusterEpoch::new(1),
        vec![key_version as u8],
    )
}

fn records(values: &[(&str, u64)]) -> BTreeMap<String, ReplicatedValueRecord> {
    let partition = PartitionId::new(3);
    values
        .iter()
        .map(|(key, version)| ((*key).to_owned(), record(partition, *version)))
        .collect()
}

#[test]
fn merkle_repair_diff_descends_only_mismatches() {
    let partition = PartitionId::new(3);
    let left = MerkleTree::from_records(partition, &records(&[("a", 1), ("b", 1), ("c", 1)]));
    let same = MerkleTree::from_records(partition, &records(&[("a", 1), ("b", 1), ("c", 1)]));
    let right = MerkleTree::from_records(partition, &records(&[("a", 1), ("b", 2), ("c", 1)]));

    assert!(left.diff(&same).is_empty());
    assert_eq!(left.diff(&right), vec![KeyRange::single("b")]);
}

#[test]
fn merkle_repair_foreground_read_repair_fixes_divergence_inline() {
    let partition = PartitionId::new(3);
    let stale = record(partition, 1);
    let fresh = record(partition, 2);

    let outcome = foreground_read_repair([Some(stale), Some(fresh.clone())]);

    assert_eq!(outcome.served, Some(fresh.clone()));
    assert_eq!(outcome.repairs, vec![fresh]);
}

#[test]
fn merkle_repair_incremental_repair_skips_repaired_ranges() {
    let partition = PartitionId::new(3);
    let left = MerkleTree::from_records(partition, &records(&[("a", 1), ("b", 1), ("c", 1)]));
    let right = MerkleTree::from_records(partition, &records(&[("a", 1), ("b", 2), ("c", 1)]));
    let mut session = RepairSession::new(partition, vec![ClusterNodeId::new("replica-b")]);

    let first = session.run(&left, &right, RepairKind::ScheduledIncremental);
    let second = session.run(&left, &right, RepairKind::ScheduledIncremental);

    assert_eq!(first.ranges_exchanged(), 1);
    assert_eq!(second.ranges_exchanged(), 0);
    assert_eq!(second.skipped_repaired_ranges, 1);
}

#[test]
fn merkle_repair_preserves_tombstone_invariant() {
    let partition = PartitionId::new(3);
    let tombstone = ReplicatedValueRecord::tombstone(partition, 2, ClusterEpoch::new(1), None);
    let concurrent_value = record(partition, 2);

    let outcome = foreground_read_repair([Some(concurrent_value), Some(tombstone.clone())]);

    assert_eq!(outcome.served, Some(tombstone.clone()));
    assert_eq!(outcome.repairs, vec![tombstone]);
}

#[test]
fn merkle_repair_empty_tree_has_no_ranges() {
    let partition = PartitionId::new(3);
    let empty = MerkleTree::empty(partition);

    assert!(empty.is_empty());
    assert!(empty.diff(&MerkleTree::empty(partition)).is_empty());
}

#[test]
#[ignore = "chaos marker: resumable repair after coordinator crash"]
fn merkle_repair_session_resumes_after_coordinator_crash() {
    let partition = PartitionId::new(3);
    let left = MerkleTree::from_records(partition, &records(&[("a", 1), ("b", 1)]));
    let right = MerkleTree::from_records(partition, &records(&[("a", 2), ("b", 2)]));
    let mut session = RepairSession::new(partition, vec![ClusterNodeId::new("replica-b")]);

    let first = session.run(&left, &right, RepairKind::ScheduledIncremental);
    let resumed = RepairSession {
        partition,
        peers: vec![ClusterNodeId::new("replica-b")],
        repaired_watermark: first.repaired_watermark.clone(),
    }
    .run(&left, &right, RepairKind::ScheduledIncremental);

    assert!(resumed.ranges_exchanged() < first.ranges_exchanged());
}
