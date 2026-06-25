use hydracache::{
    ClusterEpoch, ConditionalError, ConsistencyLevel, LockOwner, LogicalDuration, LogicalTime,
    SingleKeyConditionalStore,
};

fn store() -> SingleKeyConditionalStore {
    SingleKeyConditionalStore::new(ClusterEpoch::new(1), 16)
}

fn owner(session: &str, endpoint: u64) -> LockOwner {
    LockOwner::new(session, endpoint)
}

fn at(millis: u64) -> LogicalTime {
    LogicalTime::from_millis(millis)
}

fn lease(millis: u64) -> LogicalDuration {
    LogicalDuration::from_millis(millis)
}

#[test]
fn reentrant_acquire_keeps_same_fence_and_counts() {
    let mut store = store();
    let owner = owner("session-a", 7);
    let first = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner.clone(),
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first acquire");
    let second = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner,
            lease(10),
            at(1),
        )
        .unwrap()
        .expect("reentrant acquire");

    assert_eq!(second, first);
    assert_eq!(
        store.lock_hold("lock:refresh").map(|hold| hold.holds),
        Some(2)
    );
}

#[test]
fn unlock_frees_only_at_zero_holds() {
    let mut store = store();
    let owner = owner("session-a", 7);
    let token = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner.clone(),
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first acquire");
    store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner.clone(),
            lease(10),
            at(1),
        )
        .unwrap()
        .expect("reentrant acquire");

    store.release_lock("lock:refresh", &owner, token).unwrap();
    assert!(store.is_locked("lock:refresh"));
    assert_eq!(
        store.lock_hold("lock:refresh").map(|hold| hold.holds),
        Some(1)
    );

    store.release_lock("lock:refresh", &owner, token).unwrap();
    assert!(!store.is_locked("lock:refresh"));
}

#[test]
fn reentrancy_limit_fails_loud() {
    let mut store = store().with_lock_acquire_limit(Some(1));
    let owner = owner("session-a", 7);
    let token = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner.clone(),
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first acquire");

    let error = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            owner,
            lease(10),
            at(1),
        )
        .expect_err("limit should reject second acquire");

    assert_eq!(
        error,
        ConditionalError::ReentrancyLimit {
            key: "lock:refresh".to_owned(),
            limit: 1
        }
    );
    assert_eq!(store.current_fence("lock:refresh"), Some(token));
    assert_eq!(store.metrics().lock_reentrancy_limit_rejected_total, 1);
}

#[test]
fn is_locked_by_owner_reflects_state() {
    let mut store = store();
    let lock_owner = owner("session-a", 7);
    let other = owner("session-b", 7);
    let token = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            lock_owner.clone(),
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first acquire");

    assert!(store.is_locked("lock:refresh"));
    assert!(store.is_locked_by("lock:refresh", &lock_owner));
    assert!(!store.is_locked_by("lock:refresh", &other));
    assert_eq!(store.current_fence("lock:refresh"), Some(token));
}

#[test]
fn reentrancy_is_acquire_count_on_session() {
    let mut store = store();
    let lock_owner = owner("session-a", 7);
    let same_session_other_endpoint = owner("session-a", 8);
    let first = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            lock_owner.clone(),
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first acquire");

    assert_eq!(
        store
            .try_acquire_lock(
                "lock:refresh",
                ConsistencyLevel::Quorum,
                lock_owner,
                lease(10),
                at(1),
            )
            .unwrap(),
        Some(first)
    );
    assert_eq!(
        store
            .try_acquire_lock(
                "lock:refresh",
                ConsistencyLevel::Quorum,
                same_session_other_endpoint,
                lease(10),
                at(2),
            )
            .unwrap(),
        None
    );
    assert_eq!(
        store.lock_hold("lock:refresh").map(|hold| hold.holds),
        Some(2)
    );
}
