use hydracache::{ClusterGridCounters, HydraCache};
use hydracache_client_transport_axum::{ClientSurfaceDrain, ClientSurfaceRuntime};
use hydracache_observability::{
    ClusterMemberView, ClusterOverview, ClusterTopologyOverview, ConsistencyView,
    HydraCacheRegistry, LeaderView, LifecycleView, PartitionSummary, TopologyReshardPhase,
    TopologyStatusSource,
};
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;

use crate::cluster_status::{
    ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime, LiveClusterStatus, MemberRole,
    ModeledClusterStatus, Reachability, ReshardPhase, StatusSource,
};
use crate::config::{ServerConfig, ServerConfigError, ServerRole};
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
    /// Whether the runtime believes quorum is available.
    pub quorum_ok: bool,
    /// Observed member count.
    pub members: u32,
    /// Observed raft voter count.
    pub voters: u32,
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
    cluster_status: Arc<dyn ClusterStatusProvider>,
    observability: ServerObservabilityModel,
    last_client_surface_drain: Option<ClientSurfaceDrain>,
    last_drain: Option<DrainOutcome>,
}

impl ServerRuntime {
    /// Validate config and construct a runtime.
    pub fn new(config: ServerConfig) -> Result<Self, ServerConfigError> {
        config.validate()?;
        let (cache, cluster_status): (HydraCache, Arc<dyn ClusterStatusProvider>) =
            match config.role {
                ServerRole::Member => {
                    let (cache, grid) = crate::grid_host::build_member(&config)?;
                    (cache, Arc::new(LiveClusterStatus::new(grid)))
                }
                ServerRole::Local | ServerRole::Client => {
                    (HydraCache::local().build(), Arc::new(ModeledClusterStatus))
                }
            };
        let client_surface = if config.client_api.enabled {
            Some(
                ClientSurfaceRuntime::new(config.client_api.limits)
                    .map_err(|error| ServerConfigError::InvalidClientApi(error.to_string()))?,
            )
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
            cluster_status,
            observability: ServerObservabilityModel::default(),
            last_client_surface_drain: None,
            last_drain: None,
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
        self.leave_cluster_for_shutdown();
        self.cluster_status.begin_drain();
        let outcome = GracefulShutdown::new(self.config.drain_timeout()).drain(&mut self.services);
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
        self.leave_cluster_for_shutdown();
        self.cluster_status.begin_drain();
        let outcome = GracefulShutdown::new(self.config.drain_timeout()).drain(&mut self.services);
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

    /// Return admin/operator status derived from the runtime model.
    pub fn admin_status(&self) -> ServerAdminStatus {
        let status = self.cluster_status_snapshot();
        ServerAdminStatus {
            source: status.source,
            leader: status.leader,
            term: status.term,
            quorum_ok: status.quorum_ok,
            members: status.members.len() as u32,
            voters: status.voters,
            reshard_phase: status.reshard_phase.to_string(),
            draining: status.draining,
        }
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

    fn cluster_status_snapshot(&self) -> ClusterStatus {
        let cluster_ready = self.cluster_ready && self.state != ServerState::Stopped;
        self.cluster_status
            .cluster_status(ClusterStatusRuntime::new(cluster_ready, self.is_draining()))
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
    if status.source != StatusSource::Live {
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
