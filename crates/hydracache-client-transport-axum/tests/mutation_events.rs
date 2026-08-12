use std::sync::Arc;
use std::time::Duration;

use hydracache_client_protocol::{ClientRequest, ClientRequestEnvelope, Namespace, StructuredKey};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceLimits, ClientSurfaceMutationKind, ClientSurfaceMutationRecvError,
    ClientSurfaceState, CLIENT_SURFACE_MUTATION_EVENT_CAPACITY,
};
use tokio::time::timeout;

#[tokio::test]
async fn mutation_subscriber_is_tenant_fenced_and_metadata_only() {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let tenant_a = ClientIdentity::new("client-a", "tenant-a").unwrap();
    let tenant_b = ClientIdentity::new("client-b", "tenant-b").unwrap();
    let mut subscriber = state.subscribe_mutations(&tenant_a).unwrap();

    put(&state, &tenant_b, "foreign", b"secret");
    put(&state, &tenant_a, "visible", b"value-is-never-in-the-event");

    let event = timeout(Duration::from_secs(1), subscriber.recv())
        .await
        .expect("tenant-scoped mutation timeout")
        .unwrap();
    assert_eq!(event.tenant(), "tenant-a");
    assert_eq!(event.namespace().as_str(), "redis");
    assert_eq!(
        event.key(),
        Some(&StructuredKey::new(vec!["visible".to_owned()]).unwrap())
    );
    assert_eq!(event.kind(), ClientSurfaceMutationKind::Stored);
    assert!(event.message_id() > 0);
}

#[tokio::test]
async fn mutation_subscriber_fails_loudly_after_bounded_bus_lag() {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let identity = ClientIdentity::new("slow-listener", "tenant-a").unwrap();
    let mut subscriber = state.subscribe_mutations(&identity).unwrap();

    for index in 0..=CLIENT_SURFACE_MUTATION_EVENT_CAPACITY {
        put(&state, &identity, &format!("key-{index}"), b"bounded-value");
    }

    assert!(matches!(
        subscriber.recv().await,
        Err(ClientSurfaceMutationRecvError::Lagged(count)) if count >= 1
    ));
}

fn put(state: &ClientSurfaceState, identity: &ClientIdentity, key: &str, value: &[u8]) {
    let response = state.dispatch_verified_request(
        identity,
        ClientRequestEnvelope::new(
            format!("put-{key}"),
            ClientRequest::Put {
                ns: Namespace::new("redis").unwrap(),
                key: StructuredKey::new(vec![key.to_owned()]).unwrap(),
                value: value.to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ),
    );
    assert!(response.result.is_ok());
}
