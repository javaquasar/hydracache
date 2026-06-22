use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use hydracache_core::{CacheCodec, CacheError, CacheStats, PostcardCodec, Result};
use serde::{Deserialize, Serialize};

use crate::builder::HydraCacheBuilder;
use crate::cache::HydraCache;
use crate::invalidation_bus::{CacheInvalidationBus, InMemoryInvalidationBus};
use tokio::sync::broadcast;

static NEXT_CLUSTER_CLIENT_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_CLUSTER_MEMBER_ID: AtomicU64 = AtomicU64::new(1);

/// Metadata key used by members to advertise their peer-fetch base URL.
///
/// The value is a base URL such as `http://127.0.0.1:3000`, not the full
/// peer-fetch route. Transport adapters append their own route path so one
/// advertised endpoint can stay stable across route-versioning changes.
pub const CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY: &str = "hydracache.peer_fetch.base_url";

fn next_client_id() -> ClusterNodeId {
    let id = NEXT_CLUSTER_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
    ClusterNodeId::new(format!("hydracache-client-{id}"))
}

fn next_member_id() -> ClusterNodeId {
    let id = NEXT_CLUSTER_MEMBER_ID.fetch_add(1, Ordering::Relaxed);
    ClusterNodeId::new(format!("hydracache-member-{id}"))
}

/// Stable logical id for a HydraCache cluster participant.
///
/// The id is separate from transport-level identities. A future libp2p adapter
/// can map this value to a `PeerId`, while a server deployment can map it to a
/// configured node name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClusterNodeId(String);

impl ClusterNodeId {
    /// Create a node id from an application-defined string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the node id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ClusterNodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for ClusterNodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ClusterNodeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Monotonic process generation for a cluster node id.
///
/// A restarted process should use a larger generation than the previous
/// process. This lets the cluster reject stale clients or members that still
/// emit invalidation messages after a restart.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ClusterGeneration(u64);

impl ClusterGeneration {
    /// Create a generation from a numeric value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw generation value.
    pub fn value(self) -> u64 {
        self.0
    }

    /// Return the next generation value.
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Committed cluster metadata epoch.
///
/// In v0.20 this is simulated by [`InMemoryCluster`]. A future Raft-backed
/// adapter should advance this value only after committed membership changes.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ClusterEpoch(u64);

impl ClusterEpoch {
    /// Create an epoch from a numeric value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw epoch value.
    pub fn value(self) -> u64 {
        self.0
    }

    fn advance(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

/// Runtime role of a HydraCache instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterRole {
    /// No distributed behavior.
    Local,
    /// Application-side near-cache connected to a cluster.
    Client,
    /// Cluster participant that routes invalidations and later owns metadata.
    Member,
}

impl ClusterRole {
    /// Return whether this role is allowed to vote in future Raft metadata.
    pub fn can_vote(self) -> bool {
        matches!(self, Self::Member)
    }
}

/// Transport-security posture declared for a controlled cluster pilot.
///
/// HydraCache does not terminate TLS or manage certificates. This structure is
/// a loud, machine-readable contract that says whether the embedded HTTP
/// transport is protected by HydraCache auth headers or by an explicitly
/// declared external mesh/mTLS boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportPosture {
    /// Whether HydraCache transport auth is configured on routes and clients.
    pub auth: bool,
    /// Whether strict current wire-version compatibility is enforced.
    pub wire_strict: bool,
    /// Whether an operator declared that an external mesh/mTLS boundary handles
    /// identity and transport security.
    pub mesh_declared: bool,
}

impl TransportPosture {
    /// Create a posture from explicit booleans.
    pub const fn new(auth: bool, wire_strict: bool, mesh_declared: bool) -> Self {
        Self {
            auth,
            wire_strict,
            mesh_declared,
        }
    }

    /// Return whether the posture is acceptable for the 0.40 pilot gate.
    pub fn is_safe(&self) -> bool {
        (self.auth && self.wire_strict) || self.mesh_declared
    }

    /// Return the actuator highlight for an unsafe missing-auth posture.
    pub fn highlight(&self) -> Option<&'static str> {
        if !self.auth && !self.mesh_declared {
            Some("AUTH MISSING")
        } else {
            None
        }
    }
}

/// Client-side routing behavior for owner peer-fetch traffic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingMode {
    /// Smart routing: resolve the owner for each key and contact that owner.
    #[default]
    Direct,
    /// Unisocket routing: always send owner traffic through a configured
    /// gateway/single endpoint.
    SingleEndpoint,
}

/// Minimal epoch fence for topology-authoritative decisions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyFence {
    committed_epoch: ClusterEpoch,
}

impl TopologyFence {
    /// Create a fence at the provided committed epoch.
    pub const fn new(committed_epoch: ClusterEpoch) -> Self {
        Self { committed_epoch }
    }

    /// Return the latest committed topology epoch known to this fence.
    pub fn committed_epoch(&self) -> ClusterEpoch {
        self.committed_epoch
    }

    /// Return whether a message stamped with `msg_epoch` is still admissible.
    pub fn admit(&self, msg_epoch: ClusterEpoch) -> bool {
        msg_epoch >= self.committed_epoch
    }

    /// Advance the fence. Older epochs never move it backwards.
    pub fn commit(&mut self, epoch: ClusterEpoch) {
        if epoch > self.committed_epoch {
            self.committed_epoch = epoch;
        }
    }
}

/// Lifecycle state for an embedded cluster component.
///
/// The lifecycle model is diagnostic, not supervisory: HydraCache records what
/// happened, while the application still decides when to start HTTP servers,
/// admission bridges, or other background work.
///
/// # Example
///
/// ```rust
/// use hydracache::{ClusterLifecycleDiagnostics, ClusterLifecycleStatus};
///
/// let mut lifecycle = ClusterLifecycleDiagnostics::idle("admission-bridge");
/// assert_eq!(lifecycle.status, ClusterLifecycleStatus::Idle);
///
/// lifecycle.record_start();
/// assert!(lifecycle.is_running());
///
/// lifecycle.record_shutdown_requested();
/// lifecycle.record_graceful_stop();
/// assert!(lifecycle.is_stopped());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterLifecycleStatus {
    /// The component has not been started yet.
    #[default]
    Idle,
    /// The component is currently running.
    Running,
    /// Shutdown was requested, but the component has not reported completion.
    Stopping,
    /// The component stopped gracefully.
    Stopped,
    /// The component failed.
    Failed,
}

impl ClusterLifecycleStatus {
    /// Return whether this status represents active work.
    pub fn is_running(self) -> bool {
        self == Self::Running
    }

    /// Return whether shutdown was requested and completion is pending.
    pub fn is_stopping(self) -> bool {
        self == Self::Stopping
    }

    /// Return whether this status represents a graceful stop.
    pub fn is_stopped(self) -> bool {
        self == Self::Stopped
    }

    /// Return whether this status represents a failure.
    pub fn has_failed(self) -> bool {
        self == Self::Failed
    }

    /// Return whether no more work is expected for this lifecycle instance.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
    }
}

/// Point-in-time lifecycle diagnostics for an embedded cluster component.
///
/// # Example
///
/// ```rust
/// use hydracache::ClusterLifecycleDiagnostics;
///
/// let mut lifecycle = ClusterLifecycleDiagnostics::idle("peer-fetch-service");
/// lifecycle.record_start();
/// lifecycle.record_failure("listener closed unexpectedly");
///
/// assert!(lifecycle.has_failed());
/// assert_eq!(lifecycle.start_count, 1);
/// assert_eq!(
///     lifecycle.last_error.as_deref(),
///     Some("listener closed unexpectedly"),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterLifecycleDiagnostics {
    /// Stable component name used in diagnostics and sandbox reports.
    pub component: String,
    /// Current lifecycle status.
    pub status: ClusterLifecycleStatus,
    /// Number of recorded starts.
    pub start_count: u64,
    /// Number of graceful stops.
    pub stop_count: u64,
    /// Whether shutdown was requested at least once.
    pub shutdown_requested: bool,
    /// Last lifecycle error, if any.
    pub last_error: Option<String>,
}

impl ClusterLifecycleDiagnostics {
    /// Create an idle lifecycle snapshot for a component.
    pub fn idle(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            status: ClusterLifecycleStatus::Idle,
            start_count: 0,
            stop_count: 0,
            shutdown_requested: false,
            last_error: None,
        }
    }

    /// Create a running lifecycle snapshot for a component.
    pub fn running(component: impl Into<String>) -> Self {
        let mut lifecycle = Self::idle(component);
        lifecycle.record_start();
        lifecycle
    }

    /// Record that the component started.
    pub fn record_start(&mut self) {
        self.status = ClusterLifecycleStatus::Running;
        self.start_count = self.start_count.saturating_add(1);
        self.shutdown_requested = false;
        self.last_error = None;
    }

    /// Record that shutdown was requested.
    pub fn record_shutdown_requested(&mut self) {
        self.shutdown_requested = true;
        if !self.status.is_terminal() {
            self.status = ClusterLifecycleStatus::Stopping;
        }
    }

    /// Record a graceful stop.
    pub fn record_graceful_stop(&mut self) {
        self.status = ClusterLifecycleStatus::Stopped;
        self.stop_count = self.stop_count.saturating_add(1);
        self.shutdown_requested = true;
    }

    /// Record a component failure.
    pub fn record_failure(&mut self, error: impl Into<String>) {
        self.status = ClusterLifecycleStatus::Failed;
        self.last_error = Some(error.into());
    }

    /// Return whether this component is currently running.
    pub fn is_running(&self) -> bool {
        self.status.is_running()
    }

    /// Return whether this component is stopping.
    pub fn is_stopping(&self) -> bool {
        self.status.is_stopping()
    }

    /// Return whether this component stopped gracefully.
    pub fn is_stopped(&self) -> bool {
        self.status.is_stopped()
    }

    /// Return whether this component failed.
    pub fn has_failed(&self) -> bool {
        self.status.has_failed()
    }

    /// Return whether this component reached a terminal status.
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

/// Error returned by a background cluster component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterComponentError {
    component: &'static str,
    message: String,
}

impl ClusterComponentError {
    /// Create a component error with a stable component name.
    pub fn new(component: &'static str, message: impl Into<String>) -> Self {
        Self {
            component,
            message: message.into(),
        }
    }

    /// Return the stable component name.
    pub fn component(&self) -> &'static str {
        self.component
    }

    /// Return the human-readable error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ClusterComponentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cluster component '{}' failed: {}",
            self.component, self.message
        )
    }
}

impl std::error::Error for ClusterComponentError {}

/// Uniform lifecycle surface for background cluster components.
#[async_trait::async_trait]
pub trait ClusterComponent: Send + Sync {
    /// Stable name for diagnostics.
    fn name(&self) -> &'static str;

    /// Start background work. Implementations should be idempotent.
    async fn start(&self) -> std::result::Result<(), ClusterComponentError>;

    /// Request a graceful stop. Implementations should be idempotent.
    async fn stop(&self) -> std::result::Result<(), ClusterComponentError>;

    /// Return a point-in-time lifecycle diagnostics snapshot.
    fn diagnostics(&self) -> ClusterLifecycleDiagnostics;

    /// Return the most recent error, if one was recorded.
    fn last_error(&self) -> Option<String>;
}

/// Minimal reusable lifecycle component for adapters that already own work.
#[derive(Debug, Clone)]
pub struct ClusterLifecycleComponent {
    name: &'static str,
    state: Arc<Mutex<ClusterLifecycleDiagnostics>>,
}

impl ClusterLifecycleComponent {
    /// Create an idle component with a stable diagnostic name.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            state: Arc::new(Mutex::new(ClusterLifecycleDiagnostics::idle(name))),
        }
    }

    /// Record a component failure.
    pub fn fail(&self, message: impl Into<String>) {
        self.state
            .lock()
            .expect("cluster component lifecycle poisoned")
            .record_failure(message);
    }
}

#[async_trait::async_trait]
impl ClusterComponent for ClusterLifecycleComponent {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn start(&self) -> std::result::Result<(), ClusterComponentError> {
        let mut diagnostics = self
            .state
            .lock()
            .expect("cluster component lifecycle poisoned");
        if !diagnostics.is_running() {
            diagnostics.record_start();
        }
        Ok(())
    }

    async fn stop(&self) -> std::result::Result<(), ClusterComponentError> {
        let mut diagnostics = self
            .state
            .lock()
            .expect("cluster component lifecycle poisoned");
        if !diagnostics.is_stopped() {
            diagnostics.record_shutdown_requested();
            diagnostics.record_graceful_stop();
        }
        Ok(())
    }

    fn diagnostics(&self) -> ClusterLifecycleDiagnostics {
        self.state
            .lock()
            .expect("cluster component lifecycle poisoned")
            .clone()
    }

    fn last_error(&self) -> Option<String> {
        self.diagnostics().last_error
    }
}

/// Advertised endpoints for a cluster participant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClusterEndpoints {
    /// Control endpoint for future member/client protocol requests.
    pub control: Option<String>,
    /// Invalidation endpoint used by a future external bus.
    pub invalidation: Option<String>,
    /// Diagnostics or actuator endpoint.
    pub diagnostics: Option<String>,
}

impl ClusterEndpoints {
    /// Create an empty endpoint set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the control endpoint.
    pub fn control(mut self, endpoint: impl Into<String>) -> Self {
        self.control = Some(endpoint.into());
        self
    }

    /// Set the invalidation endpoint.
    pub fn invalidation(mut self, endpoint: impl Into<String>) -> Self {
        self.invalidation = Some(endpoint.into());
        self
    }

    /// Set the diagnostics endpoint.
    pub fn diagnostics(mut self, endpoint: impl Into<String>) -> Self {
        self.diagnostics = Some(endpoint.into());
        self
    }
}

/// Candidate discovered before authoritative membership admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterCandidate {
    /// Candidate node id.
    pub node_id: ClusterNodeId,
    /// Candidate process generation.
    pub generation: ClusterGeneration,
    /// Requested runtime role.
    pub role: ClusterRole,
    /// Advertised endpoints.
    pub endpoints: ClusterEndpoints,
    /// Small metadata map for future discovery adapters.
    pub metadata: BTreeMap<String, String>,
}

impl ClusterCandidate {
    /// Create a member candidate.
    pub fn member(node_id: impl Into<ClusterNodeId>) -> Self {
        Self::new(node_id, ClusterRole::Member)
    }

    /// Create a client candidate.
    pub fn client(node_id: impl Into<ClusterNodeId>) -> Self {
        Self::new(node_id, ClusterRole::Client)
    }

    fn new(node_id: impl Into<ClusterNodeId>, role: ClusterRole) -> Self {
        Self {
            node_id: node_id.into(),
            generation: ClusterGeneration::default(),
            role,
            endpoints: ClusterEndpoints::default(),
            metadata: BTreeMap::new(),
        }
    }

    /// Set the candidate generation.
    pub fn generation(mut self, generation: ClusterGeneration) -> Self {
        self.generation = generation;
        self
    }

    /// Set advertised endpoints.
    pub fn endpoints(mut self, endpoints: ClusterEndpoints) -> Self {
        self.endpoints = endpoints;
        self
    }

    /// Add one metadata entry.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Advertise the base URL used by peer-fetch transports.
    ///
    /// The URL should not include the concrete peer-fetch path. For example,
    /// use `http://127.0.0.1:3000`, not
    /// `http://127.0.0.1:3000/cluster/peer-fetch`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{ClusterCandidate, CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY};
    ///
    /// let candidate = ClusterCandidate::member("member-a")
    ///     .peer_fetch_base_url("http://127.0.0.1:3000");
    ///
    /// assert_eq!(
    ///     candidate.peer_fetch_base_url_value(),
    ///     Some("http://127.0.0.1:3000")
    /// );
    /// assert_eq!(
    ///     candidate
    ///         .metadata
    ///         .get(CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY)
    ///         .map(String::as_str),
    ///     Some("http://127.0.0.1:3000")
    /// );
    /// ```
    pub fn peer_fetch_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.metadata.insert(
            CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY.to_owned(),
            base_url.into(),
        );
        self
    }

    /// Return the advertised peer-fetch base URL, when present.
    pub fn peer_fetch_base_url_value(&self) -> Option<&str> {
        self.metadata
            .get(CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY)
            .map(String::as_str)
    }
}

/// Admitted cluster participant snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterMember {
    /// Admitted node id.
    pub node_id: ClusterNodeId,
    /// Admitted process generation.
    pub generation: ClusterGeneration,
    /// Runtime role.
    pub role: ClusterRole,
    /// Cluster epoch observed when this participant was admitted.
    pub epoch: ClusterEpoch,
    /// Advertised endpoints.
    pub endpoints: ClusterEndpoints,
    /// Metadata carried from discovery.
    pub metadata: BTreeMap<String, String>,
}

impl ClusterMember {
    fn from_candidate(candidate: ClusterCandidate, epoch: ClusterEpoch) -> Self {
        Self {
            node_id: candidate.node_id,
            generation: candidate.generation,
            role: candidate.role,
            epoch,
            endpoints: candidate.endpoints,
            metadata: candidate.metadata,
        }
    }

    /// Return whether this member is a client near-cache.
    pub fn is_client(&self) -> bool {
        self.role == ClusterRole::Client
    }

    /// Return whether this member is a cluster member node.
    pub fn is_member(&self) -> bool {
        self.role == ClusterRole::Member
    }

    /// Return the advertised peer-fetch base URL, when present.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{
    ///     ClusterCandidate, ClusterControlPlane, InMemoryCluster,
    /// };
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cluster = InMemoryCluster::new("orders");
    /// let member = ClusterControlPlane::join_member(
    ///     &cluster,
    ///     ClusterCandidate::member("member-a")
    ///         .peer_fetch_base_url("http://127.0.0.1:3000"),
    /// )
    /// .await?;
    ///
    /// assert_eq!(
    ///     member.peer_fetch_base_url(),
    ///     Some("http://127.0.0.1:3000")
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn peer_fetch_base_url(&self) -> Option<&str> {
        self.metadata
            .get(CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY)
            .map(String::as_str)
    }
}

/// Result of resolving which admitted member owns a cache key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterOwnershipDecision {
    /// Logical cache key used for the lookup.
    pub key: String,
    /// Owner selected by the resolver, if at least one member is eligible.
    pub owner: Option<ClusterMember>,
    /// Number of eligible member nodes considered by the resolver.
    pub member_count: usize,
    /// Stable resolver name for diagnostics and sandbox reports.
    pub resolver: &'static str,
}

impl ClusterOwnershipDecision {
    /// Return whether an owner was selected.
    pub fn has_owner(&self) -> bool {
        self.owner.is_some()
    }

    /// Return the selected owner node id.
    pub fn owner_node_id(&self) -> Option<&ClusterNodeId> {
        self.owner.as_ref().map(|owner| &owner.node_id)
    }

    /// Return the selected owner generation.
    pub fn owner_generation(&self) -> Option<ClusterGeneration> {
        self.owner.as_ref().map(|owner| owner.generation)
    }

    /// Build a peer-fetch request for this decision, if it has an owner.
    pub fn peer_fetch_request(&self) -> Option<ClusterPeerFetchRequest> {
        self.owner.as_ref().map(|owner| {
            ClusterPeerFetchRequest::new(owner.node_id.clone(), self.key.clone())
                .generation(owner.generation)
        })
    }
}

/// Strategy for mapping cache keys to admitted cluster members.
///
/// This trait is intentionally value-agnostic. It decides ownership only; a
/// later peer-fetch layer can use the decision to contact the owner.
pub trait ClusterOwnershipResolver: Send + Sync {
    /// Stable resolver name for diagnostics.
    fn name(&self) -> &'static str;

    /// Resolve the owner for `key` among the provided participants.
    fn resolve_owner(&self, key: &str, participants: &[ClusterMember]) -> ClusterOwnershipDecision;
}

/// Deterministic rendezvous-style ownership resolver.
///
/// The resolver scores each admitted member by hashing `key` with the member
/// node id and picks the highest score. It ignores clients and local roles.
#[derive(Debug, Clone, Copy, Default)]
pub struct RendezvousClusterOwnership;

