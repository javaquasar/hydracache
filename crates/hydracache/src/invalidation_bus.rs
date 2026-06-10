use std::fmt;

use async_trait::async_trait;
use hydracache_core::Result;
use tokio::sync::broadcast;

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
    invalidation: CacheInvalidation,
}

impl CacheInvalidationMessage {
    /// Create an invalidation message from a source node id.
    pub fn new(source_id: impl Into<String>, invalidation: CacheInvalidation) -> Self {
        Self {
            source_id: source_id.into(),
            invalidation,
        }
    }

    /// Return the id of the cache instance that published this message.
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Return the invalidation operation.
    pub fn invalidation(&self) -> &CacheInvalidation {
        &self.invalidation
    }

    pub(crate) fn into_parts(self) -> (String, CacheInvalidation) {
        (self.source_id, self.invalidation)
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
            Self::Message(_) | Self::Closed => None,
        }
    }
}

/// Receiver side of a cache invalidation bus.
#[async_trait]
pub trait CacheInvalidationReceiver: Send + 'static {
    /// Receive the next invalidation message.
    ///
    /// [`CacheInvalidationReceive::Closed`] means the bus is closed and the
    /// background cache sync task should exit.
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
mod tests {
    use super::{
        CacheInvalidation, CacheInvalidationBus, CacheInvalidationMessage,
        CacheInvalidationReceive, InMemoryInvalidationBus,
    };

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
        assert_eq!(message.invalidation().tag_value(), Some("users"));
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
        assert!(CacheInvalidationReceive::Closed.is_closed());
        assert_eq!(CacheInvalidationReceive::Closed.lagged_count(), None);
        assert_eq!(CacheInvalidationReceive::Lagged(3).lagged_count(), Some(3));
    }
}
