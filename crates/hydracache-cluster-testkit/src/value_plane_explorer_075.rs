//! Composite bounded explorer for transfer, expiry, failover and namespace lifecycle.

use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompositeTransferPhase {
    Idle,
    Snapshot,
    Delta,
    Ready,
    Committed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompositeNamespacePhase {
    Active,
    Draining,
    DeleteCommitted,
    Reclaimable,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompositeSnapshot {
    pub epoch: u64,
    pub logical_time: u64,
    pub owner_alive: bool,
    pub backup_alive: bool,
    pub acknowledged_version: u64,
    pub live_version: u64,
    pub tombstone_version: u64,
    pub expires_at: Option<u64>,
    pub transfer: CompositeTransferPhase,
    pub source_serving: bool,
    pub target_serving: bool,
    pub namespace_generation: u64,
    pub record_generation: Option<u64>,
    pub namespace: CompositeNamespacePhase,
}

impl Default for CompositeSnapshot {
    fn default() -> Self {
        Self {
            epoch: 1,
            logical_time: 0,
            owner_alive: true,
            backup_alive: true,
            acknowledged_version: 0,
            live_version: 0,
            tombstone_version: 0,
            expires_at: None,
            transfer: CompositeTransferPhase::Idle,
            source_serving: true,
            target_serving: false,
            namespace_generation: 1,
            record_generation: None,
            namespace: CompositeNamespacePhase::Active,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompositeAction {
    PutExpiring,
    Delete,
    Tick,
    LoseOwner,
    PromoteBackup,
    StartTransfer,
    FinishSnapshot,
    FinishDelta,
    CommitCutover,
    BeginNamespaceDelete,
    CommitNamespaceDelete,
    ProveReclaimable,
    RecreateNamespace,
}

impl CompositeAction {
    const ALL: [Self; 13] = [
        Self::PutExpiring,
        Self::Delete,
        Self::Tick,
        Self::LoseOwner,
        Self::PromoteBackup,
        Self::StartTransfer,
        Self::FinishSnapshot,
        Self::FinishDelta,
        Self::CommitCutover,
        Self::BeginNamespaceDelete,
        Self::CommitNamespaceDelete,
        Self::ProveReclaimable,
        Self::RecreateNamespace,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeBounds {
    pub max_depth: usize,
    pub max_states: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeViolation {
    pub invariant: &'static str,
    pub trace: Vec<CompositeAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeReport {
    pub explored_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: usize,
    pub truncated: bool,
    pub violations: Vec<CompositeViolation>,
}

impl CompositeReport {
    pub fn passed(&self) -> bool {
        !self.truncated && self.violations.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositeExplorerError {
    InvalidBound(&'static str),
}

pub struct CompositeExplorer {
    bounds: CompositeBounds,
}

impl CompositeExplorer {
    pub fn new(bounds: CompositeBounds) -> Result<Self, CompositeExplorerError> {
        if bounds.max_depth == 0 {
            return Err(CompositeExplorerError::InvalidBound("depth"));
        }
        if bounds.max_states == 0 {
            return Err(CompositeExplorerError::InvalidBound("states"));
        }
        Ok(Self { bounds })
    }

    pub fn explore(&self) -> CompositeReport {
        let initial = CompositeSnapshot::default();
        let mut seen = BTreeSet::from([initial.clone()]);
        let mut queue = VecDeque::from([(initial, Vec::new())]);
        let mut explored_transitions = 0_usize;
        let mut max_depth_reached = 0_usize;
        let mut truncated = false;
        let mut violations = Vec::new();
        while let Some((state, trace)) = queue.pop_front() {
            max_depth_reached = max_depth_reached.max(trace.len());
            for invariant in composite_invariant_violations(&state) {
                violations.push(CompositeViolation {
                    invariant,
                    trace: trace.clone(),
                });
            }
            if trace.len() >= self.bounds.max_depth {
                continue;
            }
            for action in CompositeAction::ALL {
                let Some(next) = apply_action(&state, action) else {
                    continue;
                };
                explored_transitions = explored_transitions.saturating_add(1);
                if seen.contains(&next) {
                    continue;
                }
                if seen.len() >= self.bounds.max_states {
                    truncated = true;
                    continue;
                }
                let mut next_trace = trace.clone();
                next_trace.push(action);
                seen.insert(next.clone());
                queue.push_back((next, next_trace));
            }
        }
        CompositeReport {
            explored_states: seen.len(),
            explored_transitions,
            max_depth_reached,
            truncated,
            violations,
        }
    }
}

pub fn composite_invariant_violations(state: &CompositeSnapshot) -> Vec<&'static str> {
    let mut violations = Vec::new();
    if state.acknowledged_version > state.live_version.max(state.tombstone_version) {
        violations.push("acknowledgement_without_record");
    }
    if state.target_serving && state.transfer != CompositeTransferPhase::Committed {
        violations.push("target_serves_before_cutover");
    }
    if state.source_serving && state.transfer == CompositeTransferPhase::Committed {
        violations.push("source_serves_after_cutover");
    }
    if state.owner_alive && !state.source_serving && !state.target_serving {
        violations.push("live_owner_without_serving_authority");
    }
    if state.record_generation.is_some_and(|generation| {
        generation != state.namespace_generation
            && state.live_version.max(state.tombstone_version) > 0
    }) {
        violations.push("namespace_generation_crossing");
    }
    if state.live_version > 0
        && state.tombstone_version > 0
        && state.live_version <= state.tombstone_version
    {
        violations.push("stale_value_resurrection");
    }
    violations
}

pub fn shrink_failing_schedule<T: Clone>(
    schedule: &[T],
    mut still_fails: impl FnMut(&[T]) -> bool,
) -> Vec<T> {
    let mut minimized = schedule.to_vec();
    let mut index = 0;
    while index < minimized.len() {
        let mut candidate = minimized.clone();
        candidate.remove(index);
        if still_fails(&candidate) {
            minimized = candidate;
        } else {
            index += 1;
        }
    }
    minimized
}

fn apply_action(state: &CompositeSnapshot, action: CompositeAction) -> Option<CompositeSnapshot> {
    let mut next = state.clone();
    match action {
        CompositeAction::PutExpiring
            if state.owner_alive
                && state.backup_alive
                && state.namespace == CompositeNamespacePhase::Active =>
        {
            let version = state
                .live_version
                .max(state.tombstone_version)
                .saturating_add(1);
            next.live_version = version;
            next.acknowledged_version = version;
            next.expires_at = Some(state.logical_time.saturating_add(2));
            next.record_generation = Some(state.namespace_generation);
        }
        CompositeAction::Delete
            if state.owner_alive
                && state.backup_alive
                && state.namespace == CompositeNamespacePhase::Active =>
        {
            let version = state
                .live_version
                .max(state.tombstone_version)
                .saturating_add(1);
            next.live_version = 0;
            next.tombstone_version = version;
            next.acknowledged_version = version;
            next.expires_at = None;
            next.record_generation = Some(state.namespace_generation);
        }
        CompositeAction::Tick if state.logical_time < 3 => {
            next.logical_time += 1;
            if state
                .expires_at
                .is_some_and(|expires| expires <= next.logical_time)
            {
                next.tombstone_version = state.live_version.max(state.tombstone_version);
                next.live_version = 0;
                next.expires_at = None;
            }
        }
        CompositeAction::LoseOwner if state.owner_alive => next.owner_alive = false,
        CompositeAction::PromoteBackup if !state.owner_alive && state.backup_alive => {
            next.epoch += 1;
            next.owner_alive = true;
        }
        CompositeAction::StartTransfer if state.transfer == CompositeTransferPhase::Idle => {
            next.transfer = CompositeTransferPhase::Snapshot;
        }
        CompositeAction::FinishSnapshot if state.transfer == CompositeTransferPhase::Snapshot => {
            next.transfer = CompositeTransferPhase::Delta;
        }
        CompositeAction::FinishDelta if state.transfer == CompositeTransferPhase::Delta => {
            next.transfer = CompositeTransferPhase::Ready;
        }
        CompositeAction::CommitCutover if state.transfer == CompositeTransferPhase::Ready => {
            next.transfer = CompositeTransferPhase::Committed;
            next.epoch += 1;
            next.source_serving = false;
            next.target_serving = true;
        }
        CompositeAction::BeginNamespaceDelete
            if state.namespace == CompositeNamespacePhase::Active =>
        {
            next.namespace = CompositeNamespacePhase::Draining;
        }
        CompositeAction::CommitNamespaceDelete
            if state.namespace == CompositeNamespacePhase::Draining =>
        {
            next.namespace = CompositeNamespacePhase::DeleteCommitted;
            next.live_version = 0;
            next.tombstone_version = 0;
            next.acknowledged_version = 0;
            next.expires_at = None;
            next.record_generation = None;
        }
        CompositeAction::ProveReclaimable
            if state.namespace == CompositeNamespacePhase::DeleteCommitted =>
        {
            next.namespace = CompositeNamespacePhase::Reclaimable;
        }
        CompositeAction::RecreateNamespace
            if state.namespace == CompositeNamespacePhase::Reclaimable =>
        {
            next.namespace = CompositeNamespacePhase::Active;
            next.namespace_generation += 1;
        }
        _ => return None,
    }
    Some(next)
}
