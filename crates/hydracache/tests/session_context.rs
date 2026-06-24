use hydracache::{
    ClusterEpoch, HybridLogicalClock, PartitionId, PartitionKey, SessionContextMetrics, SessionId,
    SessionRequest, SessionToken, SessionTokenError, SessionWatermark, VersionStamp,
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
fn session_context_watermark_observe_and_covers_roundtrip() {
    let mut watermark = SessionWatermark::new(4);
    let key = key(7, "eu");

    watermark.observe(key.clone(), stamp(5));

    assert!(watermark.covers(&key, stamp(5)));
    assert!(watermark.covers(&key, stamp(4)));
    assert!(!watermark.covers(&key, stamp(6)));
}

#[test]
fn session_context_watermark_is_bounded_and_coarsens_when_full() {
    let mut watermark = SessionWatermark::new(2);

    watermark.observe(key(1, "eu"), stamp(1));
    watermark.observe(key(2, "eu"), stamp(2));
    watermark.observe(key(3, "us"), stamp(3));

    assert_eq!(watermark.len(), 2);
    assert_eq!(watermark.cap(), 2);
    assert_eq!(watermark.coarsened_total(), 1);
    assert!(!watermark.covers(&key(1, "eu"), stamp(1)));
    assert!(watermark.covers(&key(3, "us"), stamp(3)));
}

#[test]
fn session_context_forged_or_replayed_token_is_rejected() {
    let secret = b"test-secret";
    let session = SessionId::new("session-a");
    let token = SessionToken::issue(session.clone(), SessionWatermark::new(4), 7, 10, secret);

    assert!(token.verify(&session, secret, 7).is_ok());
    assert_eq!(
        token.clone().forged().verify(&session, secret, 7),
        Err(SessionTokenError::Forged)
    );
    assert_eq!(
        token.verify(&session, secret, 8),
        Err(SessionTokenError::Replayed)
    );
    assert_eq!(
        token.verify(&SessionId::new("session-b"), secret, 7),
        Err(SessionTokenError::WrongSession)
    );
}

#[test]
fn session_context_sessionless_path_is_unchanged() {
    let request = SessionRequest::Sessionless;

    assert!(request.is_sessionless());
}

#[test]
fn session_context_metrics_reflect_watermark_shape() {
    let mut watermark = SessionWatermark::new(1);
    watermark.observe(key(1, "eu"), stamp(1));
    watermark.observe(key(2, "us"), stamp(2));

    let metrics = SessionContextMetrics::from(&watermark);

    assert_eq!(metrics.session_watermark_entries, 1);
    assert_eq!(metrics.session_watermark_coarsened_total, 1);
}
