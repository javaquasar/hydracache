use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache_core::Result;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::{JoinError, JoinHandle};

use crate::grid::invalidation_ring::InvalidationRing;
use crate::invalidation_bus::{
    CacheInvalidationBus, CacheInvalidationFrame, CacheInvalidationMessage,
    CacheInvalidationReceive, InMemoryFramedInvalidationBus, CACHE_INVALIDATION_FRAME_VERSION,
};
use crate::ClusterGeneration;

/// Shared invalidation-ring handle reserved for resume support.
///
/// W1 wires the relay around the existing bus and transport seam. W2 starts
/// using this ring handle for `replay_from` resume after bounded-queue gaps and
/// reconnects.
pub type SharedInvalidationRing = Arc<Mutex<InvalidationRing>>;

/// A bus that can apply an already encoded invalidation frame locally.
///
/// External transports receive versioned frame bytes. Applying those bytes via
/// this trait keeps inbound traffic on the same decode path as
/// [`InMemoryFramedInvalidationBus::publish_encoded_frame`] and prevents
/// transports from fabricating cache mutations directly.
pub trait CacheInvalidationFrameSink: CacheInvalidationBus {
    /// Publish encoded frame bytes into the local invalidation bus.
    fn publish_encoded_frame(&self, bytes: Bytes) -> Result<()>;
}

impl CacheInvalidationFrameSink for InMemoryFramedInvalidationBus {
    fn publish_encoded_frame(&self, bytes: Bytes) -> Result<()> {
        InMemoryFramedInvalidationBus::publish_encoded_frame(self, bytes)
    }
}

/// Error reported by an external invalidation transport.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    /// The backend failed while publishing or receiving.
    #[error("invalidation transport backend error: {0}")]
    Backend(String),
    /// The backend applied backpressure before accepting a frame.
    #[error("invalidation transport backpressure: {0}")]
    Backpressure(String),
    /// A frame payload could not be decoded.
    #[error("invalidation transport decode error: {0}")]
    Decode(String),
    /// A frame uses a future wire version and must fail closed.
    #[error("unsupported invalidation frame version {found}; max supported {max_supported}")]
    UnknownFrameVersion { found: u16, max_supported: u16 },
}

/// Async transport for moving invalidation frames outside the process.
///
/// Implementations only move [`CacheInvalidationFrame`] values. They do not
/// apply invalidations, own cache data, or participate in the cache write path.
#[async_trait]
pub trait InvalidationTransport: Send + Sync + 'static {
    /// Publish one frame to the external transport.
    async fn publish(
        &self,
        frame: &CacheInvalidationFrame,
    ) -> std::result::Result<(), TransportError>;

    /// Receive the next inbound frame from the external transport.
    async fn next_inbound(
        &mut self,
    ) -> Option<std::result::Result<CacheInvalidationFrame, TransportError>>;
}

/// Configuration for an invalidation relay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportConfig {
    /// Logical HydraCache cluster name accepted by this relay.
    pub cluster_name: String,
    /// Backend channel, subject, or topic name.
    pub channel: String,
    /// Local node id; outbound relay only publishes messages from this source.
    pub local_node_id: String,
    /// Bound for bus-to-transport frames.
    pub outbound_capacity: usize,
    /// Bound for transport-to-bus frames.
    pub inbound_capacity: usize,
    /// Reconnect backoff reserved for concrete backends and W2 resume.
    pub reconnect_backoff_ms: u64,
    /// Number of `(source,message_id)` pairs retained for deduplication.
    pub dedup_window: usize,
}

impl TransportConfig {
    /// Default number of frames held in each relay queue.
    pub const DEFAULT_QUEUE_CAPACITY: usize = 1024;
    /// Default deduplication window.
    pub const DEFAULT_DEDUP_WINDOW: usize = 4096;
    /// Default reconnect backoff in milliseconds.
    pub const DEFAULT_RECONNECT_BACKOFF_MS: u64 = 250;

    /// Build a config for `cluster_name` and this process's `local_node_id`.
    pub fn new(cluster_name: impl Into<String>, local_node_id: impl Into<String>) -> Self {
        let cluster_name = cluster_name.into();
        Self {
            channel: format!("hydracache:inval:{cluster_name}"),
            cluster_name,
            local_node_id: local_node_id.into(),
            outbound_capacity: Self::DEFAULT_QUEUE_CAPACITY,
            inbound_capacity: Self::DEFAULT_QUEUE_CAPACITY,
            reconnect_backoff_ms: Self::DEFAULT_RECONNECT_BACKOFF_MS,
            dedup_window: Self::DEFAULT_DEDUP_WINDOW,
        }
    }

