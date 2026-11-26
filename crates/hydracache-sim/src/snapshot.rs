use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hydracache::{ClusterNodeMessage, LogicalTime};
use serde::{Deserialize, Serialize};

use crate::{
    ElectionNodeState, ElectionSignal, History, InvariantChecker, InvariantReport, TimedMessage,
    WorkloadOp, WorkloadResult,
};

/// Current stable simulator snapshot JSON schema version.
pub const SIM_SNAPSHOT_SCHEMA_VERSION: u16 = 11;

/// Region every simulator node reports; the simulator models one region.
pub(crate) const SIM_REGION: &str = "local";

/// Maximum number of in-flight messages rendered in one snapshot.
pub const MAX_IN_FLIGHT_RENDERED: usize = 64;

/// Maximum pending cache events kept for one subscriber.
pub const MAX_SUBSCRIBER_BUFFER: usize = 16;

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
    /// Manual-mode client actors.
    pub clients: Vec<ClientView>,
    /// Manual-mode namespace subscribers.
    pub subscribers: Vec<SubscriberView>,
    /// Client processes admitted through the simulated Raft metadata path
    /// (schema 10): one entry per client name, writer or reader, with the
    /// member its connection is open to.
    pub admitted_clients: Vec<AdmittedClientView>,
    /// Write path manual pushes follow (schema 11).
    pub replication: ReplicationView,
    /// Recent routed writes with their owners and acknowledgements (schema 11);
    /// empty in the `full_broadcast` model.
    pub writes: Vec<GridWriteView>,
    /// Cluster sync progress visible to manual/topology modes.
    pub sync_progress: Vec<SyncProgressView>,
    /// Current deterministic rebalance/reshard progress.
    pub rebalance: Option<RebalanceView>,
    /// Current lab mode.
    pub mode: String,
    /// Active scripted scenario, when any.
    pub active_scenario: Option<String>,
    /// Number of replay-visible interventions recorded by the control surface.
    pub intervention_count: u64,
    /// Whether the built-in smoke workload is currently issuing operations.
    pub workload_enabled: bool,
    /// Names of the invariants the verdict below is computed from.
    pub checked_invariants: Vec<String>,
    /// First scheduler step at which the verdict reported a violation, if ever.
    pub first_violation_step: Option<u64>,
    /// Whether each product consistency level could be satisfied right now.
    pub consistency: Vec<ConsistencyLevelView>,
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
            clients: Vec::new(),
            subscribers: Vec::new(),
            admitted_clients: Vec::new(),
            replication: ReplicationView {
                model: "none".to_owned(),
                replication_factor: None,
                write_level: None,
                disclosure: "history-only snapshots do not include a write path".to_owned(),
            },
            writes: Vec::new(),
            sync_progress: Vec::new(),
            rebalance: None,
            mode: "manual".to_owned(),
            active_scenario: None,
            intervention_count: 0,
            workload_enabled: false,
            checked_invariants: InvariantChecker::HISTORY_INVARIANTS
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
            first_violation_step: (!report.violations.is_empty()).then_some(step),
            consistency: Vec::new(),
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
    /// Write, invalidate and flush operations this node itself has applied
    /// (schema 9). Unlike `commit_index`, which is cluster-wide, this lags on
    /// a node that missed writes (crashed, isolated, partitioned) and catches
    /// up when the node copies state on restart or rejoin.
    pub applied_writes: u64,
    /// Whether the node is currently up.
    pub up: bool,
    /// Whether the node is currently crashed.
    pub crashed: bool,
    /// Whether the node is administratively disabled in the simulator.
    pub disabled: bool,
}

/// Messages one directed link dropped, by cause (schema 9). Cumulative over
/// the run; a cut link that keeps refusing traffic shows `cut_at_send` rising.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkDropsView {
    /// Refused at send because the link was partitioned.
    pub cut_at_send: u64,
    /// Discarded by an injected link fault (`drop_next` and similar).
    pub injected: u64,
    /// In flight when the link was cut; discarded on delivery.
    pub cut_at_delivery: u64,
    /// Delivered to a node that was crashed, disabled or isolated.
    pub target_down: u64,
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
    /// Messages this directed link has dropped so far, by cause (schema 9).
    pub drops: LinkDropsView,
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
    /// Rendezvous owners of the key in the `replicated` model (schema 11);
    /// `None` in the other models, where every member may hold it.
    pub owners: Option<KeyOwnersView>,
}

/// Primary and backup owners of one key (schema 11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyOwnersView {
    /// Primary owner.
    pub primary: String,
    /// Backup owners in rendezvous order.
    pub backups: Vec<String>,
}

/// The write path manual pushes follow (schema 11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationView {
    /// `full_broadcast`, `local_first`, or `replicated`.
    pub model: String,
    /// Copies including the primary (`replicated` only).
    pub replication_factor: Option<u64>,
    /// Write acknowledgement level (`replicated` only).
    pub write_level: Option<String>,
    /// Disclosure for demo surfaces.
    pub disclosure: String,
}

/// One routed manual write (schema 11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridWriteView {
    /// Write id, in push order.
    pub id: u64,
    /// Writer client.
    pub client: String,
    /// Namespaced key.
    pub key: String,
    /// Member the client is connected to; it answers the client.
    pub coordinator: String,
    /// Primary owner (the coordinator in `local_first`).
    pub primary: String,
    /// Backup owners.
    pub backups: Vec<String>,
    /// Backups whose copy the primary has seen acknowledged.
    pub backups_applied: Vec<String>,
    /// Acknowledgement level (`replicated` only).
    pub level: Option<String>,
    /// Acknowledgements the level requires.
    pub required_acks: u64,
    /// Acknowledgements the coordinator has counted.
    pub acks: u64,
    /// `pending`, `acknowledged`, or `failed`.
    pub status: String,
    /// The product's reason when the level can no longer be met.
    pub reason: Option<String>,
    /// Step of the push.
    pub started_at_step: u64,
    /// Step the required acknowledgements arrived.
    pub acknowledged_at_step: Option<u64>,
}

