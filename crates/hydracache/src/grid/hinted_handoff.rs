use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, ClusterNodeId, PartitionId};
use crate::grid::hardening::{ReplicatedValueRecord, SealedBytes, ValueVersion};

/// Missed write captured for a replica that was temporarily unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hint {
    /// Replica that should receive this write when it returns.
    pub target: ClusterNodeId,
    /// Cache key covered by the missed write.
    pub key: String,
    /// Owning partition.
    pub partition: PartitionId,
    /// Monotonic value version.
    pub version: ValueVersion,
    /// Authority epoch that produced this write.
    pub epoch: ClusterEpoch,
    /// Sealed value bytes.
    pub sealed: SealedBytes,
    /// Logical creation time in milliseconds.
    pub created_at_millis: u64,
}

impl Hint {
    /// Create a hint.
    pub fn new(
        target: impl Into<ClusterNodeId>,
        key: impl Into<String>,
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
        sealed: impl Into<SealedBytes>,
        created_at_millis: u64,
    ) -> Self {
        Self {
            target: target.into(),
            key: key.into(),
            partition,
            version,
            epoch,
            sealed: sealed.into(),
            created_at_millis,
        }
    }

    /// Approximate storage bytes charged against the hint budget.
    pub fn approx_bytes(&self) -> u64 {
        self.key.len().saturating_add(self.sealed.len()).max(1) as u64
    }

    fn is_expired(&self, now_millis: u64, budget: HintBudget) -> bool {
        let max_age_millis = budget.max_age.as_millis().min(u128::from(u64::MAX)) as u64;
        now_millis.saturating_sub(self.created_at_millis) > max_age_millis
    }

    fn to_record(&self) -> ReplicatedValueRecord {
        ReplicatedValueRecord::value(
            self.partition,
            self.version,
            self.epoch,
            self.sealed.clone(),
        )
    }
}

/// Bounded hint retention policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HintBudget {
    /// Maximum retained hint count.
    pub max_hints: usize,
    /// Maximum retained hint bytes.
    pub max_bytes: u64,
    /// Maximum age before a hint falls back to repair.
    pub max_age: Duration,
}

impl HintBudget {
    /// Create a normalized non-zero budget.
    pub fn new(max_hints: usize, max_bytes: u64, max_age: Duration) -> Self {
        Self {
            max_hints: max_hints.max(1),
            max_bytes: max_bytes.max(1),
            max_age: if max_age.is_zero() {
                Duration::from_millis(1)
            } else {
                max_age
            },
        }
    }
}

/// Outcome of admitting or replaying a hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HintOutcome {
    /// Hint was retained for later replay.
    Stored,
    /// Hint was replayed to the target.
    Replayed,
    /// Hint was dropped because the store was over budget.
    DroppedOverBudget,
    /// Hint was dropped because it exceeded the max age window.
    DroppedExpired,
    /// The missing replica was required by the requested consistency level.
    RequiredReplicaMiss,
}

/// Result of applying a hint against the target's current record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HintReplayDecision {
    /// Hint produced the record that should be applied.
    Replayed { record: ReplicatedValueRecord },
    /// A newer value already exists on the target.
    SuppressedByNewerValue { current: ReplicatedValueRecord },
    /// A tombstone at the same or newer authority point prevents resurrection.
    SuppressedByTombstone { current: ReplicatedValueRecord },
}

impl HintReplayDecision {
    /// Return the hint outcome counter value for this decision.
    pub fn outcome(&self) -> HintOutcome {
        match self {
            Self::Replayed { .. } => HintOutcome::Replayed,
            Self::SuppressedByNewerValue { .. } | Self::SuppressedByTombstone { .. } => {
                HintOutcome::DroppedExpired
            }
        }
    }
}

/// Hint store metrics with bounded labels represented by counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HintMetrics {
    /// Stored hints.
    pub hints_stored_total: u64,
    /// Replayed hints.
    pub hints_replayed_total: u64,
    /// Dropped hints because they exceeded budget or age.
    pub hints_dropped_total: u64,
    /// Current approximate store bytes.
    pub hint_store_bytes: u64,
}

/// Errors returned by hint storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintError {
    message: String,
}

impl fmt::Display for HintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for HintError {}

/// Bounded hint storage contract.
pub trait HintStore {
    /// Store one hint unless the required-ack path forbids doing so.
    fn store(
        &mut self,
        hint: Hint,
        required_by_consistency: bool,
        now_millis: u64,
    ) -> Result<HintOutcome, HintError>;

