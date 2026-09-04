use hydracache::{ClusterGridCounters, HydraCache, MemoryInstrumentationMode};
use hydracache_client_transport_axum::{ClientSurfaceDrain, ClientSurfaceRuntime};
use hydracache_observability::{
    ClusterMemberView, ClusterOverview, ClusterTopologyOverview, ConsistencyView,
    HydraCacheRegistry, LeaderView, LifecycleView, PartitionSummary, TopologyReshardPhase,
    TopologyStatusSource,
};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer, RedisServeError};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;

use crate::cluster_status::{
    ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime, GridControlPlaneHandle,
    LiveClusterStatus, MemberRole, ModeledClusterStatus, RaftCompactionError, RaftCompactionStatus,
    Reachability, ReshardPhase, StatusSource,
};
use crate::config::{ServerConfig, ServerConfigError, ServerRole};
use crate::redis_tcp::{RedisTlsAcceptor, RedisTlsError};
use crate::services::{DrainOutcome, GracefulShutdown, ServiceSet};

/// Runtime state exposed by health/readiness checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerState {
    /// Runtime has not started.
    Created,
    /// Runtime accepts requests.
    Running,
    /// Runtime is draining in-flight work.
    Draining,
    /// Runtime stopped cleanly.
    Stopped,
}

/// Liveness response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerHealth {
    /// Stable status field for probes.
    pub status: &'static str,
    /// Current runtime state.
    pub state: ServerState,
}

/// Readiness response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerReadiness {
    /// Whether the daemon can serve traffic.
    pub ready: bool,
    /// Whether durable storage has opened.
    pub storage_open: bool,
    /// Whether the configured cluster role is ready.
    pub cluster_ready: bool,
    /// Whether listeners are accepting new work.
    pub accepting: bool,
    /// Whether the external client surface is accepting work.
    pub client_surface_ready: bool,
    /// Whether the optional Redis RESP surface is accepting work.
    pub redis_surface_ready: bool,
}

/// Admin status consumed by the Kubernetes operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerAdminStatus {
    /// Whether this status is live or modeled.
    pub source: StatusSource,
    /// Current leader id if known to the runtime.
    pub leader: Option<String>,
    /// Current control-plane term if known.
    pub term: u64,
    /// Current committed membership authority epoch.
    pub epoch: u64,
    /// Whether the runtime believes quorum is available.
    pub quorum_ok: bool,
    /// Observed member count.
    pub members: u32,
    /// Sorted logical member ids in the committed metadata view.
    pub member_ids: Vec<String>,
    /// Observed raft voter count.
    pub voters: u32,
    /// Sorted numeric ids in the committed Raft voter set.
    pub voter_ids: Vec<u64>,
    /// Current reshard phase.
    pub reshard_phase: String,
    /// Whether the runtime is draining.
    pub draining: bool,
}

/// Accepted admin action response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerAdminAction {
    /// Stable action name.
    pub action: &'static str,
    /// Stable outcome string.
    pub outcome: &'static str,
    /// Human-readable detail, safe for operator Conditions.
    pub detail: String,
}

/// Modeled Redis RESP listener runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisSurfaceRuntime {
    accepting: bool,
    active_connections: u64,
    accepted_connections: u64,
    closed_connections: u64,
    rejected_connections: u64,
}

impl RedisSurfaceRuntime {
    fn new() -> Self {
        Self {
            accepting: false,
            active_connections: 0,
            accepted_connections: 0,
            closed_connections: 0,
            rejected_connections: 0,
        }
    }

    fn start(&mut self) {
        self.accepting = true;
    }

    fn accepting(&self) -> bool {
        self.accepting
    }

    fn begin_connection(&mut self) -> bool {
        if !self.accepting {
            self.rejected_connections = self.rejected_connections.saturating_add(1);
            return false;
        }
        self.active_connections = self.active_connections.saturating_add(1);
        self.accepted_connections = self.accepted_connections.saturating_add(1);
        true
    }

    fn finish_connection(&mut self) {
        if self.active_connections > 0 {
            self.closed_connections = self.closed_connections.saturating_add(1);
        }
        self.active_connections = self.active_connections.saturating_sub(1);
    }

    fn active_connections(&self) -> u64 {
        self.active_connections
    }

