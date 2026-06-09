use std::fmt;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use hydracache_core::{CacheEvent, CacheEventKind, CacheEventOptions};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::stats::StatsCounters;

#[derive(Debug)]
pub(crate) struct EventBus {
    sender: broadcast::Sender<CacheEvent>,
    access_events: bool,
}

impl EventBus {
    pub(crate) fn new(capacity: usize, access_events: bool) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self {
            sender,
            access_events,
        }
    }

    pub(crate) fn subscribe(
        &self,
        options: CacheEventOptions,
        stats: Arc<StatsCounters>,
    ) -> CacheEventSubscriber {
        CacheEventSubscriber {
            receiver: self.sender.subscribe(),
            options,
            stats,
        }
    }

    pub(crate) fn publish(&self, event: CacheEvent, stats: &StatsCounters) {
        if !self.should_publish(event.kind()) || self.sender.receiver_count() == 0 {
            return;
        }

        if self.sender.send(event).is_ok() {
            stats.events_published.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn should_publish(&self, kind: CacheEventKind) -> bool {
        self.access_events || kind.is_mutation()
    }
}

/// Error returned by [`CacheEventSubscriber::recv`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheEventRecvError {
    /// The cache event bus has been closed because the cache was dropped.
    Closed,
    /// The subscriber was too slow and skipped this many events.
    Lagged(u64),
}

impl fmt::Display for CacheEventRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => f.write_str("cache event subscription closed"),
            Self::Lagged(skipped) => write!(f, "cache event subscriber lagged by {skipped} events"),
        }
    }
}

impl std::error::Error for CacheEventRecvError {}

/// Receiver for cache events emitted by one [`HydraCache`](crate::HydraCache).
///
/// Dropping the subscriber automatically unregisters it. Slow subscribers may
/// receive [`CacheEventRecvError::Lagged`] because the underlying event bus is a
/// bounded ring buffer and cache operations never wait for listeners.
#[derive(Debug)]
pub struct CacheEventSubscriber {
    receiver: broadcast::Receiver<CacheEvent>,
    options: CacheEventOptions,
    stats: Arc<StatsCounters>,
}

impl CacheEventSubscriber {
    /// Receive the next event matching this subscriber's filters.
    pub async fn recv(&mut self) -> Result<CacheEvent, CacheEventRecvError> {
        loop {
            match self.receiver.recv().await {
                Ok(event) if self.options.matches(&event) => return Ok(event),
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(CacheEventRecvError::Closed);
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    self.stats
                        .event_subscriber_lagged
                        .fetch_add(skipped, Ordering::Relaxed);
                    return Err(CacheEventRecvError::Lagged(skipped));
                }
            }
        }
    }

    /// Receive the next matching event and skip lag notifications.
    ///
    /// This helper is useful for dashboards and lightweight listeners where
    /// "latest matching event" is more important than reacting to lag as an
    /// error. Use [`recv`](Self::recv) when lag must be handled explicitly.
    pub async fn next_event(&mut self) -> Option<CacheEvent> {
        loop {
            match self.recv().await {
                Ok(event) => return Some(event),
                Err(CacheEventRecvError::Closed) => return None,
                Err(CacheEventRecvError::Lagged(_)) => continue,
            }
        }
    }

    /// Return this subscriber's immutable filter options.
    pub fn options(&self) -> &CacheEventOptions {
        &self.options
    }
}

/// Handle for a callback-style cache event listener.
///
/// Dropping the handle unsubscribes the listener by aborting its background
/// task. The callback is never run on the cache operation hot path.
pub struct CacheEventListenerHandle {
    task: JoinHandle<()>,
}

impl CacheEventListenerHandle {
    pub(crate) fn spawn<F>(mut subscriber: CacheEventSubscriber, listener: F) -> Self
    where
        F: Fn(CacheEvent) + Send + 'static,
    {
        let task = tokio::spawn(async move {
            while let Some(event) = subscriber.next_event().await {
                listener(event);
            }
        });

        Self { task }
    }

    /// Unsubscribe this listener.
    pub fn unsubscribe(self) {
        self.task.abort();
    }

    /// Return whether the background listener task has finished.
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }
}

impl Drop for CacheEventListenerHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}
