use std::collections::BTreeMap;

use hydracache::{
    partition_for_key, CasResult, ClusterEpoch, ClusterNodeId, ConsistencyLevel,
    ReplicaAppliedPrefix, ReplicatedRecordApply, ReplicatedValueRecord, SingleKeyConditionalStore,
    TombstoneGcError, TombstoneGcWatermark,
};

fn store() -> SingleKeyConditionalStore {
    SingleKeyConditionalStore::new(ClusterEpoch::new(7), 16)
}

#[test]
fn remove_if_value_writes_tombstone_at_new_version() {
    let mut store = store();
    assert_eq!(
        store
            .compare_and_set(
                "user:42",
                None,
                b"active".to_vec(),
                ConsistencyLevel::Quorum
            )
            .unwrap(),
        CasResult::Applied { new_version: 1 }
    );

    let removed = store
        .remove_if_value("user:42", b"active", ConsistencyLevel::Quorum)
        .unwrap();

    assert_eq!(removed, CasResult::Applied { new_version: 2 });
    let record = store.record("user:42").expect("tombstone record");
    assert!(record.is_tombstone());
    assert_eq!(record.version, 2);
    assert_eq!(store.current_value("user:42"), None);
    assert_eq!(store.metrics().cas_applied_total, 2);
    let retained = store.retained_state();
    assert_eq!(retained.records, 1);
    assert_eq!(retained.live_records, 0);
    assert_eq!(retained.tombstones, 1);
    assert_eq!(retained.locks, 0);
    assert_eq!(retained.session_heartbeats, 0);
    assert!(retained.identity_bytes >= "user:42".len());
}

#[test]
fn remove_if_value_mismatch_leaves_record_untouched() {
    let mut store = store();
    store
        .compare_and_set(
            "user:42",
            None,
            b"active".to_vec(),
            ConsistencyLevel::Quorum,
        )
        .unwrap();
    let before = store.record("user:42").cloned();

    let mismatch = store
        .remove_if_value("user:42", b"disabled", ConsistencyLevel::Quorum)
        .unwrap();

    assert_eq!(
        mismatch,
        CasResult::Mismatch {
            current: Some(b"active".to_vec())
        }
    );
    assert_eq!(store.record("user:42").cloned(), before);
    assert_eq!(store.metrics().cas_mismatch_total, 1);
}

#[test]
fn tombstone_from_remove_if_value_is_not_resurrected_by_stale_put() {
    let mut store = store();
    store
        .compare_and_set(
            "user:42",
            None,
            b"active".to_vec(),
            ConsistencyLevel::Quorum,
        )
        .unwrap();
    store
        .remove_if_value("user:42", b"active", ConsistencyLevel::Quorum)
        .unwrap();
    let tombstone = store.record("user:42").expect("tombstone").clone();
    let stale_value = ReplicatedValueRecord::value(
        partition_for_key("user:42", 16),
        tombstone.version,
        ClusterEpoch::new(7),
        b"stale".to_vec(),
    );

    let merged = tombstone.merge(stale_value);

    assert!(merged.is_tombstone());
    assert_eq!(merged.version, 2);
}

#[test]
fn removed_key_reads_as_absent_after_tombstone() {
    let mut store = store();
    store
        .compare_and_set(
            "user:42",
            None,
            b"active".to_vec(),
            ConsistencyLevel::Quorum,
        )
        .unwrap();

    store
        .remove_if_value("user:42", b"active", ConsistencyLevel::Quorum)
        .unwrap();

    assert_eq!(store.current_value("user:42"), None);
    assert_eq!(
        store
            .remove_if_value("user:42", b"active", ConsistencyLevel::Quorum)
            .unwrap(),
        CasResult::Mismatch { current: None }
    );
}

#[test]
fn replace_if_present_applies_only_when_live_value_exists() {
    let mut store = store();
    store
        .compare_and_set(
            "user:42",
            None,
            b"active".to_vec(),
            ConsistencyLevel::Quorum,
        )
        .unwrap();

    let replaced = store
        .replace_if_present("user:42", b"disabled".to_vec(), ConsistencyLevel::Quorum)
        .unwrap();

    assert_eq!(replaced, CasResult::Applied { new_version: 2 });
    assert_eq!(store.current_value("user:42"), Some(b"disabled".to_vec()));
}

#[test]
fn replace_if_present_on_absent_is_mismatch_not_insert() {
    let mut store = store();

    let result = store
        .replace_if_present("missing", b"created".to_vec(), ConsistencyLevel::Quorum)
        .unwrap();

    assert_eq!(result, CasResult::Mismatch { current: None });
    assert_eq!(store.current_value("missing"), None);
    assert_eq!(store.metrics().cas_mismatch_total, 1);
}

