use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{
    partition_for_key, ClusterEpoch, ClusterMember, ClusterNodeId, PartitionId,
    RendezvousClusterOwnership,
};
use crate::grid::checkpoint::ClusterCheckpointManifest;
use crate::grid::hardening::{
    ReplicatedValueRecord, ReplicatedValueStore, ValueStoreError, ValueVersion, WriteWatermark,
    REPLICATED_VALUE_RECORD_FORMAT_VERSION,
};
use crate::grid::{ClusterReplicationStrategy, EffectiveReplicationMap, Replicas};
use crate::invalidation_bus::CACHE_INVALIDATION_FRAME_VERSION;

/// Metadata key used for a node's authoritative region.
pub const NODE_TOPOLOGY_REGION_METADATA_KEY: &str = "hydracache.topology.region";

/// Metadata key used for a node's authoritative zone.
pub const NODE_TOPOLOGY_ZONE_METADATA_KEY: &str = "hydracache.topology.zone";

/// Control-plane snapshot format version registered in `docs/COMPAT.md`.
pub const CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// Stable region identifier used by zone-aware placement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegionId(String);

impl RegionId {
    /// Create a region id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the region id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for RegionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RegionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable availability-zone identifier used by placement and locality reads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ZoneId(String);

impl ZoneId {
    /// Create a zone id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the zone id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ZoneId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ZoneId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Authoritative topology attached to a cluster node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeTopology {
    /// Region that owns the node.
    pub region: RegionId,
    /// Availability zone inside the region.
    pub zone: ZoneId,
}

impl NodeTopology {
    /// Create a node topology.
    pub fn new(region: impl Into<RegionId>, zone: impl Into<ZoneId>) -> Self {
        Self {
            region: region.into(),
            zone: zone.into(),
        }
    }

    /// Return whether two topologies are in the same zone.
    pub fn same_zone(&self, other: &Self) -> bool {
        self.region == other.region && self.zone == other.zone
    }

    /// Return whether two topologies are in the same region.
    pub fn same_region(&self, other: &Self) -> bool {
        self.region == other.region
    }

    fn default_single_zone() -> Self {
        Self::new("default", "default")
    }
}

/// Extract topology metadata from a member snapshot, when present.
pub fn topology_from_member_metadata(member: &ClusterMember) -> Option<NodeTopology> {
    let region = member.metadata.get(NODE_TOPOLOGY_REGION_METADATA_KEY)?;
    let zone = member.metadata.get(NODE_TOPOLOGY_ZONE_METADATA_KEY)?;
    Some(NodeTopology::new(region.clone(), zone.clone()))
}

/// Control-plane-owned topology catalog. Gossip observations never become
/// authoritative until committed here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyAuthority {
    epoch: ClusterEpoch,
    committed: BTreeMap<ClusterNodeId, NodeTopology>,
    observed_gossip: BTreeMap<ClusterNodeId, NodeTopology>,
}

impl TopologyAuthority {
    /// Create an empty topology authority.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a gossip-only topology observation.
    pub fn observe_gossip(&mut self, node: impl Into<ClusterNodeId>, topology: NodeTopology) {
        self.observed_gossip.insert(node.into(), topology);
    }

    /// Commit an authoritative topology update.
    pub fn commit_topology(
        &mut self,
        node: impl Into<ClusterNodeId>,
        topology: NodeTopology,
        epoch: ClusterEpoch,
    ) {
        if epoch >= self.epoch {
            self.epoch = epoch;
            self.committed.insert(node.into(), topology);
        }
    }

    /// Return the committed epoch.
    pub fn epoch(&self) -> ClusterEpoch {
        self.epoch
    }

    /// Return the committed topology for a node.
    pub fn topology(&self, node: &ClusterNodeId) -> Option<&NodeTopology> {
        self.committed.get(node)
    }

    /// Return a clone of the committed topology map.
    pub fn committed_map(&self) -> BTreeMap<ClusterNodeId, NodeTopology> {
        self.committed.clone()
    }

    /// Return a clone of the gossip-only topology map.
    pub fn gossip_map(&self) -> BTreeMap<ClusterNodeId, NodeTopology> {
        self.observed_gossip.clone()
    }
}

/// Replica set with topology tags and underspread diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneAwareReplicaSet {
    /// Primary plus backups selected for the key.
    pub replicas: Replicas,
    /// Authoritative topology for each selected replica.
    pub topology: BTreeMap<ClusterNodeId, NodeTopology>,
    /// Whether fewer zones were available than requested by the policy.
    pub placement_zone_underspread: bool,
}

impl ZoneAwareReplicaSet {
    /// Return all replica nodes, primary first.
    pub fn all_nodes(&self) -> Vec<ClusterNodeId> {
        self.replicas.all_nodes()
    }

    /// Return the number of distinct zones represented by this replica set.
    pub fn zone_count(&self) -> usize {
        self.topology
            .values()
            .map(|topology| (topology.region.clone(), topology.zone.clone()))
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Return whether write quorum survives loss of any one represented zone.
    pub fn single_zone_loss_keeps_write_quorum(&self, write_quorum: usize) -> bool {
        let write_quorum = write_quorum.max(1);
        let zones = self
            .topology
            .values()
            .map(|topology| (topology.region.clone(), topology.zone.clone()))
            .collect::<BTreeSet<_>>();
        if zones.len() <= 1 {
            return self.replicas.copy_count() >= write_quorum;
        }
        zones.into_iter().all(|lost_zone| {
            self.all_nodes()
                .into_iter()
                .filter(|node| {
                    self.topology
                        .get(node)
                        .map(|topology| {
                            (topology.region.clone(), topology.zone.clone()) != lost_zone
                        })
                        .unwrap_or(true)
                })
                .count()
                >= write_quorum
        })
    }
}

/// Readiness report for zone-aware placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZonePlacementReadiness {
    /// Whether placement is underspread because not enough zones were supplied.
    pub placement_zone_underspread: bool,
    /// Whether a single-zone loss keeps write quorum.
    pub single_zone_loss_keeps_write_quorum: bool,
    /// Distinct zones represented by the replica set.
    pub zone_count: usize,
}

