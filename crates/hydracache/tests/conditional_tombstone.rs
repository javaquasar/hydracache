use hydracache::{
    partition_for_key, CasResult, ClusterEpoch, ConsistencyLevel, ReplicatedValueRecord,
    SingleKeyConditionalStore,
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
