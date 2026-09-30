use std::sync::{Arc, Barrier};
use std::thread;

use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    ClientErrorCode, ClientRequest, ClientRequestEnvelope, ClientResponse, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};

fn namespace() -> Namespace {
    Namespace::new("redis").unwrap()
}

fn key(value: &str) -> StructuredKey {
    StructuredKey::new(vec![value.to_owned()]).unwrap()
}

fn identity() -> ClientIdentity {
    ClientIdentity::new("client-a", "tenant-a").unwrap()
}

fn put(state: &ClientSurfaceState, identity: &ClientIdentity, name: &str) {
    let response = state.dispatch_verified_request(
        identity,
        ClientRequestEnvelope::new(
            format!("put-{name}"),
            ClientRequest::Put {
                ns: namespace(),
                key: key(name),
                value: name.as_bytes().to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ),
    );
    assert!(matches!(response.result, Ok(ClientResponse::Stored)));
}

fn isolated_state() -> ClientSurfaceState {
    let roster = TenantRoster::new(vec![Tenant::new("tenant-a")
        .unwrap()
        .allow_client("client-a")
        .namespace("redis", NamespaceQuota::new(1_024, 32))])
    .unwrap();
    ClientSurfaceState::with_isolation(
        ClientSurfaceLimits::default(),
        ConsumerIsolation::new(roster, ConsumerIsolationConfig::default()),
    )
    .unwrap()
}

#[test]
fn verified_batch_invalidation_is_ordered_and_uses_one_dispatch_and_store_lock() {
    let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
    let identity = identity();
    put(&state, &identity, "a");
    put(&state, &identity, "b");

    state.set_profile_instrumentation_enabled(true);
    state.reset_profile_metrics();
    let outcome = state.dispatch_verified_batch_invalidation(
        &identity,
        "batch-delete",
        namespace(),
        vec![key("a"), key("missing"), key("a"), key("b")],
    );

    let items = outcome.result.unwrap();
    assert_eq!(
        items.iter().map(|item| item.removed).collect::<Vec<_>>(),
        [true, false, false, true]
    );
    assert_eq!(
        items.iter().map(|item| item.index).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    let metrics = state.profile_metrics();
    assert_eq!(metrics.dispatches, 1);
    assert_eq!(metrics.store_lock_acquisitions, 1);
}

#[test]
fn verified_batch_invalidation_rejects_over_limit_before_mutation() {
    let state = ClientSurfaceState::new(ClientSurfaceLimits {
        max_batch_entries: 2,
        ..ClientSurfaceLimits::default()
    })
    .unwrap();
    let identity = identity();
    for name in ["a", "b", "c"] {
        put(&state, &identity, name);
    }

    state.set_profile_instrumentation_enabled(true);
    state.reset_profile_metrics();
    let outcome = state.dispatch_verified_batch_invalidation(
        &identity,
        "oversized-delete",
        namespace(),
        vec![key("a"), key("b"), key("c")],
    );
    assert_eq!(outcome.result.unwrap_err().code, ClientErrorCode::TooLarge);
    let metrics = state.profile_metrics();
    assert_eq!(metrics.dispatches, 1);
    assert_eq!(metrics.store_lock_acquisitions, 0);
    assert_eq!(state.retained_state_for_diagnostics().store_entries, 3);
}

#[test]
fn verified_batch_invalidation_rejects_mismatched_tenant_before_dispatch() {
    let state = isolated_state();
    let mismatched = ClientIdentity::new("client-a", "tenant-b").unwrap();
    let outcome = state.dispatch_verified_batch_invalidation(
        &mismatched,
        "unauthorized-delete",
        namespace(),
        vec![key("a")],
    );

    assert_eq!(
        outcome.result.unwrap_err().code,
        ClientErrorCode::Unauthorized
    );
    assert_eq!(state.dispatch_attempts(), 0);
    assert_eq!(state.audit_events_for_tests().len(), 1);
}

#[test]
fn batch_get_never_observes_a_partially_applied_batch_invalidation() {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let identity = identity();
    put(&state, &identity, "a");
    put(&state, &identity, "b");
    let barrier = Arc::new(Barrier::new(2));

    let reader_state = Arc::clone(&state);
    let reader_identity = identity.clone();
    let reader_barrier = Arc::clone(&barrier);
    let reader = thread::spawn(move || {
        reader_barrier.wait();
        for sequence in 0..1_000 {
            let response = reader_state.dispatch_verified_request(
                &reader_identity,
                ClientRequestEnvelope::new(
                    format!("read-{sequence}"),
                    ClientRequest::BatchGet {
                        ns: namespace(),
                        keys: vec![key("a"), key("b")],
                    },
                ),
            );
            let Ok(ClientResponse::Batch { items }) = response.result else {
                panic!("batch read failed");
            };
            let present = items
                .iter()
                .map(|item| item.result.as_ref().unwrap().is_some())
                .collect::<Vec<_>>();
            assert!(present == [true, true] || present == [false, false]);
        }
    });

    barrier.wait();
    let outcome = state.dispatch_verified_batch_invalidation(
        &identity,
        "concurrent-delete",
        namespace(),
        vec![key("a"), key("b")],
    );
    assert!(outcome.result.unwrap().iter().all(|item| item.removed));
    reader.join().unwrap();
}