impl ClusterOwnershipResolver for RendezvousClusterOwnership {
    fn name(&self) -> &'static str {
        "rendezvous"
    }

    fn resolve_owner(&self, key: &str, participants: &[ClusterMember]) -> ClusterOwnershipDecision {
        let mut member_count = 0_usize;
        let mut best: Option<(u64, ClusterMember)> = None;

        for participant in participants
            .iter()
            .filter(|candidate| candidate.is_member())
        {
            member_count = member_count.saturating_add(1);
            let score = rendezvous_score(key, &participant.node_id);
            let replace = best
                .as_ref()
                .map(|(best_score, best_member)| {
                    score > *best_score
                        || (score == *best_score && participant.node_id > best_member.node_id)
                })
                .unwrap_or(true);
            if replace {
                best = Some((score, participant.clone()));
            }
        }

        ClusterOwnershipDecision {
            key: key.to_owned(),
            owner: best.map(|(_, member)| member),
            member_count,
            resolver: self.name(),
        }
    }
}

fn rendezvous_score(key: &str, node_id: &ClusterNodeId) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in key.bytes().chain([0xff]).chain(node_id.as_str().bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Stable partition id used as a thin indirection over rendezvous ownership.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct PartitionId(u32);

impl PartitionId {
    /// Create a partition id from its numeric value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the numeric partition id.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Return the deterministic partition for a key.
///
/// A zero `partition_count` is normalized to one partition so callers cannot
/// accidentally divide by zero while validating a partially built config.
pub fn partition_for_key(key: impl AsRef<str>, partition_count: u32) -> PartitionId {
    let partition_count = partition_count.max(1);
    PartitionId((stable_key_hash(key.as_ref()) % u64::from(partition_count)) as u32)
}

fn stable_key_hash(key: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in key.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Replica/quorum pilot configuration error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterReplicaConfigError {
    /// `min_replica` must be at least one.
    MinReplicaZero,
    /// `quorum` must be at least one.
    QuorumZero,
    /// `quorum` cannot exceed `replication_factor`.
    QuorumExceedsReplication,
}

impl fmt::Display for ClusterReplicaConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MinReplicaZero => "min_replica must be at least 1",
            Self::QuorumZero => "quorum must be at least 1",
            Self::QuorumExceedsReplication => "quorum cannot exceed replication_factor",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ClusterReplicaConfigError {}

/// Validate the narrow 0.40 pilot replica/quorum shape.
///
/// Value replication still lands in a later release, but validating these
/// values now makes invalid future topology config fail early.
pub fn validate_replica_config(
    min_replica: usize,
    replication_factor: usize,
    quorum: usize,
) -> std::result::Result<(), ClusterReplicaConfigError> {
    if min_replica < 1 {
        return Err(ClusterReplicaConfigError::MinReplicaZero);
    }
    if quorum == 0 {
        return Err(ClusterReplicaConfigError::QuorumZero);
    }
    if quorum > replication_factor {
        return Err(ClusterReplicaConfigError::QuorumExceedsReplication);
    }
    Ok(())
}

/// Action selected by near-cache watermark repair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NearCacheRepairAction {
    /// Apply the frame normally.
    Apply,
    /// Owner generation changed; clear the partition before applying/refreshing.
    ClearPartition,
    /// A sequence gap was observed; invalidate conservatively.
    InvalidateConservatively,
}

/// Per-partition near-cache watermark metadata.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaDataContainer {
    last_uuid: Option<ClusterGeneration>,
    last_seq: u64,
}

impl MetaDataContainer {
    /// Return the last owner generation observed for this partition.
    pub fn last_uuid(&self) -> Option<ClusterGeneration> {
        self.last_uuid
    }

    /// Return the last applied invalidation sequence.
    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// Update the watermark from an invalidation frame.
    pub fn on_watermark(
        &mut self,
        generation: Option<ClusterGeneration>,
        message_id: Option<u64>,
    ) -> NearCacheRepairAction {
        let generation = generation.unwrap_or_default();
        let seq = message_id.unwrap_or_default();

        if self.last_uuid != Some(generation) {
            self.last_uuid = Some(generation);
            self.last_seq = seq;
            return NearCacheRepairAction::ClearPartition;
        }

        if seq > self.last_seq.saturating_add(1) {
            self.last_seq = seq;
            return NearCacheRepairAction::InvalidateConservatively;
        }

        self.last_seq = self.last_seq.max(seq);
        NearCacheRepairAction::Apply
    }
}

/// Request for fetching an encoded cache value from an owner member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPeerFetchRequest {
    /// Owner member expected to serve this request.
    pub owner: ClusterNodeId,
    /// Logical cache key requested from the owner.
    pub key: String,
    /// Optional owner generation observed by the caller.
    pub generation: Option<ClusterGeneration>,
}

/// Requested owner generation did not match the current owner generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterPeerFetchGenerationMismatch {
    /// Generation observed by the caller when it resolved ownership.
    pub requested: ClusterGeneration,
    /// Current generation known by the owner or transport.
    pub current: ClusterGeneration,
}

impl ClusterPeerFetchRequest {
    /// Create a new peer-fetch request.
    pub fn new(owner: impl Into<ClusterNodeId>, key: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            key: key.into(),
            generation: None,
        }
    }

    /// Attach the owner generation observed by the caller.
    pub fn generation(mut self, generation: ClusterGeneration) -> Self {
        self.generation = Some(generation);
        self
    }

    /// Return whether this request carries an observed owner generation.
    pub fn has_generation(&self) -> bool {
        self.generation.is_some()
    }

    /// Return whether this request can be served by `current` owner generation.
    pub fn matches_generation(&self, current: ClusterGeneration) -> bool {
        self.generation_mismatch(current).is_none()
    }

    /// Return mismatch details when the observed owner generation is stale.
    pub fn generation_mismatch(
        &self,
        current: ClusterGeneration,
    ) -> Option<ClusterPeerFetchGenerationMismatch> {
        match self.generation {
            Some(requested) if requested != current => {
                Some(ClusterPeerFetchGenerationMismatch { requested, current })
            }
            _ => None,
        }
    }
}

/// Response returned by a peer-fetch implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPeerFetchResponse {
    /// Owner member that served or attempted to serve the request.
    pub owner: ClusterNodeId,
    /// Logical cache key requested from the owner.
    pub key: String,
    /// Encoded cache value, when the owner had it.
    pub value: Option<Bytes>,
}

impl ClusterPeerFetchResponse {
    /// Create a cache-hit response.
    pub fn hit(owner: impl Into<ClusterNodeId>, key: impl Into<String>, value: Bytes) -> Self {
        Self {
            owner: owner.into(),
            key: key.into(),
            value: Some(value),
        }
    }

    /// Create a cache-miss response.
    pub fn miss(owner: impl Into<ClusterNodeId>, key: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            key: key.into(),
            value: None,
        }
    }

    /// Return whether the owner returned a value.
    pub fn is_hit(&self) -> bool {
        self.value.is_some()
    }

    /// Return whether the owner did not have the requested value.
    pub fn is_miss(&self) -> bool {
        self.value.is_none()
    }
}

/// Transport-neutral peer-fetch seam for future owner-side value loading.
#[async_trait::async_trait]
pub trait ClusterPeerFetch: Send + Sync {
    /// Fetch an encoded value from the requested owner.
    async fn fetch(&self, request: ClusterPeerFetchRequest) -> Result<ClusterPeerFetchResponse>;
}

/// In-memory peer-fetch implementation for tests, demos, and sandbox reports.
#[derive(Debug, Clone, Default)]
pub struct InMemoryPeerFetch {
    state: Arc<Mutex<InMemoryPeerFetchState>>,
}

#[derive(Debug, Default)]
struct InMemoryPeerFetchState {
    values: BTreeMap<(ClusterNodeId, String), Bytes>,
    hits: u64,
    misses: u64,
}

/// Point-in-time counters for an [`InMemoryPeerFetch`] registry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClusterPeerFetchDiagnostics {
    /// Number of stored owner/key values.
    pub stored_values: usize,
    /// Number of fetch requests that returned a value.
    pub hits: u64,
    /// Number of fetch requests that did not find a value.
    pub misses: u64,
}

impl ClusterPeerFetchDiagnostics {
    /// Return total fetch requests observed by this registry.
    pub fn total_requests(&self) -> u64 {
        self.hits.saturating_add(self.misses)
    }

    /// Return the hit ratio when at least one request has been observed.
    pub fn hit_ratio(&self) -> Option<f64> {
        let total = self.total_requests();
        if total == 0 {
            None
        } else {
            Some(self.hits as f64 / total as f64)
        }
    }
}

impl InMemoryPeerFetch {
    /// Create an empty in-memory peer-fetch registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store an encoded value for an owner/key pair.
    pub fn put(
        &self,
        owner: impl Into<ClusterNodeId>,
        key: impl Into<String>,
        value: impl Into<Bytes>,
    ) {
        self.state
            .lock()
            .expect("peer fetch state poisoned")
            .values
            .insert((owner.into(), key.into()), value.into());
    }

    /// Remove an encoded value for an owner/key pair.
    pub fn remove(&self, owner: &ClusterNodeId, key: &str) -> Option<Bytes> {
        self.state
            .lock()
            .expect("peer fetch state poisoned")
            .values
            .remove(&(owner.clone(), key.to_owned()))
    }

    /// Return the number of stored owner/key values.
    pub fn len(&self) -> usize {
        self.state
            .lock()
            .expect("peer fetch state poisoned")
            .values
            .len()
    }

    /// Return whether no values are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return current in-memory peer-fetch diagnostics.
    pub fn diagnostics(&self) -> ClusterPeerFetchDiagnostics {
        let state = self.state.lock().expect("peer fetch state poisoned");
        ClusterPeerFetchDiagnostics {
            stored_values: state.values.len(),
            hits: state.hits,
            misses: state.misses,
        }
    }
}

#[async_trait::async_trait]
impl ClusterPeerFetch for InMemoryPeerFetch {
    async fn fetch(&self, request: ClusterPeerFetchRequest) -> Result<ClusterPeerFetchResponse> {
        let mut state = self.state.lock().expect("peer fetch state poisoned");
        let value = state
            .values
            .get(&(request.owner.clone(), request.key.clone()))
            .cloned();
        if value.is_some() {
            state.hits = state.hits.saturating_add(1);
        } else {
            state.misses = state.misses.saturating_add(1);
        }

        Ok(ClusterPeerFetchResponse {
            owner: request.owner,
            key: request.key,
            value,
        })
    }
}

/// Event emitted by discovery before authoritative admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterDiscoveryEvent {
    /// A candidate was observed by discovery.
    CandidateSeen(ClusterCandidate),
    /// A member appears live.
    MemberLive(ClusterNodeId),
    /// A member or client published an intentional graceful-leave marker.
    MemberLeaving {
        /// Leaving node id.
        node_id: ClusterNodeId,
        /// Generation that published the leave marker.
        generation: ClusterGeneration,
        /// Runtime role that is leaving.
        role: ClusterRole,
    },
    /// A member is suspected unhealthy.
    MemberSuspect(ClusterNodeId),
    /// A member is considered dead.
    MemberDead(ClusterNodeId),
}

/// Transport-neutral discovery contract for cluster candidates and liveness.
///
/// This is the seam where future chitchat, DNS, mDNS, or P2P discovery
/// adapters can plug in. Discovery observes candidates and liveness; it does
/// not make authoritative membership decisions. Admission remains the
/// responsibility of [`ClusterControlPlane`].
#[async_trait::async_trait]
pub trait ClusterDiscovery: fmt::Debug + Send + Sync {
    /// Announce or update a candidate.
    async fn announce(&self, candidate: ClusterCandidate) -> Result<()>;

    /// Record that a node appears live.
    async fn mark_live(&self, node_id: ClusterNodeId) -> Result<()>;

    /// Record that a node is suspected unhealthy.
    async fn mark_suspect(&self, node_id: ClusterNodeId) -> Result<()>;

    /// Record that a node is considered dead.
    async fn mark_dead(&self, node_id: ClusterNodeId) -> Result<()>;

    /// Return the latest candidate snapshot for every discovered node id.
    fn candidates(&self) -> Vec<ClusterCandidate>;

    /// Return discovery events recorded by this adapter.
    fn events(&self) -> Vec<ClusterDiscoveryEvent>;
}

#[derive(Debug, Default)]
struct InMemoryClusterDiscoveryState {
    candidates: BTreeMap<ClusterNodeId, ClusterCandidate>,
    events: Vec<ClusterDiscoveryEvent>,
}

/// In-memory discovery journal for tests, demos, and future adapter contracts.
///
/// `InMemoryClusterDiscovery` models the chitchat side of the design without
/// depending on chitchat yet: nodes first become visible as candidates with
/// metadata, endpoints, role, and generation; authoritative admission remains
/// the responsibility of [`InMemoryCluster`].
#[derive(Debug, Default)]
pub struct InMemoryClusterDiscovery {
    state: Mutex<InMemoryClusterDiscoveryState>,
}

impl InMemoryClusterDiscovery {
    /// Create an empty in-memory discovery journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Announce or update a candidate.
    pub fn announce(&self, candidate: ClusterCandidate) {
        let mut state = self.state.lock().expect("cluster discovery poisoned");
        state
            .candidates
            .insert(candidate.node_id.clone(), candidate.clone());
        state
            .events
            .push(ClusterDiscoveryEvent::CandidateSeen(candidate));
    }

    /// Record that a node appears live.
    pub fn mark_live(&self, node_id: impl Into<ClusterNodeId>) {
        self.push_liveness(ClusterDiscoveryEvent::MemberLive(node_id.into()));
    }

    /// Record that a node is suspected unhealthy.
    pub fn mark_suspect(&self, node_id: impl Into<ClusterNodeId>) {
        self.push_liveness(ClusterDiscoveryEvent::MemberSuspect(node_id.into()));
    }

    /// Record that a node is considered dead.
    pub fn mark_dead(&self, node_id: impl Into<ClusterNodeId>) {
        self.push_liveness(ClusterDiscoveryEvent::MemberDead(node_id.into()));
    }

    fn push_liveness(&self, event: ClusterDiscoveryEvent) {
        self.state
            .lock()
            .expect("cluster discovery poisoned")
            .events
            .push(event);
    }

    /// Return the latest candidate snapshot for every discovered node id.
    pub fn candidates(&self) -> Vec<ClusterCandidate> {
        self.state
            .lock()
            .expect("cluster discovery poisoned")
            .candidates
            .values()
            .cloned()
            .collect()
    }

    /// Return discovery events recorded by the in-memory journal.
    pub fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        self.state
            .lock()
            .expect("cluster discovery poisoned")
            .events
            .clone()
    }
}

#[async_trait::async_trait]
impl ClusterDiscovery for InMemoryClusterDiscovery {
    async fn announce(&self, candidate: ClusterCandidate) -> Result<()> {
        InMemoryClusterDiscovery::announce(self, candidate);
        Ok(())
    }

    async fn mark_live(&self, node_id: ClusterNodeId) -> Result<()> {
        InMemoryClusterDiscovery::mark_live(self, node_id);
        Ok(())
    }

    async fn mark_suspect(&self, node_id: ClusterNodeId) -> Result<()> {
        InMemoryClusterDiscovery::mark_suspect(self, node_id);
        Ok(())
    }

    async fn mark_dead(&self, node_id: ClusterNodeId) -> Result<()> {
        InMemoryClusterDiscovery::mark_dead(self, node_id);
        Ok(())
    }

    fn candidates(&self) -> Vec<ClusterCandidate> {
        InMemoryClusterDiscovery::candidates(self)
    }

    fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        InMemoryClusterDiscovery::events(self)
    }
}

/// Dependency-free, chitchat-style discovery adapter for tests and API spikes.
///
/// This adapter intentionally does not run the real `chitchat` network
/// protocol yet. It models the part of chitchat that matters to HydraCache's
/// public cluster API: a node starts with seed addresses, announces itself as a
/// candidate, and records liveness transitions separately from authoritative
/// control-plane admission.
///
/// Candidate announcements are stored in-memory and annotated with adapter
/// metadata so tests, diagnostics, and the sandbox can distinguish this path
/// from the plain [`InMemoryClusterDiscovery`] journal.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use hydracache::{ChitchatStyleDiscovery, HydraCache, InMemoryCluster};
///
/// # #[tokio::main]
/// # async fn main() -> hydracache::CacheResult<()> {
/// let cluster = Arc::new(InMemoryCluster::new("orders"));
/// let discovery = Arc::new(ChitchatStyleDiscovery::new([
///     "127.0.0.1:7000",
///     "127.0.0.1:7001",
/// ]));
///
/// let member = HydraCache::member()
///     .shared_cluster(cluster)
///     .discovery(discovery.clone())
///     .node_id("member-a")
///     .start()
///     .await?;
///
/// assert_eq!(discovery.seed_count(), 2);
/// assert_eq!(discovery.candidates().len(), 1);
/// assert!(member.cluster_discovery_diagnostics().unwrap().has_candidates());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ChitchatStyleDiscovery {
    seeds: Vec<String>,
    inner: InMemoryClusterDiscovery,
}

impl ChitchatStyleDiscovery {
    /// Create a chitchat-style discovery journal with seed addresses.
    pub fn new<I, S>(seeds: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            seeds: seeds.into_iter().map(Into::into).collect(),
            inner: InMemoryClusterDiscovery::new(),
        }
    }

    /// Return the static seed addresses used to bootstrap discovery.
    pub fn seeds(&self) -> &[String] {
        &self.seeds
    }

    /// Return the number of configured seed addresses.
    pub fn seed_count(&self) -> usize {
        self.seeds.len()
    }

    /// Return whether the adapter has at least one seed address.
    pub fn has_seeds(&self) -> bool {
        !self.seeds.is_empty()
    }

    /// Return the adapter label attached to candidate metadata.
    pub fn adapter_name(&self) -> &'static str {
        "chitchat-style"
    }

    /// Announce or update a candidate with chitchat-style metadata.
    pub fn announce(&self, mut candidate: ClusterCandidate) {
        candidate
            .metadata
            .entry("discovery.adapter".to_owned())
            .or_insert_with(|| self.adapter_name().to_owned());
        if self.has_seeds() {
            candidate
                .metadata
                .entry("discovery.seeds".to_owned())
                .or_insert_with(|| self.seeds.join(","));
        }
        self.inner.announce(candidate);
    }

    /// Record that a node appears live.
    pub fn mark_live(&self, node_id: impl Into<ClusterNodeId>) {
        self.inner.mark_live(node_id);
    }

    /// Record that a node is suspected unhealthy.
    pub fn mark_suspect(&self, node_id: impl Into<ClusterNodeId>) {
        self.inner.mark_suspect(node_id);
    }

    /// Record that a node is considered dead.
    pub fn mark_dead(&self, node_id: impl Into<ClusterNodeId>) {
        self.inner.mark_dead(node_id);
    }

    /// Return the latest candidate snapshot for every discovered node id.
    pub fn candidates(&self) -> Vec<ClusterCandidate> {
        self.inner.candidates()
    }

    /// Return discovery events recorded by the adapter.
    pub fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        self.inner.events()
    }
}

impl Default for ChitchatStyleDiscovery {
    fn default() -> Self {
        Self::new(std::iter::empty::<String>())
    }
}

#[async_trait::async_trait]
impl ClusterDiscovery for ChitchatStyleDiscovery {
    async fn announce(&self, candidate: ClusterCandidate) -> Result<()> {
        ChitchatStyleDiscovery::announce(self, candidate);
        Ok(())
    }

    async fn mark_live(&self, node_id: ClusterNodeId) -> Result<()> {
        ChitchatStyleDiscovery::mark_live(self, node_id);
        Ok(())
    }

    async fn mark_suspect(&self, node_id: ClusterNodeId) -> Result<()> {
        ChitchatStyleDiscovery::mark_suspect(self, node_id);
        Ok(())
    }

    async fn mark_dead(&self, node_id: ClusterNodeId) -> Result<()> {
        ChitchatStyleDiscovery::mark_dead(self, node_id);
        Ok(())
    }

    fn candidates(&self) -> Vec<ClusterCandidate> {
        ChitchatStyleDiscovery::candidates(self)
    }

    fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        ChitchatStyleDiscovery::events(self)
    }
}

/// Authoritative or simulated cluster membership event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterMembershipEvent {
    /// A member node joined or was updated.
    MemberJoined(ClusterMember),
    /// A client near-cache connected or was updated.
    ClientConnected(ClusterMember),
    /// A node left the in-memory cluster model.
    NodeLeft {
        /// Node id.
        node_id: ClusterNodeId,
        /// Role before leaving.
        role: ClusterRole,
        /// Epoch after the leave operation.
        epoch: ClusterEpoch,
    },
    /// A stale process generation was rejected.
    StaleGenerationRejected {
        /// Rejected node id.
        node_id: ClusterNodeId,
        /// Runtime role associated with the rejected generation.
        role: ClusterRole,
        /// Existing accepted generation.
        existing: ClusterGeneration,
        /// Attempted stale generation.
        attempted: ClusterGeneration,
        /// Machine-friendly rejection reason.
        reason: String,
    },
}

