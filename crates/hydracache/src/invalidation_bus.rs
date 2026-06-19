use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache_core::{CacheCodec, CacheError, PostcardCodec, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::cluster::ClusterGeneration;

/// Current binary encoding version for cross-process invalidation frames.
pub const CACHE_INVALIDATION_FRAME_VERSION: u16 = 1;

/// Cache invalidation operation that can be propagated to another cache node.
///
/// The operation intentionally carries no cached value. A bus invalidates local
/// entries on other nodes; it does not replicate data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheInvalidation {
    /// Invalidate one physical cache key.
    Key {
        /// Physical cache key.
        key: String,
    },
    /// Invalidate all entries associated with one tag.
    Tag {
        /// Invalidation tag.
        tag: String,
    },
    /// Flush the whole local cache on receiving nodes.
    Flush,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum CacheInvalidationFrameKind {
    Key { key: String },
    Tag { tag: String },
    Flush,
}

impl From<CacheInvalidation> for CacheInvalidationFrameKind {
    fn from(invalidation: CacheInvalidation) -> Self {
        match invalidation {
            CacheInvalidation::Key { key } => Self::Key { key },
            CacheInvalidation::Tag { tag } => Self::Tag { tag },
            CacheInvalidation::Flush => Self::Flush,
        }
    }
}

impl From<CacheInvalidationFrameKind> for CacheInvalidation {
    fn from(kind: CacheInvalidationFrameKind) -> Self {
        match kind {
            CacheInvalidationFrameKind::Key { key } => Self::Key { key },
            CacheInvalidationFrameKind::Tag { tag } => Self::Tag { tag },
            CacheInvalidationFrameKind::Flush => Self::Flush,
        }
    }
}

impl CacheInvalidation {
    /// Create a key invalidation operation.
    pub fn key(key: impl Into<String>) -> Self {
        Self::Key { key: key.into() }
    }

    /// Create a tag invalidation operation.
    pub fn tag(tag: impl Into<String>) -> Self {
        Self::Tag { tag: tag.into() }
    }

    /// Create a cache-wide flush operation.
    pub fn flush() -> Self {
        Self::Flush
    }

    /// Return the key when this is a key invalidation.
    pub fn key_value(&self) -> Option<&str> {
        match self {
            Self::Key { key } => Some(key),
            Self::Tag { .. } | Self::Flush => None,
        }
    }

    /// Return the tag when this is a tag invalidation.
    pub fn tag_value(&self) -> Option<&str> {
        match self {
            Self::Tag { tag } => Some(tag),
            Self::Key { .. } | Self::Flush => None,
        }
    }

    /// Return whether this operation flushes the whole cache.
    pub fn is_flush(&self) -> bool {
        matches!(self, Self::Flush)
    }
}

/// Invalidation message published on a [`CacheInvalidationBus`].
///
/// `source_id` is used by each cache instance to ignore its own messages and
/// avoid echo loops when several caches share the same bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheInvalidationMessage {
    source_id: String,
    source_generation: Option<ClusterGeneration>,
    invalidation: CacheInvalidation,
}

impl CacheInvalidationMessage {
    /// Create an invalidation message from a source node id.
    pub fn new(source_id: impl Into<String>, invalidation: CacheInvalidation) -> Self {
        Self {
            source_id: source_id.into(),
            source_generation: None,
            invalidation,
        }
    }

    /// Attach the cluster generation that published this message.
    pub fn with_source_generation(mut self, generation: ClusterGeneration) -> Self {
        self.source_generation = Some(generation);
        self
    }

    /// Return the id of the cache instance that published this message.
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Return the cluster generation that published this message, if known.
    pub fn source_generation(&self) -> Option<ClusterGeneration> {
        self.source_generation
    }

    /// Return the invalidation operation.
    pub fn invalidation(&self) -> &CacheInvalidation {
        &self.invalidation
    }

    pub(crate) fn into_parts(self) -> (String, CacheInvalidation) {
        (self.source_id, self.invalidation)
    }
}

