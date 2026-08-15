//! Bounded reconnect, subscription repair, and fail-loud fenced-session loss.
//!
//! Reconnect ownership is deliberately separate from a single HC/2 connection
//! and from invocation retry. A connection never changes generation in place;
//! a successful reconnect installs a newly negotiated `Hc2Client` and fences
//! every owner of the previous generation.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;

use bytes::Bytes;
use tokio::sync::{mpsc, Mutex as AsyncMutex, Notify};
use tokio::time::Instant;

use crate::adapter::TransportAdapter;
use crate::client::SubscriptionControl;
use crate::{
    BatchItemResult, BatchOperation, CacheEvent, CacheValue, ClientConfig, ClientError,
    ClientRetainedStateSnapshot, ErrorCode, FencedSession, Hc2Client, LockAcquireResult,
    LockOwnership, MutationResult, RequestOptions, RetryAdvice, Subscription, SubscriptionEvent,
    TransportKind,
};

/// One named endpoint and its already validated, security-owning adapter.
#[derive(Clone)]
pub struct ReconnectEndpoint {
    id: Arc<str>,
    adapter: Arc<dyn TransportAdapter>,
}

impl std::fmt::Debug for ReconnectEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReconnectEndpoint")
            .field("id", &self.id)
            .field("transport", &self.adapter.kind())
            .finish_non_exhaustive()
    }
}

impl ReconnectEndpoint {
    /// Bind a privacy-safe endpoint identity to one transport adapter.
    pub fn new(
        id: impl Into<String>,
        adapter: Arc<dyn TransportAdapter>,
    ) -> Result<Self, ClientError> {
        let id = id.into();
        if id.trim().is_empty() || id.len() > 128 || id.chars().any(char::is_control) {
            return Err(invalid("reconnect endpoint id is invalid"));
        }
        Ok(Self {
            id: Arc::from(id),
            adapter,
        })
    }

    /// Stable, non-secret endpoint identity used by leader-hint selection.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Adapter kind. All configured endpoints must use the same kind.
    pub fn transport(&self) -> TransportKind {
        self.adapter.kind()
    }
}

/// Connection-establishment policy. It never authorizes replaying an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    max_attempts: usize,
    initial_backoff: Duration,
    max_backoff: Duration,
    jitter: Duration,
    seed: u64,
}

impl ReconnectPolicy {
    /// Create a bounded deterministic reconnect policy.
    pub fn new(
        max_attempts: usize,
        initial_backoff: Duration,
        max_backoff: Duration,
        jitter: Duration,
        seed: u64,
    ) -> Result<Self, ClientError> {
        let value = Self {
            max_attempts,
            initial_backoff,
            max_backoff,
            jitter,
            seed,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<Self, ClientError> {
        if self.max_attempts == 0
            || self.max_attempts > 32
            || self.initial_backoff.is_zero()
            || self.initial_backoff > self.max_backoff
            || self.max_backoff > Duration::from_secs(30)
            || self.jitter > self.max_backoff
        {
            return Err(invalid("reconnect policy is outside supported bounds"));
        }
        Ok(self)
    }

    fn delay(self, attempt: usize) -> Duration {
        let shift = u32::try_from(attempt.saturating_sub(1).min(20)).unwrap_or(20);
        let multiplier = 1u32.checked_shl(shift).unwrap_or(u32::MAX);
        let base = self
            .initial_backoff
            .saturating_mul(multiplier)
            .min(self.max_backoff);
        if self.jitter.is_zero() {
            return base;
        }
        let jitter_nanos = self.jitter.as_nanos();
        let mixed = mix64(self.seed ^ u64::try_from(attempt).unwrap_or(u64::MAX));
        let offset = u128::from(mixed) % jitter_nanos.saturating_add(1);
        base.saturating_add(duration_from_nanos(offset))
            .min(self.max_backoff)
    }
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(25),
            max_backoff: Duration::from_secs(2),
            jitter: Duration::from_millis(25),
            seed: 0x6843_3200_0011,
        }
    }
}

/// Operation replay policy, intentionally distinct from reconnect attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvocationRetryPolicy {
    max_attempts: usize,
}

impl InvocationRetryPolicy {
    /// Bound total operation attempts, including the original attempt.
    pub fn new(max_attempts: usize) -> Result<Self, ClientError> {
        if max_attempts == 0 || max_attempts > 4 {
            return Err(invalid("invocation retry attempts must be in [1, 4]"));
        }
        Ok(Self { max_attempts })
    }
}

impl Default for InvocationRetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 2 }
    }
}

/// Pull-based recovery counters with no endpoint, tenant, key, or value labels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoveryMetricsSnapshot {
    pub reconnect_attempts: u64,
    pub reconnect_successes: u64,
    pub reconnect_exhausted: u64,
    pub invocation_retries: u64,
    pub subscription_repairs: u64,
    pub duplicate_events: u64,
    pub gap_repairs_required: u64,
    pub session_losses: u64,
}

