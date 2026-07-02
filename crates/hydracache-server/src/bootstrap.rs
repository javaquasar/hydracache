use hydracache::HydraCache;
use hydracache_client_transport_axum::{ClientSurfaceDrain, ClientSurfaceRuntime};
use hydracache_observability::{
    ClusterTopologyOverview, HydraCacheRegistry, TopologyReshardPhase, TopologyStatusSource,
};
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;

use crate::cluster_status::{
    ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime, ModeledClusterStatus, ReshardPhase,
    StatusSource,
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
    last_client_surface_drain: Option<ClientSurfaceDrain>,
    last_drain: Option<DrainOutcome>,
}

impl ServerRuntime {
    /// Validate config and construct a runtime.
    pub fn new(config: ServerConfig) -> Result<Self, ServerConfigError> {
        config.validate()?;
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
            cache: HydraCache::local().build(),
            services: ServiceSet::default(),
            state: ServerState::Created,
            storage_open: false,
            cluster_ready: false,
            accepting: false,
            flushed: false,
            client_surface,
            cluster_status: Arc::new(ModeledClusterStatus),
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
        self.begin_drain();
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

    /// Return admin/operator status derived from the runtime model.
    pub fn admin_status(&self) -> ServerAdminStatus {
        let status = self.cluster_status_snapshot();
        ServerAdminStatus {
            source: status.source,
            leader: status.leader,
            term: status.term,
            quorum_ok: status.quorum_ok,
            members: status.members.len() as u32,
            reshard_phase: status.reshard_phase.to_string(),
            draining: status.draining,
        }
    }

    /// Build a metrics registry snapshot for the admin surface.
    pub fn metrics_registry(&self) -> HydraCacheRegistry {
        let status = self.cluster_status_snapshot();
        HydraCacheRegistry::new()
            .with_cache("server", self.cache.clone())
            .with_topology(ClusterTopologyOverview::new(
                topology_status_source(status.source),
                status.members.len() as u64,
                status.leader,
                status.epoch,
                topology_reshard_phase(status.reshard_phase),
            ))
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

fn topology_reshard_phase(phase: ReshardPhase) -> TopologyReshardPhase {
    match phase {
        ReshardPhase::Idle => TopologyReshardPhase::Idle,
        ReshardPhase::Planning => TopologyReshardPhase::Planning,
        ReshardPhase::Moving => TopologyReshardPhase::Moving,
        ReshardPhase::Finalizing => TopologyReshardPhase::Finalizing,
    }
}
