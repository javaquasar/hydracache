use std::convert::Infallible;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use futures_util::stream;
use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse, ClientWireMessage,
    CompareValueExpireMode, ConditionalPutCondition, Namespace, StructuredKey, TtlState,
    PROTOCOL_VERSION,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, CLIENT_DATA_PATH, CLIENT_SUBSCRIPTIONS_PATH,
    HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use tokio::sync::Notify;
use tower::ServiceExt;

#[derive(Clone, Debug)]
struct CancellationCheckpoint {
    name: &'static str,
    reached: Arc<AtomicBool>,
    reached_notify: Arc<Notify>,
}

impl CancellationCheckpoint {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            reached: Arc::new(AtomicBool::new(false)),
            reached_notify: Arc::new(Notify::new()),
        }
    }

    fn mark_reached(&self) {
        self.reached.store(true, Ordering::SeqCst);
        self.reached_notify.notify_waiters();
    }

    async fn wait_until_reached(&self) {
        while !self.reached.load(Ordering::SeqCst) {
            let notified = self.reached_notify.notified();
            if self.reached.load(Ordering::SeqCst) {
                break;
            }
            notified.await;
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

enum BodyPhase {
    First {
        chunk: Vec<u8>,
        remainder: Vec<u8>,
        checkpoint: CancellationCheckpoint,
    },
    Waiting {
        remainder: Vec<u8>,
        checkpoint: CancellationCheckpoint,
    },
    Done,
}

fn cancellable_body(frame: Vec<u8>, checkpoint: CancellationCheckpoint) -> Body {
    let split = (frame.len() / 2).max(1).min(frame.len());
    let phase = BodyPhase::First {
        chunk: frame[..split].to_vec(),
        remainder: frame[split..].to_vec(),
        checkpoint,
    };
    Body::from_stream(stream::unfold(phase, |phase| async move {
        match phase {
            BodyPhase::First {
                chunk,
                remainder,
                checkpoint,
            } => Some((
                Ok::<_, Infallible>(chunk),
                BodyPhase::Waiting {
                    remainder,
                    checkpoint,
                },
            )),
            BodyPhase::Waiting {
                remainder,
                checkpoint,
            } => {
                checkpoint.mark_reached();
                std::future::pending::<()>().await;
                Some((Ok::<_, Infallible>(remainder), BodyPhase::Done))
            }
            BodyPhase::Done => None,
        }
    }))
}

fn isolated_surface(max_streams_per_connection: usize) -> AxumClientSurface {
    let roster = TenantRoster::new(vec![Tenant::new("tenant-a")
        .unwrap()
        .allow_client("client-a")
        .namespace("users", NamespaceQuota::new(16 * 1024 * 1024, 8))])
    .unwrap();
    let limits = ClientSurfaceLimits {
        max_streams_per_connection,
        ..ClientSurfaceLimits::default()
    };
    AxumClientSurface::with_isolation(
        limits,
        ConsumerIsolation::new(roster, ConsumerIsolationConfig::default()),
    )
    .unwrap()
}

fn namespace() -> Namespace {
    Namespace::new("users").unwrap()
}

fn key() -> StructuredKey {
    StructuredKey::new(vec!["user".to_owned(), "cancelled-lock".to_owned()]).unwrap()
}

fn encode_request(request_id: &str, request: ClientRequest) -> Vec<u8> {
    ClientFrame::from_message_with_version(
        PROTOCOL_VERSION,
        &ClientWireMessage::Request(ClientRequestEnvelope::new(request_id, request)),
    )
    .unwrap()
    .encode()
    .unwrap()
    .to_vec()
}

async fn send(
    surface: &AxumClientSurface,
    request_id: &str,
    request: ClientRequest,
) -> ClientResponse {
    let frame = encode_request(request_id, request);
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
    let ClientWireMessage::Response(response) = frame.decode_message().unwrap() else {
        panic!("expected response frame");
    };
    response.result.unwrap()
}

fn write_evidence(test_name: &str, checkpoints: &[&str], assertions: &[&str]) {
    let Ok(path) = std::env::var("HYDRACACHE_CANCELLATION_CLIENT_EVIDENCE") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).expect("create client cancellation evidence directory");
    }
    let evidence = serde_json::json!({
        "suite": "W39b",
        "test": test_name,
        "status": "passed",
        "checkpoints": checkpoints,
        "assertions": assertions,
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&evidence).expect("serialize client evidence"),
    )
    .expect("write client cancellation evidence");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_drop_at_registered_boundaries_preserves_lock_token_and_ttl() {
    let surface = isolated_surface(1);
    surface.state().set_cache_time_for_tests(Some(1_000));
    let checkpoint = CancellationCheckpoint::new("axum.body.read.before_protocol_decode");
    let body = cancellable_body(
        encode_request(
            "cancelled-lock",
            ClientRequest::ConditionalPut {
                ns: namespace(),
                key: key(),
                value: b"token".to_vec(),
                ttl_ms: Some(100),
                condition: ConditionalPutCondition::IfAbsent,
            },
        ),
        checkpoint.clone(),
    );
    let surface_for_task = surface.clone();
    let task = tokio::spawn(async move {
        surface_for_task
            .routes()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(CLIENT_DATA_PATH)
                    .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
                    .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
                    .body(body)
                    .unwrap(),
            )
            .await
    });

    checkpoint.wait_until_reached().await;
    task.abort();
    assert!(task
        .await
        .expect_err("incomplete request must be cancelled")
        .is_cancelled());
    assert_eq!(surface.state().dispatch_attempts(), 0);
    assert_eq!(surface.state().state_mutations(), 0);

    assert_eq!(
        send(
            &surface,
            "lock-put",
            ClientRequest::ConditionalPut {
                ns: namespace(),
                key: key(),
                value: b"token".to_vec(),
                ttl_ms: Some(100),
                condition: ConditionalPutCondition::IfAbsent,
            },
        )
        .await,
        ClientResponse::ConditionalStored { stored: true }
    );
    surface.state().advance_cache_time_for_tests(25);
    assert_eq!(
        send(
            &surface,
            "lock-extend",
            ClientRequest::CompareValueAndExpire {
                ns: namespace(),
                key: key(),
                expected_value: b"token".to_vec(),
                ttl_ms: 40,
                mode: CompareValueExpireMode::AddToRemaining,
            },
        )
        .await,
        ClientResponse::CompareValueApplied { applied: true }
    );
    assert_eq!(
        send(
            &surface,
            "lock-ttl",
            ClientRequest::GetTtl {
                ns: namespace(),
                key: key(),
            },
        )
        .await,
        ClientResponse::Ttl {
            state: TtlState::ExpiresIn { ttl_ms: 115 }
        }
    );

    write_evidence(
        "client_drop_at_registered_boundaries_preserves_lock_token_and_ttl",
        &[checkpoint.name()],
        &[
            "cancelled body did not reach dispatch or mutate state",
            "conditional lock acquisition remained available after cancellation",
            "token-safe additive TTL extension preserved the expected 115ms TTL",
        ],
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_drop_does_not_leak_subscription_or_inflight_budget() {
    let surface = isolated_surface(1);
    let checkpoint = CancellationCheckpoint::new("axum.subscription.body.read.before_reservation");
    let surface_for_task = surface.clone();
    let task_checkpoint = checkpoint.clone();
    let task = tokio::spawn(async move {
        surface_for_task
            .routes()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(CLIENT_SUBSCRIPTIONS_PATH)
                    .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
                    .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
                    .body(cancellable_body(Vec::from([1, 2, 3, 4]), task_checkpoint))
                    .unwrap(),
            )
            .await
    });

    checkpoint.wait_until_reached().await;
    task.abort();
    assert!(task
        .await
        .expect_err("incomplete subscription must be cancelled")
        .is_cancelled());
    assert_eq!(surface.state().active_subscriptions(), 0);
    assert_eq!(surface.state().dispatch_attempts(), 0);

    let response = surface
        .routes()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_SUBSCRIPTIONS_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
                .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
                .body(Body::from(Vec::<u8>::new()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(surface.state().active_subscriptions(), 1);
    assert_eq!(surface.state().dispatch_attempts(), 1);

    write_evidence(
        "client_drop_does_not_leak_subscription_or_inflight_budget",
        &[checkpoint.name()],
        &[
            "cancelled subscription body did not reserve a stream slot",
            "the single-stream budget accepted the next complete subscription",
            "dispatch accounting counted only the complete request",
        ],
    );
}
