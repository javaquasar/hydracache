//! Standalone HydraCache server bootstrap primitives.
//!
//! The crate keeps the first production daemon surface framework-neutral where
//! possible: configuration, validation, readiness, and graceful drain can be
//! tested without binding sockets. The binary wrapper wires these primitives to
//! process startup.

pub mod bootstrap;
pub mod config;
pub mod services;
pub mod upgrade;

pub use bootstrap::{ServerHealth, ServerReadiness, ServerRuntime, ServerState};
pub use config::{BackupConfig, ServerConfig, ServerConfigError, ServerRole, TlsConfig};
pub use services::{DrainOutcome, GracefulShutdown, ServiceSet};
pub use upgrade::{
    GracefulUpgrade, UpgradeError, UpgradePhase, UpgradePlan, UpgradeReport, UpgradeStrategy,
};
