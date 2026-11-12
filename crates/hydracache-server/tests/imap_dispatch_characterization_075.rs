//! Current dispatch characterization before backend extraction. No new capability is enabled.
use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    CasExpectation, ClientErrorCode, ClientFrame, ClientRequest, ClientRequestEnvelope,
    ClientResponse, ClientWireMessage, LockConsistency, Namespace, StructuredKey, TtlState,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};

fn ns() -> Namespace {
    Namespace::new("imap075").unwrap()
}
fn key() -> StructuredKey {
    StructuredKey::new(vec!["binary-00ff".into(), "ключ".into()]).unwrap()
}
fn state() -> ClientSurfaceState {
    let state = ClientSurfaceState::new(ClientSurfaceLimits {
        max_value_bytes: 32,
        ..ClientSurfaceLimits::default()
    })
    .unwrap();
    state.set_cache_time_for_tests(Some(1000));
    state
}
fn identity(tenant: &str) -> ClientIdentity {
    ClientIdentity::new("characterization", tenant).unwrap()
}
fn put(value: &[u8], ttl: Option<u64>) -> ClientRequest {
    ClientRequest::Put {
        ns: ns(),
        key: key(),
        value: value.to_vec(),
        ttl_ms: ttl,
        dimensions: Vec::new(),
    }
}
fn get() -> ClientRequest {
    ClientRequest::Get {
        ns: ns(),
        key: key(),
    }
}
fn dispatch(
    state: &ClientSurfaceState,
    tenant: &str,
    request: ClientRequest,
) -> Result<ClientResponse, hydracache_client_protocol::ClientErrorEnvelope> {
    // Exercise the actual current HC/1 codec on both sides of verified dispatch.
    let message =
        ClientWireMessage::Request(ClientRequestEnvelope::new("characterization", request));
    let bytes = ClientFrame::from_message(&message)
        .unwrap()
        .encode()
        .unwrap();
    let ClientWireMessage::Request(request) = ClientFrame::decode(&bytes, 1 << 20)
        .unwrap()
        .decode_message()
        .unwrap()
    else {
        panic!("request codec")
    };
    let response = state.dispatch_verified_request(&identity(tenant), request);
    let frame = ClientFrame::from_message(&ClientWireMessage::Response(response))
        .unwrap()
        .encode()
        .unwrap();
    let ClientWireMessage::Response(response) = ClientFrame::decode(&frame, 1 << 20)
        .unwrap()
        .decode_message()
        .unwrap()
    else {
        panic!("response codec")
    };
    response.result
}

#[test]
fn binary_empty_values_and_tenant_namespace_boundaries_survive_current_codec() {
    let state = state();
    for value in [vec![], vec![0, 128, 255]] {
        assert_eq!(
            dispatch(&state, "first", put(&value, None)).unwrap(),
            ClientResponse::Stored
        );
        assert_eq!(
            dispatch(&state, "first", get()).unwrap(),
            ClientResponse::Value { value: Some(value) }
        );
        assert_eq!(
            dispatch(&state, "second", get()).unwrap(),
            ClientResponse::Value { value: None }
        );
        assert_eq!(
            dispatch(
                &state,
                "first",
                ClientRequest::Get {
                    ns: Namespace::new("other").unwrap(),
                    key: key()
                }
            )
            .unwrap(),
            ClientResponse::Value { value: None }
        );
    }
}

