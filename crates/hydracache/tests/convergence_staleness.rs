use hydracache::{
    converge_replicated_values, resolve_session_read_mode, within_staleness_bound, ClusterEpoch,
    HigherVersionWins, HybridLogicalClock, PartitionId, PartitionKey, ReplicatedValueRecord,
    SessionReadMode, SessionWatermark, StalenessBound, StalenessDecision,
    StalenessEscalationReason, VersionStamp,
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

fn record(version: u64, value: u8) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        PartitionId::new(7),
        version,
        ClusterEpoch::new(1),
        vec![value],
    )
}

#[test]
fn convergence_staleness_replicas_converge_to_one_value_without_new_writes() {
    let policy = HigherVersionWins;
    let first_order = vec![record(1, b'a'), record(3, b'c'), record(2, b'b')];
    let second_order = vec![record(2, b'b'), record(1, b'a'), record(3, b'c')];

    let first = converge_replicated_values(&policy, first_order).expect("winner");
    let second = converge_replicated_values(&policy, second_order).expect("winner");

    assert_eq!(first, second);
    assert_eq!(first.version, 3);
}

#[test]
fn convergence_staleness_bounded_staleness_serves_fast_within_bound() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.observe(user_key.clone(), stamp(10));

    let decision = resolve_session_read_mode(
        &watermark,
        &user_key,
        Some(stamp(8)),
        stamp(8),
        SessionReadMode::BoundedStaleness {
            max: StalenessBound::versions(2),
        },
    );

    assert_eq!(
        decision,
        StalenessDecision::ServeFast {
            observed_version_lag: 2
        }
    );
    assert!(within_staleness_bound(
        &watermark,
        &user_key,
        Some(stamp(8)),
        stamp(8),
        SessionReadMode::BoundedStaleness {
            max: StalenessBound::versions(2),
        },
    ));
}

#[test]
fn convergence_staleness_bounded_staleness_never_serves_below_causal_floor() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.observe(user_key.clone(), stamp(10));

    assert_eq!(
        resolve_session_read_mode(
            &watermark,
            &user_key,
            Some(stamp(9)),
            stamp(8),
            SessionReadMode::BoundedStaleness {
                max: StalenessBound::versions(2),
            },
        ),
        StalenessDecision::Escalate {
            reason: StalenessEscalationReason::BelowCausalFloor,
            observed_version_lag: 2,
        }
    );
}

#[test]
fn convergence_staleness_beyond_bound_escalates() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.observe(user_key.clone(), stamp(10));

    assert_eq!(
        resolve_session_read_mode(
            &watermark,
            &user_key,
            Some(stamp(7)),
            stamp(7),
            SessionReadMode::BoundedStaleness {
                max: StalenessBound::versions(2),
            },
        ),
        StalenessDecision::Escalate {
            reason: StalenessEscalationReason::BeyondBound,
            observed_version_lag: 3,
        }
    );
}

#[test]
fn convergence_staleness_causal_mode_requires_full_session_watermark() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.observe(user_key.clone(), stamp(10));

    assert_eq!(
        resolve_session_read_mode(
            &watermark,
            &user_key,
            Some(stamp(9)),
            stamp(9),
            SessionReadMode::Causal,
        ),
        StalenessDecision::Escalate {
            reason: StalenessEscalationReason::BelowSessionWatermark,
            observed_version_lag: 1,
        }
    );
}
