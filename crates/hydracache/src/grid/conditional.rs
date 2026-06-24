use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{partition_for_key, ClusterEpoch};
use crate::grid::consistency_level::ConsistencyLevel;
use crate::grid::hardening::{ReplicatedValueRecord, ValueVersion};
use crate::grid::ReplicatedSlot;

/// Result of a single-key compare-and-set operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CasResult {
    /// The condition matched and a new version was written.
    Applied {
        /// New monotonic value version.
        new_version: ValueVersion,
    },
    /// The condition did not match.
    Mismatch {
        /// Current live value bytes, if present.
        current: Option<Vec<u8>>,
    },
}

/// Monotonic fencing token returned by a lock acquisition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FenceToken(u64);

impl FenceToken {
    /// Create a token from a raw value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw token value.
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Errors returned by conditional writes and fenced locks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionalError {
    /// Requested consistency level cannot provide a linearizable single-key decision.
    WeakConsistency { level: ConsistencyLevel },
    /// Conditional operation tried to span more than one key.
    MultiKeyRejected { key_count: usize },
    /// A stale fencing token was used.
    StaleFenceToken {
        /// Current lock token.
        current: Option<FenceToken>,
        /// Token presented by the caller.
        presented: FenceToken,
    },
    /// Lock is not held.
    LockNotHeld,
}

impl fmt::Display for ConditionalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WeakConsistency { level } => write!(
                formatter,
                "conditional writes require Quorum/EachQuorum/All, got {:?}",
                level
            ),
            Self::MultiKeyRejected { key_count } => write!(
                formatter,
                "conditional write is single-key only, got {} keys",
                key_count
            ),
            Self::StaleFenceToken { current, presented } => write!(
                formatter,
                "stale fence token {}, current token is {:?}",
                presented.value(),
                current.map(FenceToken::value)
            ),
            Self::LockNotHeld => formatter.write_str("fenced lock is not held"),
        }
    }
}

impl std::error::Error for ConditionalError {}

/// Conditional-write metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalMetrics {
    /// Applied compare-and-set operations.
    pub cas_applied_total: u64,
    /// Compare-and-set mismatches.
    pub cas_mismatch_total: u64,
    /// Lock acquisitions.
    pub lock_acquired_total: u64,
    /// Stale lock tokens rejected.
    pub lock_stale_token_rejected_total: u64,
}

/// Deterministic single-key conditional store backed by versioned records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleKeyConditionalStore {
    records: BTreeMap<String, ReplicatedValueRecord>,
    locks: BTreeMap<String, FenceToken>,
    next_version: ValueVersion,
    next_fence: u64,
    epoch: ClusterEpoch,
    partition_count: u32,
    metrics: ConditionalMetrics,
}

impl SingleKeyConditionalStore {
    /// Create an empty conditional store.
    pub fn new(epoch: ClusterEpoch, partition_count: u32) -> Self {
        Self {
            records: BTreeMap::new(),
            locks: BTreeMap::new(),
            next_version: 1,
            next_fence: 1,
            epoch,
            partition_count: partition_count.max(1),
            metrics: ConditionalMetrics::default(),
        }
    }

    /// Return current metrics.
    pub fn metrics(&self) -> ConditionalMetrics {
        self.metrics
    }

    /// Return the record for a key.
    pub fn record(&self, key: &str) -> Option<&ReplicatedValueRecord> {
        self.records.get(key)
    }

    /// Apply a tombstone at an explicit version, preserving A5 delete semantics.
    pub fn apply_tombstone(&mut self, key: &str, version: ValueVersion) {
        let record = ReplicatedValueRecord::tombstone(
            partition_for_key(key, self.partition_count),
            version,
            self.epoch,
            None,
        );
        self.next_version = self.next_version.max(version.saturating_add(1));
        self.records.insert(key.to_owned(), record);
    }