#[test]
fn ttl_conditional_mismatch_and_persist_keep_the_current_atomic_behavior() {
    let state = state();
    dispatch(&state, "first", put(&[0, 255], Some(10))).unwrap();
    state.advance_cache_time_for_tests(5);
    let mismatch = ClientRequest::CompareAndSet {
        ns: ns(),
        key: key(),
        expected: CasExpectation::Exact(vec![42]),
        new_value: vec![99],
        level: LockConsistency::Quorum,
    };
    assert!(
        matches!(dispatch(&state,"first",mismatch).unwrap(),ClientResponse::CasMismatch{current:Some(v)} if v==[0,255])
    );
    assert_eq!(
        dispatch(
            &state,
            "first",
            ClientRequest::GetTtl {
                ns: ns(),
                key: key()
            }
        )
        .unwrap(),
        ClientResponse::Ttl {
            state: TtlState::ExpiresIn { ttl_ms: 5 }
        }
    );
    state.advance_cache_time_for_tests(5);
    assert_eq!(
        dispatch(&state, "first", get()).unwrap(),
        ClientResponse::Value { value: None }
    );
    dispatch(&state, "first", put(&[1], Some(5))).unwrap();
    assert_eq!(
        dispatch(
            &state,
            "first",
            ClientRequest::Persist {
                ns: ns(),
                key: key()
            }
        )
        .unwrap(),
        ClientResponse::Expiry { applied: true }
    );
    state.advance_cache_time_for_tests(100);
    assert_eq!(
        dispatch(&state, "first", get()).unwrap(),
        ClientResponse::Value {
            value: Some(vec![1])
        }
    );
}

#[test]
fn current_hc1_deadline_origin_is_zero_and_zero_deadline_refuses_before_mutation() {
    let state = state();
    let identity = identity("first");
    // This records the existing limitation. It does not assert a wall-clock deadline guarantee.
    let positive = ClientRequestEnvelope::new("positive", put(&[1], None)).with_deadline_ms(1);
    assert_eq!(
        state
            .dispatch_verified_request(&identity, positive)
            .result
            .unwrap(),
        ClientResponse::Stored
    );
    let before = state.state_mutations();
    let zero = ClientRequestEnvelope::new("zero", put(&[2], None)).with_deadline_ms(0);
    assert_eq!(
        state
            .dispatch_verified_request(&identity, zero)
            .result
            .unwrap_err()
            .code,
        ClientErrorCode::DeadlineExceeded
    );
    assert_eq!(state.state_mutations(), before);
    assert_eq!(
        dispatch(&state, "first", get()).unwrap(),
        ClientResponse::Value {
            value: Some(vec![1])
        }
    );
}

#[test]
fn future_protocol_and_oversized_values_do_not_change_the_current_owner() {
    let state = state();
    dispatch(&state, "first", put(&[1], None)).unwrap();
    let before = state.state_mutations();
    let mut future = ClientRequestEnvelope::new("future", put(&[2], None));
    future.protocol_version = u16::MAX;
    assert_eq!(
        state
            .dispatch_verified_request(&identity("first"), future)
            .result
            .unwrap_err()
            .code,
        ClientErrorCode::IncompatibleVersion
    );
    assert_eq!(
        dispatch(
            &state,
            "first",
            put(&vec![0; state.limits().max_value_bytes + 1], None)
        )
        .unwrap_err()
        .code,
        ClientErrorCode::TooLarge
    );
    assert_eq!(state.state_mutations(), before);
    assert_eq!(
        dispatch(&state, "first", get()).unwrap(),
        ClientResponse::Value {
            value: Some(vec![1])
        }
    );
}

#[test]
fn configured_roster_refuses_tenant_substitution_before_dispatch_and_mutation() {
    let roster = TenantRoster::new(vec![
        Tenant::new("first")
            .unwrap()
            .allow_client("allowed")
            .namespace("imap075", NamespaceQuota::new(16, 2)),
        Tenant::new("second")
            .unwrap()
            .allow_client("other")
            .namespace("imap075", NamespaceQuota::new(16, 2)),
    ])
    .unwrap();
    let state = ClientSurfaceState::with_isolation(
        ClientSurfaceLimits::default(),
        ConsumerIsolation::new(roster, ConsumerIsolationConfig::default()),
    )
    .unwrap();
    let substituted = ClientIdentity::new("allowed", "second").unwrap();
    assert_eq!(
        state
            .dispatch_verified_request(
                &substituted,
                ClientRequestEnvelope::new("forged", put(&[1], None))
            )
            .result
            .unwrap_err()
            .code,
        ClientErrorCode::Unauthorized
    );
    assert_eq!(state.dispatch_attempts(), 0);
    assert_eq!(state.state_mutations(), 0);
}
