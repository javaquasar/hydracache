use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, ClusterNodeId, PartitionId};
use crate::grid::elasticity::{
    restore_topology_from_snapshot, ControlPlaneSnapshot, RegionId, SnapshotError,
    TopologyAuthority, CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION,
};
use crate::grid::hardening::{
    anti_entropy_repair, PromotionFreezeWindow, ReplicatedValueRecord, ReplicatedValueStore,
    ValueStoreError,
};
use crate::grid::{EffectiveReplicationMap, PromotionPhase, Replicas};

/// Operator-visible region health state used by region failover decisions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionState {
    /// Region is reachable and not under suspicion.
    #[default]
    Up,
    /// Region has symptoms but is not safe to promote away from automatically.
    Suspect,
    /// Region was explicitly declared down and can be promoted away from.
    Down,
}

/// Conservative input for region-state detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionObservation {
    /// Region being observed.
    pub region: RegionId,
    /// Consecutive missed health signals.
    pub missed_heartbeats: u64,
    /// Whether the local quorum path for the region is reachable.
    pub quorum_reachable: bool,
    /// Operator-declared region-down intent.
    pub operator_declared_down: bool,
    /// Whether split-brain/double-promotion risk is still visible.
    pub split_brain_risk: bool,
}

impl RegionObservation {
    /// Create a healthy observation for a region.
    pub fn healthy(region: impl Into<RegionId>) -> Self {
        Self {
            region: region.into(),
            missed_heartbeats: 0,
            quorum_reachable: true,
            operator_declared_down: false,
            split_brain_risk: false,
        }
    }
}

/// Conservative detector: automatic checks can only move a region to `Suspect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionStateDetector {
    suspect_after_missed: u64,
}

impl RegionStateDetector {
    /// Create a detector with a normalized missed-heartbeat threshold.
    pub fn new(suspect_after_missed: u64) -> Self {
        Self {
            suspect_after_missed: suspect_after_missed.max(1),
        }
    }

    /// Classify one observation.
    pub fn classify(&self, observation: &RegionObservation) -> RegionState {
        if observation.operator_declared_down && !observation.split_brain_risk {
            return RegionState::Down;
        }
        if observation.operator_declared_down
            || observation.split_brain_risk
            || !observation.quorum_reachable
            || observation.missed_heartbeats >= self.suspect_after_missed
        {
            return RegionState::Suspect;
        }
        RegionState::Up
    }
}

impl Default for RegionStateDetector {
    fn default() -> Self {
        Self::new(3)
    }
}

/// Error returned by region failover helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionFailoverError {
    message: String,
}

impl RegionFailoverError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RegionFailoverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RegionFailoverError {}

/// Region-home promotion committed through the control-plane topology epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionPromotion {
    /// Partitions whose home region changed.
    pub partitions: Vec<PartitionId>,
    /// Region that used to own the home primary.
    pub from_home: RegionId,
    /// Surviving region selected as the new home.
    pub to_home: RegionId,
    /// New committed epoch for the topology operation.
    pub epoch: ClusterEpoch,
    /// Final phase reached by the deterministic freeze -> commit -> converge -> unfreeze flow.
    pub phase: PromotionPhase,
}

/// Result of a region-home promotion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionPromotionReport {
    /// Promotion metadata.
    pub promotion: RegionPromotion,
    /// Snapshot after the ownership commit.
    pub snapshot: ControlPlaneSnapshot,
    /// Partitions promoted with fewer copies than their previous replication factor.
    pub degraded_partitions: Vec<PartitionId>,
    /// Bounded freeze window observed for the promotion.
    pub freeze: PromotionFreezeWindow,
}

impl RegionPromotionReport {
    /// Return whether every affected partition retained its previous copy count.
    pub fn is_fully_replicated(&self) -> bool {
        self.degraded_partitions.is_empty()
    }
}

