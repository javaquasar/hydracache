use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache_client_hc2::adapter::{AdapterConnection, ConnectionControl, TransportAdapter};
use hydracache_client_hc2::wire::client_envelope;
use hydracache_client_hc2::wire::server_envelope;
use hydracache_client_hc2::wire::{
    invocation_response, CacheEvent, HandshakeAck, InvocationResponse, MutationResult,
    ResponseMeta, ServerEnvelope, SessionHeartbeat, SubscriptionAck, ValueResult,
};
use hydracache_client_hc2::{
    ClientConfig, ClientError, ErrorCode, InvocationRetryPolicy, ReconnectEndpoint,
    ReconnectPolicy, RecoveringHc2Client, RequestOptions, RetryAdvice, SubscriptionEvent,
    TransportKind, HC2_GENERATION,
};
use tokio::sync::{mpsc, watch};

#[derive(Default)]
struct ScriptState {
    connections: AtomicUsize,
    completions: AtomicUsize,
    subscriptions: AtomicUsize,
    disconnect_once: AtomicBool,
    fail_after_first_connection: AtomicBool,
    fail_connects_remaining: AtomicUsize,
    received_generations: Mutex<Vec<u64>>,
}

impl ScriptState {
    fn new() -> Self {
        Self {
            disconnect_once: AtomicBool::new(true),
            ..Self::default()
        }
    }
}

#[derive(Clone)]
struct ScriptedAdapter {
    state: Arc<ScriptState>,
}

struct ScriptControl(watch::Sender<bool>);

impl ConnectionControl for ScriptControl {
    fn close(&self) {
        let _ = self.0.send(true);
    }
}

#[async_trait]
impl TransportAdapter for ScriptedAdapter {
    fn kind(&self) -> TransportKind {
        TransportKind::GrpcBidirectional
    }

    async fn connect(&self, config: &ClientConfig) -> Result<AdapterConnection, ClientError> {
        let connection = self.state.connections.fetch_add(1, Ordering::AcqRel) + 1;
        self.state
            .received_generations
            .lock()
            .expect("generation mutex poisoned")
            .push(config.connection_generation);
        if self
            .state
            .fail_connects_remaining
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
        {
            return Err(unavailable("scripted handshake reset"));
        }
        if connection > 1
            && self
                .state
                .fail_after_first_connection
                .load(Ordering::Acquire)
        {
            return Err(unavailable("scripted reconnect refusal"));
        }
        let state = Arc::clone(&self.state);
        let (outbound, mut outbound_rx) =
            mpsc::channel::<hydracache_client_hc2::wire::ClientEnvelope>(32);
        let (inbound_tx, inbound) = mpsc::channel::<Result<ServerEnvelope, ClientError>>(32);
        let (close_tx, mut close_rx) = watch::channel(false);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    changed = close_rx.changed() => {
                        if changed.is_err() || *close_rx.borrow() {
                            return;
                        }
                    }
                    envelope = outbound_rx.recv() => {
                        let Some(envelope) = envelope else { return };
                        let generation = envelope.connection_generation;
                        match envelope.message {
                            Some(client_envelope::Message::Handshake(handshake)) => {
                                let response = server(
                                    generation,
                                    envelope.correlation_id,
                                    server_envelope::Message::Handshake(HandshakeAck {
                                        generation: HC2_GENERATION,
                                        cluster_id: "hc2-reconnect-test".to_owned(),
                                        accepted: handshake.requested,
                                        topology_epoch: 1,
                                        connection_generation: generation,
                                        ..HandshakeAck::default()
                                    }),
                                );
                                if inbound_tx.send(Ok(response)).await.is_err() { return; }
                            }
                            Some(client_envelope::Message::Invocation(invocation)) => {
                                let disconnect = invocation.operation.as_ref().is_some_and(|operation| {
                                    match operation {
                                        hydracache_client_hc2::wire::invocation_request::Operation::Get(get) => get.key == b"disconnect-once"[..],
                                        hydracache_client_hc2::wire::invocation_request::Operation::Put(put) => put.key == b"unsafe-disconnect"[..],
                                        _ => false,
                                    }
                                }) && state.disconnect_once.swap(false, Ordering::AcqRel);
                                if disconnect {
                                    let _ = inbound_tx.send(Err(unavailable("scripted stream reset"))).await;
                                    return;
                                }
                                state.completions.fetch_add(1, Ordering::Relaxed);
                                let result = match invocation.operation {
                                    Some(hydracache_client_hc2::wire::invocation_request::Operation::Get(_)) => {
                                        invocation_response::Result::Value(ValueResult {
                                            found: true,
                                            value: Bytes::from_static(b"value"),
                                            expires_at_unix_ms: 0,
                                        })
                                    }
                                    _ => invocation_response::Result::Mutation(MutationResult { applied: true }),
                                };
                                let response = server(
                                    generation,
                                    envelope.correlation_id,
                                    server_envelope::Message::Invocation(InvocationResponse {
                                        meta: Some(ok_meta()),
                                        result: Some(result),
                                    }),
                                );
                                if inbound_tx.send(Ok(response)).await.is_err() { return; }
                            }
                            Some(client_envelope::Message::Subscribe(subscribe)) => {
                                let registration = state.subscriptions.fetch_add(1, Ordering::AcqRel) + 1;
                                let ack = server(
                                    generation,
                                    envelope.correlation_id,
                                    server_envelope::Message::Subscribed(SubscriptionAck {
                                        subscription_id: subscribe.subscription_id,
                                        watermark: subscribe.resume_watermark,
                                    }),
                                );
                                if inbound_tx.send(Ok(ack)).await.is_err() { return; }
                                let events = if registration == 1 {
                                    vec![(1, b"first".as_slice())]
                                } else {
                                    vec![(1, b"duplicate".as_slice()), (2, b"second".as_slice())]
                                };
                                for (watermark, value) in events {
                                    let event = server(
                                        generation,
                                        0,
                                        server_envelope::Message::Event(CacheEvent {
                                            subscription_id: subscribe.subscription_id,
                                            watermark,
                                            key: Bytes::from_static(b"events/key"),
                                            value: Bytes::copy_from_slice(value),
                                            removed: false,
                                        }),
                                    );
                                    if inbound_tx.send(Ok(event)).await.is_err() { return; }
                                }
                            }
                            Some(client_envelope::Message::SessionOpen(_)) => {
                                let response = server(
                                    generation,
                                    envelope.correlation_id,
                                    server_envelope::Message::SessionHeartbeat(SessionHeartbeat {
                                        session_id: Bytes::from_static(b"session-1"),
                                        fence: 7,
                                    }),
                                );
                                if inbound_tx.send(Ok(response)).await.is_err() { return; }
                            }
                            Some(client_envelope::Message::SessionHeartbeat(heartbeat)) => {
                                let response = server(
                                    generation,
                                    envelope.correlation_id,
                                    server_envelope::Message::SessionHeartbeat(heartbeat),
                                );
                                if inbound_tx.send(Ok(response)).await.is_err() { return; }
                            }
                            Some(client_envelope::Message::Cancel(_))
                            | Some(client_envelope::Message::Unsubscribe(_))
                            | Some(client_envelope::Message::SessionClose(_))
                            | None => {}
                        }
                    }
                }
            }
        });
        Ok(AdapterConnection::new(
            outbound,
            inbound,
            Arc::new(ScriptControl(close_tx)),
        ))
    }
}

