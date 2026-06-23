use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, ClusterNodeId, PartitionId};
use crate::grid::elasticity::{
    validate_move_preserves_zone_quorum, MovePhase, NodeTopology, PartitionMove, RegionId,
    ReshardPlan, ReshardPlanError, UpgradeGuard, UpgradeGuardError, UpgradeStep,
    ZoneAwareReplicaSet,
};
use crate::grid::ReplicationConfig;

/// Recommendation emitted for an external autoscaler or operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaleRecommendation {
    /// No scaling action should be taken.
    Hold,
    /// Add nodes and backfill before quorum admission.
    ScaleOut {
        /// Suggested number of nodes to add.
        suggested: usize,
    },
    /// Drain listed nodes before removal.
    ScaleIn {
        /// Nodes that can be drained by an external autoscaler.
        drain: Vec<ClusterNodeId>,
    },
    /// Rebalance hot partitions without changing membership.
    Rebalance,
}

/// One capacity sample consumed by the recommendation engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapacitySample {
    /// Region that owns this sample.
    pub region: RegionId,
    /// Memory pressure, normalized to `0.0..=1.0+`.
    pub memory_pressure: f32,
    /// Cross-region replication lag.
    pub replication_lag: u64,
    /// Hot-partition skew ratio.
    pub hot_partition_skew: f32,
    /// Repair debt that can block safe scaling.
    pub repair_debt: u64,
    /// Seconds since the last accepted scale action.
    pub seconds_since_last_scale: u64,
    /// Nodes eligible for scale-in drain.
    pub scale_in_candidates: Vec<ClusterNodeId>,
}

impl CapacitySample {
    /// Create a capacity sample with no scale-in candidates.
    pub fn new(region: impl Into<RegionId>) -> Self {
        Self {
            region: region.into(),
            memory_pressure: 0.0,
            replication_lag: 0,
            hot_partition_skew: 0.0,
            repair_debt: 0,
            seconds_since_last_scale: u64::MAX,
            scale_in_candidates: Vec::new(),
        }
    }
}

/// Tunables for capacity recommendations.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CapacityThresholds {
    /// Memory pressure at which scale-out is recommended.
    pub scale_out_memory_pressure: f32,
    /// Memory pressure below which scale-in can be recommended.
    pub scale_in_memory_pressure: f32,
    /// Replication lag at which scale-out is recommended.
    pub replication_lag_limit: u64,
    /// Hot-partition skew at which rebalance is recommended.
    pub hot_partition_skew_limit: f32,
    /// Repair debt at which rebalance is recommended.
    pub repair_debt_limit: u64,
    /// Minimum seconds between accepted scale actions.
    pub minimum_dwell_secs: u64,
    /// Suggested node count for scale-out recommendations.
    pub scale_out_suggested: usize,
}

impl Default for CapacityThresholds {
    fn default() -> Self {
        Self {
            scale_out_memory_pressure: 0.85,
            scale_in_memory_pressure: 0.25,
            replication_lag_limit: 1_000,
            hot_partition_skew_limit: 2.0,
            repair_debt_limit: 100,
            minimum_dwell_secs: 300,
            scale_out_suggested: 1,
        }
    }
}

/// Structured capacity signal exported by status/metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapacitySignal {
    /// Region this signal describes.
    pub region: RegionId,
    /// Memory pressure, normalized to `0.0..=1.0+`.
    pub memory_pressure: f32,
    /// Cross-region replication lag.
    pub replication_lag: u64,
    /// Hot-partition skew ratio.
    pub hot_partition_skew: f32,
    /// Repair debt that can block safe scaling.
    pub repair_debt: u64,
    /// Autoscaler recommendation.
    pub recommendation: ScaleRecommendation,
}

/// Compute one capacity recommendation with dwell-time hysteresis.
pub fn evaluate_capacity(sample: CapacitySample, thresholds: CapacityThresholds) -> CapacitySignal {
    let in_dwell_window = sample.seconds_since_last_scale < thresholds.minimum_dwell_secs;
    let recommendation = if in_dwell_window {
        ScaleRecommendation::Hold
    } else if sample.memory_pressure >= thresholds.scale_out_memory_pressure
        || sample.replication_lag > thresholds.replication_lag_limit
    {
        ScaleRecommendation::ScaleOut {
            suggested: thresholds.scale_out_suggested.max(1),
        }
    } else if sample.hot_partition_skew >= thresholds.hot_partition_skew_limit
        || sample.repair_debt > thresholds.repair_debt_limit
    {
        ScaleRecommendation::Rebalance
    } else if sample.memory_pressure <= thresholds.scale_in_memory_pressure
        && !sample.scale_in_candidates.is_empty()
    {
        ScaleRecommendation::ScaleIn {
            drain: sample.scale_in_candidates.clone(),
        }
    } else {
        ScaleRecommendation::Hold
    };

    CapacitySignal {
        region: sample.region,
        memory_pressure: sample.memory_pressure,
        replication_lag: sample.replication_lag,
        hot_partition_skew: sample.hot_partition_skew,
        repair_debt: sample.repair_debt,
        recommendation,
    }
}

/// Autoscaler membership intent accepted through the guarded admission surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoscalerIntent {
    /// Add a node, backfill partitions, then count it toward quorum.
    ScaleOut {
        /// Joining node.
        node: ClusterNodeId,
        /// Authoritative topology for the joining node.
        topology: NodeTopology,
        /// Candidate replica set after the join.
        candidate: ZoneAwareReplicaSet,
        /// Backfill work from existing owners into the joining node.
        backfill_sources: Vec<(PartitionId, ClusterNodeId, u64)>,
        /// Compatibility step advertised by the joining binary.
        compat: UpgradeStep,
    },
    /// Drain a node before removing it.
    ScaleIn {
        /// Node being drained.
        drain: ClusterNodeId,
        /// Remaining voters after removal.
        remaining_voters: usize,
        /// Drain movements to surviving owners.
        drain_targets: Vec<(PartitionId, ClusterNodeId, u64)>,
        /// Compatibility step advertised by the operator/controller.
        compat: UpgradeStep,
    },
    /// Rebalance partitions without a membership change.
    Rebalance {
        /// Candidate reshard plan.
        plan: ReshardPlan,
        /// Compatibility step advertised by the operator/controller.
        compat: UpgradeStep,
    },
}

