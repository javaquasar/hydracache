//! Bounded, namespace-generation-aware retained outcome lifecycle.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedOutcome {
    pub namespace_generation: u64,
    pub digest: u64,
    pub outcome: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupAcceptance {
    Recorded,
    Replayed(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupError {
    InvalidBound,
    InvalidRecord,
    Capacity,
    IdentityConflict,
    SafeWatermarkRegression,
    EvictionBeyondSafeWatermark,
}

impl fmt::Display for DedupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DedupError {}

#[derive(Debug, Clone)]
pub struct DedupLifecycle {
    max_entries: usize,
    safe_watermark: u64,
    entries: BTreeMap<u64, RetainedOutcome>,
}

impl DedupLifecycle {
    pub fn new(max_entries: usize) -> Result<Self, DedupError> {
        if max_entries == 0 {
            return Err(DedupError::InvalidBound);
        }
        Ok(Self {
            max_entries,
            safe_watermark: 0,
            entries: BTreeMap::new(),
        })
    }

    pub fn record(
        &mut self,
        identity: u64,
        retained: RetainedOutcome,
    ) -> Result<DedupAcceptance, DedupError> {
        if identity == 0 || retained.namespace_generation == 0 || retained.sequence == 0 {
            return Err(DedupError::InvalidRecord);
        }
        if let Some(existing) = self.entries.get(&identity) {
            return if existing.namespace_generation == retained.namespace_generation
                && existing.digest == retained.digest
            {
                Ok(DedupAcceptance::Replayed(existing.outcome))
            } else {
                Err(DedupError::IdentityConflict)
            };
        }
        if self.entries.len() >= self.max_entries {
            return Err(DedupError::Capacity);
        }
        self.entries.insert(identity, retained);
        Ok(DedupAcceptance::Recorded)
    }

    pub fn advance_safe_watermark(&mut self, sequence: u64) -> Result<(), DedupError> {
        if sequence < self.safe_watermark {
            return Err(DedupError::SafeWatermarkRegression);
        }
        self.safe_watermark = sequence;
        Ok(())
    }

    pub fn evict_through(&mut self, sequence: u64) -> Result<usize, DedupError> {
        if sequence > self.safe_watermark {
            return Err(DedupError::EvictionBeyondSafeWatermark);
        }
        let before = self.entries.len();
        self.entries
            .retain(|_, outcome| outcome.sequence > sequence);
        Ok(before - self.entries.len())
    }

    pub fn export(&self) -> Vec<(u64, RetainedOutcome)> {
        self.entries
            .iter()
            .map(|(identity, outcome)| (*identity, outcome.clone()))
            .collect()
    }

    pub fn import(
        &mut self,
        entries: impl IntoIterator<Item = (u64, RetainedOutcome)>,
    ) -> Result<(), DedupError> {
        for (identity, outcome) in entries {
            self.record(identity, outcome)?;
        }
        Ok(())
    }

    pub fn namespace_reclaimable(&self, generation: u64) -> bool {
        !self
            .entries
            .values()
            .any(|entry| entry.namespace_generation == generation)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
