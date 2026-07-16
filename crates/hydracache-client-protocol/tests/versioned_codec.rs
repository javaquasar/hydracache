use hydracache_client_protocol::{
    BatchPutEntry, CasExpectation, ClientErrorCode, ClientErrorEnvelope, ClientFrame,
    ClientProtocolError, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, ClientWireMessage, CompareValueExpireMode, ConditionalPutCondition,
    EntryEventProjection, InvalidationEvent, LockConsistency, Namespace, RegionId, StructuredKey,
    TtlState, VersionHandshake, Watermark,
};

fn ns() -> Namespace {
    Namespace::new("legacy").unwrap()
}

fn key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["key".to_owned(), id.to_owned()]).unwrap()
}

fn v1_requests() -> Vec<ClientRequest> {
    vec![
        ClientRequest::Get {
            ns: ns(),
            key: key("get"),
        },
        ClientRequest::Put {
            ns: ns(),
            key: key("put"),
            value: b"value".to_vec(),
            ttl_ms: None,
            dimensions: vec!["region".to_owned()],
        },
        ClientRequest::Invalidate {
            ns: ns(),
            key: key("invalidate"),
        },
        ClientRequest::BatchGet {
            ns: ns(),
            keys: vec![key("batch-get")],
        },
        ClientRequest::BatchPut {
            ns: ns(),
            entries: vec![BatchPutEntry {
                key: key("batch-put"),
                value: b"batch".to_vec(),
            }],
        },
        ClientRequest::EvictRegion { ns: ns() },
        ClientRequest::SubscribeInvalidations {
            ns: ns(),
            region: Some(RegionId::new("region-a").unwrap()),
            from: Some(Watermark::new(1, 2)),
            include_value: true,
        },
    ]
}

fn post_v1_requests() -> Vec<ClientRequest> {
    vec![
        ClientRequest::Put {
            ns: ns(),
            key: key("put-ttl"),
            value: b"value".to_vec(),
            ttl_ms: Some(500),
            dimensions: vec![],
        },
        ClientRequest::Expire {
            ns: ns(),
            key: key("expire"),
            ttl_ms: 500,
        },
        ClientRequest::Persist {
            ns: ns(),
            key: key("persist"),
        },
        ClientRequest::GetTtl {
            ns: ns(),
            key: key("ttl"),
        },
        ClientRequest::ConditionalPut {
            ns: ns(),
            key: key("conditional-put"),
            value: b"owner".to_vec(),
            ttl_ms: Some(500),
            condition: ConditionalPutCondition::IfAbsent,
        },
        ClientRequest::CompareValueAndInvalidate {
            ns: ns(),
            key: key("compare-invalidate"),
            expected_value: b"owner".to_vec(),
        },
        ClientRequest::CompareValueAndExpire {
            ns: ns(),
            key: key("compare-expire"),
            expected_value: b"owner".to_vec(),
            ttl_ms: 500,
            mode: CompareValueExpireMode::Replace,
        },
        ClientRequest::SubscribeEntryEvents {
            ns: ns(),
            region: Some(RegionId::new("region-a").unwrap()),
            from: Some(Watermark::new(2, 3)),
            include_value: false,
            projection: EntryEventProjection::IMapEntryEvent,
        },
        ClientRequest::TryLock {
            ns: ns(),
            key: key("try-lock"),
            lease_ms: 5_000,
            wait_ms: 25,
            level: LockConsistency::Quorum,
        },
        ClientRequest::Unlock {
            ns: ns(),
            key: key("unlock"),
            fence: 7,
        },
        ClientRequest::RenewLockLease {
            ns: ns(),
            key: key("renew"),
            fence: 8,
            lease_ms: 4_000,
        },
        ClientRequest::ForceUnlock {
            ns: ns(),
            key: key("force"),
        },
        ClientRequest::GetLockOwnership {
            ns: ns(),
            key: key("owner"),
        },
        ClientRequest::CompareAndSet {
            ns: ns(),
            key: key("cas"),
            expected: CasExpectation::Present,
            new_value: b"new".to_vec(),
            level: LockConsistency::EachQuorum,
        },
        ClientRequest::RemoveIfValue {
            ns: ns(),
            key: key("remove"),
            expected: b"old".to_vec(),
            level: LockConsistency::All,
        },
    ]
}

fn v1_responses() -> Vec<ClientResponse> {
    vec![
        ClientResponse::Value {
            value: Some(b"value".to_vec()),
        },
        ClientResponse::Stored,
        ClientResponse::Invalidated,
        ClientResponse::Batch { items: Vec::new() },
        ClientResponse::Evicted,
        ClientResponse::Subscribed {
            from: Some(Watermark::new(3, 4)),
        },
    ]
}

