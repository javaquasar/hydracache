use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use prost::Message;
use tokio::sync::{mpsc, oneshot, OwnedSemaphorePermit, Semaphore};

use crate::adapter::{ConnectionControl, TransportAdapter};
use crate::types::{invalid, ttl_millis, validate_key, validate_value};
use crate::wire::client_envelope;
use crate::wire::server_envelope;
use crate::wire::{
    BatchItem, BatchRequest, Cancel, ClientEnvelope, CompareAndSetRequest, DeleteRequest,
    GetRequest, Handshake, HandshakeAck, InvocationRequest, InvocationResponse, PutRequest,
    RequestMeta, ServerEnvelope, SessionClose, SessionHeartbeat, SessionOpen, Subscribe,
    SubscriptionAck, Unsubscribe,
};
use crate::{
    BatchItemResult, BatchOperation, CacheEvent, CacheValue, Capability, ClientConfig, ClientError,
    ClientMetricsSnapshot, ErrorCode, MutationResult, NodeEndpoint, RequestOptions, RetryAdvice,
    SubscriptionEvent, TopologySnapshot, TransportKind,
};

struct Metrics {
    submitted: AtomicU64,
    completed: AtomicU64,
    failed: AtomicU64,
    cancelled: AtomicU64,
    late_responses: AtomicU64,
    events: AtomicU64,
    dropped_events: AtomicU64,
}

impl Metrics {
    const fn new() -> Self {
        Self {
            submitted: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            cancelled: AtomicU64::new(0),
            late_responses: AtomicU64::new(0),
            events: AtomicU64::new(0),
            dropped_events: AtomicU64::new(0),
        }
    }
}

struct PendingInvocation {
    reply: oneshot::Sender<Result<InvocationResponse, ClientError>>,
    _permit: OwnedSemaphorePermit,
}

struct PendingSubscription {
    subscription_id: u64,
    events: mpsc::Sender<SubscriptionEvent>,
    reply: oneshot::Sender<Result<SubscriptionAck, ClientError>>,
    permit: OwnedSemaphorePermit,
}

struct ActiveSubscription {
    events: mpsc::Sender<SubscriptionEvent>,
    gap_after: u64,
    _permit: OwnedSemaphorePermit,
}

struct PendingSession {
    ttl: Duration,
    reply: oneshot::Sender<Result<(Bytes, u64), ClientError>>,
    permit: OwnedSemaphorePermit,
}

struct ActiveSession {
    fence: Arc<AtomicU64>,
    _permit: OwnedSemaphorePermit,
}

type HandshakeReply = oneshot::Sender<Result<HandshakeAck, ClientError>>;

struct Runtime {
    config: ClientConfig,
    outbound: mpsc::Sender<ClientEnvelope>,
    control: Arc<dyn ConnectionControl>,
    closed: AtomicBool,
    correlation: AtomicU64,
    subscription_ids: AtomicU64,
    handshake: Mutex<Option<(u64, HandshakeReply)>>,
    invocations: Mutex<BTreeMap<u64, PendingInvocation>>,
    pending_subscriptions: Mutex<BTreeMap<u64, PendingSubscription>>,
    subscriptions: Mutex<BTreeMap<u64, ActiveSubscription>>,
    pending_sessions: Mutex<BTreeMap<u64, PendingSession>>,
    sessions: Mutex<BTreeMap<Bytes, ActiveSession>>,
    topology: RwLock<TopologySnapshot>,
    invocation_permits: Arc<Semaphore>,
    subscription_permits: Arc<Semaphore>,
    session_permits: Arc<Semaphore>,
    metrics: Metrics,
}

struct ClientHandle {
    runtime: Arc<Runtime>,
}

impl Drop for ClientHandle {
    fn drop(&mut self) {
        self.runtime
            .terminate(connection_error("last client handle dropped"));
    }
}

/// Cloneable native HC/2 client. Generated wire types do not appear in this API.
#[derive(Clone)]
pub struct Hc2Client {
    handle: Arc<ClientHandle>,
    cluster_id: Arc<str>,
    transport: TransportKind,
    negotiated_generation_deprecated: bool,
    preferred_protocol_generation: u32,
}

impl std::fmt::Debug for Hc2Client {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Hc2Client")
            .field("cluster_id", &self.cluster_id)
            .field("transport", &self.transport)
            .finish_non_exhaustive()
    }
}

