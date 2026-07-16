use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    CasExpectation, ClientErrorCode, ClientRequest, ClientRequestEnvelope, ClientResponse,
    LockConsistency, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientIdentity, ClientSurfaceLimits, ClientSurfaceState, CLIENT_DATA_PATH,
    CLIENT_STATUS_PATH, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_cluster_testkit::client_surface_conformance::{
    self as conformance, ClientSurfaceBackend, ClientSurfaceBackendFactory,
    ClientSurfaceConformanceConfig, ConformanceIdentity, ConformanceResult,
};
use hydracache_observability::AuditEvent;
use tokio::sync::Barrier;
use tower::ServiceExt;

#[derive(Debug, Clone, Copy)]
struct LocalFactory;

#[derive(Debug)]
struct LocalBackend {
    state: Arc<ClientSurfaceState>,
}

#[async_trait]
impl ClientSurfaceBackend for LocalBackend {
    async fn execute(
        &self,
        identity: &ConformanceIdentity,
        request: hydracache_client_protocol::ClientRequestEnvelope,
    ) -> ConformanceResult<hydracache_client_protocol::ClientResponseEnvelope> {
        let identity = ClientIdentity::new(&identity.client_id, &identity.tenant_id)?;
        Ok(self.state.dispatch_verified_request(&identity, request))
    }

    async fn freeze_time(&self, now_ms: u64) -> ConformanceResult<()> {
        self.state.set_cache_time_for_tests(Some(now_ms));
        Ok(())
    }

    async fn advance_time(&self, millis: u64) -> ConformanceResult<()> {
        self.state.advance_cache_time_for_tests(millis);
        Ok(())
    }
}

fn local_state(
    config: ClientSurfaceConformanceConfig,
) -> ConformanceResult<Arc<ClientSurfaceState>> {
    let ClientSurfaceConformanceConfig {
        limits,
        tenants: configured_tenants,
        now_ms,
    } = config;
    let tenants = configured_tenants
        .into_iter()
        .map(|configured| {
            let mut tenant = Tenant::new(configured.tenant_id)?;
            for client_id in configured.client_ids {
                tenant = tenant.allow_client(client_id);
            }
            for namespace in configured.namespaces {
                tenant = tenant.namespace(
                    namespace.name,
                    NamespaceQuota::new(namespace.max_bytes, namespace.max_entries),
                );
            }
            Ok(tenant)
        })
        .collect::<Result<Vec<_>, hydracache::MultitenancyError>>()?;
    let isolation = ConsumerIsolation::new(
        TenantRoster::new(tenants)?,
        ConsumerIsolationConfig {
            max_value_bytes: limits.max_value_bytes as u64,
            max_request_bytes: limits.max_batch_bytes as u64,
            max_batch_items: limits.max_batch_entries,
        },
    );
    let state = Arc::new(ClientSurfaceState::with_isolation(
        ClientSurfaceLimits {
            max_value_bytes: limits.max_value_bytes,
            max_batch_entries: limits.max_batch_entries,
            max_batch_bytes: limits.max_batch_bytes,
            ..ClientSurfaceLimits::default()
        },
        isolation,
    )?);
    state.set_cache_time_for_tests(Some(now_ms));
    Ok(state)
}

fn namespace() -> Namespace {
    Namespace::new("users").unwrap()
}

fn key(name: &str) -> StructuredKey {
    StructuredKey::new(vec![name.to_owned()]).unwrap()
}

fn dispatch(
    state: &ClientSurfaceState,
    identity: &ClientIdentity,
    request_id: &str,
    request: ClientRequest,
) -> ClientResponse {
    state
        .dispatch_verified_request(identity, ClientRequestEnvelope::new(request_id, request))
        .result
        .unwrap()
}

fn put(
    state: &ClientSurfaceState,
    identity: &ClientIdentity,
    request_id: &str,
    key_name: &str,
    value: Vec<u8>,
    ttl_ms: Option<u64>,
) -> ClientResponse {
    dispatch(
        state,
        identity,
        request_id,
        ClientRequest::Put {
            ns: namespace(),
            key: key(key_name),
            value,
            ttl_ms,
            dimensions: Vec::new(),
        },
    )
}

fn get(
    state: &ClientSurfaceState,
    identity: &ClientIdentity,
    request_id: &str,
    key_name: &str,
) -> Option<Vec<u8>> {
    match dispatch(
        state,
        identity,
        request_id,
        ClientRequest::Get {
            ns: namespace(),
            key: key(key_name),
        },
    ) {
        ClientResponse::Value { value } => value,
        response => panic!("expected value response, got {response:?}"),
    }
}