    fn shutdown(&mut self) -> RedisSurfaceDrain {
        self.accepting = false;
        let started_with = self.active_connections;
        self.closed_connections = self.closed_connections.saturating_add(started_with);
        self.active_connections = 0;
        RedisSurfaceDrain {
            started_with,
            remaining: self.active_connections,
        }
    }
}

/// Result of draining modeled Redis RESP connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedisSurfaceDrain {
    /// Active RESP connections observed when drain started.
    pub started_with: u64,
    /// Active RESP connections after drain.
    pub remaining: u64,
}

/// Additional read-only observability signals supplied by the daemon host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerObservabilityModel {
    cluster_grid: ClusterGridCounters,
    partition_count: u64,
    configured_default_consistency: Option<String>,
    backup_age_seconds: Option<u64>,
    upgrade_phase: String,
}

impl ServerObservabilityModel {
    /// Attach aggregate grid counters supplied by the hosting runtime.
    pub fn with_cluster_grid_counters(mut self, counters: ClusterGridCounters) -> Self {
        self.cluster_grid = counters;
        self
    }

    /// Attach the effective partition count.
    pub fn with_partition_count(mut self, count: u64) -> Self {
        self.partition_count = count;
        self
    }

    /// Attach the configured default consistency label.
    pub fn with_configured_default_consistency(mut self, level: impl Into<String>) -> Self {
        self.configured_default_consistency = Some(level.into());
        self
    }

    /// Attach the worst known backup age.
    pub fn with_backup_age_seconds(mut self, seconds: u64) -> Self {
        self.backup_age_seconds = Some(seconds);
        self
    }

    /// Attach backup ages for namespaces, keeping the oldest/worst age.
    pub fn with_backup_age_seconds_from_namespaces(
        mut self,
        ages: impl IntoIterator<Item = u64>,
    ) -> Self {
        self.backup_age_seconds = ages.into_iter().max();
        self
    }

    /// Attach the current graceful-upgrade phase.
    pub fn with_upgrade_phase(mut self, phase: impl Into<String>) -> Self {
        self.upgrade_phase = phase.into();
        self
    }
}

impl Default for ServerObservabilityModel {
    fn default() -> Self {
        Self {
            cluster_grid: ClusterGridCounters::default(),
            partition_count: 0,
            configured_default_consistency: None,
            backup_age_seconds: None,
            upgrade_phase: "idle".to_owned(),
        }
    }
}

/// Fail-loud admin action errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ServerAdminActionError {
    /// The runtime is not ready to accept the requested action.
    #[error("server is not ready for admin action: {0}")]
    NotReady(&'static str),
    /// The action requires member mode in the current server model.
    #[error("{0} requires member mode")]
    RequiresMember(&'static str),
    /// Backup cannot run without configured backup support.
    #[error("backup admin action requires backup.enabled and backup.location")]
    BackupDisabled,
    /// The live grid rejected a Raft compaction request.
    #[error(transparent)]
    RaftCompaction(#[from] RaftCompactionError),
}

#[derive(Debug, Clone)]
struct ManagementSnapshotOverride {
    local: crate::management_aggregation::ManagementMemberSnapshot,
    committed_targets: Vec<crate::management_aggregation::ManagementPeerTarget>,
    transport: Arc<dyn crate::management_aggregation::ManagementPeerTransport>,
}

/// Standalone server runtime.
#[derive(Debug, Clone)]
pub struct ServerRuntime {
    config: ServerConfig,
    cache: HydraCache,
    services: ServiceSet,
    state: ServerState,
    storage_open: bool,
    cluster_ready: bool,
    accepting: bool,
    flushed: bool,
    client_surface: Option<ClientSurfaceRuntime>,
    client_dispatch_state: Option<Arc<hydracache_client_transport_axum::ClientSurfaceState>>,
    redis_listener_config: Option<RedisListenerConfig>,
    redis_surface: Option<RedisSurfaceRuntime>,
    cluster_status: Arc<dyn ClusterStatusProvider>,
    grid_control: Option<Arc<dyn GridControlPlaneHandle>>,
    management_snapshot_override: Option<ManagementSnapshotOverride>,
    management_topology: crate::management_topology::ManagementTopologyModel,
    observability: ServerObservabilityModel,
    last_client_surface_drain: Option<ClientSurfaceDrain>,
    last_redis_surface_drain: Option<RedisSurfaceDrain>,
    last_drain: Option<DrainOutcome>,
    started_at: Instant,
}

/// Secret-free local process facts used by the authenticated management surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalManagementDiagnostics {
    pub(crate) product_version: String,
    pub(crate) uptime_seconds: u64,
    pub(crate) rss_bytes: Option<u64>,
    pub(crate) open_fds: Option<u64>,
    pub(crate) thread_count: Option<u64>,
    pub(crate) client_count: u64,
    pub(crate) config_digest: String,
}

/// Node-local protocol accounting; unavailable dimensions stay optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientLifecycleDiagnostics {
    pub(crate) hc1_rejected_total: Option<u64>,
    pub(crate) hc1_active_subscriptions: Option<u64>,
    pub(crate) resp_active: Option<u64>,
    pub(crate) resp_accepted: Option<u64>,
    pub(crate) resp_closed: Option<u64>,
    pub(crate) resp_rejected: Option<u64>,
}

