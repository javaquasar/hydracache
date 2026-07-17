//! Honest cluster-status seam for the admin and Management Center surfaces.

use std::fmt;
use std::sync::Arc;

use hydracache::{ClusterMember, ClusterNodeId, ClusterRole, RaftMetadataSnapshot};
use serde::Serialize;
use thiserror::Error;

/// Runtime state supplied by the server around the cluster-status provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterStatusRuntime {
    /// Whether the server runtime is ready to serve.
    pub ready: bool,
    /// Whether the server runtime is draining.
    pub draining: bool,
}

impl ClusterStatusRuntime {
    /// Build a runtime status input.
    pub fn new(ready: bool, draining: bool) -> Self {
        Self { ready, draining }
    }
}

/// Where a status reading came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusSource {
    /// Status came from a live grid/control-plane handle.
    Live,
    /// Status is the local daemon model and must not be painted as live.
    Modeled,
}

/// Role rendered for a cluster member in management views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    /// Local, non-clustered runtime.
    Local,
    /// Client near-cache runtime.
    Client,
    /// Voting/cache member runtime.
    Member,
}

impl From<ClusterRole> for MemberRole {
    fn from(value: ClusterRole) -> Self {
        match value {
            ClusterRole::Local => Self::Local,
            ClusterRole::Client => Self::Client,
            ClusterRole::Member => Self::Member,
        }
    }
}

/// Reachability state reported for a known member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reachability {
    /// Member is currently reachable.
    Reachable,
    /// Member is suspected but not yet declared unreachable.
    Suspect,
    /// Member is unreachable and must remain visible in the view.
    Unreachable,
}

/// Coarse reshard lifecycle phase for read-only status surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReshardPhase {
    /// No reshard is active.
    Idle,
    /// A reshard is being planned.
    Planning,
    /// Partitions or replicas are moving.
    Moving,
    /// The reshard is finalizing.
    Finalizing,
}

impl fmt::Display for ReshardPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Idle => "idle",
            Self::Planning => "planning",
            Self::Moving => "moving",
            Self::Finalizing => "finalizing",
        };
        formatter.write_str(value)
    }
}

/// Read-only member status shown by the Management Center.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemberStatus {
    /// Stable logical node id.
    pub node_id: String,
    /// Runtime role.
    pub role: MemberRole,
    /// Current reachability.
    pub reachable: Reachability,
    /// Process generation.
    pub generation: u64,
}

/// Read-only cluster status snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClusterStatus {
    /// Whether the snapshot is live or modeled.
    pub source: StatusSource,
    /// Current leader id, if a live raft source knows one.
    pub leader: Option<String>,
    /// Current control-plane term.
    pub term: u64,
    /// Current authority epoch.
    pub epoch: u64,
    /// Whether quorum is available while the runtime is not draining.
    pub quorum_ok: bool,
    /// Visible members. Unreachable members remain present.
    pub members: Vec<MemberStatus>,
    /// Current raft voter count, if known.
    pub voters: u32,
    /// Current reshard phase.
    pub reshard_phase: ReshardPhase,
    /// Whether the runtime is draining.
    pub draining: bool,
}

/// Read-only state of the disk-backed Raft compaction control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RaftCompactionStatus {
    /// Whether the current runtime owns a disk-backed Raft log.
    pub available: bool,
    /// Whether explicit compaction requests are enabled by configuration.
    pub enabled: bool,
    /// Last locally applied Raft index, when available.
    pub applied_index: Option<u64>,
    /// Durable snapshot index, when available.
    pub snapshot_index: Option<u64>,
    /// First retained durable log index, when available.
    pub first_log_index: Option<u64>,
    /// Last durable log index, when available.
    pub last_log_index: Option<u64>,
    /// Real HTTP snapshot send attempts since this daemon started.
    pub snapshot_send_attempts: Option<u64>,
    /// Successful real HTTP snapshot sends since this daemon started.
    pub snapshot_send_successes: Option<u64>,
    /// Failed or timed-out real HTTP snapshot sends since this daemon started.
    pub snapshot_send_failures: Option<u64>,
    /// Real HTTP snapshot requests currently awaiting an outcome.
    pub snapshot_sends_in_flight: Option<u64>,
    /// Snapshots installed into the local Raft state machine this process run.
    pub snapshot_installs: Option<u64>,
}

impl RaftCompactionStatus {
    pub(crate) fn unavailable() -> Self {
        Self {
            available: false,
            enabled: false,
            applied_index: None,
            snapshot_index: None,
            first_log_index: None,
            last_log_index: None,
            snapshot_send_attempts: None,
            snapshot_send_successes: None,
            snapshot_send_failures: None,
            snapshot_sends_in_flight: None,
            snapshot_installs: None,
        }
    }
}