/// Guardrails used to admit an autoscaler intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoscalerAdmissionPolicy {
    /// Current control-plane epoch.
    pub epoch: ClusterEpoch,
    /// Replication/quorum configuration.
    pub replication: ReplicationConfig,
    /// Compatibility guard.
    pub upgrade_guard: UpgradeGuard,
    /// Maximum concurrent reshard moves.
    pub max_concurrent_moves: usize,
}

impl AutoscalerAdmissionPolicy {
    /// Create a policy with normalized concurrency.
    pub fn new(
        epoch: ClusterEpoch,
        replication: ReplicationConfig,
        upgrade_guard: UpgradeGuard,
        max_concurrent_moves: usize,
    ) -> Self {
        Self {
            epoch,
            replication,
            upgrade_guard,
            max_concurrent_moves: max_concurrent_moves.max(1),
        }
    }
}

/// Accepted scale action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaleAction {
    /// Add a node.
    ScaleOut,
    /// Drain/remove a node.
    ScaleIn,
    /// Rebalance partitions.
    Rebalance,
}

/// Accepted autoscaler admission result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoscalerAdmission {
    /// Action accepted by the guard.
    pub action: ScaleAction,
    /// Reshard/drain/backfill work to execute before final membership effect.
    pub plan: ReshardPlan,
    /// Whether a joining node can count toward quorum immediately.
    pub quorum_eligible: bool,
    /// Whether a draining node may be removed immediately.
    pub removal_allowed: bool,
}

/// Validate an autoscaler intent against zone, quorum, and COMPAT invariants.
pub fn admit_autoscaler_intent(
    intent: AutoscalerIntent,
    policy: AutoscalerAdmissionPolicy,
) -> Result<AutoscalerAdmission, AutoscalerIntentError> {
    policy
        .replication
        .validate()
        .map_err(|error| AutoscalerIntentError::new(error.to_string()))?;

    match intent {
        AutoscalerIntent::ScaleOut {
            node,
            topology: _,
            candidate,
            backfill_sources,
            compat,
        } => {
            policy
                .upgrade_guard
                .check(compat)
                .map_err(AutoscalerIntentError::from)?;
            validate_move_preserves_zone_quorum(&candidate, policy.replication.write_quorum)
                .map_err(AutoscalerIntentError::from)?;
            let moves = backfill_sources
                .into_iter()
                .map(|(partition, from, bytes)| {
                    PartitionMove::new(partition, from, node.clone(), bytes)
                })
                .collect();
            Ok(AutoscalerAdmission {
                action: ScaleAction::ScaleOut,
                plan: ReshardPlan::new(policy.epoch, moves, policy.max_concurrent_moves),
                quorum_eligible: false,
                removal_allowed: false,
            })
        }
        AutoscalerIntent::ScaleIn {
            drain,
            remaining_voters,
            drain_targets,
            compat,
        } => {
            policy
                .upgrade_guard
                .check(compat)
                .map_err(AutoscalerIntentError::from)?;
            if remaining_voters < policy.replication.write_quorum {
                return Err(AutoscalerIntentError::new(
                    "autoscaler intent would break write quorum",
                ));
            }
            Ok(AutoscalerAdmission {
                action: ScaleAction::ScaleIn,
                plan: ReshardPlan::drain_node(
                    policy.epoch,
                    drain,
                    drain_targets,
                    policy.max_concurrent_moves,
                ),
                quorum_eligible: true,
                removal_allowed: false,
            })
        }
        AutoscalerIntent::Rebalance { plan, compat } => {
            policy
                .upgrade_guard
                .check(compat)
                .map_err(AutoscalerIntentError::from)?;
            Ok(AutoscalerAdmission {
                action: ScaleAction::Rebalance,
                plan,
                quorum_eligible: true,
                removal_allowed: true,
            })
        }
    }
}

/// Return whether every move has reached the committed owner.
pub fn scale_out_counts_toward_quorum(plan: &ReshardPlan) -> bool {
    !plan.moves.is_empty()
        && plan
            .moves
            .iter()
            .all(|movement| movement.phase >= MovePhase::Commit)
}

/// Return whether a scale-in drain completed and the node can be removed.
pub fn scale_in_removal_allowed(plan: &ReshardPlan) -> bool {
    !plan.moves.is_empty()
        && plan
            .moves
            .iter()
            .all(|movement| movement.phase == MovePhase::Cleanup)
}

/// Error returned by autoscaler intent admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoscalerIntentError {
    message: String,
}

impl AutoscalerIntentError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<UpgradeGuardError> for AutoscalerIntentError {
    fn from(error: UpgradeGuardError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<ReshardPlanError> for AutoscalerIntentError {
    fn from(error: ReshardPlanError) -> Self {
        Self::new(error.to_string())
    }
}

impl fmt::Display for AutoscalerIntentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AutoscalerIntentError {}

/// Bounded metric snapshot for capacity/autoscaler surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityAutoscalerMetrics {
    /// Last bounded recommendation label.
    pub capacity_recommendation: ScaleRecommendation,
    /// Total accepted scale actions.
    pub scale_actions_total: u64,
}