/// Privacy-safe snapshot of allocation-owning recovery resources.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoveryRetainedStateSnapshot {
    pub logical_subscriptions: usize,
    pub session_registrations: usize,
    pub live_sessions: usize,
    pub current_client: ClientRetainedStateSnapshot,
    pub closed: bool,
}

struct RecoveryMetrics {
    reconnect_attempts: AtomicU64,
    reconnect_successes: AtomicU64,
    reconnect_exhausted: AtomicU64,
    invocation_retries: AtomicU64,
    subscription_repairs: AtomicU64,
    duplicate_events: AtomicU64,
    gap_repairs_required: AtomicU64,
    session_losses: AtomicU64,
}

impl RecoveryMetrics {
    const fn new() -> Self {
        Self {
            reconnect_attempts: AtomicU64::new(0),
            reconnect_successes: AtomicU64::new(0),
            reconnect_exhausted: AtomicU64::new(0),
            invocation_retries: AtomicU64::new(0),
            subscription_repairs: AtomicU64::new(0),
            duplicate_events: AtomicU64::new(0),
            gap_repairs_required: AtomicU64::new(0),
            session_losses: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> RecoveryMetricsSnapshot {
        RecoveryMetricsSnapshot {
            reconnect_attempts: self.reconnect_attempts.load(Ordering::Relaxed),
            reconnect_successes: self.reconnect_successes.load(Ordering::Relaxed),
            reconnect_exhausted: self.reconnect_exhausted.load(Ordering::Relaxed),
            invocation_retries: self.invocation_retries.load(Ordering::Relaxed),
            subscription_repairs: self.subscription_repairs.load(Ordering::Relaxed),
            duplicate_events: self.duplicate_events.load(Ordering::Relaxed),
            gap_repairs_required: self.gap_repairs_required.load(Ordering::Relaxed),
            session_losses: self.session_losses.load(Ordering::Relaxed),
        }
    }
}

struct CurrentConnection {
    generation: u64,
    endpoint_index: usize,
    client: Arc<Hc2Client>,
}

struct RecoveringRuntime {
    config: ClientConfig,
    endpoints: Vec<ReconnectEndpoint>,
    reconnect_policy: ReconnectPolicy,
    retry_policy: InvocationRetryPolicy,
    preferred_endpoint: AtomicUsize,
    current: RwLock<CurrentConnection>,
    reconnect_owner: AsyncMutex<()>,
    subscription_ids: AtomicU64,
    session_ids: AtomicU64,
    subscriptions: Mutex<BTreeMap<u64, Arc<SubscriptionState>>>,
    sessions: Mutex<BTreeMap<u64, Weak<RecoveringSessionState>>>,
    metrics: Arc<RecoveryMetrics>,
    closed: AtomicBool,
}

struct RecoveringHandle {
    runtime: Arc<RecoveringRuntime>,
}

impl Drop for RecoveringHandle {
    fn drop(&mut self) {
        self.runtime
            .close(connection_error("last recovery handle dropped"));
    }
}

/// Cloneable HC/2 client with bounded reconnect and conservative repair.
#[derive(Clone)]
pub struct RecoveringHc2Client {
    handle: Arc<RecoveringHandle>,
}

impl std::fmt::Debug for RecoveringHc2Client {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecoveringHc2Client")
            .field("endpoint", &self.current_endpoint())
            .field("generation", &self.connection_generation())
            .finish_non_exhaustive()
    }
}

impl RecoveringHc2Client {
    /// Connect using a bounded ordered endpoint set.
    pub async fn connect(
        endpoints: Vec<ReconnectEndpoint>,
        config: ClientConfig,
        reconnect_policy: ReconnectPolicy,
        retry_policy: InvocationRetryPolicy,
    ) -> Result<Self, ClientError> {
        let config = config.validate()?;
        reconnect_policy.validate()?;
        validate_endpoints(&endpoints)?;
        let metrics = Arc::new(RecoveryMetrics::new());
        let initial = establish(
            &endpoints,
            &config,
            reconnect_policy,
            0,
            config.connection_generation,
            &metrics,
            EstablishMode::Initial,
        )
        .await?;
        Ok(Self {
            handle: Arc::new(RecoveringHandle {
                runtime: Arc::new(RecoveringRuntime {
                    config,
                    endpoints,
                    reconnect_policy,
                    retry_policy,
                    preferred_endpoint: AtomicUsize::new(initial.endpoint_index),
                    current: RwLock::new(initial),
                    reconnect_owner: AsyncMutex::new(()),
                    subscription_ids: AtomicU64::new(0),
                    session_ids: AtomicU64::new(0),
                    subscriptions: Mutex::new(BTreeMap::new()),
                    sessions: Mutex::new(BTreeMap::new()),
                    metrics,
                    closed: AtomicBool::new(false),
                }),
            }),
        })
    }

    /// Current endpoint identity after the latest completed reconnect.
    pub fn current_endpoint(&self) -> String {
        let current = self
            .handle
            .runtime
            .current
            .read()
            .expect("recovery current lock poisoned");
        self.handle.runtime.endpoints[current.endpoint_index]
            .id()
            .to_owned()
    }

