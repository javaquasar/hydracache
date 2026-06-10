use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hydracache_core::{CacheCodec, CacheError, PostcardCodec, Result};

use crate::builder::HydraCacheBuilder;
use crate::cache::HydraCache;
use crate::invalidation_bus::{CacheInvalidationBus, InMemoryInvalidationBus};

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
    /// A member is suspected unhealthy.
    MemberSuspect(ClusterNodeId),
    /// A member is considered dead.
    MemberDead(ClusterNodeId),
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
        /// Existing accepted generation.
        existing: ClusterGeneration,
        /// Attempted stale generation.
        attempted: ClusterGeneration,
    },
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
    state: Mutex<InMemoryClusterState>,
}

impl InMemoryCluster {
    /// Create an in-memory cluster model.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            invalidation_bus: Arc::new(InMemoryInvalidationBus::default()),
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
        reject_stale_generation(&mut state, &candidate)?;

        match role {
            ClusterRole::Local => Err(CacheError::Backend(
                "local caches cannot join an in-memory cluster".to_owned(),
            )),
            ClusterRole::Client => {
                let member = ClusterMember::from_candidate(candidate, state.epoch);
                state.clients.insert(member.node_id.clone(), member.clone());
                state
                    .events
                    .push(ClusterMembershipEvent::ClientConnected(member.clone()));
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
                state
                    .events
                    .push(ClusterMembershipEvent::MemberJoined(member.clone()));
                Ok(member)
            }
        }
    }

    /// Remove a node from the in-memory cluster model.
    pub fn leave(&self, node_id: &ClusterNodeId) -> Option<ClusterMembershipEvent> {
        let mut state = self.state.lock().expect("cluster state poisoned");
        let removed_member = state.members.remove(node_id);
        let removed_client = state.clients.remove(node_id);
        let removed = removed_member.or(removed_client)?;
        if removed.role == ClusterRole::Member {
            state.epoch.advance();
        }
        let event = ClusterMembershipEvent::NodeLeft {
            node_id: removed.node_id,
            role: removed.role,
            epoch: state.epoch,
        };
        state.events.push(event.clone());
        Some(event)
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
        }
    }
}

fn reject_stale_generation(
    state: &mut InMemoryClusterState,
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

    state
        .events
        .push(ClusterMembershipEvent::StaleGenerationRejected {
            node_id: candidate.node_id.clone(),
            existing,
            attempted: candidate.generation,
        });
    Err(CacheError::Backend(format!(
        "stale cluster generation for node '{}': existing {}, attempted {}",
        candidate.node_id,
        existing.value(),
        candidate.generation.value()
    )))
}

#[derive(Debug, Clone)]
pub(crate) struct ClusterRuntime {
    cluster: Arc<InMemoryCluster>,
    role: ClusterRole,
    node_id: ClusterNodeId,
    generation: ClusterGeneration,
    bootstrap: Vec<String>,
}

impl ClusterRuntime {
    fn new(
        cluster: Arc<InMemoryCluster>,
        role: ClusterRole,
        node_id: ClusterNodeId,
        generation: ClusterGeneration,
        bootstrap: Vec<String>,
    ) -> Self {
        Self {
            cluster,
            role,
            node_id,
            generation,
            bootstrap,
        }
    }

    pub(crate) fn diagnostics(&self) -> ClusterDiagnostics {
        self.cluster.diagnostics_for(
            self.role,
            self.node_id.clone(),
            self.generation,
            self.bootstrap.clone(),
        )
    }
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
    cluster: Option<Arc<InMemoryCluster>>,
    discovery: Option<Arc<InMemoryClusterDiscovery>>,
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
            cluster: None,
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
        self.cluster = Some(cluster);
        self
    }

    /// Attach an in-process discovery journal.
    pub fn shared_discovery(mut self, discovery: Arc<InMemoryClusterDiscovery>) -> Self {
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
            cluster: self.cluster,
            discovery: self.discovery,
            node_id: self.node_id,
            generation: self.generation,
            endpoints: self.endpoints,
        }
    }

    /// Connect the client near-cache.
    pub async fn connect(self) -> Result<HydraCache<C>> {
        let cluster = self
            .cluster
            .unwrap_or_else(|| Arc::new(InMemoryCluster::new(self.cluster_name.clone())));
        let node_id = self.node_id.unwrap_or_else(next_client_id);
        let candidate = ClusterCandidate::client(node_id.clone())
            .generation(self.generation)
            .endpoints(self.endpoints);
        if let Some(discovery) = &self.discovery {
            discovery.announce(candidate.clone());
        }
        let admitted = cluster.join_client(candidate)?;

        Ok(self
            .cache_builder
            .shared_invalidation_bus(cluster.invalidation_bus())
            .invalidation_node_id(admitted.node_id.as_str())
            .cluster_runtime(ClusterRuntime::new(
                cluster,
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
    cluster: Option<Arc<InMemoryCluster>>,
    discovery: Option<Arc<InMemoryClusterDiscovery>>,
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
            cluster: None,
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
        self.cluster = Some(cluster);
        self
    }

    /// Attach an in-process discovery journal.
    pub fn shared_discovery(mut self, discovery: Arc<InMemoryClusterDiscovery>) -> Self {
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
            cluster: self.cluster,
            discovery: self.discovery,
            node_id: self.node_id,
            generation: self.generation,
            endpoints: self.endpoints,
        }
    }

    /// Start the member runtime.
    pub async fn start(self) -> Result<HydraCache<C>> {
        let cluster = self
            .cluster
            .unwrap_or_else(|| Arc::new(InMemoryCluster::new(self.cluster_name.clone())));
        let node_id = self.node_id.unwrap_or_else(next_member_id);
        let candidate = ClusterCandidate::member(node_id.clone())
            .generation(self.generation)
            .endpoints(self.endpoints);
        if let Some(discovery) = &self.discovery {
            discovery.announce(candidate.clone());
        }
        let admitted = cluster.join_member(candidate)?;

        Ok(self
            .cache_builder
            .shared_invalidation_bus(cluster.invalidation_bus())
            .invalidation_node_id(admitted.node_id.as_str())
            .cluster_runtime(ClusterRuntime::new(
                cluster,
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

    use super::{
        ClusterCandidate, ClusterDiscoveryEvent, ClusterEndpoints, ClusterGeneration,
        ClusterMembershipEvent, ClusterNodeId, ClusterRole, InMemoryCluster,
        InMemoryClusterDiscovery,
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

        let client_left = cluster.leave(&client_id).unwrap();
        assert_eq!(cluster.epoch().value(), 1);
        assert!(matches!(
            client_left,
            ClusterMembershipEvent::NodeLeft {
                role: ClusterRole::Client,
                ..
            }
        ));

        let member_left = cluster.leave(&member_id).unwrap();
        assert_eq!(cluster.epoch().value(), 2);
        assert!(matches!(
            member_left,
            ClusterMembershipEvent::NodeLeft {
                role: ClusterRole::Member,
                ..
            }
        ));
        assert!(cluster.leave(&member_id).is_none());
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
    }
}