/// Error returned by [`ClusterMembershipSubscriber::recv`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterMembershipRecvError {
    /// The membership event stream has been closed.
    Closed,
    /// The subscriber lagged behind the bounded event stream.
    Lagged(u64),
}

impl fmt::Display for ClusterMembershipRecvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("cluster membership subscription closed"),
            Self::Lagged(skipped) => {
                write!(
                    formatter,
                    "cluster membership subscriber lagged by {skipped} events"
                )
            }
        }
    }
}

impl std::error::Error for ClusterMembershipRecvError {}

/// Receiver for cluster membership events from a control plane.
///
/// The stream is intentionally bounded. Admission and leave operations never
/// wait for slow subscribers; slow consumers receive
/// [`ClusterMembershipRecvError::Lagged`] and can decide whether to rebuild
/// their view from diagnostics/snapshots.
#[derive(Debug)]
pub struct ClusterMembershipSubscriber {
    receiver: broadcast::Receiver<ClusterMembershipEvent>,
}

impl ClusterMembershipSubscriber {
    fn new(receiver: broadcast::Receiver<ClusterMembershipEvent>) -> Self {
        Self { receiver }
    }

    fn closed() -> Self {
        let (sender, receiver) = broadcast::channel(1);
        drop(sender);
        Self { receiver }
    }

    /// Receive the next membership event.
    pub async fn recv(
        &mut self,
    ) -> std::result::Result<ClusterMembershipEvent, ClusterMembershipRecvError> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Closed) => Err(ClusterMembershipRecvError::Closed),
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                Err(ClusterMembershipRecvError::Lagged(skipped))
            }
        }
    }

    /// Receive the next event, skipping lag notifications.
    pub async fn next_event(&mut self) -> Option<ClusterMembershipEvent> {
        loop {
            match self.recv().await {
                Ok(event) => return Some(event),
                Err(ClusterMembershipRecvError::Closed) => return None,
                Err(ClusterMembershipRecvError::Lagged(_)) => continue,
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ClusterMembershipEventBus {
    sender: broadcast::Sender<ClusterMembershipEvent>,
}

impl ClusterMembershipEventBus {
    fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    fn publish(&self, event: ClusterMembershipEvent) {
        let _ = self.sender.send(event);
    }

    fn subscribe(&self) -> ClusterMembershipSubscriber {
        ClusterMembershipSubscriber::new(self.sender.subscribe())
    }

    fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for ClusterMembershipEventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

/// Cluster diagnostics visible from a [`HydraCache`] instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterDiagnostics {
    /// Cluster name.
    pub cluster_name: String,
    /// Local runtime role.
    pub role: ClusterRole,
    /// Local node id.
    pub node_id: ClusterNodeId,
    /// Local process generation.
    pub generation: ClusterGeneration,
    /// Current cluster epoch observed by the in-memory model.
    pub epoch: ClusterEpoch,
    /// Number of admitted member nodes.
    pub member_count: usize,
    /// Number of connected clients.
    pub client_count: usize,
    /// Configured bootstrap addresses.
    pub bootstrap: Vec<String>,
    /// Whether this cache has an attached in-memory cluster runtime.
    pub connected: bool,
    /// Number of active invalidation bus receivers.
    pub invalidation_subscribers: usize,
    /// Number of active cluster membership event subscribers.
    pub membership_subscribers: usize,
    /// Local embedded runtime lifecycle snapshot.
    pub lifecycle: ClusterLifecycleDiagnostics,
}

impl ClusterDiagnostics {
    /// Return whether this diagnostics snapshot belongs to a local cache role.
    pub fn is_local_role(&self) -> bool {
        self.role == ClusterRole::Local
    }

    /// Return whether this diagnostics snapshot belongs to a client runtime.
    pub fn is_client_role(&self) -> bool {
        self.role == ClusterRole::Client
    }

    /// Return whether this diagnostics snapshot belongs to a member runtime.
    pub fn is_member_role(&self) -> bool {
        self.role == ClusterRole::Member
    }

    /// Return the total number of admitted members and connected clients.
    pub fn participant_count(&self) -> usize {
        self.member_count.saturating_add(self.client_count)
    }

    /// Return the number of configured bootstrap addresses.
    pub fn bootstrap_count(&self) -> usize {
        self.bootstrap.len()
    }

    /// Return whether at least one member is currently admitted.
    pub fn has_members(&self) -> bool {
        self.member_count > 0
    }

    /// Return whether at least one client is currently connected.
    pub fn has_clients(&self) -> bool {
        self.client_count > 0
    }

    /// Return whether at least one bootstrap address is configured.
    pub fn has_bootstrap(&self) -> bool {
        !self.bootstrap.is_empty()
    }

    /// Return whether the invalidation bus has active receivers.
    pub fn has_invalidation_subscribers(&self) -> bool {
        self.invalidation_subscribers > 0
    }

    /// Return whether the membership event bus has active receivers.
    pub fn has_membership_subscribers(&self) -> bool {
        self.membership_subscribers > 0
    }

    /// Return whether the current view contains more than one participant.
    pub fn has_multiple_participants(&self) -> bool {
        self.participant_count() > 1
    }

    /// Return whether this runtime appears connected to a usable cluster view.
    pub fn is_operational(&self) -> bool {
        self.connected && self.lifecycle.is_running() && self.participant_count() > 0
    }
}

/// Machine-readable reason used by [`ClusterHealthState`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClusterHealthReason {
    /// At least one background cluster component is not running.
    LifecycleNotRunning,
    /// The runtime sees no admitted members or connected clients.
    NoParticipants,
    /// The distributed invalidation receiver lagged behind its source stream.
    LaggedReceivers { count: u64 },
    /// Transport frames could not be decoded.
    DecodeErrors { count: u64 },
    /// Publishing an invalidation to the bus failed.
    PublishFailures { count: u64 },
    /// The invalidation receiver stream closed.
    ReceiverClosed { count: u64 },
    /// A stale process generation was fenced off.
    StaleGenerationRejections { count: u64 },
    /// Peer-fetch or owner-load transport auth rejected a caller.
    PeerFetchAuthFailures { count: u64 },
    /// Peer-fetch or owner-load transport wire compatibility rejected a caller.
    WireVersionRejections { count: u64 },
    /// Gossip state was reset recently enough to matter for staging.
    GossipResetRecent {
        /// Age of the most recent observed tombstone/reset signal.
        tombstone_age_ms: u64,
        /// Number of observed gossip resets since process start.
        reset_count: u64,
    },
}

/// Derived staging health state with machine-readable reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClusterHealthState {
    /// All checked staging invariants hold.
    Healthy,
    /// The runtime is usable, but at least one soft signal is degraded.
    Degraded { reasons: Vec<ClusterHealthReason> },
    /// The runtime is not safe to run staging traffic against.
    NotReady { reasons: Vec<ClusterHealthReason> },
}

impl ClusterHealthState {
    /// Return `true` only when the derived state is healthy.
    pub fn ready_for_staging(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Return the machine-readable reasons, if the state is not healthy.
    pub fn reasons(&self) -> &[ClusterHealthReason] {
        match self {
            Self::Healthy => &[],
            Self::Degraded { reasons } | Self::NotReady { reasons } => reasons,
        }
    }
}

/// Point-in-time owner/remote/hot fill counters for cluster staging.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterFillCounters {
    /// This node owned the key and loaded it from the origin.
    pub owner_load_success: u64,
    /// Owner-side origin loads that returned an error.
    pub owner_load_errors: u64,
    /// This node fetched encoded bytes from the owner over peer-fetch.
    pub remote_fetch_success: u64,
    /// Remote peer-fetch attempts that failed.
    pub remote_fetch_errors: u64,
    /// This node served a non-owned hot-copy without contacting the owner.
    pub hot_cache_hits: u64,
}

impl ClusterFillCounters {
    /// Return the total number of successful fill/hit observations.
    pub fn successful_events(&self) -> u64 {
        self.owner_load_success
            .saturating_add(self.remote_fetch_success)
            .saturating_add(self.hot_cache_hits)
    }

    /// Return the total number of owner/remote fill errors.
    pub fn error_events(&self) -> u64 {
        self.owner_load_errors
            .saturating_add(self.remote_fetch_errors)
    }
}

/// Three-part groupcache-style counters for pilot dashboards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterCacheCounters {
    /// This node loaded a key for which it was the owner.
    pub owner_load_total: u64,
    /// This node fetched encoded bytes from another owner.
    pub remote_fetch_total: u64,
    /// This node served a borrowed hot near-cache copy.
    pub hot_cache_hit_total: u64,
}

impl From<ClusterFillCounters> for ClusterCacheCounters {
    fn from(value: ClusterFillCounters) -> Self {
        Self {
            owner_load_total: value.owner_load_success,
            remote_fetch_total: value.remote_fetch_success,
            hot_cache_hit_total: value.hot_cache_hits,
        }
    }
}

/// Additional staging counters that do not belong to local cache stats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterStagingCounters {
    /// Peer-fetch/owner-load auth failures observed by the staging gate.
    pub peer_fetch_auth_failures: u64,
    /// Wire-version rejections observed by the staging gate.
    pub wire_version_rejections: u64,
    /// Stale-generation publish/admission attempts rejected by fencing.
    pub stale_generation_rejected: u64,
    /// Age of the most recent gossip tombstone/reset signal.
    pub tombstone_age_ms: u64,
    /// Number of observed gossip resets since process start.
    pub gossip_reset_count: u64,
    /// Quorum/read-after-write barrier timeouts observed by pilot reads.
    pub barrier_timeouts: u64,
    /// Near-cache conservative invalidations caused by watermark repair.
    pub near_cache_conservative_invalidations: u64,
    /// Lifecycle graceful stop events observed by pilot probes.
    pub lifecycle_stop_count: u64,
    /// Lifecycle restart/rejoin events observed by pilot probes.
    pub lifecycle_restart_count: u64,
}

/// Boolean readiness contract for a controlled internal production pilot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterPilotReadiness {
    /// Declared transport-security posture.
    pub transport_posture: TransportPosture,
    /// Whether at least one member is admitted.
    pub has_members: bool,
    /// Number of admitted member nodes.
    pub member_count: usize,
    /// Whether the member count is inside the supported 2..=5 pilot range.
    pub within_supported_size: bool,
    /// Whether strict current wire compatibility is enabled.
    pub strict_wire_compatibility: bool,
    /// Whether invalidation diagnostics are free of hard errors.
    pub diagnostics_clean: bool,
    /// Whether the local cluster runtime lifecycle is running.
    pub lifecycle_operational: bool,
    /// Whether at least one authoritative topology epoch has been committed.
    pub topology_committed: bool,
}

impl ClusterPilotReadiness {
    /// Single boolean gate used by tests, actuator, and release docs.
    pub fn is_pilot_ready(&self) -> bool {
        self.transport_posture.is_safe()
            && self.has_members
            && self.within_supported_size
            && self.strict_wire_compatibility
            && self.diagnostics_clean
            && self.lifecycle_operational
            && self.topology_committed
    }

    /// Human-facing highlights that should be rendered loudly by adapters.
    pub fn highlights(&self) -> Vec<&'static str> {
        self.transport_posture.highlight().into_iter().collect()
    }
}

/// Dashboard-ready pilot snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterPilotReport {
    /// Boolean readiness gates.
    pub readiness: ClusterPilotReadiness,
    /// Three-part owner/remote/hot cache counters.
    pub counters: ClusterCacheCounters,
    /// Current metadata epoch.
    pub epoch: u64,
    /// Local process generation.
    pub generation: u64,
    /// Partition-table stamp for ownership-view drift detection.
    pub stamp: u64,
    /// Invalidation messages published by this cache.
    pub invalidations_published: u64,
    /// Invalidation messages received by this cache.
    pub invalidations_received: u64,
    /// Invalidation messages applied by this cache.
    pub invalidations_applied: u64,
    /// Invalidation receiver lag events.
    pub invalidation_lagged: u64,
    /// Invalidation decode errors.
    pub decode_errors: u64,
    /// Invalidation publish failures.
    pub publish_failures: u64,
    /// Invalidation receiver closed events.
    pub receiver_closed: u64,
    /// Owner-load successes.
    pub owner_load_success: u64,
    /// Owner-load errors.
    pub owner_load_errors: u64,
    /// Remote peer-fetch successes.
    pub remote_fetch_success: u64,
    /// Remote peer-fetch errors.
    pub remote_fetch_errors: u64,
    /// Auth failures observed by pilot probes.
    pub auth_failures: u64,
    /// Wire-version failures observed by pilot probes.
    pub wire_version_failures: u64,
    /// Stale-generation rejections.
    pub stale_generation_rejections: u64,
    /// Barrier timeouts.
    pub barrier_timeouts: u64,
    /// Near-cache conservative invalidations caused by repair.
    pub near_cache_conservative_invalidations: u64,
    /// Lifecycle stop count.
    pub lifecycle_stop_count: u64,
    /// Lifecycle restart count.
    pub lifecycle_restart_count: u64,
    /// Declared transport-security posture.
    pub transport_posture: TransportPosture,
    /// Loud actuator highlights such as `AUTH MISSING`.
    pub highlights: Vec<String>,
}

/// Staging-focused health summary derived from diagnostics and counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterStagingHealth {
    /// Local runtime role.
    pub role: ClusterRole,
    /// Local node id.
    pub node_id: String,
    /// Whether this runtime is connected to a cluster view.
    pub connected: bool,
    /// Number of admitted member nodes.
    pub member_count: usize,
    /// Number of connected clients.
    pub client_count: usize,
    /// Current metadata epoch.
    pub epoch: u64,
    /// Local process generation.
    pub generation: u64,
    /// Invalidation messages published by this cache.
    pub invalidations_published: u64,
    /// Invalidation messages received by this cache.
    pub invalidations_received: u64,
    /// Invalidation messages applied by this cache.
    pub invalidations_applied: u64,
    /// Invalidation receiver lag events.
    pub lagged_receivers: u64,
    /// Invalidation decode errors.
    pub decode_errors: u64,
    /// Invalidation publish failures.
    pub publish_failures: u64,
    /// Invalidation receiver closed events.
    pub receiver_closed: u64,
    /// Owner-side origin loads that returned a value.
    pub owner_load_success: u64,
    /// Owner-side origin loads that returned an error.
    pub owner_load_errors: u64,
    /// Remote peer-fetch calls that returned a value.
    pub remote_fetch_success: u64,
    /// Remote peer-fetch calls that failed.
    pub remote_fetch_errors: u64,
    /// Hot near-cache hits for non-owned values.
    pub hot_cache_hits: u64,
    /// Auth failures observed by peer-fetch/owner-load staging checks.
    pub peer_fetch_auth_failures: u64,
    /// Wire-version rejections observed by staging checks.
    pub wire_version_rejections: u64,
    /// Stale-generation attempts rejected by fencing.
    pub stale_generation_rejected: u64,
    /// Age of the most recent gossip tombstone/reset signal.
    pub tombstone_age_ms: u64,
    /// Number of observed gossip resets since process start.
    pub gossip_reset_count: u64,
    /// Quorum/read-after-write barrier timeouts observed by pilot reads.
    pub barrier_timeouts: u64,
    /// Near-cache conservative invalidations caused by watermark repair.
    pub near_cache_conservative_invalidations: u64,
    /// Lifecycle graceful stop events observed by pilot probes.
    pub lifecycle_stop_count: u64,
    /// Lifecycle restart/rejoin events observed by pilot probes.
    pub lifecycle_restart_count: u64,
    /// Derived overall staging health state.
    pub state: ClusterHealthState,
}

impl ClusterStagingHealth {
    /// Derive staging health from cluster diagnostics and logical counters.
    pub fn from_parts(
        diagnostics: ClusterDiagnostics,
        stats: CacheStats,
        fill: ClusterFillCounters,
        staging: ClusterStagingCounters,
    ) -> Self {
        let state = derive_cluster_health_state(&diagnostics, &stats, &staging);
        Self {
            role: diagnostics.role,
            node_id: diagnostics.node_id.to_string(),
            connected: diagnostics.connected,
            member_count: diagnostics.member_count,
            client_count: diagnostics.client_count,
            epoch: diagnostics.epoch.value(),
            generation: diagnostics.generation.value(),
            invalidations_published: stats.distributed_invalidations_published,
            invalidations_received: stats.distributed_invalidations_received,
            invalidations_applied: stats.distributed_invalidations_applied,
            lagged_receivers: stats.distributed_invalidation_lagged,
            decode_errors: stats.distributed_invalidation_decode_errors,
            publish_failures: stats.distributed_invalidation_publish_failures,
            receiver_closed: stats.distributed_invalidation_receiver_closed,
            owner_load_success: fill.owner_load_success,
            owner_load_errors: fill.owner_load_errors,
            remote_fetch_success: fill.remote_fetch_success,
            remote_fetch_errors: fill.remote_fetch_errors,
            hot_cache_hits: fill.hot_cache_hits,
            peer_fetch_auth_failures: staging.peer_fetch_auth_failures,
            wire_version_rejections: staging.wire_version_rejections,
            stale_generation_rejected: staging.stale_generation_rejected,
            tombstone_age_ms: staging.tombstone_age_ms,
            gossip_reset_count: staging.gossip_reset_count,
            barrier_timeouts: staging.barrier_timeouts,
            near_cache_conservative_invalidations: staging.near_cache_conservative_invalidations,
            lifecycle_stop_count: staging.lifecycle_stop_count,
            lifecycle_restart_count: staging.lifecycle_restart_count,
            state,
        }
    }
}

/// Structured cluster load report used by deterministic staging gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterLoadReport {
    /// Number of cluster nodes participating in the scenario.
    pub nodes: usize,
    /// Total logical operations driven by the scenario.
    pub requests: usize,
    /// Logical read operations.
    pub read_ops: usize,
    /// Logical invalidation operations.
    pub invalidation_ops: usize,
    /// Invalidation messages published.
    pub published: u64,
    /// Invalidation messages received.
    pub received: u64,
    /// Invalidation messages applied.
    pub applied: u64,
    /// Receiver lag events.
    pub lagged: u64,
    /// Decode errors.
    pub decode_errors: u64,
    /// Publish failures.
    pub publish_failures: u64,
    /// Receiver closed events.
    pub receiver_closed: u64,
    /// Stale-generation attempts rejected by fencing.
    pub stale_generation_rejected: u64,
    /// Peer-fetch or owner-load auth failures observed by the gate probes.
    pub peer_fetch_auth_failures: u64,
    /// Peer-fetch or owner-load wire-version rejections observed by the gate probes.
    pub wire_version_rejections: u64,
    /// Owner-side origin load successes.
    pub owner_load_success: u64,
    /// Remote peer-fetch successes.
    pub remote_fetch_success: u64,
    /// Hot near-cache hits for non-owned values.
    pub hot_cache_hits: u64,
    /// Recorded wall-clock duration. Deterministic gates must not assert on it.
    pub elapsed_ms: u64,
}

impl ClusterLoadReport {
    /// Return whether read + invalidation operations match total requests.
    pub fn totals_match_requests(&self) -> bool {
        self.read_ops.saturating_add(self.invalidation_ops) == self.requests
    }

    /// Return whether the invalidation health counters are clean.
    pub fn has_clean_invalidation_health(&self) -> bool {
        self.lagged == 0
            && self.decode_errors == 0
            && self.publish_failures == 0
            && self.receiver_closed == 0
    }
}

