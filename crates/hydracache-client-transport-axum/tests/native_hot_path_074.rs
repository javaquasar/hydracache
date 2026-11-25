use hydracache_client_protocol::{
    BatchPutEntry, ClientRequest, ClientRequestEnvelope, ClientResponse, Namespace, StructuredKey,
    TtlState,
};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceLimits, ClientSurfaceMutationKind, ClientSurfaceProfileMetrics,
    ClientSurfaceState,
};

fn namespace() -> Namespace {
    Namespace::new("profile-074").unwrap()
}

fn key() -> StructuredKey {
    StructuredKey::new(vec!["key".to_owned()]).unwrap()
}

#[tokio::test]
async fn put_samples_the_write_clock_only_for_ttl_or_idempotency() {
    let identity = ClientIdentity::new("profile-client", "profile-tenant").unwrap();
    for (ttl_ms, idempotent, clock_reads) in [
        (None, false, 1),
        (Some(50), false, 2),
        (None, true, 2),
        (Some(50), true, 2),
    ] {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        state.set_cache_time_for_tests(Some(1_000));
        state.set_profile_instrumentation_enabled(true);
        let dispatcher = state.dispatcher();
        let mut subscriber = state.subscribe_mutations(&identity).unwrap();
        let put = |id, value| {
            let request = ClientRequestEnvelope::new(
                id,
                ClientRequest::Put {
                    ns: namespace(),
                    key: key(),
                    value,
                    ttl_ms,
                    dimensions: Vec::new(),
                },
            );
            if idempotent {
                request.with_idempotency_key("write-once")
            } else {
                request
            }
        };

        let response = dispatcher
            .dispatch_verified_request(&identity, put("write", vec![1]))
            .await;
        assert_eq!(response.request_id, "write");
        assert_eq!(response.result, Ok(ClientResponse::Stored));
        let metrics = state.profile_metrics();
        assert_eq!(metrics.expiry_sweep_checks, 1);
        assert_eq!(metrics.clock_reads, clock_reads);
        assert_eq!(metrics.clock_floor_seqcst_operations, clock_reads);
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), subscriber.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.key(), Some(&key()));
        assert_eq!(event.kind(), ClientSurfaceMutationKind::Stored);

        if idempotent {
            state.advance_cache_time_for_tests(1);
            let replay = dispatcher
                .dispatch_verified_request(&identity, put("replay", vec![2]))
                .await;
            assert_eq!(replay.request_id, "replay");
            assert_eq!(replay.result, Ok(ClientResponse::Stored));
            assert_eq!(state.state_mutations(), 1);
        }
        let response = dispatcher
            .dispatch_verified_request(
                &identity,
                ClientRequestEnvelope::new(
                    "read",
                    ClientRequest::Get {
                        ns: namespace(),
                        key: key(),
                    },
                ),
            )
            .await;
        assert_eq!(
            response.result,
            Ok(ClientResponse::Value {
                value: Some(vec![1])
            })
        );
        let ttl = dispatcher
            .dispatch_verified_request(
                &identity,
                ClientRequestEnvelope::new(
                    "ttl",
                    ClientRequest::GetTtl {
                        ns: namespace(),
                        key: key(),
                    },
                ),
            )
            .await;
        let expected = ttl_ms.map_or(TtlState::Persistent, |ttl_ms| TtlState::ExpiresIn {
            ttl_ms: ttl_ms - u64::from(idempotent),
        });
        assert_eq!(ttl.result, Ok(ClientResponse::Ttl { state: expected }));
    }
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
    // A real clock may report zero for a fast operation. Exact duration accounting,
    // including zero samples, is checked by the library's deterministic timing test.
    assert!(metrics.expiry_sweeps_claimed <= metrics.expiry_sweep_checks);
    assert!(
        metrics.store_lock_acquisitions >= 2,
        "unexpected profile metrics: {metrics:?}"
    );
    assert_eq!(metrics.live_get_hits, 1);
    assert_eq!(metrics.expired_get_removals, 0);
    assert_eq!(metrics.missing_gets, 0);
    assert_eq!(metrics.structured_key_materializations, 2);
    assert!(metrics.structured_key_bytes_copied > 0);
    assert_eq!(metrics.batch_operations, 0);
    assert_eq!(metrics.batch_items, 0);
    assert_eq!(metrics.dispatch_counter_seqcst_operations, 2);
    assert!(metrics.clock_floor_seqcst_operations >= 2);
    assert!(metrics.expiry_gate_seqcst_operations >= 2);
    assert_eq!(metrics.mutation_counter_seqcst_operations, 1);
    assert_eq!(metrics.message_id_seqcst_operations, 0);

    state.set_profile_instrumentation_enabled(false);
    state.reset_profile_metrics();
    assert_eq!(
        state.profile_metrics(),
        ClientSurfaceProfileMetrics::default()
    );
}

#[test]
fn client_surface_profile_attributes_multi_key_shape_and_key_materialization() {
    let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
    let identity = ClientIdentity::new("profile-client", "profile-tenant").unwrap();
    state.set_profile_instrumentation_enabled(true);
    state.reset_profile_metrics();

    let entries = (0..4)
        .map(|index| BatchPutEntry {
            key: StructuredKey::new(vec![format!("key-{index}")]).unwrap(),
            value: vec![index as u8; 32],
        })
        .collect::<Vec<_>>();
    let put = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "batch-put-074",
            ClientRequest::BatchPut {
                ns: namespace(),
                entries,
            },
        ),
    );
    assert!(matches!(put.result, Ok(ClientResponse::Batch { ref items }) if items.len() == 4));

    let get = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "batch-get-074",
            ClientRequest::BatchGet {
                ns: namespace(),
                keys: (0..4)
                    .map(|index| StructuredKey::new(vec![format!("key-{index}")]).unwrap())
                    .collect(),
            },
        ),
    );
    assert!(matches!(get.result, Ok(ClientResponse::Batch { ref items }) if items.len() == 4));

    let metrics = state.profile_metrics();
    assert_eq!(metrics.batch_operations, 2);
    assert_eq!(metrics.batch_items, 8);
    assert_eq!(metrics.structured_key_materializations, 8);
    assert!(metrics.structured_key_bytes_copied >= 8 * "key-0".len() as u64);
    assert_eq!(metrics.mutation_counter_seqcst_operations, 1);
}

#[test]
fn client_surface_profile_separates_direct_expiry_cleanup_from_miss() {
    let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
    let identity = ClientIdentity::new("profile-client", "profile-tenant").unwrap();
    state.set_cache_time_for_tests(Some(1_000));
    let put = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "put-expiring-074",
            ClientRequest::Put {
                ns: namespace(),
                key: key(),
                value: vec![7; 256],
                ttl_ms: Some(1),
                dimensions: Vec::new(),
            },
        ),
    );
    assert!(matches!(put.result, Ok(ClientResponse::Stored)));
    state.advance_cache_time_for_tests(1);
    state.set_profile_instrumentation_enabled(true);
    state.reset_profile_metrics();

    let expired = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "get-expired-074",
            ClientRequest::Get {
                ns: namespace(),
                key: key(),
            },
        ),
    );
    assert!(matches!(
        expired.result,
        Ok(ClientResponse::Value { value: None })
    ));
    let metrics = state.profile_metrics();
    assert_eq!(metrics.live_get_hits, 0);
    assert_eq!(metrics.expired_get_removals, 1);
    assert_eq!(metrics.missing_gets, 0);
}
