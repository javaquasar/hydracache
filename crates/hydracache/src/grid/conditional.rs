use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{
    partition_for_key, ClusterEpoch, ClusterNodeId, LogicalDuration, LogicalTime, PartitionId,
};
use crate::grid::consistency_level::ConsistencyLevel;
use crate::grid::hardening::{ReplicatedValueRecord, ValueVersion};
use crate::grid::lock_session::SessionHeartbeats;
use crate::grid::session_context::SessionId;
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

/// Owner identity for a session-bound fenced lock.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LockOwner {
    /// Client/application session that owns the lock.
    pub session: SessionId,
    /// Stable endpoint/thread identity within the session.
    pub endpoint: u64,
}

impl LockOwner {
    /// Create a lock owner from a session id and endpoint.
    pub fn new(session: impl Into<SessionId>, endpoint: u64) -> Self {
        Self {
            session: session.into(),
            endpoint,
        }
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
    /// The lock lease already expired.
    LeaseExpired {
        /// Lock key.
        key: String,
        /// Current token, when another owner already acquired the lock.
        current: Option<FenceToken>,
    },
    /// The caller is not the current lock owner.
    NotOwner {
        /// Lock key.
        key: String,
        /// Current owner, when the lock is held.
        current_owner: Option<LockOwner>,
    },
    /// Reentrant acquire would exceed the configured limit.
    ReentrancyLimit {
        /// Lock key.
        key: String,
        /// Configured acquire limit.
        limit: u32,
    },
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
            Self::LeaseExpired { key, current } => write!(
                formatter,
                "fenced lock lease for key '{key}' expired; current token is {:?}",
                current.map(FenceToken::value)
            ),
            Self::NotOwner { key, current_owner } => write!(
                formatter,
                "caller is not owner of fenced lock '{key}'; current owner is {:?}",
                current_owner
            ),
            Self::ReentrancyLimit { key, limit } => write!(
                formatter,
                "fenced lock '{key}' reached reentrancy limit {limit}"
            ),
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
    /// Lock holds expired due to lease deadline or session loss.
    pub lock_lease_expired_total: u64,
    /// Lock releases/renews rejected because the caller was not the owner.
    pub lock_not_owner_rejected_total: u64,
    /// Lock leases renewed.
    pub lock_lease_renewed_total: u64,
    /// Reentrant acquires rejected by the configured limit.
    pub lock_reentrancy_limit_rejected_total: u64,
    /// Delete tombstones reclaimed after an all-replica applied-prefix proof.
    #[serde(default)]
    pub tombstone_gc_reclaimed_total: u64,
    /// Replicated records rejected because their ordering point was already reclaimed.
    #[serde(default)]
    pub tombstone_gc_stale_record_rejected_total: u64,
}

/// Aggregate retained-state snapshot for conditional values and fenced locks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalRetainedState {
    /// Versioned records, including live values and tombstones.
    pub records: usize,
    /// Records currently carrying live values.
    pub live_records: usize,
    /// Delete tombstones retained for ordering safety.
    pub tombstones: usize,
    /// Currently held fenced locks.
    pub locks: usize,
    /// Session heartbeat records retained by lock ownership.
    pub session_heartbeats: usize,
    /// Bounded per-partition tombstone GC watermarks.
    #[serde(default)]
    pub tombstone_gc_watermarks: usize,
    /// Conservative bytes in retained map keys and session identifiers.
    pub identity_bytes: usize,
}

/// Result of applying an explicitly versioned replicated conditional record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicatedRecordApply {
    /// The incoming record became the current record.
    Applied,
    /// A newer or tie-winning current record already existed.
    IgnoredStale,
    /// The record is at or below a reclaimed all-replica prefix.
    RejectedBelowGcWatermark,
}

/// One replica's ordered applied prefix for a partition in an authority epoch.
///
/// This is deliberately distinct from a quorum write acknowledgement: tombstone reclamation needs
/// proof that every earlier record in the partition stream has been applied, not merely that one
/// write reached a quorum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaAppliedPrefix {
    replica: ClusterNodeId,
    partition: PartitionId,
    version: ValueVersion,
    epoch: ClusterEpoch,
}

