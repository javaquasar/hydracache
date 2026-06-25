use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_client_protocol::{
    CasExpectation, ClientErrorCode, ClientFrame, ClientRequest, ClientRequestEnvelope,
    ClientResponse, ClientResponseEnvelope, ClientWireMessage, LockConsistency, Namespace,
    StructuredKey,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, CLIENT_DATA_PATH, HYDRACACHE_ADMIN_HEADER,
    HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_observability::AuditEvent;
use tower::ServiceExt;

fn ns() -> Namespace {
    Namespace::new("locks").unwrap()
}

fn key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["lock".to_owned(), id.to_owned()]).unwrap()
}

async fn send(
    surface: &AxumClientSurface,
    client_id: &str,
    request: ClientRequest,
) -> ClientResponseEnvelope {
    send_with_admin(surface, client_id, false, request).await
}

async fn send_with_admin(
    surface: &AxumClientSurface,
    client_id: &str,
    admin: bool,
    request: ClientRequest,
) -> ClientResponseEnvelope {
    let frame = ClientFrame::from_message(&ClientWireMessage::Request(ClientRequestEnvelope::new(
        format!("{client_id}-request"),
        request,
    )))
    .unwrap()
    .encode()
    .unwrap();
    let mut request = Request::builder()
        .method("POST")
        .uri(CLIENT_DATA_PATH)
        .header(HYDRACACHE_CLIENT_ID_HEADER, client_id)
        .header(HYDRACACHE_TENANT_HEADER, "tenant-a");
    if admin {
        request = request.header(HYDRACACHE_ADMIN_HEADER, "true");
    }
    let response = surface
        .routes()
        .oneshot(request.body(Body::from(frame)).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let message = ClientFrame::decode(&bytes, 1024 * 1024)
        .unwrap()
        .decode_message()
        .unwrap();
    let ClientWireMessage::Response(response) = message else {
        panic!("expected response message");
    };
    response
}

fn try_lock(level: LockConsistency, lease_ms: u64) -> ClientRequest {
    ClientRequest::TryLock {
        ns: ns(),
        key: key("user:42"),
        lease_ms,
        wait_ms: 0,
        level,
    }
}

#[tokio::test]
async fn two_clients_contend_one_wins_fence_monotonic() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();

    let first = send(&surface, "client-a", try_lock(LockConsistency::Quorum, 100))
        .await
        .result
        .unwrap();
    let ClientResponse::LockAcquired { fence: first_fence } = first else {
        panic!("expected first acquisition");
    };

    let busy = send(&surface, "client-b", try_lock(LockConsistency::Quorum, 100))
        .await
        .result
        .unwrap();
    assert_eq!(busy, ClientResponse::LockBusy);

    send(
        &surface,
        "client-a",
        ClientRequest::Unlock {
            ns: ns(),
            key: key("user:42"),
            fence: first_fence,
        },
    )
    .await
    .result
    .unwrap();

    let second = send(&surface, "client-b", try_lock(LockConsistency::Quorum, 100))
        .await
        .result
        .unwrap();
    let ClientResponse::LockAcquired {
        fence: second_fence,
    } = second
    else {
        panic!("expected second acquisition");
    };
    assert!(second_fence > first_fence);
}

#[tokio::test]
async fn lease_renew_extends_then_expiry_frees() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();

    let first = send(&surface, "client-a", try_lock(LockConsistency::Quorum, 5))
        .await
        .result
        .unwrap();
    let ClientResponse::LockAcquired { fence } = first else {
        panic!("expected acquisition");
    };
    send(
        &surface,
        "client-a",
        ClientRequest::RenewLockLease {
            ns: ns(),
            key: key("user:42"),
            fence,
            lease_ms: 50,
        },
    )
    .await
    .result
    .unwrap();

    surface.state().advance_lock_logical_time_for_tests(10);
    let still_busy = send(&surface, "client-b", try_lock(LockConsistency::Quorum, 5))
        .await
        .result
        .unwrap();
    assert_eq!(still_busy, ClientResponse::LockBusy);

    surface.state().advance_lock_logical_time_for_tests(50);
    let acquired = send(&surface, "client-b", try_lock(LockConsistency::Quorum, 5))
        .await
        .result
        .unwrap();
    let ClientResponse::LockAcquired { fence: next_fence } = acquired else {
        panic!("expected acquisition after expiry");
    };
    assert!(next_fence > fence);
}

#[tokio::test]
async fn weak_level_lock_returns_weakconsistency_envelope() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();

    let error = send(&surface, "client-a", try_lock(LockConsistency::One, 10))
        .await
        .result
        .unwrap_err();

    assert_eq!(error.code, ClientErrorCode::Conflict);
    assert!(error.message.contains("Quorum/EachQuorum/All"));
}

#[tokio::test]
async fn not_leader_forwards_or_redirects() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    surface.state().set_lock_leader_for_tests(2, 1);

    let error = send(&surface, "client-a", try_lock(LockConsistency::Quorum, 10))
        .await
        .result
        .unwrap_err();

    assert_eq!(error.code, ClientErrorCode::BackendUnavailable);
    assert!(error.retryable);
    assert_eq!(error.retry_after_ms, Some(1));
    assert!(error.message.contains("leader=1"));
}

