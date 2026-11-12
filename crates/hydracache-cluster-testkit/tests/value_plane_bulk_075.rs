use std::collections::BTreeMap;

use hydracache_cluster_testkit::value_plane_bulk_075::{
    BulkBounds, BulkError, BulkInput, BulkOutcome, PartitionedBulkExecution,
};
use hydracache_cluster_testkit::value_plane_model_075::{CanonicalMapKey, MutationIdentity};

fn input(index: usize, partition: u32) -> BulkInput {
    BulkInput {
        index,
        key: CanonicalMapKey::new("tenant-a", "orders", 1, vec![b'k', index as u8], partition),
        identity: MutationIdentity::new("bulk-client", index as u64 + 1),
        request_bytes: 4,
    }
}

fn execution() -> PartitionedBulkExecution {
    PartitionedBulkExecution::new(
        BulkBounds {
            max_items: 4,
            max_request_bytes: 32,
            max_partitions: 2,
        },
        vec![input(0, 1), input(1, 2), input(2, 1)],
        &BTreeMap::from([(1, 7), (2, 8)]),
    )
    .unwrap()
}

#[test]
fn work_is_partition_grouped_but_receipts_remain_input_ordered() {
    let mut execution = execution();
    let groups = execution.pending_groups();
    assert_eq!(groups[0].partition, 1);
    assert_eq!(groups[0].input_indices, vec![0, 2]);
    assert_eq!(groups[1].input_indices, vec![1]);
    execution
        .complete(2, 7, BulkOutcome::Applied { version: 12 })
        .unwrap();
    execution.complete(0, 7, BulkOutcome::Absent).unwrap();
    let receipt = execution.receipt();
    assert_eq!(
        receipt
            .items
            .iter()
            .map(|item| item.input_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(!receipt.complete);
}

#[test]
fn owner_change_retries_only_pending_items_and_preserves_successes() {
    let mut execution = execution();
    execution
        .complete(0, 7, BulkOutcome::Applied { version: 10 })
        .unwrap();
    execution.advance_owner(1, 9).unwrap();
    let group = execution
        .pending_groups()
        .into_iter()
        .find(|group| group.partition == 1)
        .unwrap();
    assert_eq!(group.owner_generation, 9);
    assert_eq!(group.input_indices, vec![2]);
    let receipt = execution.receipt();
    assert_eq!(receipt.items[0].owner_generation, 7);
    assert_eq!(receipt.items[2].owner_generation, 9);
}

#[test]
fn stale_completion_and_conflicting_replay_fail_loud() {
    let mut execution = execution();
    execution.advance_owner(1, 9).unwrap();
    assert_eq!(
        execution.complete(0, 7, BulkOutcome::Absent),
        Err(BulkError::StaleOwnerGeneration {
            expected: 9,
            actual: 7
        })
    );
    execution.complete(0, 9, BulkOutcome::Absent).unwrap();
    assert_eq!(
        execution.complete(0, 9, BulkOutcome::Applied { version: 1 }),
        Err(BulkError::ConflictingCompletion)
    );
}

#[test]
fn duplicate_keys_and_request_bounds_reject_before_execution() {
    let first = input(0, 1);
    let mut duplicate = input(1, 1);
    duplicate.key = first.key.clone();
    assert_eq!(
        PartitionedBulkExecution::new(
            BulkBounds {
                max_items: 2,
                max_request_bytes: 8,
                max_partitions: 1,
            },
            vec![first, duplicate],
            &BTreeMap::from([(1, 1)]),
        )
        .unwrap_err(),
        BulkError::DuplicateKey
    );
    assert_eq!(
        PartitionedBulkExecution::new(
            BulkBounds {
                max_items: 1,
                max_request_bytes: 4,
                max_partitions: 1,
            },
            vec![input(0, 1), input(1, 2)],
            &BTreeMap::from([(1, 1), (2, 1)]),
        )
        .unwrap_err(),
        BulkError::ItemLimit
    );
}
