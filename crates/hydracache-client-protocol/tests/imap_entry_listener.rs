use hydracache_client_protocol::java_migration::{
    JavaMapListenerProjection, JavaMapOperation, JavaMapProtocolFamily,
    UnsupportedHazelcastApiManifest,
};
use hydracache_client_protocol::{
    ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse, ClientResponseEnvelope,
    ClientWireMessage, EntryEvent, EntryEventKind, EntryEventProjection, EntryEventSource,
    EntryListenerContract, Namespace, StructuredKey, Watermark, LOCK_PROTOCOL_VERSION,
    PROTOCOL_VERSION,
};

fn ns() -> Namespace {
    Namespace::new("users").unwrap()
}

fn key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["user".to_owned(), id.to_owned()]).unwrap()
}

#[test]
fn invalidation_reasons_project_to_entry_event_kinds() {
    let cases = [
        (EntryEventSource::Stored, EntryEventKind::Upserted),
        (EntryEventSource::Removed, EntryEventKind::Removed),
        (EntryEventSource::Expired, EntryEventKind::Evicted),
        (EntryEventSource::Evicted, EntryEventKind::Evicted),
        (
            EntryEventSource::KeyInvalidated,
            EntryEventKind::Invalidated,
        ),
        (
            EntryEventSource::TagInvalidated,
            EntryEventKind::Invalidated,
        ),
        (EntryEventSource::Flushed, EntryEventKind::Invalidated),
        (
            EntryEventSource::StaleLoadDiscarded,
            EntryEventKind::Invalidated,
        ),
    ];

    for (source, expected) in cases {
        let event = EntryEvent::from_source(
            ns(),
            Some(key("42")),
            source,
            None,
            Some(Watermark::new(1, 7)),
        );
        assert_eq!(event.kind, expected, "{source:?}");
    }
}

#[test]
fn unmappable_reason_falls_back_to_invalidated_kind() {
    assert_eq!(
        EntryEventKind::from_source(EntryEventSource::Unknown),
        EntryEventKind::Invalidated
    );
}

#[test]
fn slow_listener_is_dropped_with_counter_not_unbounded() {
    let contract = EntryListenerContract::cache_signal();

    assert!(contract.coalesced);
    assert!(contract.bounded_buffer);
    assert!(contract.lag_drop_counter);
    assert!(!contract.business_event_log);
}

#[test]
fn add_entry_listener_maps_to_subscribe_family() {
    assert_eq!(
        JavaMapOperation::AddEntryListener.protocol_family(),
        JavaMapProtocolFamily::SubscribeInvalidations {
            projection: JavaMapListenerProjection::EntryEvent,
        }
    );

    let request = ClientRequest::SubscribeEntryEvents {
        ns: ns(),
        region: None,
        from: Some(Watermark::new(1, 2)),
        include_value: true,
        projection: EntryEventProjection::IMapEntryEvent,
    };
    assert_eq!(request.minimum_protocol_version(), LOCK_PROTOCOL_VERSION);

    let message = ClientWireMessage::Request(ClientRequestEnvelope::new("entries-1", request));
    let decoded = ClientFrame::decode(
        ClientFrame::from_message_with_version(PROTOCOL_VERSION, &message)
            .unwrap()
            .encode()
            .unwrap()
            .as_ref(),
        4096,
    )
    .unwrap()
    .decode_message()
    .unwrap();
    assert_eq!(decoded, message);

    let response = ClientWireMessage::Response(ClientResponseEnvelope::ok(
        "entries-1",
        ClientResponse::Subscribed {
            from: Some(Watermark::new(1, 2)),
        },
    ));
    assert_eq!(
        ClientFrame::decode(
            ClientFrame::from_message_with_version(PROTOCOL_VERSION, &response)
                .unwrap()
                .encode()
                .unwrap()
                .as_ref(),
            4096,
        )
        .unwrap()
        .decode_message()
        .unwrap(),
        response
    );
}

#[test]
fn entry_listener_is_not_a_business_event_log() {
    let contract = EntryListenerContract::cache_signal();
    assert_eq!(
        contract,
        EntryListenerContract {
            coalesced: true,
            bounded_buffer: true,
            lag_drop_counter: true,
            business_event_log: false,
        }
    );

    let manifest = UnsupportedHazelcastApiManifest::parse(include_str!(
        "../manifests/unsupported_hazelcast_apis.txt"
    ))
    .unwrap();
    assert!(manifest.find_supported("IMap.addEntryListener").is_some());
    assert!(manifest.find("Ringbuffer").is_some());
    assert!(manifest.find("ReliableTopic").is_some());
}
