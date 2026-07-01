use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache_core::Result;
use tokio::sync::{mpsc, watch, Mutex, Notify};
use tokio::task::{JoinError, JoinHandle};

use crate::grid::invalidation_ring::{InvalidationEvent, InvalidationRing, ReplayResult};
use crate::invalidation_bus::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationFrame, CacheInvalidationMessage,
    CacheInvalidationReceive, InMemoryFramedInvalidationBus, CACHE_INVALIDATION_FRAME_VERSION,
};
use crate::{ClusterGeneration, PartitionId};

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
    /// Maximum accepted inbound frames per source for the current relay window.
    pub inbound_rate_limit_per_source: Option<u64>,
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
            inbound_rate_limit_per_source: None,
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

    /// Override the per-source inbound rate limit.
    pub fn inbound_rate_limit_per_source(mut self, limit: Option<u64>) -> Self {
        self.inbound_rate_limit_per_source = limit;
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
    /// Valid inbound frames dropped by the per-source rate limit.
    pub rate_limited_total: u64,
    /// Resume requests sent to the replay loop.
    pub resume_requested_total: u64,
    /// Resume requests skipped because no ring was configured.
    pub resume_unavailable_total: u64,
    /// Outbound drops that marked a resume gap.
    pub resume_marked_total: u64,
    /// Ring events replayed after a resume request.
    pub resume_replayed_total: u64,
    /// Ring resume requests that fell behind retention.
    pub resume_fell_behind_total: u64,
    /// Conservative clear-partition actions emitted for fell-behind gaps.
    pub resume_clear_partition_total: u64,
    /// Last partition that needed a clear after falling behind.
    pub last_clear_partition: Option<PartitionId>,
    /// Current ring distance from the last requested resume watermark.
    pub inbound_lag: u64,
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
    rate_limited_total: AtomicU64,
    resume_requested_total: AtomicU64,
    resume_unavailable_total: AtomicU64,
    resume_marked_total: AtomicU64,
    resume_replayed_total: AtomicU64,
    resume_fell_behind_total: AtomicU64,
    resume_clear_partition_total: AtomicU64,
    last_clear_partition: AtomicU64,
    inbound_lag: AtomicU64,
}

impl TransportMetrics {
    /// Return a stable snapshot of all counters.
    pub fn snapshot(&self) -> TransportMetricsSnapshot {
        let resume_clear_partition_total =
            self.resume_clear_partition_total.load(Ordering::Relaxed);
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
            rate_limited_total: self.rate_limited_total.load(Ordering::Relaxed),
            resume_requested_total: self.resume_requested_total.load(Ordering::Relaxed),
            resume_unavailable_total: self.resume_unavailable_total.load(Ordering::Relaxed),
            resume_marked_total: self.resume_marked_total.load(Ordering::Relaxed),
            resume_replayed_total: self.resume_replayed_total.load(Ordering::Relaxed),
            resume_fell_behind_total: self.resume_fell_behind_total.load(Ordering::Relaxed),
            resume_clear_partition_total,
            last_clear_partition: if resume_clear_partition_total == 0 {
                None
            } else {
                Some(PartitionId::new(
                    self.last_clear_partition.load(Ordering::Relaxed) as u32,
                ))
            },
            inbound_lag: self.inbound_lag.load(Ordering::Relaxed),
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

/// Static metric descriptor used by observability adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportMetricDescriptor {
    /// Metric name.
    pub name: &'static str,
    /// Bounded label keys allowed for this metric.
    pub labels: &'static [&'static str],
}

const KIND_DIRECTION_LABELS: &[&str] = &["kind", "direction"];

const TRANSPORT_METRIC_DESCRIPTORS: &[TransportMetricDescriptor] = &[
    TransportMetricDescriptor {
        name: "transport_published_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_received_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_deduped_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_fenced_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_dropped_full_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_replayed_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_resume_fell_behind_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_publish_error_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_rate_limited_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_bus_lag_total",
        labels: KIND_DIRECTION_LABELS,
    },
    TransportMetricDescriptor {
        name: "transport_inbound_lag",
        labels: KIND_DIRECTION_LABELS,
    },
];