/// Satisfiability of one product consistency level on the simulator topology.
///
/// Computed with the product `hydracache::ConsistencyLevel` rules (see
/// `crate::consistency` for the parity-tested mirror) with every simulator
/// node as a replica (the simulator fully replicates) and the live nodes as the
/// ones that can acknowledge. It says whether an operation at this level could
/// be acknowledged now; the simulator does not run per-level reads or writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyLevelView {
    /// Level name: `one`, `local_quorum`, `quorum`, `each_quorum`, or `all`.
    pub level: String,
    /// Effective replicas for the level (every simulator node).
    pub total_replicas: u64,
    /// Acknowledgements the level requires.
    pub required_acks: u64,
    /// Replicas currently able to acknowledge (not crashed, disabled, or isolated).
    pub live_replicas: u64,
    /// Whether the requirement can be met by the live replicas.
    pub satisfiable: bool,
    /// Product explanation when the level is unsatisfiable.
    pub reason: Option<String>,
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
    /// Step at which this node applied the value it holds (schema 9): its own
    /// write or invalidate, a replicated one, or a catch-up copy. `None` when
    /// the node has no value or got it before the step was recorded.
    pub applied_at_step: Option<u64>,
}

/// Manual-mode client actor projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientView {
    /// Stable client id.
    pub id: String,
    /// Namespace used by the latest action.
    pub namespace: String,
    /// Last operation shown in the client lane.
    pub last_op: Option<String>,
    /// Operations currently in flight for this actor.
    pub in_flight: u32,
    /// Member this client's connection is open to (its coordinator). Sticky
    /// since schema 10: it changes only when that member goes down.
    pub connected_node: Option<String>,
}

/// A client process admitted through the simulated `ClientUpsert` metadata
/// command (schema 10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmittedClientView {
    /// Client name.
    pub id: String,
    /// Step at which the simulated `ClientUpsert` committed.
    pub admitted_at_step: u64,
    /// Member the client's connection is open to; it coordinates the client's
    /// writes. `None` when no member is live.
    pub connected_node: Option<String>,
}

/// Manual-mode subscriber projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriberView {
    /// Stable subscriber id.
    pub id: String,
    /// Client actor that owns the subscription.
    pub client_id: String,
    /// Subscribed namespace.
    pub namespace: String,
    /// Last delivered cache-signal event.
    pub last_event: Option<SubscriberEventView>,
    /// Pending events in the bounded subscriber buffer.
    pub lag: u64,
    /// Events dropped because the bounded buffer was full.
    pub dropped: u64,
    /// Member the subscriber's client is connected to (sticky since schema 10).
    pub connected_node: Option<String>,
}

/// Subscriber event visible to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriberEventView {
    /// Bus-carried cache event kind.
    pub kind: String,
    /// Namespaced cache key.
    pub key: String,
    /// Stable value version/checksum.
    pub version: u64,
    /// Commit index required before delivery.
    pub commit_index: u64,
    /// Scheduler step where the event was delivered.
    pub delivered_at_step: u64,
}

/// Per-node catch-up/sync progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncProgressView {
    /// Node id.
    pub node_id: String,
    /// Applied entries for this node.
    pub applied_index: u64,
    /// Leader commit index visible to this node.
    pub leader_commit_index: u64,
}

/// Deterministic rebalance progress visible to topology demos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceView {
    /// Rebalance phase.
    pub phase: String,
    /// Partitions/keys moved by the simulator model.
    pub moved_partitions: u64,
    /// Total partitions/keys considered by the simulator model.
    pub total_partitions: u64,
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
    applied_writes: u64,
    crashed: bool,
    disabled: bool,
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
        region: SIM_REGION.to_owned(),
        zone: "default".to_owned(),
        role,
        term,
        vote_state,
        voted_for,
        votes_received,
        commit_index: committed_entries,
        applied_index: applied_entries,
        applied_writes,
        up: !crashed && !disabled,
        crashed,
        disabled,
    }
}

pub(crate) fn link_view(
    from: String,
    to: String,
    can_deliver: bool,
    delay: Option<u64>,
    in_flight: usize,
    drops: crate::network::LinkDrops,
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
        drops: LinkDropsView {
            cut_at_send: drops.cut_at_send,
            injected: drops.injected,
            cut_at_delivery: drops.cut_at_delivery,
            target_down: drops.target_down,
        },
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
    observations: BTreeMap<String, BTreeMap<String, (u64, Option<u64>)>>,
) -> Vec<KeyView> {
    observations
        .into_iter()
        .map(|(key, replicas)| KeyView {
            key,
            owners: None,
            replicas: replicas
                .into_iter()
                .map(|(node_id, (version, applied_at_step))| KeyReplicaView {
                    node_id,
                    version,
                    epoch: 0,
                    applied_at_step,
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
                owners: None,
                replicas: vec![KeyReplicaView {
                    node_id: "history".to_owned(),
                    version,
                    epoch: 0,
                    applied_at_step: None,
                }],
            }
        })
        .collect()
}

pub(crate) fn logical_millis(time: LogicalTime) -> u64 {
    time.as_millis()
}
