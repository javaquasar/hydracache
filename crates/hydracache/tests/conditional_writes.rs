use hydracache::{
    CasResult, ClusterEpoch, ConditionalError, ConsistencyLevel, LockOwner, LogicalDuration,
    LogicalTime, SingleKeyConditionalStore,
};

fn store() -> SingleKeyConditionalStore {
    SingleKeyConditionalStore::new(ClusterEpoch::new(1), 16)
}

fn owner(name: &str) -> LockOwner {
    LockOwner::new(name, 1)
}

fn lease() -> LogicalDuration {
    LogicalDuration::from_millis(10)
}

fn now(ms: u64) -> LogicalTime {
    LogicalTime::from_millis(ms)
}

#[test]
fn conditional_writes_compare_and_set_applies_only_on_match() {
    let mut store = store();

    let first = store
        .compare_and_set("job:1", None, b"queued".to_vec(), ConsistencyLevel::Quorum)
        .unwrap();
    let mismatch = store
        .compare_and_set(
            "job:1",
            Some(b"running"),
            b"done".to_vec(),
            ConsistencyLevel::Quorum,
        )
        .unwrap();

    assert_eq!(first, CasResult::Applied { new_version: 1 });
    assert!(matches!(mismatch, CasResult::Mismatch { .. }));
    assert_eq!(store.metrics().cas_applied_total, 1);
    assert_eq!(store.metrics().cas_mismatch_total, 1);
}

#[test]
fn conditional_writes_put_if_absent_is_linearizable_under_contention() {
    let mut store = store();
    let mut applied = 0;

    for contender in 0..10 {
        if matches!(
            store
                .put_if_absent("leader", vec![contender], ConsistencyLevel::EachQuorum)
                .unwrap(),
            CasResult::Applied { .. }
        ) {
            applied += 1;
        }
    }

    assert_eq!(applied, 1);
    assert_eq!(store.metrics().cas_applied_total, 1);
    assert_eq!(store.metrics().cas_mismatch_total, 9);
}

#[test]
fn conditional_writes_cas_at_consistency_one_is_refused() {
    let mut store = store();

    let error = store
        .compare_and_set("job:1", None, b"queued".to_vec(), ConsistencyLevel::One)
        .expect_err("One cannot provide a linearizable CAS");

    assert_eq!(
        error,
        ConditionalError::WeakConsistency {
            level: ConsistencyLevel::One
        }
    );
}

#[test]
fn conditional_writes_multi_key_conditional_is_rejected_loud() {
    let error = SingleKeyConditionalStore::reject_multi_key(["a", "b"])
        .expect_err("multi-key CAS must stay out of scope");

    assert_eq!(error, ConditionalError::MultiKeyRejected { key_count: 2 });
}

#[test]
fn conditional_writes_fenced_lock_rejects_stale_token() {
    let mut store = store();
    let first = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner("session-a"),
            lease(),
            now(0),
        )
        .unwrap()
        .expect("first lock acquisition succeeds");
    let second = store
        .force_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner("session-b"),
            lease(),
            now(1),
        )
        .unwrap();

    assert!(second > first);
    assert!(matches!(
        store.validate_fence_token("lock:refresh", first),
        Err(ConditionalError::StaleFenceToken { .. })
    ));
    assert_eq!(store.metrics().lock_stale_token_rejected_total, 1);
}

#[test]
fn conditional_writes_cas_respects_tombstone_version() {
    let mut store = store();
    store.apply_tombstone("job:1", 5);

    let result = store
        .put_if_absent("job:1", b"reborn".to_vec(), ConsistencyLevel::All)
        .unwrap();

    assert_eq!(result, CasResult::Applied { new_version: 6 });
    assert_eq!(store.record("job:1").map(|record| record.version), Some(6));
}

#[test]
#[ignore = "chaos marker: lock survives partition via partition-home authority"]
fn conditional_writes_lock_survives_partition_via_raft_authority() {
    let mut store = store();
    let token = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner("session-a"),
            lease(),
            now(0),
        )
        .unwrap()
        .expect("lock acquired before simulated partition");

    assert!(store
        .release_lock("lock:refresh", &owner("session-a"), token)
        .is_ok());
}
