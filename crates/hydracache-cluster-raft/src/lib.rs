//! raft-rs metadata control-plane runtime for HydraCache cluster mode.
//!
//! The base `hydracache` crate exposes a transport-neutral
//! [`hydracache::ClusterControlPlane`] trait. This crate plugs a real
//! `raft-rs` [`raft::RawNode`] behind that trait while keeping the local cache
//! crate free from Raft dependencies.
//!
//! The embedded default can run single-node and in-memory, while the standalone
//! server opens the feature-gated sled store for process-restart durability.
//! Both paths drive the real raft-rs lifecycle: campaign, propose, `Ready`,
//! stable-log append, and committed-entry application.
//!
//! Applied commands are stored as [`RaftMetadataCommandEnvelope`] values with a
//! stable command id. Duplicate command ids are reported as
//! [`RaftCommandStatus::Duplicate`], and materialized membership changes happen
//! only after a successful Raft commit. [`RaftMetadataRuntime::export_snapshot`]
//! and [`RaftMetadataRuntime::from_snapshot`] provide an in-memory recovery
//! boundary for tests and demos.
//!
//! ## Bridging Discovery To Raft Metadata
//!
//! The cluster composition is deliberately split:
//!
//! - `hydracache-cluster-chitchat` discovers member/client candidates.
//! - [`hydracache::ClusterAdmissionBridge`] polls those candidates.
//! - `RaftMetadataRuntime` commits accepted membership metadata.
//!
//! ```no_run
//! use std::net::SocketAddr;
//! use std::sync::Arc;
//!
//! use hydracache::{
//!     ClusterAdmissionBridge, ClusterCandidate, ClusterGeneration,
//!     ClusterDiscovery,
//! };
//! use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
//! use hydracache_cluster_raft::RaftMetadataRuntime;
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let discovery = Arc::new(
//!     ChitchatDiscovery::spawn_udp(ChitchatDiscoveryConfig::new(
//!         "orders",
//!         "member-a",
//!         ClusterGeneration::new(1),
//!         "127.0.0.1:7000".parse::<SocketAddr>().unwrap(),
//!     ))
//!     .await?,
//! );
//! let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1)?);
//! let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());
//!
//! discovery
//!     .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
//!     .await?;
//! bridge.run_once().await;
//!
//! assert_eq!(bridge.diagnostics().candidates_admitted, 1);
//! assert_eq!(control_plane.snapshot().commands_committed, 1);
//! # Ok(())
//! # }
//! ```
//!
//! # Example
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use hydracache::{ClusterGeneration, HydraCache};
//! use hydracache_cluster_raft::RaftMetadataRuntime;
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1)?);
//!
//! let member = HydraCache::member()
//!     .control_plane(control_plane.clone())
//!     .node_id("member-a")
//!     .generation(ClusterGeneration::new(1))
//!     .start()
//!     .await?;
//!
//! assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
//! assert_eq!(control_plane.snapshot().commands_committed, 1);
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeSet;
use std::fmt;
#[cfg(feature = "sled-log-store")]
use std::path::Path;
use std::sync::{Arc, Mutex};

use hydracache::{
    CacheError, CacheInvalidationBus, CacheResult, ClusterCandidate, ClusterControlPlane,
    ClusterDiagnostics, ClusterEpoch, ClusterGeneration, ClusterMember, ClusterMembershipEvent,
    ClusterMembershipSubscriber, ClusterNodeId, ClusterRole, InMemoryCluster, RaftMetadataCommand,
    RaftMetadataSnapshot,
};
mod log_store;

#[cfg(feature = "sled-log-store")]
pub use log_store::SledRaftLogStore;
#[cfg(feature = "durable-log")]
pub use log_store::{
    DurableControlPlaneCluster, DurableRaftLogDirectory, DurableRaftLogStore,
    RAFT_LOG_FORMAT_VERSION,
};
pub use log_store::{InMemoryRaftLogStore, RaftLogStore, RaftStoreError, RaftStoreResult};

use protobuf::Message as ProtobufMessage;
use raft::eraftpb::{
    ConfChange, ConfChangeType, ConfChangeV2, Entry, EntryType, Message as RaftMessage, Snapshot,
};
use raft::storage::Storage;
use raft::{Config, RawNode, SnapshotStatus, StateRole};
use serde::{Deserialize, Serialize};
use slog::{o, Logger};
use tokio::time::{sleep, Duration};

const FORWARDED_APPLY_WAIT_ATTEMPTS: usize = 500;
const FORWARDED_APPLY_WAIT_INTERVAL: Duration = Duration::from_millis(10);
const DEFAULT_MAX_SIZE_PER_MSG: u64 = 1024 * 1024;

/// Configuration for an embedded raft-rs metadata runtime.
#[derive(Debug, Clone)]
pub struct RaftMetadataRuntimeConfig {
    cluster_name: String,
    raft_node_id: u64,
    voters: Vec<u64>,
    auto_campaign: bool,
    election_tick: usize,
    heartbeat_tick: usize,
    max_size_per_msg: u64,
    max_inflight_msgs: usize,
    pre_vote: bool,
}

impl RaftMetadataRuntimeConfig {
    /// Build a single-node runtime configuration.
    pub fn single_node(cluster_name: impl Into<String>, raft_node_id: u64) -> Self {
        Self {
            cluster_name: cluster_name.into(),
            raft_node_id: raft_node_id.max(1),
            voters: vec![raft_node_id.max(1)],
            auto_campaign: true,
            election_tick: 10,
            heartbeat_tick: 3,
            max_size_per_msg: DEFAULT_MAX_SIZE_PER_MSG,
            max_inflight_msgs: 256,
            pre_vote: true,
        }
    }

    /// Build a runtime configuration for an explicitly bootstrapped voter set.
    pub fn multi_voter<I>(cluster_name: impl Into<String>, raft_node_id: u64, voters: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        let raft_node_id = raft_node_id.max(1);
        Self {
            cluster_name: cluster_name.into(),
            raft_node_id,
            voters: normalize_voters(raft_node_id, voters),
            auto_campaign: false,
            election_tick: 10,
            heartbeat_tick: 3,
            max_size_per_msg: DEFAULT_MAX_SIZE_PER_MSG,
            max_inflight_msgs: 256,
            pre_vote: true,
        }
    }

    /// Build a runtime configuration for a node joining an existing voter set.
    ///
    /// The local node is deliberately not added to `remote_voters`; it becomes
    /// a voter only after the existing leader commits a ConfChange for it.
    pub fn try_joining<I>(
        cluster_name: impl Into<String>,
        raft_node_id: u64,
        remote_voters: I,
    ) -> CacheResult<Self>
    where
        I: IntoIterator<Item = u64>,
    {
        let raft_node_id = raft_node_id.max(1);
        let voters = normalize_remote_voters(remote_voters);
        if voters.is_empty() {
            return Err(CacheError::Backend(
                "joining raft runtime requires at least one remote voter".to_owned(),
            ));
        }
        if voters.contains(&raft_node_id) {
            return Err(CacheError::Backend(format!(
                "joining raft runtime remote voters must not include local node {raft_node_id}"
            )));
        }
        Ok(Self {
            cluster_name: cluster_name.into(),
            raft_node_id,
            voters,
            auto_campaign: false,
            election_tick: 10,
            heartbeat_tick: 3,
            max_size_per_msg: DEFAULT_MAX_SIZE_PER_MSG,
            max_inflight_msgs: 256,
            pre_vote: true,
        })
    }

    /// Control whether the runtime campaigns during construction.
    pub fn auto_campaign(mut self, auto_campaign: bool) -> Self {
        self.auto_campaign = auto_campaign;
        self
    }

    /// Return the configured voter ids.
    pub fn voter_ids(&self) -> &[u64] {
        &self.voters
    }

    /// Set Raft election and heartbeat ticks.
    pub fn ticks(mut self, election_tick: usize, heartbeat_tick: usize) -> Self {
        self.election_tick = election_tick.max(2);
        self.heartbeat_tick = heartbeat_tick.max(1);
        self
    }

    /// Set the max bytes per Raft message.
    pub fn max_size_per_msg(mut self, size: u64) -> Self {
        self.max_size_per_msg = size.max(1);
        self
    }

    /// Set max inflight messages.
    pub fn max_inflight_msgs(mut self, value: usize) -> Self {
        self.max_inflight_msgs = value.max(1);
        self
    }

    /// Override raft pre-vote behavior for mixed-version compatibility tests.
    ///
    /// The production default is `true` starting with HydraCache 0.62.0.
    pub fn pre_vote(mut self, pre_vote: bool) -> Self {
        self.pre_vote = pre_vote;
        self
    }

    fn raft_config(&self) -> Config {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("canary_raft_disable_prevote", |_| {
            Config {
                id: self.raft_node_id,
                election_tick: self.election_tick,
                heartbeat_tick: self.heartbeat_tick,
                max_size_per_msg: self.max_size_per_msg,
                max_inflight_msgs: self.max_inflight_msgs,
                pre_vote: false,
                ..Default::default()
            }
        });

        Config {
            id: self.raft_node_id,
            election_tick: self.election_tick,
            heartbeat_tick: self.heartbeat_tick,
            max_size_per_msg: self.max_size_per_msg,
            max_inflight_msgs: self.max_inflight_msgs,
            pre_vote: self.pre_vote,
            ..Default::default()
        }
    }
}

fn normalize_voters<I>(raft_node_id: u64, voters: I) -> Vec<u64>
where
    I: IntoIterator<Item = u64>,
{
    let mut voters = voters
        .into_iter()
        .map(|voter| voter.max(1))
        .chain(std::iter::once(raft_node_id.max(1)))
        .collect::<Vec<_>>();
    voters.sort_unstable();
    voters.dedup();
    voters
}

fn normalize_remote_voters<I>(voters: I) -> Vec<u64>
where
    I: IntoIterator<Item = u64>,
{
    let mut voters = voters
        .into_iter()
        .map(|voter| voter.max(1))
        .collect::<Vec<_>>();
    voters.sort_unstable();
    voters.dedup();
    voters
}