/// Binary envelope for invalidation messages that cross a process boundary.
///
/// The frame keeps transport metadata outside the hot local invalidation
/// operation: protocol version, optional cluster name, optional message id for
/// diagnostics/idempotency, source node id, source generation, and the
/// key/tag/flush operation. Real transports can encode this frame into TCP,
/// Redis, NATS, Postgres notifications, or another byte-oriented channel.
///
/// # Example
///
/// ```rust
/// use hydracache::{
///     CacheInvalidation, CacheInvalidationFrame, CacheInvalidationMessage,
///     ClusterGeneration,
/// };
///
/// let message = CacheInvalidationMessage::new("member-a", CacheInvalidation::tag("users"))
///     .with_source_generation(ClusterGeneration::new(3));
/// let frame = CacheInvalidationFrame::new(message)
///     .with_cluster_name("orders")
///     .with_message_id(42);
///
/// let encoded = frame.encode().unwrap();
/// let decoded = CacheInvalidationFrame::decode(&encoded).unwrap();
///
/// assert_eq!(decoded.cluster_name(), Some("orders"));
/// assert_eq!(decoded.message_id(), Some(42));
/// assert_eq!(decoded.source_generation(), Some(ClusterGeneration::new(3)));
/// assert_eq!(decoded.invalidation().tag_value(), Some("users"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheInvalidationFrame {
    version: u16,
    message_id: Option<u64>,
    cluster_name: Option<String>,
    source_id: String,
    source_generation: Option<u64>,
    invalidation: CacheInvalidationFrameKind,
}

impl CacheInvalidationFrame {
    /// Build a frame from a cache invalidation message.
    pub fn new(message: CacheInvalidationMessage) -> Self {
        let source_generation = message.source_generation().map(ClusterGeneration::value);
        Self {
            version: CACHE_INVALIDATION_FRAME_VERSION,
            message_id: None,
            cluster_name: None,
            source_id: message.source_id().to_owned(),
            source_generation,
            invalidation: message.invalidation().clone().into(),
        }
    }

    /// Attach a logical cluster name for diagnostics and transport filtering.
    pub fn with_cluster_name(mut self, cluster_name: impl Into<String>) -> Self {
        self.cluster_name = Some(cluster_name.into());
        self
    }

    /// Attach a monotonic transport message id.
    pub fn with_message_id(mut self, message_id: u64) -> Self {
        self.message_id = Some(message_id);
        self
    }

    /// Return the wire-format version.
    pub fn version(&self) -> u16 {
        self.version
    }

    /// Return the optional logical cluster name.
    pub fn cluster_name(&self) -> Option<&str> {
        self.cluster_name.as_deref()
    }

    /// Return the optional transport message id.
    pub fn message_id(&self) -> Option<u64> {
        self.message_id
    }

    /// Return the publishing node id.
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Return the publishing generation, if the source is cluster-backed.
    pub fn source_generation(&self) -> Option<ClusterGeneration> {
        self.source_generation.map(ClusterGeneration::new)
    }

    /// Return the invalidation operation in this frame.
    pub fn invalidation(&self) -> CacheInvalidation {
        self.invalidation.clone().into()
    }

    /// Convert this frame back into the message consumed by HydraCache.
    pub fn into_message(self) -> CacheInvalidationMessage {
        let mut message = CacheInvalidationMessage::new(self.source_id, self.invalidation.into());
        if let Some(generation) = self.source_generation {
            message = message.with_source_generation(ClusterGeneration::new(generation));
        }
        message
    }

    /// Encode this frame into compact bytes.
    pub fn encode(&self) -> Result<Bytes> {
        PostcardCodec.encode(self)
    }

    /// Decode a frame from bytes and reject unsupported encoding versions.
    pub fn decode(bytes: &Bytes) -> Result<Self> {
        let frame: Self = PostcardCodec.decode(bytes)?;
        if frame.version != CACHE_INVALIDATION_FRAME_VERSION {
            return Err(CacheError::Decode(format!(
                "unsupported invalidation frame version {}",
                frame.version
            )));
        }
        Ok(frame)
    }
}

