use hydracache_client_protocol::{
    ClientRequest, ClientRequestEnvelope, ClientResponse, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceLimits, ClientSurfaceProfileMetrics, ClientSurfaceState,
};

fn namespace() -> Namespace {
    Namespace::new("profile-074").unwrap()
}

fn key() -> StructuredKey {
    StructuredKey::new(vec!["key".to_owned()]).unwrap()
}

#[test]
fn client_surface_profile_counters_are_opt_in_bounded_and_resettable() {
    let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
    let identity = ClientIdentity::new("profile-client", "profile-tenant").unwrap();

    let put = || {
        state.dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new(
                "put-074",
                ClientRequest::Put {
                    ns: namespace(),
                    key: key(),
                    value: vec![7; 256],
                    ttl_ms: None,
                    dimensions: Vec::new(),
                },
            ),
        )
    };
    assert!(matches!(put().result, Ok(ClientResponse::Stored)));
    assert_eq!(
        state.profile_metrics(),
        ClientSurfaceProfileMetrics::default()
    );

    state.set_profile_instrumentation_enabled(true);
    state.reset_profile_metrics();
    assert!(matches!(put().result, Ok(ClientResponse::Stored)));
    let get = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "get-074",
            ClientRequest::Get {
                ns: namespace(),
                key: key(),
            },
        ),
    );
    assert!(matches!(
        get.result,
        Ok(ClientResponse::Value { value: Some(ref value) }) if value.len() == 256
    ));

    let metrics = state.profile_metrics();
    assert_eq!(metrics.dispatches, 2);
    assert_eq!(metrics.expiry_sweep_checks, 2);
    assert!(metrics.clock_reads >= 2);
    assert!(metrics.expiry_sweeps_claimed <= metrics.expiry_sweep_checks);
    assert!(
        metrics.store_lock_acquisitions >= 2,
        "unexpected profile metrics: {metrics:?}"
    );
    assert!(
        metrics.store_lock_wait_nanoseconds > 0,
        "unexpected profile metrics: {metrics:?}"
    );
    assert!(
        metrics.store_lock_hold_nanoseconds > 0,
        "unexpected profile metrics: {metrics:?}"
    );

    state.set_profile_instrumentation_enabled(false);
    state.reset_profile_metrics();
    assert_eq!(
        state.profile_metrics(),
        ClientSurfaceProfileMetrics::default()
    );
}
