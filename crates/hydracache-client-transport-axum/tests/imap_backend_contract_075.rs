//! Contracts for the production dispatcher and its actual local data owner.
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use futures_util::FutureExt;
use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    ClientErrorCode, ClientErrorEnvelope, ClientRequest, ClientRequestEnvelope, ClientResponse,
    EntryEventProjection, LockConsistency, Namespace, StructuredKey, PROTOCOL_VERSION,
};
use hydracache_client_transport_axum::{
    ClientBackendFuture, ClientDataBackend, ClientIdentity, ClientSurfaceDispatcher,
    ClientSurfaceLimits, ClientSurfaceState, LocalDispatchVariant, VerifiedClientRequest,
};
use tokio::sync::Semaphore;

#[derive(Debug)]
struct GatedBackend {
    entered: AtomicUsize,
    active: AtomicUsize,
    completed: AtomicUsize,
    release: Semaphore,
    observed: Mutex<Vec<(String, String, String, usize)>>,
    fail: bool,
}

impl GatedBackend {
    fn new(fail: bool) -> Self {
        Self {
            entered: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            release: Semaphore::new(0),
            observed: Mutex::new(Vec::new()),
            fail,
        }
    }
}

struct Active<'a>(&'a AtomicUsize);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl ClientDataBackend for GatedBackend {
    fn execute<'a>(&'a self, request: VerifiedClientRequest<'a>) -> ClientBackendFuture<'a> {
        Box::pin(async move {
            self.entered.fetch_add(1, Ordering::SeqCst);
            self.active.fetch_add(1, Ordering::SeqCst);
            let _active = Active(&self.active);
            let tenant = request.identity().tenant().to_owned();
            let client = request.identity().client_id().to_owned();
            let limit = request.limits().max_value_bytes;
            let request_id = request.envelope().request_id.clone();
            assert_eq!(request.into_envelope().request_id, request_id);
            self.observed
                .lock()
                .unwrap()
                .push((client, tenant, request_id, limit));
            self.release.acquire().await.unwrap().forget();
            self.completed.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err(ClientErrorEnvelope::new(
                    ClientErrorCode::BackendUnavailable,
                    true,
                    "owner unavailable",
                ))
            } else {
                Ok(ClientResponse::Value {
                    value: Some(b"remote-owner".to_vec()),
                })
            }
        })
    }
}

fn identity() -> ClientIdentity {
    ClientIdentity::new("client-a", "tenant-a").unwrap()
}
fn ns() -> Namespace {
    Namespace::new("data").unwrap()
}
fn key() -> StructuredKey {
    StructuredKey::new(vec!["key".to_owned()]).unwrap()
}
fn get(id: &str) -> ClientRequestEnvelope {
    ClientRequestEnvelope::new(
        id,
        ClientRequest::Get {
            ns: ns(),
            key: key(),
        },
    )
}

#[tokio::test]
async fn pending_backend_is_awaited_and_cancellation_drops_without_completion() {
    let backend = Arc::new(GatedBackend::new(false));
    let dispatcher =
        ClientSurfaceDispatcher::from_backend(Default::default(), backend.clone()).unwrap();
    assert_eq!(dispatcher.local_dispatch_variant(), None);
    let identity = identity();
    let mut request = Box::pin(dispatcher.dispatch_verified_request(&identity, get("cancelled")));
    assert_eq!(backend.entered.load(Ordering::SeqCst), 0);
    assert!(request.as_mut().now_or_never().is_none());
    assert_eq!(backend.active.load(Ordering::SeqCst), 1);
    assert_eq!(dispatcher.dispatch_attempts(), 1);
    assert_eq!(dispatcher.state_mutations(), 0);
    drop(request);
    assert_eq!(backend.active.load(Ordering::SeqCst), 0);
    assert_eq!(backend.completed.load(Ordering::SeqCst), 0);
    backend.release.add_permits(1);
    let result = dispatcher
        .dispatch_verified_request(&identity, get("completed"))
        .await;
    assert_eq!(result.request_id, "completed");
    assert_eq!(result.protocol_version, PROTOCOL_VERSION);
    assert_eq!(
        result.result.unwrap(),
        ClientResponse::Value {
            value: Some(b"remote-owner".to_vec())
        }
    );
    assert_eq!(backend.completed.load(Ordering::SeqCst), 1);
    assert_eq!(backend.active.load(Ordering::SeqCst), 0);
    assert_eq!(
        backend.observed.lock().unwrap()[1],
        (
            "client-a".into(),
            "tenant-a".into(),
            "completed".into(),
            dispatcher.limits().max_value_bytes
        )
    );
}

#[tokio::test]
async fn backend_failure_keeps_edge_correlation_and_never_falls_back_to_local_data() {
    let backend = Arc::new(GatedBackend::new(true));
    backend.release.add_permits(1);
    let dispatcher = ClientSurfaceDispatcher::from_backend(Default::default(), backend).unwrap();
    let mut request = get("failed");
    request.protocol_version = 1;
    let response = dispatcher
        .dispatch_verified_request(&identity(), request)
        .await;
    assert_eq!(response.request_id, "failed");
    assert_eq!(response.protocol_version, 1);
    assert_eq!(
        response.result.unwrap_err().code,
        ClientErrorCode::BackendUnavailable
    );
    assert_eq!(dispatcher.state_mutations(), 0);
}

