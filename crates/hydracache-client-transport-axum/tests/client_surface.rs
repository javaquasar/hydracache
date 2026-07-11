use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    BatchPutEntry, ClientErrorCode, ClientFrame, ClientRequest, ClientRequestEnvelope,
    ClientResponse, ClientResponseEnvelope, ClientWireMessage, ConditionalPutCondition,
    EntryEventProjection, Namespace, StructuredKey, TtlState, Watermark, PROTOCOL_VERSION,
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
    send_with_frame_version(surface, envelope, PROTOCOL_VERSION)
        .await
        .1
}

async fn send_with_frame_version(
    surface: &AxumClientSurface,
    envelope: ClientRequestEnvelope,
    frame_protocol_version: u16,
) -> (u16, ClientResponseEnvelope) {
    let frame = ClientFrame::from_message_with_version(
        frame_protocol_version,
        &ClientWireMessage::Request(envelope),
    )
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
    let frame = ClientFrame::decode(&bytes, 1024 * 1024).unwrap();
    let response_protocol_version = frame.protocol_version();
    let message = frame.decode_message().unwrap();
    let ClientWireMessage::Response(response) = message else {
        panic!("expected response message");
    };
    (response_protocol_version, response)
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

    let batch = ClientRequestEnvelope::new(
        "batch-expired",
        ClientRequest::BatchGet {
            ns: ns(),
            keys: vec![key("ttl")],
        },
    );
    let ClientResponse::Batch { items } = send(&surface, batch).await.result.unwrap() else {
        panic!("expected batch response");
    };
    assert_eq!(items[0].result.as_ref().unwrap(), &None);
}

#[tokio::test]
async fn set_ex_and_px_apply_expiry_through_client_surface() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    surface.state().set_cache_time_for_tests(Some(1_000));

    let put = ClientRequestEnvelope::new(
        "put-ttl",
        ClientRequest::Put {
            ns: ns(),
            key: key("ttl"),
            value: b"short".to_vec(),
            ttl_ms: Some(100),
            dimensions: Vec::new(),
        },
    );
    assert!(matches!(
        send(&surface, put).await.result.unwrap(),
        ClientResponse::Stored
    ));

    let ttl = ClientRequestEnvelope::new(
        "ttl-1",
        ClientRequest::GetTtl {
            ns: ns(),
            key: key("ttl"),
        },
    );
    let ClientResponse::Ttl {
        state: TtlState::ExpiresIn { ttl_ms },
    } = send(&surface, ttl).await.result.unwrap()
    else {
        panic!("expected expiring TTL response");
    };
    assert_eq!(ttl_ms, 100);

    surface.state().advance_cache_time_for_tests(101);
    let get = ClientRequestEnvelope::new(
        "get-expired",
        ClientRequest::Get {
            ns: ns(),
            key: key("ttl"),
        },
    );
    let ClientResponse::Value { value } = send(&surface, get).await.result.unwrap() else {
        panic!("expected value response");
    };
    assert!(value.is_none());
}

#[tokio::test]
async fn expire_pexpire_persist_and_ttl_pttl_match_redis_semantics() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    surface.state().set_cache_time_for_tests(Some(10_000));

    let put = ClientRequestEnvelope::new(
        "put-persistent",
        ClientRequest::Put {
            ns: ns(),
            key: key("ttl-meta"),
            value: b"value".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    );
    assert!(send(&surface, put).await.result.is_ok());

    let ttl = || {
        ClientRequestEnvelope::new(
            "ttl-meta",
            ClientRequest::GetTtl {
                ns: ns(),
                key: key("ttl-meta"),
            },
        )
    };
    assert_eq!(
        send(&surface, ttl()).await.result.unwrap(),
        ClientResponse::Ttl {
            state: TtlState::Persistent
        }
    );

    let expire = ClientRequestEnvelope::new(
        "expire",
        ClientRequest::Expire {
            ns: ns(),
            key: key("ttl-meta"),
            ttl_ms: 250,
        },
    );
    assert_eq!(
        send(&surface, expire).await.result.unwrap(),
        ClientResponse::Expiry { applied: true }
    );

    surface.state().advance_cache_time_for_tests(50);
    let ClientResponse::Ttl {
        state: TtlState::ExpiresIn { ttl_ms },
    } = send(&surface, ttl()).await.result.unwrap()
    else {
        panic!("expected expiring TTL response");
    };
    assert_eq!(ttl_ms, 200);

    let persist = ClientRequestEnvelope::new(
        "persist",
        ClientRequest::Persist {
            ns: ns(),
            key: key("ttl-meta"),
        },
    );
    assert_eq!(
        send(&surface, persist).await.result.unwrap(),
        ClientResponse::Expiry { applied: true }
    );
    assert_eq!(
        send(&surface, ttl()).await.result.unwrap(),
        ClientResponse::Ttl {
            state: TtlState::Persistent
        }
    );

    let missing = ClientRequestEnvelope::new(
        "ttl-missing",
        ClientRequest::GetTtl {
            ns: ns(),
            key: key("missing"),
        },
    );
    assert_eq!(
        send(&surface, missing).await.result.unwrap(),
        ClientResponse::Ttl {
            state: TtlState::Missing
        }
    );
}

