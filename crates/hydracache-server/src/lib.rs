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
pub mod management_history;
mod management_http;
pub mod management_operations;
pub mod management_security;
pub mod management_topology;
pub mod redis_tcp;
pub mod services;
pub mod upgrade;

pub use admin_http::{
    AdminDiagnosticResetReply, AdminHttpError, AdminHttpSurface, ADMIN_ACTUATOR_PATH,
    ADMIN_BACKUP_PATH, ADMIN_CLUSTER_OVERVIEW_PATH, ADMIN_CONSOLE_PATH,
    ADMIN_DIAGNOSTIC_RESET_PATH, ADMIN_DRAIN_PATH, ADMIN_HEALTHZ_PATH, ADMIN_MEMORY_FOOTPRINT_PATH,
    ADMIN_METRICS_PATH, ADMIN_RAFT_COMPACTION_PATH, ADMIN_READYZ_PATH, ADMIN_RESHARD_PATH,
    ADMIN_STATUS_PATH, HYDRACACHE_MANAGEMENT_READ_HEADER,
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
    Hc2ClientPlaneConfig, ManagementHistoryConfig, RedisApiConfig, ServerConfig, ServerConfigError,
    ServerRole, TlsConfig,
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
pub use management_history::{
    parse_prometheus_response, validate_resolved_addresses, ManagementHistoryData,
    ManagementHistoryError, ManagementHistoryPoint, ManagementHistoryQueryId,
    ManagementHistoryRequest, ManagementHistorySeries, ManagementHistoryService,
    ManagementHistoryState, MANAGEMENT_HISTORY_DEADLINE, MANAGEMENT_HISTORY_MAX_CONCURRENCY,
    MANAGEMENT_HISTORY_MAX_POINTS, MANAGEMENT_HISTORY_MAX_RANGE_MS,
    MANAGEMENT_HISTORY_MAX_RESOLVED_ADDRESSES, MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES,
    MANAGEMENT_HISTORY_MAX_SERIES, MANAGEMENT_HISTORY_MAX_TOKEN_BYTES,
    MANAGEMENT_HISTORY_MIN_STEP_MS, MANAGEMENT_HISTORY_PATH,
};
pub use management_http::{
    MANAGEMENT_AUDIT_PATH, MANAGEMENT_CAPABILITIES_PATH, MANAGEMENT_CLIENTS_PATH,
    MANAGEMENT_CLUSTER_FORMATION_PATH, MANAGEMENT_CLUSTER_MEMBERS_PATH,
    MANAGEMENT_CLUSTER_PARTITIONS_PATH, MANAGEMENT_CONSENSUS_PROGRESS_PATH,
    MANAGEMENT_CURSOR_TTL_MS, MANAGEMENT_DASHBOARD_PATH, MANAGEMENT_FORMATION_PATH,
    MANAGEMENT_HEALTHCHECKS_PATH, MANAGEMENT_MAX_RESPONSE_BYTES, MANAGEMENT_MAX_RETAINED_CURSORS,
    MANAGEMENT_NAMESPACES_PATH, MANAGEMENT_NAMESPACE_CACHES_PREFIX, MANAGEMENT_OPERATIONS_PATH,
    MANAGEMENT_PERSISTENCE_PATH, MANAGEMENT_PLACEMENT_TRACE_PREFIX, MANAGEMENT_RECOVERY_PATH,
    MANAGEMENT_STALE_AFTER_MS,
};
pub use management_operations::{
    ManagementAuditRecord, ManagementAuditSnapshot, ManagementOperationError,
    ManagementOperationJournal, ManagementOperationKind, ManagementOperationRecord,
    ManagementOperationSnapshot, ManagementOperationState, MANAGEMENT_AUDIT_CAPACITY,
    MANAGEMENT_OPERATION_CAPACITY,
};
pub use management_security::{
    ManagementReadLimitError, ManagementReadLimiter, MANAGEMENT_READ_MAX_CONCURRENCY,
};
pub use management_topology::{normalize_placement_trace, ManagementTopologyModel};
pub use redis_tcp::{serve_redis_listener, RedisTcpError, RedisTlsAcceptor, RedisTlsError};
pub use services::{DrainOutcome, GracefulShutdown, ServiceSet};
pub use upgrade::{
    GracefulUpgrade, UpgradeError, UpgradePhase, UpgradePlan, UpgradeReport, UpgradeStrategy,
};