impl ReplicaAppliedPrefix {
    /// Record an ordered applied prefix reported by one replica.
    pub fn new(
        replica: impl Into<ClusterNodeId>,
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
    ) -> Self {
        Self {
            replica: replica.into(),
            partition,
            version,
            epoch,
        }
    }

    /// Replica that reported this prefix.
    pub fn replica(&self) -> &ClusterNodeId {
        &self.replica
    }
}

/// Ordering-safe, per-partition prefix through which tombstones may be reclaimed.
///
/// Construction requires progress from every effective replica in one authority epoch. The safe
/// version is the minimum applied version, so a lagging or missing replica prevents advancement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TombstoneGcWatermark {
    partition: PartitionId,
    version: ValueVersion,
    epoch: ClusterEpoch,
    replica_count: usize,
}

impl TombstoneGcWatermark {
    /// Build a safe prefix from the exact effective replica set and its ordered applied progress.
    pub fn from_all_replicas(
        partition: PartitionId,
        epoch: ClusterEpoch,
        replicas: impl IntoIterator<Item = ClusterNodeId>,
        progress: impl IntoIterator<Item = ReplicaAppliedPrefix>,
    ) -> Result<Self, TombstoneGcError> {
        let replicas = replicas.into_iter().collect::<BTreeSet<_>>();
        if replicas.is_empty() {
            return Err(TombstoneGcError::EmptyReplicaSet);
        }

        let mut progress_by_replica = BTreeMap::new();
        for applied_prefix in progress {
            let replica = applied_prefix.replica.clone();
            if !replicas.contains(&replica) {
                return Err(TombstoneGcError::UnexpectedReplica { replica });
            }
            if progress_by_replica
                .insert(replica.clone(), applied_prefix)
                .is_some()
            {
                return Err(TombstoneGcError::DuplicateReplica { replica });
            }
        }

        let mut safe_version = ValueVersion::MAX;
        for replica in &replicas {
            let applied_prefix = progress_by_replica.get(replica).ok_or_else(|| {
                TombstoneGcError::MissingReplica {
                    replica: replica.clone(),
                }
            })?;
            if applied_prefix.partition != partition {
                return Err(TombstoneGcError::ProgressPartitionMismatch {
                    replica: replica.clone(),
                    expected: partition,
                    actual: applied_prefix.partition,
                });
            }
            if applied_prefix.epoch != epoch {
                return Err(TombstoneGcError::ProgressEpochMismatch {
                    replica: replica.clone(),
                    expected: epoch,
                    actual: applied_prefix.epoch,
                });
            }
            safe_version = safe_version.min(applied_prefix.version);
        }

        Ok(Self {
            partition,
            version: safe_version,
            epoch,
            replica_count: replicas.len(),
        })
    }

    /// Partition covered by this prefix.
    pub const fn partition(self) -> PartitionId {
        self.partition
    }

    /// Minimum version applied by every effective replica.
    pub const fn version(self) -> ValueVersion {
        self.version
    }

    /// Authority epoch shared by every acknowledgement.
    pub const fn epoch(self) -> ClusterEpoch {
        self.epoch
    }

    /// Number of effective replicas included in the proof.
    pub const fn replica_count(self) -> usize {
        self.replica_count
    }
}

/// Fail-closed errors for conditional tombstone watermark construction and application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TombstoneGcError {
    /// An empty topology cannot prove that stale copies are absent.
    EmptyReplicaSet,
    /// One effective replica has no applied-prefix acknowledgement.
    MissingReplica { replica: ClusterNodeId },
    /// Progress was supplied for a node outside the effective replica set.
    UnexpectedReplica { replica: ClusterNodeId },
    /// More than one progress record was supplied for a replica.
    DuplicateReplica { replica: ClusterNodeId },
    /// A replica reported progress for another partition.
    ProgressPartitionMismatch {
        replica: ClusterNodeId,
        expected: PartitionId,
        actual: PartitionId,
    },
    /// A replica reported progress from another authority epoch.
    ProgressEpochMismatch {
        replica: ClusterNodeId,
        expected: ClusterEpoch,
        actual: ClusterEpoch,
    },
    /// The proof belongs to another authority epoch than the conditional store.
    StoreEpochMismatch {
        expected: ClusterEpoch,
        actual: ClusterEpoch,
    },
    /// The proof names a partition outside the store's configured partition range.
    PartitionOutOfRange {
        partition: PartitionId,
        partition_count: u32,
    },
    /// A watermark cannot move backwards.
    WatermarkRegression {
        partition: PartitionId,
        current: ValueVersion,
        proposed: ValueVersion,
    },
    /// The record's declared partition does not match its key.
    RecordPartitionMismatch {
        expected: PartitionId,
        actual: PartitionId,
    },
}

