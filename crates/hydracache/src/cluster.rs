use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hydracache_core::{CacheCodec, CacheError, PostcardCodec, Result};

use crate::builder::HydraCacheBuilder;
use crate::cache::HydraCache;
use crate::invalidation_bus::{CacheInvalidationBus, InMemoryInvalidationBus};
use tokio::sync::broadcast;

static NEXT_CLUSTER_CLIENT_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_CLUSTER_MEMBER_ID: AtomicU64 = AtomicU64::new(1);

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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        self.connected && self.participant_count() > 0
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

#[derive(Debug, Default)]
struct ClusterAdmissionBridgeState {
    admitted: BTreeMap<ClusterNodeId, ClusterAdmissionSnapshot>,
    events: Vec<ClusterAdmissionBridgeEvent>,
    diagnostics: ClusterAdmissionBridgeDiagnostics,
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
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(bridge.config().normalized_poll_interval());
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            bridge.record_event(ClusterAdmissionBridgeEvent::BridgeStopped);
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        bridge.run_once().await;
                    }
                }
            }
        });

        ClusterAdmissionBridgeHandle { shutdown, task }
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
}

/// Handle for a background [`ClusterAdmissionBridge`] polling task.
#[must_use]
#[derive(Debug)]
pub struct ClusterAdmissionBridgeHandle {
    shutdown: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl ClusterAdmissionBridgeHandle {
    /// Ask the polling task to stop and wait until it exits.
    pub async fn shutdown(self) {
        let _ = self.shutdown.send(true);
        let _ = self.task.await;
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
}

#[derive(Debug, Default)]
struct InMemoryClusterState {
    epoch: ClusterEpoch,
    members: BTreeMap<ClusterNodeId, ClusterMember>,
    clients: BTreeMap<ClusterNodeId, ClusterMember>,
    events: Vec<ClusterMembershipEvent>,
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
        Self {
            control_plane,
            discovery,
            role,
            node_id,
            generation,
            bootstrap,
        }
    }

    pub(crate) fn diagnostics(&self) -> ClusterDiagnostics {
        self.control_plane.diagnostics_for(
            self.role,
            self.node_id.clone(),
            self.generation,
            self.bootstrap.clone(),
        )
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
        self.control_plane
            .leave(&self.node_id, self.generation)
            .await
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
        ClusterAdmissionBridge, ClusterAdmissionBridgeConfig, ClusterAdmissionBridgeDiagnostics,
        ClusterAdmissionBridgeEvent, ClusterAdmissionIgnoreReason, ClusterAdmissionRejectReason,
        ClusterCandidate, ClusterControlPlane, ClusterDiscovery, ClusterDiscoveryDiagnostics,
        ClusterDiscoveryEvent, ClusterEndpoints, ClusterEpoch, ClusterGeneration, ClusterMember,
        ClusterMembershipEvent, ClusterMembershipEventBus, ClusterMembershipRecvError,
        ClusterNodeId, ClusterRole, InMemoryCluster, InMemoryClusterDiscovery,
    };

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
        let handle = bridge.start();

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
        assert!(matches!(
            bridge.events().last(),
            Some(ClusterAdmissionBridgeEvent::BridgeStopped)
        ));
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
