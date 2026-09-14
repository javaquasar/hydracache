use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    BatchPutEntry, ClientErrorCode, ClientRequest, ClientRequestEnvelope, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};

fn namespace() -> Namespace {
    Namespace::new("memory-admission").expect("namespace")
}

fn key(value: &str) -> StructuredKey {
    StructuredKey::new(vec![value.to_owned()]).expect("key")
}

fn isolated_state(max_bytes: u64, max_entries: u64) -> ClientSurfaceState {
    let roster = TenantRoster::new(vec![Tenant::new("tenant")
        .expect("tenant")
        .allow_client("client")
        .namespace(
            "memory-admission",
            NamespaceQuota::new(max_bytes, max_entries),
        )])
    .expect("roster");
    ClientSurfaceState::with_isolation(
        ClientSurfaceLimits::default(),
        ConsumerIsolation::new(roster, ConsumerIsolationConfig::default()),
    )
    .expect("state")
}

fn dispatch(
    state: &ClientSurfaceState,
    request_id: &str,
    request: ClientRequest,
) -> hydracache_client_protocol::ClientResponseEnvelope {
    state.dispatch_verified_request(
        &ClientIdentity::new("client", "tenant").expect("identity"),
        ClientRequestEnvelope::new(request_id, request),
    )
}

#[test]
fn aggregate_batch_quota_is_rejected_before_partial_mutation() {
    let state = isolated_state(8, 8);
    let response = dispatch(
        &state,
        "batch-over-budget",
        ClientRequest::BatchPut {
            ns: namespace(),
            entries: vec![
                BatchPutEntry {
                    key: key("first"),
                    value: vec![1; 5],
                },
                BatchPutEntry {
                    key: key("second"),
                    value: vec![2; 5],
                },
            ],
        },
    );

    let error = response.result.expect_err("aggregate batch must fail");
    assert_eq!(error.code, ClientErrorCode::TenantQuota);
    let retained = state.retained_state_for_diagnostics();
    assert_eq!(retained.store_entries, 0);
    assert_eq!(retained.value_bytes, 0);
}

#[test]
fn replacement_is_charged_by_delta_and_delete_releases_quota() {
    let state = isolated_state(8, 1);
    let put = |request_id: &str, value: Vec<u8>| {
        dispatch(
            &state,
            request_id,
            ClientRequest::Put {
                ns: namespace(),
                key: key("stable"),
                value,
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        )
    };

    assert!(put("initial", vec![1; 5]).result.is_ok());
    assert!(put("replacement", vec![2; 8]).result.is_ok());
    assert_eq!(state.retained_state_for_diagnostics().value_bytes, 8);

    let rejected = put("too-large-replacement", vec![3; 9]);
    assert_eq!(
        rejected.result.expect_err("quota must reject").code,
        ClientErrorCode::TenantQuota
    );
    assert_eq!(state.retained_state_for_diagnostics().value_bytes, 8);

    assert!(dispatch(
        &state,
        "delete",
        ClientRequest::Invalidate {
            ns: namespace(),
            key: key("stable"),
        },
    )
    .result
    .is_ok());
    assert_eq!(state.retained_state_for_diagnostics().store_entries, 0);
    assert!(put("refill", vec![4; 8]).result.is_ok());
}

#[test]
fn active_expiry_releases_entry_and_tenant_quota_without_reading_the_key() {
    let state = isolated_state(5, 1);
    state.set_cache_time_for_tests(Some(1_000));
    assert!(dispatch(
        &state,
        "expiring",
        ClientRequest::Put {
            ns: namespace(),
            key: key("old"),
            value: vec![1; 5],
            ttl_ms: Some(500),
            dimensions: Vec::new(),
        },
    )
    .result
    .is_ok());

    state.advance_cache_time_for_tests(501);
    assert_eq!(state.retained_state_for_diagnostics().store_entries, 0);
    assert!(dispatch(
        &state,
        "replacement",
        ClientRequest::Put {
            ns: namespace(),
            key: key("new"),
            value: vec![2; 5],
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    )
    .result
    .is_ok());
}
