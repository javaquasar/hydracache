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
fn expired_lease_can_be_stolen_and_fence_advances() {
    let mut store = store();
    let first_owner = owner("session-a", 1);
    let second_owner = owner("session-b", 1);
    let first = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            first_owner,
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first owner acquires");

    let second = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            second_owner,
            lease(10),
            at(10),
        )
        .unwrap()
        .expect("expired lease can be stolen");

    assert!(second > first);
    assert_eq!(store.metrics().lock_lease_expired_total, 1);
}

#[test]
fn stale_holder_release_after_expiry_is_rejected() {
    let mut store = store();
    let first_owner = owner("session-a", 1);
    let first = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            first_owner.clone(),
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first owner acquires");
    assert_eq!(store.expire_due(at(10)), 1);

    let error = store
        .release_lock("lock:refresh", &first_owner, first)
        .expect_err("expired holder must not release successfully");

    assert!(matches!(error, ConditionalError::StaleFenceToken { .. }));
    assert_eq!(store.metrics().lock_stale_token_rejected_total, 1);
}

#[test]
fn heartbeat_renew_keeps_ownership() {
    let mut store = store();
    let first_owner = owner("session-a", 1);
    let second_owner = owner("session-b", 1);
    let first = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            first_owner.clone(),
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first owner acquires");

    store
        .renew_lease("lock:refresh", &first_owner, first, at(30))
        .unwrap();
    assert_eq!(store.expire_due(at(15)), 0);
    assert_eq!(
        store
            .try_acquire_lock(
                "lock:refresh",
                ConsistencyLevel::Quorum,
                second_owner,
                lease(10),
                at(15),
            )
            .unwrap(),
        None
    );
    assert_eq!(store.metrics().lock_lease_renewed_total, 1);
}

#[test]
fn session_loss_releases_all_its_locks_and_advances_fence() {
    let mut store = store();
    let first_owner = owner("session-a", 1);
    let first = store
        .try_acquire_lock(
            "lock:a",
            ConsistencyLevel::Quorum,
            first_owner.clone(),
            lease(100),
            at(0),
        )
        .unwrap()
        .expect("first lock");
    store
        .try_acquire_lock(
            "lock:b",
            ConsistencyLevel::Quorum,
            first_owner.clone(),
            lease(100),
            at(0),
        )
        .unwrap()
        .expect("second lock");

    assert_eq!(store.expire_lost_sessions(at(20), lease(10)), 2);
    assert!(matches!(
        store.validate_fence_token("lock:a", first),
        Err(ConditionalError::StaleFenceToken { .. })
    ));

    let second = store
        .try_acquire_lock(
            "lock:a",
            ConsistencyLevel::Quorum,
            owner("session-b", 1),
            lease(10),
            at(21),
        )
        .unwrap()
        .expect("new owner after session loss");
    assert!(second > first);
    assert_eq!(store.metrics().lock_lease_expired_total, 2);
}

#[test]
fn release_by_non_owner_is_rejected_and_counted() {
    let mut store = store();
    let first_owner = owner("session-a", 1);
    let token = store
        .try_acquire_lock(
            "lock:refresh",
            ConsistencyLevel::Quorum,
            first_owner,
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("first owner acquires");

    let error = store
        .release_lock("lock:refresh", &owner("session-b", 1), token)
        .expect_err("non-owner release must fail loud");

    assert!(matches!(error, ConditionalError::NotOwner { .. }));
    assert_eq!(store.metrics().lock_not_owner_rejected_total, 1);
}

#[test]
fn fence_assigned_only_on_apply_not_propose() {
    #[derive(Clone)]
    struct ProposedAcquire {
        key: &'static str,
        owner: LockOwner,
    }

    let mut store = store();
    let proposed = ProposedAcquire {
        key: "lock:refresh",
        owner: owner("session-a", 1),
    };
    let before_apply = proposed.clone();

    assert!(store.lock_hold(before_apply.key).is_none());

    let applied = store
        .try_acquire_lock(
            proposed.key,
            ConsistencyLevel::Quorum,
            proposed.owner,
            lease(10),
            at(0),
        )
        .unwrap()
        .expect("apply assigns fence");

    assert_eq!(applied.value(), 1);
    assert_eq!(
        store.lock_hold("lock:refresh").map(|hold| hold.fence),
        Some(applied)
    );
}