fn derive_cluster_health_state(
    diagnostics: &ClusterDiagnostics,
    stats: &CacheStats,
    staging: &ClusterStagingCounters,
) -> ClusterHealthState {
    let mut hard_reasons = Vec::new();
    let mut soft_reasons = Vec::new();

    if !diagnostics.lifecycle.is_running() {
        hard_reasons.push(ClusterHealthReason::LifecycleNotRunning);
    }
    if diagnostics.participant_count() == 0 {
        hard_reasons.push(ClusterHealthReason::NoParticipants);
    }
    if stats.distributed_invalidation_decode_errors > 0 {
        hard_reasons.push(ClusterHealthReason::DecodeErrors {
            count: stats.distributed_invalidation_decode_errors,
        });
    }
    if stats.distributed_invalidation_publish_failures > 0 {
        hard_reasons.push(ClusterHealthReason::PublishFailures {
            count: stats.distributed_invalidation_publish_failures,
        });
    }
    if stats.distributed_invalidation_receiver_closed > 0 {
        hard_reasons.push(ClusterHealthReason::ReceiverClosed {
            count: stats.distributed_invalidation_receiver_closed,
        });
    }

    if stats.distributed_invalidation_lagged > 0 {
        soft_reasons.push(ClusterHealthReason::LaggedReceivers {
            count: stats.distributed_invalidation_lagged,
        });
    }
    if staging.peer_fetch_auth_failures > 0 {
        soft_reasons.push(ClusterHealthReason::PeerFetchAuthFailures {
            count: staging.peer_fetch_auth_failures,
        });
    }
    if staging.wire_version_rejections > 0 {
        soft_reasons.push(ClusterHealthReason::WireVersionRejections {
            count: staging.wire_version_rejections,
        });
    }
    if staging.gossip_reset_count > 0 {
        soft_reasons.push(ClusterHealthReason::GossipResetRecent {
            tombstone_age_ms: staging.tombstone_age_ms,
            reset_count: staging.gossip_reset_count,
        });
    }

    if hard_reasons.is_empty() {
        if soft_reasons.is_empty() {
            ClusterHealthState::Healthy
        } else {
            ClusterHealthState::Degraded {
                reasons: soft_reasons,
            }
        }
    } else {
        hard_reasons.extend(soft_reasons);
        ClusterHealthState::NotReady {
            reasons: hard_reasons,
        }
    }
}

/// Ownership diagnostics visible from a cluster control plane.
///
/// This is intentionally separate from [`ClusterDiagnostics`] so ownership
/// counters can evolve without adding fields to the externally constructible
/// runtime diagnostics snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterOwnershipDiagnostics {
    /// Resolver name used by the control plane.
    pub resolver: &'static str,
    /// Number of ownership resolution attempts handled by this control plane.
    pub resolutions: u64,
    /// Number of ownership resolutions that found no admitted member owner.
    pub no_owner: u64,
    /// Monotonic ownership table stamp for stale-view detection.
    pub stamp: u64,
}

impl ClusterOwnershipDiagnostics {
    /// Create an ownership diagnostics snapshot.
    pub fn new(resolver: &'static str, resolutions: u64, no_owner: u64, stamp: u64) -> Self {
        Self {
            resolver,
            resolutions,
            no_owner,
            stamp,
        }
    }

    /// Number of ownership resolutions that selected an owner.
    pub fn owner_found(&self) -> u64 {
        self.resolutions.saturating_sub(self.no_owner)
    }

    /// Return whether any ownership resolution has been attempted.
    pub fn has_resolutions(&self) -> bool {
        self.resolutions > 0
    }

    /// Ratio of ownership resolutions that found an admitted owner.
    pub fn owner_found_ratio(&self) -> Option<f64> {
        (self.resolutions > 0).then(|| self.owner_found() as f64 / self.resolutions as f64)
    }
}

/// Discovery diagnostics visible from a [`HydraCache`] client/member runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterDiscoveryDiagnostics {
    /// Local node id that owns this diagnostics snapshot.
    pub local_node_id: ClusterNodeId,
    /// Latest candidate snapshots known to the discovery adapter.
    pub candidates: Vec<ClusterCandidate>,
    /// Discovery events known to the discovery adapter.
    pub events: Vec<ClusterDiscoveryEvent>,
}

impl ClusterDiscoveryDiagnostics {
    /// Number of latest candidate snapshots.
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Number of discovery events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Return whether discovery has observed at least one candidate.
    pub fn has_candidates(&self) -> bool {
        !self.candidates.is_empty()
    }

    /// Return whether discovery has recorded at least one event.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }
}

/// Reason why the admission bridge ignored a discovered candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterAdmissionIgnoreReason {
    /// The candidate already matches authoritative metadata.
    AlreadyCurrent,
    /// The candidate role is not admitted by this bridge configuration.
    RoleDisabled,
    /// Local cache roles are never admitted into a cluster control plane.
    LocalRole,
}

/// Reason why the admission bridge rejected a discovered candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterAdmissionRejectReason {
    /// The candidate generation is older than authoritative metadata.
    StaleGeneration {
        /// Existing accepted generation.
        existing: ClusterGeneration,
        /// Attempted generation.
        attempted: ClusterGeneration,
    },
    /// The control plane returned an admission error.
    AdmissionError(String),
}

/// Event emitted by a cluster admission bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterAdmissionBridgeEvent {
    /// A discovery candidate was observed by the bridge.
    CandidateSeen(ClusterCandidate),
    /// A candidate was admitted by the control plane.
    CandidateAdmitted(ClusterMember),
    /// A candidate did not require a control-plane write.
    CandidateIgnored {
        /// Ignored candidate.
        candidate: ClusterCandidate,
        /// Ignore reason.
        reason: ClusterAdmissionIgnoreReason,
    },
    /// A candidate was rejected before or during admission.
    CandidateRejected {
        /// Rejected candidate.
        candidate: ClusterCandidate,
        /// Rejection reason.
        reason: ClusterAdmissionRejectReason,
    },
    /// The bridge loop stopped.
    BridgeStopped,
}

/// Lightweight counters for a cluster admission bridge.
///
/// # Example
///
/// ```rust
/// use hydracache::{
///     ClusterAdmissionBridgeDiagnostics, ClusterAdmissionBridgeEvent,
///     ClusterAdmissionIgnoreReason, ClusterCandidate,
/// };
///
/// let mut diagnostics = ClusterAdmissionBridgeDiagnostics::default();
/// let candidate = ClusterCandidate::client("client-a");
///
/// diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateSeen(candidate.clone()));
/// diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateIgnored {
///     candidate,
///     reason: ClusterAdmissionIgnoreReason::AlreadyCurrent,
/// });
///
/// assert_eq!(diagnostics.candidates_seen, 1);
/// assert_eq!(diagnostics.total_decisions(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClusterAdmissionBridgeDiagnostics {
    /// Number of candidate snapshots observed.
    pub candidates_seen: u64,
    /// Number of candidates admitted.
    pub candidates_admitted: u64,
    /// Number of candidates ignored without writing metadata.
    pub candidates_ignored: u64,
    /// Number of candidates rejected as stale or invalid.
    pub candidates_rejected: u64,
    /// Number of admission attempts that returned an error.
    pub admission_failures: u64,
    /// Last candidate node id observed by the bridge.
    pub last_candidate: Option<ClusterNodeId>,
    /// Last admitted node id.
    pub last_admitted: Option<ClusterNodeId>,
    /// Last error message, if any.
    pub last_error: Option<String>,
}

impl ClusterAdmissionBridgeDiagnostics {
    /// Return the total number of terminal bridge decisions.
    pub fn total_decisions(&self) -> u64 {
        self.candidates_admitted
            .saturating_add(self.candidates_ignored)
            .saturating_add(self.candidates_rejected)
    }

    /// Return whether the bridge has observed at least one candidate.
    pub fn has_seen_candidates(&self) -> bool {
        self.candidates_seen > 0
    }

    /// Return whether the bridge admitted at least one candidate.
    pub fn has_admissions(&self) -> bool {
        self.candidates_admitted > 0
    }

    /// Return whether the bridge reported any rejection or failure.
    pub fn has_issues(&self) -> bool {
        self.candidates_rejected > 0 || self.admission_failures > 0
    }

    /// Update counters from a bridge event.
    pub fn record_event(&mut self, event: &ClusterAdmissionBridgeEvent) {
        match event {
            ClusterAdmissionBridgeEvent::CandidateSeen(candidate) => {
                self.candidates_seen = self.candidates_seen.saturating_add(1);
                self.last_candidate = Some(candidate.node_id.clone());
            }
            ClusterAdmissionBridgeEvent::CandidateAdmitted(member) => {
                self.candidates_admitted = self.candidates_admitted.saturating_add(1);
                self.last_admitted = Some(member.node_id.clone());
            }
            ClusterAdmissionBridgeEvent::CandidateIgnored { candidate, .. } => {
                self.candidates_ignored = self.candidates_ignored.saturating_add(1);
                self.last_candidate = Some(candidate.node_id.clone());
            }
            ClusterAdmissionBridgeEvent::CandidateRejected { candidate, reason } => {
                self.candidates_rejected = self.candidates_rejected.saturating_add(1);
                self.last_candidate = Some(candidate.node_id.clone());
                if let ClusterAdmissionRejectReason::AdmissionError(error) = reason {
                    self.admission_failures = self.admission_failures.saturating_add(1);
                    self.last_error = Some(error.clone());
                }
            }
            ClusterAdmissionBridgeEvent::BridgeStopped => {}
        }
    }
}

/// Polling behavior for [`ClusterAdmissionBridge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterAdmissionBridgeConfig {
    /// How often the background task should poll discovery candidates.
    pub poll_interval: Duration,
    /// Whether client candidates should be admitted.
    pub admit_clients: bool,
    /// Whether member candidates should be admitted.
    pub admit_members: bool,
}

impl ClusterAdmissionBridgeConfig {
    /// Return config with a custom polling interval.
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Enable or disable client admission.
    pub fn admit_clients(mut self, admit_clients: bool) -> Self {
        self.admit_clients = admit_clients;
        self
    }

    /// Enable or disable member admission.
    pub fn admit_members(mut self, admit_members: bool) -> Self {
        self.admit_members = admit_members;
        self
    }

    fn normalized_poll_interval(self) -> Duration {
        if self.poll_interval.is_zero() {
            Duration::from_millis(1)
        } else {
            self.poll_interval
        }
    }
}

impl Default for ClusterAdmissionBridgeConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            admit_clients: true,
            admit_members: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClusterAdmissionSnapshot {
    generation: ClusterGeneration,
    role: ClusterRole,
}

#[derive(Debug)]
struct ClusterAdmissionBridgeState {
    admitted: BTreeMap<ClusterNodeId, ClusterAdmissionSnapshot>,
    events: Vec<ClusterAdmissionBridgeEvent>,
    diagnostics: ClusterAdmissionBridgeDiagnostics,
    lifecycle: ClusterLifecycleDiagnostics,
}

impl Default for ClusterAdmissionBridgeState {
    fn default() -> Self {
        Self {
            admitted: BTreeMap::new(),
            events: Vec::new(),
            diagnostics: ClusterAdmissionBridgeDiagnostics::default(),
            lifecycle: ClusterLifecycleDiagnostics::idle("cluster-admission-bridge"),
        }
    }
}

#[derive(Debug)]
struct ClusterAdmissionBridgeInner {
    discovery: Arc<dyn ClusterDiscovery>,
    control_plane: Arc<dyn ClusterControlPlane>,
    config: ClusterAdmissionBridgeConfig,
    state: Mutex<ClusterAdmissionBridgeState>,
    run_lock: tokio::sync::Mutex<()>,
}

/// Polls discovery candidates and admits them into an authoritative control plane.
///
/// The bridge is the seam between gossip-style discovery and Raft-style
/// metadata. Discovery can be eventually consistent and noisy; the bridge keeps
/// a local admission snapshot so repeated polls do not rewrite the same
/// generation, and only the control plane decides whether a candidate is truly
/// accepted.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use hydracache::{
///     ClusterAdmissionBridge, ClusterCandidate, InMemoryCluster,
///     InMemoryClusterDiscovery,
/// };
///
/// # #[tokio::main]
/// # async fn main() -> hydracache::CacheResult<()> {
/// let discovery = Arc::new(InMemoryClusterDiscovery::new());
/// let control_plane = Arc::new(InMemoryCluster::new("orders"));
/// let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());
///
/// discovery.announce(ClusterCandidate::member("member-a"));
/// assert_eq!(bridge.run_once().await, 1);
/// assert_eq!(control_plane.members().len(), 1);
/// assert_eq!(bridge.diagnostics().candidates_admitted, 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ClusterAdmissionBridge {
    inner: Arc<ClusterAdmissionBridgeInner>,
}

impl ClusterAdmissionBridge {
    /// Create a bridge with default polling behavior.
    pub fn new(
        discovery: Arc<dyn ClusterDiscovery>,
        control_plane: Arc<dyn ClusterControlPlane>,
    ) -> Self {
        Self::with_config(
            discovery,
            control_plane,
            ClusterAdmissionBridgeConfig::default(),
        )
    }

    /// Create a bridge with explicit polling behavior.
    pub fn with_config(
        discovery: Arc<dyn ClusterDiscovery>,
        control_plane: Arc<dyn ClusterControlPlane>,
        config: ClusterAdmissionBridgeConfig,
    ) -> Self {
        Self {
            inner: Arc::new(ClusterAdmissionBridgeInner {
                discovery,
                control_plane,
                config,
                state: Mutex::new(ClusterAdmissionBridgeState::default()),
                run_lock: tokio::sync::Mutex::new(()),
            }),
        }
    }

    /// Return this bridge config.
    pub fn config(&self) -> ClusterAdmissionBridgeConfig {
        self.inner.config
    }

    /// Return a point-in-time diagnostics snapshot.
    pub fn diagnostics(&self) -> ClusterAdmissionBridgeDiagnostics {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .diagnostics
            .clone()
    }

    /// Return lifecycle diagnostics for the background polling loop.
    ///
    /// `run_once` does not change lifecycle status. The lifecycle snapshot
    /// describes only the optional background task started by [`Self::start`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use hydracache::{
    ///     ClusterAdmissionBridge, ClusterLifecycleStatus, InMemoryCluster,
    ///     InMemoryClusterDiscovery,
    /// };
    ///
    /// let discovery = Arc::new(InMemoryClusterDiscovery::new());
    /// let control_plane = Arc::new(InMemoryCluster::new("orders"));
    /// let bridge = ClusterAdmissionBridge::new(discovery, control_plane);
    ///
    /// assert_eq!(
    ///     bridge.lifecycle_diagnostics().status,
    ///     ClusterLifecycleStatus::Idle,
    /// );
    /// ```
    pub fn lifecycle_diagnostics(&self) -> ClusterLifecycleDiagnostics {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .clone()
    }

    /// Return bridge events recorded so far.
    pub fn events(&self) -> Vec<ClusterAdmissionBridgeEvent> {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .events
            .clone()
    }

    /// Poll discovery once and try to admit every latest candidate snapshot.
    ///
    /// The return value is the number of candidate snapshots processed.
    pub async fn run_once(&self) -> usize {
        let _guard = self.inner.run_lock.lock().await;
        let candidates = self.inner.discovery.candidates();
        let processed = candidates.len();
        for candidate in candidates {
            self.admit_candidate(candidate).await;
        }
        processed
    }

    /// Start a background polling loop.
    ///
    /// Use [`ClusterAdmissionBridgeHandle::shutdown`] to stop the loop
    /// gracefully. Dropping the handle also asks the task to stop, but does not
    /// wait for it.
    pub fn start(&self) -> ClusterAdmissionBridgeHandle {
        let bridge = self.clone();
        let (shutdown, mut shutdown_rx) = tokio::sync::watch::channel(false);
        self.record_lifecycle_start();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(bridge.config().normalized_poll_interval());
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            bridge.record_event(ClusterAdmissionBridgeEvent::BridgeStopped);
                            bridge.record_lifecycle_graceful_stop();
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        bridge.run_once().await;
                    }
                }
            }
        });

        ClusterAdmissionBridgeHandle {
            bridge: self.clone(),
            shutdown: Some(shutdown),
            task: Some(task),
        }
    }

    async fn admit_candidate(&self, candidate: ClusterCandidate) {
        self.record_event(ClusterAdmissionBridgeEvent::CandidateSeen(
            candidate.clone(),
        ));

        if let Some(event) = self.pre_admission_event(&candidate) {
            self.record_event(event);
            return;
        }

        let result = match candidate.role {
            ClusterRole::Member => {
                self.inner
                    .control_plane
                    .join_member(candidate.clone())
                    .await
            }
            ClusterRole::Client => {
                self.inner
                    .control_plane
                    .join_client(candidate.clone())
                    .await
            }
            ClusterRole::Local => unreachable!("local candidates are ignored before admission"),
        };

        match result {
            Ok(member) => self.record_admitted(member),
            Err(error) => self.record_event(ClusterAdmissionBridgeEvent::CandidateRejected {
                candidate,
                reason: ClusterAdmissionRejectReason::AdmissionError(error.to_string()),
            }),
        }
    }

    fn pre_admission_event(
        &self,
        candidate: &ClusterCandidate,
    ) -> Option<ClusterAdmissionBridgeEvent> {
        let ignore_reason = match candidate.role {
            ClusterRole::Local => Some(ClusterAdmissionIgnoreReason::LocalRole),
            ClusterRole::Client if !self.inner.config.admit_clients => {
                Some(ClusterAdmissionIgnoreReason::RoleDisabled)
            }
            ClusterRole::Member if !self.inner.config.admit_members => {
                Some(ClusterAdmissionIgnoreReason::RoleDisabled)
            }
            ClusterRole::Client | ClusterRole::Member => None,
        };
        if let Some(reason) = ignore_reason {
            return Some(ClusterAdmissionBridgeEvent::CandidateIgnored {
                candidate: candidate.clone(),
                reason,
            });
        }

        let state = self
            .inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned");
        let existing = state.admitted.get(&candidate.node_id)?;

        if existing.generation > candidate.generation {
            return Some(ClusterAdmissionBridgeEvent::CandidateRejected {
                candidate: candidate.clone(),
                reason: ClusterAdmissionRejectReason::StaleGeneration {
                    existing: existing.generation,
                    attempted: candidate.generation,
                },
            });
        }

        if existing.generation == candidate.generation && existing.role == candidate.role {
            return Some(ClusterAdmissionBridgeEvent::CandidateIgnored {
                candidate: candidate.clone(),
                reason: ClusterAdmissionIgnoreReason::AlreadyCurrent,
            });
        }

        None
    }

    fn record_admitted(&self, member: ClusterMember) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned");
        state.admitted.insert(
            member.node_id.clone(),
            ClusterAdmissionSnapshot {
                generation: member.generation,
                role: member.role,
            },
        );
        let event = ClusterAdmissionBridgeEvent::CandidateAdmitted(member);
        state.diagnostics.record_event(&event);
        state.events.push(event);
    }

    fn record_event(&self, event: ClusterAdmissionBridgeEvent) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned");
        state.diagnostics.record_event(&event);
        state.events.push(event);
    }

    fn record_lifecycle_start(&self) {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .record_start();
    }

    fn record_lifecycle_shutdown_requested(&self) {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .record_shutdown_requested();
    }

    fn record_lifecycle_graceful_stop(&self) {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .record_graceful_stop();
    }

    fn record_lifecycle_failure(&self, error: impl Into<String>) {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .record_failure(error);
    }
}

/// Handle for a background [`ClusterAdmissionBridge`] polling task.
#[must_use]
#[derive(Debug)]
pub struct ClusterAdmissionBridgeHandle {
    bridge: ClusterAdmissionBridge,
    shutdown: Option<tokio::sync::watch::Sender<bool>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl ClusterAdmissionBridgeHandle {
    /// Ask the polling task to stop and wait until it exits.
    pub async fn shutdown(mut self) {
        self.bridge.record_lifecycle_shutdown_requested();
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(true);
        }
        if let Some(task) = self.task.take() {
            if let Err(error) = task.await {
                self.bridge.record_lifecycle_failure(error.to_string());
            }
        }
    }
}

impl Drop for ClusterAdmissionBridgeHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            self.bridge.record_lifecycle_shutdown_requested();
            let _ = shutdown.send(true);
        }
    }
}

/// Transport-neutral control-plane contract for cluster admission and metadata.
///
/// This trait is the seam where future chitchat/Raft-backed adapters can plug
/// in without changing [`HydraCache::client`] or [`HydraCache::member`] usage.
/// It is intentionally focused on control-plane decisions: admission, leave,
/// diagnostics, and the invalidation bus used for the hot freshness path.
#[async_trait::async_trait]
pub trait ClusterControlPlane: fmt::Debug + Send + Sync {
    /// Return the logical cluster name.
    fn name(&self) -> String;

    /// Return the invalidation bus used by admitted participants.
    fn invalidation_bus(&self) -> Arc<dyn CacheInvalidationBus>;

    /// Admit or update a member candidate.
    async fn join_member(&self, candidate: ClusterCandidate) -> Result<ClusterMember>;

    /// Admit or update a client candidate.
    async fn join_client(&self, candidate: ClusterCandidate) -> Result<ClusterMember>;