impl Hc2Client {
    /// Connect through one explicitly selected adapter and complete negotiation.
    pub async fn connect<A: TransportAdapter + ?Sized>(
        adapter: &A,
        config: ClientConfig,
    ) -> Result<Self, ClientError> {
        let config = config.validate()?;
        let transport = adapter.kind();
        let parts = adapter.connect(&config).await?;
        let runtime = Arc::new(Runtime {
            invocation_permits: Arc::new(Semaphore::new(config.limits.max_pending_invocations)),
            subscription_permits: Arc::new(Semaphore::new(config.limits.max_subscriptions)),
            session_permits: Arc::new(Semaphore::new(config.limits.max_sessions)),
            config,
            outbound: parts.outbound,
            control: parts.control,
            closed: AtomicBool::new(false),
            correlation: AtomicU64::new(0),
            subscription_ids: AtomicU64::new(0),
            handshake: Mutex::new(None),
            invocations: Mutex::new(BTreeMap::new()),
            pending_subscriptions: Mutex::new(BTreeMap::new()),
            subscriptions: Mutex::new(BTreeMap::new()),
            pending_sessions: Mutex::new(BTreeMap::new()),
            sessions: Mutex::new(BTreeMap::new()),
            topology: RwLock::new(TopologySnapshot::default()),
            metrics: Metrics::new(),
        });
        let reader_runtime = Arc::clone(&runtime);
        tokio::spawn(async move {
            let mut inbound = parts.inbound;
            while let Some(item) = inbound.recv().await {
                match item {
                    Ok(envelope) => reader_runtime.accept(envelope),
                    Err(error) => {
                        reader_runtime.terminate(error);
                        return;
                    }
                }
            }
            reader_runtime.terminate(connection_error("adapter inbound stream ended"));
        });

        let correlation = runtime.next_correlation()?;
        let (reply, receive) = oneshot::channel();
        *runtime.handshake.lock().expect("handshake mutex poisoned") = Some((correlation, reply));
        if let Err(error) = runtime
            .send(ClientEnvelope {
                generation: runtime.config.protocol_generation,
                connection_generation: runtime.config.connection_generation,
                correlation_id: correlation,
                message: Some(client_envelope::Message::Handshake(Handshake {
                    generation: runtime.config.protocol_generation,
                    client_id: runtime.config.client_id.clone(),
                    requested: runtime
                        .config
                        .capabilities
                        .iter()
                        .copied()
                        .map(to_wire_capability)
                        .collect(),
                    connection_generation: runtime.config.connection_generation,
                })),
            })
            .await
        {
            runtime.terminate(error.clone());
            return Err(error);
        }
        let ack = match tokio::time::timeout(runtime.config.connect_timeout, receive).await {
            Ok(Ok(result)) => result.and_then(|ack| {
                validate_handshake(&runtime.config, &ack)?;
                Ok(ack)
            }),
            Ok(Err(_)) => Err(connection_error("HC/2 handshake owner disappeared")),
            Err(_) => Err(connection_error("HC/2 handshake timed out")),
        };
        let ack = match ack {
            Ok(ack) => ack,
            Err(error) => {
                runtime.terminate(error.clone());
                return Err(error);
            }
        };
        let cluster_id: Arc<str> = Arc::from(ack.cluster_id);
        if cluster_id.trim().is_empty() || cluster_id.len() > 128 {
            runtime.terminate(protocol_error("peer returned an invalid cluster identity"));
            return Err(protocol_error("peer returned an invalid cluster identity"));
        }
        let preferred_protocol_generation = if ack.preferred_generation == 0 {
            runtime.config.protocol_generation
        } else {
            ack.preferred_generation
        };
        Ok(Self {
            handle: Arc::new(ClientHandle { runtime }),
            cluster_id,
            transport,
            negotiated_generation_deprecated: ack.negotiated_generation_deprecated,
            preferred_protocol_generation,
        })
    }

    /// Authenticated cluster identity returned by negotiation.
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Explicit selected transport. HC/1 fallback is impossible.
    pub const fn transport(&self) -> TransportKind {
        self.transport
    }

    /// Exact wire generation selected for this connection.
    pub fn protocol_generation(&self) -> u32 {
        self.handle.runtime.config.protocol_generation
    }

    /// Preferred generation advertised by the peer, or the selected generation for legacy peers.
    pub const fn preferred_protocol_generation(&self) -> u32 {
        self.preferred_protocol_generation
    }

    /// Whether the peer accepted this generation only inside a deprecation window.
    pub const fn negotiated_generation_deprecated(&self) -> bool {
        self.negotiated_generation_deprecated
    }

    /// Read a value.
    pub async fn get(
        &self,
        key: Bytes,
        options: Option<RequestOptions>,
    ) -> Result<Option<CacheValue>, ClientError> {
        validate_key(&key)?;
        let response = self
            .invoke(
                invocation(client_envelope_request_meta(
                    &self.handle.runtime,
                    options.as_ref(),
                )?)
                .get(GetRequest { key }),
                options,
            )
            .await?;
        let Some(crate::wire::invocation_response::Result::Value(value)) = response.result else {
            return Err(protocol_error("peer omitted the value result"));
        };
        if !value.found {
            return Ok(None);
        }
        let expires_at = if value.expires_at_unix_ms == 0 {
            None
        } else {
            UNIX_EPOCH.checked_add(Duration::from_millis(value.expires_at_unix_ms))
        };
        Ok(Some(CacheValue {
            value: value.value,
            expires_at,
        }))
    }

    /// Store a value.
    pub async fn put(
        &self,
        key: Bytes,
        value: Bytes,
        ttl: Option<Duration>,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        validate_key(&key)?;
        validate_value(&value)?;
        let ttl_ms = ttl_millis(ttl)?;
        self.mutation(
            invocation(client_envelope_request_meta(
                &self.handle.runtime,
                options.as_ref(),
            )?)
            .put(PutRequest { key, value, ttl_ms }),
            options,
        )
        .await
    }

    /// Delete a value.
    pub async fn delete(
        &self,
        key: Bytes,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        validate_key(&key)?;
        self.mutation(
            invocation(client_envelope_request_meta(
                &self.handle.runtime,
                options.as_ref(),
            )?)
            .delete(DeleteRequest { key }),
            options,
        )
        .await
    }

    /// Atomically replace an expected value.
    pub async fn compare_and_set(
        &self,
        key: Bytes,
        expected: Bytes,
        replacement: Bytes,
        ttl: Option<Duration>,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        validate_key(&key)?;
        validate_value(&expected)?;
        validate_value(&replacement)?;
        let ttl_ms = ttl_millis(ttl)?;
        self.mutation(
            invocation(client_envelope_request_meta(
                &self.handle.runtime,
                options.as_ref(),
            )?)
            .compare_and_set(CompareAndSetRequest {
                key,
                expected,
                replacement,
                ttl_ms,
            }),
            options,
        )
        .await
    }

