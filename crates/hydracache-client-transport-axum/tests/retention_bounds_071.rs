use hydracache_client_protocol::{
    ClientErrorCode, ClientRequest, ClientRequestEnvelope, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};
use std::sync::Arc;

use hydracache_observability::{AuditEvent, AuditRecorder, InMemoryAuditSink};

fn put(
    state: &ClientSurfaceState,
    identity: &ClientIdentity,
    request_id: String,
    key: String,
    idempotency_key: Option<String>,
) -> hydracache_client_protocol::ClientResponseEnvelope {
    let mut request = ClientRequestEnvelope::new(
        request_id,
        ClientRequest::Put {
            ns: Namespace::new("retention").expect("namespace"),
            key: StructuredKey::new(vec![key]).expect("key"),
            value: b"value".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    );
    request.idempotency_key = idempotency_key;
    state.dispatch_verified_request(identity, request)
}

#[test]
fn fixed_keyspace_mutations_plateau_and_diagnostic_reset_releases_owners() {
    let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).expect("state");
    let identity = ClientIdentity::new("client", "tenant").expect("identity");
    for index in 0..100_000 {
        let response = put(
            &state,
            &identity,
            format!("put-{index}"),
            format!("key-{}", index % 64),
            None,
        );
        assert!(response.result.is_ok());
    }

    let plateau = state.retained_state_for_diagnostics();
    assert_eq!(plateau.store_entries, 64);
    assert_eq!(plateau.idempotency_outcomes, 0);
    let reset = state
        .reset_retained_state_for_diagnostics()
        .expect("diagnostic reset");
    assert_eq!(reset.after.store_entries, 0);
    assert_eq!(reset.after.value_bytes, 0);
    assert_eq!(reset.after.store_identity_bytes, 0);
}

#[test]
fn idempotency_retention_is_bounded_and_overflow_fails_before_mutation() {
    let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).expect("state");
    let identity = ClientIdentity::new("client", "tenant").expect("identity");
    state.set_cache_time_for_tests(Some(1_000));
    let mut rejected_at = None;
    for index in 0..10_000 {
        let response = put(
            &state,
            &identity,
            format!("request-{index}"),
            "fixed-key".to_owned(),
            Some(format!("idempotency-{index}")),
        );
        if let Err(error) = response.result {
            assert_eq!(error.code, ClientErrorCode::RateLimited);
            rejected_at = Some(index);
            break;
        }
    }
    let capacity = rejected_at.expect("finite idempotency capacity");
    let retained = state.retained_state_for_diagnostics();
    assert_eq!(retained.idempotency_outcomes, capacity);
    assert_eq!(retained.store_entries, 1);
}

#[test]
fn mandatory_audit_capacity_fails_closed_and_never_exceeds_its_bound() {
    let sink = Arc::new(InMemoryAuditSink::with_capacity(2));
    let mut recorder = AuditRecorder::new(Arc::clone(&sink));
    for index in 0..2 {
        recorder
            .record(&AuditEvent::AuthFailure {
                tenant: Some("tenant".to_owned()),
                route: "/client/v1/data".to_owned(),
                request_id: Some(format!("request-{index}")),
            })
            .expect("within capacity");
    }
    let error = recorder
        .record(&AuditEvent::AuthFailure {
            tenant: Some("tenant".to_owned()),
            route: "/client/v1/data".to_owned(),
            request_id: Some("overflow".to_owned()),
        })
        .expect_err("mandatory audit must fail closed");
    assert!(error.to_string().contains("capacity exhausted"));
    assert_eq!(sink.events().len(), 2);
}

#[test]
#[ignore = "scheduled 0.71 million-operation retention proof"]
fn million_operation_fixed_keyspace_plateau() {
    let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).expect("state");
    let identity = ClientIdentity::new("client", "tenant").expect("identity");
    for index in 0..1_000_000 {
        assert!(put(
            &state,
            &identity,
            format!("scheduled-{index}"),
            format!("key-{}", index % 64),
            None,
        )
        .result
        .is_ok());
    }
    assert_eq!(state.retained_state_for_diagnostics().store_entries, 64);
}