/// Return the bounded-label metric descriptors emitted by the relay.
pub fn transport_metric_descriptors() -> &'static [TransportMetricDescriptor] {
    TRANSPORT_METRIC_DESCRIPTORS
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

#[derive(Debug)]
struct BoundedFrameQueue {
    capacity: usize,
    inner: Arc<StdMutex<VecDeque<CacheInvalidationFrame>>>,
    notify: Arc<Notify>,
}

impl BoundedFrameQueue {
    fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            inner: Arc::new(StdMutex::new(VecDeque::with_capacity(capacity.max(1)))),
            notify: Arc::new(Notify::new()),
        }
    }

    fn push_drop_oldest(&self, frame: CacheInvalidationFrame) -> QueuePush {
        let mut queue = self
            .inner
            .lock()
            .expect("transport frame queue mutex poisoned");
        let dropped = if queue.len() == self.capacity {
            queue.pop_front()
        } else {
            None
        };
        queue.push_back(frame);
        drop(queue);
        self.notify.notify_one();
        match dropped {
            Some(frame) => QueuePush::DroppedOldest(frame),
            None => QueuePush::Enqueued,
        }
    }

    async fn recv(&self, shutdown: &mut watch::Receiver<bool>) -> Option<CacheInvalidationFrame> {
        loop {
            if let Some(frame) = self
                .inner
                .lock()
                .expect("transport frame queue mutex poisoned")
                .pop_front()
            {
                return Some(frame);
            }

            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return None;
                    }
                }
                _ = self.notify.notified() => {}
            }
        }
    }
}

impl Clone for BoundedFrameQueue {
    fn clone(&self) -> Self {
        Self {
            capacity: self.capacity,
            inner: self.inner.clone(),
            notify: self.notify.clone(),
        }
    }
}

#[derive(Debug)]
enum QueuePush {
    Enqueued,
    DroppedOldest(CacheInvalidationFrame),
}

#[derive(Debug, Clone, Copy)]
struct ResumeRequest {
    last_seen: u64,
}

/// Running invalidation relay with observable metrics.
#[derive(Debug)]
pub struct InvalidationRelayHandle {
    join: JoinHandle<()>,
    shutdown: watch::Sender<bool>,
    resume: mpsc::UnboundedSender<ResumeRequest>,
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

    /// Request replay from the configured invalidation ring after `last_seen`.
    pub fn request_resume(&self, last_seen: u64) -> std::result::Result<(), TransportError> {
        self.resume
            .send(ResumeRequest { last_seen })
            .map_err(|_| TransportError::Backend("invalidation relay is closed".to_owned()))
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
        ring: Option<SharedInvalidationRing>,
        config: TransportConfig,
    ) -> InvalidationRelayHandle
    where
        B: CacheInvalidationFrameSink,
        T: InvalidationTransport + Clone,
    {
        let metrics = Arc::new(TransportMetrics::default());
        let bus_receiver = bus.subscribe();
        let outbound_queue = BoundedFrameQueue::new(config.outbound_capacity);
        let inbound_queue = BoundedFrameQueue::new(config.inbound_capacity);
        let (resume, resume_rx) = mpsc::unbounded_channel();
        let (shutdown, shutdown_rx) = watch::channel(false);
        let next_message_id = Arc::new(AtomicU64::new(1));

        let outbound_reader = tokio::spawn(run_outbound_bus_loop(
            shutdown_rx.clone(),
            outbound_queue.clone(),
            config.clone(),
            metrics.clone(),
            next_message_id,
            bus_receiver,
            resume.clone(),
        ));
        let outbound_publisher = tokio::spawn(run_outbound_transport_loop(
            shutdown_rx.clone(),
            outbound_queue,
            transport.clone(),
            metrics.clone(),
        ));
        let inbound_reader = tokio::spawn(run_inbound_transport_loop(
            shutdown_rx.clone(),
            inbound_queue.clone(),
            transport,
            metrics.clone(),
        ));
        let inbound_apply = tokio::spawn(run_inbound_apply_loop(
            shutdown_rx.clone(),
            inbound_queue.clone(),
            bus,
            config,
            metrics.clone(),
        ));
        let resume_loop = tokio::spawn(run_resume_loop(
            shutdown_rx,
            resume_rx,
            ring,
            inbound_queue,
            metrics.clone(),
        ));

        let supervisor_shutdown = shutdown.clone();
        let join = tokio::spawn(async move {
            let mut outbound_reader = outbound_reader;
            let mut outbound_publisher = outbound_publisher;
            let mut inbound_reader = inbound_reader;
            let mut inbound_apply = inbound_apply;
            let mut resume_loop = resume_loop;

            tokio::select! {
                _ = &mut outbound_reader => {}
                _ = &mut outbound_publisher => {}
                _ = &mut inbound_reader => {}
                _ = &mut inbound_apply => {}
                _ = &mut resume_loop => {}
            }

            let _ = supervisor_shutdown.send(true);
            outbound_reader.abort();
            outbound_publisher.abort();
            inbound_reader.abort();
            inbound_apply.abort();
            resume_loop.abort();
        });

        InvalidationRelayHandle {
            join,
            shutdown,
            resume,
            metrics,
        }
    }
}