/// Result of polling an invalidation receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheInvalidationReceive {
    /// A valid invalidation message is ready to apply.
    Message(CacheInvalidationMessage),
    /// The receiver skipped messages because it lagged behind the bus.
    ///
    /// A cache records this as diagnostics and keeps listening for the next
    /// message. External transports can use this when they detect dropped
    /// messages, truncated streams, or compacted offsets.
    Lagged(u64),
    /// The bus stream closed and the background listener should stop.
    Closed,
    /// A transport frame could not be decoded.
    DecodeError(String),
}

impl CacheInvalidationReceive {
    /// Wrap a message receive result.
    pub fn message(message: CacheInvalidationMessage) -> Self {
        Self::Message(message)
    }

    /// Return whether this result reports a closed bus stream.
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed)
    }

    /// Return the number of skipped messages when this result reports lag.
    pub fn lagged_count(&self) -> Option<u64> {
        match self {
            Self::Lagged(count) => Some(*count),
            Self::Message(_) | Self::Closed | Self::DecodeError(_) => None,
        }
    }

    /// Return the decode error message when a transport frame was invalid.
    pub fn decode_error(&self) -> Option<&str> {
        match self {
            Self::DecodeError(error) => Some(error),
            Self::Message(_) | Self::Lagged(_) | Self::Closed => None,
        }
    }
}

/// Receiver side of a cache invalidation bus.
#[async_trait]
pub trait CacheInvalidationReceiver: Send + 'static {
    /// Receive the next invalidation message.
    ///
    /// [`CacheInvalidationReceive::Closed`] means the bus is closed and the
    /// background cache sync task should exit. [`CacheInvalidationReceive::DecodeError`]
    /// means one transport frame was skipped and the receiver can continue.
    async fn recv(&mut self) -> CacheInvalidationReceive;
}

/// Transport abstraction for cross-cache invalidation.
///
/// Implement this trait for a real transport such as Postgres LISTEN/NOTIFY,
/// Redis Pub/Sub, NATS, or an application-specific message bus. HydraCache only
/// requires fire-and-forget invalidation messages; values are never replicated.
#[async_trait]
pub trait CacheInvalidationBus: fmt::Debug + Send + Sync + 'static {
    /// Publish an invalidation message.
    async fn publish(&self, message: CacheInvalidationMessage) -> Result<()>;

    /// Subscribe to invalidation messages.
    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver>;
}

/// In-process invalidation bus for tests, demos, and embedded multi-cache apps.
///
/// This bus uses a bounded Tokio broadcast channel. It is not durable and does
/// not cross process boundaries, but it exercises the same HydraCache sync path
/// that future external transports will use.
#[derive(Debug, Clone)]
pub struct InMemoryInvalidationBus {
    sender: broadcast::Sender<CacheInvalidationMessage>,
}

impl InMemoryInvalidationBus {
    /// Create an in-memory bus with a bounded subscriber buffer.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    /// Return the number of currently active bus subscribers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for InMemoryInvalidationBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[async_trait]
impl CacheInvalidationBus for InMemoryInvalidationBus {
    async fn publish(&self, message: CacheInvalidationMessage) -> Result<()> {
        let _ = self.sender.send(message);
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(InMemoryInvalidationReceiver {
            receiver: self.sender.subscribe(),
        })
    }
}

/// In-memory framed invalidation bus for cross-process transport experiments.
///
/// Unlike [`InMemoryInvalidationBus`], this bus encodes every message into a
/// [`CacheInvalidationFrame`] and sends bytes through the channel. It is still
/// in-process and non-durable, but it exercises the same encode/decode boundary
/// a TCP, Redis, NATS, or Postgres transport would need.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use hydracache::{HydraCache, InMemoryFramedInvalidationBus};
///
/// # #[tokio::main]
/// # async fn main() -> hydracache::CacheResult<()> {
/// let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 128));
/// let first = HydraCache::local()
///     .shared_invalidation_bus(bus.clone())
///     .invalidation_node_id("first")
///     .build();
/// let second = HydraCache::local()
///     .shared_invalidation_bus(bus)
///     .invalidation_node_id("second")
///     .build();
///
/// # let _ = (first, second);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryFramedInvalidationBus {
    sender: broadcast::Sender<Bytes>,
    cluster_name: Option<String>,
    next_message_id: Arc<AtomicU64>,
    closed: Arc<AtomicBool>,
}

