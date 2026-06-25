use hydracache_client_protocol::{
    ClientErrorCode, ClientErrorEnvelope, ClientFrame, ClientRequest, ClientRequestEnvelope,
    ClientResponse, ClientResponseEnvelope, ClientWireMessage, LockConsistency, Namespace,
    StructuredKey, LOCK_PROTOCOL_VERSION, PROTOCOL_VERSION,
};

fn ns() -> Namespace {
    Namespace::new("locks").unwrap()
}

fn key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["lock".to_owned(), id.to_owned()]).unwrap()
}

#[test]
fn lock_request_response_roundtrips() {
    let request = ClientWireMessage::Request(ClientRequestEnvelope::new(
        "try-lock-1",
        ClientRequest::TryLock {
            ns: ns(),
            key: key("user:42"),
            lease_ms: 5_000,
            wait_ms: 0,
            level: LockConsistency::Quorum,
        },
    ));
    let response = ClientWireMessage::Response(ClientResponseEnvelope::ok(
        "try-lock-1",
        ClientResponse::LockAcquired { fence: 42 },
    ));

    for message in [request, response] {
        let frame = ClientFrame::from_message_with_version(PROTOCOL_VERSION, &message).unwrap();
        let decoded = ClientFrame::decode(frame.encode().unwrap().as_ref(), 4096)
            .unwrap()
            .decode_message()
            .unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn lock_op_requires_negotiated_v2_on_request_envelope() {
    let mut envelope = ClientRequestEnvelope::new(
        "try-lock-v1",
        ClientRequest::TryLock {
            ns: ns(),
            key: key("user:42"),
            lease_ms: 1_000,
            wait_ms: 0,
            level: LockConsistency::Quorum,
        },
    );
    envelope.protocol_version = 1;

    let error = envelope.validate_protocol().unwrap_err();

    assert_eq!(error.code, ClientErrorCode::IncompatibleVersion);
    assert_eq!(
        envelope.request.minimum_protocol_version(),
        LOCK_PROTOCOL_VERSION
    );
}

#[test]
fn unknown_future_lock_variant_refuses_loud() {
    let frame = ClientFrame::with_version(PROTOCOL_VERSION, vec![0xff, 0xff, 0xff]);
    let error = frame.decode_message().unwrap_err();

    assert!(error.to_string().contains("client protocol codec error"));
}

#[test]
fn weak_level_lock_returns_weakconsistency_envelope() {
    let request = ClientRequest::TryLock {
        ns: ns(),
        key: key("user:42"),
        lease_ms: 1_000,
        wait_ms: 0,
        level: LockConsistency::One,
    };
    assert_eq!(request.minimum_protocol_version(), LOCK_PROTOCOL_VERSION);

    let envelope = ClientErrorEnvelope::new(
        ClientErrorCode::Conflict,
        false,
        "conditional writes require Quorum/EachQuorum/All, got One",
    );

    assert_eq!(envelope.code, ClientErrorCode::Conflict);
    assert!(envelope.message.contains("Quorum/EachQuorum/All"));
}