    /// Current nonzero wire generation.
    pub fn connection_generation(&self) -> u64 {
        self.handle
            .runtime
            .current
            .read()
            .expect("recovery current lock poisoned")
            .generation
    }

    /// Negotiated HC/2 protocol generation for the current connection.
    pub fn protocol_generation(&self) -> u32 {
        self.handle
            .runtime
            .current
            .read()
            .expect("recovery current lock poisoned")
            .client
            .protocol_generation()
    }

    /// Server-preferred HC/2 generation advertised during the current handshake.
    pub fn preferred_protocol_generation(&self) -> u32 {
        self.handle
            .runtime
            .current
            .read()
            .expect("recovery current lock poisoned")
            .client
            .preferred_protocol_generation()
    }

    /// Whether the selected generation is inside a supported deprecation window.
    pub fn negotiated_generation_deprecated(&self) -> bool {
        self.handle
            .runtime
            .current
            .read()
            .expect("recovery current lock poisoned")
            .client
            .negotiated_generation_deprecated()
    }

    /// Prefer a known endpoint for the next reconnect (for example, a verified leader hint).
    pub fn prefer_endpoint(&self, endpoint_id: &str) -> Result<(), ClientError> {
        let Some(index) = self
            .handle
            .runtime
            .endpoints
            .iter()
            .position(|endpoint| endpoint.id() == endpoint_id)
        else {
            return Err(invalid("leader hint names an unknown endpoint"));
        };
        self.handle
            .runtime
            .preferred_endpoint
            .store(index, Ordering::Release);
        Ok(())
    }

    /// Explicitly replace the current connection without replaying an operation.
    pub async fn reconnect_now(&self) -> Result<(), ClientError> {
        let generation = self.connection_generation();
        self.handle
            .runtime
            .reconnect_if_generation(generation)
            .await
    }

    /// Read with a bounded safe replay after an authenticated transport break.
    pub async fn get(
        &self,
        key: Bytes,
        options: Option<RequestOptions>,
    ) -> Result<Option<CacheValue>, ClientError> {
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(true, timeout, move |client, remaining| {
            let key = key.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.get(key, options).await }
        })
        .await
    }