#[tokio::test]
async fn protocol_v2_clients_do_not_receive_v3_ttl_shapes() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let mut request = ClientRequestEnvelope::new(
        "ttl-v2",
        ClientRequest::GetTtl {
            ns: ns(),
            key: key("ttl"),
        },
    );
    request.protocol_version = 2;

    let (frame_version, response) = send_with_frame_version(&surface, request, 2).await;

    assert_eq!(frame_version, 2);
    assert_eq!(response.protocol_version, 2);
    assert_eq!(
        response.result.unwrap_err().code,
        ClientErrorCode::IncompatibleVersion
    );
}

#[tokio::test]
async fn protocol_v2_v3_clients_do_not_receive_lock_conditional_shapes() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let mut request = ClientRequestEnvelope::new(
        "lock-v3",
        ClientRequest::ConditionalPut {
            ns: ns(),
            key: key("lock"),
            value: b"token".to_vec(),
            ttl_ms: Some(5_000),
            condition: ConditionalPutCondition::IfAbsent,
        },
    );
    request.protocol_version = 3;

    let (frame_version, response) = send_with_frame_version(&surface, request, 3).await;

    assert_eq!(frame_version, 3);
    assert_eq!(response.protocol_version, 3);
    assert_eq!(
        response.result.unwrap_err().code,
        ClientErrorCode::IncompatibleVersion
    );
    assert_eq!(surface.state().state_mutations(), 0);
}

#[tokio::test]
async fn conditional_put_if_absent_is_atomic_under_contention() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    surface.state().set_cache_time_for_tests(Some(1_000));

    let acquire = |request_id: &str, token: &'static [u8]| {
        ClientRequestEnvelope::new(
            request_id,
            ClientRequest::ConditionalPut {
                ns: ns(),
                key: key("lock"),
                value: token.to_vec(),
                ttl_ms: Some(5_000),
                condition: ConditionalPutCondition::IfAbsent,
            },
        )
    };

    assert_eq!(
        send(&surface, acquire("lock-1", b"token-1"))
            .await
            .result
            .unwrap(),
        ClientResponse::ConditionalStored { stored: true }
    );
    assert_eq!(
        send(&surface, acquire("lock-2", b"token-2"))
            .await
            .result
            .unwrap(),
        ClientResponse::ConditionalStored { stored: false }
    );

    let ClientResponse::Value { value } = send(
        &surface,
        ClientRequestEnvelope::new(
            "lock-get",
            ClientRequest::Get {
                ns: ns(),
                key: key("lock"),
            },
        ),
    )
    .await
    .result
    .unwrap() else {
        panic!("expected value response");
    };
    assert_eq!(value, Some(b"token-1".to_vec()));
    assert_eq!(surface.state().state_mutations(), 1);
}

#[tokio::test]
async fn conditional_put_treats_expired_key_as_absent() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    surface.state().set_cache_time_for_tests(Some(1_000));

    let put = |request_id: &str, token: &'static [u8], ttl_ms| {
        ClientRequestEnvelope::new(
            request_id,
            ClientRequest::ConditionalPut {
                ns: ns(),
                key: key("lock"),
                value: token.to_vec(),
                ttl_ms: Some(ttl_ms),
                condition: ConditionalPutCondition::IfAbsent,
            },
        )
    };

    assert_eq!(
        send(&surface, put("lock-1", b"token-1", 10))
            .await
            .result
            .unwrap(),
        ClientResponse::ConditionalStored { stored: true }
    );
    surface.state().advance_cache_time_for_tests(11);
    assert_eq!(
        send(&surface, put("lock-2", b"token-2", 10))
            .await
            .result
            .unwrap(),
        ClientResponse::ConditionalStored { stored: true }
    );
}