    /// Execute an ordered bounded batch.
    pub async fn batch(
        &self,
        operations: Vec<BatchOperation>,
        options: Option<RequestOptions>,
    ) -> Result<Vec<BatchItemResult>, ClientError> {
        if operations.is_empty()
            || operations.len() > self.handle.runtime.config.limits.max_batch_items
        {
            return Err(invalid("batch size is outside the configured range"));
        }
        let mut items = Vec::with_capacity(operations.len());
        for (index, operation) in operations.into_iter().enumerate() {
            let item_id =
                u32::try_from(index + 1).map_err(|_| invalid("batch item id overflow"))?;
            let operation = match operation {
                BatchOperation::Get { key } => {
                    validate_key(&key)?;
                    crate::wire::batch_item::Operation::Get(GetRequest { key })
                }
                BatchOperation::Put { key, value, ttl } => {
                    validate_key(&key)?;
                    validate_value(&value)?;
                    crate::wire::batch_item::Operation::Put(PutRequest {
                        key,
                        value,
                        ttl_ms: ttl_millis(ttl)?,
                    })
                }
                BatchOperation::Delete { key } => {
                    validate_key(&key)?;
                    crate::wire::batch_item::Operation::Delete(DeleteRequest { key })
                }
                BatchOperation::CompareAndSet {
                    key,
                    expected,
                    replacement,
                    ttl,
                } => {
                    validate_key(&key)?;
                    validate_value(&expected)?;
                    validate_value(&replacement)?;
                    crate::wire::batch_item::Operation::CompareAndSet(CompareAndSetRequest {
                        key,
                        expected,
                        replacement,
                        ttl_ms: ttl_millis(ttl)?,
                    })
                }
            };
            items.push(BatchItem {
                item_id,
                operation: Some(operation),
            });
        }
        let expected = items.len();
        let response = self
            .invoke(
                invocation(client_envelope_request_meta(
                    &self.handle.runtime,
                    options.as_ref(),
                )?)
                .batch(BatchRequest { items }),
                options,
            )
            .await?;
        let Some(crate::wire::invocation_response::Result::Batch(batch)) = response.result else {
            return Err(protocol_error("peer omitted the batch result"));
        };
        if batch.items.len() != expected {
            return Err(protocol_error("peer returned the wrong batch result count"));
        }
        batch
            .items
            .into_iter()
            .enumerate()
            .map(|(index, item)| map_batch_item(index, item))
            .collect()
    }

    /// Register a bounded subscription.
    pub async fn subscribe(
        &self,
        key_prefix: Bytes,
        resume_watermark: u64,
    ) -> Result<Subscription, ClientError> {
        if key_prefix.len() > 1024 * 1024 {
            return Err(invalid("subscription key prefix exceeds 1048576 bytes"));
        }
        let runtime = &self.handle.runtime;
        runtime.require_open()?;
        let permit = Arc::clone(&runtime.subscription_permits)
            .try_acquire_owned()
            .map_err(|_| quota_error("subscription limit reached"))?;
        let subscription_id = next_nonzero(&runtime.subscription_ids)?;
        let correlation = runtime.next_correlation()?;
        let (event_tx, event_rx) = mpsc::channel(runtime.config.limits.max_subscription_events);
        let (reply, receive) = oneshot::channel();
        runtime
            .pending_subscriptions
            .lock()
            .expect("pending subscription mutex poisoned")
            .insert(
                correlation,
                PendingSubscription {
                    subscription_id,
                    events: event_tx,
                    reply,
                    permit,
                },
            );
        let mut guard = PendingGuard::subscription(runtime, correlation, subscription_id);
        runtime
            .send(ClientEnvelope {
                generation: runtime.config.protocol_generation,
                connection_generation: runtime.config.connection_generation,
                correlation_id: correlation,
                message: Some(client_envelope::Message::Subscribe(Subscribe {
                    subscription_id,
                    key_prefix,
                    resume_watermark,
                })),
            })
            .await?;
        let ack = tokio::time::timeout(runtime.config.default_request_timeout, receive)
            .await
            .map_err(|_| timeout_error("subscription acknowledgement timed out"))?
            .map_err(|_| connection_error("subscription acknowledgement owner disappeared"))??;
        guard.disarm();
        Ok(Subscription {
            runtime: Arc::downgrade(runtime),
            id: ack.subscription_id,
            initial_watermark: ack.watermark,
            events: event_rx,
            closed: false,
        })
    }

    /// Open a fenced session and start bounded heartbeat ownership.
    pub async fn open_session(&self, ttl: Duration) -> Result<FencedSession, ClientError> {
        let ttl_ms = ttl_millis(Some(ttl))?;
        if ttl_ms == 0 {
            return Err(invalid("session TTL must be positive"));
        }
        let runtime = &self.handle.runtime;
        runtime.require_open()?;
        let permit = Arc::clone(&runtime.session_permits)
            .try_acquire_owned()
            .map_err(|_| quota_error("session limit reached"))?;
        let correlation = runtime.next_correlation()?;
        let (reply, receive) = oneshot::channel();
        runtime
            .pending_sessions
            .lock()
            .expect("pending session mutex poisoned")
            .insert(correlation, PendingSession { ttl, reply, permit });
        let mut guard = PendingGuard::session(runtime, correlation);
        runtime
            .send(ClientEnvelope {
                generation: runtime.config.protocol_generation,
                connection_generation: runtime.config.connection_generation,
                correlation_id: correlation,
                message: Some(client_envelope::Message::SessionOpen(SessionOpen {
                    requested_ttl_ms: ttl_ms,
                })),
            })
            .await?;
        let (id, fence) = tokio::time::timeout(runtime.config.default_request_timeout, receive)
            .await
            .map_err(|_| timeout_error("session acknowledgement timed out"))?
            .map_err(|_| connection_error("session acknowledgement owner disappeared"))??;
        guard.disarm();
        Ok(FencedSession {
            runtime: Arc::downgrade(runtime),
            id,
            fence,
            closed: false,
        })
    }

    /// Latest monotonic topology accepted by the runtime.
    pub fn topology(&self) -> TopologySnapshot {
        self.handle
            .runtime
            .topology
            .read()
            .expect("topology lock poisoned")
            .clone()
    }

    /// Pull a privacy-safe metrics snapshot.
    pub fn metrics(&self) -> ClientMetricsSnapshot {
        self.handle.runtime.metrics()
    }

    /// Idempotently stop I/O and fail every retained owner.
    pub fn close(&self) {
        self.handle
            .runtime
            .terminate(connection_error("client closed"));
    }

