use hydracache_cluster_testkit::value_plane_bulk_control_075::{
    BulkControl, BulkControlError, ControlledItemOutcome,
};

#[test]
fn deadlines_and_cancellation_only_finish_pending_items_in_stable_order() {
    let mut bulk = BulkControl::new(vec![2, 1, 2], 5, 3).unwrap();
    assert_eq!(bulk.pending_groups()[&2], vec![0, 2]);
    bulk.complete(0).unwrap();
    assert_eq!(bulk.advance_to(4).unwrap(), 0);
    assert_eq!(bulk.advance_to(5).unwrap(), 2);
    let receipt = bulk.receipt();
    assert_eq!(receipt[0].outcome, ControlledItemOutcome::Completed);
    assert_eq!(receipt[1].outcome, ControlledItemOutcome::TimedOut);
    assert_eq!(receipt[2].outcome, ControlledItemOutcome::TimedOut);
    assert_eq!(bulk.cancel(), 0);
}

#[test]
fn owner_changes_consume_bounded_retry_budget_for_pending_only() {
    let mut bulk = BulkControl::new(vec![1, 1], 10, 2).unwrap();
    bulk.complete(0).unwrap();
    assert_eq!(bulk.owner_changed(1), 1);
    assert_eq!(bulk.owner_changed(1), 0);
    let receipt = bulk.receipt();
    assert_eq!(receipt[0].attempts, 1);
    assert_eq!(receipt[1].attempts, 2);
    assert_eq!(receipt[1].outcome, ControlledItemOutcome::RetryExhausted);
    assert_eq!(bulk.complete(1), Err(BulkControlError::AlreadyTerminal));
}

#[test]
fn invalid_bounds_indices_and_time_regression_fail_loud() {
    assert!(BulkControl::new(Vec::new(), 1, 1).is_err());
    assert!(BulkControl::new(vec![1], 0, 1).is_err());
    let mut bulk = BulkControl::new(vec![1], 3, 1).unwrap();
    assert_eq!(bulk.complete(2), Err(BulkControlError::UnknownInput));
    bulk.advance_to(2).unwrap();
    assert_eq!(bulk.advance_to(1), Err(BulkControlError::TimeRegression));
    assert_eq!(bulk.cancel(), 1);
    assert_eq!(bulk.receipt()[0].outcome, ControlledItemOutcome::Cancelled);
}