#[tokio::test]
async fn compare_value_invalidate_removes_only_matching_token() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    surface.state().set_cache_time_for_tests(Some(1_000));
    let put = ClientRequestEnvelope::new(
        "put-lock",
        ClientRequest::ConditionalPut {
            ns: ns(),
            key: key("lock"),
            value: b"owner".to_vec(),
            ttl_ms: Some(5_000),
            condition: ConditionalPutCondition::IfAbsent,
        },
    );
    assert!(send(&surface, put).await.result.is_ok());

    let release = |request_id: &str, token: &'static [u8]| {
        ClientRequestEnvelope::new(
            request_id,
            ClientRequest::CompareValueAndInvalidate {
                ns: ns(),
                key: key("lock"),
                expected_value: token.to_vec(),
            },
        )
    };
    assert_eq!(
        send(&surface, release("wrong", b"wrong"))
            .await
            .result
            .unwrap(),
        ClientResponse::CompareValueApplied { applied: false }
    );
    let ClientResponse::Value { value } = send(
        &surface,
        ClientRequestEnvelope::new(
            "get-after-wrong",
            ClientRequest::Get {
                ns: ns(),
                key: key("lock"),
            },
        ),
    )
    .await
    .result
    .unwrap() else {
        panic!("expected value response");
    };
    assert_eq!(value, Some(b"owner".to_vec()));

    assert_eq!(
        send(&surface, release("right", b"owner"))
            .await
            .result
            .unwrap(),
        ClientResponse::CompareValueApplied { applied: true }
    );
    let ClientResponse::Value { value } = send(
        &surface,
        ClientRequestEnvelope::new(
            "get-after-right",
            ClientRequest::Get {
                ns: ns(),
                key: key("lock"),
            },
        ),
    )
    .await
    .result
    .unwrap() else {
        panic!("expected value response");
    };
    assert!(value.is_none());
}

#[tokio::test]
async fn compare_value_expire_extends_only_matching_token() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    surface.state().set_cache_time_for_tests(Some(1_000));
    let put = ClientRequestEnvelope::new(
        "put-lock",
        ClientRequest::ConditionalPut {
            ns: ns(),
            key: key("lock"),
            value: b"owner".to_vec(),
            ttl_ms: Some(100),
            condition: ConditionalPutCondition::IfAbsent,
        },
    );
    assert!(send(&surface, put).await.result.is_ok());

    let extend = |request_id: &str, token: &'static [u8], ttl_ms| {
        ClientRequestEnvelope::new(
            request_id,
            ClientRequest::CompareValueAndExpire {
                ns: ns(),
                key: key("lock"),
                expected_value: token.to_vec(),
                ttl_ms,
            },
        )
    };
    assert_eq!(
        send(&surface, extend("wrong", b"wrong", 1_000))
            .await
            .result
            .unwrap(),
        ClientResponse::CompareValueApplied { applied: false }
    );
    surface.state().advance_cache_time_for_tests(50);
    assert_eq!(
        send(&surface, extend("right", b"owner", 1_000))
            .await
            .result
            .unwrap(),
        ClientResponse::CompareValueApplied { applied: true }
    );
    surface.state().advance_cache_time_for_tests(100);
    let ClientResponse::Value { value } = send(
        &surface,
        ClientRequestEnvelope::new(
            "get-extended",
            ClientRequest::Get {
                ns: ns(),
                key: key("lock"),
            },
        ),
    )
    .await
    .result
    .unwrap() else {
        panic!("expected value response");
    };
    assert_eq!(value, Some(b"owner".to_vec()));
}

#[tokio::test]
async fn client_surface_subscribe_entry_events_uses_bounded_subscription_family() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let from = Some(Watermark::new(3, 5));

    let response = send(
        &surface,
        ClientRequestEnvelope::new(
            "entry-events-1",
            ClientRequest::SubscribeEntryEvents {
                ns: ns(),
                region: None,
                from,
                include_value: true,
                projection: EntryEventProjection::IMapEntryEvent,
            },
        ),
    )
    .await
    .result
    .unwrap();

    assert_eq!(response, ClientResponse::Subscribed { from });
}

#[tokio::test]
async fn client_surface_batch_put_is_atomic_when_an_item_is_too_large() {
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

    let error = send(&surface, batch).await.result.unwrap_err();
    assert_eq!(error.code, ClientErrorCode::TooLarge);

    let get = ClientRequestEnvelope::new(
        "get-after-rejected-batch",
        ClientRequest::Get {
            ns: ns(),
            key: key("1"),
        },
    );
    let ClientResponse::Value { value } = send(&surface, get).await.result.unwrap() else {
        panic!("expected value response");
    };
    assert!(value.is_none());
    assert_eq!(surface.state().state_mutations(), 0);
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
async fn mixed_version_v1_client_never_sees_v2_response() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let mut request = ClientRequestEnvelope::new(
        "get-v1",
        ClientRequest::Get {
            ns: ns(),
            key: key("42"),
        },
    );
    request.protocol_version = 1;

    let (frame_version, response) = send_with_frame_version(&surface, request, 1).await;

    assert_eq!(frame_version, 1);
    assert_eq!(response.protocol_version, 1);
    assert!(matches!(
        response.result.unwrap(),
        ClientResponse::Value { value: None }
    ));
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