/// Promote partitions whose primary/home node belonged to `from_home`.
///
/// The function models the safe control-plane sequence from the release plan:
/// freeze writes, commit a higher epoch, converge through anti-entropy, then
/// unfreeze. Automatic detector suspicion is not enough: the old home must be
/// explicitly `Down`.
pub fn promote_region_home(
    snapshot: &ControlPlaneSnapshot,
    from_home: RegionId,
    to_home: RegionId,
    from_state: RegionState,
    freeze: PromotionFreezeWindow,
) -> Result<RegionPromotionReport, RegionFailoverError> {
    if from_home == to_home {
        return Err(RegionFailoverError::new(
            "from_home and to_home must be different regions",
        ));
    }
    if from_state != RegionState::Down {
        return Err(RegionFailoverError::new(
            "region must be explicitly declared down before promotion",
        ));
    }
    if !freeze.is_bounded() {
        return Err(RegionFailoverError::new(
            "region promotion freeze window exceeded its bound",
        ));
    }

    let target_nodes = nodes_in_region(snapshot, &to_home);
    let mut next = snapshot.clone();
    next.epoch = next_epoch(snapshot.epoch);
    let mut promoted = Vec::new();
    let mut degraded = Vec::new();

    for (partition, replicas) in &snapshot.ownership {
        if primary_region(snapshot, replicas) != Some(&from_home) {
            continue;
        }
        promoted.push(*partition);
        let Some(primary) = target_nodes.first().cloned() else {
            degraded.push(*partition);
            continue;
        };
        let desired_copies = replicas.copy_count();
        let mut backups = surviving_backups(snapshot, replicas, &from_home, &primary);
        for candidate in target_nodes.iter().skip(1) {
            if 1 + backups.len() >= desired_copies {
                break;
            }
            if !backups.contains(candidate) {
                backups.push(candidate.clone());
            }
        }
        let promoted_replicas = Replicas::new(primary, backups);
        if promoted_replicas.copy_count() < desired_copies {
            degraded.push(*partition);
        }
        next.ownership.insert(*partition, promoted_replicas);
    }

    Ok(RegionPromotionReport {
        promotion: RegionPromotion {
            partitions: promoted,
            from_home,
            to_home,
            epoch: next.epoch,
            phase: PromotionPhase::Finalize,
        },
        snapshot: next,
        degraded_partitions: degraded,
        freeze,
    })
}

fn next_epoch(epoch: ClusterEpoch) -> ClusterEpoch {
    ClusterEpoch::new(epoch.value().saturating_add(1))
}

fn nodes_in_region(snapshot: &ControlPlaneSnapshot, region: &RegionId) -> Vec<ClusterNodeId> {
    snapshot
        .topology
        .iter()
        .filter(|(_, topology)| &topology.region == region)
        .map(|(node, _)| node.clone())
        .collect()
}

fn primary_region<'a>(
    snapshot: &'a ControlPlaneSnapshot,
    replicas: &Replicas,
) -> Option<&'a RegionId> {
    snapshot
        .topology
        .get(&replicas.primary)
        .map(|topology| &topology.region)
}

fn surviving_backups(
    snapshot: &ControlPlaneSnapshot,
    replicas: &Replicas,
    down_region: &RegionId,
    new_primary: &ClusterNodeId,
) -> Vec<ClusterNodeId> {
    replicas
        .all_nodes()
        .into_iter()
        .filter(|node| node != new_primary)
        .filter(|node| {
            snapshot
                .topology
                .get(node)
                .map(|topology| &topology.region != down_region)
                .unwrap_or(false)
        })
        .collect()
}

/// Decision for a region that rejoins after a promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejoiningRegionDecision {
    /// The rejoining region is not stale relative to the committed epoch.
    AcceptAuthority,
    /// The region must drop authority and backfill from the current epoch.
    FenceLowerEpoch {
        /// Current committed authority epoch.
        current_epoch: ClusterEpoch,
        /// Epoch advertised by the rejoining region.
        rejoining_epoch: ClusterEpoch,
    },
}

/// Apply the A1-style epoch fence to a rejoining region.
pub fn rejoining_region_authority(
    current_epoch: ClusterEpoch,
    rejoining_epoch: ClusterEpoch,
) -> RejoiningRegionDecision {
    if rejoining_epoch < current_epoch {
        RejoiningRegionDecision::FenceLowerEpoch {
            current_epoch,
            rejoining_epoch,
        }
    } else {
        RejoiningRegionDecision::AcceptAuthority
    }
}