fn tenant_usage(state: &ClientSurfaceState, identity: &ClientIdentity) -> (u64, u64) {
    let status = state.tenant_status(identity).unwrap();
    let users = status
        .namespaces
        .iter()
        .find(|namespace| namespace.namespace == "users")
        .unwrap();
    (users.bytes, users.entries)
}

fn two_tenant_config(max_bytes: u64, max_entries: u64) -> ClientSurfaceConformanceConfig {
    let mut config = ClientSurfaceConformanceConfig::single_tenant(
        conformance::ConformanceLimits::default(),
        max_bytes,
        max_entries,
    );
    config
        .tenants
        .push(conformance::ConformanceTenant::single_namespace(
            "tenant-b",
            "client-b",
            conformance::ConformanceNamespace::new("users", max_bytes, max_entries),
        ));
    config
}

#[async_trait]
impl ClientSurfaceBackendFactory for LocalFactory {
    async fn create(
        &self,
        config: ClientSurfaceConformanceConfig,
    ) -> ConformanceResult<Arc<dyn ClientSurfaceBackend>> {
        let state = local_state(config)?;
        Ok(Arc::new(LocalBackend { state }))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_conditional_put_if_absent_is_atomic_under_n_concurrent_acquirers() {
    conformance::assert_conditional_put_if_absent_is_atomic_under_n_concurrent_acquirers(
        &LocalFactory,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn conformance_conditional_put_treats_expired_key_as_absent() {
    conformance::assert_conditional_put_treats_expired_key_as_absent(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_compare_value_invalidate_is_token_safe_and_returns_applied_count() {
    conformance::assert_compare_value_invalidate_is_token_safe_and_returns_applied_count(
        &LocalFactory,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn conformance_compare_value_expire_add_to_remaining_and_replace_if_expiring_and_persistent_guard(
) {
    conformance::assert_compare_value_expire_add_to_remaining_and_replace_if_expiring_and_persistent_guard(
        &LocalFactory,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn conformance_batch_put_is_all_or_nothing_under_prevalidation_failure() {
    conformance::assert_batch_put_is_all_or_nothing_under_prevalidation_failure(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_ttl_states_missing_persistent_expiring_round_trip() {
    conformance::assert_ttl_states_missing_persistent_expiring_round_trip(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_expired_key_absent_for_get_and_batch_get() {
    conformance::assert_expired_key_absent_for_get_and_batch_get(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_enforces_value_bytes_batch_and_tenant_quota_limits() {
    conformance::assert_enforces_value_bytes_batch_and_tenant_quota_limits(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_rejected_conditionals_and_batches_do_not_reserve_quota() {
    conformance::assert_rejected_conditionals_and_batches_do_not_reserve_quota(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_delete_and_expiry_release_tenant_quota() {
    conformance::assert_delete_and_expiry_release_tenant_quota(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_duplicate_batch_keys_account_last_write_only() {
    conformance::assert_duplicate_batch_keys_account_last_write_only(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_tenant_binding_and_same_namespace_keys_are_isolated() {
    conformance::assert_tenant_binding_and_same_namespace_keys_are_isolated(&LocalFactory)
        .await
        .unwrap();
}

#[tokio::test]
async fn conformance_batch_entry_and_byte_limits_reject_at_boundary_plus_one_without_mutation() {
    conformance::assert_batch_entry_and_byte_limits_reject_at_boundary_plus_one_without_mutation(
        &LocalFactory,
    )
    .await
    .unwrap();
}

#[test]
fn quota_rejection_records_exactly_one_redacted_audit_event() {
    let state = local_state(ClientSurfaceConformanceConfig::single_tenant(
        conformance::ConformanceLimits {
            max_value_bytes: 6,
            ..conformance::ConformanceLimits::default()
        },
        2,
        1,
    ))
    .unwrap();
    let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
    let rejected = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "quota-rejected",
            ClientRequest::Put {
                ns: Namespace::new("users").unwrap(),
                key: StructuredKey::new(vec!["secret-key".to_owned()]).unwrap(),
                value: vec![7; 3],
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ),
    );
    assert_eq!(
        rejected.result.unwrap_err().code,
        ClientErrorCode::TenantQuota
    );
    assert_eq!(
        state.audit_events_for_tests(),
        vec![AuditEvent::QuotaRejected {
            tenant: "tenant-a".to_owned(),
            namespace: "users".to_owned(),
            request_id: Some("quota-rejected".to_owned()),
        }]
    );

    let accepted = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "accepted",
            ClientRequest::Put {
                ns: Namespace::new("users").unwrap(),
                key: StructuredKey::new(vec!["ordinary".to_owned()]).unwrap(),
                value: vec![1; 2],
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ),
    );
    assert_eq!(accepted.result.unwrap(), ClientResponse::Stored);
    assert_eq!(state.audit_events_for_tests().len(), 1);
}

#[test]
fn tenant_binding_rejection_records_exactly_one_auth_failure() {
    let state = local_state(ClientSurfaceConformanceConfig::default()).unwrap();
    let forged = ClientIdentity::new("client-a", "tenant-b").unwrap();
    let response = state.dispatch_verified_request(
        &forged,
        ClientRequestEnvelope::new(
            "forged-binding",
            ClientRequest::Get {
                ns: Namespace::new("users").unwrap(),
                key: StructuredKey::new(vec!["key".to_owned()]).unwrap(),
            },
        ),
    );
    assert_eq!(
        response.result.unwrap_err().code,
        ClientErrorCode::Unauthorized
    );
    assert_eq!(
        state.audit_events_for_tests(),
        vec![AuditEvent::AuthFailure {
            tenant: Some("tenant-b".to_owned()),
            route: CLIENT_DATA_PATH.to_owned(),
            request_id: Some("forged-binding".to_owned()),
        }]
    );
}

#[test]
fn force_unlock_auth_failure_is_not_double_recorded_by_dispatch_audit() {
    let state = local_state(ClientSurfaceConformanceConfig::default()).unwrap();
    let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
    let response = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "force-unlock",
            ClientRequest::ForceUnlock {
                ns: Namespace::new("users").unwrap(),
                key: StructuredKey::new(vec!["lock".to_owned()]).unwrap(),
            },
        ),
    );
    assert_eq!(
        response.result.unwrap_err().code,
        ClientErrorCode::Unauthorized
    );
    assert_eq!(
        state.audit_events_for_tests(),
        vec![AuditEvent::AuthFailure {
            tenant: Some("tenant-a".to_owned()),
            route: CLIENT_DATA_PATH.to_owned(),
            request_id: Some("force-unlock".to_owned()),
        }]
    );
}

#[test]
fn client_surface_native_lock_cas_and_idempotency_are_tenant_scoped() {
    let state = local_state(two_tenant_config(128, 16)).unwrap();
    let tenant_a = ClientIdentity::new("client-a", "tenant-a").unwrap();
    let tenant_b = ClientIdentity::new("client-b", "tenant-b").unwrap();

    let fence_a = match dispatch(
        &state,
        &tenant_a,
        "lock-a",
        ClientRequest::TryLock {
            ns: namespace(),
            key: key("same-lock"),
            lease_ms: 1_000,
            wait_ms: 0,
            level: LockConsistency::Quorum,
        },
    ) {
        ClientResponse::LockAcquired { fence } => fence,
        response => panic!("tenant A should acquire its lock, got {response:?}"),
    };
    let fence_b = match dispatch(
        &state,
        &tenant_b,
        "lock-b",
        ClientRequest::TryLock {
            ns: namespace(),
            key: key("same-lock"),
            lease_ms: 1_000,
            wait_ms: 0,
            level: LockConsistency::Quorum,
        },
    ) {
        ClientResponse::LockAcquired { fence } => fence,
        response => panic!("tenant B should acquire its independent lock, got {response:?}"),
    };
    assert!(matches!(
        dispatch(
            &state,
            &tenant_a,
            "unlock-a",
            ClientRequest::Unlock {
                ns: namespace(),
                key: key("same-lock"),
                fence: fence_a,
            },
        ),
        ClientResponse::LockReleased
    ));
    assert_eq!(
        dispatch(
            &state,
            &tenant_b,
            "ownership-b",
            ClientRequest::GetLockOwnership {
                ns: namespace(),
                key: key("same-lock"),
            },
        ),
        ClientResponse::LockOwnership {
            fence: Some(fence_b),
            locked: true,
        }
    );

    assert_eq!(
        put(
            &state,
            &tenant_a,
            "cas-seed-a",
            "same-cas",
            b"shared-token".to_vec(),
            None,
        ),
        ClientResponse::Stored
    );
    assert_eq!(
        put(
            &state,
            &tenant_b,
            "cas-seed-b",
            "same-cas",
            b"shared-token".to_vec(),
            None,
        ),
        ClientResponse::Stored
    );
    assert!(matches!(
        dispatch(
            &state,
            &tenant_a,
            "cas-a",
            ClientRequest::CompareAndSet {
                ns: namespace(),
                key: key("same-cas"),
                expected: CasExpectation::Exact(b"shared-token".to_vec()),
                new_value: b"tenant-a-value".to_vec(),
                level: LockConsistency::Quorum,
            },
        ),
        ClientResponse::CasApplied { .. }
    ));
    assert!(matches!(
        dispatch(
            &state,
            &tenant_b,
            "cas-b",
            ClientRequest::CompareAndSet {
                ns: namespace(),
                key: key("same-cas"),
                expected: CasExpectation::Exact(b"shared-token".to_vec()),
                new_value: b"tenant-b-value".to_vec(),
                level: LockConsistency::Quorum,
            },
        ),
        ClientResponse::CasApplied { .. }
    ));
    assert_eq!(
        get(&state, &tenant_a, "get-cas-a", "same-cas"),
        Some(b"tenant-a-value".to_vec())
    );
    assert_eq!(
        get(&state, &tenant_b, "get-cas-b", "same-cas"),
        Some(b"tenant-b-value".to_vec())
    );

    let idem_a = ClientRequestEnvelope::new(
        "idem-a",
        ClientRequest::Put {
            ns: namespace(),
            key: key("idem"),
            value: b"tenant-a-idem".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    )
    .with_idempotency_key("shared-idempotency-key");
    let idem_b = ClientRequestEnvelope::new(
        "idem-b",
        ClientRequest::Put {
            ns: namespace(),
            key: key("idem"),
            value: b"tenant-b-idem".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    )
    .with_idempotency_key("shared-idempotency-key");
    assert_eq!(
        state
            .dispatch_verified_request(&tenant_a, idem_a)
            .result
            .unwrap(),
        ClientResponse::Stored
    );
    assert_eq!(
        state
            .dispatch_verified_request(&tenant_b, idem_b)
            .result
            .unwrap(),
        ClientResponse::Stored
    );
    assert_eq!(
        get(&state, &tenant_b, "get-idem-b", "idem"),
        Some(b"tenant-b-idem".to_vec())
    );
}

#[test]
fn client_surface_cas_mismatch_does_not_reserve_quota() {
    let state = local_state(ClientSurfaceConformanceConfig::single_tenant(
        conformance::ConformanceLimits {
            max_value_bytes: 4,
            ..conformance::ConformanceLimits::default()
        },
        4,
        2,
    ))
    .unwrap();
    let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
    assert_eq!(
        put(&state, &identity, "seed", "cas", vec![1; 2], None),
        ClientResponse::Stored
    );
    assert_eq!(
        dispatch(
            &state,
            &identity,
            "cas-mismatch",
            ClientRequest::CompareAndSet {
                ns: namespace(),
                key: key("cas"),
                expected: CasExpectation::Exact(vec![9; 2]),
                new_value: vec![2; 4],
                level: LockConsistency::Quorum,
            },
        ),
        ClientResponse::CasMismatch {
            current: Some(vec![1; 2]),
        }
    );
    assert_eq!(
        put(&state, &identity, "remaining", "other", vec![3; 2], None),
        ClientResponse::Stored
    );
    assert_eq!(get(&state, &identity, "get-cas", "cas"), Some(vec![1; 2]));
    assert_eq!(tenant_usage(&state, &identity), (4, 2));
    assert!(state.audit_events_for_tests().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn client_surface_concurrent_put_overwrite_keeps_store_and_quota_consistent() {
    for round in 0..16 {
        let state = local_state(ClientSurfaceConformanceConfig::single_tenant(
            conformance::ConformanceLimits::default(),
            64,
            1,
        ))
        .unwrap();
        let contenders = 16;
        let barrier = Arc::new(Barrier::new(contenders));
        let mut tasks = Vec::new();
        for size in 1..=contenders {
            let state = Arc::clone(&state);
            let barrier = Arc::clone(&barrier);
            tasks.push(tokio::spawn(async move {
                let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
                barrier.wait().await;
                put(
                    &state,
                    &identity,
                    &format!("round-{round}-put-{size}"),
                    "race",
                    vec![size as u8; size],
                    None,
                )
            }));
        }
        for task in tasks {
            assert_eq!(task.await.unwrap(), ClientResponse::Stored);
        }
        let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
        let stored = get(&state, &identity, "race-result", "race").unwrap();
        assert_eq!(tenant_usage(&state, &identity), (stored.len() as u64, 1));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn client_surface_concurrent_put_delete_keeps_store_and_quota_consistent() {
    for round in 0..32 {
        let state = local_state(ClientSurfaceConformanceConfig::single_tenant(
            conformance::ConformanceLimits::default(),
            4,
            1,
        ))
        .unwrap();
        let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
        assert_eq!(
            put(&state, &identity, "seed", "race", vec![1; 4], None),
            ClientResponse::Stored
        );
        let barrier = Arc::new(Barrier::new(2));
        let put_task = {
            let state = Arc::clone(&state);
            let barrier = Arc::clone(&barrier);
            tokio::spawn(async move {
                let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
                barrier.wait().await;
                put(
                    &state,
                    &identity,
                    &format!("round-{round}-put"),
                    "race",
                    vec![2; 4],
                    None,
                )
            })
        };
        let delete_task = {
            let state = Arc::clone(&state);
            let barrier = Arc::clone(&barrier);
            tokio::spawn(async move {
                let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
                barrier.wait().await;
                dispatch(
                    &state,
                    &identity,
                    &format!("round-{round}-delete"),
                    ClientRequest::Invalidate {
                        ns: namespace(),
                        key: key("race"),
                    },
                )
            })
        };
        assert_eq!(put_task.await.unwrap(), ClientResponse::Stored);
        assert_eq!(delete_task.await.unwrap(), ClientResponse::Invalidated);
        let value = get(&state, &identity, "race-result", "race");
        assert_eq!(
            tenant_usage(&state, &identity),
            if value.is_some() { (4, 1) } else { (0, 0) }
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn client_surface_concurrent_expiry_and_overwrite_keeps_store_and_quota_consistent() {
    for round in 0..32 {
        let state = local_state(ClientSurfaceConformanceConfig::single_tenant(
            conformance::ConformanceLimits::default(),
            4,
            1,
        ))
        .unwrap();
        let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
        assert_eq!(
            put(
                &state,
                &identity,
                "seed-expiring",
                "race",
                vec![1; 4],
                Some(1),
            ),
            ClientResponse::Stored
        );
        state.advance_cache_time_for_tests(1);
        let barrier = Arc::new(Barrier::new(2));
        let get_task = {
            let state = Arc::clone(&state);
            let barrier = Arc::clone(&barrier);
            tokio::spawn(async move {
                let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
                barrier.wait().await;
                get(&state, &identity, &format!("round-{round}-expire"), "race")
            })
        };
        let put_task = {
            let state = Arc::clone(&state);
            let barrier = Arc::clone(&barrier);
            tokio::spawn(async move {
                let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
                barrier.wait().await;
                put(
                    &state,
                    &identity,
                    &format!("round-{round}-overwrite"),
                    "race",
                    vec![2; 4],
                    None,
                )
            })
        };
        let _ = get_task.await.unwrap();
        assert_eq!(put_task.await.unwrap(), ClientResponse::Stored);
        assert_eq!(
            get(&state, &identity, "expiry-race-result", "race"),
            Some(vec![2; 4])
        );
        assert_eq!(tenant_usage(&state, &identity), (4, 1));
    }
}

#[tokio::test]
async fn http_tenant_binding_rejection_records_exactly_one_auth_failure() {
    let state = local_state(ClientSurfaceConformanceConfig::default()).unwrap();
    let routes = AxumClientSurface::from_state(Arc::clone(&state)).routes();
    let data_response = routes
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_DATA_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
                .header(HYDRACACHE_TENANT_HEADER, "tenant-b")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(data_response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        state.audit_events_for_tests(),
        vec![AuditEvent::AuthFailure {
            tenant: Some("tenant-b".to_owned()),
            route: CLIENT_DATA_PATH.to_owned(),
            request_id: None,
        }]
    );

    let status_response = routes
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(CLIENT_STATUS_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
                .header(HYDRACACHE_TENANT_HEADER, "tenant-b")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::FORBIDDEN);
    assert_eq!(state.audit_events_for_tests().len(), 2);
    assert_eq!(
        state.audit_events_for_tests()[1],
        AuditEvent::AuthFailure {
            tenant: Some("tenant-b".to_owned()),
            route: CLIENT_STATUS_PATH.to_owned(),
            request_id: None,
        }
    );
}
