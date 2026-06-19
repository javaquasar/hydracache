use super::{
    CacheInvalidation, CacheInvalidationFrame, CacheInvalidationMessage,
    CACHE_INVALIDATION_FRAME_VERSION,
};

#[test]
fn frame_version_compat_current_wire_version_round_trips() {
    let frame = CacheInvalidationFrame::new(CacheInvalidationMessage::new(
        "member-a",
        CacheInvalidation::tag("users"),
    ))
    .with_cluster_name("orders")
    .with_message_id(7);

    let encoded = frame.encode().unwrap();
    let decoded = CacheInvalidationFrame::decode(&encoded).unwrap();

    assert_eq!(decoded.version(), CACHE_INVALIDATION_FRAME_VERSION);
    assert_eq!(decoded.cluster_name(), Some("orders"));
    assert_eq!(decoded.message_id(), Some(7));
    assert_eq!(decoded.invalidation().tag_value(), Some("users"));
}

#[test]
fn frame_version_compat_future_wire_version_fails_closed() {
    let mut frame = CacheInvalidationFrame::new(CacheInvalidationMessage::new(
        "member-a",
        CacheInvalidation::flush(),
    ));
    frame.version = CACHE_INVALIDATION_FRAME_VERSION + 1;

    let encoded = frame.encode().unwrap();
    let error = CacheInvalidationFrame::decode(&encoded).unwrap_err();

    assert!(error
        .to_string()
        .contains("unsupported invalidation frame version"));
}