/// Fail-loud rejection from the narrow Raft compaction control.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RaftCompactionError {
    /// The current role/runtime has no disk-backed Raft log.
    #[error("raft compaction control is unavailable for this runtime")]
    Unavailable,
    /// The control exists but was not explicitly enabled.
    #[error("raft compaction control is disabled; set HYDRACACHE_RAFT_COMPACTION=true explicitly")]
    Disabled,
    /// The Raft runtime or durable store rejected the operation.
    #[error("raft compaction failed: {0}")]
    Runtime(String),
}

/// Read-only provider of cluster status.
pub trait ClusterStatusProvider: fmt::Debug + Send + Sync {
    /// Notify the provider that the server has started graceful drain.
    fn begin_drain(&self) {}

    /// Return a cluster status snapshot, incorporating current runtime state.
    fn cluster_status(&self, runtime: ClusterStatusRuntime) -> ClusterStatus;
}

/// Modeled status used when the daemon has no live grid handle.
#[derive(Debug, Clone, Default)]
pub struct ModeledClusterStatus;

impl ClusterStatusProvider for ModeledClusterStatus {
    fn cluster_status(&self, runtime: ClusterStatusRuntime) -> ClusterStatus {
        ClusterStatus {
            source: StatusSource::Modeled,
            leader: runtime.ready.then(|| "local".to_owned()),
            term: u64::from(runtime.ready),
            epoch: 0,
            quorum_ok: runtime.ready && !runtime.draining,
            members: Vec::new(),
            voters: 0,
            reshard_phase: ReshardPhase::Idle,
            draining: runtime.draining,
        }
    }
}

/// Minimal read-only handle over a live grid/control-plane.
pub trait GridControlPlaneHandle: fmt::Debug + Send + Sync {
    /// Notify the live grid that graceful drain started.
    fn begin_drain(&self);

    /// Return a point-in-time control-plane snapshot.
    fn snapshot(&self) -> RaftMetadataSnapshot;
    /// Return visible members.
    fn members(&self) -> Vec<ClusterMember>;
    /// Return the raft soft-state leader id, if known.
    fn raft_leader_id(&self) -> Option<String>;
    /// Return whether the live grid currently has quorum.
    fn has_quorum(&self) -> bool;
    /// Return whether `observed` is still the fully applied local metadata view.
    ///
    /// Networked followers must fence authority while their committed index is
    /// ahead of the locally applied metadata state. The observed-snapshot
    /// argument also prevents a projection assembled across an apply boundary
    /// from being published as authoritative.
    fn metadata_authority_matches(&self, observed: &RaftMetadataSnapshot) -> bool {
        let _ = observed;
        true
    }
    /// Return current raft voter count.
    fn voter_count(&self) -> u32;
    /// Return reachability for one known node.
    fn reachability(&self, node: &ClusterNodeId) -> Reachability;
    /// Return the current reshard phase.
    fn reshard_phase(&self) -> ReshardPhase;
    /// Return whether the grid itself is draining.
    fn is_draining(&self) -> bool;

    /// Return disk-backed Raft compaction progress and enablement.
    fn raft_compaction_status(&self) -> Result<RaftCompactionStatus, RaftCompactionError> {
        Ok(RaftCompactionStatus::unavailable())
    }

    /// Compact the durable Raft log exactly at current applied progress.
    fn compact_raft_log_at_applied(&self) -> Result<RaftCompactionStatus, RaftCompactionError> {
        Err(RaftCompactionError::Unavailable)
    }
}

/// Live status backed by a grid/control-plane handle.
#[derive(Debug, Clone)]
pub struct LiveClusterStatus {
    grid: Arc<dyn GridControlPlaneHandle>,
}

impl LiveClusterStatus {
    /// Build a live status provider.
    pub fn new(grid: Arc<dyn GridControlPlaneHandle>) -> Self {
        Self { grid }
    }
}

impl ClusterStatusProvider for LiveClusterStatus {
    fn begin_drain(&self) {
        self.grid.begin_drain();
    }

    fn cluster_status(&self, runtime: ClusterStatusRuntime) -> ClusterStatus {
        let snapshot = self.grid.snapshot();
        let draining = runtime.draining || self.grid.is_draining();
        let members = self
            .grid
            .members()
            .into_iter()
            .map(|member| MemberStatus {
                node_id: member.node_id.to_string(),
                role: MemberRole::from(member.role),
                reachable: self.grid.reachability(&member.node_id),
                generation: member.generation.value(),
            })
            .collect();
        let metadata_authoritative = self.grid.metadata_authority_matches(&snapshot);

        ClusterStatus {
            source: StatusSource::Live,
            leader: metadata_authoritative
                .then(|| self.grid.raft_leader_id())
                .flatten(),
            term: snapshot.term,
            epoch: snapshot.epoch.value(),
            quorum_ok: runtime.ready
                && metadata_authoritative
                && self.grid.has_quorum()
                && !draining,
            members,
            voters: self.grid.voter_count(),
            reshard_phase: self.grid.reshard_phase(),
            draining,
        }
    }
}
