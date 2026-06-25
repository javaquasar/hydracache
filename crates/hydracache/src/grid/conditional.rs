use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{partition_for_key, ClusterEpoch, LogicalDuration, LogicalTime};
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
}

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
        let token = FenceToken::new(self.next_fence);
        self.next_fence = self.next_fence.saturating_add(1);
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
        self.expire_keys(expired)
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
            self.locks.remove(key);
        }
        Ok(())
    }

    /// Privileged release that advances the fencing sequence without requiring ownership.
    pub fn force_unlock(&mut self, key: &str) -> Option<FenceToken> {
        if self.locks.remove(key).is_some() {
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
        for key in keys {
            if self.locks.remove(&key).is_some() {
                self.next_fence = self.next_fence.saturating_add(1);
                self.metrics.lock_lease_expired_total =
                    self.metrics.lock_lease_expired_total.saturating_add(1);
                expired = expired.saturating_add(1);
            }
        }
        expired
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