    /// Override the backend channel, subject, or topic.
    pub fn channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = channel.into();
        self
    }

    /// Override the outbound queue capacity.
    pub fn outbound_capacity(mut self, capacity: usize) -> Self {
        self.outbound_capacity = capacity.max(1);
        self
    }

    /// Override the inbound queue capacity.
    pub fn inbound_capacity(mut self, capacity: usize) -> Self {
        self.inbound_capacity = capacity.max(1);
        self
    }

    /// Override the reconnect backoff used by concrete backends.
    pub fn reconnect_backoff(mut self, backoff: Duration) -> Self {
        self.reconnect_backoff_ms = backoff.as_millis().min(u128::from(u64::MAX)) as u64;
        self
    }

    /// Override the deduplication window.
    pub fn dedup_window(mut self, window: usize) -> Self {
        self.dedup_window = window;
        self
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self::new("default", "hydracache-node")
    }
}

/// Snapshot of invalidation relay counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransportMetricsSnapshot {
    /// Frames successfully published to the external transport.
    pub published_total: u64,
    /// Frames received from the external transport before local filtering.
    pub received_total: u64,
    /// Inbound frames applied to the local bus.
    pub applied_total: u64,
    /// Duplicate `(source,message_id)` inbound frames dropped.
    pub deduped_total: u64,
    /// Inbound frames rejected by generation fencing.
    pub fenced_total: u64,
    /// Inbound frames dropped because their cluster name did not match.
    pub foreign_cluster_dropped_total: u64,
    /// Inbound frames from the local node dropped to prevent loops.
    pub own_frame_dropped_total: u64,
    /// Bus messages from non-local sources suppressed by the outbound relay.
    pub remote_source_suppressed_total: u64,
    /// Frames dropped because the outbound queue was full.
    pub outbound_dropped_full_total: u64,
    /// Frames dropped because the inbound queue was full.
    pub inbound_dropped_full_total: u64,
    /// Transport publish failures.
    pub publish_error_total: u64,
    /// Transport errors observed while receiving.
    pub transport_error_total: u64,
    /// Unknown future frame versions rejected fail-closed.
    pub unknown_version_total: u64,
    /// Malformed or undecodable inbound frames.
    pub decode_error_total: u64,
    /// Local bus apply failures.
    pub bus_apply_error_total: u64,
    /// Bus lag reports observed by the outbound relay.
    pub bus_lag_total: u64,
}

/// Bounded-label invalidation relay counters.
#[derive(Debug, Default)]
pub struct TransportMetrics {
    published_total: AtomicU64,
    received_total: AtomicU64,
    applied_total: AtomicU64,
    deduped_total: AtomicU64,
    fenced_total: AtomicU64,
    foreign_cluster_dropped_total: AtomicU64,
    own_frame_dropped_total: AtomicU64,
    remote_source_suppressed_total: AtomicU64,
    outbound_dropped_full_total: AtomicU64,
    inbound_dropped_full_total: AtomicU64,
    publish_error_total: AtomicU64,
    transport_error_total: AtomicU64,
    unknown_version_total: AtomicU64,
    decode_error_total: AtomicU64,
    bus_apply_error_total: AtomicU64,
    bus_lag_total: AtomicU64,
}

impl TransportMetrics {
    /// Return a stable snapshot of all counters.
    pub fn snapshot(&self) -> TransportMetricsSnapshot {
        TransportMetricsSnapshot {
            published_total: self.published_total.load(Ordering::Relaxed),
            received_total: self.received_total.load(Ordering::Relaxed),
            applied_total: self.applied_total.load(Ordering::Relaxed),
            deduped_total: self.deduped_total.load(Ordering::Relaxed),
            fenced_total: self.fenced_total.load(Ordering::Relaxed),
            foreign_cluster_dropped_total: self
                .foreign_cluster_dropped_total
                .load(Ordering::Relaxed),
            own_frame_dropped_total: self.own_frame_dropped_total.load(Ordering::Relaxed),
            remote_source_suppressed_total: self
                .remote_source_suppressed_total
                .load(Ordering::Relaxed),
            outbound_dropped_full_total: self.outbound_dropped_full_total.load(Ordering::Relaxed),
            inbound_dropped_full_total: self.inbound_dropped_full_total.load(Ordering::Relaxed),
            publish_error_total: self.publish_error_total.load(Ordering::Relaxed),
            transport_error_total: self.transport_error_total.load(Ordering::Relaxed),
            unknown_version_total: self.unknown_version_total.load(Ordering::Relaxed),
            decode_error_total: self.decode_error_total.load(Ordering::Relaxed),
            bus_apply_error_total: self.bus_apply_error_total.load(Ordering::Relaxed),
            bus_lag_total: self.bus_lag_total.load(Ordering::Relaxed),
        }
    }

