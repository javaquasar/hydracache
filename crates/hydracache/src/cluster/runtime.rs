use super::*;

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

    /// Enable or disable the opt-in 0.41 value-replication prototype.
    pub fn replicate_values(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.replicate_values(enabled);
        self
    }

    /// Set the desired replication factor, including the primary copy.
    pub fn replication_factor(mut self, replication_factor: usize) -> Self {
        self.cache_builder = self.cache_builder.replication_factor(replication_factor);
        self
    }

    /// Set the read quorum.
    pub fn read_quorum(mut self, read_quorum: usize) -> Self {
        self.cache_builder = self.cache_builder.read_quorum(read_quorum);
        self
    }

    /// Set the write quorum.
    pub fn write_quorum(mut self, write_quorum: usize) -> Self {
        self.cache_builder = self.cache_builder.write_quorum(write_quorum);
        self
    }

    /// Set the number of synchronous backups.
    pub fn sync_backups(mut self, sync_backups: usize) -> Self {
        self.cache_builder = self.cache_builder.sync_backups(sync_backups);
        self
    }

    /// Set the number of asynchronous backups.
    pub fn async_backups(mut self, async_backups: usize) -> Self {
        self.cache_builder = self.cache_builder.async_backups(async_backups);
        self
    }

    /// Set the maximum encoded entry size accepted for replication.
    pub fn max_replicated_entry_bytes(mut self, max_replicated_entry_bytes: usize) -> Self {
        self.cache_builder = self
            .cache_builder
            .max_replicated_entry_bytes(max_replicated_entry_bytes);
        self
    }

    /// Explicitly acknowledge plaintext replicated values on this trust boundary.
    pub fn acknowledge_plaintext_replicated_values(mut self, acknowledged: bool) -> Self {
        self.cache_builder = self
            .cache_builder
            .acknowledge_plaintext_replicated_values(acknowledged);
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
        self.cache_builder.validate_replication_config()?;
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

    /// Enable or disable the opt-in 0.41 value-replication prototype.
    pub fn replicate_values(mut self, enabled: bool) -> Self {
        self.cache_builder = self.cache_builder.replicate_values(enabled);
        self
    }

    /// Set the desired replication factor, including the primary copy.
    pub fn replication_factor(mut self, replication_factor: usize) -> Self {
        self.cache_builder = self.cache_builder.replication_factor(replication_factor);
        self
    }

    /// Set the read quorum.
    pub fn read_quorum(mut self, read_quorum: usize) -> Self {
        self.cache_builder = self.cache_builder.read_quorum(read_quorum);
        self
    }

    /// Set the write quorum.
    pub fn write_quorum(mut self, write_quorum: usize) -> Self {
        self.cache_builder = self.cache_builder.write_quorum(write_quorum);
        self
    }

    /// Set the number of synchronous backups.
    pub fn sync_backups(mut self, sync_backups: usize) -> Self {
        self.cache_builder = self.cache_builder.sync_backups(sync_backups);
        self
    }

    /// Set the number of asynchronous backups.
    pub fn async_backups(mut self, async_backups: usize) -> Self {
        self.cache_builder = self.cache_builder.async_backups(async_backups);
        self
    }

    /// Set the maximum encoded entry size accepted for replication.
    pub fn max_replicated_entry_bytes(mut self, max_replicated_entry_bytes: usize) -> Self {
        self.cache_builder = self
            .cache_builder
            .max_replicated_entry_bytes(max_replicated_entry_bytes);
        self
    }

    /// Explicitly acknowledge plaintext replicated values on this trust boundary.
    pub fn acknowledge_plaintext_replicated_values(mut self, acknowledged: bool) -> Self {
        self.cache_builder = self
            .cache_builder
            .acknowledge_plaintext_replicated_values(acknowledged);
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
        self.cache_builder.validate_replication_config()?;
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
