use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_client_protocol::{
    ClientErrorCode, ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, ClientWireMessage, LockConsistency, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, CLIENT_DATA_PATH, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
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
    let frame = ClientFrame::from_message(&ClientWireMessage::Request(ClientRequestEnvelope::new(
        format!("{client_id}-request"),
        request,
    )))
    .unwrap()
    .encode()
    .unwrap();
    let response = surface
        .routes()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_DATA_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, client_id)
                .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
                .body(Body::from(frame))
                .unwrap(),
        )
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