/// Point-in-time raft-rs metadata runtime snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftMetadataRuntimeSnapshot {
    /// Local raft node id.
    pub raft_node_id: u64,
    /// Current raft term.
    pub term: u64,
    /// Current committed index.
    pub commit_index: u64,
    /// Last applied index tracked by the metadata state machine.
    pub applied_index: u64,
    /// Current raft role.
    pub role: RaftRuntimeRole,
    /// Number of metadata commands applied from committed Raft entries.
    pub commands_committed: usize,
    /// Number of raft snapshots installed into the metadata state machine.
    pub snapshot_installs: u64,
    /// Last applied metadata command, if any.
    pub last_command: Option<RaftMetadataCommand>,
    /// Number of duplicate command ids skipped by the metadata state machine.
    pub duplicate_commands: usize,
    /// Last command result, if any.
    pub last_result: Option<RaftCommandResult>,
}

/// Metadata command plus a stable idempotency key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftMetadataCommandEnvelope {
    /// Stable command id used to deduplicate retries.
    pub command_id: String,
    /// Metadata command applied after Raft commit.
    pub command: RaftMetadataCommand,
}

/// Result of proposing a metadata command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftCommandResult {
    /// Stable command id.
    pub command_id: String,
    /// Result status.
    pub status: RaftCommandStatus,
    /// Applied index observed after the command was handled.
    pub applied_index: u64,
}

/// Status for a metadata command proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftCommandStatus {
    /// Command was proposed, committed, and applied.
    Committed,
    /// Command was accepted by raft-rs but has not been applied locally yet.
    Forwarded,
    /// Command id was already applied and was skipped.
    Duplicate,
}

/// Serialized Raft message envelope used by network transports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftWireMessage {
    /// Source Raft node id.
    pub from: u64,
    /// Destination Raft node id.
    pub to: u64,
    /// Message term.
    pub term: u64,
    /// Protobuf-encoded `raft::eraftpb::Message`.
    pub payload: Vec<u8>,
}

impl RaftWireMessage {
    /// Serialize a raft-rs message for transport.
    pub fn encode(message: &RaftMessage) -> CacheResult<Self> {
        Ok(Self {
            from: message.from,
            to: message.to,
            term: message.term,
            payload: message.write_to_bytes().map_err(|error| {
                CacheError::Encode(format!("failed to encode raft message: {error}"))
            })?,
        })
    }

    /// Decode the protobuf payload back into a raft-rs message.
    pub fn decode(&self) -> CacheResult<RaftMessage> {
        RaftMessage::parse_from_bytes(&self.payload)
            .map_err(|error| CacheError::Decode(format!("failed to decode raft message: {error}")))
    }
}

impl RaftMetadataCommandEnvelope {
    /// Encode this durable metadata command envelope.
    pub fn encode(&self) -> Vec<u8> {
        encode_envelope(self)
    }

    /// Decode a durable metadata command envelope.
    pub fn decode(data: &[u8]) -> CacheResult<Self> {
        decode_envelope(data)
    }
}

/// Transport seam for sending serialized Raft messages to peers.
#[async_trait::async_trait]
pub trait RaftMessageSink: Send + Sync {
    /// Send one serialized raft message.
    async fn send(&self, message: RaftWireMessage) -> CacheResult<()>;
}

/// In-memory sink used by tests and local harnesses.
#[derive(Debug, Clone, Default)]
pub struct InMemoryRaftMessageSink {
    messages: Arc<Mutex<Vec<RaftWireMessage>>>,
}

impl InMemoryRaftMessageSink {
    /// Return captured messages.
    pub fn messages(&self) -> Vec<RaftWireMessage> {
        self.messages
            .lock()
            .expect("raft message sink poisoned")
            .clone()
    }
}

#[async_trait::async_trait]
impl RaftMessageSink for InMemoryRaftMessageSink {
    async fn send(&self, message: RaftWireMessage) -> CacheResult<()> {
        self.messages
            .lock()
            .expect("raft message sink poisoned")
            .push(message);
        Ok(())
    }
}

/// Exported in-memory metadata snapshot for recovery tests and demos.
///
/// This is not a durable multi-node Raft log format yet. It captures the
/// materialized metadata commands that have already been applied so a new
/// runtime can rebuild the same membership view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftMetadataRuntimeExport {
    /// Logical cluster name.
    pub cluster_name: String,
    /// Local raft node id.
    pub raft_node_id: u64,
    /// Last applied index tracked by the metadata state machine.
    pub applied_index: u64,
    /// Applied command envelopes in order.
    pub commands: Vec<RaftMetadataCommandEnvelope>,
}

const RAFT_METADATA_SNAPSHOT_PAYLOAD_MAGIC: &[u8; 8] = b"HCMETA01";
const RAFT_METADATA_SNAPSHOT_PAYLOAD_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RaftMetadataSnapshotPayload {
    format_version: u32,
    cluster_name: String,
    source_raft_node_id: u64,
    applied_index: u64,
    commands: Vec<RaftMetadataCommandEnvelope>,
}

#[cfg(feature = "test-failpoints")]
fn encode_metadata_snapshot_payload(snapshot: &RaftMetadataRuntimeExport) -> CacheResult<Vec<u8>> {
    let payload = RaftMetadataSnapshotPayload {
        format_version: RAFT_METADATA_SNAPSHOT_PAYLOAD_VERSION,
        cluster_name: snapshot.cluster_name.clone(),
        source_raft_node_id: snapshot.raft_node_id,
        applied_index: snapshot.applied_index,
        commands: snapshot.commands.clone(),
    };
    let mut bytes = Vec::from(RAFT_METADATA_SNAPSHOT_PAYLOAD_MAGIC.as_slice());
    bytes.extend(serde_json::to_vec(&payload).map_err(|error| {
        CacheError::Backend(format!("failed to encode raft metadata snapshot: {error}"))
    })?);
    Ok(bytes)
}

fn decode_metadata_snapshot_payload(bytes: &[u8]) -> CacheResult<RaftMetadataRuntimeExport> {
    let payload = bytes
        .strip_prefix(RAFT_METADATA_SNAPSHOT_PAYLOAD_MAGIC)
        .ok_or_else(|| {
            CacheError::Backend("unsupported raft metadata snapshot payload".to_owned())
        })?;
    let payload: RaftMetadataSnapshotPayload =
        serde_json::from_slice(payload).map_err(|error| {
            CacheError::Backend(format!("failed to decode raft metadata snapshot: {error}"))
        })?;
    if payload.format_version != RAFT_METADATA_SNAPSHOT_PAYLOAD_VERSION {
        return Err(CacheError::Backend(format!(
            "unsupported raft metadata snapshot payload version {}",
            payload.format_version
        )));
    }
    Ok(RaftMetadataRuntimeExport {
        cluster_name: payload.cluster_name,
        raft_node_id: payload.source_raft_node_id,
        applied_index: payload.applied_index,
        commands: payload.commands,
    })
}

/// Storage seam for exported raft metadata snapshots.
///
/// This trait stores the materialized metadata snapshot returned by
/// [`RaftMetadataRuntime::export_snapshot`]. It is not a replacement for the
/// full raft-rs log storage yet; it gives applications and tests a stable seam
/// for recovering committed membership metadata.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
/// use hydracache_cluster_raft::{
///     InMemoryRaftMetadataStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
/// };
///
/// # #[tokio::main]
/// # async fn main() -> hydracache::CacheResult<()> {
/// let store = Arc::new(InMemoryRaftMetadataStore::new());
/// let runtime = RaftMetadataRuntime::with_config_and_metadata_store(
///     RaftMetadataRuntimeConfig::single_node("orders", 1),
///     store.clone(),
/// )?;
/// runtime
///     .join_member(
///         ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
///     )
///     .await?;
///
/// let recovered = RaftMetadataRuntime::with_config_and_metadata_store(
///     RaftMetadataRuntimeConfig::single_node("orders", 1),
///     store,
/// )?;
/// assert_eq!(recovered.snapshot().commands_committed, 1);
/// # Ok(())
/// # }
/// ```
pub trait RaftMetadataStore: fmt::Debug + Send + Sync + 'static {
    /// Load the latest exported metadata snapshot, if one exists.
    fn load(&self) -> CacheResult<Option<RaftMetadataRuntimeExport>>;

    /// Save the latest exported metadata snapshot.
    fn save(&self, snapshot: RaftMetadataRuntimeExport) -> CacheResult<()>;
}

/// In-memory [`RaftMetadataStore`] for tests, demos, and sandbox flows.
#[derive(Debug, Clone, Default)]
pub struct InMemoryRaftMetadataStore {
    snapshot: Arc<Mutex<Option<RaftMetadataRuntimeExport>>>,
}

impl InMemoryRaftMetadataStore {
    /// Create an empty in-memory metadata store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a store preloaded with an exported snapshot.
    pub fn with_snapshot(snapshot: RaftMetadataRuntimeExport) -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(Some(snapshot))),
        }
    }

    /// Return the currently saved snapshot.
    pub fn snapshot(&self) -> Option<RaftMetadataRuntimeExport> {
        self.snapshot
            .lock()
            .expect("raft metadata store poisoned")
            .clone()
    }
}

impl RaftMetadataStore for InMemoryRaftMetadataStore {
    fn load(&self) -> CacheResult<Option<RaftMetadataRuntimeExport>> {
        Ok(self.snapshot())
    }

    fn save(&self, snapshot: RaftMetadataRuntimeExport) -> CacheResult<()> {
        *self.snapshot.lock().expect("raft metadata store poisoned") = Some(snapshot);
        Ok(())
    }
}

/// Stable debug-friendly view of raft-rs [`StateRole`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftRuntimeRole {
    /// Node is follower.
    Follower,
    /// Node is candidate or pre-candidate.
    Candidate,
    /// Node is leader.
    Leader,
}

impl From<StateRole> for RaftRuntimeRole {
    fn from(role: StateRole) -> Self {
        match role {
            StateRole::Follower => Self::Follower,
            StateRole::PreCandidate | StateRole::Candidate => Self::Candidate,
            StateRole::Leader => Self::Leader,
        }
    }
}

struct RaftRuntimeState<S>
where
    S: RaftLogStore,
{
    cluster: Arc<InMemoryCluster>,
    raw_node: RawNode<S>,
    commands: Vec<RaftMetadataCommandEnvelope>,
    applied_command_ids: BTreeSet<String>,
    results: Vec<RaftCommandResult>,
    outbound_messages: Vec<RaftWireMessage>,
    applied_index: u64,
    snapshot_installs: u64,
    #[cfg(test)]
    fail_next_proposal: bool,
}