fn post_v1_responses() -> Vec<ClientResponse> {
    vec![
        ClientResponse::Expiry { applied: true },
        ClientResponse::Ttl {
            state: TtlState::ExpiresIn { ttl_ms: 500 },
        },
        ClientResponse::ConditionalStored { stored: true },
        ClientResponse::CompareValueApplied { applied: true },
        ClientResponse::LockAcquired { fence: 9 },
        ClientResponse::LockBusy,
        ClientResponse::LockReleased,
        ClientResponse::LockLeaseRenewed,
        ClientResponse::LockOwnership {
            fence: Some(10),
            locked: true,
        },
        ClientResponse::CasApplied { new_version: 11 },
        ClientResponse::CasMismatch {
            current: Some(b"current".to_vec()),
        },
    ]
}

fn roundtrip(version: u16, message: ClientWireMessage) {
    let encoded = ClientFrame::from_message_with_version(version, &message)
        .unwrap()
        .encode()
        .unwrap();
    let decoded = ClientFrame::decode(&encoded, 1024 * 1024)
        .unwrap()
        .decode_message()
        .unwrap();
    assert_eq!(decoded, message);
}

fn v2_requests() -> Vec<ClientRequest> {
    vec![
        ClientRequest::Get {
            ns: ns(),
            key: key("get"),
        },
        ClientRequest::Put {
            ns: ns(),
            key: key("put"),
            value: b"value".to_vec(),
            ttl_ms: None,
            dimensions: vec!["region".to_owned()],
        },
        ClientRequest::Invalidate {
            ns: ns(),
            key: key("invalidate"),
        },
        ClientRequest::BatchGet {
            ns: ns(),
            keys: vec![key("batch-get")],
        },
        ClientRequest::BatchPut {
            ns: ns(),
            entries: vec![BatchPutEntry {
                key: key("batch-put"),
                value: b"batch".to_vec(),
            }],
        },
        ClientRequest::EvictRegion { ns: ns() },
        ClientRequest::SubscribeInvalidations {
            ns: ns(),
            region: Some(RegionId::new("region-a").unwrap()),
            from: Some(Watermark::new(1, 2)),
            include_value: true,
        },
        ClientRequest::SubscribeEntryEvents {
            ns: ns(),
            region: Some(RegionId::new("region-a").unwrap()),
            from: Some(Watermark::new(2, 3)),
            include_value: false,
            projection: EntryEventProjection::IMapEntryEvent,
        },
        ClientRequest::TryLock {
            ns: ns(),
            key: key("try-lock"),
            lease_ms: 5_000,
            wait_ms: 25,
            level: LockConsistency::Quorum,
        },
        ClientRequest::Unlock {
            ns: ns(),
            key: key("unlock"),
            fence: 7,
        },
        ClientRequest::RenewLockLease {
            ns: ns(),
            key: key("renew"),
            fence: 8,
            lease_ms: 4_000,
        },
        ClientRequest::ForceUnlock {
            ns: ns(),
            key: key("force"),
        },
        ClientRequest::GetLockOwnership {
            ns: ns(),
            key: key("owner"),
        },
        ClientRequest::CompareAndSet {
            ns: ns(),
            key: key("cas"),
            expected: CasExpectation::Present,
            new_value: b"new".to_vec(),
            level: LockConsistency::EachQuorum,
        },
        ClientRequest::RemoveIfValue {
            ns: ns(),
            key: key("remove"),
            expected: b"old".to_vec(),
            level: LockConsistency::All,
        },
    ]
}

fn v2_responses() -> Vec<ClientResponse> {
    vec![
        ClientResponse::Value {
            value: Some(b"value".to_vec()),
        },
        ClientResponse::Stored,
        ClientResponse::Invalidated,
        ClientResponse::Batch { items: Vec::new() },
        ClientResponse::Evicted,
        ClientResponse::Subscribed {
            from: Some(Watermark::new(3, 4)),
        },
        ClientResponse::LockAcquired { fence: 9 },
        ClientResponse::LockBusy,
        ClientResponse::LockReleased,
        ClientResponse::LockLeaseRenewed,
        ClientResponse::LockOwnership {
            fence: Some(10),
            locked: true,
        },
        ClientResponse::CasApplied { new_version: 11 },
        ClientResponse::CasMismatch {
            current: Some(b"current".to_vec()),
        },
    ]
}