async fn run_outbound_bus_loop(
    mut shutdown: watch::Receiver<bool>,
    outbound_queue: BoundedFrameQueue,
    config: TransportConfig,
    metrics: Arc<TransportMetrics>,
    next_message_id: Arc<AtomicU64>,
    mut bus_receiver: Box<dyn crate::invalidation_bus::CacheInvalidationReceiver>,
    resume: mpsc::UnboundedSender<ResumeRequest>,
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
                            &outbound_queue,
                            &config,
                            &metrics,
                            &next_message_id,
                            message,
                            &resume,
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
    outbound_queue: &BoundedFrameQueue,
    config: &TransportConfig,
    metrics: &TransportMetrics,
    next_message_id: &AtomicU64,
    message: CacheInvalidationMessage,
    resume: &mpsc::UnboundedSender<ResumeRequest>,
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

    if let QueuePush::DroppedOldest(dropped) = outbound_queue.push_drop_oldest(frame) {
        metrics
            .outbound_dropped_full_total
            .fetch_add(1, Ordering::Relaxed);
        metrics.resume_marked_total.fetch_add(1, Ordering::Relaxed);
        let last_seen = dropped.message_id().unwrap_or(message_id).saturating_sub(1);
        let _ = resume.send(ResumeRequest { last_seen });
    }
}