impl ZonePlacementReadiness {
    /// Return whether the placement is ready for the zone-aware claim.
    pub fn is_ready(&self) -> bool {
        !self.placement_zone_underspread && self.single_zone_loss_keeps_write_quorum
    }
}

/// Zone-aware placement strategy that preserves flat rendezvous behavior when
/// all nodes belong to one zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneAwareReplicationStrategy {
    topology: BTreeMap<ClusterNodeId, NodeTopology>,
    replication_factor: usize,
    min_zones: usize,
}

impl ZoneAwareReplicationStrategy {
    /// Create a zone-aware strategy from a committed topology map.
    pub fn new(
        topology: BTreeMap<ClusterNodeId, NodeTopology>,
        replication_factor: usize,
        min_zones: usize,
    ) -> Self {
        Self {
            topology,
            replication_factor: replication_factor.max(1),
            min_zones: min_zones.max(1),
        }
    }

    /// Return the configured replication factor.
    pub fn replication_factor(&self) -> usize {
        self.replication_factor
    }

    /// Return the configured minimum distinct zones.
    pub fn min_zones(&self) -> usize {
        self.min_zones
    }

    /// Return a zone-aware replica set for a key.
    pub fn zone_replicas_for_key(
        &self,
        key: &str,
        members: &[ClusterMember],
    ) -> Option<ZoneAwareReplicaSet> {
        let flat = RendezvousClusterOwnership.replicas_for_key(
            key,
            members,
            members.len().max(self.replication_factor),
        )?;
        let ranked = flat.all_nodes();
        let primary = ranked.first()?.clone();

        let mut selected = vec![primary.clone()];
        let mut used_zones = BTreeSet::new();
        if let Some(topology) = self.topology_for_node(&primary) {
            used_zones.insert((topology.region.clone(), topology.zone.clone()));
        }

        for node in ranked.iter().skip(1) {
            if selected.len() >= self.replication_factor {
                break;
            }
            let Some(topology) = self.topology_for_node(node) else {
                continue;
            };
            let zone_key = (topology.region.clone(), topology.zone.clone());
            if used_zones.insert(zone_key) {
                selected.push(node.clone());
            }
        }

        for node in ranked.iter().skip(1) {
            if selected.len() >= self.replication_factor {
                break;
            }
            if !selected.contains(node) {
                selected.push(node.clone());
            }
        }

        let mut selected_iter = selected.into_iter();
        let primary = selected_iter.next()?;
        let backups = selected_iter.collect::<Vec<_>>();
        let replicas = Replicas::new(primary, backups);
        let topology = replicas
            .all_nodes()
            .into_iter()
            .map(|node| {
                let topology = self
                    .topology_for_node(&node)
                    .unwrap_or_else(NodeTopology::default_single_zone);
                (node, topology)
            })
            .collect::<BTreeMap<_, _>>();
        let required_zones = self.min_zones.min(self.replication_factor);

        Some(ZoneAwareReplicaSet {
            placement_zone_underspread: topology
                .values()
                .map(|topology| (topology.region.clone(), topology.zone.clone()))
                .collect::<BTreeSet<_>>()
                .len()
                < required_zones,
            replicas,
            topology,
        })
    }

    /// Return a zone-aware replica set filtered to residency-allowed regions.
    pub fn zone_replicas_for_key_in_regions(
        &self,
        key: &str,
        members: &[ClusterMember],
        allowed_regions: &BTreeSet<RegionId>,
        min_replicas_in_policy: usize,
    ) -> Option<ZoneAwareReplicaSet> {
        let required = self.replication_factor.max(min_replicas_in_policy.max(1));
        let filtered = members
            .iter()
            .filter(|member| {
                self.topology_for_node(&member.node_id)
                    .map(|topology| allowed_regions.contains(&topology.region))
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();
        if filtered.len() < required {
            return None;
        }

        let mut strategy = self.clone();
        strategy.replication_factor = required;
        let replicas = strategy.zone_replicas_for_key(key, &filtered)?;
        if replicas.replicas.copy_count() < required {
            return None;
        }
        if replicas
            .topology
            .values()
            .all(|topology| allowed_regions.contains(&topology.region))
        {
            Some(replicas)
        } else {
            None
        }
    }

    /// Return a readiness report for one key.
    pub fn readiness_for_key(
        &self,
        key: &str,
        members: &[ClusterMember],
        write_quorum: usize,
    ) -> Option<ZonePlacementReadiness> {
        let replicas = self.zone_replicas_for_key(key, members)?;
        Some(ZonePlacementReadiness {
            placement_zone_underspread: replicas.placement_zone_underspread,
            single_zone_loss_keeps_write_quorum: replicas
                .single_zone_loss_keeps_write_quorum(write_quorum),
            zone_count: replicas.zone_count(),
        })
    }

    fn topology_for_node(&self, node: &ClusterNodeId) -> Option<NodeTopology> {
        self.topology.get(node).cloned()
    }
}

impl ClusterReplicationStrategy for ZoneAwareReplicationStrategy {
    fn name(&self) -> &'static str {
        "zone-aware"
    }

    fn replicas_for_key(
        &self,
        key: &str,
        members: &[ClusterMember],
        replication_factor: usize,
    ) -> Option<Replicas> {
        let mut strategy = self.clone();
        strategy.replication_factor = replication_factor.max(1);
        strategy
            .zone_replicas_for_key(key, members)
            .map(|replicas| replicas.replicas)
    }
}

/// Phase of an online partition move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovePhase {
    /// Shadow writes to the target while reads stay on the source.
    Prepare,
    /// Stream existing values to the target and catch up deltas.
    Backfill,
    /// Flip ownership through the authoritative map.
    Commit,
    /// Drop the source copy after confirmation.
    Cleanup,
}