/// DR restore input: control-plane snapshot plus operator-restored durable values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionRestore<S> {
    snapshot: ControlPlaneSnapshot,
    values: S,
}

impl<S> RegionRestore<S> {
    /// Create a restore operation.
    pub fn new(snapshot: ControlPlaneSnapshot, values: S) -> Self {
        Self { snapshot, values }
    }

    /// Return the snapshot being restored.
    pub fn snapshot(&self) -> &ControlPlaneSnapshot {
        &self.snapshot
    }

    /// Return the durable values being restored.
    pub fn values(&self) -> &S {
        &self.values
    }

    /// Return mutable durable values, used by anti-entropy backfill.
    pub fn values_mut(&mut self) -> &mut S {
        &mut self.values
    }
}

impl<S> RegionRestore<S>
where
    S: ReplicatedValueStore,
{
    /// Backfill restored durable values from anti-entropy records.
    pub fn backfill_from(
        &mut self,
        records: impl IntoIterator<Item = (String, ReplicatedValueRecord)>,
    ) -> Result<u64, RegionRestoreError> {
        anti_entropy_repair(&mut self.values, records).map_err(RegionRestoreError::from)
    }

    /// Restore the control-plane authority and return the durable store.
    pub fn restore(self) -> Result<RegionRestoreOutcome<S>, RegionRestoreError> {
        if self.snapshot.format_version > CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION {
            return Err(RegionRestoreError::from(SnapshotError::new(
                "control-plane snapshot format is newer than this binary",
            )));
        }
        let authority =
            restore_topology_from_snapshot(&self.snapshot).map_err(RegionRestoreError::from)?;
        let restored_value_count = restored_value_count(&self.snapshot, &self.values)?;
        let report = RegionRestoreReport {
            epoch: self.snapshot.epoch,
            topology_node_count: self.snapshot.topology.len(),
            partition_count: self.snapshot.ownership.len(),
            restored_value_count,
        };
        Ok(RegionRestoreOutcome {
            authority,
            values: self.values,
            report,
        })
    }
}

fn restored_value_count<S>(
    snapshot: &ControlPlaneSnapshot,
    values: &S,
) -> Result<usize, RegionRestoreError>
where
    S: ReplicatedValueStore,
{
    let mut keys = BTreeSet::new();
    for replicas in snapshot.ownership.values() {
        let map = EffectiveReplicationMap::new(replicas.clone());
        for (key, _) in values.scan_owned(&map).map_err(RegionRestoreError::from)? {
            keys.insert(key);
        }
    }
    Ok(keys.len())
}

/// Completed DR restore result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionRestoreOutcome<S> {
    /// Restored topology authority.
    pub authority: TopologyAuthority,
    /// Restored durable values.
    pub values: S,
    /// Operator-visible restore report.
    pub report: RegionRestoreReport,
}

/// Operator-visible DR restore report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionRestoreReport {
    /// Restored control-plane epoch.
    pub epoch: ClusterEpoch,
    /// Restored topology node count.
    pub topology_node_count: usize,
    /// Restored ownership partition count.
    pub partition_count: usize,
    /// Unique durable value keys visible through restored ownership.
    pub restored_value_count: usize,
}

/// Error returned by DR restore helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionRestoreError {
    message: String,
}

impl RegionRestoreError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<SnapshotError> for RegionRestoreError {
    fn from(error: SnapshotError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<ValueStoreError> for RegionRestoreError {
    fn from(error: ValueStoreError) -> Self {
        Self::new(error.to_string())
    }
}

impl fmt::Display for RegionRestoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RegionRestoreError {}

/// Bounded metric snapshot for region failover surfaces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionFailoverMetrics {
    /// Last observed state for a region.
    pub region_state: RegionState,
    /// Total successful region promotions.
    pub region_promotion_total: u64,
    /// Last restore duration in logical milliseconds.
    pub region_restore_duration_ms: u64,
}
