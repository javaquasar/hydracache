//! How a manual write travels through the simulated cluster (TD-0016 step 3).
//!
//! The data plane is still the product's sans-IO `ClusterNode` and its
//! `ClusterNodeMessage`s; this module only decides *where* those messages go.
//! Placement and acknowledgement rules are the product's own
//! (`hydracache::rank_replicas`, `hydracache::ConsistencyLevel`), built for every
//! target, so the browser runs the same rules as native.
//!
//! The product ships the placement and consistency primitives but no runtime
//! path that wires them into a networked write yet. The `Replicated` model is
//! therefore a model of those primitives, not of shipped behaviour, and the
//! demo labels it so (see `docs/architecture/RAFT_CONTROL_PLANE.md`).

use std::collections::BTreeSet;

use hydracache::{ClusterNodeId, ConsistencyLevel, Replicas};
use serde::{Deserialize, Serialize};

/// Sequence numbers of grid-routed messages start here, far above the
/// per-node sequences `ClusterNode` assigns, so the world can recognise the
/// acknowledgements that belong to a routed write.
pub(crate) const GRID_SEQUENCE_BASE: u64 = 1 << 40;

/// Number of finished writes the snapshot keeps for display.
pub(crate) const MAX_WRITES_RENDERED: usize = 16;

/// Hints one primary may hold for one target before it drops the newest and
/// leaves the key to repair (the product `HintBudget` caps count and bytes).
pub(crate) const MAX_HINTS_PER_TARGET: usize = 32;

/// Steps between anti-entropy passes that copy a key's latest routed value to
/// a live owner that lacks it.
pub(crate) const REPAIR_INTERVAL_STEPS: u64 = 5;

/// A copy the primary could not send because the backup was down or cut off.
/// Mirrors the product `Hint` semantics: held by the sender, replayed when the
/// target is reachable again, suppressed when the target already applied a
/// newer write of the key, lost with its holder (the product store is in
/// memory), and dropped over budget with the key left to repair.
#[derive(Debug, Clone)]
pub(crate) struct SimHint {
    pub(crate) write: u64,
    pub(crate) key: String,
    pub(crate) value: Vec<u8>,
    pub(crate) holder: ClusterNodeId,
    pub(crate) target: ClusterNodeId,
    pub(crate) created_at_step: u64,
}

impl SimHint {
    pub(crate) fn sequence(&self) -> u64 {
        GRID_SEQUENCE_BASE + self.write
    }

    pub(crate) fn view(&self) -> crate::snapshot::HintView {
        crate::snapshot::HintView {
            write: self.write,
            key: self.key.clone(),
            holder: self.holder.to_string(),
            target: self.target.to_string(),
            created_at_step: self.created_at_step,
        }
    }
}

/// Counters for hinted handoff and repair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RepairStats {
    pub(crate) hints_stored: u64,
    pub(crate) hints_replayed: u64,
    pub(crate) hints_suppressed: u64,
    pub(crate) hints_dropped: u64,
    pub(crate) repair_copies: u64,
    pub(crate) last_repair_step: Option<u64>,
}

/// Which write path a manual push follows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SimReplicationModel {
    /// Legacy simulator model: the client's member writes and broadcasts the
    /// value to every peer (`ClusterNode::handle_client`). The built-in
    /// workload and the deterministic-simulation invariants run on this model.
    #[default]
    FullBroadcast,
    /// Product default: the client's member writes locally and every other
    /// member drops its copy (an invalidation). Values are not replicated.
    LocalFirst,
    /// Opt-in replication primitives: the client's member (the coordinator)
    /// forwards to the key's rendezvous primary, which replicates to its
    /// backups; the write is acknowledged when `write_level` is met.
    Replicated {
        /// Copies including the primary.
        replication_factor: u64,
        /// Acknowledgement level for writes.
        write_level: ConsistencyLevel,
    },
}

impl SimReplicationModel {
    /// Stable snapshot name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FullBroadcast => "full_broadcast",
            Self::LocalFirst => "local_first",
            Self::Replicated { .. } => "replicated",
        }
    }

    /// Whether manual writes are routed by this module rather than broadcast.
    pub(crate) fn is_routed(self) -> bool {
        !matches!(self, Self::FullBroadcast)
    }
}