    async fn mutation(
        &self,
        request: InvocationRequest,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        let response = self.invoke(request, options).await?;
        let Some(crate::wire::invocation_response::Result::Mutation(mutation)) = response.result
        else {
            return Err(protocol_error("peer omitted the mutation result"));
        };
        Ok(MutationResult {
            applied: mutation.applied,
        })
    }

    async fn invoke(
        &self,
        request: InvocationRequest,
        options: Option<RequestOptions>,
    ) -> Result<InvocationResponse, ClientError> {
        let runtime = &self.handle.runtime;
        runtime.require_open()?;
        let timeout = options
            .as_ref()
            .map_or(runtime.config.default_request_timeout, |value| {
                value.timeout
            });
        if let Some(options) = &options {
            options.validate()?;
        }
        let permit = Arc::clone(&runtime.invocation_permits)
            .try_acquire_owned()
            .map_err(|_| quota_error("pending invocation limit reached"))?;
        let correlation = runtime.next_correlation()?;
        let (reply, receive) = oneshot::channel();
        runtime
            .invocations
            .lock()
            .expect("invocation mutex poisoned")
            .insert(
                correlation,
                PendingInvocation {
                    reply,
                    _permit: permit,
                },
            );
        runtime.metrics.submitted.fetch_add(1, Ordering::Relaxed);
        let mut guard = PendingGuard::invocation(runtime, correlation);
        runtime
            .send(ClientEnvelope {
                generation: runtime.config.protocol_generation,
                connection_generation: runtime.config.connection_generation,
                correlation_id: correlation,
                message: Some(client_envelope::Message::Invocation(request)),
            })
            .await?;
        let response = tokio::time::timeout(timeout, receive)
            .await
            .map_err(|_| timeout_error("local invocation deadline exceeded"))?
            .map_err(|_| connection_error("invocation owner disappeared"))??;
        guard.disarm();
        response_success(&response)?;
        Ok(response)
    }
}

/// Owned subscription event stream.
pub struct Subscription {
    runtime: Weak<Runtime>,
    id: u64,
    initial_watermark: u64,
    events: mpsc::Receiver<SubscriptionEvent>,
    closed: bool,
}

impl Subscription {
    pub const fn id(&self) -> u64 {
        self.id
    }
    pub const fn initial_watermark(&self) -> u64 {
        self.initial_watermark
    }
    pub async fn next(&mut self) -> Option<SubscriptionEvent> {
        self.events.recv().await
    }
    pub fn close(mut self) {
        self.close_inner();
    }
    fn close_inner(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.close_subscription(self.id);
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.close_inner();
    }
}

/// Owned fenced session. A lost or closed session returns `SessionLost`.
pub struct FencedSession {
    runtime: Weak<Runtime>,
    id: Bytes,
    fence: u64,
    closed: bool,
}

impl FencedSession {
    pub fn id(&self) -> &Bytes {
        &self.id
    }
    pub fn fence(&self) -> Result<u64, ClientError> {
        let runtime = self
            .runtime
            .upgrade()
            .ok_or_else(|| session_error("session runtime closed"))?;
        let sessions = runtime.sessions.lock().expect("session mutex poisoned");
        sessions
            .get(&self.id)
            .map(|session| session.fence.load(Ordering::Acquire))
            .ok_or_else(|| session_error("fenced session is no longer active"))
    }
    pub fn close(mut self) {
        self.close_inner();
    }
    fn close_inner(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        if let Some(runtime) = self.runtime.upgrade() {
            let current = runtime
                .sessions
                .lock()
                .expect("session mutex poisoned")
                .get(&self.id)
                .map_or(self.fence, |session| session.fence.load(Ordering::Acquire));
            runtime.close_session(&self.id, current);
        }
    }
}

impl Drop for FencedSession {
    fn drop(&mut self) {
        self.close_inner();
    }
}

enum PendingKind {
    Invocation,
    Subscription(u64),
    Session,
}

struct PendingGuard {
    runtime: Weak<Runtime>,
    correlation: u64,
    kind: PendingKind,
    armed: bool,
}

impl PendingGuard {
    fn invocation(runtime: &Arc<Runtime>, correlation: u64) -> Self {
        Self {
            runtime: Arc::downgrade(runtime),
            correlation,
            kind: PendingKind::Invocation,
            armed: true,
        }
    }
    fn subscription(runtime: &Arc<Runtime>, correlation: u64, id: u64) -> Self {
        Self {
            runtime: Arc::downgrade(runtime),
            correlation,
            kind: PendingKind::Subscription(id),
            armed: true,
        }
    }
    fn session(runtime: &Arc<Runtime>, correlation: u64) -> Self {
        Self {
            runtime: Arc::downgrade(runtime),
            correlation,
            kind: PendingKind::Session,
            armed: true,
        }
    }
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.cancel_pending(self.correlation, &self.kind);
        }
    }
}

impl Runtime {
    fn require_open(&self) -> Result<(), ClientError> {
        if self.closed.load(Ordering::Acquire) {
            Err(connection_error("client is closed"))
        } else {
            Ok(())
        }
    }

    fn next_correlation(&self) -> Result<u64, ClientError> {
        next_nonzero(&self.correlation)
    }

    async fn send(&self, envelope: ClientEnvelope) -> Result<(), ClientError> {
        self.require_open()?;
        if envelope.encoded_len() > self.config.limits.max_inbound_message_bytes {
            return Err(quota_error(
                "outbound HC/2 envelope exceeds the message limit",
            ));
        }
        self.outbound
            .send(envelope)
            .await
            .map_err(|_| connection_error("adapter outbound stream closed"))
    }

