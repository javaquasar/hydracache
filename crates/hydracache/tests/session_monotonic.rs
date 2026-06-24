use hydracache::{
    apply_monotonic_read, apply_monotonic_write, resolve_monotonic_read, resolve_monotonic_write,
    ClusterEpoch, HybridLogicalClock, MonotonicReadDecision, MonotonicWriteDecision, PartitionId,
    PartitionKey, SessionSequence, SessionWatermark, SessionWriteStamp, VersionStamp,
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

fn write(sequence: u64, version: u64) -> SessionWriteStamp {
    SessionWriteStamp::new(
        "session-a",
        SessionSequence::new(sequence),
        key(42, "region-a"),
        stamp(version),
    )
}

#[test]
fn session_monotonic_reads_never_go_backwards() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.observe(user_key.clone(), stamp(10));

    assert_eq!(
        resolve_monotonic_read(&watermark, &user_key, stamp(9)),
        MonotonicReadDecision::PreventStale {
            required: stamp(10),
            candidate: stamp(9),
        }
    );

    let error = apply_monotonic_read(&mut watermark, user_key.clone(), stamp(9))
        .expect_err("stale candidate must not move the session backwards");
    assert_eq!(error.required, stamp(10));
    assert_eq!(watermark.highest_seen(&user_key), Some(stamp(10)));
}

#[test]
fn session_monotonic_reads_advance_watermark_on_newer_candidate() {
    let mut watermark = SessionWatermark::new(8);
    let user_key = key(42, "region-a");
    watermark.observe(user_key.clone(), stamp(10));

    apply_monotonic_read(&mut watermark, user_key.clone(), stamp(11))
        .expect("newer candidate advances the session watermark");

    assert_eq!(watermark.highest_seen(&user_key), Some(stamp(11)));
}

#[test]
fn session_monotonic_writes_reject_duplicate_or_older_sequence() {
    let accepted = write(7, 100);
    let duplicate = write(7, 101);
    let older = write(6, 102);

    assert_eq!(
        resolve_monotonic_write(Some(&accepted), &duplicate),
        MonotonicWriteDecision::PreventReorder {
            accepted: SessionSequence::new(7),
            incoming: SessionSequence::new(7),
        }
    );
    assert_eq!(
        apply_monotonic_write(Some(&accepted), older)
            .expect_err("older session sequence must be rejected")
            .decision,
        MonotonicWriteDecision::PreventReorder {
            accepted: SessionSequence::new(7),
            incoming: SessionSequence::new(6),
        }
    );
}

#[test]
fn session_monotonic_writes_reject_newer_sequence_with_lower_stamp() {
    let accepted = write(7, 100);
    let stale_stamp = write(8, 99);

    assert_eq!(
        resolve_monotonic_write(Some(&accepted), &stale_stamp),
        MonotonicWriteDecision::PreventStaleStamp {
            accepted: stamp(100),
            incoming: stamp(99),
        }
    );
}

#[test]
fn session_monotonic_writes_allow_other_sessions_or_partitions_independently() {
    let accepted = write(7, 100);
    let other_session = SessionWriteStamp::new(
        "session-b",
        SessionSequence::new(1),
        key(42, "region-a"),
        stamp(1),
    );
    let other_partition = SessionWriteStamp::new(
        "session-a",
        SessionSequence::new(1),
        key(7, "region-b"),
        stamp(1),
    );

    assert_eq!(
        resolve_monotonic_write(Some(&accepted), &other_session),
        MonotonicWriteDecision::Apply
    );
    assert_eq!(
        resolve_monotonic_write(Some(&accepted), &other_partition),
        MonotonicWriteDecision::Apply
    );
}

#[test]
fn session_monotonic_sequence_next_saturates_at_max() {
    assert_eq!(
        SessionSequence::new(u64::MAX).next(),
        SessionSequence::new(u64::MAX)
    );
}