fn server(
    connection_generation: u64,
    correlation_id: u64,
    message: server_envelope::Message,
) -> ServerEnvelope {
    ServerEnvelope {
        generation: HC2_GENERATION,
        connection_generation,
        correlation_id,
        message: Some(message),
    }
}

fn ok_meta() -> ResponseMeta {
    ResponseMeta {
        error: hydracache_client_hc2::wire::StableErrorCode::StableErrorUnspecified.into(),
        retry: hydracache_client_hc2::wire::RetryDirective::Never.into(),
        safe_detail: String::new(),
        topology_epoch: 1,
    }
}

fn unavailable(detail: &'static str) -> ClientError {
    ClientError::new(
        ErrorCode::Unavailable,
        RetryAdvice::ReconnectIdempotent,
        detail,
    )
}

fn endpoint(id: &str, state: Arc<ScriptState>) -> ReconnectEndpoint {
    ReconnectEndpoint::new(id, Arc::new(ScriptedAdapter { state })).unwrap()
}

fn policies() -> (ReconnectPolicy, InvocationRetryPolicy) {
    (
        ReconnectPolicy::new(
            3,
            Duration::from_millis(1),
            Duration::from_millis(4),
            Duration::ZERO,
            0x11,
        )
        .unwrap(),
        InvocationRetryPolicy::new(2).unwrap(),
    )
}