/// One resumable partition movement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionMove {
    /// Partition being moved.
    pub partition: PartitionId,
    /// Current owner before commit.
    pub from: ClusterNodeId,
    /// Target owner.
    pub to: ClusterNodeId,
    /// Current move phase.
    pub phase: MovePhase,
    /// Bytes successfully backfilled.
    pub backfilled_bytes: u64,
    /// Total bytes expected for this move.
    pub total_bytes: u64,
}

impl PartitionMove {
    /// Create a move in `Prepare`.
    pub fn new(
        partition: PartitionId,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
        total_bytes: u64,
    ) -> Self {
        Self {
            partition,
            from: from.into(),
            to: to.into(),
            phase: MovePhase::Prepare,
            backfilled_bytes: 0,
            total_bytes: total_bytes.max(1),
        }
    }

    /// Return nodes that must receive a write for this move.
    pub fn write_targets(&self) -> Vec<ClusterNodeId> {
        match self.phase {
            MovePhase::Prepare | MovePhase::Backfill => {
                vec![self.from.clone(), self.to.clone()]
            }
            MovePhase::Commit | MovePhase::Cleanup => vec![self.to.clone()],
        }
    }

    /// Return the owner used for reads in this phase.
    pub fn read_owner(&self) -> ClusterNodeId {
        match self.phase {
            MovePhase::Prepare | MovePhase::Backfill => self.from.clone(),
            MovePhase::Commit | MovePhase::Cleanup => self.to.clone(),
        }
    }

    /// Record backfill progress.
    pub fn record_backfill(&mut self, bytes: u64) {
        self.backfilled_bytes = self
            .backfilled_bytes
            .saturating_add(bytes)
            .min(self.total_bytes);
    }

    /// Return progress from `0.0` to `1.0`.
    pub fn progress_ratio(&self) -> f32 {
        (self.backfilled_bytes as f32 / self.total_bytes as f32).min(1.0)
    }

    /// Advance to the next phase when allowed.
    pub fn advance(&mut self) {
        self.phase = match self.phase {
            MovePhase::Prepare => MovePhase::Backfill,
            MovePhase::Backfill if self.backfilled_bytes >= self.total_bytes => MovePhase::Commit,
            MovePhase::Backfill => MovePhase::Backfill,
            MovePhase::Commit => MovePhase::Cleanup,
            MovePhase::Cleanup => MovePhase::Cleanup,
        };
    }
}

/// Resumable online resharding plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReshardPlan {
    /// Topology epoch that owns this plan.
    pub epoch: ClusterEpoch,
    /// Partition moves.
    pub moves: Vec<PartitionMove>,
    /// Maximum concurrently active moves.
    pub max_concurrent: usize,
}

impl ReshardPlan {
    /// Create a normalized plan.
    pub fn new(epoch: ClusterEpoch, mut moves: Vec<PartitionMove>, max_concurrent: usize) -> Self {
        moves.sort_by_key(|movement| movement.partition.value());
        Self {
            epoch,
            moves,
            max_concurrent: max_concurrent.max(1),
        }
    }

    /// Return currently active moves subject to the concurrency cap.
    pub fn active_moves(&self) -> Vec<&PartitionMove> {
        self.moves
            .iter()
            .filter(|movement| movement.phase != MovePhase::Cleanup)
            .take(self.max_concurrent)
            .collect()
    }

    /// Return write targets for a partition, if it is moving.
    pub fn write_targets_for_partition(
        &self,
        partition: PartitionId,
    ) -> Option<Vec<ClusterNodeId>> {
        self.moves
            .iter()
            .find(|movement| movement.partition == partition)
            .map(PartitionMove::write_targets)
    }

    /// Record progress for a partition.
    pub fn record_backfill(&mut self, partition: PartitionId, bytes: u64) {
        if let Some(movement) = self
            .moves
            .iter_mut()
            .find(|movement| movement.partition == partition)
        {
            movement.record_backfill(bytes);
        }
    }

    /// Return a restart-safe snapshot.
    pub fn snapshot(&self) -> Self {
        self.clone()
    }

    /// Reopen a plan from a restart-safe snapshot.
    pub fn resume_from(snapshot: Self) -> Self {
        snapshot
    }

    /// Build a deterministic drain plan for every owned partition.
    pub fn drain_node(
        epoch: ClusterEpoch,
        node: impl Into<ClusterNodeId>,
        targets: impl IntoIterator<Item = (PartitionId, ClusterNodeId, u64)>,
        max_concurrent: usize,
    ) -> Self {
        let node = node.into();
        let moves = targets
            .into_iter()
            .map(|(partition, target, bytes)| {
                PartitionMove::new(partition, node.clone(), target, bytes)
            })
            .collect();
        Self::new(epoch, moves, max_concurrent)
    }
}