impl ServerRuntime {
    /// Validate config and construct a runtime.
    pub fn new(config: ServerConfig) -> Result<Self, ServerConfigError> {
        config.validate()?;
        let (cache, cluster_status, grid_control): (
            HydraCache,
            Arc<dyn ClusterStatusProvider>,
            Option<Arc<dyn GridControlPlaneHandle>>,
        ) = match config.role {
            ServerRole::Member => {
                let (cache, grid) = crate::grid_host::build_member(&config)?;
                (
                    cache,
                    Arc::new(LiveClusterStatus::new(Arc::clone(&grid))),
                    Some(grid),
                )
            }
            ServerRole::Local | ServerRole::Client => (
                HydraCache::local()
                    .memory_instrumentation_mode(MemoryInstrumentationMode::Production)
                    .build(),
                Arc::new(ModeledClusterStatus),
                None,
            ),
        };
        let client_surface = if config.client_api.enabled {
            Some(
                ClientSurfaceRuntime::new(config.client_api.limits)
                    .map_err(|error| ServerConfigError::InvalidClientApi(error.to_string()))?,
            )
        } else {
            None
        };
        let client_dispatch_state = if config.redis_api.enabled || config.hc2_client_plane.enabled {
            Some(match &client_surface {
                Some(surface) => surface.state(),
                None => Arc::new(
                    hydracache_client_transport_axum::ClientSurfaceState::new(
                        config.client_api.limits,
                    )
                    .map_err(|error| ServerConfigError::InvalidClientApi(error.to_string()))?,
                ),
            })
        } else {
            None
        };
        let redis_surface = if config.redis_api.enabled {
            Some(RedisSurfaceRuntime::new())
        } else {
            None
        };
        let redis_listener_config = if config.redis_api.enabled {
            Some(config.redis_listener_config()?)
        } else {
            None
        };
        Ok(Self {
            config,
            cache,
            services: ServiceSet::default(),
            state: ServerState::Created,
            storage_open: false,
            cluster_ready: false,
            accepting: false,
            flushed: false,
            client_surface,
            client_dispatch_state,
            redis_listener_config,
            redis_surface,
            cluster_status,
            grid_control,
            management_snapshot_override: None,
            management_topology: crate::management_topology::ManagementTopologyModel::default(),
            observability: ServerObservabilityModel::default(),
            last_client_surface_drain: None,
            last_redis_surface_drain: None,
            last_drain: None,
            started_at: Instant::now(),
        })
    }

    /// Override the cluster-status provider.
    ///
    /// This is the W0 seam used by tests and later by the member-role grid host.
    pub fn with_cluster_status_provider(
        mut self,
        cluster_status: Arc<dyn ClusterStatusProvider>,
    ) -> Self {
        self.cluster_status = cluster_status;
        self
    }

    /// Override additional read-only observability signals.
    pub fn with_observability_model(mut self, observability: ServerObservabilityModel) -> Self {
        self.observability = observability;
        self
    }

    /// Start storage, cluster membership, listeners, and background services.
    pub fn start(mut self) -> Self {
        self.storage_open = true;
        self.cluster_ready = matches!(
            self.config.role,
            ServerRole::Local | ServerRole::Member | ServerRole::Client
        );
        self.accepting = true;
        if let Some(surface) = self.client_surface.as_mut() {
            surface.start();
        }
        if let Some(surface) = self.redis_surface.as_mut() {
            surface.start();
        }
        self.services.start();
        self.state = ServerState::Running;
        self
    }