    /// Validate that a node id is still owned by the provided process generation.
    ///
    /// Cluster-backed invalidation publishers call this before sending a bus
    /// message. Control planes should reject missing nodes and generation
    /// mismatches so stale processes cannot publish freshness changes after a
    /// restart reused the same logical node id.
    async fn validate_generation(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<()>;

    /// Remove a node from this control plane when the generation still matches.
    async fn leave(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<Option<ClusterMembershipEvent>>;

    /// Subscribe to authoritative membership events.
    ///
    /// Implementations that do not expose a stream can use the default closed
    /// subscriber. Built-in control planes return a bounded non-blocking stream.
    fn subscribe_membership(&self) -> ClusterMembershipSubscriber {
        ClusterMembershipSubscriber::closed()
    }

    /// Build diagnostics for a local runtime attached to this control plane.
    fn diagnostics_for(
        &self,
        role: ClusterRole,
        node_id: ClusterNodeId,
        generation: ClusterGeneration,
        bootstrap: Vec<String>,
    ) -> ClusterDiagnostics;

    /// Return ownership-specific diagnostics for this control plane.
    fn ownership_diagnostics(&self) -> ClusterOwnershipDiagnostics {
        ClusterOwnershipDiagnostics::new("unknown", 0, 0, 0)
    }
}

/// Metadata command committed by [`RaftStyleMetadataControlPlane`].
///
/// This is intentionally small and transport-neutral. A future `raft-rs`
/// adapter can use the same command shape as the replicated state-machine input
/// while keeping [`HydraCache::client`] and [`HydraCache::member`] unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaftMetadataCommand {
    /// A member was admitted or updated.
    MemberUpsert {
        /// Admitted node id.
        node_id: ClusterNodeId,
        /// Admitted process generation.
        generation: ClusterGeneration,
        /// Cluster epoch observed after admission.
        epoch: ClusterEpoch,
    },
    /// A client was admitted or updated.
    ClientUpsert {
        /// Admitted node id.
        node_id: ClusterNodeId,
        /// Admitted process generation.
        generation: ClusterGeneration,
        /// Cluster epoch observed after admission.
        epoch: ClusterEpoch,
    },
    /// A node left membership.
    NodeLeft {
        /// Removed node id.
        node_id: ClusterNodeId,
        /// Removed node role.
        role: ClusterRole,
        /// Cluster epoch observed after removal.
        epoch: ClusterEpoch,
    },
    /// A topology table was explicitly committed as authoritative.
    CommitTopology {
        /// Committed topology epoch.
        epoch: ClusterEpoch,
        /// Authoritative member ids in stable order.
        members: Vec<ClusterNodeId>,
    },
}

/// Snapshot of the raft-style metadata journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftMetadataSnapshot {
    /// Simulated Raft term.
    pub term: u64,
    /// Number of committed metadata commands.
    pub commit_index: u64,
    /// Current cluster metadata epoch.
    pub epoch: ClusterEpoch,
    /// Current admitted member count.
    pub member_count: usize,
    /// Current connected client count.
    pub client_count: usize,
    /// Last committed command, if any.
    pub last_command: Option<RaftMetadataCommand>,
}

#[derive(Debug)]
struct RaftMetadataState {
    term: u64,
    commit_index: u64,
    commands: Vec<RaftMetadataCommand>,
}

impl Default for RaftMetadataState {
    fn default() -> Self {
        Self {
            term: 1,
            commit_index: 0,
            commands: Vec::new(),
        }
    }
}

/// Dependency-free, raft-style cluster metadata control plane.
///
/// This adapter does not run the real `raft-rs` protocol yet. It models the
/// part of Raft that HydraCache's public cluster API needs before a networked
/// implementation exists: successful membership changes are appended to a
/// committed metadata log, exposed through a snapshot, and used by the same
/// [`ClusterControlPlane`] trait as other adapters.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use hydracache::{HydraCache, RaftStyleMetadataControlPlane};
///
/// # #[tokio::main]
/// # async fn main() -> hydracache::CacheResult<()> {
/// let control_plane = Arc::new(RaftStyleMetadataControlPlane::new("orders"));
///
/// let member = HydraCache::member()
///     .control_plane(control_plane.clone())
///     .node_id("member-a")
///     .start()
///     .await?;
///
/// assert_eq!(control_plane.snapshot().commit_index, 1);
/// assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RaftStyleMetadataControlPlane {
    cluster: InMemoryCluster,
    metadata: Mutex<RaftMetadataState>,
}

impl RaftStyleMetadataControlPlane {
    /// Create a raft-style metadata control plane for a logical cluster.
    pub fn new(cluster_name: impl Into<String>) -> Self {
        Self {
            cluster: InMemoryCluster::new(cluster_name),
            metadata: Mutex::new(RaftMetadataState::default()),
        }
    }

    /// Override the simulated Raft term.
    pub fn with_term(mut self, term: u64) -> Self {
        self.metadata
            .get_mut()
            .expect("raft metadata poisoned")
            .term = term;
        self
    }

    /// Return committed metadata commands.
    pub fn commands(&self) -> Vec<RaftMetadataCommand> {
        self.metadata
            .lock()
            .expect("raft metadata poisoned")
            .commands
            .clone()
    }

    /// Return a point-in-time metadata snapshot.
    pub fn snapshot(&self) -> RaftMetadataSnapshot {
        let metadata = self.metadata.lock().expect("raft metadata poisoned");
        RaftMetadataSnapshot {
            term: metadata.term,
            commit_index: metadata.commit_index,
            epoch: self.cluster.epoch(),
            member_count: self.cluster.members().len(),
            client_count: self.cluster.clients().len(),
            last_command: metadata.commands.last().cloned(),
        }
    }

    fn append_command(&self, command: RaftMetadataCommand) {
        let mut metadata = self.metadata.lock().expect("raft metadata poisoned");
        metadata.commit_index = metadata.commit_index.saturating_add(1);
        metadata.commands.push(command);
    }

    /// Commit the current admitted member table as authoritative topology.
    pub fn commit_topology(&self) -> RaftMetadataSnapshot {
        let mut members = self
            .cluster
            .members()
            .into_iter()
            .map(|member| member.node_id)
            .collect::<Vec<_>>();
        members.sort();
        self.append_command(RaftMetadataCommand::CommitTopology {
            epoch: self.cluster.epoch(),
            members,
        });
        self.snapshot()
    }
}

impl Default for RaftStyleMetadataControlPlane {
    fn default() -> Self {
        Self::new("hydracache")
    }
}

#[async_trait::async_trait]
impl ClusterControlPlane for RaftStyleMetadataControlPlane {
    fn name(&self) -> String {
        self.cluster.name().to_owned()
    }

    fn invalidation_bus(&self) -> Arc<dyn CacheInvalidationBus> {
        self.cluster.invalidation_bus()
    }

    async fn join_member(&self, candidate: ClusterCandidate) -> Result<ClusterMember> {
        let member = self.cluster.join_member(candidate)?;
        self.append_command(RaftMetadataCommand::MemberUpsert {
            node_id: member.node_id.clone(),
            generation: member.generation,
            epoch: member.epoch,
        });
        Ok(member)
    }

    async fn join_client(&self, candidate: ClusterCandidate) -> Result<ClusterMember> {
        let member = self.cluster.join_client(candidate)?;
        self.append_command(RaftMetadataCommand::ClientUpsert {
            node_id: member.node_id.clone(),
            generation: member.generation,
            epoch: member.epoch,
        });
        Ok(member)
    }