    fn try_send(&self, envelope: ClientEnvelope) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        if self.outbound.try_send(envelope).is_err() {
            self.metrics.failed.fetch_add(1, Ordering::Relaxed);
            self.terminate(connection_error(
                "bounded adapter outbound queue rejected control work",
            ));
        }
    }

    fn accept(self: &Arc<Self>, envelope: ServerEnvelope) {
        if envelope.generation != self.config.protocol_generation
            || envelope.connection_generation != self.config.connection_generation
        {
            self.terminate(protocol_error("stale or incompatible server envelope"));
            return;
        }
        match envelope.message {
            Some(server_envelope::Message::Handshake(ack)) => {
                self.accept_handshake(envelope.correlation_id, ack)
            }
            Some(server_envelope::Message::Invocation(response)) => {
                self.accept_invocation(envelope.correlation_id, response)
            }
            Some(server_envelope::Message::Subscribed(ack)) => {
                self.accept_subscription(envelope.correlation_id, ack)
            }
            Some(server_envelope::Message::Event(event)) => self.accept_event(event),
            Some(server_envelope::Message::Gap(gap)) => self.accept_gap(gap),
            Some(server_envelope::Message::Topology(update)) => self.accept_topology(update),
            Some(server_envelope::Message::SessionHeartbeat(heartbeat)) => {
                self.accept_session(envelope.correlation_id, heartbeat)
            }
            Some(server_envelope::Message::SessionLost(lost)) => {
                self.sessions
                    .lock()
                    .expect("session mutex poisoned")
                    .remove(&lost.session_id);
            }
            Some(server_envelope::Message::Diagnostics(_)) | None => {}
        }
    }

    fn accept_handshake(&self, correlation: u64, ack: HandshakeAck) {
        let pending = self
            .handshake
            .lock()
            .expect("handshake mutex poisoned")
            .take();
        match pending {
            Some((expected, reply)) if expected == correlation => {
                let _ = reply.send(Ok(ack));
            }
            _ => self.terminate(protocol_error("handshake correlation mismatch")),
        }
    }

    fn accept_invocation(&self, correlation: u64, response: InvocationResponse) {
        let pending = self
            .invocations
            .lock()
            .expect("invocation mutex poisoned")
            .remove(&correlation);
        let Some(pending) = pending else {
            self.metrics.late_responses.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if response_success(&response).is_ok() {
            self.metrics.completed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics.failed.fetch_add(1, Ordering::Relaxed);
        }
        let result = response_success(&response).map(|()| response);
        let _ = pending.reply.send(result);
    }

    fn accept_subscription(&self, correlation: u64, ack: SubscriptionAck) {
        let pending = self
            .pending_subscriptions
            .lock()
            .expect("pending subscription mutex poisoned")
            .remove(&correlation);
        let Some(pending) = pending else { return };
        if ack.subscription_id == 0 || ack.subscription_id != pending.subscription_id {
            let _ = pending.reply.send(Err(protocol_error(
                "subscription acknowledgement identity mismatch",
            )));
            self.terminate(protocol_error(
                "subscription acknowledgement identity mismatch",
            ));
            return;
        }
        self.subscriptions
            .lock()
            .expect("subscription mutex poisoned")
            .insert(
                ack.subscription_id,
                ActiveSubscription {
                    events: pending.events,
                    gap_after: 0,
                    _permit: pending.permit,
                },
            );
        if pending.reply.send(Ok(ack)).is_err() {
            self.close_subscription(ack.subscription_id);
        }
    }

    fn accept_event(&self, event: crate::wire::CacheEvent) {
        let mut subscriptions = self
            .subscriptions
            .lock()
            .expect("subscription mutex poisoned");
        let Some(subscription) = subscriptions.get_mut(&event.subscription_id) else {
            return;
        };
        if subscription.gap_after != 0 {
            if subscription
                .events
                .try_send(SubscriptionEvent::Gap {
                    subscription_id: event.subscription_id,
                    after_watermark: subscription.gap_after,
                })
                .is_err()
            {
                subscription.gap_after = subscription.gap_after.max(event.watermark);
                self.metrics.dropped_events.fetch_add(1, Ordering::Relaxed);
                return;
            }
            subscription.gap_after = 0;
        }
        self.metrics.events.fetch_add(1, Ordering::Relaxed);
        let message = SubscriptionEvent::Event(CacheEvent {
            subscription_id: event.subscription_id,
            watermark: event.watermark,
            key: event.key,
            value: event.value,
            removed: event.removed,
        });
        if subscription.events.try_send(message).is_err() {
            subscription.gap_after = subscription.gap_after.max(event.watermark);
            self.metrics.dropped_events.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn accept_gap(&self, gap: crate::wire::EventGap) {
        let mut subscriptions = self
            .subscriptions
            .lock()
            .expect("subscription mutex poisoned");
        let Some(subscription) = subscriptions.get_mut(&gap.subscription_id) else {
            return;
        };
        if subscription
            .events
            .try_send(SubscriptionEvent::Gap {
                subscription_id: gap.subscription_id,
                after_watermark: gap.after_watermark,
            })
            .is_err()
        {
            subscription.gap_after = subscription.gap_after.max(gap.after_watermark);
            self.metrics.dropped_events.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn accept_topology(&self, update: crate::wire::TopologyUpdate) {
        let mut current = self.topology.write().expect("topology lock poisoned");
        if update.epoch <= current.epoch {
            return;
        }
        if update.nodes.len() > self.config.limits.max_topology_nodes {
            drop(current);
            self.terminate(quota_error("topology node limit exceeded"));
            return;
        }
        let mut identities = BTreeSet::new();
        let mut nodes = Vec::with_capacity(update.nodes.len());
        for node in update.nodes {
            if node.node_id.trim().is_empty()
                || node.node_id.len() > 128
                || node.node_epoch == 0
                || node.server_name.trim().is_empty()
                || !(node.endpoint_uri.starts_with("hc2+grpc://")
                    || node.endpoint_uri.starts_with("hc2+h2://")
                    || node.endpoint_uri.starts_with("hc2+tls://"))
                || !identities.insert(node.node_id.clone())
            {
                drop(current);
                self.terminate(protocol_error("peer sent malformed topology"));
                return;
            }
            nodes.push(NodeEndpoint {
                node_id: node.node_id,
                node_epoch: node.node_epoch,
                endpoint_uri: node.endpoint_uri,
                server_name: node.server_name,
            });
        }
        *current = TopologySnapshot {
            epoch: update.epoch,
            nodes,
        };
    }

    fn accept_session(self: &Arc<Self>, correlation: u64, heartbeat: SessionHeartbeat) {
        if let Some(pending) = self
            .pending_sessions
            .lock()
            .expect("pending session mutex poisoned")
            .remove(&correlation)
        {
            if heartbeat.session_id.is_empty() || heartbeat.fence == 0 {
                let _ = pending
                    .reply
                    .send(Err(protocol_error("invalid session acknowledgement")));
                self.terminate(protocol_error("invalid session acknowledgement"));
                return;
            }
            let id = heartbeat.session_id.clone();
            let fence = Arc::new(AtomicU64::new(heartbeat.fence));
            self.sessions
                .lock()
                .expect("session mutex poisoned")
                .insert(
                    id.clone(),
                    ActiveSession {
                        fence: Arc::clone(&fence),
                        _permit: pending.permit,
                    },
                );
            if pending
                .reply
                .send(Ok((id.clone(), heartbeat.fence)))
                .is_err()
            {
                self.close_session(&id, heartbeat.fence);
                return;
            }
            let weak = Arc::downgrade(self);
            let period = std::cmp::max(Duration::from_millis(100), pending.ttl / 3);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(period);
                interval.tick().await;
                loop {
                    interval.tick().await;
                    let Some(runtime) = weak.upgrade() else {
                        return;
                    };
                    if runtime.closed.load(Ordering::Acquire) {
                        return;
                    }
                    let current = runtime
                        .sessions
                        .lock()
                        .expect("session mutex poisoned")
                        .get(&id)
                        .map(|session| session.fence.load(Ordering::Acquire));
                    let Some(current) = current else { return };
                    let Ok(correlation) = runtime.next_correlation() else {
                        runtime.terminate(protocol_error("correlation id exhausted"));
                        return;
                    };
                    runtime.try_send(ClientEnvelope {
                        generation: runtime.config.protocol_generation,
                        connection_generation: runtime.config.connection_generation,
                        correlation_id: correlation,
                        message: Some(client_envelope::Message::SessionHeartbeat(
                            SessionHeartbeat {
                                session_id: id.clone(),
                                fence: current,
                            },
                        )),
                    });
                }
            });
        } else if let Some(session) = self
            .sessions
            .lock()
            .expect("session mutex poisoned")
            .get(&heartbeat.session_id)
        {
            session.fence.fetch_max(heartbeat.fence, Ordering::AcqRel);
        }
    }

    fn cancel_pending(&self, correlation: u64, kind: &PendingKind) {
        let removed = match kind {
            PendingKind::Invocation => self
                .invocations
                .lock()
                .expect("invocation mutex poisoned")
                .remove(&correlation)
                .is_some(),
            PendingKind::Subscription(_) => self
                .pending_subscriptions
                .lock()
                .expect("pending subscription mutex poisoned")
                .remove(&correlation)
                .is_some(),
            PendingKind::Session => self
                .pending_sessions
                .lock()
                .expect("pending session mutex poisoned")
                .remove(&correlation)
                .is_some(),
        };
        if !removed {
            return;
        }
        self.metrics.cancelled.fetch_add(1, Ordering::Relaxed);
        if matches!(kind, PendingKind::Invocation) {
            let Ok(control_correlation) = self.next_correlation() else {
                self.terminate(protocol_error("correlation id exhausted"));
                return;
            };
            self.try_send(ClientEnvelope {
                generation: self.config.protocol_generation,
                connection_generation: self.config.connection_generation,
                correlation_id: control_correlation,
                message: Some(client_envelope::Message::Cancel(Cancel {
                    correlation_id: correlation,
                    safe_reason: "caller cancelled or deadline elapsed".to_owned(),
                })),
            });
        } else if let PendingKind::Subscription(id) = kind {
            let Ok(control_correlation) = self.next_correlation() else {
                self.terminate(protocol_error("correlation id exhausted"));
                return;
            };
            self.try_send(ClientEnvelope {
                generation: self.config.protocol_generation,
                connection_generation: self.config.connection_generation,
                correlation_id: control_correlation,
                message: Some(client_envelope::Message::Unsubscribe(Unsubscribe {
                    subscription_id: *id,
                })),
            });
        }
    }

    fn close_subscription(&self, id: u64) {
        if self
            .subscriptions
            .lock()
            .expect("subscription mutex poisoned")
            .remove(&id)
            .is_some()
        {
            let Ok(control_correlation) = self.next_correlation() else {
                self.terminate(protocol_error("correlation id exhausted"));
                return;
            };
            self.try_send(ClientEnvelope {
                generation: self.config.protocol_generation,
                connection_generation: self.config.connection_generation,
                correlation_id: control_correlation,
                message: Some(client_envelope::Message::Unsubscribe(Unsubscribe {
                    subscription_id: id,
                })),
            });
        }
    }

    fn close_session(&self, id: &Bytes, fence: u64) {
        if self
            .sessions
            .lock()
            .expect("session mutex poisoned")
            .remove(id)
            .is_some()
        {
            let Ok(control_correlation) = self.next_correlation() else {
                self.terminate(protocol_error("correlation id exhausted"));
                return;
            };
            self.try_send(ClientEnvelope {
                generation: self.config.protocol_generation,
                connection_generation: self.config.connection_generation,
                correlation_id: control_correlation,
                message: Some(client_envelope::Message::SessionClose(SessionClose {
                    session_id: id.clone(),
                    fence,
                })),
            });
        }
    }

    fn terminate(&self, error: ClientError) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        self.control.close();
        if let Some((_, reply)) = self
            .handshake
            .lock()
            .expect("handshake mutex poisoned")
            .take()
        {
            let _ = reply.send(Err(error.clone()));
        }
        for (_, pending) in
            std::mem::take(&mut *self.invocations.lock().expect("invocation mutex poisoned"))
        {
            let _ = pending.reply.send(Err(error.clone()));
        }
        for (_, pending) in std::mem::take(
            &mut *self
                .pending_subscriptions
                .lock()
                .expect("pending subscription mutex poisoned"),
        ) {
            let _ = pending.reply.send(Err(error.clone()));
        }
        for (_, pending) in std::mem::take(
            &mut *self
                .pending_sessions
                .lock()
                .expect("pending session mutex poisoned"),
        ) {
            let _ = pending.reply.send(Err(error.clone()));
        }
        for (_, subscription) in std::mem::take(
            &mut *self
                .subscriptions
                .lock()
                .expect("subscription mutex poisoned"),
        ) {
            let _ = subscription
                .events
                .try_send(SubscriptionEvent::Closed(error.clone()));
        }
        self.sessions
            .lock()
            .expect("session mutex poisoned")
            .clear();
    }

    fn metrics(&self) -> ClientMetricsSnapshot {
        ClientMetricsSnapshot {
            submitted: self.metrics.submitted.load(Ordering::Relaxed),
            completed: self.metrics.completed.load(Ordering::Relaxed),
            failed: self.metrics.failed.load(Ordering::Relaxed),
            cancelled: self.metrics.cancelled.load(Ordering::Relaxed),
            late_responses: self.metrics.late_responses.load(Ordering::Relaxed),
            events: self.metrics.events.load(Ordering::Relaxed),
            dropped_events: self.metrics.dropped_events.load(Ordering::Relaxed),
            pending_invocations: self
                .invocations
                .lock()
                .expect("invocation mutex poisoned")
                .len(),
            active_subscriptions: self
                .subscriptions
                .lock()
                .expect("subscription mutex poisoned")
                .len(),
            active_sessions: self.sessions.lock().expect("session mutex poisoned").len(),
        }
    }
}

fn invocation(meta: RequestMeta) -> InvocationBuilder {
    InvocationBuilder(InvocationRequest {
        meta: Some(meta),
        operation: None,
    })
}

struct InvocationBuilder(InvocationRequest);
impl InvocationBuilder {
    fn get(mut self, value: GetRequest) -> InvocationRequest {
        self.0.operation = Some(crate::wire::invocation_request::Operation::Get(value));
        self.0
    }
    fn put(mut self, value: PutRequest) -> InvocationRequest {
        self.0.operation = Some(crate::wire::invocation_request::Operation::Put(value));
        self.0
    }
    fn delete(mut self, value: DeleteRequest) -> InvocationRequest {
        self.0.operation = Some(crate::wire::invocation_request::Operation::Delete(value));
        self.0
    }
    fn compare_and_set(mut self, value: CompareAndSetRequest) -> InvocationRequest {
        self.0.operation = Some(crate::wire::invocation_request::Operation::CompareAndSet(
            value,
        ));
        self.0
    }
    fn batch(mut self, value: BatchRequest) -> InvocationRequest {
        self.0.operation = Some(crate::wire::invocation_request::Operation::Batch(value));
        self.0
    }
}

fn client_envelope_request_meta(
    runtime: &Runtime,
    options: Option<&RequestOptions>,
) -> Result<RequestMeta, ClientError> {
    if let Some(options) = options {
        options.validate()?;
    }
    let timeout = options.map_or(runtime.config.default_request_timeout, |value| {
        value.timeout
    });
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| invalid("system clock is before Unix epoch"))?;
    let deadline = now
        .checked_add(timeout)
        .ok_or_else(|| invalid("deadline overflow"))?;
    Ok(RequestMeta {
        deadline_unix_ms: u64::try_from(deadline.as_millis())
            .map_err(|_| invalid("deadline overflow"))?,
        idempotency_key: options.map_or_else(Bytes::new, |value| value.idempotency_key.clone()),
        tenant: runtime.config.tenant.clone(),
        topology_epoch: runtime
            .topology
            .read()
            .expect("topology lock poisoned")
            .epoch,
    })
}

