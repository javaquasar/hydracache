use super::*;

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
