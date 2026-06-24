use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::grid::session_context::{PartitionKey, SessionWatermark, VersionStamp};

/// Bounded causal dependency summary carried by a session write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalSummary {
    cap: usize,
    deps: BTreeMap<PartitionKey, VersionStamp>,
    coarse_floor: Option<VersionStamp>,
    coarsened_total: u64,
}

impl CausalSummary {
    /// Create an empty bounded summary.
    pub fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            deps: BTreeMap::new(),
            coarse_floor: None,
            coarsened_total: 0,
        }
    }

    /// Build a summary from the current session watermark.
    pub fn from_watermark(watermark: &SessionWatermark) -> Self {
        let mut summary = Self::new(watermark.cap());
        for (key, stamp) in watermark.entries() {
            summary.observe(key.clone(), *stamp);
        }
        summary
    }

    /// Return the exact dependency cap.
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Return retained exact dependency count.
    pub fn len(&self) -> usize {
        self.deps.len()
    }

    /// Return whether no dependency metadata is retained.
    pub fn is_empty(&self) -> bool {
        self.deps.is_empty() && self.coarse_floor.is_none()
    }

    /// Return conservative coarsening events.
    pub fn coarsened_total(&self) -> u64 {
        self.coarsened_total
    }

    /// Return a stable-floor dependency created by overflow coarsening.
    pub fn coarse_floor(&self) -> Option<VersionStamp> {
        self.coarse_floor
    }

    /// Return exact dependencies for diagnostics/tests.
    pub fn dependencies(&self) -> &BTreeMap<PartitionKey, VersionStamp> {
        &self.deps
    }

    /// Approximate dependency metadata bytes for aggregate gauges.
    pub fn dependency_bytes(&self) -> u64 {
        const ENTRY_BYTES: u64 = 56;
        const COARSE_FLOOR_BYTES: u64 = 24;

        (self.deps.len() as u64 * ENTRY_BYTES)
            + u64::from(self.coarse_floor.is_some()) * COARSE_FLOOR_BYTES
    }

    /// Observe a read/write dependency.
    pub fn observe(&mut self, key: PartitionKey, stamp: VersionStamp) {
        if let Some(existing) = self.deps.get_mut(&key) {
            if stamp > *existing {
                *existing = stamp;
            }
            return;
        }

        if self.deps.len() >= self.cap {
            self.coarsen_one();
        }
        self.deps.insert(key, stamp);
    }

    /// Return dependencies not yet applied locally.
    pub fn not_yet_applied(&self, applied: &AppliedSet) -> Vec<CausalDependencyMissing> {
        let mut missing = Vec::new();
        if let Some(required) = self.coarse_floor {
            if !applied.covers_coarse_floor(required) {
                missing.push(CausalDependencyMissing::CoarseFloor { required });
            }
        }
        for (key, required) in &self.deps {
            if !applied.covers(key, *required) {
                missing.push(CausalDependencyMissing::Exact {
                    key: key.clone(),
                    required: *required,
                });
            }
        }
        missing
    }

    /// Remove metadata that is already repair-confirmed stable.
    pub fn gc_stable(&mut self, applied: &AppliedSet) -> usize {
        let before = self.deps.len() + usize::from(self.coarse_floor.is_some());
        self.deps.retain(|key, stamp| !applied.covers(key, *stamp));
        if self
            .coarse_floor
            .is_some_and(|floor| applied.covers_coarse_floor(floor))
        {
            self.coarse_floor = None;
        }
        before - (self.deps.len() + usize::from(self.coarse_floor.is_some()))
    }

    fn coarsen_one(&mut self) {
        if let Some((key, stamp)) = self
            .deps
            .iter()
            .min_by_key(|(_, stamp)| **stamp)
            .map(|(key, stamp)| (key.clone(), *stamp))
        {
            self.deps.remove(&key);
            self.coarse_floor = Some(self.coarse_floor.map_or(stamp, |floor| floor.max(stamp)));
            self.coarsened_total = self.coarsened_total.saturating_add(1);
        }
    }
}

/// Locally applied dependency high-water stamps.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedSet {
    applied: BTreeMap<PartitionKey, VersionStamp>,
    stable_floor: Option<VersionStamp>,
}