fn map_batch_item(
    index: usize,
    response: InvocationResponse,
) -> Result<BatchItemResult, ClientError> {
    response_success(&response)?;
    let item_id = u32::try_from(index + 1).map_err(|_| protocol_error("batch item id overflow"))?;
    let (value, mutation) = match response.result {
        Some(crate::wire::invocation_response::Result::Value(value)) if value.found => (
            Some(CacheValue {
                value: value.value,
                expires_at: if value.expires_at_unix_ms == 0 {
                    None
                } else {
                    UNIX_EPOCH.checked_add(Duration::from_millis(value.expires_at_unix_ms))
                },
            }),
            None,
        ),
        Some(crate::wire::invocation_response::Result::Value(_)) => (None, None),
        Some(crate::wire::invocation_response::Result::Mutation(value)) => (
            None,
            Some(MutationResult {
                applied: value.applied,
            }),
        ),
        _ => return Err(protocol_error("peer returned an invalid batch item result")),
    };
    Ok(BatchItemResult {
        item_id,
        value,
        mutation,
    })
}

fn response_success(response: &InvocationResponse) -> Result<(), ClientError> {
    let meta = response
        .meta
        .as_ref()
        .ok_or_else(|| protocol_error("peer omitted response metadata"))?;
    let code = crate::wire::StableErrorCode::try_from(meta.error)
        .map_err(|_| protocol_error("peer returned an unknown stable error code"))?;
    if code == crate::wire::StableErrorCode::StableErrorUnspecified {
        return Ok(());
    }
    Err(ClientError::new(
        map_error(code),
        map_retry(meta.retry)?,
        "peer rejected HC/2 operation",
    ))
}

