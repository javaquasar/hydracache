use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hydracache::{ClusterNodeMessage, LogicalTime};
use serde::{Deserialize, Serialize};

use crate::{
    ElectionNodeState, ElectionSignal, History, InvariantChecker, InvariantReport, TimedMessage,
    WorkloadOp, WorkloadResult,
};

/// Current stable simulator snapshot JSON schema version.
pub const SIM_SNAPSHOT_SCHEMA_VERSION: u16 = 3;

/// Maximum number of in-flight messages rendered in one snapshot.
pub const MAX_IN_FLIGHT_RENDERED: usize = 64;

/// Versioned browser/demo view over the real deterministic simulator state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimSnapshot {
    /// Snapshot JSON schema version.
    pub schema_version: u16,
    /// Seed used to build the run.
    pub seed: u64,
    /// Executed scheduler step count.
    pub step: u64,
    /// Current logical time in milliseconds.
    pub logical_time_millis: u64,
    /// Current modeled cluster formation phase.
    pub formation_phase: String,
    /// Election source used by the simulator.
    pub election_source: String,
    /// Disclosure shown by demo surfaces for the election source.
    pub election_disclosure: String,
    /// Nodes rendered by the UI.
    pub nodes: Vec<NodeView>,
    /// Directed links rendered by the UI.
    pub links: Vec<LinkView>,
    /// Typed network/election messages currently visible to the simulator.
    pub in_flight: Vec<MessageView>,
    /// Snapshot budget counters for intentionally summarized views.
    pub over_budget: SnapshotOverBudgetView,
    /// Sampled key state by replica.
    pub keys: Vec<KeyView>,
    /// Invariant verdict for the current state.
    pub verdict: VerdictView,
    /// Progress summary for dashboard panels.
    pub progress: ProgressView,
}

impl SimSnapshot {
    /// Build a minimal snapshot from a workload history and the real checker.
    pub fn from_history(seed: u64, step: u64, history: &History) -> Self {
        let report = InvariantChecker.check_history(history);
        Self {
            schema_version: SIM_SNAPSHOT_SCHEMA_VERSION,
            seed,
            step,
            logical_time_millis: 0,
            formation_phase: "history_only".to_owned(),
            election_source: "none".to_owned(),
            election_disclosure: "history-only snapshots do not include an election model"
                .to_owned(),
            nodes: Vec::new(),
            links: Vec::new(),
            in_flight: Vec::new(),
            over_budget: SnapshotOverBudgetView::default(),
            keys: keys_from_history(history),
            verdict: VerdictView::from_report(&report),
            progress: ProgressView {
                committed_entries: history.completed().count() as u64,
                last_leader_change: None,
                convergence: convergence_from_report(&report),
            },
        }
    }

    /// Serialize this snapshot as canonical JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("simulator snapshot serialization is infallible")
    }

    /// Serialize only the verdict panel view as JSON.
    pub fn verdict_json(&self) -> String {
        serde_json::to_string(&self.verdict).expect("simulator verdict serialization is infallible")
    }

    /// Decode a snapshot and reject unknown future schema versions loudly.
    pub fn from_json(input: &str) -> Result<Self, SimSnapshotDecodeError> {
        let header: SnapshotHeader =
            serde_json::from_str(input).map_err(|error| SimSnapshotDecodeError::InvalidJson {
                message: error.to_string(),
            })?;
        if header.schema_version > SIM_SNAPSHOT_SCHEMA_VERSION {
            return Err(SimSnapshotDecodeError::UnsupportedVersion {
                found: header.schema_version,
                max_supported: SIM_SNAPSHOT_SCHEMA_VERSION,
            });
        }
        if header.schema_version != SIM_SNAPSHOT_SCHEMA_VERSION {
            return Err(SimSnapshotDecodeError::UnsupportedVersion {
                found: header.schema_version,
                max_supported: SIM_SNAPSHOT_SCHEMA_VERSION,
            });
        }
        serde_json::from_str(input).map_err(|error| SimSnapshotDecodeError::InvalidJson {
            message: error.to_string(),
        })
    }
}

/// UI node projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeView {
    /// Stable node id.
    pub id: String,
    /// Region label used by the demo layout.
    pub region: String,
    /// Zone label used by the demo layout.
    pub zone: String,
    /// Human-readable role.
    pub role: String,
    /// Logical consensus term. The 0.44 simulator has no leader election yet.
    pub term: u64,
    /// Modeled election/vote state.
    pub vote_state: String,
    /// Candidate this node voted for in the current term.
    pub voted_for: Option<String>,
    /// Votes received by this node when it is the winning candidate.
    pub votes_received: u32,
    /// Committed logical operation index visible to the simulator.
    pub commit_index: u64,
    /// Applied logical operation index visible to the simulator.
    pub applied_index: u64,
    /// Whether the node is currently up.
    pub up: bool,
    /// Whether the node is currently crashed.
    pub crashed: bool,
}