impl fmt::Display for TombstoneGcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyReplicaSet => {
                formatter.write_str("tombstone GC requires at least one replica")
            }
            Self::MissingReplica { replica } => {
                write!(
                    formatter,
                    "missing applied-prefix progress for replica {replica}"
                )
            }
            Self::UnexpectedReplica { replica } => {
                write!(formatter, "unexpected applied-prefix replica {replica}")
            }
            Self::DuplicateReplica { replica } => {
                write!(
                    formatter,
                    "duplicate applied-prefix progress for replica {replica}"
                )
            }
            Self::ProgressPartitionMismatch {
                replica,
                expected,
                actual,
            } => write!(
                formatter,
                "replica {replica} acknowledged partition {}, expected {}",
                actual.value(),
                expected.value()
            ),
            Self::ProgressEpochMismatch {
                replica,
                expected,
                actual,
            } => write!(
                formatter,
                "replica {replica} acknowledged epoch {}, expected {}",
                actual.value(),
                expected.value()
            ),
            Self::StoreEpochMismatch { expected, actual } => write!(
                formatter,
                "tombstone GC proof epoch {} does not match store epoch {}",
                actual.value(),
                expected.value()
            ),
            Self::PartitionOutOfRange {
                partition,
                partition_count,
            } => write!(
                formatter,
                "tombstone GC partition {} is outside configured count {partition_count}",
                partition.value()
            ),
            Self::WatermarkRegression {
                partition,
                current,
                proposed,
            } => write!(
                formatter,
                "tombstone GC partition {} cannot regress from {current} to {proposed}",
                partition.value()
            ),
            Self::RecordPartitionMismatch { expected, actual } => write!(
                formatter,
                "replicated record partition {} does not match key partition {}",
                actual.value(),
                expected.value()
            ),
        }
    }
}

impl std::error::Error for TombstoneGcError {}

/// Current state of one held fenced lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockHold {
    /// Current owner.
    pub owner: LockOwner,
    /// Current fencing token.
    pub fence: FenceToken,
    /// Reentrant acquire count.
    pub holds: u32,
    /// Logical lease deadline.
    pub lease_deadline: LogicalTime,
}

impl LockHold {
    fn expired_at(&self, now: LogicalTime) -> bool {
        self.lease_deadline <= now
    }
}

/// Deterministic single-key conditional store backed by versioned records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleKeyConditionalStore {
    records: BTreeMap<String, ReplicatedValueRecord>,
    #[serde(default)]
    tombstone_gc_watermarks: BTreeMap<PartitionId, TombstoneGcWatermark>,
    locks: BTreeMap<String, LockHold>,
    session_heartbeats: SessionHeartbeats,
    next_version: ValueVersion,
    next_fence: u64,
    epoch: ClusterEpoch,
    partition_count: u32,
    metrics: ConditionalMetrics,
    lock_acquire_limit: Option<u32>,
}

impl SingleKeyConditionalStore {
    /// Create an empty conditional store.
    pub fn new(epoch: ClusterEpoch, partition_count: u32) -> Self {
        Self {
            records: BTreeMap::new(),
            tombstone_gc_watermarks: BTreeMap::new(),
            locks: BTreeMap::new(),
            session_heartbeats: SessionHeartbeats::default(),
            next_version: 1,
            next_fence: 1,
            epoch,
            partition_count: partition_count.max(1),
            metrics: ConditionalMetrics::default(),
            lock_acquire_limit: None,
        }
    }

    /// Configure the reentrant acquire limit. `None` keeps Hazelcast-style unbounded reentrancy.
    pub fn with_lock_acquire_limit(mut self, limit: Option<u32>) -> Self {
        self.lock_acquire_limit = limit;
        self
    }

    /// Return current metrics.
    pub fn metrics(&self) -> ConditionalMetrics {
        self.metrics
    }