impl InMemoryFramedInvalidationBus {
    /// Create a framed bus without a logical cluster name.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self {
            sender,
            cluster_name: None,
            next_message_id: Arc::new(AtomicU64::new(1)),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a framed bus that annotates each frame with a cluster name.
    pub fn for_cluster(cluster_name: impl Into<String>, capacity: usize) -> Self {
        Self {
            cluster_name: Some(cluster_name.into()),
            ..Self::new(capacity)
        }
    }

    /// Return the number of active frame receivers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Return whether this test transport has been closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    /// Close this test transport and wake existing receivers.
    pub fn close(&self) {
        if !self.closed.swap(true, Ordering::Relaxed) {
            let _ = self.sender.send(Bytes::new());
        }
    }

    /// Publish already-encoded frame bytes.
    ///
    /// This is useful for tests and future transport adapters that receive
    /// bytes outside HydraCache and want to reuse the same decode path.
    pub fn publish_encoded_frame(&self, bytes: Bytes) -> Result<()> {
        if self.is_closed() {
            return Err(CacheError::Backend(
                "framed invalidation transport is closed".to_owned(),
            ));
        }
        let _ = self.sender.send(bytes);
        Ok(())
    }
}

impl Default for InMemoryFramedInvalidationBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[async_trait]
impl CacheInvalidationBus for InMemoryFramedInvalidationBus {
    async fn publish(&self, message: CacheInvalidationMessage) -> Result<()> {
        if self.is_closed() {
            return Err(CacheError::Backend(
                "framed invalidation transport is closed".to_owned(),
            ));
        }

        let message_id = self.next_message_id.fetch_add(1, Ordering::Relaxed);
        let mut frame = CacheInvalidationFrame::new(message).with_message_id(message_id);
        if let Some(cluster_name) = &self.cluster_name {
            frame = frame.with_cluster_name(cluster_name.clone());
        }
        self.publish_encoded_frame(frame.encode()?)
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(InMemoryFramedInvalidationReceiver {
            receiver: self.sender.subscribe(),
            closed: self.closed.clone(),
        })
    }
}

struct InMemoryFramedInvalidationReceiver {
    receiver: broadcast::Receiver<Bytes>,
    closed: Arc<AtomicBool>,
}

#[async_trait]
impl CacheInvalidationReceiver for InMemoryFramedInvalidationReceiver {
    async fn recv(&mut self) -> CacheInvalidationReceive {
        if self.closed.load(Ordering::Relaxed) {
            return CacheInvalidationReceive::Closed;
        }
        match self.receiver.recv().await {
            Ok(bytes) if bytes.is_empty() && self.closed.load(Ordering::Relaxed) => {
                CacheInvalidationReceive::Closed
            }
            Ok(bytes) => match CacheInvalidationFrame::decode(&bytes) {
                Ok(frame) => CacheInvalidationReceive::Message(frame.into_message()),
                Err(error) => CacheInvalidationReceive::DecodeError(error.to_string()),
            },
            Err(broadcast::error::RecvError::Closed) => CacheInvalidationReceive::Closed,
            Err(broadcast::error::RecvError::Lagged(count)) => {
                CacheInvalidationReceive::Lagged(count)
            }
        }
    }
}

struct InMemoryInvalidationReceiver {
    receiver: broadcast::Receiver<CacheInvalidationMessage>,
}

