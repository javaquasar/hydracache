use hydracache::{
    rebuild_expired_sessionless, recover_session_after_failover, validate_session_lifecycle,
    ClusterEpoch, HybridLogicalClock, PartitionId, PartitionKey, SessionFailoverAction, SessionId,
    SessionLifecycleDecision, SessionRequest, SessionToken, SessionTokenError, SessionTtl,
    SessionWatermark, VersionStamp,
};

#[test]
fn session_ttl_boundaries_accept_then_expire_without_wrapping() {
    assert_eq!(SessionTtl::from_millis(0).as_millis(), 1);
    assert_eq!(SessionTtl::from_secs(2).as_millis(), 2_000);
    assert_eq!(SessionTtl::from_secs(u64::MAX).as_millis(), u64::MAX);
    assert_eq!(SessionTtl::default().as_millis(), 900_000);
    assert!(!SessionTtl::from_millis(10).is_expired(100, 110));
    assert!(SessionTtl::from_millis(10).is_expired(100, 111));
    assert!(!SessionTtl::from_millis(10).is_expired(100, 50));
}

#[test]
fn session_validation_and_rebuild_distinguish_expiry_from_security_errors() {
    let secret = b"session-secret";
    let session = SessionId::new("session-a");
    let token = SessionToken::issue(session.clone(), SessionWatermark::new(4), 7, 100, secret);

    assert!(matches!(
        validate_session_lifecycle(
            &token,
            &session,
            secret,
            7,
            SessionTtl::from_millis(10),
            110,
        )
        .unwrap(),
        SessionLifecycleDecision::Accepted(SessionRequest::Session(_))
    ));
    assert_eq!(
        validate_session_lifecycle(
            &token,
            &session,
            secret,
            7,
            SessionTtl::from_millis(10),
            111,
        ),
        Err(SessionTokenError::Expired)
    );
    assert_eq!(
        rebuild_expired_sessionless(SessionTokenError::Expired),
        Ok(SessionRequest::Sessionless)
    );
    assert_eq!(
        rebuild_expired_sessionless(SessionTokenError::Replayed),
        Err(SessionTokenError::Replayed)
    );
}

#[test]
fn failover_recovery_preserves_empty_repairs_nonempty_or_rebuilds_loudly() {
    let empty = SessionWatermark::new(4);
    let preserved = recover_session_after_failover(&empty, "eu", false);
    assert_eq!(preserved.action, SessionFailoverAction::PreserveEmpty);
    assert!(preserved.guarantees_preserved);
    assert_eq!(preserved.watermark_entries, 0);

    let mut watermark = SessionWatermark::new(4);
    watermark.observe(
        PartitionKey::new(PartitionId::new(1), "us"),
        VersionStamp::new(7, ClusterEpoch::new(2), HybridLogicalClock::new(10, 0)),
    );
    let repaired = recover_session_after_failover(&watermark, "eu", true);
    assert_eq!(repaired.action, SessionFailoverAction::RepairToWatermark);
    assert!(repaired.guarantees_preserved);
    assert_eq!(repaired.watermark_entries, 1);

    let rebuilt = recover_session_after_failover(&watermark, "eu", false);
    assert_eq!(rebuilt.action, SessionFailoverAction::RebuildSessionless);
    assert!(!rebuilt.guarantees_preserved);
    assert_eq!(rebuilt.promoted_region.as_str(), "eu");
}
