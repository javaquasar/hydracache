//! Standalone HydraCache server bootstrap primitives.
//!
//! The crate keeps the first production daemon surface framework-neutral where
//! possible: configuration, validation, readiness, and graceful drain can be
//! tested without binding sockets. The binary wrapper wires these primitives to
//! process startup.

pub mod admin_http;
pub mod bootstrap;
pub mod cluster_status;
pub mod config;
mod grid_host;
pub mod services;
pub mod upgrade;

pub use admin_http::{
    AdminHttpError, AdminHttpSurface, ADMIN_ACTUATOR_PATH, ADMIN_BACKUP_PATH,
    ADMIN_CLUSTER_OVERVIEW_PATH, ADMIN_CONSOLE_PATH, ADMIN_DRAIN_PATH, ADMIN_HEALTHZ_PATH,
    ADMIN_METRICS_PATH, ADMIN_READYZ_PATH, ADMIN_RESHARD_PATH, ADMIN_STATUS_PATH,
};
pub use bootstrap::{
    ServerAdminAction, ServerAdminActionError, ServerAdminStatus, ServerHealth,
    ServerObservabilityModel, ServerReadiness, ServerRuntime, ServerState,
};
pub use cluster_status::{
    ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime, GridControlPlaneHandle,
    LiveClusterStatus, MemberRole, MemberStatus, ModeledClusterStatus, Reachability, ReshardPhase,
    StatusSource,
};
pub use config::{
    AdminApiConfig, BackupConfig, ClientApiConfig, ServerConfig, ServerConfigError, ServerRole,
    TlsConfig,
};
pub use services::{DrainOutcome, GracefulShutdown, ServiceSet};
pub use upgrade::{
    GracefulUpgrade, UpgradeError, UpgradePhase, UpgradePlan, UpgradeReport, UpgradeStrategy,
};