    async fn validate_generation(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<()> {
        self.cluster.validate_generation(node_id, generation)
    }

    async fn leave(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<Option<ClusterMembershipEvent>> {
        let Some(event) = self.cluster.leave(node_id, generation)? else {
            return Ok(None);
        };
        if let ClusterMembershipEvent::NodeLeft {
            node_id,
            role,
            epoch,
        } = &event
        {
            self.append_command(RaftMetadataCommand::NodeLeft {
                node_id: node_id.clone(),
                role: *role,
                epoch: *epoch,
            });
        }
        Ok(Some(event))
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

    fn ownership_diagnostics(&self) -> ClusterOwnershipDiagnostics {
        self.cluster.ownership_diagnostics()
    }
}

#[derive(Debug, Default)]
struct InMemoryClusterState {
    epoch: ClusterEpoch,
    topology_stamp: u64,
    members: BTreeMap<ClusterNodeId, ClusterMember>,
    clients: BTreeMap<ClusterNodeId, ClusterMember>,
    events: Vec<ClusterMembershipEvent>,
    ownership_resolutions: u64,
    ownership_no_owner: u64,
}

/// In-process cluster model for tests, demos, and the first client/member API.
///
/// This is intentionally not a network cluster. It gives HydraCache a stable
/// cluster API shape while chitchat, Raft, and libp2p adapters are still being
/// designed.
#[derive(Debug)]
pub struct InMemoryCluster {
    name: String,
    invalidation_bus: Arc<InMemoryInvalidationBus>,
    membership_events: ClusterMembershipEventBus,
    state: Mutex<InMemoryClusterState>,
}

impl InMemoryCluster {
    /// Create an in-memory cluster model.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            invalidation_bus: Arc::new(InMemoryInvalidationBus::default()),
            membership_events: ClusterMembershipEventBus::default(),
            state: Mutex::new(InMemoryClusterState::default()),
        }
    }

    /// Return the cluster name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the shared invalidation bus used by this in-memory cluster.
    pub fn invalidation_bus(&self) -> Arc<dyn CacheInvalidationBus> {
        self.invalidation_bus.clone()
    }

    /// Return the current simulated cluster epoch.
    pub fn epoch(&self) -> ClusterEpoch {
        self.state.lock().expect("cluster state poisoned").epoch
    }

    /// Admit or update a member candidate.
    pub fn join_member(&self, candidate: ClusterCandidate) -> Result<ClusterMember> {
        self.join(candidate, ClusterRole::Member)
    }

    /// Connect or update a client candidate.
    pub fn join_client(&self, candidate: ClusterCandidate) -> Result<ClusterMember> {
        self.join(candidate, ClusterRole::Client)
    }

    fn join(&self, mut candidate: ClusterCandidate, role: ClusterRole) -> Result<ClusterMember> {
        candidate.role = role;
        let mut state = self.state.lock().expect("cluster state poisoned");
        reject_stale_generation(&mut state, &self.membership_events, &candidate)?;

        match role {
            ClusterRole::Local => Err(CacheError::Backend(
                "local caches cannot join an in-memory cluster".to_owned(),
            )),
            ClusterRole::Client => {
                let member = ClusterMember::from_candidate(candidate, state.epoch);
                state.clients.insert(member.node_id.clone(), member.clone());
                let event = ClusterMembershipEvent::ClientConnected(member.clone());
                state.events.push(event.clone());
                self.membership_events.publish(event);
                Ok(member)
            }
            ClusterRole::Member => {
                let should_advance_epoch = state
                    .members
                    .get(&candidate.node_id)
                    .map(|existing| existing.generation < candidate.generation)
                    .unwrap_or(true);
                if should_advance_epoch {
                    state.epoch.advance();
                    state.topology_stamp = state.topology_stamp.saturating_add(1);
                }
                state.clients.remove(&candidate.node_id);
                let member = ClusterMember::from_candidate(candidate, state.epoch);
                state.members.insert(member.node_id.clone(), member.clone());
                let event = ClusterMembershipEvent::MemberJoined(member.clone());
                state.events.push(event.clone());
                self.membership_events.publish(event);
                Ok(member)
            }
        }
    }

    /// Validate that a node id is still owned by the provided generation.
    pub fn validate_generation(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<()> {
        let mut state = self.state.lock().expect("cluster state poisoned");
        validate_generation_locked(&mut state, &self.membership_events, node_id, generation)
    }

    /// Remove a node from the in-memory cluster model when generation matches.
    pub fn leave(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<Option<ClusterMembershipEvent>> {
        let mut state = self.state.lock().expect("cluster state poisoned");
        if current_generation_locked(&state, node_id).is_none() {
            return Ok(None);
        }
        validate_generation_locked(&mut state, &self.membership_events, node_id, generation)?;
        let removed_member = state.members.remove(node_id);
        let removed_client = state.clients.remove(node_id);
        let Some(removed) = removed_member.or(removed_client) else {
            return Ok(None);
        };
        if removed.role == ClusterRole::Member {
            state.epoch.advance();
            state.topology_stamp = state.topology_stamp.saturating_add(1);
        }
        let event = ClusterMembershipEvent::NodeLeft {
            node_id: removed.node_id,
            role: removed.role,
            epoch: state.epoch,
        };
        state.events.push(event.clone());
        self.membership_events.publish(event.clone());
        Ok(Some(event))
    }

    /// Return admitted member snapshots.
    pub fn members(&self) -> Vec<ClusterMember> {
        self.state
            .lock()
            .expect("cluster state poisoned")
            .members
            .values()
            .cloned()
            .collect()
    }

    /// Return connected client snapshots.
    pub fn clients(&self) -> Vec<ClusterMember> {
        self.state
            .lock()
            .expect("cluster state poisoned")
            .clients
            .values()
            .cloned()
            .collect()
    }

    /// Resolve which admitted member owns a logical cache key.
    ///
    /// This is a local, deterministic decision over the current in-memory
    /// member view. It does not load values or contact the owner.
    pub fn owner_for_key(&self, key: impl AsRef<str>) -> ClusterOwnershipDecision {
        self.owner_for_key_with(key, &RendezvousClusterOwnership)
    }

    /// Resolve ownership with a custom resolver.
    pub fn owner_for_key_with(
        &self,
        key: impl AsRef<str>,
        resolver: &dyn ClusterOwnershipResolver,
    ) -> ClusterOwnershipDecision {
        let key = key.as_ref();
        let members = {
            let mut state = self.state.lock().expect("cluster state poisoned");
            state.ownership_resolutions = state.ownership_resolutions.saturating_add(1);
            state.members.values().cloned().collect::<Vec<_>>()
        };
        let decision = resolver.resolve_owner(key, &members);
        if !decision.has_owner() {
            let mut state = self.state.lock().expect("cluster state poisoned");
            state.ownership_no_owner = state.ownership_no_owner.saturating_add(1);
        }
        decision
    }

    /// Return ownership diagnostics for this in-memory model.
    pub fn ownership_diagnostics(&self) -> ClusterOwnershipDiagnostics {
        let state = self.state.lock().expect("cluster state poisoned");
        ClusterOwnershipDiagnostics::new(
            RendezvousClusterOwnership.name(),
            state.ownership_resolutions,
            state.ownership_no_owner,
            state.topology_stamp,
        )
    }

    /// Return membership events recorded by the in-memory model.
    pub fn events(&self) -> Vec<ClusterMembershipEvent> {
        self.state
            .lock()
            .expect("cluster state poisoned")
            .events
            .clone()
    }

    /// Subscribe to membership events emitted after subscription.
    pub fn subscribe_membership(&self) -> ClusterMembershipSubscriber {
        self.membership_events.subscribe()
    }

    fn diagnostics_for(
        &self,
        role: ClusterRole,
        node_id: ClusterNodeId,
        generation: ClusterGeneration,
        bootstrap: Vec<String>,
    ) -> ClusterDiagnostics {
        let state = self.state.lock().expect("cluster state poisoned");
        ClusterDiagnostics {
            cluster_name: self.name.clone(),
            role,
            node_id,
            generation,
            epoch: state.epoch,
            member_count: state.members.len(),
            client_count: state.clients.len(),
            bootstrap,
            connected: true,
            invalidation_subscribers: self.invalidation_bus.receiver_count(),
            membership_subscribers: self.membership_events.receiver_count(),
            lifecycle: ClusterLifecycleDiagnostics::running("cluster-runtime"),
        }
    }
}

#[async_trait::async_trait]
impl ClusterControlPlane for InMemoryCluster {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn invalidation_bus(&self) -> Arc<dyn CacheInvalidationBus> {
        InMemoryCluster::invalidation_bus(self)
    }

    async fn join_member(&self, candidate: ClusterCandidate) -> Result<ClusterMember> {
        InMemoryCluster::join_member(self, candidate)
    }

    async fn join_client(&self, candidate: ClusterCandidate) -> Result<ClusterMember> {
        InMemoryCluster::join_client(self, candidate)
    }

    async fn validate_generation(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<()> {
        InMemoryCluster::validate_generation(self, node_id, generation)
    }

    async fn leave(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<Option<ClusterMembershipEvent>> {
        InMemoryCluster::leave(self, node_id, generation)
    }

    fn subscribe_membership(&self) -> ClusterMembershipSubscriber {
        InMemoryCluster::subscribe_membership(self)
    }

    fn diagnostics_for(
        &self,
        role: ClusterRole,
        node_id: ClusterNodeId,
        generation: ClusterGeneration,
        bootstrap: Vec<String>,
    ) -> ClusterDiagnostics {
        InMemoryCluster::diagnostics_for(self, role, node_id, generation, bootstrap)
    }

    fn ownership_diagnostics(&self) -> ClusterOwnershipDiagnostics {
        InMemoryCluster::ownership_diagnostics(self)
    }
}

fn reject_stale_generation(
    state: &mut InMemoryClusterState,
    membership_events: &ClusterMembershipEventBus,
    candidate: &ClusterCandidate,
) -> Result<()> {
    let existing_generation = state
        .members
        .get(&candidate.node_id)
        .or_else(|| state.clients.get(&candidate.node_id))
        .map(|existing| existing.generation);

    let Some(existing) = existing_generation else {
        return Ok(());
    };
    if candidate.generation >= existing {
        return Ok(());
    }

    let event = ClusterMembershipEvent::StaleGenerationRejected {
        node_id: candidate.node_id.clone(),
        role: candidate.role,
        existing,
        attempted: candidate.generation,
        reason: "stale-generation".to_owned(),
    };
    state.events.push(event.clone());
    membership_events.publish(event);
    Err(CacheError::Backend(format!(
        "stale cluster generation for node '{}': existing {}, attempted {}",
        candidate.node_id,
        existing.value(),
        candidate.generation.value()
    )))
}

fn current_generation_locked(
    state: &InMemoryClusterState,
    node_id: &ClusterNodeId,
) -> Option<ClusterGeneration> {
    state
        .members
        .get(node_id)
        .or_else(|| state.clients.get(node_id))
        .map(|existing| existing.generation)
}

fn validate_generation_locked(
    state: &mut InMemoryClusterState,
    membership_events: &ClusterMembershipEventBus,
    node_id: &ClusterNodeId,
    generation: ClusterGeneration,
) -> Result<()> {
    let Some(existing_member) = state
        .members
        .get(node_id)
        .or_else(|| state.clients.get(node_id))
    else {
        return Err(CacheError::Backend(format!(
            "cluster node '{node_id}' is not admitted"
        )));
    };
    let existing = existing_member.generation;
    let role = existing_member.role;

    if existing == generation {
        return Ok(());
    }

    let event = ClusterMembershipEvent::StaleGenerationRejected {
        node_id: node_id.clone(),
        role,
        existing,
        attempted: generation,
        reason: "generation-mismatch".to_owned(),
    };
    state.events.push(event.clone());
    membership_events.publish(event);
    Err(CacheError::Backend(format!(
        "stale cluster generation for node '{}': existing {}, attempted {}",
        node_id,
        existing.value(),
        generation.value()
    )))
}

#[derive(Debug, Clone)]
pub(crate) struct ClusterRuntime {
    control_plane: Arc<dyn ClusterControlPlane>,
    discovery: Option<Arc<dyn ClusterDiscovery>>,
    role: ClusterRole,
    node_id: ClusterNodeId,
    generation: ClusterGeneration,
    bootstrap: Vec<String>,
    lifecycle: Arc<Mutex<ClusterLifecycleDiagnostics>>,
}

impl ClusterRuntime {
    fn new(
        control_plane: Arc<dyn ClusterControlPlane>,
        discovery: Option<Arc<dyn ClusterDiscovery>>,
        role: ClusterRole,
        node_id: ClusterNodeId,
        generation: ClusterGeneration,
        bootstrap: Vec<String>,
    ) -> Self {
        let mut lifecycle = ClusterLifecycleDiagnostics::idle(runtime_lifecycle_component(role));
        lifecycle.record_start();
        Self {
            control_plane,
            discovery,
            role,
            node_id,
            generation,
            bootstrap,
            lifecycle: Arc::new(Mutex::new(lifecycle)),
        }
    }

    pub(crate) fn diagnostics(&self) -> ClusterDiagnostics {
        let mut diagnostics = self.control_plane.diagnostics_for(
            self.role,
            self.node_id.clone(),
            self.generation,
            self.bootstrap.clone(),
        );
        diagnostics.lifecycle = self
            .lifecycle
            .lock()
            .expect("cluster runtime lifecycle poisoned")
            .clone();
        diagnostics
    }

    pub(crate) fn ownership_diagnostics(&self) -> ClusterOwnershipDiagnostics {
        self.control_plane.ownership_diagnostics()
    }

    pub(crate) fn discovery_diagnostics(&self) -> Option<ClusterDiscoveryDiagnostics> {
        let discovery = self.discovery.as_ref()?;
        Some(ClusterDiscoveryDiagnostics {
            local_node_id: self.node_id.clone(),
            candidates: discovery.candidates(),
            events: discovery.events(),
        })
    }

    pub(crate) fn generation(&self) -> ClusterGeneration {
        self.generation
    }

    pub(crate) async fn validate_generation(&self) -> Result<()> {
        self.control_plane
            .validate_generation(&self.node_id, self.generation)
            .await
    }

    pub(crate) fn subscribe_membership(&self) -> ClusterMembershipSubscriber {
        self.control_plane.subscribe_membership()
    }

    pub(crate) async fn validate_remote_generation(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<()> {
        self.control_plane
            .validate_generation(node_id, generation)
            .await
    }

    pub(crate) async fn leave(&self) -> Result<Option<ClusterMembershipEvent>> {
        let event = self
            .control_plane
            .leave(&self.node_id, self.generation)
            .await?;
        if event.is_some() {
            let mut lifecycle = self
                .lifecycle
                .lock()
                .expect("cluster runtime lifecycle poisoned");
            lifecycle.record_shutdown_requested();
            lifecycle.record_graceful_stop();
        }
        Ok(event)
    }
}

fn runtime_lifecycle_component(role: ClusterRole) -> &'static str {
    match role {
        ClusterRole::Local => "cluster-runtime:local",
        ClusterRole::Client => "cluster-runtime:client",
        ClusterRole::Member => "cluster-runtime:member",
    }
}

fn default_control_plane(cluster_name: String) -> Arc<dyn ClusterControlPlane> {
    Arc::new(InMemoryCluster::new(cluster_name))
}

/// Builder for a client near-cache connected to a HydraCache cluster.
#[derive(Debug, Clone)]
pub struct HydraCacheClientBuilder<C = PostcardCodec>
where
    C: CacheCodec,
{
    cache_builder: HydraCacheBuilder<C>,
    cluster_name: String,
    bootstrap: Vec<String>,
    control_plane: Option<Arc<dyn ClusterControlPlane>>,
    discovery: Option<Arc<dyn ClusterDiscovery>>,
    node_id: Option<ClusterNodeId>,
    generation: ClusterGeneration,
    endpoints: ClusterEndpoints,
}

impl HydraCacheClientBuilder<PostcardCodec> {
    pub(crate) fn default() -> Self {
        Self {
            cache_builder: HydraCacheBuilder::default(),
            cluster_name: "hydracache".to_owned(),
            bootstrap: Vec::new(),
            control_plane: None,
            discovery: None,
            node_id: None,
            generation: ClusterGeneration::default(),
            endpoints: ClusterEndpoints::default(),
        }
    }
}

impl<C> HydraCacheClientBuilder<C>
where
    C: CacheCodec,
{
    /// Set the logical cluster name.
    pub fn cluster(mut self, name: impl Into<String>) -> Self {
        self.cluster_name = name.into();
        self
    }

    /// Add a bootstrap address.
    ///
    /// v0.20 stores this as diagnostics metadata. Real network dialing belongs
    /// to a future transport adapter.
    pub fn bootstrap(mut self, address: impl Into<String>) -> Self {
        self.bootstrap.push(address.into());
        self
    }

    /// Attach an in-process cluster model.
    pub fn shared_cluster(mut self, cluster: Arc<InMemoryCluster>) -> Self {
        self.control_plane = Some(cluster);
        self
    }

    /// Attach a custom cluster control-plane adapter.
    ///
    /// Use this for future networked or Raft-backed implementations. The
    /// adapter is responsible for admission decisions and for returning the
    /// invalidation bus that the cache should use after admission.
    pub fn control_plane(mut self, control_plane: Arc<dyn ClusterControlPlane>) -> Self {
        self.control_plane = Some(control_plane);
        self
    }

    /// Attach an in-process discovery journal.
    pub fn shared_discovery(mut self, discovery: Arc<InMemoryClusterDiscovery>) -> Self {
        self.discovery = Some(discovery);
        self
    }

    /// Attach a custom discovery adapter.
    ///
    /// Use this for future chitchat, DNS, mDNS, or P2P-backed discovery. The
    /// adapter observes candidates and liveness; the control plane still owns
    /// authoritative admission.
    pub fn discovery(mut self, discovery: Arc<dyn ClusterDiscovery>) -> Self {
        self.discovery = Some(discovery);
        self
    }

    /// Set the client node id.
    pub fn node_id(mut self, node_id: impl Into<ClusterNodeId>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    /// Set the client process generation.
    pub fn generation(mut self, generation: ClusterGeneration) -> Self {
        self.generation = generation;
        self
    }

    /// Set an advertised control endpoint.
    pub fn control_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoints = self.endpoints.control(endpoint);
        self
    }

    /// Set an advertised diagnostics endpoint.
    pub fn diagnostics_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoints = self.endpoints.diagnostics(endpoint);
        self
    }

    /// Set near-cache capacity.
    pub fn near_cache_capacity(mut self, max_capacity: u64) -> Self {
        self.cache_builder = self.cache_builder.max_capacity(max_capacity);
        self
    }

    /// Set maximum encoded entry size in bytes.
    pub fn max_entry_bytes(mut self, max_entry_bytes: usize) -> Self {
        self.cache_builder = self.cache_builder.max_entry_bytes(max_entry_bytes);
        self
    }

    /// Set the default TTL for the client near-cache.
    pub fn default_ttl(mut self, default_ttl: Duration) -> Self {
        self.cache_builder = self.cache_builder.default_ttl(default_ttl);
        self
    }

    /// Enable high-volume access events on the client near-cache.
    pub fn enable_access_events(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.enable_access_events(enabled);
        self
    }

    /// Declare whether HydraCache transport auth is configured.
    pub fn transport_auth_configured(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.transport_auth_configured(enabled);
        self
    }

    /// Declare whether strict current wire compatibility is configured.
    pub fn strict_wire_compatibility(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.strict_wire_compatibility(enabled);
        self
    }

    /// Declare that an external mesh/mTLS boundary handles transport identity.
    pub fn declare_mesh_boundary(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.declare_mesh_boundary(enabled);
        self
    }

    /// Set the cluster client routing mode.
    pub fn routing_mode(mut self, routing_mode: RoutingMode) -> Self {
        self.cache_builder = self.cache_builder.routing_mode(routing_mode);
        self
    }

    /// Enable or disable cluster read-through/remote peer-fetch paths.
    pub fn read_through_enabled(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.read_through_enabled(enabled);
        self
    }

    /// Set the bounded event buffer capacity.
    pub fn event_buffer_capacity(mut self, capacity: usize) -> Self {
        self.cache_builder = self.cache_builder.event_buffer_capacity(capacity);
        self
    }

    /// Replace the default codec.
    pub fn codec<Next>(self, codec: Next) -> HydraCacheClientBuilder<Next>
    where
        Next: CacheCodec,
    {
        HydraCacheClientBuilder {
            cache_builder: self.cache_builder.codec(codec),
            cluster_name: self.cluster_name,
            bootstrap: self.bootstrap,
            control_plane: self.control_plane,
            discovery: self.discovery,
            node_id: self.node_id,
            generation: self.generation,
            endpoints: self.endpoints,
        }
    }

    /// Connect the client near-cache.
    pub async fn connect(self) -> Result<HydraCache<C>> {
        let control_plane = self
            .control_plane
            .unwrap_or_else(|| default_control_plane(self.cluster_name.clone()));
        let node_id = self.node_id.unwrap_or_else(next_client_id);
        let discovery = self.discovery.clone();
        let candidate = ClusterCandidate::client(node_id.clone())
            .generation(self.generation)
            .endpoints(self.endpoints);
        if let Some(discovery) = &discovery {
            discovery.announce(candidate.clone()).await?;
        }
        let admitted = control_plane.join_client(candidate).await?;

        Ok(self
            .cache_builder
            .shared_invalidation_bus(control_plane.invalidation_bus())
            .invalidation_node_id(admitted.node_id.as_str())
            .cluster_runtime(ClusterRuntime::new(
                control_plane,
                discovery,
                ClusterRole::Client,
                admitted.node_id,
                admitted.generation,
                self.bootstrap,
            ))
            .build())
    }
}

/// Builder for an in-process HydraCache cluster member.
#[derive(Debug, Clone)]
pub struct HydraCacheMemberBuilder<C = PostcardCodec>
where
    C: CacheCodec,
{
    cache_builder: HydraCacheBuilder<C>,
    cluster_name: String,
    bootstrap: Vec<String>,
    control_plane: Option<Arc<dyn ClusterControlPlane>>,
    discovery: Option<Arc<dyn ClusterDiscovery>>,
    node_id: Option<ClusterNodeId>,
    generation: ClusterGeneration,
    endpoints: ClusterEndpoints,
}

impl HydraCacheMemberBuilder<PostcardCodec> {
    pub(crate) fn default() -> Self {
        Self {
            cache_builder: HydraCacheBuilder::default(),
            cluster_name: "hydracache".to_owned(),
            bootstrap: Vec::new(),
            control_plane: None,
            discovery: None,
            node_id: None,
            generation: ClusterGeneration::default(),
            endpoints: ClusterEndpoints::default(),
        }
    }
}

impl<C> HydraCacheMemberBuilder<C>
where
    C: CacheCodec,
{
    /// Set the logical cluster name.
    pub fn cluster(mut self, name: impl Into<String>) -> Self {
        self.cluster_name = name.into();
        self
    }

    /// Add a bootstrap address.
    pub fn bootstrap(mut self, address: impl Into<String>) -> Self {
        self.bootstrap.push(address.into());
        self
    }

    /// Attach an in-process cluster model.
    pub fn shared_cluster(mut self, cluster: Arc<InMemoryCluster>) -> Self {
        self.control_plane = Some(cluster);
        self
    }

    /// Attach a custom cluster control-plane adapter.
    ///
    /// Use this for future networked or Raft-backed implementations. The
    /// adapter is responsible for admission decisions and for returning the
    /// invalidation bus that the cache should use after admission.
    pub fn control_plane(mut self, control_plane: Arc<dyn ClusterControlPlane>) -> Self {
        self.control_plane = Some(control_plane);
        self
    }

    /// Attach an in-process discovery journal.
    pub fn shared_discovery(mut self, discovery: Arc<InMemoryClusterDiscovery>) -> Self {
        self.discovery = Some(discovery);
        self
    }

    /// Attach a custom discovery adapter.
    ///
    /// Use this for future chitchat, DNS, mDNS, or P2P-backed discovery. The
    /// adapter observes candidates and liveness; the control plane still owns
    /// authoritative admission.
    pub fn discovery(mut self, discovery: Arc<dyn ClusterDiscovery>) -> Self {
        self.discovery = Some(discovery);
        self
    }

    /// Set the member node id.
    pub fn node_id(mut self, node_id: impl Into<ClusterNodeId>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    /// Set the member process generation.
    pub fn generation(mut self, generation: ClusterGeneration) -> Self {
        self.generation = generation;
        self
    }

    /// Set the bind address used for member control and invalidation metadata.
    pub fn bind(mut self, address: impl Into<String>) -> Self {
        let address = address.into();
        self.endpoints = self
            .endpoints
            .control(address.clone())
            .invalidation(address);
        self
    }

    /// Set an advertised diagnostics endpoint.
    pub fn diagnostics_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoints = self.endpoints.diagnostics(endpoint);
        self
    }

    /// Set local member cache capacity.
    pub fn cache_capacity(mut self, max_capacity: u64) -> Self {
        self.cache_builder = self.cache_builder.max_capacity(max_capacity);
        self
    }

    /// Set maximum encoded entry size in bytes.
    pub fn max_entry_bytes(mut self, max_entry_bytes: usize) -> Self {
        self.cache_builder = self.cache_builder.max_entry_bytes(max_entry_bytes);
        self
    }

    /// Set the default TTL for the member local cache.
    pub fn default_ttl(mut self, default_ttl: Duration) -> Self {
        self.cache_builder = self.cache_builder.default_ttl(default_ttl);
        self
    }

    /// Enable high-volume access events on the member local cache.
    pub fn enable_access_events(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.enable_access_events(enabled);
        self
    }

    /// Declare whether HydraCache transport auth is configured.
    pub fn transport_auth_configured(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.transport_auth_configured(enabled);
        self
    }

    /// Declare whether strict current wire compatibility is configured.
    pub fn strict_wire_compatibility(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.strict_wire_compatibility(enabled);
        self
    }

    /// Declare that an external mesh/mTLS boundary handles transport identity.
    pub fn declare_mesh_boundary(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.declare_mesh_boundary(enabled);
        self
    }

    /// Set the member routing mode.
    pub fn routing_mode(mut self, routing_mode: RoutingMode) -> Self {
        self.cache_builder = self.cache_builder.routing_mode(routing_mode);
        self
    }

    /// Enable or disable cluster read-through/remote peer-fetch paths.
    pub fn read_through_enabled(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.read_through_enabled(enabled);
        self
    }

    /// Set the bounded event buffer capacity.
    pub fn event_buffer_capacity(mut self, capacity: usize) -> Self {
        self.cache_builder = self.cache_builder.event_buffer_capacity(capacity);
        self
    }

    /// Replace the default codec.
    pub fn codec<Next>(self, codec: Next) -> HydraCacheMemberBuilder<Next>
    where
        Next: CacheCodec,
    {
        HydraCacheMemberBuilder {
            cache_builder: self.cache_builder.codec(codec),
            cluster_name: self.cluster_name,
            bootstrap: self.bootstrap,
            control_plane: self.control_plane,
            discovery: self.discovery,
            node_id: self.node_id,
            generation: self.generation,
            endpoints: self.endpoints,
        }
    }

    /// Start the member runtime.
    pub async fn start(self) -> Result<HydraCache<C>> {
        let control_plane = self
            .control_plane
            .unwrap_or_else(|| default_control_plane(self.cluster_name.clone()));
        let node_id = self.node_id.unwrap_or_else(next_member_id);
        let discovery = self.discovery.clone();
        let candidate = ClusterCandidate::member(node_id.clone())
            .generation(self.generation)
            .endpoints(self.endpoints);
        if let Some(discovery) = &discovery {
            discovery.announce(candidate.clone()).await?;
        }
        let admitted = control_plane.join_member(candidate).await?;

        Ok(self
            .cache_builder
            .shared_invalidation_bus(control_plane.invalidation_bus())
            .invalidation_node_id(admitted.node_id.as_str())
            .cluster_runtime(ClusterRuntime::new(
                control_plane,
                discovery,
                ClusterRole::Member,
                admitted.node_id,
                admitted.generation,
                self.bootstrap,
            ))
            .build())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::{
        ChitchatStyleDiscovery, ClusterAdmissionBridge, ClusterAdmissionBridgeConfig,
        ClusterAdmissionBridgeDiagnostics, ClusterAdmissionBridgeEvent,
        ClusterAdmissionIgnoreReason, ClusterAdmissionRejectReason, ClusterCandidate,
        ClusterComponent, ClusterControlPlane, ClusterDiagnostics, ClusterDiscovery,
        ClusterDiscoveryDiagnostics, ClusterDiscoveryEvent, ClusterEndpoints, ClusterEpoch,
        ClusterFillCounters, ClusterGeneration, ClusterHealthReason, ClusterHealthState,
        ClusterLifecycleComponent, ClusterLifecycleDiagnostics, ClusterLifecycleStatus,
        ClusterLoadReport, ClusterMember, ClusterMembershipEvent, ClusterMembershipEventBus,
        ClusterMembershipRecvError, ClusterNodeId, ClusterOwnershipDecision,
        ClusterOwnershipResolver, ClusterPeerFetch, ClusterPeerFetchGenerationMismatch,
        ClusterPeerFetchRequest, ClusterPeerFetchResponse, ClusterRole, ClusterStagingCounters,
        ClusterStagingHealth, InMemoryCluster, InMemoryClusterDiscovery, InMemoryPeerFetch,
        RendezvousClusterOwnership, CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY,
    };
    use crate::HydraCache;
    use bytes::Bytes;
    use hydracache_core::CacheStats;

    #[test]
    fn node_id_formats_and_converts_from_strings() {
        let id = ClusterNodeId::from("node-a");
        assert_eq!(id.as_str(), "node-a");
        assert_eq!(id.to_string(), "node-a");

        let owned = ClusterNodeId::from("node-b".to_owned());
        assert!(owned > id);
    }

    #[test]
    fn generation_ordering_tracks_restarts() {
        let first = ClusterGeneration::new(7);
        let second = first.next();

        assert_eq!(first.value(), 7);
        assert_eq!(second.value(), 8);
        assert!(second > first);
    }

    #[test]
    fn lifecycle_diagnostics_track_start_stop_and_helpers() {
        let mut lifecycle = ClusterLifecycleDiagnostics::idle("admission-bridge");

        assert_eq!(lifecycle.component, "admission-bridge");
        assert_eq!(lifecycle.status, ClusterLifecycleStatus::Idle);
        assert!(!lifecycle.is_terminal());

        lifecycle.record_start();
        assert!(lifecycle.is_running());
        assert_eq!(lifecycle.start_count, 1);
        assert_eq!(lifecycle.stop_count, 0);
        assert!(!lifecycle.shutdown_requested);

        lifecycle.record_shutdown_requested();
        assert!(lifecycle.is_stopping());
        assert!(lifecycle.shutdown_requested);

        lifecycle.record_graceful_stop();
        assert!(lifecycle.is_stopped());
        assert!(lifecycle.is_terminal());
        assert_eq!(lifecycle.stop_count, 1);

        lifecycle.record_start();
        assert!(lifecycle.is_running());
        assert_eq!(lifecycle.start_count, 2);
        assert!(!lifecycle.shutdown_requested);
    }

    #[test]
    fn lifecycle_diagnostics_report_failure_and_running_constructor() {
        let mut lifecycle = ClusterLifecycleDiagnostics::running("peer-fetch");

        assert_eq!(lifecycle.status, ClusterLifecycleStatus::Running);
        assert_eq!(lifecycle.start_count, 1);

        lifecycle.record_failure("socket closed");
        assert!(lifecycle.has_failed());
        assert!(lifecycle.is_terminal());
        assert_eq!(lifecycle.last_error.as_deref(), Some("socket closed"));

        lifecycle.record_shutdown_requested();
        assert!(lifecycle.has_failed());
        assert!(lifecycle.shutdown_requested);
    }

    #[tokio::test]
    async fn component_start_is_idempotent() {
        let component = ClusterLifecycleComponent::new("invalidation-pump");

        component.start().await.unwrap();
        component.start().await.unwrap();

        let diagnostics = component.diagnostics();
        assert_eq!(component.name(), "invalidation-pump");
        assert!(diagnostics.is_running());
        assert_eq!(diagnostics.start_count, 1);
    }

    #[tokio::test]
    async fn component_stop_records_graceful_stop() {
        let component = ClusterLifecycleComponent::new("transport-server");

        component.start().await.unwrap();
        component.stop().await.unwrap();
        component.stop().await.unwrap();

        let diagnostics = component.diagnostics();
        assert!(diagnostics.is_stopped());
        assert_eq!(diagnostics.stop_count, 1);
        assert!(component.last_error().is_none());
    }

    #[tokio::test]
    async fn component_failure_sets_last_error_and_failed_status() {
        let component = ClusterLifecycleComponent::new("discovery-bridge");

        component.start().await.unwrap();
        component.fail("listener closed");

        let diagnostics = component.diagnostics();
        assert!(diagnostics.has_failed());
        assert_eq!(component.last_error().as_deref(), Some("listener closed"));
    }

    #[test]
    fn staging_health_healthy_cluster_is_healthy() {
        let health = ClusterStagingHealth::from_parts(
            staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
            CacheStats::default(),
            ClusterFillCounters::default(),
            ClusterStagingCounters::default(),
        );

        assert_eq!(health.state, ClusterHealthState::Healthy);
        assert!(health.state.ready_for_staging());
        assert!(health.state.reasons().is_empty());
    }

    #[test]
    fn staging_health_lagged_receiver_is_degraded() {
        let health = ClusterStagingHealth::from_parts(
            staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
            CacheStats {
                distributed_invalidation_lagged: 1,
                ..CacheStats::default()
            },
            ClusterFillCounters::default(),
            ClusterStagingCounters::default(),
        );

        assert_eq!(
            health.state,
            ClusterHealthState::Degraded {
                reasons: vec![ClusterHealthReason::LaggedReceivers { count: 1 }]
            }
        );
        assert!(!health.state.ready_for_staging());
    }

    #[test]
    fn staging_health_decode_error_is_not_ready() {
        let health = ClusterStagingHealth::from_parts(
            staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
            CacheStats {
                distributed_invalidation_decode_errors: 1,
                ..CacheStats::default()
            },
            ClusterFillCounters::default(),
            ClusterStagingCounters::default(),
        );

        assert_eq!(
            health.state,
            ClusterHealthState::NotReady {
                reasons: vec![ClusterHealthReason::DecodeErrors { count: 1 }]
            }
        );
    }

    #[test]
    fn staging_health_stale_generation_alone_stays_healthy() {
        let health = ClusterStagingHealth::from_parts(
            staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
            CacheStats::default(),
            ClusterFillCounters::default(),
            ClusterStagingCounters {
                stale_generation_rejected: 1,
                ..ClusterStagingCounters::default()
            },
        );

        assert_eq!(health.stale_generation_rejected, 1);
        assert_eq!(health.state, ClusterHealthState::Healthy);
    }

    #[test]
    fn staging_health_stopped_lifecycle_is_not_ready() {
        let mut lifecycle = ClusterLifecycleDiagnostics::running("cluster-runtime");
        lifecycle.record_shutdown_requested();
        lifecycle.record_graceful_stop();

        let health = ClusterStagingHealth::from_parts(
            staging_diagnostics(lifecycle),
            CacheStats::default(),
            ClusterFillCounters::default(),
            ClusterStagingCounters::default(),
        );

        assert_eq!(
            health.state,
            ClusterHealthState::NotReady {
                reasons: vec![ClusterHealthReason::LifecycleNotRunning]
            }
        );
    }

    #[test]
    fn recent_gossip_reset_downgrades_health_to_degraded() {
        let health = ClusterStagingHealth::from_parts(
            staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
            CacheStats::default(),
            ClusterFillCounters::default(),
            ClusterStagingCounters {
                tombstone_age_ms: 42,
                gossip_reset_count: 1,
                ..ClusterStagingCounters::default()
            },
        );

        assert_eq!(
            health.state,
            ClusterHealthState::Degraded {
                reasons: vec![ClusterHealthReason::GossipResetRecent {
                    tombstone_age_ms: 42,
                    reset_count: 1,
                }]
            }
        );
    }

    #[test]
    fn staging_health_local_role_returns_none() {
        let cache = HydraCache::local().build();

        assert!(cache.cluster_staging_health().is_none());
    }

    #[test]
    fn fill_owner_load_increments_only_owner_counter() {
        let cache = HydraCache::local().build();

        cache.record_cluster_owner_load_success();

        assert_eq!(
            cache.cluster_fill_counters(),
            ClusterFillCounters {
                owner_load_success: 1,
                ..ClusterFillCounters::default()
            }
        );
    }

    #[test]
    fn fill_remote_fetch_increments_only_remote_counter() {
        let cache = HydraCache::local().build();

        cache.record_cluster_remote_fetch_success();

        assert_eq!(
            cache.cluster_fill_counters(),
            ClusterFillCounters {
                remote_fetch_success: 1,
                ..ClusterFillCounters::default()
            }
        );
    }

    #[test]
    fn fill_hot_cache_hit_increments_only_hot_counter() {
        let cache = HydraCache::local().build();

        cache.record_cluster_hot_cache_hit();

        assert_eq!(
            cache.cluster_fill_counters(),
            ClusterFillCounters {
                hot_cache_hits: 1,
                ..ClusterFillCounters::default()
            }
        );
    }

    #[test]
    fn fill_counters_are_mutually_exclusive_per_event() {
        let owner = HydraCache::local().build();
        let remote = HydraCache::local().build();
        let hot = HydraCache::local().build();

        owner.record_cluster_owner_load_success();
        remote.record_cluster_remote_fetch_success();
        hot.record_cluster_hot_cache_hit();

        assert_eq!(owner.cluster_fill_counters().successful_events(), 1);
        assert_eq!(remote.cluster_fill_counters().successful_events(), 1);
        assert_eq!(hot.cluster_fill_counters().successful_events(), 1);
    }

    #[test]
    fn load_report_totals_health_and_json_shape_are_stable() {
        let report = ClusterLoadReport {
            nodes: 4,
            requests: 240,
            read_ops: 228,
            invalidation_ops: 12,
            published: 12,
            received: 24,
            applied: 24,
            lagged: 0,
            decode_errors: 0,
            publish_failures: 0,
            receiver_closed: 0,
            stale_generation_rejected: 1,
            peer_fetch_auth_failures: 1,
            wire_version_rejections: 1,
            owner_load_success: 5,
            remote_fetch_success: 3,
            hot_cache_hits: 7,
            elapsed_ms: 320,
        };

        assert!(report.totals_match_requests());
        assert!(report.has_clean_invalidation_health());

        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["nodes"], 4);
        assert_eq!(value["requests"], 240);
        assert_eq!(value["published"], 12);
        assert_eq!(value["stale_generation_rejected"], 1);
        assert_eq!(value["peer_fetch_auth_failures"], 1);
        assert_eq!(value["wire_version_rejections"], 1);
        assert_eq!(value["elapsed_ms"], 320);
    }

    fn staging_diagnostics(lifecycle: ClusterLifecycleDiagnostics) -> ClusterDiagnostics {
        ClusterDiagnostics {
            cluster_name: "orders".to_owned(),
            role: ClusterRole::Member,
            node_id: ClusterNodeId::from("member-a"),
            generation: ClusterGeneration::new(3),
            epoch: ClusterEpoch::new(7),
            member_count: 1,
            client_count: 1,
            bootstrap: vec!["127.0.0.1:7000".to_owned()],
            connected: true,
            invalidation_subscribers: 1,
            membership_subscribers: 1,
            lifecycle,
        }
    }

    #[test]
    fn role_marks_only_members_as_future_voters() {
        assert!(!ClusterRole::Local.can_vote());
        assert!(!ClusterRole::Client.can_vote());
        assert!(ClusterRole::Member.can_vote());
    }

    #[test]
    fn endpoints_builder_sets_advertised_addresses() {
        let endpoints = ClusterEndpoints::new()
            .control("127.0.0.1:7000")
            .invalidation("127.0.0.1:7001")
            .diagnostics("http://127.0.0.1:3000");

        assert_eq!(endpoints.control.as_deref(), Some("127.0.0.1:7000"));
        assert_eq!(endpoints.invalidation.as_deref(), Some("127.0.0.1:7001"));
        assert_eq!(
            endpoints.diagnostics.as_deref(),
            Some("http://127.0.0.1:3000")
        );
    }

    #[test]
    fn candidate_carries_generation_endpoints_and_metadata() {
        let candidate = ClusterCandidate::member("member-a")
            .generation(ClusterGeneration::new(3))
            .endpoints(ClusterEndpoints::new().control("127.0.0.1:7000"))
            .metadata("version", "0.20.0");

        assert_eq!(candidate.node_id.as_str(), "member-a");
        assert_eq!(candidate.role, ClusterRole::Member);
        assert_eq!(candidate.generation.value(), 3);
        assert_eq!(
            candidate.endpoints.control.as_deref(),
            Some("127.0.0.1:7000")
        );
        assert_eq!(
            candidate.metadata.get("version").map(String::as_str),
            Some("0.20.0")
        );
    }

    #[test]
    fn peer_fetch_endpoint_metadata_is_carried_to_member() {
        let candidate =
            ClusterCandidate::member("member-a").peer_fetch_base_url("http://127.0.0.1:3000");

        assert_eq!(
            candidate.peer_fetch_base_url_value(),
            Some("http://127.0.0.1:3000")
        );
        assert_eq!(
            candidate
                .metadata
                .get(CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY)
                .map(String::as_str),
            Some("http://127.0.0.1:3000")
        );

        let member = ClusterMember::from_candidate(candidate, ClusterEpoch::new(1));

        assert_eq!(member.peer_fetch_base_url(), Some("http://127.0.0.1:3000"));
    }

    #[test]
    fn rendezvous_ownership_resolver_selects_stable_member_owner() {
        let resolver = RendezvousClusterOwnership;
        let first = ClusterMember::from_candidate(
            ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
            ClusterEpoch::new(1),
        );
        let second = ClusterMember::from_candidate(
            ClusterCandidate::member("member-b").generation(ClusterGeneration::new(1)),
            ClusterEpoch::new(1),
        );
        let client = ClusterMember::from_candidate(
            ClusterCandidate::client("client-a").generation(ClusterGeneration::new(1)),
            ClusterEpoch::new(1),
        );
        let participants = vec![first.clone(), second.clone(), client];
        let reversed = vec![second, first];

        let decision = resolver.resolve_owner("user:42", &participants);
        let reversed_decision = resolver.resolve_owner("user:42", &reversed);

        assert_eq!(decision.resolver, "rendezvous");
        assert_eq!(decision.key, "user:42");
        assert_eq!(decision.member_count, 2);
        assert!(decision.has_owner());
        assert_eq!(decision.owner_node_id(), reversed_decision.owner_node_id());
        assert_eq!(decision.owner_generation(), Some(ClusterGeneration::new(1)));
    }

    #[test]
    fn rendezvous_ownership_resolver_reports_no_owner_without_members() {
        let resolver = RendezvousClusterOwnership;
        let participants = vec![ClusterMember::from_candidate(
            ClusterCandidate::client("client-a"),
            ClusterEpoch::default(),
        )];

        let decision = resolver.resolve_owner("user:42", &participants);

        assert_eq!(decision.member_count, 0);
        assert!(!decision.has_owner());
        assert!(decision.owner_node_id().is_none());
        assert!(decision.owner_generation().is_none());
        assert!(decision.peer_fetch_request().is_none());
    }

    #[tokio::test]
    async fn ownership_decision_builds_peer_fetch_request_for_owner() {
        let member = ClusterMember::from_candidate(
            ClusterCandidate::member("member-a").generation(ClusterGeneration::new(3)),
            ClusterEpoch::new(1),
        );
        let decision = ClusterOwnershipDecision {
            key: "user:42".to_owned(),
            owner: Some(member),
            member_count: 1,
            resolver: "test",
        };

        let request = decision.peer_fetch_request().expect("owner exists");

        assert_eq!(request.owner.as_str(), "member-a");
        assert_eq!(request.key, "user:42");
        assert_eq!(request.generation, Some(ClusterGeneration::new(3)));
    }

    #[test]
    fn ownership_decision_without_owner_cannot_build_peer_fetch_request() {
        let decision = ClusterOwnershipDecision {
            key: "user:42".to_owned(),
            owner: None,
            member_count: 0,
            resolver: "test",
        };

        assert!(!decision.has_owner());
        assert!(decision.peer_fetch_request().is_none());
    }

    #[test]
    fn peer_fetch_request_reports_generation_mismatch() {
        let request = ClusterPeerFetchRequest::new("member-a", "user:42")
            .generation(ClusterGeneration::new(3));

        assert!(request.has_generation());
        assert!(request.matches_generation(ClusterGeneration::new(3)));
        assert_eq!(
            request.generation_mismatch(ClusterGeneration::new(4)),
            Some(ClusterPeerFetchGenerationMismatch {
                requested: ClusterGeneration::new(3),
                current: ClusterGeneration::new(4),
            })
        );
        assert!(!request.matches_generation(ClusterGeneration::new(4)));

        let generationless = ClusterPeerFetchRequest::new("member-a", "user:42");
        assert!(!generationless.has_generation());
        assert!(generationless.matches_generation(ClusterGeneration::new(99)));
        assert!(generationless
            .generation_mismatch(ClusterGeneration::new(99))
            .is_none());
    }

    #[tokio::test]
    async fn in_memory_peer_fetch_returns_hits_misses_and_removes_values() {
        let fetch = InMemoryPeerFetch::new();
        let owner = ClusterNodeId::from("member-a");

        assert!(fetch.is_empty());
        fetch.put(owner.clone(), "user:42", Bytes::from_static(b"encoded"));
        assert_eq!(fetch.len(), 1);

        let hit = fetch
            .fetch(ClusterPeerFetchRequest::new(owner.clone(), "user:42"))
            .await
            .unwrap();
        assert_eq!(
            hit,
            ClusterPeerFetchResponse::hit(owner.clone(), "user:42", Bytes::from_static(b"encoded"))
        );
        assert!(hit.is_hit());
        assert!(!hit.is_miss());

        let missing = fetch
            .fetch(ClusterPeerFetchRequest::new(owner.clone(), "user:99"))
            .await
            .unwrap();
        assert!(missing.is_miss());
        assert_eq!(
            missing,
            ClusterPeerFetchResponse::miss(owner.clone(), "user:99")
        );

        assert_eq!(
            fetch.remove(&owner, "user:42"),
            Some(Bytes::from_static(b"encoded"))
        );
        assert!(fetch.is_empty());

        let removed = fetch
            .fetch(ClusterPeerFetchRequest::new(owner.clone(), "user:42"))
            .await
            .unwrap();
        assert!(removed.is_miss());
        assert_eq!(
            fetch.remove(&owner, "user:42"),
            None,
            "removing an already removed value is a no-op"
        );

        let diagnostics = fetch.diagnostics();
        assert_eq!(diagnostics.stored_values, 0);
        assert_eq!(diagnostics.hits, 1);
        assert_eq!(diagnostics.misses, 2);
        assert_eq!(diagnostics.total_requests(), 3);
        assert_eq!(diagnostics.hit_ratio(), Some(1.0 / 3.0));
    }

    #[test]
    fn discovery_events_keep_candidate_and_liveness_information() {
        let candidate = ClusterCandidate::client("client-a");

        assert_eq!(
            ClusterDiscoveryEvent::CandidateSeen(candidate.clone()),
            ClusterDiscoveryEvent::CandidateSeen(candidate)
        );
        assert_eq!(
            ClusterDiscoveryEvent::MemberLive(ClusterNodeId::from("member-a")),
            ClusterDiscoveryEvent::MemberLive(ClusterNodeId::from("member-a"))
        );
        assert_ne!(
            ClusterDiscoveryEvent::MemberSuspect(ClusterNodeId::from("member-a")),
            ClusterDiscoveryEvent::MemberDead(ClusterNodeId::from("member-a"))
        );
    }

    #[test]
    fn discovery_diagnostics_helpers_report_candidate_and_event_counts() {
        let diagnostics = ClusterDiscoveryDiagnostics {
            local_node_id: ClusterNodeId::from("client-a"),
            candidates: vec![ClusterCandidate::client("client-a")],
            events: vec![ClusterDiscoveryEvent::MemberLive(ClusterNodeId::from(
                "client-a",
            ))],
        };

        assert_eq!(diagnostics.candidate_count(), 1);
        assert_eq!(diagnostics.event_count(), 1);
        assert!(diagnostics.has_candidates());
        assert!(diagnostics.has_events());
    }

    #[test]
    fn admission_bridge_diagnostics_record_events_without_double_counting_failures() {
        let candidate = ClusterCandidate::member("member-a").generation(ClusterGeneration::new(3));
        let admitted = ClusterMember::from_candidate(candidate.clone(), Default::default());
        let mut diagnostics = ClusterAdmissionBridgeDiagnostics::default();

        diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateSeen(
            candidate.clone(),
        ));
        diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateIgnored {
            candidate: candidate.clone(),
            reason: ClusterAdmissionIgnoreReason::AlreadyCurrent,
        });
        diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateAdmitted(admitted));
        diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateRejected {
            candidate: candidate.clone(),
            reason: ClusterAdmissionRejectReason::AdmissionError("raft unavailable".to_owned()),
        });
        diagnostics.record_event(&ClusterAdmissionBridgeEvent::BridgeStopped);

        assert_eq!(diagnostics.candidates_seen, 1);
        assert_eq!(diagnostics.candidates_ignored, 1);
        assert_eq!(diagnostics.candidates_admitted, 1);
        assert_eq!(diagnostics.candidates_rejected, 1);
        assert_eq!(diagnostics.admission_failures, 1);
        assert_eq!(diagnostics.total_decisions(), 3);
        assert!(diagnostics.has_seen_candidates());
        assert!(diagnostics.has_admissions());
        assert!(diagnostics.has_issues());
        assert_eq!(diagnostics.last_candidate, Some(candidate.node_id.clone()));
        assert_eq!(diagnostics.last_admitted, Some(candidate.node_id));
        assert_eq!(diagnostics.last_error.as_deref(), Some("raft unavailable"));
    }