async fn run_outbound_transport_loop<T>(
    mut shutdown: watch::Receiver<bool>,
    outbound_queue: BoundedFrameQueue,
    transport: T,
    metrics: Arc<TransportMetrics>,
) where
    T: InvalidationTransport,
{
    loop {
        let Some(frame) = outbound_queue.recv(&mut shutdown).await else {
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

async fn run_inbound_transport_loop<T>(
    mut shutdown: watch::Receiver<bool>,
    inbound_queue: BoundedFrameQueue,
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
                        if matches!(
                            inbound_queue.push_drop_oldest(frame),
                            QueuePush::DroppedOldest(_)
                        ) {
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
    inbound_queue: BoundedFrameQueue,
    bus: Arc<B>,
    config: TransportConfig,
    metrics: Arc<TransportMetrics>,
) where
    B: CacheInvalidationFrameSink,
{
    let mut state = InboundApplyState::new(config.dedup_window);

    loop {
        let Some(frame) = inbound_queue.recv(&mut shutdown).await else {
            break;
        };
        if state.should_drop(&frame, &config, &metrics) {
            continue;
        }

        match frame
            .encode()
            .and_then(|bytes| bus.publish_encoded_frame(bytes))
        {
            Ok(()) => {
                metrics.applied_total.fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => {
                metrics
                    .bus_apply_error_total
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

async fn run_resume_loop(
    mut shutdown: watch::Receiver<bool>,
    mut resume_rx: mpsc::UnboundedReceiver<ResumeRequest>,
    ring: Option<SharedInvalidationRing>,
    inbound_queue: BoundedFrameQueue,
    metrics: Arc<TransportMetrics>,
) {
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            maybe_request = resume_rx.recv() => {
                let Some(request) = maybe_request else {
                    break;
                };
                metrics.resume_requested_total.fetch_add(1, Ordering::Relaxed);
                let Some(ring) = &ring else {
                    metrics.resume_unavailable_total.fetch_add(1, Ordering::Relaxed);
                    continue;
                };
                replay_from_ring(
                    ring,
                    request.last_seen,
                    &inbound_queue,
                    &metrics,
                )
                .await;
            }
        }
    }
}

async fn replay_from_ring(
    ring: &SharedInvalidationRing,
    last_seen: u64,
    inbound_queue: &BoundedFrameQueue,
    metrics: &TransportMetrics,
) {
    let mut ring = ring.lock().await;
    metrics.inbound_lag.store(
        ring.next_seq().saturating_sub(last_seen.saturating_add(1)),
        Ordering::Relaxed,
    );
    match ring.replay_from(last_seen) {
        ReplayResult::Range(events) => {
            let replayed = events.len() as u64;
            for event in events {
                if matches!(
                    inbound_queue.push_drop_oldest(frame_from_replay_event(event)),
                    QueuePush::DroppedOldest(_)
                ) {
                    metrics
                        .inbound_dropped_full_total
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
            metrics
                .resume_replayed_total
                .fetch_add(replayed, Ordering::Relaxed);
        }
        ReplayResult::FellBehind { clear_partition } => {
            metrics
                .resume_fell_behind_total
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .resume_clear_partition_total
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .last_clear_partition
                .store(u64::from(clear_partition.value()), Ordering::Relaxed);
            if matches!(
                inbound_queue.push_drop_oldest(clear_partition_frame(clear_partition, last_seen)),
                QueuePush::DroppedOldest(_)
            ) {
                metrics
                    .inbound_dropped_full_total
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

fn frame_from_replay_event(event: InvalidationEvent) -> CacheInvalidationFrame {
    let mut message = CacheInvalidationMessage::new("transport-resume", event.invalidation);
    if let Some(generation) = event.source_generation {
        message = message.with_source_generation(generation);
    }
    CacheInvalidationFrame::new(message).with_message_id(event.sequence)
}

fn clear_partition_frame(partition: PartitionId, last_seen: u64) -> CacheInvalidationFrame {
    CacheInvalidationFrame::new(CacheInvalidationMessage::new(
        format!("transport-resume-partition-{}", partition.value()),
        CacheInvalidation::flush(),
    ))
    .with_message_id(last_seen.saturating_add(1))
}

struct InboundApplyState {
    dedup: DedupWindow,
    highest_generation: HashMap<String, ClusterGeneration>,
    rate_limited_by_source: HashMap<String, u64>,
}

impl InboundApplyState {
    fn new(dedup_window: usize) -> Self {
        Self {
            dedup: DedupWindow::new(dedup_window),
            highest_generation: HashMap::new(),
            rate_limited_by_source: HashMap::new(),
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

        if self.is_rate_limited(frame, config) {
            metrics.rate_limited_total.fetch_add(1, Ordering::Relaxed);
            return true;
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

    fn is_rate_limited(
        &mut self,
        frame: &CacheInvalidationFrame,
        config: &TransportConfig,
    ) -> bool {
        let Some(limit) = config.inbound_rate_limit_per_source else {
            return false;
        };
        let count = self
            .rate_limited_by_source
            .entry(frame.source_id().to_owned())
            .or_insert(0);
        if *count >= limit {
            return true;
        }
        *count = count.saturating_add(1);
        false
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