    /// Return liveness.
    pub fn health(&self) -> ServerHealth {
        ServerHealth {
            status: if self.state == ServerState::Stopped {
                "stopped"
            } else {
                "ok"
            },
            state: self.state,
        }
    }

    /// Return readiness.
    pub fn ready(&self) -> ServerReadiness {
        ServerReadiness {
            ready: self.can_serve(),
            storage_open: self.storage_open,
            cluster_ready: self.cluster_ready,
            accepting: self.accepting,
            client_surface_ready: self.client_surface_ready(),
            redis_surface_ready: self.redis_surface_ready(),
        }
    }

    /// Return whether the runtime can serve traffic.
    pub fn can_serve(&self) -> bool {
        self.state == ServerState::Running
            && self.storage_open
            && self.cluster_ready
            && self.accepting
    }

    /// Return whether the runtime is currently draining.
    pub fn is_draining(&self) -> bool {
        self.state == ServerState::Draining
    }

    /// Begin one in-flight request.
    pub fn begin_request(&mut self) -> bool {
        if !self.accepting {
            return false;
        }
        self.services.begin_request();
        true
    }

    /// Complete one in-flight request.
    pub fn finish_request(&mut self) {
        self.services.finish_request();
    }

    /// Return whether the external client surface is accepting work.
    pub fn client_surface_ready(&self) -> bool {
        self.client_surface
            .as_ref()
            .is_some_and(ClientSurfaceRuntime::accepting)
    }

    /// Begin a modeled client subscription stream.
    pub fn begin_client_subscription(&self) -> bool {
        self.client_surface
            .as_ref()
            .is_some_and(|surface| surface.begin_subscription().is_ok())
    }

    /// Return active modeled client subscription streams.
    pub fn client_active_subscriptions(&self) -> u64 {
        self.client_surface
            .as_ref()
            .map_or(0, |surface| surface.state().active_subscriptions())
    }

    /// Return the last client-surface drain result, if the surface is enabled.
    pub fn client_surface_drain(&self) -> Option<ClientSurfaceDrain> {
        self.last_client_surface_drain
    }

    /// Return whether the optional Redis RESP surface is accepting work.
    pub fn redis_surface_ready(&self) -> bool {
        self.redis_surface
            .as_ref()
            .is_some_and(RedisSurfaceRuntime::accepting)
    }

    /// Begin a modeled RESP connection.
    pub fn begin_redis_connection(&mut self) -> bool {
        self.redis_surface
            .as_mut()
            .is_some_and(RedisSurfaceRuntime::begin_connection)
    }

    /// Finish a modeled RESP connection.
    pub fn finish_redis_connection(&mut self) {
        if let Some(surface) = self.redis_surface.as_mut() {
            surface.finish_connection();
        }
    }

    /// Return active modeled RESP connections.
    pub fn redis_active_connections(&self) -> u64 {
        self.redis_surface
            .as_ref()
            .map_or(0, RedisSurfaceRuntime::active_connections)
    }

    /// Return the last Redis RESP surface drain result, if the surface is enabled.
    pub fn redis_surface_drain(&self) -> Option<RedisSurfaceDrain> {
        self.last_redis_surface_drain
    }

    /// Return the Redis RESP bind address when that optional listener is enabled.
    pub fn redis_listener_addr(&self) -> Option<SocketAddr> {
        self.redis_surface
            .as_ref()
            .map(|_| self.config.redis_api.listen_addr)
    }

    /// Build a Redis RESP executor using this runtime's shared client-surface state.
    pub fn redis_resp_server(&self) -> Result<Option<RedisRespServer>, RedisServeError> {
        let Some(state) = &self.client_dispatch_state else {
            return Ok(None);
        };
        let Some(config) = &self.redis_listener_config else {
            return Ok(None);
        };
        RedisRespServer::new(Arc::clone(state), config.clone())
            .map(|server| server.with_native_cache_events(self.cache.clone()))
            .map(Some)
    }

    /// Build the optional Redis TLS acceptor when rediss:// is enabled.
    pub fn redis_tls_acceptor(&self) -> Result<Option<RedisTlsAcceptor>, RedisTlsError> {
        if !self.config.redis_api.enabled || !self.config.redis_api.rediss_enabled {
            return Ok(None);
        }
        RedisTlsAcceptor::from_tls_config(&self.config.tls).map(Some)
    }