#[test]
fn v1_historical_request_and_response_catalog_roundtrips() {
    for (index, request) in v1_requests().into_iter().enumerate() {
        let mut envelope = ClientRequestEnvelope::new(format!("v1-request-{index}"), request);
        envelope.protocol_version = 1;
        roundtrip(1, ClientWireMessage::Request(envelope));
    }
    for (index, response) in v1_responses().into_iter().enumerate() {
        roundtrip(
            1,
            ClientWireMessage::Response(
                ClientResponseEnvelope::ok(format!("v1-response-{index}"), response)
                    .with_protocol_version(1),
            ),
        );
    }

    roundtrip(
        1,
        ClientWireMessage::Response(
            ClientResponseEnvelope::error(
                "v1-error",
                ClientErrorEnvelope::new(ClientErrorCode::Conflict, false, "conflict"),
            )
            .with_protocol_version(1),
        ),
    );
    roundtrip(1, ClientWireMessage::Handshake(VersionHandshake::new(1, 1)));
    roundtrip(
        1,
        ClientWireMessage::Invalidation(InvalidationEvent::new(ns(), key("event"), 3, 4)),
    );
    roundtrip(1, ClientWireMessage::Heartbeat(Watermark::new(5, 6)));
}

#[test]
fn v1_rejects_every_post_v1_request_and_response_on_encode_and_decode() {
    for (index, request) in post_v1_requests().into_iter().enumerate() {
        let mut envelope = ClientRequestEnvelope::new(format!("post-v1-request-{index}"), request);
        envelope.protocol_version = 1;
        let message = ClientWireMessage::Request(envelope);
        assert!(matches!(
            ClientFrame::from_message_with_version(1, &message),
            Err(ClientProtocolError::UnsupportedMessageForVersion { version: 1, .. })
        ));

        // A caller cannot bypass the typed encoder by placing the current
        // message bytes in a v1 frame: the historical schema/min-version
        // validator must still reject before returning a typed message.
        let payload = postcard::to_allocvec(&message).unwrap();
        assert!(
            ClientFrame::with_version(1, payload)
                .decode_message()
                .is_err(),
            "post-v1 request {index} decoded through the v1 surface"
        );
    }

    for (index, response) in post_v1_responses().into_iter().enumerate() {
        let message = ClientWireMessage::Response(
            ClientResponseEnvelope::ok(format!("post-v1-response-{index}"), response)
                .with_protocol_version(1),
        );
        assert!(matches!(
            ClientFrame::from_message_with_version(1, &message),
            Err(ClientProtocolError::UnsupportedMessageForVersion { version: 1, .. })
        ));
        let payload = postcard::to_allocvec(&message).unwrap();
        assert!(
            ClientFrame::with_version(1, payload)
                .decode_message()
                .is_err(),
            "post-v1 response {index} decoded through the v1 surface"
        );
    }
}

#[test]
fn legacy_v2_operation_and_response_catalog_roundtrips() {
    for (index, request) in v2_requests().into_iter().enumerate() {
        let mut envelope = ClientRequestEnvelope::new(format!("v2-request-{index}"), request);
        envelope.protocol_version = 2;
        roundtrip(2, ClientWireMessage::Request(envelope));
    }
    for (index, response) in v2_responses().into_iter().enumerate() {
        roundtrip(
            2,
            ClientWireMessage::Response(
                ClientResponseEnvelope::ok(format!("v2-response-{index}"), response)
                    .with_protocol_version(2),
            ),
        );
    }
}

#[test]
fn legacy_v3_operation_and_response_catalog_roundtrips() {
    let mut requests = v2_requests();
    requests.splice(
        5..5,
        [
            ClientRequest::Put {
                ns: ns(),
                key: key("put-ttl"),
                value: b"value".to_vec(),
                ttl_ms: Some(500),
                dimensions: vec![],
            },
            ClientRequest::Expire {
                ns: ns(),
                key: key("expire"),
                ttl_ms: 750,
            },
            ClientRequest::Persist {
                ns: ns(),
                key: key("persist"),
            },
            ClientRequest::GetTtl {
                ns: ns(),
                key: key("ttl"),
            },
        ],
    );
    for (index, request) in requests.into_iter().enumerate() {
        let mut envelope = ClientRequestEnvelope::new(format!("v3-request-{index}"), request);
        envelope.protocol_version = 3;
        roundtrip(3, ClientWireMessage::Request(envelope));
    }

    let mut responses = v2_responses();
    responses.splice(
        4..4,
        [
            ClientResponse::Expiry { applied: true },
            ClientResponse::Ttl {
                state: TtlState::ExpiresIn { ttl_ms: 750 },
            },
        ],
    );
    for (index, response) in responses.into_iter().enumerate() {
        roundtrip(
            3,
            ClientWireMessage::Response(
                ClientResponseEnvelope::ok(format!("v3-response-{index}"), response)
                    .with_protocol_version(3),
            ),
        );
    }
}