    fn record_transport_error(&self, error: &TransportError) {
        self.transport_error_total.fetch_add(1, Ordering::Relaxed);
        match error {
            TransportError::Decode(_) => {
                self.decode_error_total.fetch_add(1, Ordering::Relaxed);
            }
            TransportError::UnknownFrameVersion { .. } => {
                self.unknown_version_total.fetch_add(1, Ordering::Relaxed);
            }
            TransportError::Backend(_) | TransportError::Backpressure(_) => {}
        }
    }
}

/// In-memory transport endpoint used by W1/W5 deterministic tests.
#[derive(Debug, Clone)]
pub struct InMemoryTransport {
    sender: mpsc::Sender<std::result::Result<CacheInvalidationFrame, TransportError>>,
    receiver:
        Arc<Mutex<mpsc::Receiver<std::result::Result<CacheInvalidationFrame, TransportError>>>>,
}

impl InMemoryTransport {
    /// Create a connected pair of in-memory transport endpoints.
    pub fn pair(capacity: usize) -> (Self, Self) {
        let capacity = capacity.max(1);
        let (a_to_b_tx, a_to_b_rx) = mpsc::channel(capacity);
        let (b_to_a_tx, b_to_a_rx) = mpsc::channel(capacity);
        (
            Self {
                sender: a_to_b_tx,
                receiver: Arc::new(Mutex::new(b_to_a_rx)),
            },
            Self {
                sender: b_to_a_tx,
                receiver: Arc::new(Mutex::new(a_to_b_rx)),
            },
        )
    }

    /// Inject an inbound transport error into the peer endpoint.
    pub fn try_send_error(&self, error: TransportError) -> std::result::Result<(), TransportError> {
        self.sender
            .try_send(Err(error))
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    TransportError::Backpressure("in-memory transport queue is full".to_owned())
                }
                mpsc::error::TrySendError::Closed(_) => {
                    TransportError::Backend("in-memory transport queue is closed".to_owned())
                }
            })
    }
}

#[async_trait]
impl InvalidationTransport for InMemoryTransport {
    async fn publish(
        &self,
        frame: &CacheInvalidationFrame,
    ) -> std::result::Result<(), TransportError> {
        self.sender
            .try_send(Ok(frame.clone()))
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    TransportError::Backpressure("in-memory transport queue is full".to_owned())
                }
                mpsc::error::TrySendError::Closed(_) => {
                    TransportError::Backend("in-memory transport queue is closed".to_owned())
                }
            })
    }

    async fn next_inbound(
        &mut self,
    ) -> Option<std::result::Result<CacheInvalidationFrame, TransportError>> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }
}

/// Running invalidation relay with observable metrics.
#[derive(Debug)]
pub struct InvalidationRelayHandle {
    join: JoinHandle<()>,
    shutdown: watch::Sender<bool>,
    metrics: Arc<TransportMetrics>,
}

impl InvalidationRelayHandle {
    /// Return the shared metrics object.
    pub fn metrics(&self) -> Arc<TransportMetrics> {
        self.metrics.clone()
    }

    /// Return a point-in-time metrics snapshot.
    pub fn snapshot(&self) -> TransportMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Ask the relay tasks to shut down and wait for the supervisor task.
    pub async fn shutdown(self) -> std::result::Result<(), JoinError> {
        let _ = self.shutdown.send(true);
        self.join.await
    }

    /// Abort the relay supervisor after signalling the worker tasks.
    pub fn abort(&self) {
        let _ = self.shutdown.send(true);
        self.join.abort();
    }

    /// Consume the handle and return the supervisor join handle.
    pub fn into_join_handle(self) -> JoinHandle<()> {
        self.join
    }
}

/// Async relay between a local invalidation bus and an external transport.
pub struct InvalidationRelay;

impl InvalidationRelay {
    /// Spawn a relay task on the current Tokio runtime.
    ///
    /// The relay uses bounded queues between the local bus, transport publish,
    /// transport receive, and local apply loops. Local cache writes only publish
    /// to the bus; the relay's outbound queue uses `try_send`, so transport
    /// backpressure never blocks the cache write path.
    pub fn spawn<B, T>(
        bus: Arc<B>,
        transport: T,
        ring: Option<SharedInvalidationRing>,
        config: TransportConfig,
    ) -> JoinHandle<()>
    where
        B: CacheInvalidationFrameSink,
        T: InvalidationTransport + Clone,
    {
        Self::spawn_with_metrics(bus, transport, ring, config).into_join_handle()
    }

