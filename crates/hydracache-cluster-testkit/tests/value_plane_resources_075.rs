use hydracache_cluster_testkit::value_plane_resources_075::{
    ResourceError, ResourceKind, ResourceLedger,
};

#[test]
fn timeout_cancel_and_crash_cleanup_release_every_owner() {
    let mut ledger = ResourceLedger::new(20).unwrap();
    let first = ledger.reserve(7, ResourceKind::Bulk, 4).unwrap();
    ledger.reserve(7, ResourceKind::Staging, 5).unwrap();
    ledger.reserve(8, ResourceKind::Listener, 3).unwrap();
    assert_eq!(ledger.units_by_kind(ResourceKind::Staging), 5);
    ledger.release(first).unwrap();
    assert_eq!(ledger.cleanup_request(7), 5);
    assert_eq!(ledger.cleanup_request(8), 3);
    assert!(ledger.is_clean());
}

#[test]
fn bounds_invalid_reservations_and_double_release_fail_loud() {
    assert!(ResourceLedger::new(0).is_err());
    let mut ledger = ResourceLedger::new(2).unwrap();
    assert_eq!(
        ledger.reserve(0, ResourceKind::Dedup, 1),
        Err(ResourceError::InvalidReservation)
    );
    let id = ledger.reserve(1, ResourceKind::Quota, 2).unwrap();
    assert_eq!(
        ledger.reserve(2, ResourceKind::Bulk, 1),
        Err(ResourceError::Capacity)
    );
    ledger.release(id).unwrap();
    assert_eq!(ledger.release(id), Err(ResourceError::UnknownReservation));
    assert_eq!(ledger.used_units(), 0);
}