    /// Return the shared verified-dispatch state used by HC/1, RESP, and HC/2.
    pub fn client_dispatch_state(
        &self,
    ) -> Option<Arc<hydracache_client_transport_axum::ClientSurfaceState>> {
        self.client_surface
            .as_ref()
            .map(ClientSurfaceRuntime::state)
            .or_else(|| self.client_dispatch_state.as_ref().map(Arc::clone))
    }

    /// Return whether the local-only destructive diagnostic reset is enabled.
    pub fn diagnostic_reset_enabled(&self) -> bool {
        self.config.admin_api.diagnostic_reset_enabled
    }

    /// Clone the owners cleared by the internal diagnostic reset route.
    pub fn diagnostic_reset_targets(
        &self,
    ) -> (
        HydraCache,
        Option<Arc<hydracache_client_transport_axum::ClientSurfaceState>>,
    ) {
        (self.cache.clone(), self.client_dispatch_state())
    }

    /// Stop accepting new work and enter the draining state.
    pub fn begin_drain(&mut self) {
        self.begin_local_drain();
        self.cluster_status.begin_drain();
    }

    /// Accept an operator/admin drain request without stopping the daemon process.
    pub fn request_admin_drain(&mut self) -> DrainOutcome {
        if self.state == ServerState::Stopped {
            return self.last_drain.unwrap_or(DrainOutcome {
                started_with: 0,
                remaining: 0,
                timed_out: false,
            });
        }
        self.begin_local_drain();
        let drain_timeout = self.config.drain_timeout();
        let started = Instant::now();
        let control_plane_drained = self.prepare_cluster_drain(drain_timeout);
        let mut outcome = GracefulShutdown::new(drain_timeout.saturating_sub(started.elapsed()))
            .drain(&mut self.services);
        outcome.timed_out |= !control_plane_drained;
        self.last_drain = Some(outcome);
        outcome
    }

    fn begin_local_drain(&mut self) {
        if matches!(self.state, ServerState::Stopped) {
            return;
        }
        self.accepting = false;
        self.state = ServerState::Draining;
        if let Some(surface) = self.client_surface.as_mut() {
            if self
                .last_client_surface_drain
                .is_none_or(|drain| drain.remaining > 0)
            {
                self.last_client_surface_drain = Some(surface.shutdown());
            }
        }
        if let Some(surface) = self.redis_surface.as_mut() {
            if self
                .last_redis_surface_drain
                .is_none_or(|drain| drain.remaining > 0)
            {
                self.last_redis_surface_drain = Some(surface.shutdown());
            }
        }
    }

    /// Gracefully stop accepting, drain, flush, and stop services.
    pub fn graceful_shutdown(&mut self) -> DrainOutcome {
        if self.state == ServerState::Stopped {
            return self.last_drain.unwrap_or(DrainOutcome {
                started_with: 0,
                remaining: 0,
                timed_out: false,
            });
        }
        self.begin_local_drain();
        let drain_timeout = self.config.drain_timeout();
        let started = Instant::now();
        let control_plane_drained = self.prepare_cluster_drain(drain_timeout);
        let mut outcome = GracefulShutdown::new(drain_timeout.saturating_sub(started.elapsed()))
            .drain(&mut self.services);
        outcome.timed_out |= !control_plane_drained;
        self.flushed = true;
        self.storage_open = false;
        self.cluster_ready = false;
        self.services.stop();
        self.state = ServerState::Stopped;
        self.last_drain = Some(outcome);
        outcome
    }

    /// Backward-compatible alias for graceful shutdown.
    pub fn shutdown(&mut self) -> DrainOutcome {
        self.graceful_shutdown()
    }

    fn leave_cluster_for_shutdown(&self) {
        if matches!(self.config.role, ServerRole::Member | ServerRole::Client) {
            let _ = block_on_cluster_leave(&self.cache);
        }
    }

    fn prepare_cluster_drain(&self, timeout: std::time::Duration) -> bool {
        // NodeLeft must commit while this process is still a voter. Only then
        // may the voter-removal transition start. Waiting for that removal to
        // apply keeps a two-voter cluster live until its survivor owns a
        // single-voter configuration; surviving leaders also reconcile the
        // same committed membership if this bounded wait fails.
        self.leave_cluster_for_shutdown();
        self.cluster_status.begin_drain();
        self.cluster_status.wait_for_drain_ready(timeout)
    }