    /// Spawn a relay task and retain the metrics handle.
    pub fn spawn_with_metrics<B, T>(
        bus: Arc<B>,
        transport: T,
        _ring: Option<SharedInvalidationRing>,
        config: TransportConfig,
    ) -> InvalidationRelayHandle
    where
        B: CacheInvalidationFrameSink,
        T: InvalidationTransport + Clone,
    {
        let metrics = Arc::new(TransportMetrics::default());
        let bus_receiver = bus.subscribe();
        let (outbound_tx, outbound_rx) = mpsc::channel(config.outbound_capacity.max(1));
        let (inbound_tx, inbound_rx) = mpsc::channel(config.inbound_capacity.max(1));
        let (shutdown, shutdown_rx) = watch::channel(false);
        let next_message_id = Arc::new(AtomicU64::new(1));

        let outbound_reader = tokio::spawn(run_outbound_bus_loop(
            shutdown_rx.clone(),
            outbound_tx,
            config.clone(),
            metrics.clone(),
            next_message_id,
            bus_receiver,
        ));
        let outbound_publisher = tokio::spawn(run_outbound_transport_loop(
            shutdown_rx.clone(),
            outbound_rx,
            transport.clone(),
            metrics.clone(),
        ));
        let inbound_reader = tokio::spawn(run_inbound_transport_loop(
            shutdown_rx.clone(),
            inbound_tx,
            transport,
            metrics.clone(),
        ));
        let inbound_apply = tokio::spawn(run_inbound_apply_loop(
            shutdown_rx,
            inbound_rx,
            bus,
            config,
            metrics.clone(),
        ));

        let supervisor_shutdown = shutdown.clone();
        let join = tokio::spawn(async move {
            let mut outbound_reader = outbound_reader;
            let mut outbound_publisher = outbound_publisher;
            let mut inbound_reader = inbound_reader;
            let mut inbound_apply = inbound_apply;

            tokio::select! {
                _ = &mut outbound_reader => {}
                _ = &mut outbound_publisher => {}
                _ = &mut inbound_reader => {}
                _ = &mut inbound_apply => {}
            }

            let _ = supervisor_shutdown.send(true);
            outbound_reader.abort();
            outbound_publisher.abort();
            inbound_reader.abort();
            inbound_apply.abort();
        });

        InvalidationRelayHandle {
            join,
            shutdown,
            metrics,
        }
    }
}

async fn run_outbound_bus_loop(
    mut shutdown: watch::Receiver<bool>,
    outbound_tx: mpsc::Sender<CacheInvalidationFrame>,
    config: TransportConfig,
    metrics: Arc<TransportMetrics>,
    next_message_id: Arc<AtomicU64>,
    mut bus_receiver: Box<dyn crate::invalidation_bus::CacheInvalidationReceiver>,
) {
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            received = bus_receiver.recv() => {
                match received {
                    CacheInvalidationReceive::Message(message) => {
                        enqueue_outbound_frame(
                            &outbound_tx,
                            &config,
                            &metrics,
                            &next_message_id,
                            message,
                        );
                    }
                    CacheInvalidationReceive::Lagged(count) => {
                        metrics.bus_lag_total.fetch_add(count, Ordering::Relaxed);
                    }
                    CacheInvalidationReceive::DecodeError(_) => {
                        metrics.decode_error_total.fetch_add(1, Ordering::Relaxed);
                    }
                    CacheInvalidationReceive::Closed => break,
                }
            }
        }
    }
}

fn enqueue_outbound_frame(
    outbound_tx: &mpsc::Sender<CacheInvalidationFrame>,
    config: &TransportConfig,
    metrics: &TransportMetrics,
    next_message_id: &AtomicU64,
    message: CacheInvalidationMessage,
) {
    if message.source_id() != config.local_node_id.as_str() {
        metrics
            .remote_source_suppressed_total
            .fetch_add(1, Ordering::Relaxed);
        return;
    }

    let message_id = next_message_id.fetch_add(1, Ordering::Relaxed);
    let frame = CacheInvalidationFrame::new(message)
        .with_cluster_name(config.cluster_name.clone())
        .with_message_id(message_id);

    if outbound_tx.try_send(frame).is_err() {
        metrics
            .outbound_dropped_full_total
            .fetch_add(1, Ordering::Relaxed);
    }
}