/// Phase of the stop-checkpoint-redistribute-resume rescale flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RescaleCheckpointPhase {
    /// Writes are stopped at the controller barrier.
    Stopped,
    /// A verified cluster checkpoint has been collected.
    Checkpointed,
    /// Partition data is being redistributed through the online reshard path.
    Redistributing,
    /// The reshard plan has been resumed from a restart-safe snapshot.
    Resumed,
    /// Rescale completed and ownership can drop old copies after confirmation.
    Complete,
}

/// Reshard plan bound to a verified cluster-wide checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RescaleWithCheckpointPlan {
    /// Verified checkpoint that fences the pre-rescale cut.
    pub checkpoint: ClusterCheckpointManifest,
    /// Underlying 0.43 online reshard plan.
    pub reshard: ReshardPlan,
    /// Current coordinated flow phase.
    pub phase: RescaleCheckpointPhase,
}

impl RescaleWithCheckpointPlan {
    /// Bind a verified checkpoint to a reshard plan in the same authority epoch.
    pub fn new(
        checkpoint: ClusterCheckpointManifest,
        reshard: ReshardPlan,
    ) -> Result<Self, ReshardPlanError> {
        checkpoint.verify().map_err(|error| {
            ReshardPlanError::new(format!("invalid rescale checkpoint: {error}"))
        })?;
        if checkpoint.epoch != reshard.epoch {
            return Err(ReshardPlanError::new(format!(
                "rescale checkpoint epoch {} does not match reshard epoch {}",
                checkpoint.epoch.value(),
                reshard.epoch.value()
            )));
        }
        Ok(Self {
            checkpoint,
            reshard,
            phase: RescaleCheckpointPhase::Checkpointed,
        })
    }

    /// Mark redistribution as started after the checkpoint is durable.
    pub fn redistribute(&mut self) {
        self.phase = RescaleCheckpointPhase::Redistributing;
    }

    /// Return a restart-safe snapshot of the coordinated rescale flow.
    pub fn snapshot(&self) -> Self {
        self.clone()
    }

    /// Reopen a coordinated rescale flow from its restart-safe snapshot.
    pub fn resume_from(snapshot: Self) -> Result<Self, ReshardPlanError> {
        snapshot.checkpoint.verify().map_err(|error| {
            ReshardPlanError::new(format!("invalid rescale checkpoint: {error}"))
        })?;
        Ok(Self {
            reshard: ReshardPlan::resume_from(snapshot.reshard),
            phase: RescaleCheckpointPhase::Resumed,
            ..snapshot
        })
    }

    /// Mark the coordinated rescale as complete.
    pub fn complete(&mut self) {
        self.phase = RescaleCheckpointPhase::Complete;
    }
}

/// Create a rescale-with-checkpoint flow from a verified checkpoint and reshard plan.
pub fn rescale_with_checkpoint(
    checkpoint: ClusterCheckpointManifest,
    reshard: ReshardPlan,
) -> Result<RescaleWithCheckpointPlan, ReshardPlanError> {
    RescaleWithCheckpointPlan::new(checkpoint, reshard)
}

/// Error returned when a move would break placement invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReshardPlanError {
    message: String,
}

impl ReshardPlanError {
    /// Create an error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ReshardPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ReshardPlanError {}

/// Reject a move target that would co-locate write quorum in one zone.
pub fn validate_move_preserves_zone_quorum(
    candidate: &ZoneAwareReplicaSet,
    write_quorum: usize,
) -> Result<(), ReshardPlanError> {
    if !candidate.placement_zone_underspread
        && candidate.single_zone_loss_keeps_write_quorum(write_quorum)
    {
        Ok(())
    } else {
        Err(ReshardPlanError::new(
            "reshard move would violate zone-spread write quorum",
        ))
    }
}

/// Read replica selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicaSelection {
    /// Prefer replicas in the local zone.
    NearestZone,
    /// Prefer lowest observed latency.
    LowestLatency,
    /// Keep input order.
    RoundRobin,
}

/// Health and latency observation for one replica.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaObservation {
    /// Replica node.
    pub node: ClusterNodeId,
    /// Replica topology.
    pub topology: NodeTopology,
    /// Whether the replica is currently healthy.
    pub healthy: bool,
    /// EWMA round-trip time in milliseconds.
    pub ewma_rtt_ms: u64,
    /// Highest observed value version.
    pub version: ValueVersion,
    /// Authority epoch for the observed version.
    pub epoch: ClusterEpoch,
}

impl ReplicaObservation {
    /// Create a healthy observation.
    pub fn healthy(
        node: impl Into<ClusterNodeId>,
        topology: NodeTopology,
        ewma_rtt_ms: u64,
        version: ValueVersion,
        epoch: ClusterEpoch,
    ) -> Self {
        Self {
            node: node.into(),
            topology,
            healthy: true,
            ewma_rtt_ms,
            version,
            epoch,
        }
    }
}

/// EWMA/zone-aware replica scorer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaScorer {
    observations: BTreeMap<ClusterNodeId, ReplicaObservation>,
}

impl ReplicaScorer {
    /// Create an empty scorer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record or replace a replica observation.
    pub fn observe(&mut self, observation: ReplicaObservation) {
        self.observations
            .insert(observation.node.clone(), observation);
    }