fn validate_handshake(config: &ClientConfig, ack: &HandshakeAck) -> Result<(), ClientError> {
    if ack.generation != config.protocol_generation
        || ack.connection_generation != config.connection_generation
    {
        return Err(protocol_error(
            "peer negotiated an unexpected HC/2 generation",
        ));
    }
    if (ack.minimum_generation != 0 && ack.generation < ack.minimum_generation)
        || (ack.preferred_generation != 0 && ack.generation > ack.preferred_generation)
        || (ack.negotiated_generation_deprecated
            && ack.preferred_generation != 0
            && ack.generation >= ack.preferred_generation)
    {
        return Err(protocol_error(
            "peer returned inconsistent HC/2 generation policy",
        ));
    }
    let accepted = ack
        .accepted
        .iter()
        .filter_map(|value| crate::wire::Capability::try_from(*value).ok())
        .filter_map(from_wire_capability)
        .collect::<BTreeSet<_>>();
    if config
        .capabilities
        .iter()
        .any(|capability| !accepted.contains(capability))
    {
        return Err(protocol_error("peer omitted a required HC/2 capability"));
    }
    Ok(())
}

fn to_wire_capability(value: Capability) -> i32 {
    match value {
        Capability::Data => crate::wire::Capability::Data as i32,
        Capability::Batch => crate::wire::Capability::Batch as i32,
        Capability::Subscriptions => crate::wire::Capability::Subscriptions as i32,
        Capability::Topology => crate::wire::Capability::Topology as i32,
        Capability::FencedSessions => crate::wire::Capability::FencedSessions as i32,
        Capability::Diagnostics => crate::wire::Capability::Diagnostics as i32,
    }
}

