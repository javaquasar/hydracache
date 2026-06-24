use hydracache::HydraCache;
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
}

impl ServerRuntime {
    /// Validate config and construct a runtime.
    pub fn new(config: ServerConfig) -> Result<Self, ServerConfigError> {
        config.validate()?;
        Ok(Self {
            config,
            cache: HydraCache::local().build(),
            services: ServiceSet::default(),
            state: ServerState::Created,
            storage_open: false,
            cluster_ready: false,
            accepting: false,
            flushed: false,
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

    /// Gracefully stop accepting, drain, flush, and stop services.
    pub fn shutdown(&mut self) -> DrainOutcome {
        self.accepting = false;
        self.state = ServerState::Draining;
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
