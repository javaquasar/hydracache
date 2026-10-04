//! Partition-watermarked, gap-visible listener model for provisional 0.75 proofs.

use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerBounds {
    pub max_partitions: usize,
    pub max_backlog: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenerEventKind {
    Mutation { key: Vec<u8>, removed: bool },
    Gap { after_watermark: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerEvent {
    pub partition: u32,
    pub generation: u64,
    pub watermark: u64,
    pub kind: ListenerEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerAcceptance {
    Delivered,
    DuplicateSuppressed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenerError {
    InvalidBound(&'static str),
    InvalidCursor,
    TooManyPartitions,
    UnknownPartition,
    StaleGeneration,
    GapRequired,
    RepairNotRequired,
    RepairBehindWatermark,
}

#[derive(Debug, Clone)]
struct PartitionCursor {
    generation: u64,
    watermark: u64,
    gap: bool,
}

#[derive(Debug)]
pub struct ClusterListenerModel {
    bounds: ListenerBounds,
    cursors: BTreeMap<u32, PartitionCursor>,
    backlog: VecDeque<ListenerEvent>,
}

impl ClusterListenerModel {
    pub fn new(bounds: ListenerBounds) -> Result<Self, ListenerError> {
        if bounds.max_partitions == 0 {
            return Err(ListenerError::InvalidBound("partitions"));
        }
        if bounds.max_backlog == 0 {
            return Err(ListenerError::InvalidBound("backlog"));
        }
        Ok(Self {
            bounds,
            cursors: BTreeMap::new(),
            backlog: VecDeque::new(),
        })
    }

    pub fn register(
        &mut self,
        partition: u32,
        generation: u64,
        watermark: u64,
    ) -> Result<(), ListenerError> {
        if generation == 0 {
            return Err(ListenerError::InvalidCursor);
        }
        if !self.cursors.contains_key(&partition)
            && self.cursors.len() >= self.bounds.max_partitions
        {
            return Err(ListenerError::TooManyPartitions);
        }
        self.cursors.insert(
            partition,
            PartitionCursor {
                generation,
                watermark,
                gap: false,
            },
        );
        Ok(())
    }

    pub fn publish(&mut self, event: ListenerEvent) -> Result<ListenerAcceptance, ListenerError> {
        let cursor = self
            .cursors
            .get(&event.partition)
            .ok_or(ListenerError::UnknownPartition)?;
        let generation = cursor.generation;
        let watermark = cursor.watermark;
        let gap = cursor.gap;
        if gap {
            return Err(ListenerError::GapRequired);
        }
        if event.generation < generation {
            return Err(ListenerError::StaleGeneration);
        }
        if event.generation > generation || event.watermark > watermark.saturating_add(1) {
            self.mark_gap(event.partition, event.generation.max(generation))?;
            return Err(ListenerError::GapRequired);
        }
        if event.watermark <= watermark {
            return Ok(ListenerAcceptance::DuplicateSuppressed);
        }
        self.cursors
            .get_mut(&event.partition)
            .expect("cursor checked above")
            .watermark = event.watermark;
        self.push_bounded(event)?;
        Ok(ListenerAcceptance::Delivered)
    }

    pub fn migrate(&mut self, partition: u32, generation: u64) -> Result<(), ListenerError> {
        let current = self
            .cursors
            .get(&partition)
            .ok_or(ListenerError::UnknownPartition)?;
        if generation <= current.generation {
            return Err(ListenerError::StaleGeneration);
        }
        self.mark_gap(partition, generation)
    }

    pub fn repair(
        &mut self,
        partition: u32,
        generation: u64,
        snapshot_watermark: u64,
    ) -> Result<(), ListenerError> {
        let cursor = self
            .cursors
            .get_mut(&partition)
            .ok_or(ListenerError::UnknownPartition)?;
        if !cursor.gap {
            return Err(ListenerError::RepairNotRequired);
        }
        if generation < cursor.generation || snapshot_watermark < cursor.watermark {
            return Err(ListenerError::RepairBehindWatermark);
        }
        cursor.generation = generation;
        cursor.watermark = snapshot_watermark;
        cursor.gap = false;
        Ok(())
    }

    pub fn drain(&mut self) -> Vec<ListenerEvent> {
        self.backlog.drain(..).collect()
    }

    pub fn backlog_len(&self) -> usize {
        self.backlog.len()
    }

    pub fn requires_repair(&self, partition: u32) -> Result<bool, ListenerError> {
        self.cursors
            .get(&partition)
            .map(|cursor| cursor.gap)
            .ok_or(ListenerError::UnknownPartition)
    }

    fn mark_gap(&mut self, partition: u32, generation: u64) -> Result<(), ListenerError> {
        let cursor = self
            .cursors
            .get_mut(&partition)
            .ok_or(ListenerError::UnknownPartition)?;
        cursor.generation = generation;
        cursor.gap = true;
        let gap = ListenerEvent {
            partition,
            generation,
            watermark: cursor.watermark,
            kind: ListenerEventKind::Gap {
                after_watermark: cursor.watermark,
            },
        };
        self.backlog.clear();
        self.backlog.push_back(gap);
        Ok(())
    }

    fn push_bounded(&mut self, event: ListenerEvent) -> Result<(), ListenerError> {
        if self.backlog.len() >= self.bounds.max_backlog {
            self.mark_gap(event.partition, event.generation)?;
            return Err(ListenerError::GapRequired);
        }
        self.backlog.push_back(event);
        Ok(())
    }
}