fn from_wire_capability(value: crate::wire::Capability) -> Option<Capability> {
    match value {
        crate::wire::Capability::Data => Some(Capability::Data),
        crate::wire::Capability::Batch => Some(Capability::Batch),
        crate::wire::Capability::Subscriptions => Some(Capability::Subscriptions),
        crate::wire::Capability::Topology => Some(Capability::Topology),
        crate::wire::Capability::FencedSessions => Some(Capability::FencedSessions),
        crate::wire::Capability::Diagnostics => Some(Capability::Diagnostics),
        crate::wire::Capability::Unspecified => None,
    }
}

fn map_error(value: crate::wire::StableErrorCode) -> ErrorCode {
    match value {
        crate::wire::StableErrorCode::StableErrorInvalidRequest => ErrorCode::InvalidRequest,
        crate::wire::StableErrorCode::StableErrorUnauthenticated => ErrorCode::Unauthenticated,
        crate::wire::StableErrorCode::StableErrorUnauthorized => ErrorCode::Unauthorized,
        crate::wire::StableErrorCode::StableErrorNotFound => ErrorCode::NotFound,
        crate::wire::StableErrorCode::StableErrorConflict => ErrorCode::Conflict,
        crate::wire::StableErrorCode::StableErrorQuotaExceeded => ErrorCode::QuotaExceeded,
        crate::wire::StableErrorCode::StableErrorDeadlineExceeded => ErrorCode::DeadlineExceeded,
        crate::wire::StableErrorCode::StableErrorUnavailable => ErrorCode::Unavailable,
        crate::wire::StableErrorCode::StableErrorUnsupported => ErrorCode::Unsupported,
        crate::wire::StableErrorCode::StableErrorGapRepairRequired => ErrorCode::GapRepairRequired,
        crate::wire::StableErrorCode::StableErrorSessionLost => ErrorCode::SessionLost,
        crate::wire::StableErrorCode::StableErrorInternal
        | crate::wire::StableErrorCode::StableErrorUnspecified => ErrorCode::Internal,
    }
}

fn map_retry(value: i32) -> Result<RetryAdvice, ClientError> {
    Ok(
        match crate::wire::RetryDirective::try_from(value)
            .map_err(|_| protocol_error("peer returned an unknown retry directive"))?
        {
            crate::wire::RetryDirective::SameConnection => RetryAdvice::SameConnection,
            crate::wire::RetryDirective::ReconnectIdempotent => RetryAdvice::ReconnectIdempotent,
            crate::wire::RetryDirective::RepairRequired => RetryAdvice::RepairRequired,
            crate::wire::RetryDirective::Never | crate::wire::RetryDirective::Unspecified => {
                RetryAdvice::Never
            }
        },
    )
}

fn next_nonzero(sequence: &AtomicU64) -> Result<u64, ClientError> {
    sequence
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .map(|value| value + 1)
        .map_err(|_| protocol_error("monotonic identifier exhausted"))
}

const fn protocol_error(detail: &'static str) -> ClientError {
    ClientError::new(ErrorCode::Internal, RetryAdvice::Never, detail)
}
const fn connection_error(detail: &'static str) -> ClientError {
    ClientError::new(
        ErrorCode::Unavailable,
        RetryAdvice::ReconnectIdempotent,
        detail,
    )
}
const fn quota_error(detail: &'static str) -> ClientError {
    ClientError::new(ErrorCode::QuotaExceeded, RetryAdvice::Never, detail)
}
const fn timeout_error(detail: &'static str) -> ClientError {
    ClientError::new(ErrorCode::DeadlineExceeded, RetryAdvice::Never, detail)
}
const fn session_error(detail: &'static str) -> ClientError {
    ClientError::new(ErrorCode::SessionLost, RetryAdvice::RepairRequired, detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(error: i32, retry: i32) -> InvocationResponse {
        InvocationResponse {
            meta: Some(crate::wire::ResponseMeta {
                error,
                retry,
                safe_detail: String::new(),
                topology_epoch: 1,
            }),
            result: None,
        }
    }

    #[test]
    fn unknown_error_and_retry_numbers_fail_closed() {
        assert!(response_success(&response(i32::MAX, 1)).is_err());
        assert!(response_success(&response(
            crate::wire::StableErrorCode::StableErrorInternal as i32,
            i32::MAX,
        ))
        .is_err());
    }
}
