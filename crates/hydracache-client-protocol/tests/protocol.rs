use std::fs;
use std::path::Path;

use hydracache_client_protocol::{
    ClientContext, ClientErrorCode, ClientErrorEnvelope, ClientFrame, ClientRequest,
    ClientRequestEnvelope, ClientResponse, ClientResponseEnvelope, ClientWireMessage,
    InvalidationEvent, Namespace, ReadConsistency, RegionId, RepairAction, StructuredKey,
    SubscriptionWatermarkTracker, VersionHandshake, Watermark,
};

fn ns() -> Namespace {
    Namespace::new("users").unwrap()
}

fn key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["user".to_owned(), id.to_owned()]).unwrap()
}

#[test]
fn protocol_version_handshake_picks_highest_common() {
    let client = VersionHandshake::new(1, 3);
    let server = VersionHandshake::new(1, 2);

    assert_eq!(client.negotiate(server).unwrap(), 2);
}

#[test]
fn protocol_out_of_window_version_is_refused_loud() {
    let client = VersionHandshake::new(2, 3);
    let server = VersionHandshake::new(1, 1);
    let error = client.negotiate(server).unwrap_err();

    assert_eq!(error.code, ClientErrorCode::IncompatibleVersion);
    assert!(!error.retryable);
}

#[test]
fn protocol_subscribe_invalidations_carries_b1_watermark() {
    let event = InvalidationEvent::new(ns(), key("42"), 7, 11);

    assert_eq!(event.watermark(), Watermark::new(7, 11));

    let mut tracker = SubscriptionWatermarkTracker::default();
    assert_eq!(tracker.on_event(&event), RepairAction::ClearPartition);
}

#[test]
fn protocol_old_client_new_server_compat() {
    let old_client = VersionHandshake::new(1, 1);
    let new_server = VersionHandshake::new(1, 2);

    assert_eq!(old_client.negotiate(new_server).unwrap(), 1);
}

#[test]
fn protocol_new_client_old_server_compat() {
    let new_client = VersionHandshake::new(1, 2);
    let old_server = VersionHandshake::new(1, 1);

    assert_eq!(new_client.negotiate(old_server).unwrap(), 1);
}

#[test]
fn protocol_golden_wire_fixtures_are_stable() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/client-v1");
    let fixture = fs::read_to_string(fixture_dir.join("handshake.hex")).unwrap();
    let bytes = decode_hex(&fixture);
    let frame = ClientFrame::decode(&bytes, 1024).unwrap();

    assert_eq!(frame.encode().unwrap().as_ref(), bytes.as_slice());
}

#[test]
fn protocol_malformed_or_truncated_frame_is_refused_not_panicked() {
    let error = ClientFrame::decode(&[0, 0, 0], 1024).unwrap_err();

    assert!(error.to_string().contains("truncated client frame"));
}

#[test]
fn protocol_stable_error_envelope_is_retryable_and_redacted() {
    let error = ClientErrorEnvelope::new(
        ClientErrorCode::BackendUnavailable,
        true,
        "backend unavailable; value=super-secret",
    )
    .with_retry_after_ms(25);

    assert!(error.retryable);
    assert_eq!(error.retry_after_ms, Some(25));
    assert!(error.message.contains("value=<redacted>"));
    assert!(!error.message.contains("super-secret"));
}

#[test]
fn protocol_batch_partial_failures_preserve_order_and_item_status() {
    let response = ClientResponseEnvelope::ok(
        "batch-1",
        ClientResponse::Batch {
            items: vec![
                hydracache_client_protocol::BatchItemStatus {
                    index: 0,
                    result: Ok(Some(vec![1])),
                },
                hydracache_client_protocol::BatchItemStatus {
                    index: 1,
                    result: Err(ClientErrorEnvelope::new(
                        ClientErrorCode::TooLarge,
                        false,
                        "too large",
                    )),
                },
            ],
        },
    );

    let ClientResponse::Batch { items } = response.result.unwrap() else {
        panic!("expected batch response");
    };
    assert_eq!(items[0].index, 0);
    assert_eq!(items[1].index, 1);
    assert!(items[1].result.is_err());
}