/// UI directed-link projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkView {
    /// Source node id.
    pub from: String,
    /// Destination node id.
    pub to: String,
    /// Link state.
    pub state: LinkStateView,
    /// Delay in milliseconds when [`LinkStateView::Delayed`].
    pub delay_millis: Option<u64>,
    /// Packets currently in flight on this directed link.
    pub in_flight: u32,
}

/// Typed in-flight message projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageView {
    /// Stable message id for deterministic rendering and replay hashing.
    pub id: String,
    /// Source node id.
    pub from: String,
    /// Destination node id.
    pub to: String,
    /// Stable message kind label.
    pub kind: String,
    /// Cache key carried by replication messages, when present.
    pub key: Option<String>,
    /// Message sequence or logical term.
    pub sequence: Option<u64>,
    /// Logical delivery timestamp in milliseconds.
    pub deliver_at_millis: u64,
    /// Logical time until delivery in milliseconds.
    pub remaining_millis: u64,
}

/// Snapshot budget counters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotOverBudgetView {
    /// In-flight messages omitted after [`MAX_IN_FLIGHT_RENDERED`].
    pub in_flight_summarized: u64,
}

/// Link state visible to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkStateView {
    /// Link can currently deliver packets.
    Up,
    /// Link is partitioned.
    Partitioned,
    /// Link has delayed packets in flight.
    Delayed,
}

/// Sampled key projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyView {
    /// Cache key.
    pub key: String,
    /// Per-replica observations.
    pub replicas: Vec<KeyReplicaView>,
}

/// Per-replica key observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyReplicaView {
    /// Node id.
    pub node_id: String,
    /// Simulator-visible value version. For 0.44 storage this is the stored checksum.
    pub version: u64,
    /// Logical epoch. The 0.44 simulator does not model storage epochs yet.
    pub epoch: u64,
}

/// Invariant verdict visible to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VerdictView {
    /// All checked invariants hold.
    Holding,
    /// At least one real invariant checker violation is present.
    Violated {
        /// First violated invariant name.
        invariant: String,
        /// Human-readable violation detail.
        detail: String,
    },
}

impl VerdictView {
    /// Build a UI verdict from a real invariant report.
    pub fn from_report(report: &InvariantReport) -> Self {
        report
            .violations
            .first()
            .map(|violation| Self::Violated {
                invariant: violation.name.to_owned(),
                detail: violation.message.clone(),
            })
            .unwrap_or(Self::Holding)
    }
}

/// Progress panel projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressView {
    /// Completed operations recorded by the simulator.
    pub committed_entries: u64,
    /// Last leader-change logical timestamp, when a leader model exists.
    pub last_leader_change: Option<u64>,
    /// Convergence status derived from the real invariant report.
    pub convergence: ConvergenceView,
}

/// Convergence status visible to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvergenceView {
    /// No divergence was reported.
    Converged,
    /// The real checker reported a violation.
    Diverged,
}

/// Error returned by strict snapshot JSON decoders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimSnapshotDecodeError {
    /// JSON could not be parsed into the snapshot schema.
    InvalidJson { message: String },
    /// Snapshot schema version is not supported by this reader.
    UnsupportedVersion { found: u16, max_supported: u16 },
}

impl fmt::Display for SimSnapshotDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson { message } => write!(formatter, "invalid snapshot JSON: {message}"),
            Self::UnsupportedVersion {
                found,
                max_supported,
            } => write!(
                formatter,
                "unsupported simulator snapshot schema version {found}; max supported is {max_supported}"
            ),
        }
    }
}

impl std::error::Error for SimSnapshotDecodeError {}

#[derive(Debug, Deserialize)]
struct SnapshotHeader {
    schema_version: u16,
}

pub(crate) fn node_view(
    id: String,
    committed_entries: u64,
    applied_entries: u64,
    crashed: bool,
    election: Option<&ElectionNodeState>,
) -> NodeView {
    let (role, term, vote_state, voted_for, votes_received) = election
        .map(|node| {
            (
                node.state.to_string(),
                node.term,
                node.state.to_string(),
                node.voted_for.as_ref().map(ToString::to_string),
                node.votes_received.min(u32::MAX as usize) as u32,
            )
        })
        .unwrap_or_else(|| ("member".to_owned(), 0, "unknown".to_owned(), None, 0));
    NodeView {
        id,
        region: "local".to_owned(),
        zone: "default".to_owned(),
        role,
        term,
        vote_state,
        voted_for,
        votes_received,
        commit_index: committed_entries,
        applied_index: applied_entries,
        up: !crashed,
        crashed,
    }
}

pub(crate) fn link_view(
    from: String,
    to: String,
    can_deliver: bool,
    delay: Option<u64>,
    in_flight: usize,
) -> LinkView {
    let state = if !can_deliver {
        LinkStateView::Partitioned
    } else if delay.is_some() {
        LinkStateView::Delayed
    } else {
        LinkStateView::Up
    };
    LinkView {
        from,
        to,
        state,
        delay_millis: delay,
        in_flight: in_flight.min(u32::MAX as usize) as u32,
    }
}