#[tokio::test]
async fn force_unlock_requires_admin_and_audits() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let acquired = send(&surface, "client-a", try_lock(LockConsistency::Quorum, 100))
        .await
        .result
        .unwrap();
    assert!(matches!(acquired, ClientResponse::LockAcquired { .. }));

    let error = send(
        &surface,
        "client-b",
        ClientRequest::ForceUnlock {
            ns: ns(),
            key: key("user:42"),
        },
    )
    .await
    .result
    .unwrap_err();

    assert_eq!(error.code, ClientErrorCode::Unauthorized);
    let events = surface.state().audit_events_for_tests();
    assert!(matches!(
        events.as_slice(),
        [AuditEvent::AuthFailure {
            tenant: Some(_),
            route,
            request_id: Some(_)
        }] if route == CLIENT_DATA_PATH
    ));
    assert_eq!(
        send(&surface, "client-b", try_lock(LockConsistency::Quorum, 100))
            .await
            .result
            .unwrap(),
        ClientResponse::LockBusy
    );
}

#[tokio::test]
async fn force_unlock_advances_fence() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let acquired = send(&surface, "client-a", try_lock(LockConsistency::Quorum, 100))
        .await
        .result
        .unwrap();
    let ClientResponse::LockAcquired { fence } = acquired else {
        panic!("expected acquisition");
    };

    send_with_admin(
        &surface,
        "admin-a",
        true,
        ClientRequest::ForceUnlock {
            ns: ns(),
            key: key("user:42"),
        },
    )
    .await
    .result
    .unwrap();

    let reacquired = send(&surface, "client-b", try_lock(LockConsistency::Quorum, 100))
        .await
        .result
        .unwrap();
    let ClientResponse::LockAcquired { fence: next_fence } = reacquired else {
        panic!("expected acquisition after force unlock");
    };
    assert!(next_fence > fence);
    assert!(surface
        .state()
        .audit_events_for_tests()
        .iter()
        .any(|event| matches!(
            event,
            AuditEvent::PolicyChanged { summary, .. } if summary == "force_unlock"
        )));
}

#[tokio::test]
async fn compare_and_set_matching_old_updates_visible_value() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    send(
        &surface,
        "client-a",
        ClientRequest::Put {
            ns: ns(),
            key: key("user:42"),
            value: b"old".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    )
    .await
    .result
    .unwrap();

    let applied = send(
        &surface,
        "client-a",
        ClientRequest::CompareAndSet {
            ns: ns(),
            key: key("user:42"),
            expected: CasExpectation::Exact(b"old".to_vec()),
            new_value: b"new".to_vec(),
            level: LockConsistency::Quorum,
        },
    )
    .await
    .result
    .unwrap();
    assert!(matches!(applied, ClientResponse::CasApplied { .. }));

    let value = send(
        &surface,
        "client-a",
        ClientRequest::Get {
            ns: ns(),
            key: key("user:42"),
        },
    )
    .await
    .result
    .unwrap();
    assert_eq!(
        value,
        ClientResponse::Value {
            value: Some(b"new".to_vec())
        }
    );
}

#[tokio::test]
async fn compare_and_set_stale_old_returns_current_without_update() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    send(
        &surface,
        "client-a",
        ClientRequest::Put {
            ns: ns(),
            key: key("user:42"),
            value: b"current".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    )
    .await
    .result
    .unwrap();

    let mismatch = send(
        &surface,
        "client-a",
        ClientRequest::CompareAndSet {
            ns: ns(),
            key: key("user:42"),
            expected: CasExpectation::Exact(b"stale".to_vec()),
            new_value: b"new".to_vec(),
            level: LockConsistency::Quorum,
        },
    )
    .await
    .result
    .unwrap();

    assert_eq!(
        mismatch,
        ClientResponse::CasMismatch {
            current: Some(b"current".to_vec())
        }
    );
}

#[tokio::test]
async fn remove_if_value_match_writes_tombstone_visible_as_absent() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    send(
        &surface,
        "client-a",
        ClientRequest::Put {
            ns: ns(),
            key: key("user:42"),
            value: b"current".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    )
    .await
    .result
    .unwrap();

    let removed = send(
        &surface,
        "client-a",
        ClientRequest::RemoveIfValue {
            ns: ns(),
            key: key("user:42"),
            expected: b"current".to_vec(),
            level: LockConsistency::Quorum,
        },
    )
    .await
    .result
    .unwrap();
    assert!(matches!(removed, ClientResponse::CasApplied { .. }));

    let value = send(
        &surface,
        "client-a",
        ClientRequest::Get {
            ns: ns(),
            key: key("user:42"),
        },
    )
    .await
    .result
    .unwrap();
    assert_eq!(value, ClientResponse::Value { value: None });
}

#[tokio::test]
async fn replace_if_present_on_absent_is_mismatch_not_inserted() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();

    let mismatch = send(
        &surface,
        "client-a",
        ClientRequest::CompareAndSet {
            ns: ns(),
            key: key("missing"),
            expected: CasExpectation::Present,
            new_value: b"created".to_vec(),
            level: LockConsistency::Quorum,
        },
    )
    .await
    .result
    .unwrap();
    assert_eq!(mismatch, ClientResponse::CasMismatch { current: None });

    let value = send(
        &surface,
        "client-a",
        ClientRequest::Get {
            ns: ns(),
            key: key("missing"),
        },
    )
    .await
    .result
    .unwrap();
    assert_eq!(value, ClientResponse::Value { value: None });
}