#[test]
fn protocol_deadline_and_idempotency_are_honored() {
    let envelope = ClientRequestEnvelope::new(
        "put-1",
        ClientRequest::Put {
            ns: ns(),
            key: key("42"),
            value: vec![1, 2, 3],
            ttl_ms: None,
            dimensions: vec!["tenant".to_owned(), "core".to_owned()],
        },
    )
    .with_deadline_ms(10)
    .with_idempotency_key("idem-1");

    assert!(!envelope.deadline_expired(9));
    assert!(envelope.deadline_expired(10));
    assert_eq!(envelope.idempotency_key.as_deref(), Some("idem-1"));
}

#[test]
fn protocol_session_context_preserves_remote_ryw_when_available() {
    let context = ClientContext {
        session_token: Some("session-token".to_owned()),
        read: Some(ReadConsistency::Session),
        ..ClientContext::default()
    };
    let envelope = ClientRequestEnvelope::new(
        "get-1",
        ClientRequest::Get {
            ns: ns(),
            key: key("42"),
        },
    )
    .with_context(context);

    assert_eq!(
        envelope.context.session_token.as_deref(),
        Some("session-token")
    );
    assert_eq!(envelope.context.read, Some(ReadConsistency::Session));
}

#[test]
fn protocol_region_scoped_subscription_streams_only_that_regions_applied_events() {
    let eu = RegionId::new("eu").unwrap();
    let us = RegionId::new("us").unwrap();
    let event = InvalidationEvent::new(ns(), key("42"), 1, 1).applied_in(eu.clone());

    assert!(event.should_deliver_to(Some(&eu)));
    assert!(!event.should_deliver_to(Some(&us)));
}

#[test]
fn protocol_region_filter_does_not_hide_cross_region_invalidation_affecting_subscriber() {
    let eu = RegionId::new("eu").unwrap();
    let us = RegionId::new("us").unwrap();
    let event = InvalidationEvent::new(ns(), key("42"), 1, 1)
        .applied_in(us)
        .affects_subscriber_view();

    assert!(event.should_deliver_to(Some(&eu)));
}

#[test]
fn protocol_include_value_is_residency_gated_and_degrades_to_invalidation() {
    let event = InvalidationEvent::new(ns(), key("42"), 1, 1)
        .with_value(vec![1, 2, 3])
        .residency_gated(false);

    assert!(event.value.is_none());
    assert!(event.residency_degraded);
}

#[test]
fn protocol_region_subscription_resume_and_gap_trigger_repair() {
    let mut tracker = SubscriptionWatermarkTracker::default();
    let first = InvalidationEvent::new(ns(), key("1"), 1, 1);
    let gap = InvalidationEvent::new(ns(), key("2"), 1, 3);

    assert_eq!(tracker.on_event(&first), RepairAction::ClearPartition);
    assert_eq!(
        tracker.on_event(&gap),
        RepairAction::InvalidateConservatively
    );
}

#[test]
fn protocol_wire_message_round_trips_inside_length_prefixed_frame() {
    let message = ClientWireMessage::Request(ClientRequestEnvelope::new(
        "get-1",
        ClientRequest::Get {
            ns: ns(),
            key: key("42"),
        },
    ));
    let frame = ClientFrame::from_message(&message).unwrap();
    let decoded = ClientFrame::decode(frame.encode().unwrap().as_ref(), 4096)
        .unwrap()
        .decode_message()
        .unwrap();

    assert_eq!(decoded, message);
}

#[test]
fn protocol_framing_adr_exists_and_compat_references_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let adr = root.join("docs/adr/0007-client-wire-framing.md");
    let compat = fs::read_to_string(root.join("docs/COMPAT.md")).unwrap();

    assert!(adr.exists());
    assert!(compat.contains("0007-client-wire-framing.md"));
    assert!(compat.contains("HydraCache external client protocol"));
}

fn decode_hex(input: &str) -> Vec<u8> {
    let compact = input
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    compact
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (from_hex(pair[0]) << 4) | from_hex(pair[1]))
        .collect()
}

fn from_hex(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => 10 + byte - b'a',
        b'A'..=b'F' => 10 + byte - b'A',
        _ => panic!("invalid hex byte {}", byte as char),
    }
}
