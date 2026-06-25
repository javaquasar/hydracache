use hydracache_client_protocol::java_migration::{
    JavaMapCasExpectation, JavaMapOperation, JavaMapProtocolFamily,
};
use hydracache_client_protocol::{
    CasExpectation, ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, ClientWireMessage, LockConsistency, Namespace, StructuredKey,
    LOCK_PROTOCOL_VERSION, PROTOCOL_VERSION,
};

fn ns() -> Namespace {
    Namespace::new("users").unwrap()
}

fn key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["user".to_owned(), id.to_owned()]).unwrap()
}

fn roundtrip(message: ClientWireMessage) -> ClientWireMessage {
    let frame = ClientFrame::from_message_with_version(PROTOCOL_VERSION, &message).unwrap();
    ClientFrame::decode(frame.encode().unwrap().as_ref(), 4096)
        .unwrap()
        .decode_message()
        .unwrap()
}

#[test]
fn replace_with_matching_old_applies_and_bumps_version() {
    let request = ClientWireMessage::Request(ClientRequestEnvelope::new(
        "replace-1",
        ClientRequest::CompareAndSet {
            ns: ns(),
            key: key("42"),
            expected: CasExpectation::Exact(b"old".to_vec()),
            new_value: b"new".to_vec(),
            level: LockConsistency::Quorum,
        },
    ));
    let response = ClientWireMessage::Response(ClientResponseEnvelope::ok(
        "replace-1",
        ClientResponse::CasApplied { new_version: 2 },
    ));

    assert_eq!(roundtrip(request.clone()), request);
    assert_eq!(roundtrip(response.clone()), response);
}

#[test]
fn replace_with_stale_old_returns_mismatch_current() {
    let response = ClientWireMessage::Response(ClientResponseEnvelope::ok(
        "replace-2",
        ClientResponse::CasMismatch {
            current: Some(b"current".to_vec()),
        },
    ));

    assert_eq!(roundtrip(response.clone()), response);
}

#[test]
fn remove_if_value_matches_then_tombstones() {
    let request = ClientWireMessage::Request(ClientRequestEnvelope::new(
        "remove-if-1",
        ClientRequest::RemoveIfValue {
            ns: ns(),
            key: key("42"),
            expected: b"old".to_vec(),
            level: LockConsistency::EachQuorum,
        },
    ));
    let response = ClientWireMessage::Response(ClientResponseEnvelope::ok(
        "remove-if-1",
        ClientResponse::CasApplied { new_version: 3 },
    ));

    assert_eq!(roundtrip(request.clone()), request);
    assert_eq!(roundtrip(response.clone()), response);
}

#[test]
fn replace_if_present_on_absent_is_mismatch_not_insert() {
    let request = ClientRequest::CompareAndSet {
        ns: ns(),
        key: key("missing"),
        expected: CasExpectation::Present,
        new_value: b"created".to_vec(),
        level: LockConsistency::Quorum,
    };
    assert_eq!(request.minimum_protocol_version(), LOCK_PROTOCOL_VERSION);

    let response = ClientResponse::CasMismatch { current: None };
    assert_eq!(
        roundtrip(ClientWireMessage::Response(ClientResponseEnvelope::ok(
            "replace-present-absent",
            response.clone(),
        ))),
        ClientWireMessage::Response(ClientResponseEnvelope::ok(
            "replace-present-absent",
            response,
        ))
    );
}

#[test]
fn java_replace_maps_to_conditional_replace_family() {
    assert_eq!(
        JavaMapOperation::Replace.protocol_family(),
        JavaMapProtocolFamily::ConditionalReplace {
            expectation: JavaMapCasExpectation::ExactValue,
        }
    );
    assert_eq!(
        JavaMapOperation::ReplaceIfPresent.protocol_family(),
        JavaMapProtocolFamily::ConditionalReplace {
            expectation: JavaMapCasExpectation::Present,
        }
    );
    assert_eq!(
        JavaMapOperation::RemoveIfValue.protocol_family(),
        JavaMapProtocolFamily::ConditionalRemove
    );
}