impl<S> fmt::Debug for RaftRuntimeState<S>
where
    S: RaftLogStore,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RaftRuntimeState")
            .field("commands", &self.commands)
            .field("applied_index", &self.applied_index)
            .finish_non_exhaustive()
    }
}

/// Single-node raft-rs metadata control plane.
pub struct RaftMetadataRuntime<S = InMemoryRaftLogStore>
where
    S: RaftLogStore,
{
    cluster: Arc<InMemoryCluster>,
    raft_node_id: u64,
    raft: Mutex<RaftRuntimeState<S>>,
    metadata_store: Option<Arc<dyn RaftMetadataStore>>,
}

impl<S> fmt::Debug for RaftMetadataRuntime<S>
where
    S: RaftLogStore,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RaftMetadataRuntime")
            .field("cluster", &self.cluster.name())
            .field("raft_node_id", &self.raft_node_id)
            .field("snapshot", &self.snapshot())
            .finish_non_exhaustive()
    }
}

impl RaftMetadataRuntime<InMemoryRaftLogStore> {
    /// Start a single-node raft-rs metadata runtime.
    pub fn single_node(cluster_name: impl Into<String>, raft_node_id: u64) -> CacheResult<Self> {
        Self::with_config(RaftMetadataRuntimeConfig::single_node(
            cluster_name,
            raft_node_id,
        ))
    }

    /// Start a single-node raft-rs metadata runtime with explicit config.
    pub fn with_config(config: RaftMetadataRuntimeConfig) -> CacheResult<Self> {
        Self::build_empty(config, None)
    }

    /// Start a single-node raft-rs metadata runtime backed by a metadata store.
    pub fn single_node_with_metadata_store(
        cluster_name: impl Into<String>,
        raft_node_id: u64,
        store: Arc<dyn RaftMetadataStore>,
    ) -> CacheResult<Self> {
        Self::with_config_and_metadata_store(
            RaftMetadataRuntimeConfig::single_node(cluster_name, raft_node_id),
            store,
        )
    }

    /// Start a raft-rs metadata runtime from config and recover any stored
    /// materialized metadata snapshot.
    pub fn with_config_and_metadata_store(
        config: RaftMetadataRuntimeConfig,
        store: Arc<dyn RaftMetadataStore>,
    ) -> CacheResult<Self> {
        let stored = store.load()?;
        if let Some(snapshot) = stored.as_ref() {
            validate_snapshot_identity(&config, snapshot)?;
        }
        let runtime = Self::build_empty(config, Some(store))?;
        if let Some(snapshot) = stored {
            runtime.restore_export(snapshot)?;
        }
        Ok(runtime)
    }

    fn build_empty(
        config: RaftMetadataRuntimeConfig,
        metadata_store: Option<Arc<dyn RaftMetadataStore>>,
    ) -> CacheResult<Self> {
        let storage =
            InMemoryRaftLogStore::new_with_conf_state((config.voter_ids().to_vec(), vec![]));
        Self::build_with_storage(config, storage, metadata_store)
    }

    /// Rebuild a single-node runtime from an exported metadata snapshot.
    pub fn from_snapshot(snapshot: RaftMetadataRuntimeExport) -> CacheResult<Self> {
        let runtime = Self::single_node(snapshot.cluster_name.clone(), snapshot.raft_node_id)?;
        runtime.restore_export(snapshot)?;
        Ok(runtime)
    }
}

#[cfg(feature = "durable-log")]
impl RaftMetadataRuntime<DurableRaftLogStore> {
    /// Open or create a single-node runtime backed by a durable raft log.
    pub fn durable(
        cluster_name: impl Into<String>,
        raft_node_id: u64,
        directory: DurableRaftLogDirectory,
    ) -> CacheResult<Self> {
        let config = RaftMetadataRuntimeConfig::single_node(cluster_name, raft_node_id);
        Self::durable_with_config(config, directory)
    }

    /// Open or create a runtime backed by a durable raft log using explicit config.
    pub fn durable_with_config(
        config: RaftMetadataRuntimeConfig,
        directory: DurableRaftLogDirectory,
    ) -> CacheResult<Self> {
        let storage = directory.open().map_err(to_cache_error)?;
        if storage
            .initial_state()
            .map_err(to_cache_error)?
            .conf_state
            .voters
            .is_empty()
        {
            storage.initialize_with_conf_state((config.voter_ids().to_vec(), vec![]));
        }
        Self::build_with_storage(config, storage, None)
    }
}

#[cfg(feature = "sled-log-store")]
impl RaftMetadataRuntime<SledRaftLogStore> {
    /// Open or create a process-restart durable runtime at `path`.
    pub fn sled_with_config(
        config: RaftMetadataRuntimeConfig,
        path: impl AsRef<Path>,
    ) -> CacheResult<Self> {
        let storage = SledRaftLogStore::open(path).map_err(to_cache_error)?;
        if storage
            .initial_state()
            .map_err(to_cache_error)?
            .conf_state
            .voters
            .is_empty()
        {
            storage.initialize_with_conf_state((config.voter_ids().to_vec(), vec![]));
            storage
                .save_conf_state(&storage.initial_state().map_err(to_cache_error)?.conf_state)
                .map_err(to_cache_error)?;
        }
        Self::build_with_storage(config, storage, None)
    }
}