    /// Return admin/operator status derived from the runtime model.
    pub fn admin_status(&self) -> ServerAdminStatus {
        let status = self.cluster_status_snapshot();
        let mut member_ids = status
            .members
            .iter()
            .map(|member| member.node_id.clone())
            .collect::<Vec<_>>();
        member_ids.sort();
        ServerAdminStatus {
            source: status.source,
            leader: status.leader,
            term: status.term,
            epoch: status.epoch,
            quorum_ok: status.quorum_ok,
            members: status.members.len() as u32,
            member_ids,
            voters: status.voters,
            voter_ids: status.voter_ids,
            reshard_phase: status.reshard_phase.to_string(),
            draining: status.draining,
        }
    }

    /// Return current disk-backed Raft compaction progress without mutating it.
    pub fn raft_compaction_status(&self) -> Result<RaftCompactionStatus, RaftCompactionError> {
        self.grid_control.as_ref().map_or_else(
            || Ok(RaftCompactionStatus::unavailable()),
            |grid| grid.raft_compaction_status(),
        )
    }

    /// Explicitly compact the durable Raft log at current applied progress.
    pub fn request_raft_compaction(&self) -> Result<RaftCompactionStatus, ServerAdminActionError> {
        if !self.can_serve() {
            return Err(ServerAdminActionError::NotReady("raft compaction"));
        }
        if !matches!(self.config.role, ServerRole::Member) {
            return Err(ServerAdminActionError::RequiresMember("raft compaction"));
        }
        self.grid_control
            .as_ref()
            .ok_or(RaftCompactionError::Unavailable)?
            .compact_raft_log_at_applied()
            .map_err(ServerAdminActionError::from)
    }

    /// Build a metrics registry snapshot for the admin surface.
    pub fn metrics_registry(&self) -> HydraCacheRegistry {
        let status = self.cluster_status_snapshot();
        let registry = HydraCacheRegistry::new()
            .with_cache("server", self.cache.clone())
            .with_cluster_grid_counters(self.observability.cluster_grid)
            .with_topology(ClusterTopologyOverview::new(
                topology_status_source(status.source),
                status.members.len() as u64,
                status.leader,
                status.epoch,
                topology_reshard_phase(status.reshard_phase),
            ));
        if let Some(seconds) = self.observability.backup_age_seconds {
            registry.with_backup_age_seconds(seconds)
        } else {
            registry
        }
    }

    /// Build a read-only Management Center cluster overview.
    pub fn cluster_overview(&self) -> ClusterOverview {
        let status = self.cluster_status_snapshot();
        let counters = overview_cluster_grid_counters(
            self.cache.cluster_grid_counters(),
            self.observability.cluster_grid,
        );
        ClusterOverview::new(
            topology_status_source(status.source),
            status
                .members
                .iter()
                .map(|member| {
                    ClusterMemberView::new(
                        member.node_id.clone(),
                        member_role_label(member.role),
                        member.reachable == Reachability::Reachable,
                        reachability_label(member.reachable),
                        member.generation,
                    )
                })
                .collect(),
            cluster_overview_leader(&status),
            PartitionSummary::from_grid_counters(counters, self.observability.partition_count),
            ConsistencyView::from_grid_counters(
                self.observability.configured_default_consistency.clone(),
                counters,
            ),
            self.observability.backup_age_seconds,
            LifecycleView::new(
                status.reshard_phase.to_string(),
                self.observability.upgrade_phase.clone(),
            ),
        )
    }

    pub(crate) fn cluster_status_snapshot(&self) -> ClusterStatus {
        let cluster_ready = self.cluster_ready && self.state != ServerState::Stopped;
        self.cluster_status
            .cluster_status(ClusterStatusRuntime::new(cluster_ready, self.is_draining()))
    }

    /// Override the read-only management aggregation source for embedding and tests.
    pub fn with_management_snapshot_source(
        mut self,
        local: crate::management_aggregation::ManagementMemberSnapshot,
        committed_targets: Vec<crate::management_aggregation::ManagementPeerTarget>,
        transport: Arc<dyn crate::management_aggregation::ManagementPeerTransport>,
    ) -> Self {
        self.management_snapshot_override = Some(ManagementSnapshotOverride {
            local,
            committed_targets,
            transport,
        });
        self
    }

