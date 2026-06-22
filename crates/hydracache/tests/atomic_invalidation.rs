use hydracache::{
    partition_for_key, BatchInvalidationState, ClusterEpoch, InvalidateBatch, InvalidationSaga,
    InvalidationTarget, WriteWatermark,
};

fn two_keys_same_partition(partition_count: u32) -> (String, String) {
    for left in 0..100 {
        for right in left + 1..100 {
            let left_key = format!("key:{left}");
            let right_key = format!("key:{right}");
            if partition_for_key(&left_key, partition_count)
                == partition_for_key(&right_key, partition_count)
            {
                return (left_key, right_key);
            }
        }
    }
    panic!("no same-partition keys found");
}

fn two_keys_different_partitions(partition_count: u32) -> (String, String) {
    for left in 0..100 {
        for right in left + 1..100 {
            let left_key = format!("key:{left}");
            let right_key = format!("key:{right}");
            if partition_for_key(&left_key, partition_count)
                != partition_for_key(&right_key, partition_count)
            {
                return (left_key, right_key);
            }
        }
    }
    panic!("no cross-partition keys found");
}

#[test]
fn atomic_invalidation_single_partition_batch_is_all_or_nothing() {
    let (left, right) = two_keys_same_partition(16);
    let batch =
        InvalidateBatch::try_new([left.clone(), right.clone()], 16, 10, ClusterEpoch::new(2))
            .unwrap();
    let mut state = BatchInvalidationState::default();

    state.apply_batch(&batch);

    assert!(state.batch_is_all_or_nothing(&batch));
    assert_eq!(state.watermark(&left), state.watermark(&right));
}

#[test]
fn atomic_invalidation_cross_partition_batch_is_rejected_pointing_at_saga() {
    let (left, right) = two_keys_different_partitions(16);

    let error = InvalidateBatch::try_new([left, right], 16, 1, ClusterEpoch::new(1)).unwrap_err();

    assert!(error.to_string().contains("InvalidationSaga"));
}

#[test]
fn atomic_invalidation_saga_fans_out_at_least_once_idempotently() {
    let targets = vec![
        InvalidationTarget::new(partition_for_key("a", 16), "a"),
        InvalidationTarget::new(partition_for_key("b", 16), "b"),
    ];
    let mut saga = InvalidationSaga::new("commit:1", targets.clone());

    assert!(saga.dispatch_target(&targets[0]));
    assert!(!saga.dispatch_target(&targets[0]));
    assert_eq!(saga.pending(), 1);
    assert!(saga.dispatch_target(&targets[1]));
    assert!(saga.is_complete());
}

#[test]
#[ignore = "chaos gate: dispatcher crash/resume"]
fn atomic_invalidation_saga_survives_dispatcher_crash() {
    let targets = vec![
        InvalidationTarget::new(partition_for_key("a", 16), "a"),
        InvalidationTarget::new(partition_for_key("b", 16), "b"),
    ];
    let mut saga = InvalidationSaga::new("commit:2", targets.clone());
    assert!(saga.dispatch_target(&targets[0]));

    let mut resumed = saga.clone();
    assert!(!resumed.dispatch_target(&targets[0]));
    assert!(resumed.dispatch_target(&targets[1]));
    assert!(resumed.is_complete());
}

#[test]
fn atomic_invalidation_batch_version_beats_concurrent_single_writes() {
    let (key, other) = two_keys_same_partition(16);
    let partition = partition_for_key(&key, 16);
    let batch =
        InvalidateBatch::try_new([key.clone(), other], 16, 10, ClusterEpoch::new(5)).unwrap();
    let mut state = BatchInvalidationState::default();
    state.apply_single(
        key.clone(),
        WriteWatermark::new(partition, 9, ClusterEpoch::new(5)),
    );

    state.apply_batch(&batch);

    assert_eq!(state.watermark(&key).expect("watermark").version, 10);
}
