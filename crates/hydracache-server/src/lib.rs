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
pub mod hc2;
pub mod management_aggregation;
mod management_http;
pub mod redis_tcp;
pub mod services;
pub mod upgrade;

pub use admin_http::{
    AdminDiagnosticResetReply, AdminHttpError, AdminHttpSurface, ADMIN_ACTUATOR_PATH,
    ADMIN_BACKUP_PATH, ADMIN_CLUSTER_OVERVIEW_PATH, ADMIN_CONSOLE_PATH,
    ADMIN_DIAGNOSTIC_RESET_PATH, ADMIN_DRAIN_PATH, ADMIN_HEALTHZ_PATH, ADMIN_MEMORY_FOOTPRINT_PATH,
    ADMIN_METRICS_PATH, ADMIN_RAFT_COMPACTION_PATH, ADMIN_READYZ_PATH, ADMIN_RESHARD_PATH,
    ADMIN_STATUS_PATH,
};
pub use bootstrap::{
    RedisSurfaceDrain, ServerAdminAction, ServerAdminActionError, ServerAdminStatus, ServerHealth,
    ServerObservabilityModel, ServerReadiness, ServerRuntime, ServerState,
};
pub use cluster_status::{
    ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime, GridControlPlaneHandle,
    LiveClusterStatus, LocalConsensusStatus, MemberRole, MemberStatus, ModeledClusterStatus,
    RaftCompactionError, RaftCompactionStatus, Reachability, ReshardPhase, StatusSource,
};
pub use config::{
    AdminApiConfig, BackupConfig, ClientApiConfig, ClusterAuthConfig, ClusterStartMode,
    Hc2ClientPlaneConfig, RedisApiConfig, ServerConfig, ServerConfigError, ServerRole, TlsConfig,
};
pub use hc2::{serve_hc2_listener, Hc2ClientPlaneService, Hc2ListenerTls, Hc2ServeError};
pub use management_aggregation::{
    AggregatedManagementSnapshot, HttpManagementPeerTransport, LocalManagementSnapshotProvider,
    ManagementAggregationIssue, ManagementConsensusObservation, ManagementMemberSnapshot,
    ManagementPeerFailure, ManagementPeerTarget, ManagementPeerTransport,
    ManagementSnapshotAggregator, ManagementSnapshotRequest, ManagementSnapshotRpcService,
    RetainedLocalManagementSnapshot, CLUSTER_MANAGEMENT_SNAPSHOT_PATH,
    MANAGEMENT_SNAPSHOT_CACHE_TTL, MANAGEMENT_SNAPSHOT_MAX_CONCURRENCY,
    MANAGEMENT_SNAPSHOT_MAX_PEERS, MANAGEMENT_SNAPSHOT_MAX_REQUEST_BYTES,
    MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES, MANAGEMENT_SNAPSHOT_PEER_TIMEOUT,
    MANAGEMENT_SNAPSHOT_REFRESH_TIMEOUT,
};
pub use management_http::{
    MANAGEMENT_CAPABILITIES_PATH, MANAGEMENT_CONSENSUS_PROGRESS_PATH, MANAGEMENT_CURSOR_TTL_MS,
    MANAGEMENT_DASHBOARD_PATH, MANAGEMENT_FORMATION_PATH, MANAGEMENT_MAX_RESPONSE_BYTES,
    MANAGEMENT_MAX_RETAINED_CURSORS, MANAGEMENT_RECOVERY_PATH, MANAGEMENT_STALE_AFTER_MS,
};
pub use redis_tcp::{serve_redis_listener, RedisTcpError, RedisTlsAcceptor, RedisTlsError};
pub use services::{DrainOutcome, GracefulShutdown, ServiceSet};
pub use upgrade::{
    GracefulUpgrade, UpgradeError, UpgradePhase, UpgradePlan, UpgradeReport, UpgradeStrategy,
};