    /// Store with replay allowed only when an idempotency key is present.
    pub async fn put(
        &self,
        key: Bytes,
        value: Bytes,
        ttl: Option<Duration>,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        let retry_safe = has_idempotency_key(options.as_ref());
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(retry_safe, timeout, move |client, remaining| {
            let key = key.clone();
            let value = value.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.put(key, value, ttl, options).await }
        })
        .await
    }

    /// Delete with replay allowed only when an idempotency key is present.
    pub async fn delete(
        &self,
        key: Bytes,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        let retry_safe = has_idempotency_key(options.as_ref());
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(retry_safe, timeout, move |client, remaining| {
            let key = key.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.delete(key, options).await }
        })
        .await
    }

    /// Compare-and-set with replay allowed only when an idempotency key is present.
    pub async fn compare_and_set(
        &self,
        key: Bytes,
        expected: Bytes,
        replacement: Bytes,
        ttl: Option<Duration>,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        let retry_safe = has_idempotency_key(options.as_ref());
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(retry_safe, timeout, move |client, remaining| {
            let key = key.clone();
            let expected = expected.clone();
            let replacement = replacement.clone();
            let options = adjusted_options(&options, remaining);
            async move {
                client
                    .compare_and_set(key, expected, replacement, ttl, options)
                    .await
            }
        })
        .await
    }

    /// Delete an exact value, replaying only when an idempotency key is present.
    pub async fn remove_if_value(
        &self,
        key: Bytes,
        expected: Bytes,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        let retry_safe = has_idempotency_key(options.as_ref());
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(retry_safe, timeout, move |client, remaining| {
            let key = key.clone();
            let expected = expected.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.remove_if_value(key, expected, options).await }
        })
        .await
    }

    /// Try once to acquire a fenced lock. An ambiguous result is never replayed.
    pub async fn try_lock(
        &self,
        key: Bytes,
        lease: Duration,
        options: Option<RequestOptions>,
    ) -> Result<LockAcquireResult, ClientError> {
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(false, timeout, move |client, remaining| {
            let key = key.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.try_lock(key, lease, options).await }
        })
        .await
    }

    /// Release once by fence. An ambiguous result is never replayed.
    pub async fn unlock(
        &self,
        key: Bytes,
        fence: u64,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(false, timeout, move |client, remaining| {
            let key = key.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.unlock(key, fence, options).await }
        })
        .await
    }

    /// Renew once by fence. An ambiguous result is never replayed.
    pub async fn renew_lock(
        &self,
        key: Bytes,
        fence: u64,
        lease: Duration,
        options: Option<RequestOptions>,
    ) -> Result<MutationResult, ClientError> {
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(false, timeout, move |client, remaining| {
            let key = key.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.renew_lock(key, fence, lease, options).await }
        })
        .await
    }

    /// Read lock ownership with a bounded safe replay.
    pub async fn lock_ownership(
        &self,
        key: Bytes,
        options: Option<RequestOptions>,
    ) -> Result<LockOwnership, ClientError> {
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(true, timeout, move |client, remaining| {
            let key = key.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.lock_ownership(key, options).await }
        })
        .await
    }

    /// Execute a bounded batch. Mutating batches require an idempotency key for replay.
    pub async fn batch(
        &self,
        operations: Vec<BatchOperation>,
        options: Option<RequestOptions>,
    ) -> Result<Vec<BatchItemResult>, ClientError> {
        let read_only = operations
            .iter()
            .all(|operation| matches!(operation, BatchOperation::Get { .. }));
        let retry_safe = read_only || has_idempotency_key(options.as_ref());
        let timeout = request_timeout(&self.handle.runtime.config, options.as_ref());
        self.with_retry(retry_safe, timeout, move |client, remaining| {
            let operations = operations.clone();
            let options = adjusted_options(&options, remaining);
            async move { client.batch(operations, options).await }
        })
        .await
    }

    /// Register a logical subscription that survives connection replacement.
    pub async fn subscribe(
        &self,
        key_prefix: Bytes,
        resume_watermark: u64,
    ) -> Result<RecoveringSubscription, ClientError> {
        if self.handle.runtime.closed.load(Ordering::Acquire) {
            return Err(connection_error("recovering client is closed"));
        }
        let id = next_nonzero(&self.handle.runtime.subscription_ids)?;
        let (events, receiver) =
            mpsc::channel(self.handle.runtime.config.limits.max_subscription_events);
        let state = Arc::new(SubscriptionState {
            id,
            key_prefix,
            last_watermark: AtomicU64::new(resume_watermark),
            accepted_watermark: AtomicU64::new(resume_watermark),
            repair_required: AtomicBool::new(false),
            pending_gap: Mutex::new(None),
            terminal: Mutex::new(None),
            events,
            notify: Notify::new(),
            binding: AtomicU64::new(0),
            binding_control: Mutex::new(None),
            closed: AtomicBool::new(false),
            runtime: Arc::downgrade(&self.handle.runtime),
        });
        self.handle
            .runtime
            .subscriptions
            .lock()
            .expect("recovery subscriptions mutex poisoned")
            .insert(id, Arc::clone(&state));
        if let Err(error) = self.handle.runtime.bind_subscription(&state, false).await {
            self.handle.runtime.close_subscription(id, error.clone());
            return Err(error);
        }
        Ok(RecoveringSubscription {
            state,
            receiver,
            terminal_emitted: false,
        })
    }

    /// Open a fenced session. Reconnect never silently reacquires it.
    pub async fn open_session(
        &self,
        ttl: Duration,
    ) -> Result<RecoveringFencedSession, ClientError> {
        let current = self.handle.runtime.current_snapshot();
        let session = current.client.open_session(ttl).await?;
        let logical_id = next_nonzero(&self.handle.runtime.session_ids)?;
        let state = Arc::new(RecoveringSessionState {
            logical_id,
            session: Mutex::new(Some(session)),
            lost: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            runtime: Arc::downgrade(&self.handle.runtime),
        });
        self.handle
            .runtime
            .sessions
            .lock()
            .expect("recovery sessions mutex poisoned")
            .insert(logical_id, Arc::downgrade(&state));
        Ok(RecoveringFencedSession { state })
    }

    /// Pull bounded privacy-safe reconnect and repair counters.
    pub fn recovery_metrics(&self) -> RecoveryMetricsSnapshot {
        self.handle.runtime.metrics.snapshot()
    }

    /// Pull allocation-owning recovery maps together with the current client snapshot.
    pub fn retained_state(&self) -> RecoveryRetainedStateSnapshot {
        self.handle.runtime.retained_state()
    }

    /// Idempotently close every connection-owned recovery resource.
    pub fn close(&self) {
        self.handle
            .runtime
            .close(connection_error("recovering client closed"));
    }

    async fn with_retry<T, F, Fut>(
        &self,
        retry_safe: bool,
        timeout: Duration,
        mut operation: F,
    ) -> Result<T, ClientError>
    where
        F: FnMut(Arc<Hc2Client>, Duration) -> Fut,
        Fut: Future<Output = Result<T, ClientError>>,
    {
        let deadline = Instant::now() + timeout;
        for attempt in 1..=self.handle.runtime.retry_policy.max_attempts {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(deadline_error)?;
            let current = self.handle.runtime.current_snapshot();
            let result =
                tokio::time::timeout(remaining, operation(Arc::clone(&current.client), remaining))
                    .await
                    .map_err(|_| deadline_error())?;
            match result {
                Ok(value) => return Ok(value),
                Err(error)
                    if retry_safe
                        && error.retry_advice() == RetryAdvice::ReconnectIdempotent
                        && attempt < self.handle.runtime.retry_policy.max_attempts =>
                {
                    self.handle
                        .runtime
                        .metrics
                        .invocation_retries
                        .fetch_add(1, Ordering::Relaxed);
                    let remaining = deadline
                        .checked_duration_since(Instant::now())
                        .ok_or_else(deadline_error)?;
                    tokio::time::timeout(
                        remaining,
                        self.handle
                            .runtime
                            .reconnect_if_generation(current.generation),
                    )
                    .await
                    .map_err(|_| deadline_error())??;
                }
                Err(error) => return Err(error),
            }
        }
        Err(connection_error("invocation retry attempts exhausted"))
    }
}