    /// Attach validated immutable partition and placement evidence.
    pub fn with_management_topology_model(
        mut self,
        model: crate::management_topology::ManagementTopologyModel,
    ) -> Self {
        self.management_topology = model;
        self
    }

    pub(crate) fn management_topology_model(
        &self,
    ) -> crate::management_topology::ManagementTopologyModel {
        self.management_topology.clone()
    }

    pub(crate) fn management_local_diagnostics(&self) -> LocalManagementDiagnostics {
        let (rss_bytes, open_fds, thread_count) = process_resource_counts();
        let mut hasher = Sha256::new();
        hasher.update(b"hydracache-management-config-v1\0");
        hasher.update(format!(
            "role={:?};tls={};client={};hc2={};admin={};redis={};backup={};partitions={}",
            self.config.role,
            self.config.tls.enabled,
            self.config.client_api.enabled,
            self.config.hc2_client_plane.enabled,
            self.config.admin_api.enabled,
            self.config.redis_api.enabled,
            self.config.backup.enabled,
            self.observability.partition_count,
        ));
        let digest = hasher.finalize();
        LocalManagementDiagnostics {
            product_version: env!("CARGO_PKG_VERSION").to_owned(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            rss_bytes,
            open_fds,
            thread_count,
            client_count: self
                .client_active_subscriptions()
                .saturating_add(self.redis_active_connections()),
            config_digest: format!(
                "sha256-v1:{}",
                digest[..12]
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ),
        }
    }

    pub(crate) fn management_cache(&self) -> HydraCache {
        self.cache.clone()
    }

    pub(crate) fn management_client_lifecycle(&self) -> ClientLifecycleDiagnostics {
        let hc1 = self
            .config
            .client_api
            .enabled
            .then(|| self.client_dispatch_state())
            .flatten();
        ClientLifecycleDiagnostics {
            hc1_rejected_total: hc1.as_ref().map(|state| {
                state
                    .rejected_anonymous()
                    .saturating_add(state.rejected_oversized())
            }),
            hc1_active_subscriptions: hc1.as_ref().map(|state| state.active_subscriptions()),
            resp_active: self
                .redis_surface
                .as_ref()
                .map(RedisSurfaceRuntime::active_connections),
            resp_accepted: self
                .redis_surface
                .as_ref()
                .map(|surface| surface.accepted_connections),
            resp_closed: self
                .redis_surface
                .as_ref()
                .map(|surface| surface.closed_connections),
            resp_rejected: self
                .redis_surface
                .as_ref()
                .map(|surface| surface.rejected_connections),
        }
    }

    pub(crate) fn management_snapshot_input(
        &self,
    ) -> Option<(
        crate::management_aggregation::ManagementMemberSnapshot,
        Vec<crate::management_aggregation::ManagementPeerTarget>,
    )> {
        if let Some(source) = &self.management_snapshot_override {
            return Some((source.local.clone(), source.committed_targets.clone()));
        }
        let grid = self.grid_control.as_ref()?;
        Some((
            grid.local_management_snapshot()?,
            grid.management_peer_targets(),
        ))
    }

    pub(crate) fn management_peer_transport(
        &self,
    ) -> Option<Arc<dyn crate::management_aggregation::ManagementPeerTransport>> {
        if let Some(source) = &self.management_snapshot_override {
            return Some(Arc::clone(&source.transport));
        }
        self.grid_control
            .as_ref()
            .and_then(|grid| grid.management_peer_transport())
    }

    /// Request an online reshard through the current runtime model.
    pub fn request_reshard(&self) -> Result<ServerAdminAction, ServerAdminActionError> {
        if !self.can_serve() {
            return Err(ServerAdminActionError::NotReady("reshard"));
        }
        if !matches!(self.config.role, ServerRole::Member) {
            return Err(ServerAdminActionError::RequiresMember("reshard"));
        }
        Ok(ServerAdminAction {
            action: "reshard",
            outcome: "accepted",
            detail: "reshard request accepted by member runtime".to_owned(),
        })
    }

    /// Request a backup through the current runtime model.
    pub fn request_backup(&self) -> Result<ServerAdminAction, ServerAdminActionError> {
        if !self.can_serve() {
            return Err(ServerAdminActionError::NotReady("backup"));
        }
        if !self.config.backup.enabled
            || self
                .config
                .backup
                .location
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err(ServerAdminActionError::BackupDisabled);
        }
        Ok(ServerAdminAction {
            action: "backup",
            outcome: "accepted",
            detail: "backup request accepted by configured runtime".to_owned(),
        })
    }