    /// Return aggregate retained owner counts without exposing keys or sessions.
    pub fn retained_state(&self) -> ConditionalRetainedState {
        let live_records = self
            .records
            .values()
            .filter(|record| matches!(&record.state, ReplicatedSlot::Value { .. }))
            .count();
        ConditionalRetainedState {
            records: self.records.len(),
            live_records,
            tombstones: self.records.len().saturating_sub(live_records),
            locks: self.locks.len(),
            session_heartbeats: self.session_heartbeats.len(),
            tombstone_gc_watermarks: self.tombstone_gc_watermarks.len(),
            identity_bytes: self.records.keys().map(String::len).sum::<usize>()
                + self.locks.keys().map(String::len).sum::<usize>()
                + self.session_heartbeats.string_bytes(),
        }
    }

    /// Clear all conditional values, tombstones, locks, and session records.
    ///
    /// This is intended only for an explicitly enabled administrative
    /// diagnostic reset. Monotonic versions and fences advance instead of
    /// restarting, so a token issued before the reset cannot become valid
    /// again after new state is created.
    pub fn clear_for_diagnostic_reset(&mut self) -> ConditionalRetainedState {
        let before = self.retained_state();
        self.records.clear();
        self.tombstone_gc_watermarks.clear();
        self.locks.clear();
        self.session_heartbeats.clear();
        self.next_version = self.next_version.saturating_add(1);
        self.next_fence = self.next_fence.saturating_add(1);
        before
    }

    /// Return the record for a key.
    pub fn record(&self, key: &str) -> Option<&ReplicatedValueRecord> {
        self.records.get(key)
    }

    /// Return the current live value for a key, treating tombstones as absent.
    pub fn current_value(&self, key: &str) -> Option<Vec<u8>> {
        self.records.get(key).and_then(current_bytes)
    }

    /// Return the current lock hold for a key.
    pub fn lock_hold(&self, key: &str) -> Option<&LockHold> {
        self.locks.get(key)
    }

    /// Return whether a lock is currently held.
    pub fn is_locked(&self, key: &str) -> bool {
        self.locks.contains_key(key)
    }

    /// Return whether a lock is currently held by `owner`.
    pub fn is_locked_by(&self, key: &str, owner: &LockOwner) -> bool {
        self.locks.get(key).is_some_and(|hold| &hold.owner == owner)
    }

    /// Return the current fence token for a held lock.
    pub fn current_fence(&self, key: &str) -> Option<FenceToken> {
        self.locks.get(key).map(|hold| hold.fence)
    }

    /// Return the current tombstone GC prefix for a partition.
    pub fn tombstone_gc_watermark(&self, partition: PartitionId) -> Option<TombstoneGcWatermark> {
        self.tombstone_gc_watermarks.get(&partition).copied()
    }

    /// Advance an all-replica applied prefix and reclaim covered tombstones.
    ///
    /// The watermark must come from the store's current authority epoch and cannot regress.
    /// Future replicated records at or below the retained prefix are rejected, preventing a stale
    /// value from resurrecting a key after its tombstone has been physically removed.
    pub fn advance_tombstone_gc_watermark(
        &mut self,
        watermark: TombstoneGcWatermark,
    ) -> Result<usize, TombstoneGcError> {
        if watermark.epoch != self.epoch {
            return Err(TombstoneGcError::StoreEpochMismatch {
                expected: self.epoch,
                actual: watermark.epoch,
            });
        }
        if watermark.partition.value() >= self.partition_count {
            return Err(TombstoneGcError::PartitionOutOfRange {
                partition: watermark.partition,
                partition_count: self.partition_count,
            });
        }
        if let Some(current) = self.tombstone_gc_watermarks.get(&watermark.partition) {
            if watermark.version < current.version {
                return Err(TombstoneGcError::WatermarkRegression {
                    partition: watermark.partition,
                    current: current.version,
                    proposed: watermark.version,
                });
            }
        }

        self.next_version = self.next_version.max(watermark.version.saturating_add(1));
        self.tombstone_gc_watermarks
            .insert(watermark.partition, watermark);
        let before = self.records.len();
        self.records.retain(|_, record| {
            record.partition != watermark.partition
                || !record.is_tombstone()
                || record_order(record) > (watermark.version, watermark.epoch)
        });
        let reclaimed = before.saturating_sub(self.records.len());
        self.metrics.tombstone_gc_reclaimed_total = self
            .metrics
            .tombstone_gc_reclaimed_total
            .saturating_add(u64::try_from(reclaimed).unwrap_or(u64::MAX));
        Ok(reclaimed)
    }

