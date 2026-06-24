use hydracache::HydraCache;
use hydracache_client_transport_axum::{ClientSurfaceDrain, ClientSurfaceRuntime};
use serde::Serialize;

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
    last_client_surface_drain: Option<ClientSurfaceDrain>,
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
            last_client_surface_drain: None,
        })
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
            ready: self.state == ServerState::Running
                && self.storage_open
                && self.cluster_ready
                && self.accepting,
            storage_open: self.storage_open,
            cluster_ready: self.cluster_ready,
            accepting: self.accepting,
            client_surface_ready: self.client_surface_ready(),
        }
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

    /// Gracefully stop accepting, drain, flush, and stop services.
    pub fn shutdown(&mut self) -> DrainOutcome {
        self.accepting = false;
        self.state = ServerState::Draining;
        if let Some(surface) = self.client_surface.as_mut() {
            self.last_client_surface_drain = Some(surface.shutdown());
        }
        let outcome = GracefulShutdown::new(self.config.drain_timeout()).drain(&mut self.services);
        self.flushed = true;
        self.storage_open = false;
        self.cluster_ready = false;
        self.services.stop();
        self.state = ServerState::Stopped;
        outcome
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