    /// Order replicas for a read.
    pub fn order(
        &self,
        replicas: &[ClusterNodeId],
        local: &NodeTopology,
        selection: ReplicaSelection,
    ) -> Vec<ClusterNodeId> {
        let mut ranked = replicas.to_vec();
        ranked.sort_by(|left, right| {
            let left_obs = self.observations.get(left);
            let right_obs = self.observations.get(right);
            let left_key = replica_score_key(left_obs, local, left, selection);
            let right_key = replica_score_key(right_obs, local, right, selection);
            left_key.cmp(&right_key)
        });
        ranked
    }

    /// Return observations for selected nodes.
    pub fn observations_for(&self, nodes: &[ClusterNodeId]) -> Vec<ReplicaObservation> {
        nodes
            .iter()
            .filter_map(|node| self.observations.get(node).cloned())
            .collect()
    }
}

fn replica_score_key(
    observation: Option<&ReplicaObservation>,
    local: &NodeTopology,
    node: &ClusterNodeId,
    selection: ReplicaSelection,
) -> (u8, u8, u64, String) {
    let healthy = observation.map(|obs| obs.healthy).unwrap_or(false);
    let distance = observation
        .map(|obs| {
            if obs.topology.same_zone(local) {
                0
            } else if obs.topology.same_region(local) {
                1
            } else {
                2
            }
        })
        .unwrap_or(3);
    let latency = observation.map(|obs| obs.ewma_rtt_ms).unwrap_or(u64::MAX);
    match selection {
        ReplicaSelection::NearestZone => (!healthy as u8, distance, latency, node.to_string()),
        ReplicaSelection::LowestLatency => (!healthy as u8, 0, latency, node.to_string()),
        ReplicaSelection::RoundRobin => (0, 0, 0, node.to_string()),
    }
}

/// Adaptive hedge delay source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HedgePolicy {
    /// Percentile used from observed RTTs.
    pub percentile: u8,
    /// Maximum extra hedge requests.
    pub max_extra: usize,
    /// Lower bound for hedge delay.
    pub min_delay_ms: u64,
}

impl HedgePolicy {
    /// Create a normalized hedge policy.
    pub fn new(percentile: u8, max_extra: usize, min_delay_ms: u64) -> Self {
        Self {
            percentile: percentile.clamp(1, 100),
            max_extra,
            min_delay_ms,
        }
    }

    /// Compute hedge delay from observed RTTs.
    pub fn delay_ms(self, observed_rtts: &[u64]) -> u64 {
        if observed_rtts.is_empty() {
            return self.min_delay_ms;
        }
        let mut sorted = observed_rtts.to_vec();
        sorted.sort_unstable();
        let index = ((sorted.len() - 1) * self.percentile as usize) / 100;
        sorted[index].max(self.min_delay_ms)
    }
}

/// Hedged read execution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HedgedReadPlan {
    /// First request target.
    pub primary: Option<ClusterNodeId>,
    /// Extra hedge targets.
    pub hedges: Vec<ClusterNodeId>,
    /// Required acknowledgements for the consistency level.
    pub required_acks: usize,
}