    /// Apply a record received from an ordered replication or repair stream.
    pub fn apply_replicated_record(
        &mut self,
        key: &str,
        record: ReplicatedValueRecord,
    ) -> Result<ReplicatedRecordApply, TombstoneGcError> {
        let expected_partition = partition_for_key(key, self.partition_count);
        if record.partition != expected_partition {
            return Err(TombstoneGcError::RecordPartitionMismatch {
                expected: expected_partition,
                actual: record.partition,
            });
        }
        if self
            .tombstone_gc_watermarks
            .get(&record.partition)
            .is_some_and(|watermark| record_order(&record) <= (watermark.version, watermark.epoch))
        {
            self.metrics.tombstone_gc_stale_record_rejected_total = self
                .metrics
                .tombstone_gc_stale_record_rejected_total
                .saturating_add(1);
            return Ok(ReplicatedRecordApply::RejectedBelowGcWatermark);
        }

        self.next_version = self.next_version.max(record.version.saturating_add(1));
        match self.records.get(key) {
            Some(current) => {
                let merged = current.clone().merge(record);
                if &merged == current {
                    Ok(ReplicatedRecordApply::IgnoredStale)
                } else {
                    self.records.insert(key.to_owned(), merged);
                    Ok(ReplicatedRecordApply::Applied)
                }
            }
            None => {
                self.records.insert(key.to_owned(), record);
                Ok(ReplicatedRecordApply::Applied)
            }
        }
    }