impl AppliedSet {
    /// Create an empty applied set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a specific dependency as locally applied.
    pub fn mark_applied(&mut self, key: PartitionKey, stamp: VersionStamp) {
        self.applied
            .entry(key)
            .and_modify(|existing| *existing = (*existing).max(stamp))
            .or_insert(stamp);
    }

    /// Mark a global repair-confirmed stable floor.
    pub fn mark_stable_floor(&mut self, stamp: VersionStamp) {
        self.stable_floor = Some(self.stable_floor.map_or(stamp, |floor| floor.max(stamp)));
    }

    /// Return the global repair-confirmed stable floor.
    pub fn stable_floor(&self) -> Option<VersionStamp> {
        self.stable_floor
    }

    /// Return whether a dependency is covered by local state.
    pub fn covers(&self, key: &PartitionKey, required: VersionStamp) -> bool {
        self.covers_coarse_floor(required)
            || self
                .applied
                .get(key)
                .map(|stamp| stamp.covers(required))
                .unwrap_or(false)
    }

    /// Return whether the global stable floor covers a coarsened dependency.
    pub fn covers_coarse_floor(&self, required: VersionStamp) -> bool {
        self.stable_floor
            .map(|floor| floor.covers(required))
            .unwrap_or(false)
    }

    /// Return exact applied dependencies for diagnostics/tests.
    pub fn entries(&self) -> &BTreeMap<PartitionKey, VersionStamp> {
        &self.applied
    }
}

/// Dependency that is missing before a causal write can become visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CausalDependencyMissing {
    /// Exact partition/region dependency is missing.
    Exact {
        /// Missing dependency key.
        key: PartitionKey,
        /// Required stamp.
        required: VersionStamp,
    },
    /// Overflow was coarsened and now requires a stable floor.
    CoarseFloor {
        /// Required stable floor.
        required: VersionStamp,
    },
}

/// Causal apply decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyDecision {
    /// Every dependency is already applied.
    Apply,
    /// The write must stay invisible until dependencies are repaired/applied.
    Defer {
        /// Dependencies that are not yet satisfied locally.
        missing: Vec<CausalDependencyMissing>,
    },
}

/// Session write with attached causal dependency summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalWrite {
    /// Effect partition/region.
    pub key: PartitionKey,
    /// Effect version.
    pub stamp: VersionStamp,
    /// Dependencies captured from the session before the write.
    pub deps: CausalSummary,
}

impl CausalWrite {
    /// Create a causal write.
    pub fn new(key: PartitionKey, stamp: VersionStamp, deps: CausalSummary) -> Self {
        Self { key, stamp, deps }
    }
}

/// Error returned when a causal write is not yet visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalApplyDeferred {
    /// Missing dependencies.
    pub missing: Vec<CausalDependencyMissing>,
}

impl fmt::Display for CausalApplyDeferred {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "causal write deferred with {} missing dependencies",
            self.missing.len()
        )
    }
}

impl std::error::Error for CausalApplyDeferred {}

/// Causal consistency metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalConsistencyMetrics {
    /// Writes deferred because dependencies were missing.
    pub causal_writes_deferred_total: u64,
    /// Summary overflow/coarsening events.
    pub causal_summary_coarsened_total: u64,
    /// Approximate dependency metadata bytes.
    pub causal_dependency_bytes: u64,
}

impl From<&CausalSummary> for CausalConsistencyMetrics {
    fn from(summary: &CausalSummary) -> Self {
        Self {
            causal_writes_deferred_total: 0,
            causal_summary_coarsened_total: summary.coarsened_total(),
            causal_dependency_bytes: summary.dependency_bytes(),
        }
    }
}

/// Decide whether a causal write can become visible on this replica.
pub fn causal_apply(local_applied: &AppliedSet, write_deps: &CausalSummary) -> ApplyDecision {
    let missing = write_deps.not_yet_applied(local_applied);
    if missing.is_empty() {
        ApplyDecision::Apply
    } else {
        ApplyDecision::Defer { missing }
    }
}

/// Apply a causal write or return the dependencies that must be repaired first.
pub fn apply_causal_write(
    local_applied: &mut AppliedSet,
    write: CausalWrite,
) -> Result<(), CausalApplyDeferred> {
    match causal_apply(local_applied, &write.deps) {
        ApplyDecision::Apply => {
            local_applied.mark_applied(write.key, write.stamp);
            Ok(())
        }
        ApplyDecision::Defer { missing } => Err(CausalApplyDeferred { missing }),
    }
}