#[test]
fn replace_if_present_after_tombstone_is_mismatch_not_resurrection() {
    let mut store = store();
    store
        .compare_and_set(
            "user:42",
            None,
            b"active".to_vec(),
            ConsistencyLevel::Quorum,
        )
        .unwrap();
    store
        .remove_if_value("user:42", b"active", ConsistencyLevel::Quorum)
        .unwrap();

    let result = store
        .replace_if_present("user:42", b"resurrected".to_vec(), ConsistencyLevel::Quorum)
        .unwrap();

    assert_eq!(result, CasResult::Mismatch { current: None });
    assert_eq!(store.current_value("user:42"), None);
    assert!(store.record("user:42").expect("tombstone").is_tombstone());
}

#[test]
fn ordering_safe_conditional_tombstone_watermark_reclaims_unique_deletes() {
    const PARTITIONS: u32 = 16;
    const UNIQUE_DELETES: u64 = 10_000;
    let epoch = ClusterEpoch::new(7);
    let replicas = [
        ClusterNodeId::new("replica-a"),
        ClusterNodeId::new("replica-b"),
        ClusterNodeId::new("replica-c"),
    ];
    let mut store = store();
    let mut applied_prefixes = BTreeMap::new();
    let mut stale_sample = None;

    for index in 0..UNIQUE_DELETES {
        let key = format!("deleted:{index}");
        store
            .put_if_absent(&key, b"value".to_vec(), ConsistencyLevel::All)
            .unwrap();
        let result = store
            .remove_if_value(&key, b"value", ConsistencyLevel::All)
            .unwrap();
        let CasResult::Applied { new_version } = result else {
            panic!("delete must write a tombstone");
        };
        let partition = partition_for_key(&key, PARTITIONS);
        applied_prefixes
            .entry(partition)
            .and_modify(|version: &mut u64| *version = (*version).max(new_version))
            .or_insert(new_version);
        if stale_sample.is_none() {
            stale_sample = Some((key, partition, new_version));
        }
    }

    assert_eq!(store.retained_state().tombstones, UNIQUE_DELETES as usize);
    let mut reclaimed = 0usize;
    for (partition, version) in applied_prefixes {
        let progress = replicas
            .iter()
            .cloned()
            .map(|replica| ReplicaAppliedPrefix::new(replica, partition, version, epoch));
        let watermark = TombstoneGcWatermark::from_all_replicas(
            partition,
            epoch,
            replicas.iter().cloned(),
            progress,
        )
        .expect("all effective replicas acknowledged one ordered prefix");
        assert_eq!(watermark.replica_count(), replicas.len());
        reclaimed += store
            .advance_tombstone_gc_watermark(watermark)
            .expect("watermark advances monotonically");
    }

    assert_eq!(reclaimed, UNIQUE_DELETES as usize);
    let retained = store.retained_state();
    assert_eq!(retained.records, 0);
    assert_eq!(retained.tombstones, 0);
    assert_eq!(retained.identity_bytes, 0);
    assert!(retained.tombstone_gc_watermarks <= PARTITIONS as usize);
    assert_eq!(store.metrics().tombstone_gc_reclaimed_total, UNIQUE_DELETES);

    let (key, partition, deleted_version) = stale_sample.expect("sample delete");
    let stale_value =
        ReplicatedValueRecord::value(partition, deleted_version, epoch, b"stale".to_vec());
    assert_eq!(
        store.apply_replicated_record(&key, stale_value).unwrap(),
        ReplicatedRecordApply::RejectedBelowGcWatermark
    );
    assert_eq!(store.current_value(&key), None);
    assert_eq!(store.metrics().tombstone_gc_stale_record_rejected_total, 1);

    let CasResult::Applied { new_version } = store
        .put_if_absent(&key, b"fresh".to_vec(), ConsistencyLevel::All)
        .unwrap()
    else {
        panic!("a new ordered write may recreate the key");
    };
    assert!(new_version > deleted_version);
    assert_eq!(store.current_value(&key), Some(b"fresh".to_vec()));
}

#[test]
fn tombstone_watermark_requires_every_effective_replica() {
    let epoch = ClusterEpoch::new(7);
    let partition = partition_for_key("deleted", 16);
    let replicas = [
        ClusterNodeId::new("replica-a"),
        ClusterNodeId::new("replica-b"),
        ClusterNodeId::new("replica-c"),
    ];
    let progress = replicas[..2]
        .iter()
        .cloned()
        .map(|replica| ReplicaAppliedPrefix::new(replica, partition, 9, epoch));

    assert!(matches!(
        TombstoneGcWatermark::from_all_replicas(
            partition,
            epoch,
            replicas.iter().cloned(),
            progress,
        ),
        Err(TombstoneGcError::MissingReplica { replica }) if replica == replicas[2]
    ));
}