    /// Compare and set one key at a linearizable-capable level.
    pub fn compare_and_set(
        &mut self,
        key: &str,
        expected: Option<&[u8]>,
        new_value: Vec<u8>,
        level: ConsistencyLevel,
    ) -> Result<CasResult, ConditionalError> {
        require_linearizable_level(level)?;
        let current = self.records.get(key).and_then(current_bytes);
        if current.as_deref() != expected {
            self.metrics.cas_mismatch_total = self.metrics.cas_mismatch_total.saturating_add(1);
            return Ok(CasResult::Mismatch { current });
        }

        let version = self.next_version;
        self.next_version = self.next_version.saturating_add(1);
        let record = ReplicatedValueRecord::value(
            partition_for_key(key, self.partition_count),
            version,
            self.epoch,
            new_value,
        );
        self.records.insert(key.to_owned(), record);
        self.metrics.cas_applied_total = self.metrics.cas_applied_total.saturating_add(1);
        Ok(CasResult::Applied {
            new_version: version,
        })
    }

    /// Put one key only if it is absent or tombstoned.
    pub fn put_if_absent(
        &mut self,
        key: &str,
        value: Vec<u8>,
        level: ConsistencyLevel,
    ) -> Result<CasResult, ConditionalError> {
        self.compare_and_set(key, None, value, level)
    }

    /// Reject multi-key conditional attempts loudly.
    pub fn reject_multi_key<'a, I>(keys: I) -> Result<(), ConditionalError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let count = keys.into_iter().count();
        if count == 1 {
            Ok(())
        } else {
            Err(ConditionalError::MultiKeyRejected { key_count: count })
        }
    }

    /// Try to acquire a fenced lock.
    pub fn try_acquire_lock(
        &mut self,
        key: &str,
        level: ConsistencyLevel,
    ) -> Result<Option<FenceToken>, ConditionalError> {
        require_linearizable_level(level)?;
        if self.locks.contains_key(key) {
            return Ok(None);
        }
        let token = FenceToken::new(self.next_fence);
        self.next_fence = self.next_fence.saturating_add(1);
        self.locks.insert(key.to_owned(), token);
        self.metrics.lock_acquired_total = self.metrics.lock_acquired_total.saturating_add(1);
        Ok(Some(token))
    }

    /// Force lock ownership to advance, simulating lease expiry / failover in deterministic tests.
    pub fn force_acquire_lock(
        &mut self,
        key: &str,
        level: ConsistencyLevel,
    ) -> Result<FenceToken, ConditionalError> {
        require_linearizable_level(level)?;
        let token = FenceToken::new(self.next_fence);
        self.next_fence = self.next_fence.saturating_add(1);
        self.locks.insert(key.to_owned(), token);
        self.metrics.lock_acquired_total = self.metrics.lock_acquired_total.saturating_add(1);
        Ok(token)
    }

    /// Release a fenced lock when the token is current.
    pub fn release_lock(&mut self, key: &str, token: FenceToken) -> Result<(), ConditionalError> {
        let Some(current) = self.locks.get(key).copied() else {
            return Err(ConditionalError::LockNotHeld);
        };
        if current != token {
            self.metrics.lock_stale_token_rejected_total = self
                .metrics
                .lock_stale_token_rejected_total
                .saturating_add(1);
            return Err(ConditionalError::StaleFenceToken {
                current: Some(current),
                presented: token,
            });
        }
        self.locks.remove(key);
        Ok(())
    }

    /// Validate that a token is still current.
    pub fn validate_fence_token(
        &mut self,
        key: &str,
        token: FenceToken,
    ) -> Result<(), ConditionalError> {
        let current = self.locks.get(key).copied();
        if current == Some(token) {
            Ok(())
        } else {
            self.metrics.lock_stale_token_rejected_total = self
                .metrics
                .lock_stale_token_rejected_total
                .saturating_add(1);
            Err(ConditionalError::StaleFenceToken {
                current,
                presented: token,
            })
        }
    }
}

fn require_linearizable_level(level: ConsistencyLevel) -> Result<(), ConditionalError> {
    if level.allows_single_key_linearizable() {
        Ok(())
    } else {
        Err(ConditionalError::WeakConsistency { level })
    }
}

fn current_bytes(record: &ReplicatedValueRecord) -> Option<Vec<u8>> {
    match &record.state {
        ReplicatedSlot::Value { value, .. } => Some(value.clone()),
        ReplicatedSlot::Tombstone { .. } => None,
    }
}