#[tokio::test]
async fn admission_rejects_identity_protocol_and_deadline_before_backend_execution() {
    let backend = Arc::new(GatedBackend::new(false));
    let isolation = ConsumerIsolation::new(
        TenantRoster::new(vec![Tenant::new("tenant-a")
            .unwrap()
            .allow_client("client-a")
            .namespace("data", NamespaceQuota::new(1024, 16))])
        .unwrap(),
        ConsumerIsolationConfig::default(),
    );
    let dispatcher = ClientSurfaceDispatcher::from_backend_with_isolation(
        Default::default(),
        isolation,
        backend.clone(),
    )
    .unwrap();
    let denied = ClientIdentity::new("intruder", "tenant-a").unwrap();
    let response = dispatcher
        .dispatch_verified_request(&denied, get("denied"))
        .await;
    assert_eq!(
        response.result.unwrap_err().code,
        ClientErrorCode::Unauthorized
    );
    assert_eq!(dispatcher.dispatch_attempts(), 0);
    let mut request = get("protocol");
    request.protocol_version = u16::MAX;
    let response = dispatcher
        .dispatch_verified_request(&identity(), request)
        .await;
    assert_eq!(response.protocol_version, PROTOCOL_VERSION);
    assert_eq!(
        response.result.unwrap_err().code,
        ClientErrorCode::IncompatibleVersion
    );
    let response = dispatcher
        .dispatch_verified_request(&identity(), get("deadline").with_deadline_ms(0))
        .await;
    assert_eq!(
        response.result.unwrap_err().code,
        ClientErrorCode::DeadlineExceeded
    );
    assert_eq!(backend.entered.load(Ordering::SeqCst), 0);
    assert_eq!(dispatcher.dispatch_attempts(), 2);
}

#[tokio::test]
async fn subscriptions_and_cp_lock_engine_bypass_the_data_backend() {
    let backend = Arc::new(GatedBackend::new(false));
    let dispatcher =
        ClientSurfaceDispatcher::from_backend(Default::default(), backend.clone()).unwrap();
    let operations = vec![
        ClientRequest::SubscribeInvalidations {
            ns: ns(),
            region: None,
            from: None,
            include_value: false,
        },
        ClientRequest::SubscribeEntryEvents {
            ns: ns(),
            region: None,
            from: None,
            include_value: true,
            projection: EntryEventProjection::IMapEntryEvent,
        },
        ClientRequest::TryLock {
            ns: ns(),
            key: key(),
            lease_ms: 60_000,
            wait_ms: 0,
            level: LockConsistency::Quorum,
        },
        ClientRequest::GetLockOwnership {
            ns: ns(),
            key: key(),
        },
        ClientRequest::RenewLockLease {
            ns: ns(),
            key: key(),
            fence: 1,
            lease_ms: 60_000,
        },
        ClientRequest::Unlock {
            ns: ns(),
            key: key(),
            fence: 1,
        },
        ClientRequest::ForceUnlock {
            ns: ns(),
            key: key(),
        },
    ];
    for (index, operation) in operations.into_iter().enumerate() {
        let response = dispatcher
            .dispatch_verified_request(
                &identity(),
                ClientRequestEnvelope::new(index.to_string(), operation),
            )
            .await;
        let expected = match index {
            0 | 1 => Ok(ClientResponse::Subscribed { from: None }),
            2 => Ok(ClientResponse::LockAcquired { fence: 1 }),
            3 => Ok(ClientResponse::LockOwnership {
                fence: Some(1),
                locked: true,
            }),
            4 => Ok(ClientResponse::LockLeaseRenewed),
            5 => Ok(ClientResponse::LockReleased),
            6 => {
                assert_eq!(
                    response.result.unwrap_err().code,
                    ClientErrorCode::Unauthorized
                );
                continue;
            }
            _ => unreachable!(),
        };
        assert_eq!(response.result, expected);
    }
    assert_eq!(backend.entered.load(Ordering::SeqCst), 0);
    assert_eq!(dispatcher.dispatch_attempts(), 7);
    let _subscriber = dispatcher.subscribe_mutations(&identity()).unwrap();
}