pub(crate) fn message_views_from_network_and_election<'a>(
    network_messages: impl IntoIterator<Item = &'a TimedMessage>,
    election_signals: impl IntoIterator<Item = &'a ElectionSignal>,
    now: LogicalTime,
) -> (Vec<MessageView>, SnapshotOverBudgetView) {
    let mut messages = network_messages
        .into_iter()
        .map(|packet| message_view_from_network(packet, now))
        .chain(
            election_signals
                .into_iter()
                .map(|signal| message_view_from_election(signal, now)),
        )
        .collect::<Vec<_>>();
    messages.sort_by(|left, right| left.id.cmp(&right.id));
    let summarized = messages.len().saturating_sub(MAX_IN_FLIGHT_RENDERED);
    messages.truncate(MAX_IN_FLIGHT_RENDERED);
    (
        messages,
        SnapshotOverBudgetView {
            in_flight_summarized: summarized as u64,
        },
    )
}

fn message_view_from_network(packet: &TimedMessage, now: LogicalTime) -> MessageView {
    let (kind, key, sequence) = match &packet.message {
        ClusterNodeMessage::Heartbeat { sequence, .. } => ("heartbeat", None, Some(*sequence)),
        ClusterNodeMessage::ReplicatePut { key, sequence, .. } => {
            ("replicate_put", Some(key.clone()), Some(*sequence))
        }
        ClusterNodeMessage::ReplicateInvalidate { key, sequence } => {
            ("replicate_invalidate", Some(key.clone()), Some(*sequence))
        }
        ClusterNodeMessage::ReplicateFlush { sequence } => {
            ("replicate_flush", None, Some(*sequence))
        }
        ClusterNodeMessage::Ack { sequence } => ("ack", None, Some(*sequence)),
    };
    MessageView {
        id: format!("net-{}", packet.packet_id()),
        from: packet.from.to_string(),
        to: packet.to.to_string(),
        kind: kind.to_owned(),
        key,
        sequence,
        deliver_at_millis: packet.deliver_at.as_millis(),
        remaining_millis: packet
            .deliver_at
            .as_millis()
            .saturating_sub(now.as_millis()),
    }
}

fn message_view_from_election(signal: &ElectionSignal, now: LogicalTime) -> MessageView {
    MessageView {
        id: format!("election-{}", signal.id),
        from: signal.from.to_string(),
        to: signal.to.to_string(),
        kind: signal.kind.as_str().to_owned(),
        key: None,
        sequence: Some(signal.term),
        deliver_at_millis: now.as_millis(),
        remaining_millis: 0,
    }
}

pub(crate) fn key_views_from_storage(
    observations: BTreeMap<String, BTreeMap<String, u64>>,
) -> Vec<KeyView> {
    observations
        .into_iter()
        .map(|(key, replicas)| KeyView {
            key,
            replicas: replicas
                .into_iter()
                .map(|(node_id, version)| KeyReplicaView {
                    node_id,
                    version,
                    epoch: 0,
                })
                .collect(),
        })
        .collect()
}

pub(crate) fn progress_from_report(
    committed_entries: u64,
    report: &InvariantReport,
) -> ProgressView {
    ProgressView {
        committed_entries,
        last_leader_change: None,
        convergence: convergence_from_report(report),
    }
}

pub(crate) fn convergence_from_report(report: &InvariantReport) -> ConvergenceView {
    if report.violations.is_empty() {
        ConvergenceView::Converged
    } else {
        ConvergenceView::Diverged
    }
}

fn keys_from_history(history: &History) -> Vec<KeyView> {
    let mut versions = BTreeMap::<String, u64>::new();
    let mut keys = BTreeSet::<String>::new();
    for event in history.completed() {
        match (&event.op, &event.result) {
            (
                WorkloadOp::Put { key, value } | WorkloadOp::CompareAndSet { key, value, .. },
                Some(WorkloadResult::Accepted { .. }),
            ) => {
                let version = versions.entry(key.clone()).or_default();
                *version = version
                    .saturating_add(1)
                    .max(crate::storage::checksum(value));
                keys.insert(key.clone());
            }
            (WorkloadOp::Invalidate { key }, Some(WorkloadResult::Accepted { .. })) => {
                let version = versions.entry(key.clone()).or_default();
                *version = version.saturating_add(1);
                keys.insert(key.clone());
            }
            (
                WorkloadOp::Get { key } | WorkloadOp::SessionRead { key },
                Some(WorkloadResult::Value(_)),
            ) => {
                keys.insert(key.clone());
            }
            _ => {}
        }
    }
    keys.into_iter()
        .map(|key| {
            let version = versions.get(&key).copied().unwrap_or_default();
            KeyView {
                key,
                replicas: vec![KeyReplicaView {
                    node_id: "history".to_owned(),
                    version,
                    epoch: 0,
                }],
            }
        })
        .collect()
}

pub(crate) fn logical_millis(time: LogicalTime) -> u64 {
    time.as_millis()
}