#[async_trait]
impl CacheInvalidationReceiver for InMemoryInvalidationReceiver {
    async fn recv(&mut self) -> CacheInvalidationReceive {
        match self.receiver.recv().await {
            Ok(message) => CacheInvalidationReceive::Message(message),
            Err(broadcast::error::RecvError::Closed) => CacheInvalidationReceive::Closed,
            Err(broadcast::error::RecvError::Lagged(count)) => {
                CacheInvalidationReceive::Lagged(count)
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/frame_version_compat.rs"]
mod frame_version_compat;

#[cfg(test)]
mod tests {
    use super::{
        CacheInvalidation, CacheInvalidationBus, CacheInvalidationFrame, CacheInvalidationMessage,
        CacheInvalidationReceive, InMemoryFramedInvalidationBus, InMemoryInvalidationBus,
        CACHE_INVALIDATION_FRAME_VERSION,
    };
    use crate::ClusterGeneration;

    #[test]
    fn invalidation_helpers_expose_operation_metadata() {
        let key = CacheInvalidation::key("user:42");
        let tag = CacheInvalidation::tag("users");
        let flush = CacheInvalidation::flush();

        assert_eq!(key.key_value(), Some("user:42"));
        assert_eq!(key.tag_value(), None);
        assert_eq!(tag.tag_value(), Some("users"));
        assert_eq!(tag.key_value(), None);
        assert!(flush.is_flush());
        assert_eq!(flush.key_value(), None);
        assert_eq!(flush.tag_value(), None);
    }

    #[tokio::test]
    async fn in_memory_bus_delivers_messages_to_subscribers() {
        let bus = InMemoryInvalidationBus::new(8);
        let mut subscriber = bus.subscribe();

        bus.publish(CacheInvalidationMessage::new(
            "node-a",
            CacheInvalidation::tag("users"),
        ))
        .await
        .unwrap();

        let CacheInvalidationReceive::Message(message) = subscriber.recv().await else {
            panic!("expected invalidation message");
        };
        assert_eq!(message.source_id(), "node-a");
        assert_eq!(message.source_generation(), None);
        assert_eq!(message.invalidation().tag_value(), Some("users"));
    }

    #[test]
    fn invalidation_message_can_carry_cluster_generation() {
        let message = CacheInvalidationMessage::new("node-a", CacheInvalidation::flush())
            .with_source_generation(ClusterGeneration::new(7));

        assert_eq!(message.source_id(), "node-a");
        assert_eq!(message.source_generation(), Some(ClusterGeneration::new(7)));
        assert!(message.invalidation().is_flush());
    }

    #[test]
    fn invalidation_frame_roundtrips_transport_metadata_and_message() {
        let message = CacheInvalidationMessage::new("member-a", CacheInvalidation::tag("users"))
            .with_source_generation(ClusterGeneration::new(7));
        let frame = CacheInvalidationFrame::new(message)
            .with_cluster_name("orders")
            .with_message_id(42);

        let encoded = frame.encode().unwrap();
        let decoded = CacheInvalidationFrame::decode(&encoded).unwrap();
        let decoded_message = decoded.clone().into_message();

        assert_eq!(decoded.version(), CACHE_INVALIDATION_FRAME_VERSION);
        assert_eq!(decoded.cluster_name(), Some("orders"));
        assert_eq!(decoded.message_id(), Some(42));
        assert_eq!(decoded.source_id(), "member-a");
        assert_eq!(decoded.source_generation(), Some(ClusterGeneration::new(7)));
        assert_eq!(decoded.invalidation().tag_value(), Some("users"));
        assert_eq!(decoded_message.source_id(), "member-a");
        assert_eq!(
            decoded_message.source_generation(),
            Some(ClusterGeneration::new(7))
        );
        assert_eq!(decoded_message.invalidation().tag_value(), Some("users"));
    }

    #[test]
    fn invalidation_frame_rejects_unsupported_encoding_version() {
        let mut frame = CacheInvalidationFrame::new(CacheInvalidationMessage::new(
            "member-a",
            CacheInvalidation::flush(),
        ));
        frame.version = CACHE_INVALIDATION_FRAME_VERSION + 1;
        let encoded = frame.encode().unwrap();

        let error = CacheInvalidationFrame::decode(&encoded).unwrap_err();

        assert!(error
            .to_string()
            .contains("unsupported invalidation frame version"));
    }

    #[tokio::test]
    async fn framed_bus_delivers_messages_through_binary_frames() {
        let bus = InMemoryFramedInvalidationBus::for_cluster("orders", 8);
        let mut subscriber = bus.subscribe();

        bus.publish(
            CacheInvalidationMessage::new("member-a", CacheInvalidation::key("user:42"))
                .with_source_generation(ClusterGeneration::new(3)),
        )
        .await
        .unwrap();

        let CacheInvalidationReceive::Message(message) = subscriber.recv().await else {
            panic!("expected decoded invalidation message");
        };
        assert_eq!(message.source_id(), "member-a");
        assert_eq!(message.source_generation(), Some(ClusterGeneration::new(3)));
        assert_eq!(message.invalidation().key_value(), Some("user:42"));
    }

    #[tokio::test]
    async fn framed_bus_reports_decode_errors_without_closing_receiver() {
        let bus = InMemoryFramedInvalidationBus::new(8);
        let mut subscriber = bus.subscribe();

        bus.publish_encoded_frame(bytes::Bytes::from_static(b"not-a-frame"))
            .unwrap();
        bus.publish(CacheInvalidationMessage::new(
            "member-a",
            CacheInvalidation::tag("users"),
        ))
        .await
        .unwrap();

        let decode_error = subscriber.recv().await;
        assert!(decode_error.decode_error().is_some());

        let CacheInvalidationReceive::Message(message) = subscriber.recv().await else {
            panic!("receiver should continue after a decode error");
        };
        assert_eq!(message.invalidation().tag_value(), Some("users"));
    }

    #[tokio::test]
    async fn framed_bus_close_wakes_receivers_and_rejects_publishes() {
        let bus = InMemoryFramedInvalidationBus::new(8);
        let mut subscriber = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        bus.close();

        assert!(bus.is_closed());
        assert_eq!(subscriber.recv().await, CacheInvalidationReceive::Closed);
        let error = bus
            .publish(CacheInvalidationMessage::new(
                "member-a",
                CacheInvalidation::flush(),
            ))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("framed invalidation transport is closed"));
    }

    #[test]
    fn in_memory_bus_default_and_receiver_count_are_observable() {
        let bus = InMemoryInvalidationBus::default();
        assert_eq!(bus.receiver_count(), 0);

        let first = bus.subscribe();
        let second = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);

        drop(first);
        drop(second);
        assert_eq!(bus.receiver_count(), 0);
    }

    #[tokio::test]
    async fn in_memory_receiver_returns_none_when_bus_is_closed() {
        let bus = InMemoryInvalidationBus::new(1);
        let mut subscriber = bus.subscribe();

        drop(bus);

        assert_eq!(subscriber.recv().await, CacheInvalidationReceive::Closed);
    }

    #[tokio::test]
    async fn in_memory_receiver_reports_lagged_messages_before_latest() {
        let bus = InMemoryInvalidationBus::new(1);
        let mut subscriber = bus.subscribe();

        for key in ["stale-1", "stale-2", "latest"] {
            bus.publish(CacheInvalidationMessage::new(
                "node-a",
                CacheInvalidation::key(key),
            ))
            .await
            .unwrap();
        }

        let lag = subscriber.recv().await;
        assert_eq!(lag.lagged_count(), Some(2));

        let CacheInvalidationReceive::Message(message) = subscriber.recv().await else {
            panic!("expected latest invalidation message after lag notification");
        };
        assert_eq!(message.invalidation().key_value(), Some("latest"));
    }

    #[test]
    fn receive_helpers_describe_closed_and_message_states() {
        let message = CacheInvalidationMessage::new("node-a", CacheInvalidation::key("user:42"));
        let received = CacheInvalidationReceive::message(message.clone());

        assert_eq!(received, CacheInvalidationReceive::Message(message));
        assert!(!received.is_closed());
        assert_eq!(received.lagged_count(), None);
        assert_eq!(received.decode_error(), None);
        assert!(CacheInvalidationReceive::Closed.is_closed());
        assert_eq!(CacheInvalidationReceive::Closed.lagged_count(), None);
        assert_eq!(CacheInvalidationReceive::Lagged(3).lagged_count(), Some(3));
        assert_eq!(
            CacheInvalidationReceive::DecodeError("bad frame".to_owned()).decode_error(),
            Some("bad frame")
        );
    }
}
