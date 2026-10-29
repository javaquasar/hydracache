use std::sync::Arc;

use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    BatchPutEntry, ClientErrorCode, ClientRequest, ClientRequestEnvelope, ClientResponse,
    Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};

const BATCH: usize = 32;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_batch_get_observes_all_old_or_all_new_values() {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let identity = ClientIdentity::new("client", "tenant").unwrap();
    batch_put(&state, &identity, 0, 1);

    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let writer_state = Arc::clone(&state);
    let writer_identity = identity.clone();
    let writer_barrier = Arc::clone(&barrier);
    let writer = tokio::spawn(async move {
        writer_barrier.wait().await;
        for round in 1..=500 {
            batch_put(
                &writer_state,
                &writer_identity,
                round,
                if round % 2 == 0 { 1 } else { 2 },
            );
            tokio::task::yield_now().await;
        }
    });

    let reader_state = Arc::clone(&state);
    let reader_identity = identity.clone();
    let reader_barrier = Arc::clone(&barrier);
    let reader = tokio::spawn(async move {
        reader_barrier.wait().await;
        for round in 0..1_000 {
            let values = batch_get(&reader_state, &reader_identity, round);
            assert_eq!(values.len(), BATCH);
            let first = values[0][0];
            assert!(first == 1 || first == 2);
            assert!(
                values.iter().all(|value| value == &vec![first; 16]),
                "batch GET observed a torn batch at round {round}"
            );
            tokio::task::yield_now().await;
        }
    });

    barrier.wait().await;
    writer.await.unwrap();
    reader.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn expiration_race_removes_each_batch_entry_once_and_returns_only_misses() {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let identity = ClientIdentity::new("client", "tenant").unwrap();
    state.set_cache_time_for_tests(Some(1_000));
    let entries = (0..BATCH)
        .map(|index| BatchPutEntry {
            key: key(index),
            value: vec![7; 16],
        })
        .collect::<Vec<_>>();
    // BatchPut has no TTL, so preload expiring entries individually at the same logical instant.
    for entry in entries {
        let response = dispatch(
            &state,
            &identity,
            format!("expiry-put-{}", entry.key.stable_key()),
            ClientRequest::Put {
                ns: namespace(),
                key: entry.key,
                value: entry.value,
                ttl_ms: Some(1),
                dimensions: Vec::new(),
            },
        );
        assert_eq!(response, ClientResponse::Stored);
    }
    let mutations_before = state.state_mutations();
    state.advance_cache_time_for_tests(1_001);
    state.set_profile_instrumentation_enabled(true);
    state.reset_profile_metrics();

    let barrier = Arc::new(tokio::sync::Barrier::new(17));
    let mut readers = Vec::new();
    for worker in 0..16 {
        let state = Arc::clone(&state);
        let identity = identity.clone();
        let barrier = Arc::clone(&barrier);
        readers.push(tokio::spawn(async move {
            barrier.wait().await;
            let response = dispatch(
                &state,
                &identity,
                format!("expiry-batch-{worker}"),
                ClientRequest::BatchGet {
                    ns: namespace(),
                    keys: (0..BATCH).map(key).collect(),
                },
            );
            let ClientResponse::Batch { items } = response else {
                panic!("expiry race must return a batch");
            };
            assert!(items.iter().all(|item| matches!(&item.result, Ok(None))));
        }));
    }
    barrier.wait().await;
    for reader in readers {
        reader.await.unwrap();
    }

    let metrics = state.profile_metrics();
    assert_eq!(metrics.expiry_sweeps_claimed, 1);
    assert_eq!(metrics.expiry_sweep_entries_examined, BATCH as u64);
    assert_eq!(metrics.expiry_sweep_entries_removed, BATCH as u64);
    assert_eq!(metrics.batch_operations, 16);
    assert_eq!(metrics.batch_items, (16 * BATCH) as u64);
    assert_eq!(state.state_mutations() - mutations_before, BATCH as u64);
    assert_eq!(state.retained_state_for_diagnostics().store_entries, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_callers_never_reuse_authorization_across_tenant_bindings() {
    let tenant = Tenant::new("tenant-a")
        .unwrap()
        .allow_client("client-a")
        .namespace("users", NamespaceQuota::new(1 << 20, 1024));
    let state = Arc::new(
        ClientSurfaceState::with_isolation(
            ClientSurfaceLimits::default(),
            ConsumerIsolation::new(
                TenantRoster::new(vec![tenant]).unwrap(),
                ConsumerIsolationConfig::default(),
            ),
        )
        .unwrap(),
    );
    let authorized = ClientIdentity::new("client-a", "tenant-a").unwrap();
    let forged = ClientIdentity::new("client-a", "tenant-b").unwrap();
    let stored = dispatch(
        &state,
        &authorized,
        "authorized-put",
        ClientRequest::Put {
            ns: namespace(),
            key: key(0),
            value: vec![9; 16],
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    );
    assert_eq!(stored, ClientResponse::Stored);

    let barrier = Arc::new(tokio::sync::Barrier::new(65));
    let mut callers = Vec::new();
    for index in 0..64 {
        let state = Arc::clone(&state);
        let identity = if index % 2 == 0 {
            authorized.clone()
        } else {
            forged.clone()
        };
        let barrier = Arc::clone(&barrier);
        callers.push(tokio::spawn(async move {
            barrier.wait().await;
            state.dispatch_verified_request(
                &identity,
                ClientRequestEnvelope::new(
                    format!("identity-race-{index}"),
                    ClientRequest::Get {
                        ns: namespace(),
                        key: key(0),
                    },
                ),
            )
        }));
    }
    barrier.wait().await;
    for (index, caller) in callers.into_iter().enumerate() {
        let response = caller.await.unwrap();
        if index % 2 == 0 {
            assert!(matches!(
                response.result,
                Ok(ClientResponse::Value { value: Some(ref value) }) if value == &vec![9; 16]
            ));
        } else {
            assert_eq!(
                response.result.unwrap_err().code,
                ClientErrorCode::Unauthorized
            );
        }
    }
    assert_eq!(state.audit_events_for_tests().len(), 32);
}

fn batch_put(state: &ClientSurfaceState, identity: &ClientIdentity, round: usize, byte: u8) {
    let response = dispatch(
        state,
        identity,
        format!("batch-put-{round}"),
        ClientRequest::BatchPut {
            ns: namespace(),
            entries: (0..BATCH)
                .map(|index| BatchPutEntry {
                    key: key(index),
                    value: vec![byte; 16],
                })
                .collect(),
        },
    );
    assert!(matches!(response, ClientResponse::Batch { ref items } if items.len() == BATCH));
}

fn batch_get(state: &ClientSurfaceState, identity: &ClientIdentity, round: usize) -> Vec<Vec<u8>> {
    let response = dispatch(
        state,
        identity,
        format!("batch-get-{round}"),
        ClientRequest::BatchGet {
            ns: namespace(),
            keys: (0..BATCH).map(key).collect(),
        },
    );
    let ClientResponse::Batch { items } = response else {
        panic!("batch GET must return a batch");
    };
    items
        .into_iter()
        .map(|item| item.result.unwrap().expect("preloaded batch value"))
        .collect()
}

fn dispatch(
    state: &ClientSurfaceState,
    identity: &ClientIdentity,
    request_id: impl Into<String>,
    request: ClientRequest,
) -> ClientResponse {
    state
        .dispatch_verified_request(identity, ClientRequestEnvelope::new(request_id, request))
        .result
        .unwrap()
}

fn namespace() -> Namespace {
    Namespace::new("users").unwrap()
}

fn key(index: usize) -> StructuredKey {
    StructuredKey::new(vec![format!("key-{index:02}")]).unwrap()
}