#[derive(Clone)]
struct CurrentSnapshot {
    generation: u64,
    client: Arc<Hc2Client>,
}

impl RecoveringRuntime {
    fn current_snapshot(&self) -> CurrentSnapshot {
        let current = self.current.read().expect("recovery current lock poisoned");
        CurrentSnapshot {
            generation: current.generation,
            client: Arc::clone(&current.client),
        }
    }

    async fn reconnect_if_generation(self: &Arc<Self>, observed: u64) -> Result<(), ClientError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(connection_error("recovering client is closed"));
        }
        let _owner = self.reconnect_owner.lock().await;
        if self.current_snapshot().generation != observed {
            return Ok(());
        }
        self.lose_sessions();
        let generation = observed
            .checked_add(1)
            .filter(|value| *value != 0)
            .ok_or_else(|| protocol_error("connection generation exhausted"))?;
        let start = self.preferred_endpoint.load(Ordering::Acquire);
        let cluster_id = self
            .current
            .read()
            .expect("recovery current lock poisoned")
            .client
            .cluster_id()
            .to_owned();
        let next = establish(
            &self.endpoints,
            &self.config,
            self.reconnect_policy,
            start,
            generation,
            &self.metrics,
            EstablishMode::Reconnect {
                expected_cluster: &cluster_id,
            },
        )
        .await;
        let next = match next {
            Ok(next) => next,
            Err(error) => {
                self.metrics
                    .reconnect_exhausted
                    .fetch_add(1, Ordering::Relaxed);
                self.fail_subscriptions(error.clone());
                return Err(error);
            }
        };
        let previous = {
            let mut current = self
                .current
                .write()
                .expect("recovery current lock poisoned");
            std::mem::replace(&mut *current, next)
        };
        previous.client.close();
        self.metrics
            .reconnect_successes
            .fetch_add(1, Ordering::Relaxed);
        self.repair_subscriptions().await;
        Ok(())
    }

    async fn repair_subscriptions(self: &Arc<Self>) {
        let states: Vec<_> = self
            .subscriptions
            .lock()
            .expect("recovery subscriptions mutex poisoned")
            .values()
            .cloned()
            .collect();
        for state in states {
            if state.closed.load(Ordering::Acquire) {
                continue;
            }
            state.require_repair();
            if let Err(error) = self.bind_subscription(&state, true).await {
                self.close_subscription(state.id, error);
            }
        }
    }

    async fn bind_subscription(
        self: &Arc<Self>,
        state: &Arc<SubscriptionState>,
        reconnect: bool,
    ) -> Result<(), ClientError> {
        let current = self.current_snapshot();
        let watermark = state.last_watermark.load(Ordering::Acquire);
        let subscription = current
            .client
            .subscribe(state.key_prefix.clone(), watermark)
            .await?;
        let control = subscription.control();
        let binding = state
            .binding
            .fetch_add(1, Ordering::AcqRel)
            .checked_add(1)
            .ok_or_else(|| protocol_error("subscription binding exhausted"))?;
        let previous = {
            let mut binding_control = state
                .binding_control
                .lock()
                .expect("subscription binding control mutex poisoned");
            if state.closed.load(Ordering::Acquire) {
                control.close();
                return Err(connection_error("subscription is closed"));
            }
            binding_control.replace(control)
        };
        if let Some(previous) = previous {
            previous.close();
        }
        if reconnect {
            self.metrics
                .subscription_repairs
                .fetch_add(1, Ordering::Relaxed);
        }
        spawn_subscription_forwarder(
            Arc::downgrade(self),
            Arc::clone(state),
            subscription,
            binding,
            current.generation,
        );
        Ok(())
    }

    async fn explicit_subscription_repair(
        self: &Arc<Self>,
        state: &Arc<SubscriptionState>,
        watermark: u64,
    ) -> Result<(), ClientError> {
        if state.closed.load(Ordering::Acquire) {
            return Err(connection_error("subscription is closed"));
        }
        let current = state.last_watermark.load(Ordering::Acquire);
        if watermark < current {
            return Err(invalid("repair watermark cannot move backwards"));
        }
        state.last_watermark.store(watermark, Ordering::Release);
        state.accepted_watermark.store(watermark, Ordering::Release);
        state.repair_required.store(false, Ordering::Release);
        *state
            .pending_gap
            .lock()
            .expect("subscription gap mutex poisoned") = None;
        state.notify.notify_waiters();
        if let Err(error) = self.bind_subscription(state, true).await {
            state.require_repair();
            return Err(error);
        }
        Ok(())
    }

    fn close_subscription(&self, id: u64, error: ClientError) {
        let state = self
            .subscriptions
            .lock()
            .expect("recovery subscriptions mutex poisoned")
            .remove(&id);
        if let Some(state) = state {
            state.close(error);
        }
    }

    fn fail_subscriptions(&self, error: ClientError) {
        let subscriptions = std::mem::take(
            &mut *self
                .subscriptions
                .lock()
                .expect("recovery subscriptions mutex poisoned"),
        );
        for (_, subscription) in subscriptions {
            subscription.close(error.clone());
        }
    }

    fn lose_sessions(&self) {
        let sessions = std::mem::take(
            &mut *self
                .sessions
                .lock()
                .expect("recovery sessions mutex poisoned"),
        );
        for (_, session) in sessions {
            if let Some(session) = session.upgrade() {
                if session.mark_lost() {
                    self.metrics.session_losses.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    fn close(&self, error: ClientError) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        self.current_snapshot().client.close();
        self.fail_subscriptions(error);
        self.lose_sessions();
    }

    fn retained_state(&self) -> RecoveryRetainedStateSnapshot {
        let logical_subscriptions = self
            .subscriptions
            .lock()
            .expect("recovery subscriptions mutex poisoned")
            .len();
        let sessions = self
            .sessions
            .lock()
            .expect("recovery sessions mutex poisoned");
        let session_registrations = sessions.len();
        let live_sessions = sessions
            .values()
            .filter(|session| session.strong_count() > 0)
            .count();
        drop(sessions);
        RecoveryRetainedStateSnapshot {
            logical_subscriptions,
            session_registrations,
            live_sessions,
            current_client: self.current_snapshot().client.retained_state(),
            closed: self.closed.load(Ordering::Acquire),
        }
    }
}

struct SubscriptionState {
    id: u64,
    key_prefix: Bytes,
    last_watermark: AtomicU64,
    accepted_watermark: AtomicU64,
    repair_required: AtomicBool,
    pending_gap: Mutex<Option<u64>>,
    terminal: Mutex<Option<ClientError>>,
    events: mpsc::Sender<CacheEvent>,
    notify: Notify,
    binding: AtomicU64,
    binding_control: Mutex<Option<SubscriptionControl>>,
    closed: AtomicBool,
    runtime: Weak<RecoveringRuntime>,
}

impl SubscriptionState {
    fn require_repair(&self) {
        if !self.repair_required.swap(true, Ordering::AcqRel) {
            let watermark = self.last_watermark.load(Ordering::Acquire);
            *self
                .pending_gap
                .lock()
                .expect("subscription gap mutex poisoned") = Some(watermark);
            if let Some(runtime) = self.runtime.upgrade() {
                runtime
                    .metrics
                    .gap_repairs_required
                    .fetch_add(1, Ordering::Relaxed);
            }
            self.notify.notify_waiters();
        }
    }

    fn close(&self, error: ClientError) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        self.binding.fetch_add(1, Ordering::AcqRel);
        if let Some(control) = self
            .binding_control
            .lock()
            .expect("subscription binding control mutex poisoned")
            .take()
        {
            control.close();
        }
        *self
            .terminal
            .lock()
            .expect("subscription terminal mutex poisoned") = Some(error);
        self.notify.notify_waiters();
    }
}

/// Logical subscription that remains stable across connection replacement.
pub struct RecoveringSubscription {
    state: Arc<SubscriptionState>,
    receiver: mpsc::Receiver<CacheEvent>,
    terminal_emitted: bool,
}

impl RecoveringSubscription {
    /// Stable logical subscription ID, independent of server registrations.
    pub fn id(&self) -> u64 {
        self.state.id
    }

    /// Last event watermark delivered or explicitly repaired by the caller.
    pub fn last_watermark(&self) -> u64 {
        self.state.last_watermark.load(Ordering::Acquire)
    }

    /// Read the next event, explicit repair boundary, or terminal failure.
    pub async fn next(&mut self) -> Option<SubscriptionEvent> {
        loop {
            let notified = self.state.notify.notified();
            if let Some(after_watermark) = self
                .state
                .pending_gap
                .lock()
                .expect("subscription gap mutex poisoned")
                .take()
            {
                return Some(SubscriptionEvent::Gap {
                    subscription_id: self.state.id,
                    after_watermark,
                });
            }
            if self.state.closed.load(Ordering::Acquire) {
                if self.terminal_emitted {
                    return None;
                }
                self.terminal_emitted = true;
                if let Some(error) = self
                    .state
                    .terminal
                    .lock()
                    .expect("subscription terminal mutex poisoned")
                    .take()
                {
                    return Some(SubscriptionEvent::Closed(error));
                }
                return None;
            }
            tokio::select! {
                _ = notified => {}
                event = self.receiver.recv() => {
                    if let Some(event) = event {
                        self.state
                            .last_watermark
                            .store(event.watermark, Ordering::Release);
                        return Some(SubscriptionEvent::Event(event));
                    }
                    return None;
                },
            }
        }
    }

    /// Confirm conservative cache repair and re-register from its watermark.
    pub async fn repair(&mut self, watermark: u64) -> Result<(), ClientError> {
        while self.receiver.try_recv().is_ok() {}
        let runtime = self
            .state
            .runtime
            .upgrade()
            .ok_or_else(|| connection_error("subscription recovery owner closed"))?;
        runtime
            .explicit_subscription_repair(&self.state, watermark)
            .await
    }

    /// Idempotently close the logical subscription and current registration.
    pub fn close(self) {
        if let Some(runtime) = self.state.runtime.upgrade() {
            runtime.close_subscription(self.state.id, connection_error("subscription closed"));
        }
    }
}

impl Drop for RecoveringSubscription {
    fn drop(&mut self) {
        if let Some(runtime) = self.state.runtime.upgrade() {
            runtime.close_subscription(self.state.id, connection_error("subscription dropped"));
        }
    }
}

struct RecoveringSessionState {
    logical_id: u64,
    session: Mutex<Option<FencedSession>>,
    lost: AtomicBool,
    closed: AtomicBool,
    runtime: Weak<RecoveringRuntime>,
}

impl RecoveringSessionState {
    fn mark_lost(&self) -> bool {
        if self.lost.swap(true, Ordering::AcqRel) {
            return false;
        }
        if let Some(session) = self
            .session
            .lock()
            .expect("recovering session mutex poisoned")
            .take()
        {
            session.close();
        }
        true
    }

    fn close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(session) = self
            .session
            .lock()
            .expect("recovering session mutex poisoned")
            .take()
        {
            session.close();
        }
        if let Some(runtime) = self.runtime.upgrade() {
            runtime
                .sessions
                .lock()
                .expect("recovery sessions mutex poisoned")
                .remove(&self.logical_id);
        }
    }
}

