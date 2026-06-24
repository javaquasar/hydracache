use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    BatchPutEntry, ClientErrorCode, ClientFrame, ClientRequest, ClientRequestEnvelope,
    ClientResponse, ClientResponseEnvelope, ClientWireMessage, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, CLIENT_DATA_PATH, CLIENT_STATUS_PATH,
    HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use tower::ServiceExt;

fn ns() -> Namespace {
    Namespace::new("users").unwrap()
}

fn key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["user".to_owned(), id.to_owned()]).unwrap()
}

fn isolated_surface(max_bytes: u64) -> AxumClientSurface {
    let roster = TenantRoster::new(vec![Tenant::new("tenant-a")
        .unwrap()
        .allow_client("client-a")
        .namespace("users", NamespaceQuota::new(max_bytes, 8))])
    .unwrap();
    AxumClientSurface::with_isolation(
        ClientSurfaceLimits::default(),
        ConsumerIsolation::new(roster, ConsumerIsolationConfig::default()),
    )
    .unwrap()
}

async fn send(
    surface: &AxumClientSurface,
    envelope: ClientRequestEnvelope,
) -> ClientResponseEnvelope {
    let frame = ClientFrame::from_message(&ClientWireMessage::Request(envelope))
        .unwrap()
        .encode()
        .unwrap();
    let response = surface
        .routes()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_DATA_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
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

#[tokio::test]
async fn client_surface_get_put_invalidate_round_trip() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();

    let put = ClientRequestEnvelope::new(
        "put-1",
        ClientRequest::Put {
            ns: ns(),
            key: key("42"),
            value: b"profile".to_vec(),
            ttl_ms: None,
            dimensions: vec!["tenant".to_owned(), "core".to_owned()],
        },
    );
    assert!(matches!(
        send(&surface, put).await.result.unwrap(),
        ClientResponse::Stored
    ));

    let get = ClientRequestEnvelope::new(
        "get-1",
        ClientRequest::Get {
            ns: ns(),
            key: key("42"),
        },
    );
    let ClientResponse::Value { value } = send(&surface, get).await.result.unwrap() else {
        panic!("expected value response");
    };
    assert_eq!(value, Some(b"profile".to_vec()));

    let invalidate = ClientRequestEnvelope::new(
        "invalidate-1",
        ClientRequest::Invalidate {
            ns: ns(),
            key: key("42"),
        },
    );
    assert!(matches!(
        send(&surface, invalidate).await.result.unwrap(),
        ClientResponse::Invalidated
    ));

    let get = ClientRequestEnvelope::new(
        "get-2",
        ClientRequest::Get {
            ns: ns(),
            key: key("42"),
        },
    );
    let ClientResponse::Value { value } = send(&surface, get).await.result.unwrap() else {
        panic!("expected value response");
    };
    assert!(value.is_none());
}

#[tokio::test]
async fn client_surface_batch_partial_failures_preserve_order_and_item_status() {
    let limits = ClientSurfaceLimits {
        max_value_bytes: 4,
        ..ClientSurfaceLimits::default()
    };
    let surface = AxumClientSurface::new(limits).unwrap();
    let batch = ClientRequestEnvelope::new(
        "batch-1",
        ClientRequest::BatchPut {
            ns: ns(),
            entries: vec![
                BatchPutEntry {
                    key: key("1"),
                    value: b"ok".to_vec(),
                },
                BatchPutEntry {
                    key: key("2"),
                    value: b"too-large".to_vec(),
                },
            ],
        },
    );

    let ClientResponse::Batch { items } = send(&surface, batch).await.result.unwrap() else {
        panic!("expected batch response");
    };

    assert_eq!(items[0].index, 0);
    assert!(items[0].result.is_ok());
    assert_eq!(items[1].index, 1);
    assert_eq!(
        items[1].result.as_ref().unwrap_err().code,
        ClientErrorCode::TooLarge
    );
}

#[tokio::test]
async fn client_surface_deadline_and_idempotency_are_honored() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let expired = ClientRequestEnvelope::new(
        "expired-1",
        ClientRequest::Get {
            ns: ns(),
            key: key("42"),
        },
    )
    .with_deadline_ms(0);

    let error = send(&surface, expired).await.result.unwrap_err();
    assert_eq!(error.code, ClientErrorCode::DeadlineExceeded);
    assert!(error.retryable);

    let put = || {
        ClientRequestEnvelope::new(
            "put-idem",
            ClientRequest::Put {
                ns: ns(),
                key: key("42"),
                value: b"profile".to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        )
        .with_idempotency_key("idem-1")
    };

    assert!(send(&surface, put()).await.result.is_ok());
    assert!(send(&surface, put()).await.result.is_ok());
    assert_eq!(surface.state().state_mutations(), 1);
}

#[tokio::test]
async fn client_surface_remote_request_respects_authority_and_fence() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let mut request = ClientRequestEnvelope::new(
        "get-fenced",
        ClientRequest::Get {
            ns: ns(),
            key: key("42"),
        },
    );
    request.protocol_version = 999;

    let error = send(&surface, request).await.result.unwrap_err();
    assert_eq!(error.code, ClientErrorCode::IncompatibleVersion);
    assert_eq!(surface.state().state_mutations(), 0);
}

#[tokio::test]
async fn client_surface_tenant_quota_returns_retryable_backpressure() {
    let surface = isolated_surface(4);
    let put = |request_id: &str, id: &str, value: &'static [u8]| {
        ClientRequestEnvelope::new(
            request_id,
            ClientRequest::Put {
                ns: ns(),
                key: key(id),
                value: value.to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        )
    };

    assert!(matches!(
        send(&surface, put("put-1", "1", b"okay"))
            .await
            .result
            .unwrap(),
        ClientResponse::Stored
    ));

    let error = send(&surface, put("put-2", "2", b"nope!"))
        .await
        .result
        .unwrap_err();
    assert_eq!(error.code, ClientErrorCode::TenantQuota);
    assert!(error.retryable);
    assert!(error.retry_after_ms.is_some());
    assert_eq!(surface.state().state_mutations(), 1);
}

#[tokio::test]
async fn client_surface_status_is_scoped_to_verified_tenant() {
    let surface = isolated_surface(100);
    let put = ClientRequestEnvelope::new(
        "put-status",
        ClientRequest::Put {
            ns: ns(),
            key: key("42"),
            value: b"profile".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    );
    assert!(send(&surface, put).await.result.is_ok());

    let response = surface
        .routes()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(CLIENT_STATUS_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
                .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["tenant"], "tenant-a");
    assert_eq!(json["tenant_status"]["tenant"], "tenant-a");
    assert_eq!(json["tenant_status"]["namespaces"][0]["namespace"], "users");
    assert_eq!(json["tenant_status"]["namespaces"][0]["bytes"], 7);
    assert!(!json.to_string().contains("tenant-b"));

    let forbidden = surface
        .routes()
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
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}