#[tokio::test]
async fn local_candidates_share_one_owner_and_sync_entry_refuses_async_before_effects() {
    for variant in [
        LocalDispatchVariant::Static,
        LocalDispatchVariant::BoxedFuture,
    ] {
        let state = ClientSurfaceState::with_dispatch_variant(Default::default(), variant).unwrap();
        assert!(Arc::ptr_eq(&state.local_backend(), &state.local_backend()));
        assert!(Arc::ptr_eq(&state.dispatcher(), &state.dispatcher()));
        let dispatcher = state.dispatcher();
        assert_eq!(dispatcher.local_dispatch_variant(), Some(variant));
        if variant == LocalDispatchVariant::Static {
            let rejected = state
                .dispatch_verified_request(&identity(), get("expired-sync").with_deadline_ms(0));
            assert_eq!(
                rejected.result.unwrap_err().code,
                ClientErrorCode::DeadlineExceeded
            );
            assert_eq!(state.state_mutations(), 0);
        }
        let write = ClientRequestEnvelope::new(
            "put",
            ClientRequest::Put {
                ns: ns(),
                key: key(),
                value: vec![0, 255, 1],
                ttl_ms: None,
                dimensions: vec![],
            },
        );
        if variant == LocalDispatchVariant::BoxedFuture {
            assert_eq!(
                state
                    .dispatch_verified_request(&identity(), write.clone())
                    .result
                    .unwrap_err()
                    .code,
                ClientErrorCode::BackendUnavailable
            );
            assert_eq!(state.dispatch_attempts(), 0);
            assert_eq!(state.state_mutations(), 0);
        }
        assert_eq!(
            dispatcher
                .dispatch_verified_request(&identity(), write)
                .await
                .result
                .unwrap(),
            ClientResponse::Stored
        );
        assert_eq!(
            dispatcher
                .dispatch_verified_request(&identity(), get("read"))
                .await
                .result
                .unwrap(),
            ClientResponse::Value {
                value: Some(vec![0, 255, 1])
            }
        );
        assert_eq!(state.state_mutations(), 1);
    }
}

#[test]
fn invalid_limits_refuse_both_local_candidates_and_external_backend() {
    let limits = ClientSurfaceLimits {
        max_value_bytes: 0,
        ..Default::default()
    };
    for variant in [
        LocalDispatchVariant::Static,
        LocalDispatchVariant::BoxedFuture,
    ] {
        assert!(ClientSurfaceState::with_dispatch_variant(limits, variant).is_err());
    }
    let backend = Arc::new(GatedBackend::new(false));
    assert!(ClientSurfaceDispatcher::from_backend(limits, backend).is_err());
}

#[tokio::test]
async fn hc1_wire_route_awaits_the_selected_backend_and_counts_admission_once() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use hydracache_client_protocol::{ClientFrame, ClientWireMessage};
    use hydracache_client_transport_axum::{
        CLIENT_DATA_PATH, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
    };
    use tower::ServiceExt;
    let backend = Arc::new(GatedBackend::new(false));
    backend.release.add_permits(1);
    let dispatcher = Arc::new(
        ClientSurfaceDispatcher::from_backend(Default::default(), backend.clone()).unwrap(),
    );
    let frame = ClientFrame::from_message(&ClientWireMessage::Request(get("wire")))
        .unwrap()
        .encode()
        .unwrap();
    let response = dispatcher
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
    let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
    let message = ClientFrame::decode(&bytes, 1_048_576)
        .unwrap()
        .decode_message()
        .unwrap();
    let ClientWireMessage::Response(response) = message else {
        panic!("response required")
    };
    assert_eq!(response.request_id, "wire");
    assert_eq!(
        response.result.unwrap(),
        ClientResponse::Value {
            value: Some(b"remote-owner".to_vec())
        }
    );
    assert_eq!(dispatcher.dispatch_attempts(), 1);
    assert_eq!(backend.completed.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn hc1_handshake_and_wrong_direction_frames_never_enter_the_data_backend() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use hydracache_client_protocol::{
        ClientFrame, ClientResponseEnvelope, ClientWireMessage, VersionHandshake, Watermark,
    };
    use hydracache_client_transport_axum::{
        CLIENT_DATA_PATH, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
    };
    use tower::ServiceExt;
    let backend = Arc::new(GatedBackend::new(false));
    let dispatcher = Arc::new(
        ClientSurfaceDispatcher::from_backend(Default::default(), backend.clone()).unwrap(),
    );
    let post = |message| {
        let frame = ClientFrame::from_message(&message)
            .unwrap()
            .encode()
            .unwrap();
        dispatcher.routes().oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_DATA_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
                .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
                .body(Body::from(frame))
                .unwrap(),
        )
    };
    let response = post(ClientWireMessage::Handshake(VersionHandshake::default()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
    assert_eq!(
        ClientFrame::decode(&bytes, 1_048_576)
            .unwrap()
            .decode_message()
            .unwrap(),
        ClientWireMessage::Handshake(VersionHandshake::new(PROTOCOL_VERSION, PROTOCOL_VERSION))
    );
    for message in [
        ClientWireMessage::Handshake(VersionHandshake::new(
            PROTOCOL_VERSION + 1,
            PROTOCOL_VERSION,
        )),
        ClientWireMessage::Response(ClientResponseEnvelope::ok(
            "wrong-direction",
            ClientResponse::Stored,
        )),
        ClientWireMessage::Heartbeat(Watermark::new(1, 1)),
    ] {
        assert_eq!(
            post(message).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(backend.entered.load(Ordering::SeqCst), 0);
    assert_eq!(dispatcher.dispatch_attempts(), 4);
    assert_eq!(dispatcher.state_mutations(), 0);
}