#[test]
fn tombstone_watermark_uses_the_slowest_replica_and_rejects_malformed_progress() {
    let epoch = ClusterEpoch::new(7);
    let partition = partition_for_key("deleted", 16);
    let other_partition = partition_for_key("another-partition", 16);
    assert_ne!(partition, other_partition);
    let replica_a = ClusterNodeId::new("replica-a");
    let replica_b = ClusterNodeId::new("replica-b");
    let replicas = [replica_a.clone(), replica_b.clone()];

    let watermark = TombstoneGcWatermark::from_all_replicas(
        partition,
        epoch,
        replicas.iter().cloned(),
        [
            ReplicaAppliedPrefix::new(replica_a.clone(), partition, 12, epoch),
            ReplicaAppliedPrefix::new(replica_b.clone(), partition, 9, epoch),
        ],
    )
    .unwrap();
    assert_eq!(watermark.version(), 9);

    assert_eq!(
        TombstoneGcWatermark::from_all_replicas(partition, epoch, [], []),
        Err(TombstoneGcError::EmptyReplicaSet)
    );
    assert!(matches!(
        TombstoneGcWatermark::from_all_replicas(
            partition,
            epoch,
            replicas.iter().cloned(),
            [
                ReplicaAppliedPrefix::new(replica_a.clone(), partition, 9, epoch),
                ReplicaAppliedPrefix::new(replica_a.clone(), partition, 9, epoch),
            ],
        ),
        Err(TombstoneGcError::DuplicateReplica { replica }) if replica == replica_a
    ));
    assert!(matches!(
        TombstoneGcWatermark::from_all_replicas(
            partition,
            epoch,
            [replica_a.clone()],
            [ReplicaAppliedPrefix::new(
                replica_b.clone(),
                partition,
                9,
                epoch,
            )],
        ),
        Err(TombstoneGcError::UnexpectedReplica { replica }) if replica == replica_b
    ));
    assert!(matches!(
        TombstoneGcWatermark::from_all_replicas(
            partition,
            epoch,
            [replica_a.clone()],
            [ReplicaAppliedPrefix::new(
                replica_a.clone(),
                other_partition,
                9,
                epoch,
            )],
        ),
        Err(TombstoneGcError::ProgressPartitionMismatch { .. })
    ));
    assert!(matches!(
        TombstoneGcWatermark::from_all_replicas(
            partition,
            epoch,
            [replica_a.clone()],
            [ReplicaAppliedPrefix::new(
                replica_a,
                partition,
                9,
                ClusterEpoch::new(8),
            )],
        ),
        Err(TombstoneGcError::ProgressEpochMismatch { .. })
    ));
}

#[test]
fn replicated_record_must_match_its_key_partition() {
    let mut store = store();
    let key = "deleted";
    let expected = partition_for_key(key, 16);
    let actual = partition_for_key("another-partition", 16);
    assert_ne!(expected, actual);

    let error = store
        .apply_replicated_record(
            key,
            ReplicatedValueRecord::value(actual, 1, ClusterEpoch::new(7), b"value".to_vec()),
        )
        .unwrap_err();
    assert_eq!(
        error,
        TombstoneGcError::RecordPartitionMismatch { expected, actual }
    );
    assert_eq!(store.retained_state().records, 0);
}

#[test]
fn tombstone_watermark_rejects_epoch_change_and_regression() {
    let epoch = ClusterEpoch::new(7);
    let partition = partition_for_key("deleted", 16);
    let replicas = [ClusterNodeId::new("replica-a")];
    let proof = |version, proof_epoch| {
        TombstoneGcWatermark::from_all_replicas(
            partition,
            proof_epoch,
            replicas.iter().cloned(),
            replicas
                .iter()
                .cloned()
                .map(|replica| ReplicaAppliedPrefix::new(replica, partition, version, proof_epoch)),
        )
        .unwrap()
    };
    let mut store = store();

    assert!(matches!(
        store.advance_tombstone_gc_watermark(proof(9, ClusterEpoch::new(8))),
        Err(TombstoneGcError::StoreEpochMismatch { .. })
    ));
    store
        .advance_tombstone_gc_watermark(proof(9, epoch))
        .unwrap();
    assert!(matches!(
        store.advance_tombstone_gc_watermark(proof(8, epoch)),
        Err(TombstoneGcError::WatermarkRegression {
            current: 9,
            proposed: 8,
            ..
        })
    ));
}