    #[tokio::test]
    async fn admission_bridge_run_once_admits_candidates_and_deduplicates_generation() {
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let control_plane = Arc::new(InMemoryCluster::new("orders"));
        let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

        discovery
            .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)));

        assert_eq!(bridge.run_once().await, 1);
        assert_eq!(control_plane.members().len(), 1);
        assert_eq!(control_plane.events().len(), 1);

        assert_eq!(bridge.run_once().await, 1);
        assert_eq!(control_plane.events().len(), 1);

        let diagnostics = bridge.diagnostics();
        assert_eq!(diagnostics.candidates_seen, 2);
        assert_eq!(diagnostics.candidates_admitted, 1);
        assert_eq!(diagnostics.candidates_ignored, 1);
        assert_eq!(diagnostics.total_decisions(), 2);
        assert!(matches!(
            bridge.events().last(),
            Some(ClusterAdmissionBridgeEvent::CandidateIgnored {
                reason: ClusterAdmissionIgnoreReason::AlreadyCurrent,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn admission_bridge_allows_role_transition_for_same_generation() {
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let control_plane = Arc::new(InMemoryCluster::new("orders"));
        let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

        discovery
            .announce(ClusterCandidate::client("node-a").generation(ClusterGeneration::new(1)));
        assert_eq!(bridge.run_once().await, 1);
        assert_eq!(control_plane.clients().len(), 1);

        discovery
            .announce(ClusterCandidate::member("node-a").generation(ClusterGeneration::new(1)));
        assert_eq!(bridge.run_once().await, 1);

        assert_eq!(control_plane.clients().len(), 0);
        assert_eq!(control_plane.members().len(), 1);
        assert_eq!(control_plane.events().len(), 2);
        assert_eq!(bridge.diagnostics().candidates_admitted, 2);
    }

    #[tokio::test]
    async fn admission_bridge_rejects_stale_candidate_before_control_plane_write() {
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let control_plane = Arc::new(InMemoryCluster::new("orders"));
        let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

        discovery
            .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)));
        assert_eq!(bridge.run_once().await, 1);

        discovery
            .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)));
        assert_eq!(bridge.run_once().await, 1);

        assert_eq!(control_plane.members()[0].generation.value(), 2);
        assert_eq!(control_plane.events().len(), 1);
        assert!(matches!(
            bridge.events().last(),
            Some(ClusterAdmissionBridgeEvent::CandidateRejected {
                reason: ClusterAdmissionRejectReason::StaleGeneration { existing, attempted },
                ..
            }) if existing.value() == 2 && attempted.value() == 1
        ));
    }

    #[tokio::test]
    async fn admission_bridge_respects_role_filters_and_ignores_local_candidates() {
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let control_plane = Arc::new(InMemoryCluster::new("orders"));
        let bridge = ClusterAdmissionBridge::with_config(
            discovery.clone(),
            control_plane.clone(),
            ClusterAdmissionBridgeConfig::default().admit_clients(false),
        );
        let mut local_candidate = ClusterCandidate::client("local-a");
        local_candidate.role = ClusterRole::Local;

        discovery.announce(ClusterCandidate::client("client-a"));
        discovery.announce(local_candidate);

        assert_eq!(bridge.run_once().await, 2);
        assert!(control_plane.clients().is_empty());
        assert!(control_plane.members().is_empty());

        let diagnostics = bridge.diagnostics();
        assert_eq!(diagnostics.candidates_seen, 2);
        assert_eq!(diagnostics.candidates_ignored, 2);
        assert!(bridge.events().iter().any(|event| matches!(
            event,
            ClusterAdmissionBridgeEvent::CandidateIgnored {
                reason: ClusterAdmissionIgnoreReason::RoleDisabled,
                ..
            }
        )));
        assert!(bridge.events().iter().any(|event| matches!(
            event,
            ClusterAdmissionBridgeEvent::CandidateIgnored {
                reason: ClusterAdmissionIgnoreReason::LocalRole,
                ..
            }
        )));
    }

    #[test]
    fn admission_bridge_config_normalizes_zero_interval_and_member_filter() {
        let config = ClusterAdmissionBridgeConfig::default()
            .poll_interval(Duration::ZERO)
            .admit_members(false);

        assert_eq!(config.normalized_poll_interval(), Duration::from_millis(1));
        assert!(!config.admit_members);
        assert!(config.admit_clients);
    }

    #[tokio::test]
    async fn admission_bridge_background_loop_can_shutdown_gracefully() {
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let control_plane = Arc::new(InMemoryCluster::new("orders"));
        let bridge = ClusterAdmissionBridge::with_config(
            discovery.clone(),
            control_plane.clone(),
            ClusterAdmissionBridgeConfig::default().poll_interval(Duration::from_millis(1)),
        );

        discovery.announce(ClusterCandidate::member("member-a"));
        assert_eq!(
            bridge.lifecycle_diagnostics().status,
            ClusterLifecycleStatus::Idle
        );
        let handle = bridge.start();
        assert!(bridge.lifecycle_diagnostics().is_running());
        assert_eq!(bridge.lifecycle_diagnostics().start_count, 1);

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if control_plane.members().len() == 1 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("background bridge should admit the candidate");

        handle.shutdown().await;

        assert_eq!(control_plane.members().len(), 1);
        let lifecycle = bridge.lifecycle_diagnostics();
        assert!(lifecycle.is_stopped());
        assert_eq!(lifecycle.stop_count, 1);
        assert!(lifecycle.shutdown_requested);
        assert!(matches!(
            bridge.events().last(),
            Some(ClusterAdmissionBridgeEvent::BridgeStopped)
        ));
    }

    #[tokio::test]
    async fn admission_bridge_handle_drop_requests_shutdown_and_records_stop() {
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let control_plane = Arc::new(InMemoryCluster::new("orders"));
        let bridge = ClusterAdmissionBridge::with_config(
            discovery,
            control_plane,
            ClusterAdmissionBridgeConfig::default().poll_interval(Duration::from_millis(1)),
        );

        let handle = bridge.start();
        assert!(bridge.lifecycle_diagnostics().is_running());

        drop(handle);
        let lifecycle = bridge.lifecycle_diagnostics();
        assert!(lifecycle.is_stopping() || lifecycle.is_stopped());
        assert!(lifecycle.shutdown_requested);

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if bridge.lifecycle_diagnostics().is_stopped() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("dropped bridge handle should stop the task");

        assert_eq!(bridge.lifecycle_diagnostics().stop_count, 1);
    }

    #[test]
    fn in_memory_discovery_records_candidates_and_liveness_events() {
        let discovery = InMemoryClusterDiscovery::new();
        let first = ClusterCandidate::member("member-a")
            .generation(ClusterGeneration::new(1))
            .metadata("zone", "eu");
        let second = ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2));

        discovery.announce(first);
        discovery.announce(second);
        discovery.mark_live("member-a");
        discovery.mark_suspect("member-a");
        discovery.mark_dead("member-a");

        let candidates = discovery.candidates();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].generation.value(), 2);
        assert_eq!(discovery.events().len(), 5);
        assert!(matches!(
            discovery.events().last(),
            Some(ClusterDiscoveryEvent::MemberDead(node_id)) if node_id.as_str() == "member-a"
        ));
    }

    #[tokio::test]
    async fn in_memory_discovery_satisfies_discovery_contract() {
        let discovery: Arc<dyn ClusterDiscovery> = Arc::new(InMemoryClusterDiscovery::new());

        discovery
            .announce(ClusterCandidate::client("client-a"))
            .await
            .unwrap();
        discovery
            .mark_live(ClusterNodeId::from("client-a"))
            .await
            .unwrap();
        discovery
            .mark_suspect(ClusterNodeId::from("client-a"))
            .await
            .unwrap();
        discovery
            .mark_dead(ClusterNodeId::from("client-a"))
            .await
            .unwrap();

        assert_eq!(discovery.candidates().len(), 1);
        assert_eq!(discovery.events().len(), 4);
        assert!(matches!(
            discovery.events().last(),
            Some(ClusterDiscoveryEvent::MemberDead(node_id)) if node_id.as_str() == "client-a"
        ));
    }

    #[tokio::test]
    async fn chitchat_style_discovery_satisfies_trait_and_seed_metadata_paths() {
        let empty = ChitchatStyleDiscovery::default();
        assert_eq!(empty.seed_count(), 0);
        assert!(!empty.has_seeds());
        assert_eq!(empty.adapter_name(), "chitchat-style");

        let discovery = ChitchatStyleDiscovery::new(["127.0.0.1:7000", "127.0.0.1:7001"]);
        assert_eq!(discovery.seed_count(), 2);
        assert!(discovery.has_seeds());
        assert_eq!(discovery.seeds()[0], "127.0.0.1:7000");

        let discovery: Arc<dyn ClusterDiscovery> = Arc::new(discovery);
        discovery
            .announce(ClusterCandidate::member("member-a"))
            .await
            .unwrap();
        discovery
            .mark_live(ClusterNodeId::from("member-a"))
            .await
            .unwrap();
        discovery
            .mark_suspect(ClusterNodeId::from("member-a"))
            .await
            .unwrap();
        discovery
            .mark_dead(ClusterNodeId::from("member-a"))
            .await
            .unwrap();

        let candidates = discovery.candidates();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0]
                .metadata
                .get("discovery.adapter")
                .map(String::as_str),
            Some("chitchat-style")
        );
        assert_eq!(
            candidates[0]
                .metadata
                .get("discovery.seeds")
                .map(String::as_str),
            Some("127.0.0.1:7000,127.0.0.1:7001")
        );
        assert_eq!(discovery.events().len(), 4);
    }

    #[tokio::test]
    async fn closed_membership_subscriber_and_display_errors_are_observable() {
        assert_eq!(
            ClusterMembershipRecvError::Closed.to_string(),
            "cluster membership subscription closed"
        );
        assert_eq!(
            ClusterMembershipRecvError::Lagged(3).to_string(),
            "cluster membership subscriber lagged by 3 events"
        );

        let mut subscriber = super::ClusterMembershipSubscriber::closed();
        assert_eq!(
            subscriber.recv().await.unwrap_err(),
            ClusterMembershipRecvError::Closed
        );
        assert_eq!(subscriber.next_event().await, None);
    }

    #[test]
    fn in_memory_cluster_admits_members_and_clients() {
        let cluster = InMemoryCluster::new("orders");

        let member = cluster
            .join_member(ClusterCandidate::member("member-a"))
            .unwrap();
        let client = cluster
            .join_client(ClusterCandidate::client("client-a"))
            .unwrap();

        assert_eq!(cluster.name(), "orders");
        assert!(member.is_member());
        assert!(client.is_client());
        assert_eq!(cluster.epoch().value(), 1);
        assert_eq!(cluster.members().len(), 1);
        assert_eq!(cluster.clients().len(), 1);
        assert_eq!(cluster.events().len(), 2);
    }

    #[tokio::test]
    async fn membership_subscriber_receives_join_leave_and_stale_rejection_events() {
        let cluster = InMemoryCluster::new("orders");
        let mut events = cluster.subscribe_membership();
        let member_id = ClusterNodeId::from("member-a");

        cluster
            .join_member(
                ClusterCandidate::member(member_id.clone()).generation(ClusterGeneration::new(2)),
            )
            .unwrap();
        assert!(matches!(
            events.recv().await.unwrap(),
            ClusterMembershipEvent::MemberJoined(member) if member.node_id == member_id
        ));

        let error = cluster
            .join_member(
                ClusterCandidate::member(member_id.clone()).generation(ClusterGeneration::new(1)),
            )
            .unwrap_err();
        assert!(error.to_string().contains("stale cluster generation"));
        assert!(matches!(
            events.recv().await.unwrap(),
            ClusterMembershipEvent::StaleGenerationRejected {
                node_id,
                role: ClusterRole::Member,
                existing,
                attempted,
                reason,
            } if node_id == member_id
                && existing.value() == 2
                && attempted.value() == 1
                && reason == "stale-generation"
        ));

        cluster
            .leave(&member_id, ClusterGeneration::new(2))
            .unwrap()
            .unwrap();
        assert!(matches!(
            events.recv().await.unwrap(),
            ClusterMembershipEvent::NodeLeft {
                node_id,
                role: ClusterRole::Member,
                ..
            } if node_id == member_id
        ));
    }

    #[tokio::test]
    async fn membership_subscriber_reports_lag_for_slow_consumers() {
        let bus = ClusterMembershipEventBus::new(1);
        let mut events = bus.subscribe();
        let first = ClusterMember::from_candidate(
            ClusterCandidate::member("member-a"),
            ClusterEpoch::new(1),
        );
        let second = ClusterMember::from_candidate(
            ClusterCandidate::member("member-b"),
            ClusterEpoch::new(2),
        );

        bus.publish(ClusterMembershipEvent::MemberJoined(first));
        bus.publish(ClusterMembershipEvent::MemberJoined(second));

        assert!(matches!(
            events.recv().await,
            Err(ClusterMembershipRecvError::Lagged(1))
        ));
        assert!(matches!(
            events.recv().await.unwrap(),
            ClusterMembershipEvent::MemberJoined(member) if member.node_id.as_str() == "member-b"
        ));
    }

    #[test]
    fn in_memory_cluster_rejects_stale_generation() {
        let cluster = InMemoryCluster::new("orders");
        cluster
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)))
            .unwrap();

        let error = cluster
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .unwrap_err();

        assert!(error.to_string().contains("stale cluster generation"));
        assert!(matches!(
            cluster.events().last(),
            Some(ClusterMembershipEvent::StaleGenerationRejected { .. })
        ));
    }

    #[test]
    fn in_memory_cluster_allows_generation_upgrade_and_advances_epoch() {
        let cluster = InMemoryCluster::new("orders");
        cluster
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .unwrap();
        cluster
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)))
            .unwrap();

        assert_eq!(cluster.epoch().value(), 2);
        assert_eq!(cluster.members()[0].generation.value(), 2);
    }

    #[test]
    fn client_to_member_promotion_moves_node_between_role_sets() {
        let cluster = InMemoryCluster::new("orders");
        cluster
            .join_client(ClusterCandidate::client("node-a"))
            .unwrap();
        cluster
            .join_member(ClusterCandidate::member("node-a"))
            .unwrap();

        assert_eq!(cluster.clients().len(), 0);
        assert_eq!(cluster.members().len(), 1);
        assert_eq!(cluster.members()[0].role, ClusterRole::Member);
    }

    #[test]
    fn leave_removes_clients_without_advancing_epoch_and_members_with_epoch() {
        let cluster = InMemoryCluster::new("orders");
        let member_id = ClusterNodeId::from("member-a");
        let client_id = ClusterNodeId::from("client-a");
        cluster
            .join_member(ClusterCandidate::member(member_id.clone()))
            .unwrap();
        cluster
            .join_client(ClusterCandidate::client(client_id.clone()))
            .unwrap();

        let client_left = cluster
            .leave(&client_id, ClusterGeneration::default())
            .unwrap()
            .unwrap();
        assert_eq!(cluster.epoch().value(), 1);
        assert!(matches!(
            client_left,
            ClusterMembershipEvent::NodeLeft {
                role: ClusterRole::Client,
                ..
            }
        ));

        let member_left = cluster
            .leave(&member_id, ClusterGeneration::default())
            .unwrap()
            .unwrap();
        assert_eq!(cluster.epoch().value(), 2);
        assert!(matches!(
            member_left,
            ClusterMembershipEvent::NodeLeft {
                role: ClusterRole::Member,
                ..
            }
        ));
        assert!(cluster
            .leave(&member_id, ClusterGeneration::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn leave_rejects_stale_generation_without_removing_newer_node() {
        let cluster = InMemoryCluster::new("orders");
        let node_id = ClusterNodeId::from("member-a");

        cluster
            .join_member(
                ClusterCandidate::member(node_id.clone()).generation(ClusterGeneration::new(1)),
            )
            .unwrap();
        cluster
            .join_member(
                ClusterCandidate::member(node_id.clone()).generation(ClusterGeneration::new(2)),
            )
            .unwrap();

        let error = cluster
            .leave(&node_id, ClusterGeneration::new(1))
            .unwrap_err();

        assert!(error.to_string().contains("stale cluster generation"));
        assert_eq!(cluster.members().len(), 1);
        assert_eq!(cluster.members()[0].generation.value(), 2);
        assert!(matches!(
            cluster.events().last(),
            Some(ClusterMembershipEvent::StaleGenerationRejected { .. })
        ));
    }

    #[test]
    fn diagnostics_report_counts_bootstrap_and_subscribers() {
        let cluster = Arc::new(InMemoryCluster::new("orders"));
        cluster
            .join_member(ClusterCandidate::member("member-a"))
            .unwrap();
        let _subscriber = cluster.invalidation_bus().subscribe();

        let diagnostics = cluster.diagnostics_for(
            ClusterRole::Member,
            ClusterNodeId::from("member-a"),
            ClusterGeneration::default(),
            vec!["seed-a:7000".to_owned()],
        );

        assert_eq!(diagnostics.cluster_name, "orders");
        assert_eq!(diagnostics.role, ClusterRole::Member);
        assert_eq!(diagnostics.node_id.as_str(), "member-a");
        assert_eq!(diagnostics.member_count, 1);
        assert_eq!(diagnostics.client_count, 0);
        assert_eq!(diagnostics.bootstrap, ["seed-a:7000".to_owned()]);
        assert!(diagnostics.connected);
        assert_eq!(diagnostics.invalidation_subscribers, 1);
        assert!(diagnostics.is_member_role());
        assert!(!diagnostics.is_client_role());
        assert!(!diagnostics.is_local_role());
        assert_eq!(diagnostics.participant_count(), 1);
        assert_eq!(diagnostics.bootstrap_count(), 1);
        assert!(diagnostics.has_members());
        assert!(!diagnostics.has_clients());
        assert!(diagnostics.has_bootstrap());
        assert!(diagnostics.has_invalidation_subscribers());
        assert!(!diagnostics.has_membership_subscribers());
        assert!(!diagnostics.has_multiple_participants());
        assert!(diagnostics.is_operational());

        let ownership = cluster.ownership_diagnostics();
        assert_eq!(ownership.resolver, "rendezvous");
        assert_eq!(ownership.resolutions, 0);
        assert_eq!(ownership.no_owner, 0);
        assert_eq!(ownership.owner_found(), 0);
        assert!(!ownership.has_resolutions());
        assert_eq!(ownership.owner_found_ratio(), None);
    }

    #[test]
    fn in_memory_cluster_resolves_key_owner_from_admitted_members() {
        let cluster = InMemoryCluster::new("orders");

        let empty = cluster.owner_for_key("user:42");
        assert!(!empty.has_owner());
        assert_eq!(empty.member_count, 0);

        cluster
            .join_member(ClusterCandidate::member("member-a"))
            .unwrap();
        cluster
            .join_member(ClusterCandidate::member("member-b"))
            .unwrap();
        cluster
            .join_client(ClusterCandidate::client("client-a"))
            .unwrap();

        let first = cluster.owner_for_key("user:42");
        let second = cluster.owner_for_key("user:42");
        let different_key = cluster.owner_for_key("user:99");

        assert_eq!(first.resolver, "rendezvous");
        assert_eq!(first.member_count, 2);
        assert!(first.has_owner());
        assert_eq!(first.owner_node_id(), second.owner_node_id());
        assert!(different_key.has_owner());
        assert!(["member-a", "member-b"]
            .contains(&different_key.owner_node_id().expect("owner").as_str()));

        let diagnostics = cluster.ownership_diagnostics();
        assert_eq!(diagnostics.resolutions, 4);
        assert_eq!(diagnostics.no_owner, 1);
        assert_eq!(diagnostics.owner_found(), 3);
        assert_eq!(diagnostics.owner_found_ratio(), Some(0.75));
    }

    #[test]
    fn ownership_ignores_client_join_and_leave() {
        let cluster = InMemoryCluster::new("orders");
        cluster
            .join_member(ClusterCandidate::member("member-a"))
            .unwrap();
        cluster
            .join_member(ClusterCandidate::member("member-b"))
            .unwrap();

        let before_clients = cluster.owner_for_key("user:42");
        cluster
            .join_client(ClusterCandidate::client("client-a"))
            .unwrap();
        cluster
            .join_client(ClusterCandidate::client("client-b"))
            .unwrap();
        let after_client_join = cluster.owner_for_key("user:42");

        cluster
            .leave(
                &ClusterNodeId::from("client-a"),
                ClusterGeneration::default(),
            )
            .unwrap();
        let after_client_leave = cluster.owner_for_key("user:42");

        assert_eq!(before_clients.member_count, 2);
        assert_eq!(
            before_clients.owner_node_id(),
            after_client_join.owner_node_id()
        );
        assert_eq!(
            before_clients.owner_node_id(),
            after_client_leave.owner_node_id()
        );
        assert_eq!(cluster.clients().len(), 1);
    }

    #[test]
    fn ownership_moves_when_owner_member_leaves_and_returns_on_rejoin() {
        let cluster = InMemoryCluster::new("orders");
        cluster
            .join_member(ClusterCandidate::member("member-a"))
            .unwrap();
        cluster
            .join_member(ClusterCandidate::member("member-b"))
            .unwrap();

        let initial = cluster.owner_for_key("user:42");
        let initial_owner = initial.owner.clone().expect("initial owner");
        let initial_owner_id = initial_owner.node_id.clone();
        let survivor = cluster
            .members()
            .into_iter()
            .find(|member| member.node_id != initial_owner_id)
            .expect("surviving member");

        cluster
            .leave(&initial_owner.node_id, initial_owner.generation)
            .unwrap();
        let after_leave = cluster.owner_for_key("user:42");

        assert_eq!(after_leave.member_count, 1);
        assert_eq!(after_leave.owner_node_id(), Some(&survivor.node_id));

        let rejoined_generation = initial_owner.generation.next();
        cluster
            .join_member(
                ClusterCandidate::member(initial_owner_id.as_str()).generation(rejoined_generation),
            )
            .unwrap();
        let after_rejoin = cluster.owner_for_key("user:42");

        assert_eq!(after_rejoin.member_count, 2);
        assert_eq!(after_rejoin.owner_node_id(), Some(&initial_owner_id));
        assert_eq!(after_rejoin.owner_generation(), Some(rejoined_generation));
    }

    #[test]
    fn stale_member_candidate_does_not_replace_owner_generation() {
        let cluster = InMemoryCluster::new("orders");
        cluster
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)))
            .unwrap();

        let stale = cluster.join_member(
            ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
        );
        let owner = cluster.owner_for_key("user:42");

        assert!(stale
            .unwrap_err()
            .to_string()
            .contains("stale cluster generation"));
        assert_eq!(owner.member_count, 1);
        assert_eq!(
            owner.owner_node_id().map(ClusterNodeId::as_str),
            Some("member-a")
        );
        assert_eq!(owner.owner_generation(), Some(ClusterGeneration::new(2)));
        assert_eq!(cluster.members().len(), 1);
    }

    #[tokio::test]
    async fn in_memory_cluster_satisfies_control_plane_contract() {
        let control_plane: Arc<dyn ClusterControlPlane> = Arc::new(InMemoryCluster::new("orders"));

        let member = control_plane
            .join_member(ClusterCandidate::member("member-a"))
            .await
            .unwrap();
        let client = control_plane
            .join_client(ClusterCandidate::client("client-a"))
            .await
            .unwrap();

        assert_eq!(control_plane.name(), "orders");
        assert!(member.is_member());
        assert!(client.is_client());
        let _receiver = control_plane.invalidation_bus().subscribe();

        let diagnostics = control_plane.diagnostics_for(
            ClusterRole::Client,
            ClusterNodeId::from("client-a"),
            ClusterGeneration::default(),
            vec!["seed-a:7000".to_owned()],
        );
        assert_eq!(diagnostics.member_count, 1);
        assert_eq!(diagnostics.client_count, 1);
        assert_eq!(diagnostics.bootstrap, ["seed-a:7000".to_owned()]);

        let left = control_plane
            .leave(
                &ClusterNodeId::from("client-a"),
                ClusterGeneration::default(),
            )
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            left,
            ClusterMembershipEvent::NodeLeft {
                role: ClusterRole::Client,
                ..
            }
        ));
    }
}
