use hydracache::{
    prepare_replicated_payload, ClusterEpoch, CompatVersion, EffectiveReplicationMap,
    InMemoryReplicatedValueStore, PartitionId, Replicas, ReplicatedValueRecord,
    ReplicatedValueStore, Replication, ReplicationCryptoError, ReplicationKeyProvider,
    UpgradeGuard, UpgradeStep, CACHE_INVALIDATION_FRAME_VERSION,
    REPLICATED_VALUE_RECORD_FORMAT_VERSION,
};

#[test]
fn durable_replicated_values_replicated_value_survives_backup_restart() {
    let mut store = InMemoryReplicatedValueStore::with_budget(1024);
    let record = ReplicatedValueRecord::value(
        PartitionId::new(3),
        7,
        ClusterEpoch::new(2),
        b"sealed-user".to_vec(),
    );

    store.upsert("user:42", record.clone()).unwrap();
    let snapshot = store.snapshot();
    drop(store);

    let reopened = InMemoryReplicatedValueStore::reopen_from_snapshot(1024, snapshot);
    assert_eq!(reopened.get("user:42").unwrap(), Some(record));
}

#[test]
fn durable_replicated_values_restart_then_anti_entropy_converges_with_primary() {
    let mut backup = InMemoryReplicatedValueStore::with_budget(1024);
    backup
        .upsert(
            "user:42",
            ReplicatedValueRecord::value(
                PartitionId::new(3),
                7,
                ClusterEpoch::new(2),
                b"old".to_vec(),
            ),
        )
        .unwrap();
    let snapshot = backup.snapshot();
    let mut reopened = InMemoryReplicatedValueStore::reopen_from_snapshot(1024, snapshot);

    reopened
        .upsert(
            "user:42",
            ReplicatedValueRecord::value(
                PartitionId::new(3),
                9,
                ClusterEpoch::new(3),
                b"new".to_vec(),
            ),
        )
        .unwrap();

    let stored = reopened.get("user:42").unwrap().unwrap();
    assert_eq!(stored.version, 9);
    assert_eq!(stored.epoch, ClusterEpoch::new(3));
}

#[test]
fn durable_replicated_values_sealed_bytes_only_are_persisted() {
    struct XorProvider;

    impl ReplicationKeyProvider for XorProvider {
        fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, ReplicationCryptoError> {
            Ok(plaintext.iter().map(|byte| byte ^ 0xaa).collect())
        }

        fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, ReplicationCryptoError> {
            self.seal(sealed)
        }
    }

    let provider = XorProvider;
    let payload = prepare_replicated_payload(
        b"secret-profile",
        Replication::Eligible,
        false,
        Some(&provider),
        None,
    )
    .unwrap()
    .unwrap();
    assert_ne!(payload.bytes, b"secret-profile");

    let mut store = InMemoryReplicatedValueStore::with_budget(1024);
    store
        .upsert(
            "profile:7",
            ReplicatedValueRecord::value(
                PartitionId::new(9),
                1,
                ClusterEpoch::new(1),
                payload.bytes.clone(),
            ),
        )
        .unwrap();

    let stored = store.get("profile:7").unwrap().unwrap();
    let sealed = match stored.state {
        hydracache::ReplicatedSlot::Value { value, .. } => value,
        hydracache::ReplicatedSlot::Tombstone { .. } => panic!("expected value"),
    };
    assert_ne!(sealed, b"secret-profile");
    assert_eq!(provider.open(&sealed).unwrap(), b"secret-profile");
}

#[test]
fn durable_replicated_values_total_bytes_budget_rejects_over_limit_not_silently() {
    let mut store = InMemoryReplicatedValueStore::with_budget(4);

    let error = store
        .upsert(
            "big",
            ReplicatedValueRecord::value(
                PartitionId::new(1),
                1,
                ClusterEpoch::new(1),
                b"too-large".to_vec(),
            ),
        )
        .unwrap_err();

    assert!(error.to_string().contains("budget exceeded"));
    assert_eq!(store.rejected_total(), 1);
    assert!(store.get("big").unwrap().is_none());
}

#[test]
fn durable_replicated_values_tombstone_persisted_blocks_resurrection_after_restart() {
    let mut store = InMemoryReplicatedValueStore::with_budget(1024);
    store
        .tombstone("user:42", PartitionId::new(3), 10, ClusterEpoch::new(4))
        .unwrap();
    let snapshot = store.snapshot();
    let mut reopened = InMemoryReplicatedValueStore::reopen_from_snapshot(1024, snapshot);

    reopened
        .upsert(
            "user:42",
            ReplicatedValueRecord::value(
                PartitionId::new(3),
                9,
                ClusterEpoch::new(3),
                b"stale".to_vec(),
            ),
        )
        .unwrap();

    let stored = reopened.get("user:42").unwrap().unwrap();
    assert!(stored.is_tombstone());
    assert_eq!(stored.version, 10);
}

#[test]
fn durable_replicated_values_format_version_round_trips() {
    assert_eq!(REPLICATED_VALUE_RECORD_FORMAT_VERSION, 1);

    let record = ReplicatedValueRecord::value(
        PartitionId::new(9),
        17,
        ClusterEpoch::new(4),
        b"sealed-current-format".to_vec(),
    );
    let encoded = serde_json::to_string(&record).unwrap();
    let decoded: ReplicatedValueRecord = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, record);
}

#[test]
fn durable_replicated_values_future_format_is_rejected_by_upgrade_guard() {
    let guard = UpgradeGuard::current();
    let step = UpgradeStep {
        from: CompatVersion::new(0, 42, 0),
        to: CompatVersion::new(0, 43, 0),
        raft_log_format: 1,
        value_record_format: REPLICATED_VALUE_RECORD_FORMAT_VERSION + 1,
        wire_frame_version: CACHE_INVALIDATION_FRAME_VERSION,
    };

    let error = guard.check(step).unwrap_err();

    assert!(error.to_string().contains("incompatible persisted"));
}

#[test]
fn durable_replicated_values_scan_owned_returns_records_for_readable_map() {
    let mut store = InMemoryReplicatedValueStore::with_budget(1024);
    store
        .upsert(
            "user:42",
            ReplicatedValueRecord::value(
                PartitionId::new(1),
                1,
                ClusterEpoch::new(1),
                b"v".to_vec(),
            ),
        )
        .unwrap();
    let map = EffectiveReplicationMap::new(Replicas::new("member-a", vec!["member-b".into()]));

    let owned = store.scan_owned(&map).unwrap();
    assert_eq!(owned.len(), 1);
    assert_eq!(owned[0].0, "user:42");
}