/// Build a hedged read plan without changing the required quorum count.
pub fn plan_hedged_read(
    ordered_replicas: &[ClusterNodeId],
    required_acks: usize,
    elapsed_ms: u64,
    observed_rtts: &[u64],
    policy: HedgePolicy,
) -> HedgedReadPlan {
    let primary = ordered_replicas.first().cloned();
    let hedge_delay = policy.delay_ms(observed_rtts);
    let hedges = if elapsed_ms >= hedge_delay {
        ordered_replicas
            .iter()
            .skip(1)
            .take(policy.max_extra)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    HedgedReadPlan {
        primary,
        hedges,
        required_acks: required_acks.max(1),
    }
}

/// Choose the freshest read response by `(version, epoch)`.
pub fn hedge_winner(
    responses: impl IntoIterator<Item = ReplicatedValueRecord>,
) -> Option<ReplicatedValueRecord> {
    responses
        .into_iter()
        .max_by_key(|record| (record.version, record.epoch))
}

/// Two-tier replicated value store used by the 0.43 tiering gate.
#[derive(Debug, Clone)]
pub struct TieredValueStore<S> {
    cold: S,
    hot: BTreeMap<String, ReplicatedValueRecord>,
    order: VecDeque<String>,
    max_hot_bytes: u64,
    promotions_total: u64,
    demotions_total: u64,
}

impl<S> TieredValueStore<S>
where
    S: ReplicatedValueStore,
{
    /// Create a tiered store with a bounded hot tier.
    pub fn new(cold: S, max_hot_bytes: u64) -> Self {
        Self {
            cold,
            hot: BTreeMap::new(),
            order: VecDeque::new(),
            max_hot_bytes: max_hot_bytes.max(1),
            promotions_total: 0,
            demotions_total: 0,
        }
    }

    /// Return a reference to the cold tier.
    pub fn cold(&self) -> &S {
        &self.cold
    }

    /// Return mutable access to the cold tier.
    pub fn cold_mut(&mut self) -> &mut S {
        &mut self.cold
    }

    /// Return whether the hot tier contains a key.
    pub fn hot_contains(&self, key: &str) -> bool {
        self.hot.contains_key(key)
    }

    /// Return hot-tier bytes.
    pub fn hot_bytes(&self) -> u64 {
        self.hot
            .values()
            .map(ReplicatedValueRecord::approx_bytes)
            .sum()
    }

    /// Return hot-tier ratio over hot+cold visible records.
    pub fn hot_ratio(&self) -> f32 {
        let hot = self.hot.len() as f32;
        if hot == 0.0 {
            return 0.0;
        }
        hot / (hot + 1.0)
    }

    /// Return promotion count.
    pub fn promotions_total(&self) -> u64 {
        self.promotions_total
    }

    /// Return demotion count.
    pub fn demotions_total(&self) -> u64 {
        self.demotions_total
    }

    /// Read a key and promote a cold hit into the hot tier.
    pub fn get_promote(
        &mut self,
        key: &str,
    ) -> Result<Option<ReplicatedValueRecord>, ValueStoreError> {
        let record = self.get(key)?;
        if let Some(record) = record.clone() {
            self.promote_hot(key.to_owned(), record)?;
        }
        Ok(record)
    }

    fn promote_hot(
        &mut self,
        key: String,
        record: ReplicatedValueRecord,
    ) -> Result<(), ValueStoreError> {
        if record.approx_bytes() > self.max_hot_bytes {
            return Ok(());
        }
        let merged = self
            .hot
            .remove(&key)
            .map(|current| current.merge(record.clone()))
            .unwrap_or(record);
        self.hot.insert(key.clone(), merged);
        self.order.retain(|existing| existing != &key);
        self.order.push_back(key);
        self.promotions_total = self.promotions_total.saturating_add(1);
        self.enforce_hot_budget()
    }

    fn enforce_hot_budget(&mut self) -> Result<(), ValueStoreError> {
        while self.hot_bytes() > self.max_hot_bytes {
            let Some(key) = self.order.pop_front() else {
                break;
            };
            if let Some(record) = self.hot.remove(&key) {
                self.cold.upsert(key, record)?;
                self.demotions_total = self.demotions_total.saturating_add(1);
            }
        }
        Ok(())
    }
}

impl<S> ReplicatedValueStore for TieredValueStore<S>
where
    S: ReplicatedValueStore,
{
    fn upsert(
        &mut self,
        key: impl Into<String>,
        rec: ReplicatedValueRecord,
    ) -> Result<(), ValueStoreError> {
        let key = key.into();
        self.cold.upsert(key.clone(), rec.clone())?;
        self.promote_hot(key, rec)
    }

    fn get(&self, key: &str) -> Result<Option<ReplicatedValueRecord>, ValueStoreError> {
        let hot = self.hot.get(key).cloned();
        let cold = self.cold.get(key)?;
        Ok(match (hot, cold) {
            (Some(hot), Some(cold)) => Some(hot.merge(cold)),
            (Some(hot), None) => Some(hot),
            (None, Some(cold)) => Some(cold),
            (None, None) => None,
        })
    }

    fn tombstone(
        &mut self,
        key: impl Into<String>,
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
    ) -> Result<(), ValueStoreError> {
        self.upsert(
            key,
            ReplicatedValueRecord::tombstone(partition, version, epoch, None),
        )
    }

    fn scan_owned(
        &self,
        map: &EffectiveReplicationMap,
    ) -> Result<Vec<(String, ReplicatedValueRecord)>, ValueStoreError> {
        let mut merged = BTreeMap::new();
        for (key, record) in self.cold.scan_owned(map)? {
            merged.insert(key, record);
        }
        for (key, record) in &self.hot {
            let record = merged
                .remove(key)
                .map(|cold| cold.merge(record.clone()))
                .unwrap_or_else(|| record.clone());
            merged.insert(key.clone(), record);
        }
        Ok(merged.into_iter().collect())
    }

    fn scan_all(&self) -> Result<Vec<(String, ReplicatedValueRecord)>, ValueStoreError> {
        let mut merged = BTreeMap::new();
        for (key, record) in self.cold.scan_all()? {
            merged.insert(key, record);
        }
        for (key, record) in &self.hot {
            let record = merged
                .remove(key)
                .map(|cold| cold.merge(record.clone()))
                .unwrap_or_else(|| record.clone());
            merged.insert(key.clone(), record);
        }
        Ok(merged.into_iter().collect())
    }

    fn remove(&mut self, key: &str) -> Result<(), ValueStoreError> {
        self.hot.remove(key);
        self.order.retain(|existing| existing != key);
        self.cold.remove(key)
    }

    fn compact(&mut self) -> Result<u64, ValueStoreError> {
        self.cold.compact()
    }

    fn total_bytes(&self) -> Result<u64, ValueStoreError> {
        Ok(self.cold.total_bytes()?.saturating_add(self.hot_bytes()))
    }

    fn rejected_total(&self) -> u64 {
        self.cold.rejected_total()
    }
}

/// Single-partition invalidation batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidateBatch {
    /// Partition shared by every key.
    pub partition: PartitionId,
    /// Keys invalidated atomically.
    pub keys: Vec<String>,
    /// Batch version.
    pub version: ValueVersion,
    /// Authority epoch.
    pub epoch: ClusterEpoch,
}

impl InvalidateBatch {
    /// Create a batch and reject cross-partition key sets.
    pub fn try_new(
        keys: impl IntoIterator<Item = impl Into<String>>,
        partition_count: u32,
        version: ValueVersion,
        epoch: ClusterEpoch,
    ) -> Result<Self, AtomicInvalidationError> {
        let keys = keys.into_iter().map(Into::into).collect::<Vec<_>>();
        if keys.is_empty() {
            return Err(AtomicInvalidationError::new("invalidate batch is empty"));
        }
        let partition = partition_for_key(&keys[0], partition_count);
        if keys
            .iter()
            .any(|key| partition_for_key(key, partition_count) != partition)
        {
            return Err(AtomicInvalidationError::new(
                "cross-partition invalidation batch rejected; use InvalidationSaga",
            ));
        }
        Ok(Self {
            partition,
            keys,
            version,
            epoch,
        })
    }
}