fn retained(case: &str) -> serde_json::Value {
    let path = format!(
        "{}/../../docs/testing/hc2-fault-proxy/{case}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&fs::read(path).expect("read retained H11 trace"))
        .expect("decode retained H11 trace")
}

#[test]
fn retained_h11_matrix_binds_every_declared_lifecycle_failure() {
    for (case, action_kind, terminal) in [
        ("h11-handshake-reset-seed-1592590357", "reset", "reset"),
        (
            "h11-server-restart-seed-1592590358",
            "close_after_bytes",
            "closed_after_bytes",
        ),
        (
            "h11-leader-hint-reset-seed-1592590359",
            "close_after_bytes",
            "closed_after_bytes",
        ),
        ("h11-subscription-gap-seed-1592590360", "drop", "open"),
        ("h11-invocation-reset-seed-1592590361", "reset", "reset"),
        (
            "h11-session-loss-seed-1592590362",
            "late_delivery",
            "half_open",
        ),
        ("h11-duplicate-event-seed-1592590363", "duplicate", "open"),
        (
            "h11-reconnect-exhausted-seed-1592590364",
            "block_direction",
            "blocked",
        ),
    ] {
        let artifact = retained(case);
        assert_eq!(
            artifact["schema"], "hydracache.hc2.fault-replay.v1",
            "{case}"
        );
        assert_eq!(
            artifact["plan"]["actions"]
                .as_array()
                .and_then(|actions| actions.last())
                .and_then(|action| action["kind"].as_str()),
            Some(action_kind),
            "{case}"
        );
        assert_eq!(artifact["trace"]["terminal"], terminal, "{case}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn initial_handshake_reset_selects_the_next_endpoint_without_operation_replay() {
    let reset = Arc::new(ScriptState::new());
    reset.disconnect_once.store(false, Ordering::Release);
    reset.fail_connects_remaining.store(1, Ordering::Release);
    let healthy = Arc::new(ScriptState::new());
    healthy.disconnect_once.store(false, Ordering::Release);
    let (reconnect, retry) = policies();
    let client = RecoveringHc2Client::connect(
        vec![
            endpoint("resetting", Arc::clone(&reset)),
            endpoint("healthy", Arc::clone(&healthy)),
        ],
        ClientConfig::new("handshake-selection", "tenant-a"),
        reconnect,
        retry,
    )
    .await
    .expect("bounded initial endpoint selection");
    assert_eq!(client.current_endpoint(), "healthy");
    assert_eq!(client.connection_generation(), 1);
    assert_eq!(client.recovery_metrics().reconnect_attempts, 0);
    assert_eq!(reset.connections.load(Ordering::Acquire), 1);
    assert_eq!(healthy.connections.load(Ordering::Acquire), 1);
    client.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_reconnects_once_repairs_subscription_dedupes_and_loses_session() {
    let reset = retained("h11-invocation-reset-seed-1592590361");
    assert_eq!(reset["trace"]["terminal"], "reset");
    let duplicate = retained("h11-duplicate-event-seed-1592590363");
    assert!(duplicate["plan"]["actions"]
        .as_array()
        .is_some_and(|actions| actions
            .iter()
            .any(|action| { action["kind"] == "duplicate" && action["copies"] == 1 })));

    let primary = Arc::new(ScriptState::new());
    let secondary = Arc::new(ScriptState::new());
    secondary.disconnect_once.store(false, Ordering::Release);
    let (reconnect, retry) = policies();
    let mut config = ClientConfig::new("recovering-rust-test", "tenant-a");
    config.default_request_timeout = Duration::from_secs(2);
    let client = RecoveringHc2Client::connect(
        vec![
            endpoint("primary", Arc::clone(&primary)),
            endpoint("secondary", Arc::clone(&secondary)),
        ],
        config,
        reconnect,
        retry,
    )
    .await
    .expect("initial connection");

    let mut subscription = client
        .subscribe(Bytes::from_static(b"events/"), 0)
        .await
        .expect("logical subscription");
    assert!(matches!(
        subscription.next().await,
        Some(SubscriptionEvent::Event(event)) if event.watermark == 1
    ));
    let session = client
        .open_session(Duration::from_secs(3))
        .await
        .expect("fenced session");
    assert_eq!(session.fence().unwrap(), 7);
    let active = client.retained_state();
    assert_eq!(active.logical_subscriptions, 1);
    assert_eq!(active.session_registrations, 1);
    assert_eq!(active.live_sessions, 1);
    assert_eq!(active.current_client.active_subscriptions, 1);
    assert_eq!(active.current_client.active_sessions, 1);

    let waiting_for_repair = tokio::spawn(async move {
        let event = subscription.next().await;
        (subscription, event)
    });

    let value = client
        .get(Bytes::from_static(b"disconnect-once"), None)
        .await
        .expect("safe read reconnect and retry");
    assert_eq!(value.unwrap().value, Bytes::from_static(b"value"));
    assert_eq!(primary.completions.load(Ordering::Acquire), 1);
    assert_eq!(client.connection_generation(), 2);
    assert_eq!(session.fence().unwrap_err().code(), ErrorCode::SessionLost);

    let (mut subscription, repair_boundary) =
        tokio::time::timeout(Duration::from_secs(1), waiting_for_repair)
            .await
            .expect("blocked subscription reader must wake for repair")
            .expect("subscription reader task");
    assert!(matches!(
        repair_boundary,
        Some(SubscriptionEvent::Gap {
            after_watermark: 1,
            ..
        })
    ));
    subscription.repair(1).await.expect("explicit repair");
    assert!(matches!(
        subscription.next().await,
        Some(SubscriptionEvent::Event(event)) if event.watermark == 2
    ));

    client.prefer_endpoint("secondary").unwrap();
    client.reconnect_now().await.expect("leader-hint reconnect");
    assert_eq!(client.current_endpoint(), "secondary");
    assert_eq!(client.connection_generation(), 3);

    let metrics = client.recovery_metrics();
    assert_eq!(metrics.invocation_retries, 1);
    assert_eq!(metrics.reconnect_successes, 2);
    assert!(metrics.subscription_repairs >= 2);
    assert!(metrics.duplicate_events >= 1);
    assert!(metrics.gap_repairs_required >= 1);
    assert_eq!(metrics.session_losses, 1);
    let generations = primary
        .received_generations
        .lock()
        .expect("generation mutex poisoned")
        .clone();
    assert_eq!(generations, vec![1, 2]);
    subscription.close();
    session.close();
    let released = client.retained_state();
    assert_eq!(released.logical_subscriptions, 0);
    assert_eq!(released.session_registrations, 0);
    assert_eq!(released.live_sessions, 0);
    assert_eq!(released.current_client.pending_invocations, 0);
    assert_eq!(released.current_client.pending_subscriptions, 0);
    assert_eq!(released.current_client.active_subscriptions, 0);
    assert_eq!(released.current_client.pending_sessions, 0);
    assert_eq!(released.current_client.active_sessions, 0);
    client.close();
    assert!(client.retained_state().closed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exhaustion_and_unsafe_mutation_fail_loud_without_hidden_replay() {
    let exhausted = retained("h11-reconnect-exhausted-seed-1592590364");
    assert_eq!(exhausted["trace"]["terminal"], "blocked");

    let state = Arc::new(ScriptState::new());
    state
        .fail_after_first_connection
        .store(true, Ordering::Release);
    let (reconnect, retry) = policies();
    let mut config = ClientConfig::new("reconnect-exhaustion", "tenant-a");
    config.default_request_timeout = Duration::from_secs(1);
    let client = RecoveringHc2Client::connect(
        vec![endpoint("only", Arc::clone(&state))],
        config,
        reconnect,
        retry,
    )
    .await
    .unwrap();
    let error = client
        .get(Bytes::from_static(b"disconnect-once"), None)
        .await
        .expect_err("bounded reconnect must exhaust");
    assert_eq!(error.code(), ErrorCode::Unavailable);
    assert_eq!(client.recovery_metrics().reconnect_exhausted, 1);
    assert_eq!(state.completions.load(Ordering::Acquire), 0);
    client.close();

    let unsafe_state = Arc::new(ScriptState::new());
    let (reconnect, retry) = policies();
    let client = RecoveringHc2Client::connect(
        vec![endpoint("only", Arc::clone(&unsafe_state))],
        ClientConfig::new("unsafe-mutation", "tenant-a"),
        reconnect,
        retry,
    )
    .await
    .unwrap();
    let error = client
        .put(
            Bytes::from_static(b"unsafe-disconnect"),
            Bytes::from_static(b"value"),
            None,
            None,
        )
        .await
        .expect_err("ambiguous mutation without idempotency key must not replay");
    assert_eq!(error.retry_advice(), RetryAdvice::ReconnectIdempotent);
    assert_eq!(unsafe_state.connections.load(Ordering::Acquire), 1);
    assert_eq!(client.recovery_metrics().invocation_retries, 0);
    client.close();

    let safe_state = Arc::new(ScriptState::new());
    let (reconnect, retry) = policies();
    let client = RecoveringHc2Client::connect(
        vec![endpoint("only", Arc::clone(&safe_state))],
        ClientConfig::new("idempotent-mutation", "tenant-a"),
        reconnect,
        retry,
    )
    .await
    .unwrap();
    let mut options = RequestOptions::with_timeout(Duration::from_secs(1)).unwrap();
    options.idempotency_key = Bytes::from_static(b"stable-operation-id");
    client
        .put(
            Bytes::from_static(b"unsafe-disconnect"),
            Bytes::from_static(b"value"),
            None,
            Some(options),
        )
        .await
        .expect("idempotency-bound mutation may replay once after reconnect");
    assert_eq!(safe_state.completions.load(Ordering::Acquire), 1);
    assert_eq!(client.recovery_metrics().invocation_retries, 1);
    assert_eq!(client.recovery_metrics().reconnect_successes, 1);
    client.close();
}
