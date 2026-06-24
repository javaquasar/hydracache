use std::collections::VecDeque;

use crate::cluster::{ClusterGeneration, PartitionId};
use crate::invalidation_bus::CacheInvalidation;

/// Sequence-numbered invalidation retained for replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationEvent {
    /// Monotonic ring sequence.
    pub sequence: u64,
    /// Partition covered by this event.
    pub partition: PartitionId,
    /// Cache invalidation payload.
    pub invalidation: CacheInvalidation,
    /// Optional publishing generation.
    pub source_generation: Option<ClusterGeneration>,
}

/// Replay result for a subscriber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayResult {
    /// Subscriber is within retention and receives the exact missed range.
    Range(Vec<InvalidationEvent>),
    /// Subscriber fell behind retention and must clear the partition.
    FellBehind { clear_partition: PartitionId },
}

/// Ring metrics with bounded label sets represented as counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InvalidationRingMetrics {
    /// Current retained event count.
    pub invalidation_ring_depth: u64,
    /// Exact replayed events.
    pub invalidation_replayed_total: u64,
    /// ClearPartition fallbacks.
    pub invalidation_fell_behind_total: u64,
    /// Events overwritten by a full ring.
    pub invalidation_ring_overrun_total: u64,
}

/// Bounded replayable invalidation ring for one partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationRing {
    partition: PartitionId,
    capacity: usize,
    head_seq: u64,
    next_seq: u64,
    events: VecDeque<InvalidationEvent>,
    metrics: InvalidationRingMetrics,
}

impl InvalidationRing {
    /// Create an empty ring with normalized non-zero capacity.
    pub fn new(partition: PartitionId, capacity: usize) -> Self {
        Self {
            partition,
            capacity: capacity.max(1),
            head_seq: 0,
            next_seq: 0,
            events: VecDeque::new(),
            metrics: InvalidationRingMetrics::default(),
        }
    }

    /// Return the covered partition.
    pub fn partition(&self) -> PartitionId {
        self.partition
    }

    /// Return oldest retained sequence.
    pub fn head_seq(&self) -> u64 {
        self.head_seq
    }

    /// Return the next sequence that will be assigned.
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    /// Return current metrics.
    pub fn metrics(&self) -> InvalidationRingMetrics {
        let mut metrics = self.metrics;
        metrics.invalidation_ring_depth = self.events.len() as u64;
        metrics
    }

    /// Publish one invalidation without blocking.
    pub fn publish(&mut self, invalidation: CacheInvalidation) -> u64 {
        self.publish_with_generation(invalidation, None)
    }

    /// Publish one invalidation with source generation metadata.
    pub fn publish_with_generation(
        &mut self,
        invalidation: CacheInvalidation,
        source_generation: Option<ClusterGeneration>,
    ) -> u64 {
        let sequence = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        if self.events.len() == self.capacity {
            self.events.pop_front();
            self.metrics.invalidation_ring_overrun_total = self
                .metrics
                .invalidation_ring_overrun_total
                .saturating_add(1);
        }

        self.events.push_back(InvalidationEvent {
            sequence,
            partition: self.partition,
            invalidation,
            source_generation,
        });
        self.head_seq = self
            .events
            .front()
            .map(|event| event.sequence)
            .unwrap_or(self.next_seq);
        sequence
    }

    /// Replay events after `last_seen`, or fall back to ClearPartition if retention was exceeded.
    pub fn replay_from(&mut self, last_seen: u64) -> ReplayResult {
        if self.events.is_empty() {
            return ReplayResult::Range(Vec::new());
        }
        if last_seen.saturating_add(1) < self.head_seq {
            self.metrics.invalidation_fell_behind_total = self
                .metrics
                .invalidation_fell_behind_total
                .saturating_add(1);
            return ReplayResult::FellBehind {
                clear_partition: self.partition,
            };
        }

        let range = self
            .events
            .iter()
            .filter(|event| event.sequence > last_seen)
            .cloned()
            .collect::<Vec<_>>();
        self.metrics.invalidation_replayed_total = self
            .metrics
            .invalidation_replayed_total
            .saturating_add(range.len() as u64);
        ReplayResult::Range(range)
    }

    /// Snapshot the retained window for durable adapters.
    pub fn snapshot(&self) -> InvalidationRingSnapshot {
        InvalidationRingSnapshot {
            partition: self.partition,
            capacity: self.capacity,
            head_seq: self.head_seq,
            next_seq: self.next_seq,
            events: self.events.iter().cloned().collect(),
        }
    }

    /// Restore a retained window from a durable adapter snapshot.
    pub fn restore(snapshot: InvalidationRingSnapshot) -> Self {
        Self {
            partition: snapshot.partition,
            capacity: snapshot.capacity.max(1),
            head_seq: snapshot.head_seq,
            next_seq: snapshot.next_seq,
            events: snapshot.events.into(),
            metrics: InvalidationRingMetrics::default(),
        }
    }
}

/// Durable-adapter snapshot of the recent invalidation window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationRingSnapshot {
    /// Covered partition.
    pub partition: PartitionId,
    /// Ring capacity.
    pub capacity: usize,
    /// Oldest retained sequence.
    pub head_seq: u64,
    /// Next sequence to assign.
    pub next_seq: u64,
    /// Retained events.
    pub events: Vec<InvalidationEvent>,
}