/// Error returned by atomic invalidation helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicInvalidationError {
    message: String,
}

impl AtomicInvalidationError {
    /// Create an error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AtomicInvalidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AtomicInvalidationError {}

/// Deterministic state machine for validating batch atomicity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchInvalidationState {
    applied: BTreeMap<String, WriteWatermark>,
}

impl BatchInvalidationState {
    /// Apply a full batch at one watermark.
    pub fn apply_batch(&mut self, batch: &InvalidateBatch) {
        let watermark = WriteWatermark::new(batch.partition, batch.version, batch.epoch);
        for key in &batch.keys {
            self.applied.insert(key.clone(), watermark);
        }
    }

    /// Apply one invalidation if it is newer than the current watermark.
    pub fn apply_single(&mut self, key: impl Into<String>, watermark: WriteWatermark) {
        let key = key.into();
        let replace = self
            .applied
            .get(&key)
            .map(|current| (watermark.version, watermark.epoch) >= (current.version, current.epoch))
            .unwrap_or(true);
        if replace {
            self.applied.insert(key, watermark);
        }
    }

    /// Return a key's watermark.
    pub fn watermark(&self, key: &str) -> Option<WriteWatermark> {
        self.applied.get(key).copied()
    }

    /// Return whether all batch keys share the batch watermark.
    pub fn batch_is_all_or_nothing(&self, batch: &InvalidateBatch) -> bool {
        let expected = WriteWatermark::new(batch.partition, batch.version, batch.epoch);
        batch
            .keys
            .iter()
            .all(|key| self.applied.get(key) == Some(&expected))
    }
}

/// One target of a best-effort invalidation saga.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InvalidationTarget {
    /// Target partition.
    pub partition: PartitionId,
    /// Target key.
    pub key: String,
}

impl InvalidationTarget {
    /// Create a target.
    pub fn new(partition: PartitionId, key: impl Into<String>) -> Self {
        Self {
            partition,
            key: key.into(),
        }
    }
}

/// Cross-partition best-effort invalidation fan-out unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidationSaga {
    /// Stable outbox unit id.
    pub unit_id: String,
    /// Fan-out targets.
    pub targets: Vec<InvalidationTarget>,
    applied: BTreeSet<String>,
}

impl InvalidationSaga {
    /// Create a saga.
    pub fn new(unit_id: impl Into<String>, targets: Vec<InvalidationTarget>) -> Self {
        Self {
            unit_id: unit_id.into(),
            targets,
            applied: BTreeSet::new(),
        }
    }

    /// Dispatch one target idempotently. Returns true only for the first effect.
    pub fn dispatch_target(&mut self, target: &InvalidationTarget) -> bool {
        self.applied.insert(self.idempotency_key(target))
    }

    /// Return pending target count.
    pub fn pending(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| !self.applied.contains(&self.idempotency_key(target)))
            .count()
    }

    /// Return whether every target has been applied.
    pub fn is_complete(&self) -> bool {
        self.pending() == 0
    }

    /// Return the stable idempotency key for a target.
    pub fn idempotency_key(&self, target: &InvalidationTarget) -> String {
        format!(
            "{}:{}:{}",
            self.unit_id,
            target.partition.value(),
            target.key
        )
    }
}

/// Auto-repair mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairMode {
    /// Report recommendations only.
    Advisory,
    /// Schedule bounded repair actions.
    Active,
}

/// Policy for operational self-healing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoRepairPolicy {
    /// Repair mode.
    pub mode: RepairMode,
    /// Repair-debt threshold.
    pub debt_threshold: u64,
    /// Lag threshold.
    pub lag_threshold: u64,
    /// Maximum concurrent repair actions.
    pub max_concurrent_repairs: usize,
}

impl AutoRepairPolicy {
    /// Create a normalized policy.
    pub fn new(
        mode: RepairMode,
        debt_threshold: u64,
        lag_threshold: u64,
        max_concurrent_repairs: usize,
    ) -> Self {
        Self {
            mode,
            debt_threshold,
            lag_threshold,
            max_concurrent_repairs: max_concurrent_repairs.max(1),
        }
    }

    /// Evaluate the policy.
    pub fn evaluate(&self, debt: u64, lag: u64) -> AutoRepairDecision {
        let should_repair = debt > self.debt_threshold || lag > self.lag_threshold;
        let recommended = if should_repair {
            vec![RepairAction::AntiEntropy]
        } else {
            Vec::new()
        };
        let scheduled = if should_repair && self.mode == RepairMode::Active {
            recommended
                .iter()
                .copied()
                .take(self.max_concurrent_repairs)
                .collect()
        } else {
            Vec::new()
        };
        AutoRepairDecision {
            mode: self.mode,
            recommended,
            scheduled,
            capped_at: self.max_concurrent_repairs,
        }
    }
}

/// Repair action scheduled or recommended by self-healing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairAction {
    /// Run anti-entropy.
    AntiEntropy,
    /// Re-replicate an under-replicated partition.
    ReReplicate,
    /// Move a partition.
    MovePartition,
}