async fn run_outbound_transport_loop<T>(
    mut shutdown: watch::Receiver<bool>,
    mut outbound_rx: mpsc::Receiver<CacheInvalidationFrame>,
    transport: T,
    metrics: Arc<TransportMetrics>,
) where
    T: InvalidationTransport,
{
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            maybe_frame = outbound_rx.recv() => {
                let Some(frame) = maybe_frame else {
                    break;
                };
                match transport.publish(&frame).await {
                    Ok(()) => {
                        metrics.published_total.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(error) => {
                        metrics.publish_error_total.fetch_add(1, Ordering::Relaxed);
                        metrics.record_transport_error(&error);
                    }
                }
            }
        }
    }
}

async fn run_inbound_transport_loop<T>(
    mut shutdown: watch::Receiver<bool>,
    inbound_tx: mpsc::Sender<CacheInvalidationFrame>,
    mut transport: T,
    metrics: Arc<TransportMetrics>,
) where
    T: InvalidationTransport,
{
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            maybe_frame = transport.next_inbound() => {
                match maybe_frame {
                    Some(Ok(frame)) => {
                        metrics.received_total.fetch_add(1, Ordering::Relaxed);
                        if inbound_tx.try_send(frame).is_err() {
                            metrics
                                .inbound_dropped_full_total
                                .fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Some(Err(error)) => metrics.record_transport_error(&error),
                    None => break,
                }
            }
        }
    }
}

async fn run_inbound_apply_loop<B>(
    mut shutdown: watch::Receiver<bool>,
    mut inbound_rx: mpsc::Receiver<CacheInvalidationFrame>,
    bus: Arc<B>,
    config: TransportConfig,
    metrics: Arc<TransportMetrics>,
) where
    B: CacheInvalidationFrameSink,
{
    let mut state = InboundApplyState::new(config.dedup_window);

    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            maybe_frame = inbound_rx.recv() => {
                let Some(frame) = maybe_frame else {
                    break;
                };
                if state.should_drop(&frame, &config, &metrics) {
                    continue;
                }

                match frame.encode().and_then(|bytes| bus.publish_encoded_frame(bytes)) {
                    Ok(()) => {
                        metrics.applied_total.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        metrics.bus_apply_error_total.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
    }
}

struct InboundApplyState {
    dedup: DedupWindow,
    highest_generation: HashMap<String, ClusterGeneration>,
}

impl InboundApplyState {
    fn new(dedup_window: usize) -> Self {
        Self {
            dedup: DedupWindow::new(dedup_window),
            highest_generation: HashMap::new(),
        }
    }

    fn should_drop(
        &mut self,
        frame: &CacheInvalidationFrame,
        config: &TransportConfig,
        metrics: &TransportMetrics,
    ) -> bool {
        if frame.version() != CACHE_INVALIDATION_FRAME_VERSION {
            metrics
                .unknown_version_total
                .fetch_add(1, Ordering::Relaxed);
            return true;
        }

        if matches!(frame.cluster_name(), Some(cluster) if cluster != config.cluster_name.as_str())
        {
            metrics
                .foreign_cluster_dropped_total
                .fetch_add(1, Ordering::Relaxed);
            return true;
        }

        if frame.source_id() == config.local_node_id.as_str() {
            metrics
                .own_frame_dropped_total
                .fetch_add(1, Ordering::Relaxed);
            return true;
        }

        if self.is_stale_generation(frame) {
            metrics.fenced_total.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        if let Some(message_id) = frame.message_id() {
            if !self.dedup.remember(frame.source_id(), message_id) {
                metrics.deduped_total.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }

        false
    }

    fn is_stale_generation(&mut self, frame: &CacheInvalidationFrame) -> bool {
        let Some(generation) = frame.source_generation() else {
            return false;
        };
        match self.highest_generation.get_mut(frame.source_id()) {
            Some(highest) if generation < *highest => true,
            Some(highest) => {
                if generation > *highest {
                    *highest = generation;
                }
                false
            }
            None => {
                self.highest_generation
                    .insert(frame.source_id().to_owned(), generation);
                false
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DedupKey {
    source_id: String,
    message_id: u64,
}

struct DedupWindow {
    capacity: usize,
    order: VecDeque<DedupKey>,
    seen: HashSet<DedupKey>,
}

impl DedupWindow {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: VecDeque::with_capacity(capacity.min(1024)),
            seen: HashSet::new(),
        }
    }

    fn remember(&mut self, source_id: &str, message_id: u64) -> bool {
        if self.capacity == 0 {
            return true;
        }

        let key = DedupKey {
            source_id: source_id.to_owned(),
            message_id,
        };
        if self.seen.contains(&key) {
            return false;
        }

        self.seen.insert(key.clone());
        self.order.push_back(key);
        while self.order.len() > self.capacity {
            if let Some(expired) = self.order.pop_front() {
                self.seen.remove(&expired);
            }
        }
        true
    }
}
