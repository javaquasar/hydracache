use hydracache_server::{UpgradeError, UpgradePhase, UpgradePlan, UpgradeStrategy};

#[test]
fn graceful_upgrade_drops_no_inflight_request() {
    let mut upgrade = UpgradePlan::new(2, "node-a")
        .with_strategy(UpgradeStrategy::InheritedSocket)
        .prepare()
        .unwrap();

    assert!(upgrade.record_request());
    assert!(upgrade.record_request());
    upgrade.mark_new_ready();
    upgrade.start_draining_old().unwrap();

    assert!(!upgrade.old_accepting());
    assert!(!upgrade.record_request());
    assert_eq!(upgrade.in_flight(), 2);

    upgrade.finish_request();
    upgrade.finish_request();
    let report = upgrade.complete().unwrap();

    assert_eq!(report.phase, UpgradePhase::Complete);
    assert_eq!(report.completed_requests, 2);
    assert_eq!(report.dropped_requests, 0);
}

#[test]
fn graceful_upgrade_refuses_to_drain_before_replacement_ready() {
    let mut upgrade = UpgradePlan::new(3, "node-a").prepare().unwrap();

    assert_eq!(
        upgrade.start_draining_old(),
        Err(UpgradeError::ReplacementNotReady)
    );
    assert!(upgrade.old_accepting());
    assert_eq!(upgrade.phase(), UpgradePhase::Prepared);
}

#[test]
fn graceful_upgrade_membership_stays_stable_across_handoff() {
    let mut upgrade = UpgradePlan::new(4, "node-a")
        .with_strategy(UpgradeStrategy::ReusePort)
        .prepare()
        .unwrap();

    assert!(upgrade.membership_stable());
    upgrade.mark_new_ready();
    upgrade.start_draining_old().unwrap();
    let report = upgrade.complete().unwrap();

    assert_eq!(report.member_id, "node-a");
    assert_eq!(report.strategy, UpgradeStrategy::ReusePort);
}

#[test]
fn graceful_upgrade_requires_explicit_generation_and_member_id() {
    assert_eq!(
        UpgradePlan::new(0, "node-a").prepare(),
        Err(UpgradeError::InvalidGeneration)
    );
    assert_eq!(
        UpgradePlan::new(1, " ").prepare(),
        Err(UpgradeError::MissingMemberId)
    );
}