impl<S> RaftMetadataRuntime<S>
where
    S: RaftLogStore,
{
    /// Start a raft-rs metadata runtime over an explicit log store.
    pub fn with_storage(config: RaftMetadataRuntimeConfig, storage: S) -> CacheResult<Self> {
        Self::build_with_storage(config, storage, None)
    }

    fn build_with_storage(
        config: RaftMetadataRuntimeConfig,
        storage: S,
        metadata_store: Option<Arc<dyn RaftMetadataStore>>,
    ) -> CacheResult<Self> {
        let cluster_name = config.cluster_name.clone();
        let raft_node_id = config.raft_node_id;
        let initial_state = storage.initial_state().map_err(to_cache_error)?;
        let retained_entries = storage.retained_entries().map_err(to_cache_error)?;
        let logger = Logger::root(slog::Discard, o!());
        let mut raw_node =
            RawNode::new(&config.raft_config(), storage, &logger).map_err(to_cache_error)?;
        if config.auto_campaign {
            raw_node.campaign().map_err(to_cache_error)?;
        }

        let cluster = Arc::new(InMemoryCluster::new(cluster_name));
        let mut state = RaftRuntimeState {
            cluster: cluster.clone(),
            raw_node,
            commands: Vec::new(),
            applied_command_ids: BTreeSet::new(),
            results: Vec::new(),
            outbound_messages: Vec::new(),
            applied_index: 0,
            snapshot_installs: 0,
            #[cfg(test)]
            fail_next_proposal: false,
        };
        let _ = state.drain_ready()?;

        let runtime = Self {
            cluster,
            raft_node_id,
            raft: Mutex::new(state),
            metadata_store,
        };
        runtime.restore_committed_entries(retained_entries, initial_state.hard_state.commit)?;
        Ok(runtime)
    }

    /// Return applied metadata commands.
    pub fn commands(&self) -> Vec<RaftMetadataCommand> {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .commands
            .iter()
            .map(|envelope| envelope.command.clone())
            .collect()
    }

    /// Return applied command envelopes with idempotency keys.
    pub fn command_envelopes(&self) -> Vec<RaftMetadataCommandEnvelope> {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .commands
            .clone()
    }

    /// Return command proposal results.
    pub fn command_results(&self) -> Vec<RaftCommandResult> {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .results
            .clone()
    }

    /// Return a metadata-journal snapshot shaped like the base control-plane seam.
    pub fn metadata_snapshot(&self) -> RaftMetadataSnapshot {
        let state = self.raft.lock().expect("raft metadata state poisoned");
        let hard_state = state.raw_node.raft.hard_state();
        RaftMetadataSnapshot {
            term: state.raw_node.raft.term,
            commit_index: hard_state.commit,
            epoch: self.cluster.epoch(),
            member_count: self.cluster.members().len(),
            client_count: self.cluster.clients().len(),
            last_command: state
                .commands
                .last()
                .map(|envelope| envelope.command.clone()),
        }
    }

    /// Return admitted member snapshots from the runtime control plane.
    pub fn members(&self) -> Vec<ClusterMember> {
        self.cluster.members()
    }

    /// Return connected client snapshots from the runtime control plane.
    pub fn clients(&self) -> Vec<ClusterMember> {
        self.cluster.clients()
    }

    /// Ask raft-rs to campaign and return outbound peer messages.
    pub fn campaign(&self) -> CacheResult<Vec<RaftWireMessage>> {
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        state.raw_node.campaign().map_err(to_cache_error)?;
        state.drain_ready()
    }

    /// Advance the raft logical clock and return outbound peer messages.
    pub fn tick(&self) -> CacheResult<Vec<RaftWireMessage>> {
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        state.raw_node.tick();
        state.drain_ready()
    }

    /// Step one inbound raft message and return outbound peer messages.
    pub fn step(&self, message: RaftWireMessage) -> CacheResult<Vec<RaftWireMessage>> {
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        state
            .raw_node
            .step(message.decode()?)
            .map_err(to_cache_error)?;
        state.drain_ready()
    }

    /// Drain any pending raft ready state and return outbound peer messages.
    pub fn drain_ready(&self) -> CacheResult<Vec<RaftWireMessage>> {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .drain_ready()
    }

    /// Report completion or failure of a snapshot transport attempt.
    ///
    /// Raft keeps a follower in snapshot progress until the transport reports
    /// an outcome. Reporting failure releases that progress for a bounded retry.
    pub fn report_snapshot_delivery(
        &self,
        peer_id: u64,
        delivered: bool,
    ) -> CacheResult<Vec<RaftWireMessage>> {
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        let status = if delivered {
            SnapshotStatus::Finish
        } else {
            SnapshotStatus::Failure
        };
        state.raw_node.report_snapshot(peer_id, status);
        state.drain_ready()
    }

    /// Force a metadata snapshot at the current applied index for compaction
    /// proof tests.
    #[cfg(feature = "test-failpoints")]
    pub fn compact_applied_log_to_snapshot_for_tests(&self) -> CacheResult<u64> {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .compact_applied_log_to_snapshot_for_tests(self.raft_node_id)
    }

    /// Return outbound messages captured while committing metadata commands.
    pub fn take_outbound_messages(&self) -> Vec<RaftWireMessage> {
        std::mem::take(
            &mut self
                .raft
                .lock()
                .expect("raft metadata state poisoned")
                .outbound_messages,
        )
    }

    /// Return the current raft soft-state leader id.
    ///
    /// Raft-rs uses `0` when no leader is known, such as during an election.
    pub fn leader_id(&self) -> Option<u64> {
        let state = self.raft.lock().expect("raft metadata state poisoned");
        known_leader_id(state.raw_node.raft.leader_id)
    }

    /// Return the current raft voter ids from the persisted conf state.
    pub fn voter_ids(&self) -> CacheResult<Vec<u64>> {
        let state = self.raft.lock().expect("raft metadata state poisoned");
        Ok(state
            .raw_node
            .raft
            .raft_log
            .store
            .initial_state()
            .map_err(to_cache_error)?
            .conf_state
            .voters)
    }

    /// Return whether a metadata command id has been applied locally.
    pub fn command_applied(&self, command_id: &str) -> bool {
        let state = self.raft.lock().expect("raft metadata state poisoned");
        state.applied_command_ids.contains(command_id)
    }

    /// Propose adding a raft voter through raft-rs ConfChange.
    pub fn propose_add_voter(&self, raft_node_id: u64) -> CacheResult<Vec<RaftWireMessage>> {
        self.propose_voter_change(raft_node_id, ConfChangeType::AddNode)
    }

    /// Propose removing a raft voter through raft-rs ConfChange.
    pub fn propose_remove_voter(&self, raft_node_id: u64) -> CacheResult<Vec<RaftWireMessage>> {
        self.propose_voter_change(raft_node_id, ConfChangeType::RemoveNode)
    }

    /// Request removing a raft voter through raft-rs ConfChange.
    ///
    /// Unlike the leader-only `propose_remove_voter` helper, this allows a
    /// follower with a known leader to forward its own graceful-drain removal.
    pub fn request_remove_voter(&self, raft_node_id: u64) -> CacheResult<Vec<RaftWireMessage>> {
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        state.request_voter_change(raft_node_id, ConfChangeType::RemoveNode)
    }

    /// Return a runtime snapshot.
    pub fn snapshot(&self) -> RaftMetadataRuntimeSnapshot {
        let state = self.raft.lock().expect("raft metadata state poisoned");
        let hard_state = state.raw_node.raft.hard_state();
        RaftMetadataRuntimeSnapshot {
            raft_node_id: self.raft_node_id,
            term: state.raw_node.raft.term,
            commit_index: hard_state.commit,
            applied_index: state.applied_index,
            role: state.raw_node.raft.state.into(),
            commands_committed: state.commands.len(),
            snapshot_installs: state.snapshot_installs,
            last_command: state
                .commands
                .last()
                .map(|envelope| envelope.command.clone()),
            duplicate_commands: state
                .results
                .iter()
                .filter(|result| result.status == RaftCommandStatus::Duplicate)
                .count(),
            last_result: state.results.last().cloned(),
        }
    }

    /// Export the applied metadata snapshot for in-memory recovery.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
    /// use hydracache_cluster_raft::RaftMetadataRuntime;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let runtime = RaftMetadataRuntime::single_node("orders", 1)?;
    /// runtime
    ///     .join_member(
    ///         ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
    ///     )
    ///     .await?;
    ///
    /// let exported = runtime.export_snapshot();
    /// let recovered = RaftMetadataRuntime::from_snapshot(exported)?;
    ///
    /// assert_eq!(recovered.snapshot().commands_committed, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn export_snapshot(&self) -> RaftMetadataRuntimeExport {
        let state = self.raft.lock().expect("raft metadata state poisoned");
        RaftMetadataRuntimeExport {
            cluster_name: self.cluster.name().to_owned(),
            raft_node_id: self.raft_node_id,
            applied_index: state.applied_index,
            commands: state.commands.clone(),
        }
    }

    fn restore_export(&self, snapshot: RaftMetadataRuntimeExport) -> CacheResult<()> {
        validate_snapshot_apply_contract(&snapshot)?;
        let snapshot_index = snapshot.applied_index;
        {
            let mut state = self.raft.lock().expect("raft metadata state poisoned");
            state.commands.clear();
            state.applied_command_ids.clear();
            state.results.clear();
            state.applied_index = snapshot.applied_index;
        }
        for (offset, envelope) in snapshot.commands.into_iter().enumerate() {
            let tail_index = offset + 1;
            let command_id = envelope.command_id.clone();
            self.apply_snapshot_envelope(envelope).map_err(|error| {
                snapshot_apply_error(snapshot_index, tail_index, &command_id, error)
            })?;
        }
        Ok(())
    }

    fn restore_committed_entries(&self, entries: Vec<Entry>, commit_index: u64) -> CacheResult<()> {
        {
            let mut state = self.raft.lock().expect("raft metadata state poisoned");
            state.commands.clear();
            state.applied_command_ids.clear();
            state.results.clear();
            state.applied_index = 0;
        }
        for entry in entries {
            if entry.index > commit_index
                || entry.data.is_empty()
                || entry.get_entry_type() != EntryType::EntryNormal
            {
                continue;
            }
            self.apply_recovered_envelope_at(decode_envelope(entry.data.as_ref())?, entry.index)?;
        }
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        state.applied_index = state.applied_index.max(commit_index);
        Ok(())
    }

    fn commit_command(
        &self,
        command_id: String,
        command: RaftMetadataCommand,
    ) -> CacheResult<RaftCommandResult> {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .commit_command(RaftMetadataCommandEnvelope {
                command_id,
                command,
            })
    }

    async fn wait_for_forwarded_apply(&self, result: &RaftCommandResult) -> CacheResult<()> {
        if result.status != RaftCommandStatus::Forwarded {
            return Ok(());
        }
        for _ in 0..FORWARDED_APPLY_WAIT_ATTEMPTS {
            if self.command_applied(&result.command_id) {
                return Ok(());
            }
            sleep(FORWARDED_APPLY_WAIT_INTERVAL).await;
        }
        Err(CacheError::Backend(format!(
            "raft metadata command {} was forwarded but not applied locally before timeout",
            result.command_id
        )))
    }

    fn propose_voter_change(
        &self,
        raft_node_id: u64,
        change_type: ConfChangeType,
    ) -> CacheResult<Vec<RaftWireMessage>> {
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        state.propose_voter_change(raft_node_id, change_type)
    }

    fn apply_snapshot_envelope(&self, envelope: RaftMetadataCommandEnvelope) -> CacheResult<()> {
        materialize_snapshot_command(&self.cluster, &envelope.command)?;
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        if !state
            .applied_command_ids
            .insert(envelope.command_id.clone())
        {
            return Err(CacheError::Backend(format!(
                "duplicate raft snapshot command id '{}'",
                envelope.command_id
            )));
        }
        state.commands.push(envelope);
        Ok(())
    }

    fn apply_recovered_envelope_at(
        &self,
        envelope: RaftMetadataCommandEnvelope,
        index: u64,
    ) -> CacheResult<()> {
        materialize_committed_command(&self.cluster, &envelope.command)?;
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        if state
            .applied_command_ids
            .insert(envelope.command_id.clone())
        {
            state.commands.push(envelope);
        }
        state.applied_index = state.applied_index.max(index);
        Ok(())
    }

    fn persist_metadata(&self) -> CacheResult<()> {
        if let Some(store) = &self.metadata_store {
            store.save(self.export_snapshot())?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn fail_next_proposal_for_test(&self) {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .fail_next_proposal = true;
    }
}

fn command_id_for(command: &RaftMetadataCommand) -> String {
    match command {
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            ..
        } => format!(
            "member-upsert:{}:{}",
            command_id_node(node_id),
            generation.value()
        ),
        RaftMetadataCommand::ClientUpsert {
            node_id,
            generation,
            ..
        } => format!(
            "client-upsert:{}:{}",
            command_id_node(node_id),
            generation.value()
        ),
        RaftMetadataCommand::NodeLeft { node_id, epoch, .. } => {
            format!("node-left:{}:{}", command_id_node(node_id), epoch.value())
        }
        RaftMetadataCommand::CommitTopology { epoch, members } => format!(
            "commit-topology:{}:{}",
            epoch.value(),
            members
                .iter()
                .map(command_id_node)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn validate_snapshot_identity(
    config: &RaftMetadataRuntimeConfig,
    snapshot: &RaftMetadataRuntimeExport,
) -> CacheResult<()> {
    if snapshot.cluster_name != config.cluster_name {
        return Err(CacheError::Backend(format!(
            "raft metadata store snapshot cluster '{}' does not match configured cluster '{}'",
            snapshot.cluster_name, config.cluster_name
        )));
    }
    if snapshot.raft_node_id != config.raft_node_id {
        return Err(CacheError::Backend(format!(
            "raft metadata store snapshot node {} does not match configured node {}",
            snapshot.raft_node_id, config.raft_node_id
        )));
    }
    Ok(())
}

fn validate_snapshot_apply_contract(snapshot: &RaftMetadataRuntimeExport) -> CacheResult<()> {
    if snapshot.applied_index < snapshot.commands.len() as u64 {
        let tail_index = snapshot.applied_index.saturating_add(1).max(1) as usize;
        let command_id = snapshot
            .commands
            .get(tail_index.saturating_sub(1))
            .or_else(|| snapshot.commands.last())
            .map(|envelope| envelope.command_id.as_str())
            .unwrap_or("<none>");
        return Err(CacheError::Backend(format!(
            "raft snapshot apply error: inconsistent snapshot membership indexes: snapshot_index={}, command_count={}, tail_index={}, command_id={}",
            snapshot.applied_index,
            snapshot.commands.len(),
            tail_index,
            command_id
        )));
    }

    let mut seen = BTreeSet::new();
    for (offset, envelope) in snapshot.commands.iter().enumerate() {
        let tail_index = offset + 1;
        if envelope.command_id.is_empty() {
            return Err(CacheError::Backend(format!(
                "raft snapshot apply error: empty command id: snapshot_index={}, tail_index={}",
                snapshot.applied_index, tail_index
            )));
        }
        let expected = command_id_for(&envelope.command);
        if envelope.command_id != expected {
            return Err(CacheError::Backend(format!(
                "raft snapshot apply error: command id does not match command: snapshot_index={}, tail_index={}, command_id={}, expected_command_id={}",
                snapshot.applied_index, tail_index, envelope.command_id, expected
            )));
        }
        if !seen.insert(envelope.command_id.as_str()) {
            return Err(CacheError::Backend(format!(
                "raft snapshot apply error: duplicate command id: snapshot_index={}, tail_index={}, command_id={}",
                snapshot.applied_index, tail_index, envelope.command_id
            )));
        }
    }
    Ok(())
}

fn command_id_node(node_id: &ClusterNodeId) -> String {
    node_id.as_str().replace('|', "%7C").replace(':', "%3A")
}

fn predicted_member_epoch(cluster: &InMemoryCluster, candidate: &ClusterCandidate) -> ClusterEpoch {
    let should_advance = cluster
        .members()
        .into_iter()
        .find(|member| member.node_id == candidate.node_id)
        .map(|existing| existing.generation < candidate.generation)
        .unwrap_or(true);
    if should_advance {
        ClusterEpoch::new(cluster.epoch().value().saturating_add(1))
    } else {
        cluster.epoch()
    }
}

fn predicted_leave_epoch(
    cluster: &InMemoryCluster,
    node_id: &ClusterNodeId,
) -> Option<(ClusterRole, ClusterEpoch)> {
    if let Some(member) = cluster
        .members()
        .into_iter()
        .find(|member| &member.node_id == node_id)
    {
        return Some((
            member.role,
            ClusterEpoch::new(cluster.epoch().value().saturating_add(1)),
        ));
    }
    cluster
        .clients()
        .into_iter()
        .find(|member| &member.node_id == node_id)
        .map(|member| (member.role, cluster.epoch()))
}

fn reject_stale_candidate(
    cluster: &InMemoryCluster,
    candidate: &ClusterCandidate,
) -> CacheResult<()> {
    let existing = cluster
        .members()
        .into_iter()
        .chain(cluster.clients())
        .find(|member| member.node_id == candidate.node_id);
    if let Some(existing) = existing {
        if existing.generation > candidate.generation {
            return Err(CacheError::Backend(format!(
                "stale cluster generation for node '{}': existing {}, attempted {}",
                candidate.node_id,
                existing.generation.value(),
                candidate.generation.value()
            )));
        }
    }
    Ok(())
}

fn find_materialized(
    cluster: &InMemoryCluster,
    node_id: &ClusterNodeId,
    role: ClusterRole,
) -> Option<ClusterMember> {
    match role {
        ClusterRole::Member => cluster
            .members()
            .into_iter()
            .find(|member| &member.node_id == node_id),
        ClusterRole::Client => cluster
            .clients()
            .into_iter()
            .find(|member| &member.node_id == node_id),
        ClusterRole::Local => None,
    }
}

fn materialize_command(
    cluster: &InMemoryCluster,
    command: &RaftMetadataCommand,
) -> CacheResult<Option<ClusterMembershipEvent>> {
    match command {
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            ..
        } => {
            cluster
                .join_member(ClusterCandidate::member(node_id.clone()).generation(*generation))?;
            Ok(None)
        }
        RaftMetadataCommand::ClientUpsert {
            node_id,
            generation,
            ..
        } => {
            cluster
                .join_client(ClusterCandidate::client(node_id.clone()).generation(*generation))?;
            Ok(None)
        }
        RaftMetadataCommand::NodeLeft { node_id, .. } => {
            let generation = cluster
                .members()
                .into_iter()
                .chain(cluster.clients())
                .find(|member| member.node_id == *node_id)
                .map(|member| member.generation);
            if let Some(generation) = generation {
                cluster.leave(node_id, generation)
            } else {
                Ok(None)
            }
        }
        RaftMetadataCommand::CommitTopology { .. } => Ok(None),
    }
}

fn materialize_snapshot_command(
    cluster: &InMemoryCluster,
    command: &RaftMetadataCommand,
) -> CacheResult<Option<ClusterMembershipEvent>> {
    if let RaftMetadataCommand::NodeLeft { node_id, role, .. } = command {
        let present = find_materialized(cluster, node_id, *role).is_some();
        if !present {
            return Err(CacheError::Backend(format!(
                "node-left references absent {:?} '{}'",
                role, node_id
            )));
        }
    }
    materialize_command(cluster, command)
}

fn snapshot_apply_error(
    snapshot_index: u64,
    tail_index: usize,
    command_id: &str,
    error: CacheError,
) -> CacheError {
    CacheError::Backend(format!(
        "raft snapshot apply error: snapshot_index={}, tail_index={}, command_id={}: {}",
        snapshot_index, tail_index, command_id, error
    ))
}

fn materialize_committed_command(
    cluster: &InMemoryCluster,
    command: &RaftMetadataCommand,
) -> CacheResult<()> {
    match command {
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            ..
        } => {
            if find_materialized(cluster, node_id, ClusterRole::Member)
                .is_some_and(|member| member.generation >= *generation)
            {
                return Ok(());
            }
        }
        RaftMetadataCommand::ClientUpsert {
            node_id,
            generation,
            ..
        } => {
            if find_materialized(cluster, node_id, ClusterRole::Client)
                .is_some_and(|member| member.generation >= *generation)
            {
                return Ok(());
            }
        }
        RaftMetadataCommand::NodeLeft { node_id, .. } => {
            let present = find_materialized(cluster, node_id, ClusterRole::Member).is_some()
                || find_materialized(cluster, node_id, ClusterRole::Client).is_some();
            if !present {
                return Ok(());
            }
        }
        RaftMetadataCommand::CommitTopology { .. } => return Ok(()),
    }
    materialize_command(cluster, command).map(|_| ())
}

impl<S> RaftRuntimeState<S>
where
    S: RaftLogStore,
{
    fn propose_voter_change(
        &mut self,
        raft_node_id: u64,
        change_type: ConfChangeType,
    ) -> CacheResult<Vec<RaftWireMessage>> {
        if self.raw_node.raft.state != StateRole::Leader {
            return Err(CacheError::Backend(
                "raft voter changes must be proposed by the leader".to_owned(),
            ));
        }
        let mut change = ConfChange {
            node_id: raft_node_id.max(1),
            ..ConfChange::default()
        };
        change.set_change_type(change_type);
        self.raw_node
            .propose_conf_change(Vec::new(), change)
            .map_err(to_cache_error)?;
        self.drain_ready()
    }

    fn request_voter_change(
        &mut self,
        raft_node_id: u64,
        change_type: ConfChangeType,
    ) -> CacheResult<Vec<RaftWireMessage>> {
        if self.raw_node.raft.state != StateRole::Leader
            && known_leader_id(self.raw_node.raft.leader_id).is_none()
        {
            return Err(CacheError::Backend(
                "raft voter changes require a known leader".to_owned(),
            ));
        }
        let mut change = ConfChange {
            node_id: raft_node_id.max(1),
            ..ConfChange::default()
        };
        change.set_change_type(change_type);
        self.raw_node
            .propose_conf_change(Vec::new(), change)
            .map_err(to_cache_error)?;
        self.drain_ready()
    }

    fn commit_command(
        &mut self,
        envelope: RaftMetadataCommandEnvelope,
    ) -> CacheResult<RaftCommandResult> {
        if self.applied_command_ids.contains(&envelope.command_id) {
            let result = RaftCommandResult {
                command_id: envelope.command_id,
                status: RaftCommandStatus::Duplicate,
                applied_index: self.applied_index,
            };
            self.results.push(result.clone());
            return Ok(result);
        }
        #[cfg(test)]
        if self.fail_next_proposal {
            self.fail_next_proposal = false;
            return Err(CacheError::Backend(
                "forced raft proposal failure".to_owned(),
            ));
        }
        let command_id = envelope.command_id.clone();
        if self.raw_node.raft.state != StateRole::Leader
            && known_leader_id(self.raw_node.raft.leader_id).is_none()
        {
            return Err(CacheError::Backend(
                "no raft leader; retry metadata proposal after election".to_owned(),
            ));
        }
        self.raw_node
            .propose(vec![], encode_envelope(&envelope))
            .map_err(to_cache_error)?;
        let outbound = self.drain_ready()?;
        self.outbound_messages.extend(outbound);
        let status = if self.applied_command_ids.contains(&command_id) {
            RaftCommandStatus::Committed
        } else {
            RaftCommandStatus::Forwarded
        };
        let result = RaftCommandResult {
            command_id,
            status,
            applied_index: self.applied_index,
        };
        self.results.push(result.clone());
        Ok(result)
    }

    #[cfg(feature = "test-failpoints")]
    fn compact_applied_log_to_snapshot_for_tests(&mut self, raft_node_id: u64) -> CacheResult<u64> {
        if self.applied_index == 0 {
            return Err(CacheError::Backend(
                "cannot compact raft metadata log before any entry is applied".to_owned(),
            ));
        }
        let store = self.raw_node.raft.raft_log.store.clone();
        let term = store
            .term(self.applied_index)
            .unwrap_or(self.raw_node.raft.term);
        let conf_state = store
            .initial_state()
            .map_err(to_cache_error)?
            .conf_state
            .clone();
        let export = RaftMetadataRuntimeExport {
            cluster_name: self.cluster.name().to_owned(),
            raft_node_id,
            applied_index: self.applied_index,
            commands: self.commands.clone(),
        };
        let mut snapshot = Snapshot::default();
        snapshot.mut_metadata().index = self.applied_index;
        snapshot.mut_metadata().term = term;
        snapshot.mut_metadata().set_conf_state(conf_state);
        snapshot.data = encode_metadata_snapshot_payload(&export)?.into();
        store
            .save_snapshot(&snapshot, usize::MAX)
            .map_err(to_cache_error)?;
        Ok(self.applied_index)
    }

    fn drain_ready(&mut self) -> CacheResult<Vec<RaftWireMessage>> {
        let mut outbound = Vec::new();
        while self.raw_node.has_ready() {
            let store = self.raw_node.raft.raft_log.store.clone();
            let mut ready = self.raw_node.ready();

            if !ready.snapshot().is_empty() {
                store
                    .save_snapshot(ready.snapshot(), 0)
                    .map_err(to_cache_error)?;
                #[cfg(feature = "test-failpoints")]
                fail::fail_point!("raft_after_save_snapshot_before_entries", |_| {
                    Err(CacheError::Backend(
                        "injected crash after raft snapshot save before entries".to_owned(),
                    ))
                });
                #[cfg(feature = "test-failpoints")]
                fail::fail_point!("raft_after_install_snapshot_before_apply", |_| {
                    Err(CacheError::Backend(
                        "injected crash after raft snapshot install before apply".to_owned(),
                    ))
                });
                self.install_metadata_snapshot(ready.snapshot())?;
            }

            let committed_entries = ready.take_committed_entries();
            outbound.extend(ready.take_messages());
            outbound.extend(ready.take_persisted_messages());

            if !ready.entries().is_empty() {
                store.append(ready.entries()).map_err(to_cache_error)?;
            }

            if let Some(hard_state) = ready.hs() {
                store.save_hard_state(hard_state).map_err(to_cache_error)?;
                #[cfg(feature = "test-failpoints")]
                fail::fail_point!("raft_after_save_hard_state_before_send", |_| {
                    Err(CacheError::Backend(
                        "injected crash after raft hard state save before send".to_owned(),
                    ))
                });
            }

            self.apply_committed_entries(committed_entries)?;

            let mut light_ready = self.raw_node.advance(ready);
            if let Some(commit) = light_ready.commit_index() {
                store.set_commit(commit).map_err(to_cache_error)?;
            }
            self.apply_committed_entries(light_ready.take_committed_entries())?;
            outbound.extend(light_ready.take_messages());
            store.mark_applied(self.applied_index);
            self.raw_node.advance_apply();
        }
        outbound
            .into_iter()
            .map(|message| RaftWireMessage::encode(&message))
            .collect()
    }

    fn apply_committed_entries(&mut self, entries: Vec<Entry>) -> CacheResult<()> {
        for entry in entries {
            self.applied_index = self.applied_index.max(entry.index);
            if entry.data.is_empty() {
                continue;
            }
            match entry.get_entry_type() {
                EntryType::EntryNormal => {
                    let envelope = decode_envelope(entry.data.as_ref())?;
                    if self.applied_command_ids.insert(envelope.command_id.clone()) {
                        materialize_committed_command(&self.cluster, &envelope.command)?;
                        self.commands.push(envelope);
                        self.applied_index = self.applied_index.max(self.commands.len() as u64);
                    }
                }
                EntryType::EntryConfChange => {
                    let change =
                        ConfChange::parse_from_bytes(entry.data.as_ref()).map_err(|error| {
                            CacheError::Decode(format!(
                                "failed to decode raft conf change: {error}"
                            ))
                        })?;
                    let conf_state = self
                        .raw_node
                        .apply_conf_change(&change)
                        .map_err(to_cache_error)?;
                    #[cfg(feature = "test-failpoints")]
                    fail::fail_point!("canary_raft_skip_save_conf_state", |_| { Ok(()) });
                    #[cfg(feature = "test-failpoints")]
                    fail::fail_point!("raft_before_save_conf_state", |_| {
                        Err(CacheError::Backend(
                            "injected crash before raft conf state save".to_owned(),
                        ))
                    });
                    self.raw_node
                        .raft
                        .raft_log
                        .store
                        .save_conf_state(&conf_state)
                        .map_err(to_cache_error)?;
                }
                EntryType::EntryConfChangeV2 => {
                    let change =
                        ConfChangeV2::parse_from_bytes(entry.data.as_ref()).map_err(|error| {
                            CacheError::Decode(format!(
                                "failed to decode raft conf change v2: {error}"
                            ))
                        })?;
                    let conf_state = self
                        .raw_node
                        .apply_conf_change(&change)
                        .map_err(to_cache_error)?;
                    #[cfg(feature = "test-failpoints")]
                    fail::fail_point!("canary_raft_skip_save_conf_state", |_| { Ok(()) });
                    #[cfg(feature = "test-failpoints")]
                    fail::fail_point!("raft_before_save_conf_state", |_| {
                        Err(CacheError::Backend(
                            "injected crash before raft conf state save".to_owned(),
                        ))
                    });
                    self.raw_node
                        .raft
                        .raft_log
                        .store
                        .save_conf_state(&conf_state)
                        .map_err(to_cache_error)?;
                }
            }
        }
        Ok(())
    }

    fn install_metadata_snapshot(&mut self, snapshot: &Snapshot) -> CacheResult<()> {
        if snapshot.data.is_empty() {
            return Ok(());
        }
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("raft_install_snapshot_oom", |_| {
            Err(CacheError::Backend(
                "injected OOM during raft snapshot install".to_owned(),
            ))
        });
        let export = decode_metadata_snapshot_payload(snapshot.data.as_ref())?;
        if export.cluster_name != self.cluster.name() {
            return Err(CacheError::Backend(format!(
                "raft metadata snapshot cluster '{}' does not match runtime cluster '{}'",
                export.cluster_name,
                self.cluster.name()
            )));
        }
        validate_snapshot_apply_contract(&export)?;
        self.commands.clear();
        self.applied_command_ids.clear();
        self.results.clear();
        self.applied_index = export.applied_index;
        for envelope in export.commands {
            materialize_snapshot_command(&self.cluster, &envelope.command)?;
            if !self.applied_command_ids.insert(envelope.command_id.clone()) {
                return Err(CacheError::Backend(format!(
                    "duplicate raft snapshot command id '{}'",
                    envelope.command_id
                )));
            }
            self.commands.push(envelope);
        }
        self.snapshot_installs = self.snapshot_installs.saturating_add(1);
        Ok(())
    }
}

#[async_trait::async_trait]
impl<S> ClusterControlPlane for RaftMetadataRuntime<S>
where
    S: RaftLogStore,
{
    fn name(&self) -> String {
        self.cluster.name().to_owned()
    }

    fn invalidation_bus(&self) -> Arc<dyn CacheInvalidationBus> {
        self.cluster.invalidation_bus()
    }

    async fn join_member(&self, candidate: ClusterCandidate) -> CacheResult<ClusterMember> {
        let mut candidate = candidate;
        candidate.role = ClusterRole::Member;
        reject_stale_candidate(&self.cluster, &candidate)?;
        let command = RaftMetadataCommand::MemberUpsert {
            node_id: candidate.node_id.clone(),
            generation: candidate.generation,
            epoch: predicted_member_epoch(&self.cluster, &candidate),
        };
        let result = self.commit_command(command_id_for(&command), command)?;
        if result.status == RaftCommandStatus::Duplicate {
            if let Some(member) =
                find_materialized(&self.cluster, &candidate.node_id, ClusterRole::Member)
            {
                return Ok(member);
            }
        }
        self.wait_for_forwarded_apply(&result).await?;
        if let Some(member) =
            find_materialized(&self.cluster, &candidate.node_id, ClusterRole::Member)
        {
            self.persist_metadata()?;
            return Ok(member);
        }
        Err(CacheError::Backend(format!(
            "committed raft metadata command {} did not materialize member {}",
            result.command_id, candidate.node_id
        )))
    }

    async fn join_client(&self, candidate: ClusterCandidate) -> CacheResult<ClusterMember> {
        let mut candidate = candidate;
        candidate.role = ClusterRole::Client;
        reject_stale_candidate(&self.cluster, &candidate)?;
        let command = RaftMetadataCommand::ClientUpsert {
            node_id: candidate.node_id.clone(),
            generation: candidate.generation,
            epoch: self.cluster.epoch(),
        };
        let result = self.commit_command(command_id_for(&command), command)?;
        if result.status == RaftCommandStatus::Duplicate {
            if let Some(member) =
                find_materialized(&self.cluster, &candidate.node_id, ClusterRole::Client)
            {
                return Ok(member);
            }
        }
        self.wait_for_forwarded_apply(&result).await?;
        if let Some(member) =
            find_materialized(&self.cluster, &candidate.node_id, ClusterRole::Client)
        {
            self.persist_metadata()?;
            return Ok(member);
        }
        Err(CacheError::Backend(format!(
            "committed raft metadata command {} did not materialize client {}",
            result.command_id, candidate.node_id
        )))
    }

    async fn validate_generation(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> CacheResult<()> {
        self.cluster.validate_generation(node_id, generation)
    }

    async fn leave(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> CacheResult<Option<ClusterMembershipEvent>> {
        if predicted_leave_epoch(&self.cluster, node_id).is_none() {
            return Ok(None);
        };
        self.cluster.validate_generation(node_id, generation)?;
        let Some((role, epoch)) = predicted_leave_epoch(&self.cluster, node_id) else {
            return Ok(None);
        };
        let command = RaftMetadataCommand::NodeLeft {
            node_id: node_id.clone(),
            role,
            epoch,
        };
        let result = self.commit_command(command_id_for(&command), command)?;
        if result.status == RaftCommandStatus::Duplicate {
            return Ok(None);
        }
        self.wait_for_forwarded_apply(&result).await?;
        if predicted_leave_epoch(&self.cluster, node_id).is_none() {
            self.persist_metadata()?;
            return Ok(Some(ClusterMembershipEvent::NodeLeft {
                node_id: node_id.clone(),
                role,
                epoch,
            }));
        }
        Err(CacheError::Backend(format!(
            "committed raft metadata command {} did not materialize leave for {}",
            result.command_id, node_id
        )))
    }

    fn subscribe_membership(&self) -> ClusterMembershipSubscriber {
        self.cluster.subscribe_membership()
    }

    fn diagnostics_for(
        &self,
        role: ClusterRole,
        node_id: ClusterNodeId,
        generation: ClusterGeneration,
        bootstrap: Vec<String>,
    ) -> ClusterDiagnostics {
        self.cluster
            .diagnostics_for(role, node_id, generation, bootstrap)
    }
}

fn encode_command(command: &RaftMetadataCommand) -> Vec<u8> {
    match command {
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            epoch,
        } => format!("member|{node_id}|{}|{}", generation.value(), epoch.value()).into_bytes(),
        RaftMetadataCommand::ClientUpsert {
            node_id,
            generation,
            epoch,
        } => format!("client|{node_id}|{}|{}", generation.value(), epoch.value()).into_bytes(),
        RaftMetadataCommand::NodeLeft {
            node_id,
            role,
            epoch,
        } => format!("left|{node_id}|{}|{}", role_to_str(*role), epoch.value()).into_bytes(),
        RaftMetadataCommand::CommitTopology { epoch, members } => format!(
            "topology|{}|{}",
            epoch.value(),
            members
                .iter()
                .map(ClusterNodeId::as_str)
                .collect::<Vec<_>>()
                .join(",")
        )
        .into_bytes(),
    }
}

fn encode_envelope(envelope: &RaftMetadataCommandEnvelope) -> Vec<u8> {
    let command = String::from_utf8(encode_command(&envelope.command))
        .expect("raft metadata command encoding is utf8");
    format!("v1|{}|{command}", envelope.command_id).into_bytes()
}

fn decode_envelope(data: &[u8]) -> CacheResult<RaftMetadataCommandEnvelope> {
    let text = std::str::from_utf8(data)
        .map_err(|error| CacheError::Backend(format!("invalid raft envelope utf8: {error}")))?;
    if let Some(rest) = text.strip_prefix("v1|") {
        let Some((command_id, command_text)) = rest.split_once('|') else {
            return Err(CacheError::Backend(format!(
                "invalid raft metadata envelope: {text}"
            )));
        };
        return Ok(RaftMetadataCommandEnvelope {
            command_id: command_id.to_owned(),
            command: decode_command(command_text.as_bytes())?,
        });
    }
    let command = decode_command(data)?;
    Ok(RaftMetadataCommandEnvelope {
        command_id: command_id_for(&command),
        command,
    })
}

fn decode_command(data: &[u8]) -> CacheResult<RaftMetadataCommand> {
    let text = std::str::from_utf8(data)
        .map_err(|error| CacheError::Backend(format!("invalid raft command utf8: {error}")))?;
    let parts = text.split('|').collect::<Vec<_>>();
    match parts.as_slice() {
        ["member", node_id, generation, epoch] => Ok(RaftMetadataCommand::MemberUpsert {
            node_id: ClusterNodeId::from((*node_id).to_owned()),
            generation: ClusterGeneration::new(parse_u64(generation, "generation")?),
            epoch: hydracache::ClusterEpoch::new(parse_u64(epoch, "epoch")?),
        }),
        ["client", node_id, generation, epoch] => Ok(RaftMetadataCommand::ClientUpsert {
            node_id: ClusterNodeId::from((*node_id).to_owned()),
            generation: ClusterGeneration::new(parse_u64(generation, "generation")?),
            epoch: hydracache::ClusterEpoch::new(parse_u64(epoch, "epoch")?),
        }),
        ["left", node_id, role, epoch] => Ok(RaftMetadataCommand::NodeLeft {
            node_id: ClusterNodeId::from((*node_id).to_owned()),
            role: parse_role(role)?,
            epoch: hydracache::ClusterEpoch::new(parse_u64(epoch, "epoch")?),
        }),
        ["topology", epoch, members] => Ok(RaftMetadataCommand::CommitTopology {
            epoch: hydracache::ClusterEpoch::new(parse_u64(epoch, "epoch")?),
            members: members
                .split(',')
                .filter(|member| !member.is_empty())
                .map(|member| ClusterNodeId::from(member.to_owned()))
                .collect(),
        }),
        _ => Err(CacheError::Backend(format!(
            "invalid raft metadata command: {text}"
        ))),
    }
}

fn parse_u64(value: &str, label: &str) -> CacheResult<u64> {
    value
        .parse::<u64>()
        .map_err(|error| CacheError::Backend(format!("invalid {label} in raft command: {error}")))
}

fn role_to_str(role: ClusterRole) -> &'static str {
    match role {
        ClusterRole::Local => "local",
        ClusterRole::Client => "client",
        ClusterRole::Member => "member",
    }
}

fn parse_role(value: &str) -> CacheResult<ClusterRole> {
    match value {
        "local" => Ok(ClusterRole::Local),
        "client" => Ok(ClusterRole::Client),
        "member" => Ok(ClusterRole::Member),
        _ => Err(CacheError::Backend(format!(
            "invalid raft metadata role: {value}"
        ))),
    }
}

fn known_leader_id(leader_id: u64) -> Option<u64> {
    if leader_id == 0 {
        None
    } else {
        Some(leader_id)
    }
}

fn to_cache_error(error: impl fmt::Display) -> CacheError {
    CacheError::Backend(format!("raft metadata runtime failed: {error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hydracache::{ClusterControlPlane, HydraCache, InMemoryCluster};

    use super::*;

    #[test]
    fn runtime_campaigns_single_node_to_leader() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        let snapshot = runtime.snapshot();

        assert_eq!(snapshot.raft_node_id, 1);
        assert_eq!(snapshot.role, RaftRuntimeRole::Leader);
        assert_eq!(runtime.leader_id(), Some(1));
        assert_eq!(snapshot.commands_committed, 0);

        let non_default_id = RaftMetadataRuntime::single_node("billing", 7).unwrap();
        assert_eq!(non_default_id.leader_id(), Some(7));
    }

    #[test]
    fn leader_id_maps_zero_soft_state_to_none() {
        assert_eq!(known_leader_id(0), None);
        assert_eq!(known_leader_id(7), Some(7));
    }

    #[tokio::test]
    async fn member_and_client_admission_are_committed_through_raft() {
        let runtime = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());

        let member = HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(1))
            .start()
            .await
            .unwrap();
        let client = HydraCache::client()
            .control_plane(runtime.clone())
            .node_id("client-a")
            .generation(ClusterGeneration::new(1))
            .connect()
            .await
            .unwrap();

        let snapshot = runtime.snapshot();
        assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
        assert_eq!(client.cluster_diagnostics().unwrap().client_count, 1);
        assert_eq!(snapshot.role, RaftRuntimeRole::Leader);
        assert_eq!(snapshot.commands_committed, 2);
        assert!(snapshot.commit_index >= 3);
        assert!(matches!(
            &runtime.commands()[0],
            RaftMetadataCommand::MemberUpsert { node_id, .. } if node_id.as_str() == "member-a"
        ));
        assert!(matches!(
            snapshot.last_command,
            Some(RaftMetadataCommand::ClientUpsert { ref node_id, .. })
                if node_id.as_str() == "client-a"
        ));
    }

    #[tokio::test]
    async fn command_idempotency_prevents_duplicate_admission_after_retry() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        let candidate = ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1));

        let first = runtime.join_member(candidate.clone()).await.unwrap();
        let second = runtime.join_member(candidate).await.unwrap();
        let snapshot = runtime.snapshot();

        assert_eq!(first.node_id, second.node_id);
        assert_eq!(snapshot.commands_committed, 1);
        assert_eq!(snapshot.duplicate_commands, 1);
        assert_eq!(
            snapshot.last_result.map(|result| result.status),
            Some(RaftCommandStatus::Duplicate)
        );
        assert_eq!(runtime.command_results().len(), 2);
        assert_eq!(runtime.cluster.members().len(), 1);
    }

    #[tokio::test]
    async fn runtime_recovers_materialized_metadata_from_exported_snapshot() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();

        runtime
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap();
        runtime
            .join_client(ClusterCandidate::client("client-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap();
        runtime
            .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(1))
            .await
            .unwrap();

        let exported = runtime.export_snapshot();
        let recovered = RaftMetadataRuntime::from_snapshot(exported).unwrap();

        assert_eq!(recovered.commands(), runtime.commands());
        assert_eq!(recovered.cluster.members().len(), 0);
        assert_eq!(recovered.cluster.clients().len(), 1);
        assert_eq!(
            recovered
                .cluster
                .clients()
                .first()
                .map(|client| client.node_id.as_str().to_owned()),
            Some("client-a".to_owned())
        );
        assert_eq!(
            recovered.snapshot().applied_index,
            runtime.snapshot().applied_index
        );
    }

    #[tokio::test]
    async fn metadata_store_saves_committed_membership_and_recovers_runtime() {
        let store = Arc::new(InMemoryRaftMetadataStore::new());
        let runtime = RaftMetadataRuntime::with_config_and_metadata_store(
            RaftMetadataRuntimeConfig::single_node("orders", 1),
            store.clone(),
        )
        .unwrap();

        runtime
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap();
        runtime
            .join_client(ClusterCandidate::client("client-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap();

        let stored = store.snapshot().expect("snapshot saved");
        assert_eq!(stored.cluster_name, "orders");
        assert_eq!(stored.raft_node_id, 1);
        assert_eq!(stored.commands.len(), 2);

        let recovered = RaftMetadataRuntime::with_config_and_metadata_store(
            RaftMetadataRuntimeConfig::single_node("orders", 1),
            store,
        )
        .unwrap();

        assert_eq!(recovered.snapshot().commands_committed, 2);
        assert_eq!(recovered.cluster.members().len(), 1);
        assert_eq!(recovered.cluster.clients().len(), 1);
    }

    #[test]
    fn metadata_store_rejects_snapshot_for_another_cluster_or_node() {
        let snapshot = RaftMetadataRuntimeExport {
            cluster_name: "orders".to_owned(),
            raft_node_id: 1,
            applied_index: 0,
            commands: Vec::new(),
        };
        let wrong_cluster = Arc::new(InMemoryRaftMetadataStore::with_snapshot(snapshot.clone()));
        let error = RaftMetadataRuntime::with_config_and_metadata_store(
            RaftMetadataRuntimeConfig::single_node("billing", 1),
            wrong_cluster,
        )
        .unwrap_err();
        assert!(error.to_string().contains("snapshot cluster"));

        let wrong_node = Arc::new(InMemoryRaftMetadataStore::with_snapshot(snapshot));
        let error = RaftMetadataRuntime::with_config_and_metadata_store(
            RaftMetadataRuntimeConfig::single_node("orders", 2),
            wrong_node,
        )
        .unwrap_err();
        assert!(error.to_string().contains("snapshot node"));
    }

    #[tokio::test]
    async fn metadata_store_does_not_save_failed_proposals() {
        let store = Arc::new(InMemoryRaftMetadataStore::new());
        let runtime = RaftMetadataRuntime::with_config_and_metadata_store(
            RaftMetadataRuntimeConfig::single_node("orders", 1),
            store.clone(),
        )
        .unwrap();
        runtime.fail_next_proposal_for_test();

        let result = runtime
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .await;

        assert!(result.is_err());
        assert!(store.snapshot().is_none());
        assert_eq!(runtime.snapshot().commands_committed, 0);
    }

    #[tokio::test]
    async fn failed_proposal_does_not_mutate_materialized_metadata() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        runtime.fail_next_proposal_for_test();

        let error = runtime
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("forced raft proposal failure"));
        assert_eq!(runtime.snapshot().commands_committed, 0);
        assert!(runtime.cluster.members().is_empty());
    }

    #[tokio::test]
    async fn stale_generation_rejection_does_not_commit_raft_command() {
        let runtime = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());

        HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(2))
            .start()
            .await
            .unwrap();

        let error = HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(1))
            .start()
            .await
            .unwrap_err();

        assert!(error.to_string().contains("stale cluster generation"));
        assert_eq!(runtime.snapshot().commands_committed, 1);
    }

    #[tokio::test]
    async fn leave_is_generation_safe_and_committed_through_raft() {
        let runtime = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());
        let stale = HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(1))
            .start()
            .await
            .unwrap();
        let current = HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(2))
            .start()
            .await
            .unwrap();

        let error = stale.leave_cluster().await.unwrap_err();
        assert!(error.to_string().contains("stale cluster generation"));
        assert_eq!(runtime.snapshot().commands_committed, 2);

        let left = current.leave_cluster().await.unwrap();
        assert!(left.is_some());
        assert_eq!(runtime.snapshot().commands_committed, 3);
        assert!(matches!(
            runtime.snapshot().last_command,
            Some(RaftMetadataCommand::NodeLeft { .. })
        ));
    }

    #[test]
    fn command_encoding_round_trips_without_json_dependency() {
        let commands = [
            RaftMetadataCommand::MemberUpsert {
                node_id: ClusterNodeId::from("member-a"),
                generation: ClusterGeneration::new(1),
                epoch: hydracache::ClusterEpoch::new(2),
            },
            RaftMetadataCommand::ClientUpsert {
                node_id: ClusterNodeId::from("client-a"),
                generation: ClusterGeneration::new(3),
                epoch: hydracache::ClusterEpoch::new(4),
            },
            RaftMetadataCommand::NodeLeft {
                node_id: ClusterNodeId::from("member-a"),
                role: ClusterRole::Member,
                epoch: hydracache::ClusterEpoch::new(5),
            },
        ];

        for command in commands {
            assert_eq!(decode_command(&encode_command(&command)).unwrap(), command);
            let envelope = RaftMetadataCommandEnvelope {
                command_id: command_id_for(&command),
                command: command.clone(),
            };
            assert_eq!(
                decode_envelope(&encode_envelope(&envelope)).unwrap(),
                envelope
            );
        }
    }

    #[test]
    fn command_decoding_reports_malformed_metadata() {
        assert!(decode_command(b"not|a|command")
            .unwrap_err()
            .to_string()
            .contains("invalid raft metadata command"));
        assert!(decode_command(b"member|member-a|nan|1")
            .unwrap_err()
            .to_string()
            .contains("invalid generation"));
        assert!(decode_command(b"left|member-a|unknown|1")
            .unwrap_err()
            .to_string()
            .contains("invalid raft metadata role"));
        assert!(decode_command(&[0xff])
            .unwrap_err()
            .to_string()
            .contains("invalid raft command utf8"));
        assert!(decode_envelope(b"v1|missing-command")
            .unwrap_err()
            .to_string()
            .contains("invalid raft metadata envelope"));

        let legacy = decode_envelope(b"client|client-a|1|2").unwrap();
        assert!(matches!(
            legacy.command,
            RaftMetadataCommand::ClientUpsert { ref node_id, .. }
                if node_id.as_str() == "client-a"
        ));
        assert_eq!(parse_role("local").unwrap(), ClusterRole::Local);
        assert_eq!(parse_role("client").unwrap(), ClusterRole::Client);
    }

    #[test]
    fn runtime_keeps_invalidation_bus_from_inner_cluster() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        let _subscriber = runtime.invalidation_bus().subscribe();

        let diagnostics = runtime.diagnostics_for(
            ClusterRole::Member,
            ClusterNodeId::from("member-a"),
            ClusterGeneration::new(1),
            Vec::new(),
        );

        assert_eq!(diagnostics.invalidation_subscribers, 1);
    }

    #[tokio::test]
    async fn runtime_accessors_and_unknown_leave_are_observable() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        assert_eq!(runtime.name(), "orders");
        assert!(format!("{:?}", runtime.snapshot()).contains("RaftMetadataRuntimeSnapshot"));

        let left = runtime
            .leave(&ClusterNodeId::from("missing"), ClusterGeneration::new(1))
            .await
            .unwrap();
        assert!(left.is_none());
        assert_eq!(runtime.snapshot().commands_committed, 0);

        runtime
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap();
        assert_eq!(runtime.command_envelopes().len(), 1);
        assert_eq!(runtime.command_results().len(), 1);
        assert!(format!("{runtime:?}").contains("RaftMetadataRuntime"));
    }

    #[test]
    fn config_clamps_small_values() {
        let config = RaftMetadataRuntimeConfig::single_node("orders", 0)
            .ticks(0, 0)
            .max_size_per_msg(0)
            .max_inflight_msgs(0);

        assert_eq!(config.raft_node_id, 1);
        assert_eq!(config.election_tick, 2);
        assert_eq!(config.heartbeat_tick, 1);
        assert_eq!(config.max_size_per_msg, 1);
        assert_eq!(config.max_inflight_msgs, 1);
    }

    #[test]
    fn runtime_config_constructors_keep_reviewed_transport_defaults() {
        let configs = [
            RaftMetadataRuntimeConfig::single_node("single", 1),
            RaftMetadataRuntimeConfig::multi_voter("multi", 1, [1, 2, 3]),
            RaftMetadataRuntimeConfig::try_joining("joining", 4, [1, 2, 3]).unwrap(),
        ];

        for config in configs {
            assert_eq!(config.max_size_per_msg, 1_048_576);
            assert_eq!(config.max_inflight_msgs, 256);
        }
    }

    #[test]
    fn raft_config_preserves_runtime_limits_and_fresh_applied_index() {
        let config = RaftMetadataRuntimeConfig::single_node("orders", 7)
            .ticks(17, 5)
            .max_size_per_msg(8_192)
            .max_inflight_msgs(19)
            .pre_vote(false)
            .raft_config();

        assert_eq!(config.id, 7);
        assert_eq!(config.election_tick, 17);
        assert_eq!(config.heartbeat_tick, 5);
        assert_eq!(config.max_size_per_msg, 8_192);
        assert_eq!(config.max_inflight_msgs, 19);
        assert!(!config.pre_vote);
        assert_eq!(config.applied, 0);
    }

    #[test]
    fn raft_runtime_state_debug_keeps_progress_context() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        let state = runtime.raft.lock().unwrap();
        let debug = format!("{state:?}");

        assert!(debug.contains("RaftRuntimeState"));
        assert!(debug.contains("commands"));
        assert!(debug.contains("applied_index"));
    }

    #[test]
    fn predicted_member_epoch_advances_only_for_newer_membership() {
        let cluster = InMemoryCluster::new("orders");
        let first = ClusterCandidate::member("member-a").generation(ClusterGeneration::new(3));
        assert_eq!(predicted_member_epoch(&cluster, &first).value(), 1);
        cluster.join_member(first).unwrap();
        let current_epoch = cluster.epoch();

        for generation in [2, 3] {
            let candidate =
                ClusterCandidate::member("member-a").generation(ClusterGeneration::new(generation));
            assert_eq!(predicted_member_epoch(&cluster, &candidate), current_epoch);
        }
        let newer = ClusterCandidate::member("member-a").generation(ClusterGeneration::new(4));
        assert_eq!(
            predicted_member_epoch(&cluster, &newer).value(),
            current_epoch.value() + 1
        );
        let different = ClusterCandidate::member("member-b").generation(ClusterGeneration::new(1));
        assert_eq!(
            predicted_member_epoch(&cluster, &different).value(),
            current_epoch.value() + 1
        );
    }

    #[test]
    fn committed_replay_accepts_an_already_newer_materialized_member() {
        let cluster = InMemoryCluster::new("orders");
        cluster
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(5)))
            .unwrap();
        let replay = RaftMetadataCommand::MemberUpsert {
            node_id: ClusterNodeId::from("member-a"),
            generation: ClusterGeneration::new(3),
            epoch: cluster.epoch(),
        };

        materialize_committed_command(&cluster, &replay).unwrap();
        assert_eq!(cluster.members()[0].generation, ClusterGeneration::new(5));
    }

    #[test]
    fn voter_change_without_a_known_leader_fails_loud() {
        let config = RaftMetadataRuntimeConfig::multi_voter("orders", 2, [1, 2, 3]);
        let runtime = RaftMetadataRuntime::with_config(config).unwrap();

        let error = runtime.request_remove_voter(2).unwrap_err();
        assert!(error.to_string().contains("require a known leader"));
    }

    #[test]
    fn joining_config_requires_remote_voters_without_self() {
        let config = RaftMetadataRuntimeConfig::try_joining("orders", 4, [3, 1, 1, 2])
            .unwrap()
            .ticks(0, 0);

        assert_eq!(config.raft_node_id, 4);
        assert_eq!(config.voter_ids(), &[1, 2, 3]);
        assert_eq!(config.election_tick, 2);
        assert_eq!(config.heartbeat_tick, 1);

        let empty = RaftMetadataRuntimeConfig::try_joining("orders", 4, []).unwrap_err();
        assert!(empty.to_string().contains("at least one remote voter"));

        let includes_self =
            RaftMetadataRuntimeConfig::try_joining("orders", 4, [1, 2, 4]).unwrap_err();
        assert!(includes_self
            .to_string()
            .contains("must not include local node 4"));
    }

    #[test]
    fn can_still_use_in_memory_cluster_for_comparison() {
        let cluster = InMemoryCluster::new("orders");
        assert_eq!(cluster.name(), "orders");
    }
}