    /// Drain all retained hints for one target in insertion order.
    fn drain_for(&mut self, target: &ClusterNodeId) -> Result<Vec<Hint>, HintError>;
}

/// In-memory bounded hint store used by tests and embedded adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InMemoryHintStore {
    budget: HintBudget,
    hints: BTreeMap<ClusterNodeId, VecDeque<Hint>>,
    repair_marks: BTreeSet<String>,
    metrics: HintMetrics,
}

impl InMemoryHintStore {
    /// Create an empty hint store.
    pub fn new(budget: HintBudget) -> Self {
        Self {
            budget,
            hints: BTreeMap::new(),
            repair_marks: BTreeSet::new(),
            metrics: HintMetrics::default(),
        }
    }

    /// Return the current metrics.
    pub fn metrics(&self) -> HintMetrics {
        let mut metrics = self.metrics;
        metrics.hint_store_bytes = self.total_bytes();
        metrics
    }

    /// Return whether a key was marked for Merkle repair fallback.
    pub fn is_marked_for_repair(&self, key: &str) -> bool {
        self.repair_marks.contains(key)
    }

    /// Return retained hint count.
    pub fn len(&self) -> usize {
        self.hints.values().map(VecDeque::len).sum()
    }

    /// Return whether no hints are retained.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn total_bytes(&self) -> u64 {
        self.hints
            .values()
            .flat_map(|hints| hints.iter())
            .map(Hint::approx_bytes)
            .sum()
    }

    fn mark_for_repair(&mut self, key: &str) {
        self.repair_marks.insert(key.to_owned());
    }
}

impl HintStore for InMemoryHintStore {
    fn store(
        &mut self,
        hint: Hint,
        required_by_consistency: bool,
        now_millis: u64,
    ) -> Result<HintOutcome, HintError> {
        if required_by_consistency {
            return Ok(HintOutcome::RequiredReplicaMiss);
        }
        if hint.is_expired(now_millis, self.budget) {
            self.metrics.hints_dropped_total = self.metrics.hints_dropped_total.saturating_add(1);
            self.mark_for_repair(&hint.key);
            return Ok(HintOutcome::DroppedExpired);
        }

        let next_count = self.len().saturating_add(1);
        let next_bytes = self.total_bytes().saturating_add(hint.approx_bytes());
        if next_count > self.budget.max_hints || next_bytes > self.budget.max_bytes {
            self.metrics.hints_dropped_total = self.metrics.hints_dropped_total.saturating_add(1);
            self.mark_for_repair(&hint.key);
            return Ok(HintOutcome::DroppedOverBudget);
        }

        self.hints
            .entry(hint.target.clone())
            .or_default()
            .push_back(hint);
        self.metrics.hints_stored_total = self.metrics.hints_stored_total.saturating_add(1);
        Ok(HintOutcome::Stored)
    }

    fn drain_for(&mut self, target: &ClusterNodeId) -> Result<Vec<Hint>, HintError> {
        let Some(hints) = self.hints.remove(target) else {
            return Ok(Vec::new());
        };
        Ok(hints.into_iter().collect())
    }
}

/// Apply a hint if it is still newer than the target state and does not resurrect a tombstone.
pub fn apply_hint(
    current: Option<&ReplicatedValueRecord>,
    hint: &Hint,
) -> Result<HintReplayDecision, HintError> {
    let candidate = hint.to_record();
    let Some(current) = current else {
        return Ok(HintReplayDecision::Replayed { record: candidate });
    };

    if current.is_tombstone() && (current.version, current.epoch) >= (hint.version, hint.epoch) {
        return Ok(HintReplayDecision::SuppressedByTombstone {
            current: current.clone(),
        });
    }

    let merged = current.clone().merge(candidate.clone());
    if merged == candidate {
        Ok(HintReplayDecision::Replayed { record: candidate })
    } else {
        Ok(HintReplayDecision::SuppressedByNewerValue {
            current: current.clone(),
        })
    }
}

/// Replay all hints for a target against caller-provided current state.
pub fn replay_hints<I>(
    hints: I,
    current_by_key: &BTreeMap<String, ReplicatedValueRecord>,
) -> Result<Vec<HintReplayDecision>, HintError>
where
    I: IntoIterator<Item = Hint>,
{
    hints
        .into_iter()
        .map(|hint| apply_hint(current_by_key.get(&hint.key), &hint))
        .collect()
}