/// Fenced session that becomes permanently lost on any reconnect.
pub struct RecoveringFencedSession {
    state: Arc<RecoveringSessionState>,
}

impl RecoveringFencedSession {
    /// Read the current fence or fail loud after session/connection loss.
    pub fn fence(&self) -> Result<u64, ClientError> {
        if self.state.lost.load(Ordering::Acquire) || self.state.closed.load(Ordering::Acquire) {
            return Err(session_error("fenced session was lost during reconnect"));
        }
        let result = self
            .state
            .session
            .lock()
            .expect("recovering session mutex poisoned")
            .as_ref()
            .ok_or_else(|| session_error("fenced session is no longer active"))?
            .fence();
        if result.is_err() {
            self.state.mark_lost();
        }
        result
    }

    /// Idempotently close without attempting reacquisition.
    pub fn close(self) {
        self.state.close();
    }
}

impl Drop for RecoveringFencedSession {
    fn drop(&mut self) {
        self.state.close();
    }
}

fn spawn_subscription_forwarder(
    runtime: Weak<RecoveringRuntime>,
    state: Arc<SubscriptionState>,
    mut subscription: Subscription,
    binding: u64,
    generation: u64,
) {
    tokio::spawn(async move {
        loop {
            if state.closed.load(Ordering::Acquire)
                || state.binding.load(Ordering::Acquire) != binding
            {
                return;
            }
            let Some(event) = subscription.next().await else {
                if state.closed.load(Ordering::Acquire)
                    || state.binding.load(Ordering::Acquire) != binding
                {
                    return;
                }
                if let Some(runtime) = runtime.upgrade() {
                    let _ = runtime.reconnect_if_generation(generation).await;
                }
                return;
            };
            match event {
                SubscriptionEvent::Event(mut event) => {
                    if state.repair_required.load(Ordering::Acquire) {
                        continue;
                    }
                    let last = state.accepted_watermark.load(Ordering::Acquire);
                    if event.watermark == 0 {
                        state.require_repair();
                        continue;
                    }
                    if event.watermark <= last {
                        if let Some(runtime) = runtime.upgrade() {
                            runtime
                                .metrics
                                .duplicate_events
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        continue;
                    }
                    if last != 0 && event.watermark != last.saturating_add(1) {
                        state.require_repair();
                        continue;
                    }
                    let watermark = event.watermark;
                    event.subscription_id = state.id;
                    if state.events.try_send(event).is_err() {
                        state.require_repair();
                        continue;
                    }
                    state.accepted_watermark.store(watermark, Ordering::Release);
                }
                SubscriptionEvent::Gap { .. } => state.require_repair(),
                SubscriptionEvent::Closed(error) => {
                    if let Some(runtime) = runtime.upgrade() {
                        if error.retry_advice() == RetryAdvice::ReconnectIdempotent {
                            if let Err(error) = runtime.reconnect_if_generation(generation).await {
                                runtime.fail_subscriptions(error);
                            }
                        } else {
                            runtime.fail_subscriptions(error);
                        }
                    }
                    return;
                }
            }
        }
    });
}

async fn establish(
    endpoints: &[ReconnectEndpoint],
    config: &ClientConfig,
    policy: ReconnectPolicy,
    start: usize,
    generation: u64,
    metrics: &RecoveryMetrics,
    mode: EstablishMode<'_>,
) -> Result<CurrentConnection, ClientError> {
    let mut last = connection_error("reconnect attempts exhausted");
    for attempt in 0..policy.max_attempts {
        if attempt > 0 {
            tokio::time::sleep(policy.delay(attempt)).await;
        }
        if matches!(mode, EstablishMode::Reconnect { .. }) {
            metrics.reconnect_attempts.fetch_add(1, Ordering::Relaxed);
        }
        let endpoint_index = (start + attempt) % endpoints.len();
        let endpoint = &endpoints[endpoint_index];
        let mut candidate_config = config.clone();
        candidate_config.connection_generation = generation;
        match Hc2Client::connect(endpoint.adapter.as_ref(), candidate_config).await {
            Ok(client) => {
                if matches!(
                    mode,
                    EstablishMode::Reconnect { expected_cluster }
                        if expected_cluster != client.cluster_id()
                ) {
                    client.close();
                    return Err(protocol_error("reconnect crossed cluster identity"));
                }
                return Ok(CurrentConnection {
                    generation,
                    endpoint_index,
                    client: Arc::new(client),
                });
            }
            Err(error) if error.retry_advice() == RetryAdvice::ReconnectIdempotent => {
                last = error;
            }
            Err(error) => return Err(error),
        }
    }
    Err(ClientError::new(
        last.code(),
        RetryAdvice::ReconnectIdempotent,
        "reconnect attempts exhausted",
    ))
}

#[derive(Clone, Copy)]
enum EstablishMode<'a> {
    Initial,
    Reconnect { expected_cluster: &'a str },
}

fn validate_endpoints(endpoints: &[ReconnectEndpoint]) -> Result<(), ClientError> {
    if endpoints.is_empty() || endpoints.len() > 256 {
        return Err(invalid("reconnect endpoint count is outside [1, 256]"));
    }
    let transport = endpoints[0].transport();
    let mut ids = BTreeMap::new();
    for endpoint in endpoints {
        if endpoint.transport() != transport || ids.insert(endpoint.id(), ()).is_some() {
            return Err(invalid(
                "reconnect endpoints must be unique and use one transport",
            ));
        }
    }
    Ok(())
}

fn request_timeout(config: &ClientConfig, options: Option<&RequestOptions>) -> Duration {
    options.map_or(config.default_request_timeout, |value| value.timeout)
}

fn adjusted_options(
    options: &Option<RequestOptions>,
    remaining: Duration,
) -> Option<RequestOptions> {
    options.clone().map(|mut options| {
        options.timeout = remaining.min(Duration::from_secs(300));
        options
    })
}

fn has_idempotency_key(options: Option<&RequestOptions>) -> bool {
    options.is_some_and(|options| !options.idempotency_key.is_empty())
}

fn next_nonzero(value: &AtomicU64) -> Result<u64, ClientError> {
    value
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            current.checked_add(1).filter(|next| *next != 0)
        })
        .map(|previous| previous + 1)
        .map_err(|_| protocol_error("logical owner id exhausted"))
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn duration_from_nanos(nanos: u128) -> Duration {
    let bounded = nanos.min(u128::from(u64::MAX));
    Duration::from_nanos(u64::try_from(bounded).unwrap_or(u64::MAX))
}