    /// Return whether shutdown flushed durable state.
    pub fn flushed(&self) -> bool {
        self.flushed
    }

    /// Return cache handle used by embedded tests/adapters.
    pub fn cache(&self) -> &HydraCache {
        &self.cache
    }

    /// Return the runtime config.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

#[cfg(target_os = "linux")]
fn process_resource_counts() -> (Option<u64>, Option<u64>, Option<u64>) {
    let open_fds = std::fs::read_dir("/proc/self/fd")
        .ok()
        .map(|entries| entries.filter_map(Result::ok).count() as u64);
    let status = std::fs::read_to_string("/proc/self/status").ok();
    let rss_bytes = status.as_deref().and_then(|status| {
        status.lines().find_map(|line| {
            line.strip_prefix("VmRSS:").and_then(|value| {
                value
                    .split_whitespace()
                    .next()
                    .and_then(|kilobytes| kilobytes.parse::<u64>().ok())
                    .and_then(|kilobytes| kilobytes.checked_mul(1024))
            })
        })
    });
    let thread_count = status.as_deref().and_then(|status| {
        status.lines().find_map(|line| {
            line.strip_prefix("Threads:")
                .and_then(|value| value.trim().parse().ok())
        })
    });
    (rss_bytes, open_fds, thread_count)
}

#[cfg(not(target_os = "linux"))]
fn process_resource_counts() -> (Option<u64>, Option<u64>, Option<u64>) {
    (None, None, None)
}

fn topology_status_source(source: StatusSource) -> TopologyStatusSource {
    match source {
        StatusSource::Live => TopologyStatusSource::Live,
        StatusSource::Modeled => TopologyStatusSource::Modeled,
    }
}

fn block_on_cluster_leave(cache: &HydraCache) -> hydracache::CacheResult<()> {
    let cache = cache.clone();
    if tokio::runtime::Handle::try_current().is_ok() {
        return std::thread::spawn(move || block_on_cluster_leave_without_current(cache))
            .join()
            .map_err(|_| {
                hydracache::CacheError::Backend("cluster leave helper thread panicked".to_owned())
            })?;
    }

    block_on_cluster_leave_without_current(cache)
}

fn block_on_cluster_leave_without_current(cache: HydraCache) -> hydracache::CacheResult<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            hydracache::CacheError::Backend(format!(
                "failed to build cluster leave runtime: {error}"
            ))
        })?;
    let left = runtime.block_on(cache.leave_cluster())?;
    let _ = left;
    Ok(())
}

fn topology_reshard_phase(phase: ReshardPhase) -> TopologyReshardPhase {
    match phase {
        ReshardPhase::Idle => TopologyReshardPhase::Idle,
        ReshardPhase::Planning => TopologyReshardPhase::Planning,
        ReshardPhase::Moving => TopologyReshardPhase::Moving,
        ReshardPhase::Finalizing => TopologyReshardPhase::Finalizing,
    }
}

fn cluster_overview_leader(status: &ClusterStatus) -> Option<LeaderView> {
    if status.source != StatusSource::Live || !status.quorum_ok {
        return None;
    }
    status
        .leader
        .as_ref()
        .map(|node_id| LeaderView::new(node_id.clone(), status.term, status.epoch))
}

fn member_role_label(role: MemberRole) -> &'static str {
    match role {
        MemberRole::Local => "local",
        MemberRole::Client => "client",
        MemberRole::Member => "member",
    }
}

fn reachability_label(reachability: Reachability) -> &'static str {
    match reachability {
        Reachability::Reachable => "reachable",
        Reachability::Suspect => "suspect",
        Reachability::Unreachable => "unreachable",
    }
}

fn overview_cluster_grid_counters(
    mut left: ClusterGridCounters,
    right: ClusterGridCounters,
) -> ClusterGridCounters {
    left.under_replicated_keys = left
        .under_replicated_keys
        .saturating_add(right.under_replicated_keys);
    left.consistency_level_operations_total = left
        .consistency_level_operations_total
        .saturating_add(right.consistency_level_operations_total);
    left
}