    /// Apply a tombstone at an explicit version, preserving A5 delete semantics.
    pub fn apply_tombstone(&mut self, key: &str, version: ValueVersion) {
        let record = ReplicatedValueRecord::tombstone(
            partition_for_key(key, self.partition_count),
            version,
            self.epoch,
            None,
        );
        let _ = self.apply_replicated_record(key, record);
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

    /// Replace one key only when a live value is currently present.
    pub fn replace_if_present(
        &mut self,
        key: &str,
        new_value: Vec<u8>,
        level: ConsistencyLevel,
    ) -> Result<CasResult, ConditionalError> {
        require_linearizable_level(level)?;
        let current = self.records.get(key).and_then(current_bytes);
        let Some(expected) = current else {
            self.metrics.cas_mismatch_total = self.metrics.cas_mismatch_total.saturating_add(1);
            return Ok(CasResult::Mismatch { current: None });
        };
        self.compare_and_set(key, Some(&expected), new_value, level)
    }

    /// Remove one key by writing a tombstone only when the live value matches.
    pub fn remove_if_value(
        &mut self,
        key: &str,
        expected: &[u8],
        level: ConsistencyLevel,
    ) -> Result<CasResult, ConditionalError> {
        require_linearizable_level(level)?;
        let current = self.records.get(key).and_then(current_bytes);
        if current.as_deref() != Some(expected) {
            self.metrics.cas_mismatch_total = self.metrics.cas_mismatch_total.saturating_add(1);
            return Ok(CasResult::Mismatch { current });
        }

        let version = self.next_version;
        self.apply_tombstone(key, version);
        self.metrics.cas_applied_total = self.metrics.cas_applied_total.saturating_add(1);
        Ok(CasResult::Applied {
            new_version: version,
        })
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
        owner: LockOwner,
        lease: LogicalDuration,
        now: LogicalTime,
    ) -> Result<Option<FenceToken>, ConditionalError> {
        require_linearizable_level(level)?;
        let replaced_session = self
            .locks
            .get(key)
            .filter(|hold| hold.expired_at(now))
            .map(|hold| hold.owner.session.clone());
        if self
            .locks
            .get(key)
            .is_some_and(|hold| !hold.expired_at(now))
        {
            if self.is_locked_by(key, &owner) {
                let limit = self.lock_acquire_limit;
                let hold = self.locks.get_mut(key).expect("checked lock hold");
                let next_holds = hold.holds.saturating_add(1);
                if limit.is_some_and(|limit| next_holds > limit) {
                    self.metrics.lock_reentrancy_limit_rejected_total = self
                        .metrics
                        .lock_reentrancy_limit_rejected_total
                        .saturating_add(1);
                    return Err(ConditionalError::ReentrancyLimit {
                        key: key.to_owned(),
                        limit: limit.unwrap_or(u32::MAX),
                    });
                }
                hold.holds = next_holds;
                hold.lease_deadline = hold.lease_deadline.max(now.saturating_add(lease));
                self.session_heartbeats.record(owner.session.clone(), now);
                self.metrics.lock_acquired_total =
                    self.metrics.lock_acquired_total.saturating_add(1);
                return Ok(Some(hold.fence));
            } else {
                return Ok(None);
            }
        }
        if self.locks.contains_key(key) {
            self.metrics.lock_lease_expired_total =
                self.metrics.lock_lease_expired_total.saturating_add(1);
        }
        let token = FenceToken::new(self.next_fence);
        self.next_fence = self.next_fence.saturating_add(1);
        let new_session = owner.session.clone();
        self.session_heartbeats.record(owner.session.clone(), now);
        self.locks.insert(
            key.to_owned(),
            LockHold {
                owner,
                fence: token,
                holds: 1,
                lease_deadline: now.saturating_add(lease),
            },
        );
        if let Some(replaced_session) = replaced_session {
            if replaced_session != new_session {
                self.remove_heartbeat_if_unused(&replaced_session);
            }
        }
        self.metrics.lock_acquired_total = self.metrics.lock_acquired_total.saturating_add(1);
        Ok(Some(token))
    }

    /// Force lock ownership to advance, simulating lease expiry / failover in deterministic tests.
    pub fn force_acquire_lock(
        &mut self,
        key: &str,
        level: ConsistencyLevel,
        owner: LockOwner,
        lease: LogicalDuration,
        now: LogicalTime,
    ) -> Result<FenceToken, ConditionalError> {
        require_linearizable_level(level)?;
        let replaced_session = self.locks.get(key).map(|hold| hold.owner.session.clone());
        let token = FenceToken::new(self.next_fence);
        self.next_fence = self.next_fence.saturating_add(1);
        let new_session = owner.session.clone();
        self.session_heartbeats.record(owner.session.clone(), now);
        self.locks.insert(
            key.to_owned(),
            LockHold {
                owner,
                fence: token,
                holds: 1,
                lease_deadline: now.saturating_add(lease),
            },
        );
        if let Some(replaced_session) = replaced_session {
            if replaced_session != new_session {
                self.remove_heartbeat_if_unused(&replaced_session);
            }
        }
        self.metrics.lock_acquired_total = self.metrics.lock_acquired_total.saturating_add(1);
        Ok(token)
    }

    /// Renew a lock lease for the current owner.
    pub fn renew_lease(
        &mut self,
        key: &str,
        owner: &LockOwner,
        token: FenceToken,
        new_deadline: LogicalTime,
    ) -> Result<(), ConditionalError> {
        let Some(hold) = self.locks.get_mut(key) else {
            self.metrics.lock_stale_token_rejected_total = self
                .metrics
                .lock_stale_token_rejected_total
                .saturating_add(1);
            return Err(ConditionalError::LeaseExpired {
                key: key.to_owned(),
                current: None,
            });
        };
        if &hold.owner != owner {
            self.metrics.lock_not_owner_rejected_total =
                self.metrics.lock_not_owner_rejected_total.saturating_add(1);
            return Err(ConditionalError::NotOwner {
                key: key.to_owned(),
                current_owner: Some(hold.owner.clone()),
            });
        }
        if hold.fence != token {
            self.metrics.lock_stale_token_rejected_total = self
                .metrics
                .lock_stale_token_rejected_total
                .saturating_add(1);
            return Err(ConditionalError::StaleFenceToken {
                current: Some(hold.fence),
                presented: token,
            });
        }
        hold.lease_deadline = new_deadline;
        self.session_heartbeats
            .record(owner.session.clone(), new_deadline);
        self.metrics.lock_lease_renewed_total =
            self.metrics.lock_lease_renewed_total.saturating_add(1);
        Ok(())
    }

    /// Record a logical session heartbeat.
    pub fn record_session_heartbeat(&mut self, session: SessionId, now: LogicalTime) {
        self.session_heartbeats.record(session, now);
    }

    /// Expire all lock holds with lease deadlines at or before `now`.
    pub fn expire_due(&mut self, now: LogicalTime) -> usize {
        let expired = self
            .locks
            .iter()
            .filter(|(_, hold)| hold.expired_at(now))
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        self.expire_keys(expired)
    }

    /// Expire all lock holds owned by sessions that stopped heartbeating.
    pub fn expire_lost_sessions(
        &mut self,
        now: LogicalTime,
        max_silence: LogicalDuration,
    ) -> usize {
        let lost = self.session_heartbeats.lost_sessions(now, max_silence);
        let expired = self
            .locks
            .iter()
            .filter(|(_, hold)| lost.contains(&hold.owner.session))
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        let expired_count = self.expire_keys(expired);
        for session in lost {
            self.session_heartbeats.remove(&session);
        }
        expired_count
    }

    /// Release a fenced lock when the token is current.
    pub fn release_lock(
        &mut self,
        key: &str,
        owner: &LockOwner,
        token: FenceToken,
    ) -> Result<(), ConditionalError> {
        let Some(hold) = self.locks.get(key) else {
            self.metrics.lock_stale_token_rejected_total = self
                .metrics
                .lock_stale_token_rejected_total
                .saturating_add(1);
            return Err(ConditionalError::StaleFenceToken {
                current: None,
                presented: token,
            });
        };
        if &hold.owner != owner {
            self.metrics.lock_not_owner_rejected_total =
                self.metrics.lock_not_owner_rejected_total.saturating_add(1);
            return Err(ConditionalError::NotOwner {
                key: key.to_owned(),
                current_owner: Some(hold.owner.clone()),
            });
        }
        if hold.fence != token {
            self.metrics.lock_stale_token_rejected_total = self
                .metrics
                .lock_stale_token_rejected_total
                .saturating_add(1);
            return Err(ConditionalError::StaleFenceToken {
                current: Some(hold.fence),
                presented: token,
            });
        }
        let remove = {
            let hold = self.locks.get_mut(key).expect("checked lock hold");
            if hold.holds > 1 {
                hold.holds = hold.holds.saturating_sub(1);
                false
            } else {
                true
            }
        };
        if remove {
            let released_session = self
                .locks
                .remove(key)
                .map(|hold| hold.owner.session)
                .expect("checked lock hold");
            self.remove_heartbeat_if_unused(&released_session);
        }
        Ok(())
    }

    /// Privileged release that advances the fencing sequence without requiring ownership.
    pub fn force_unlock(&mut self, key: &str) -> Option<FenceToken> {
        if let Some(removed) = self.locks.remove(key) {
            self.remove_heartbeat_if_unused(&removed.owner.session);
            let next = FenceToken::new(self.next_fence);
            self.next_fence = self.next_fence.saturating_add(1);
            Some(next)
        } else {
            None
        }
    }

    /// Validate that a token is still current.
    pub fn validate_fence_token(
        &mut self,
        key: &str,
        token: FenceToken,
    ) -> Result<(), ConditionalError> {
        let current = self.locks.get(key).map(|hold| hold.fence);
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

    fn expire_keys(&mut self, keys: Vec<String>) -> usize {
        let mut expired = 0usize;
        let mut released_sessions = Vec::new();
        for key in keys {
            if let Some(removed) = self.locks.remove(&key) {
                released_sessions.push(removed.owner.session);
                self.next_fence = self.next_fence.saturating_add(1);
                self.metrics.lock_lease_expired_total =
                    self.metrics.lock_lease_expired_total.saturating_add(1);
                expired = expired.saturating_add(1);
            }
        }
        for session in released_sessions {
            self.remove_heartbeat_if_unused(&session);
        }
        expired
    }

    fn remove_heartbeat_if_unused(&mut self, session: &SessionId) {
        if !self
            .locks
            .values()
            .any(|hold| &hold.owner.session == session)
        {
            self.session_heartbeats.remove(session);
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

fn record_order(record: &ReplicatedValueRecord) -> (ValueVersion, ClusterEpoch) {
    (record.version, record.epoch)
}