/// Lifecycle of a routed write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GridWriteStatus {
    /// Waiting for acknowledgements.
    Pending,
    /// The required acknowledgements arrived.
    Acknowledged,
    /// The level can no longer be met by the live replicas.
    Failed(String),
}

impl GridWriteStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Acknowledged => "acknowledged",
            Self::Failed(_) => "failed",
        }
    }
}

/// One routed manual write and what the coordinator knows about it.
#[derive(Debug, Clone)]
pub(crate) struct GridWrite {
    pub(crate) id: u64,
    pub(crate) client: String,
    pub(crate) key: String,
    pub(crate) value: Vec<u8>,
    /// Member the client is connected to; it answers the client.
    pub(crate) coordinator: ClusterNodeId,
    /// Owners of the key; for `LocalFirst` the coordinator alone.
    pub(crate) replicas: Replicas,
    /// `None` for `LocalFirst`, which waits only for its local write.
    pub(crate) level: Option<ConsistencyLevel>,
    pub(crate) required_acks: u64,
    pub(crate) acks: u64,
    /// Backups whose copy the primary has seen acknowledged.
    pub(crate) backups_applied: BTreeSet<ClusterNodeId>,
    /// Backups the primary holds a hint for instead of a sent copy.
    pub(crate) hinted: BTreeSet<ClusterNodeId>,
    pub(crate) status: GridWriteStatus,
    pub(crate) started_at_step: u64,
    pub(crate) acknowledged_at_step: Option<u64>,
}

impl GridWrite {
    pub(crate) fn sequence(&self) -> u64 {
        GRID_SEQUENCE_BASE + self.id
    }

    /// Count one acknowledgement at the coordinator.
    pub(crate) fn acknowledge(&mut self, step: u64) {
        if self.status != GridWriteStatus::Pending {
            return;
        }
        self.acks = self.acks.saturating_add(1);
        if self.acks >= self.required_acks {
            self.status = GridWriteStatus::Acknowledged;
            self.acknowledged_at_step = Some(step);
        }
    }

    pub(crate) fn view(&self) -> crate::snapshot::GridWriteView {
        crate::snapshot::GridWriteView {
            id: self.id,
            client: self.client.clone(),
            key: self.key.clone(),
            coordinator: self.coordinator.to_string(),
            primary: self.replicas.primary.to_string(),
            backups: self
                .replicas
                .backups
                .iter()
                .map(ToString::to_string)
                .collect(),
            backups_applied: self
                .backups_applied
                .iter()
                .map(ToString::to_string)
                .collect(),
            hinted: self.hinted.iter().map(ToString::to_string).collect(),
            level: self.level.map(level_name).map(str::to_owned),
            required_acks: self.required_acks,
            acks: self.acks,
            status: self.status.as_str().to_owned(),
            reason: match &self.status {
                GridWriteStatus::Failed(reason) => Some(reason.clone()),
                _ => None,
            },
            started_at_step: self.started_at_step,
            acknowledged_at_step: self.acknowledged_at_step,
        }
    }
}

/// Write id carried by a grid sequence number, if it is one.
pub(crate) fn grid_write_id(sequence: u64) -> Option<u64> {
    sequence.checked_sub(GRID_SEQUENCE_BASE)
}

/// Product consistency levels in the order the demo lists them.
pub(crate) const ALL_LEVELS: [ConsistencyLevel; 5] = [
    ConsistencyLevel::One,
    ConsistencyLevel::LocalQuorum,
    ConsistencyLevel::Quorum,
    ConsistencyLevel::EachQuorum,
    ConsistencyLevel::All,
];

/// The level with snapshot name `name` (`one`, `quorum`, ...).
pub fn level_from_name(name: &str) -> Option<ConsistencyLevel> {
    ALL_LEVELS
        .into_iter()
        .find(|level| level_name(*level) == name)
}

/// Snapshot name of a level. Exhaustive on purpose: a new product level stops
/// the simulator from compiling until it is named here.
pub(crate) fn level_name(level: ConsistencyLevel) -> &'static str {
    match level {
        ConsistencyLevel::One => "one",
        ConsistencyLevel::LocalQuorum => "local_quorum",
        ConsistencyLevel::Quorum => "quorum",
        ConsistencyLevel::EachQuorum => "each_quorum",
        ConsistencyLevel::All => "all",
    }
}