const fn invalid(detail: &'static str) -> ClientError {
    ClientError::new(ErrorCode::InvalidRequest, RetryAdvice::Never, detail)
}

const fn connection_error(detail: &'static str) -> ClientError {
    ClientError::new(
        ErrorCode::Unavailable,
        RetryAdvice::ReconnectIdempotent,
        detail,
    )
}

const fn protocol_error(detail: &'static str) -> ClientError {
    ClientError::new(ErrorCode::Internal, RetryAdvice::Never, detail)
}

const fn session_error(detail: &'static str) -> ClientError {
    ClientError::new(ErrorCode::SessionLost, RetryAdvice::RepairRequired, detail)
}

const fn deadline_error() -> ClientError {
    ClientError::new(
        ErrorCode::DeadlineExceeded,
        RetryAdvice::Never,
        "reconnect and retry exceeded the original deadline",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_backoff_is_bounded_deterministic_and_separate_from_retry() {
        let policy = ReconnectPolicy::new(
            6,
            Duration::from_millis(10),
            Duration::from_millis(80),
            Duration::from_millis(5),
            19,
        )
        .unwrap();
        let first: Vec<_> = (1..6).map(|attempt| policy.delay(attempt)).collect();
        let replayed: Vec<_> = (1..6).map(|attempt| policy.delay(attempt)).collect();
        assert_eq!(first, replayed);
        assert!(first
            .iter()
            .all(|delay| *delay <= Duration::from_millis(80)));
        assert_eq!(InvocationRetryPolicy::new(2).unwrap().max_attempts, 2);
        assert!(InvocationRetryPolicy::new(0).is_err());
        assert!(ReconnectPolicy::new(
            0,
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::ZERO,
            1
        )
        .is_err());
    }
}
