use hydracache::{
    ClusterEpoch, ConsistencyLevel, HybridLogicalClock, PartitionId, PartitionKey, ReadEscalation,
    SessionGuaranteeUnmet, SessionReadBudget, SessionWatermark, VersionStamp,
};

fn stamp(version: u64) -> VersionStamp {
    VersionStamp::new(
        version,
        ClusterEpoch::new(1),
        HybridLogicalClock::new(version, 0),
    )
}

fn key(partition: u32, region: &str) -> PartitionKey {
    PartitionKey::new(PartitionId::new(partition), region)
}

#[test]
fn session_ryw_write_region_a_read_region_b_sees_own_write() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.record_write(user_key.clone(), stamp(5));

    let decision = hydracache::serve_session_read(
        &mut watermark,
        user_key.clone(),
        stamp(5),
        SessionReadBudget::strict(),
    )
    .expect("replica satisfies the session write watermark");

    assert_eq!(decision, ReadEscalation::ServeLocal);
    assert!(watermark.covers(&user_key, stamp(5)));
}

#[test]
fn session_ryw_stale_replica_triggers_escalation_then_repair() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.record_write(user_key.clone(), stamp(5));

    assert_eq!(
        hydracache::resolve_session_read(
            &watermark,
            &user_key,
            stamp(4),
            SessionReadBudget::strict()
        ),
        ReadEscalation::TryHigherLevel(ConsistencyLevel::Quorum)
    );

    assert_eq!(
        hydracache::resolve_session_read(
            &watermark,
            &user_key,
            stamp(4),
            SessionReadBudget {
                allow_higher_level: false,
                allow_read_repair: true,
                wait_budget_millis: 0,
            },
        ),
        ReadEscalation::ReadRepair
    );
}

#[test]
fn session_ryw_unmet_within_budget_fails_loud_not_stale() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.record_write(user_key.clone(), stamp(5));

    let error = hydracache::serve_session_read(
        &mut watermark,
        user_key.clone(),
        stamp(4),
        SessionReadBudget::fail_fast(),
    )
    .expect_err("fail-fast sessions must not serve below their watermark");

    assert_eq!(
        error,
        SessionGuaranteeUnmet {
            key: user_key,
            required: stamp(5),
            candidate: stamp(4),
        }
    );
}

#[test]
fn session_ryw_concurrent_other_session_writes_do_not_break_ryw() {
    let mut watermark = SessionWatermark::new(8);
    watermark.record_write(key(42, "region-a"), stamp(5));

    let unrelated_key = key(7, "region-b");
    let decision = hydracache::serve_session_read(
        &mut watermark,
        unrelated_key.clone(),
        stamp(1),
        SessionReadBudget::fail_fast(),
    )
    .expect("unrelated writes do not create a session floor for this key");

    assert_eq!(decision, ReadEscalation::ServeLocal);
    assert!(watermark.covers(&unrelated_key, stamp(1)));
}

#[test]
fn session_ryw_wait_budget_never_serves_stale_without_recheck() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.record_write(user_key.clone(), stamp(5));

    let decision = hydracache::serve_session_read(
        &mut watermark,
        user_key.clone(),
        stamp(4),
        SessionReadBudget {
            allow_higher_level: false,
            allow_read_repair: false,
            wait_budget_millis: 10,
        },
    )
    .expect("wait budget is an escalation path, not a stale serve");

    assert_eq!(decision, ReadEscalation::WaitThenFail);
    assert_eq!(watermark.highest_seen(&user_key), Some(stamp(5)));
}

#[test]
#[ignore = "fault-injection scenario for a networked multi-region runner"]
fn session_ryw_holds_across_region_failover() {}