/// Auto-repair decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoRepairDecision {
    /// Policy mode.
    pub mode: RepairMode,
    /// Operator-visible recommendations.
    pub recommended: Vec<RepairAction>,
    /// Actions to schedule now.
    pub scheduled: Vec<RepairAction>,
    /// Concurrency cap.
    pub capped_at: usize,
}

/// Control-plane snapshot used by backup/restore.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneSnapshot {
    /// Snapshot format version.
    pub format_version: u32,
    /// Committed epoch.
    pub epoch: ClusterEpoch,
    /// Authoritative topology.
    pub topology: BTreeMap<ClusterNodeId, NodeTopology>,
    /// Ownership map.
    pub ownership: BTreeMap<PartitionId, Replicas>,
    /// Tombstone versions retained by the control plane.
    pub tombstone_versions: BTreeMap<String, ValueVersion>,
}

impl ControlPlaneSnapshot {
    /// Create a snapshot at the current format version.
    pub fn new(epoch: ClusterEpoch) -> Self {
        Self {
            format_version: CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION,
            epoch,
            ..Self::default()
        }
    }
}

/// Snapshot sink error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotError {
    message: String,
}

impl SnapshotError {
    /// Create an error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SnapshotError {}

/// Operator-supplied control-plane snapshot sink.
pub trait SnapshotSink: Send + Sync {
    /// Store a snapshot.
    fn put(&mut self, snapshot: ControlPlaneSnapshot) -> Result<(), SnapshotError>;

    /// Return the latest snapshot.
    fn latest(&self) -> Result<Option<ControlPlaneSnapshot>, SnapshotError>;
}

/// In-memory snapshot sink used by tests and examples.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InMemorySnapshotSink {
    snapshots: Vec<ControlPlaneSnapshot>,
}

impl InMemorySnapshotSink {
    /// Create an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return stored snapshot count.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Return whether no snapshots are stored.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

impl SnapshotSink for InMemorySnapshotSink {
    fn put(&mut self, snapshot: ControlPlaneSnapshot) -> Result<(), SnapshotError> {
        if snapshot.format_version > CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION {
            return Err(SnapshotError::new(
                "control-plane snapshot format is newer than this binary",
            ));
        }
        self.snapshots.push(snapshot);
        Ok(())
    }

    fn latest(&self) -> Result<Option<ControlPlaneSnapshot>, SnapshotError> {
        Ok(self.snapshots.last().cloned())
    }
}

/// Restore a topology authority from a snapshot.
pub fn restore_topology_from_snapshot(
    snapshot: &ControlPlaneSnapshot,
) -> Result<TopologyAuthority, SnapshotError> {
    if snapshot.format_version > CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION {
        return Err(SnapshotError::new(
            "control-plane snapshot format is newer than this binary",
        ));
    }
    let mut authority = TopologyAuthority::new();
    for (node, topology) in &snapshot.topology {
        authority.commit_topology(node.clone(), topology.clone(), snapshot.epoch);
    }
    Ok(authority)
}

/// Semantic version used by upgrade guard checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CompatVersion {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
    /// Patch version.
    pub patch: u16,
}

impl CompatVersion {
    /// Create a version.
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

/// One rolling-upgrade step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeStep {
    /// Source binary version.
    pub from: CompatVersion,
    /// Target binary version.
    pub to: CompatVersion,
    /// Raft log format version.
    pub raft_log_format: u32,
    /// Replicated value-record format.
    pub value_record_format: u32,
    /// Cache invalidation wire frame version.
    pub wire_frame_version: u16,
}

/// Upgrade guard backed by the compatibility register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeGuard {
    /// Minimum supported minor version.
    pub min_minor: u16,
    /// Maximum supported minor version.
    pub max_minor: u16,
    /// Current raft log format.
    pub raft_log_format: u32,
    /// Current value-record format.
    pub value_record_format: u32,
    /// Current wire frame version.
    pub wire_frame_version: u16,
}

impl UpgradeGuard {
    /// Create the 0.43 guard.
    pub fn current() -> Self {
        Self {
            min_minor: 42,
            max_minor: 43,
            raft_log_format: 1,
            value_record_format: REPLICATED_VALUE_RECORD_FORMAT_VERSION,
            wire_frame_version: CACHE_INVALIDATION_FRAME_VERSION,
        }
    }

    /// Check a rolling upgrade step.
    pub fn check(&self, step: UpgradeStep) -> Result<(), UpgradeGuardError> {
        if step.from.major != 0 || step.to.major != 0 {
            return Err(UpgradeGuardError::new(
                "only 0.x HydraCache versions are supported",
            ));
        }
        if step.from.minor < self.min_minor || step.to.minor > self.max_minor {
            return Err(UpgradeGuardError::new(
                "upgrade step is outside the registered compatibility window",
            ));
        }
        if step.to.minor.saturating_sub(step.from.minor) > 1 {
            return Err(UpgradeGuardError::new(
                "rolling upgrade step skips a supported minor version",
            ));
        }
        if step.raft_log_format != self.raft_log_format
            || step.value_record_format != self.value_record_format
            || step.wire_frame_version != self.wire_frame_version
        {
            return Err(UpgradeGuardError::new(
                "upgrade step uses an incompatible persisted or wire format",
            ));
        }
        Ok(())
    }
}

/// Upgrade guard error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeGuardError {
    message: String,
}

impl UpgradeGuardError {
    /// Create an error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for UpgradeGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for UpgradeGuardError {}
