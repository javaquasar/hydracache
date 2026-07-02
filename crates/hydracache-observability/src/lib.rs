//! Framework-neutral observability helpers for HydraCache.
//!
//! This crate deliberately has no HTTP dependency. It turns one or more
//! [`HydraCache`] instances into named diagnostic snapshots that can be exposed
//! by a web adapter, logged, tested, or inspected by application code.
//!
//! # Example
//!
//! ```rust
//! use hydracache::{CacheOptions, HydraCache};
//! use hydracache_observability::HydraCacheRegistry;
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//!
//! cache
//!     .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
//!     .await?;
//! cache
//!     .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
//!     .await?;
//!
//! let registry = HydraCacheRegistry::new().with_cache("main", cache);
//! let diagnostics = registry.diagnostics("main").await.unwrap();
//!
//! assert_eq!(diagnostics.stats.loads, 1);
//! assert_eq!(diagnostics.stats.hits, 1);
//! assert_eq!(diagnostics.hit_ratio(), Some(0.5));
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use hydracache::{
    AdmissionSnapshot, ClusterEpoch, ClusterGridCounters, ClusterGridDiagnostics, ClusterNodeId,
    ClusterPilotReport, ClusterStagingHealth, HydraCache, QuorumPosture, RegionId, RegionState,
    SplitBrainReport, StalenessBound,
};
use hydracache_core::{CacheCodec, CacheDiagnostics, CacheStats, PostcardCodec};
use serde::Serialize;

pub mod audit;
pub mod consumer;
pub mod exporter;

pub use audit::{
    AuditEnvelope, AuditError, AuditEvent, AuditHealth, AuditKey, AuditKeyPolicy, AuditOutcome,
    AuditRecorder, AuditRedactionPolicy, AuditSink, InMemoryAuditSink,
    CONSUMER_AUDIT_EVENT_SCHEMA_VERSION,
};
pub use consumer::{
    consumer_alert_metric_names, consumer_metric_names, ConsumerNearCacheStatus,
    TenantNamespaceStatus, TenantRateLimitStatus, TenantStatus, TENANT_STATUS_SCHEMA_VERSION,
};
pub use exporter::{registered_metric_names, PrometheusExporter};

/// Serializable snapshot of [`CacheStats`].
///
/// `CacheStats` itself stays a lightweight runtime type. This DTO adds derived
/// values that are convenient in JSON responses and smoke tests.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CacheStatsSnapshot {
    /// Successful cache lookups.
    pub hits: u64,
    /// Cache lookups that did not return a usable value.
    pub misses: u64,
    /// Loader closures executed by `get_or_load`.
    pub loads: u64,
    /// Calls that joined an already running single-flight load.
    pub single_flight_joins: u64,
    /// Loader results skipped because their invalidation generation became stale.
    pub stale_load_discards: u64,
    /// Per-key loader breakers opened after repeated failures.
    pub load_breaker_open_total: u64,
    /// Per-key loader breakers allowed one half-open probe.
    pub load_breaker_half_open_total: u64,
    /// Per-key loader breakers closed after a successful probe.
    pub load_breaker_recovered_total: u64,
    /// Loader calls rejected because a breaker was open.
    pub load_breaker_rejected_total: u64,
    /// Entries removed by invalidation APIs.
    pub invalidations: u64,
    /// Entries observed as evicted by the backend.
    pub evictions: u64,
    /// Entries rejected before insertion because encoded bytes exceeded
    /// `max_entry_bytes`.
    pub oversize_rejections: u64,
    /// Cache events delivered to at least one subscriber.
    pub events_published: u64,
    /// Event notifications skipped by slow subscribers.
    pub event_subscriber_lagged: u64,
    /// Invalidation messages published to an attached bus.
    pub distributed_invalidations_published: u64,
    /// Invalidation messages received from an attached bus.
    pub distributed_invalidations_received: u64,
    /// Received invalidation messages applied to the local cache.
    pub distributed_invalidations_applied: u64,
    /// Invalidation messages skipped because a bus receiver lagged behind.
    pub distributed_invalidation_lagged: u64,
    /// Invalidation transport frames that could not be decoded.
    pub distributed_invalidation_decode_errors: u64,
    /// Invalidation publish attempts that returned an error.
    pub distributed_invalidation_publish_failures: u64,
    /// Times an attached bus receiver reported that the stream closed.
    pub distributed_invalidation_receiver_closed: u64,
    /// Convenience value equal to `hits + misses`.
    pub total_requests: u64,
    /// Convenience value equal to `hits / (hits + misses)`.
    pub hit_ratio: Option<f64>,
    /// Whether at least one caller joined an existing single-flight load.
    pub single_flight_active: bool,
    /// Whether at least one stale loader result was discarded.
    pub stale_load_discards_seen: bool,
    /// Whether loader circuit-breaker activity was observed.
    pub load_breaker_active: bool,
    /// Whether at least one event subscriber lagged behind the event bus.
    pub event_subscriber_lag_seen: bool,
    /// Whether this cache published or received bus invalidations.
    pub distributed_invalidation_active: bool,
    /// Whether this cache observed invalidation bus health issues.
    pub distributed_invalidation_bus_issues: bool,
}

impl CacheStatsSnapshot {
    /// Build a serializable snapshot from runtime counters.
    pub fn from_stats(stats: CacheStats) -> Self {
        Self {
            hits: stats.hits,
            misses: stats.misses,
            loads: stats.loads,
            single_flight_joins: stats.single_flight_joins,
            stale_load_discards: stats.stale_load_discards,
            load_breaker_open_total: stats.load_breaker_open_total,
            load_breaker_half_open_total: stats.load_breaker_half_open_total,
            load_breaker_recovered_total: stats.load_breaker_recovered_total,
            load_breaker_rejected_total: stats.load_breaker_rejected_total,
            invalidations: stats.invalidations,
            evictions: stats.evictions,
            oversize_rejections: stats.oversize_rejections,
            events_published: stats.events_published,
            event_subscriber_lagged: stats.event_subscriber_lagged,
            distributed_invalidations_published: stats.distributed_invalidations_published,
            distributed_invalidations_received: stats.distributed_invalidations_received,
            distributed_invalidations_applied: stats.distributed_invalidations_applied,
            distributed_invalidation_lagged: stats.distributed_invalidation_lagged,
            distributed_invalidation_decode_errors: stats.distributed_invalidation_decode_errors,
            distributed_invalidation_publish_failures: stats
                .distributed_invalidation_publish_failures,
            distributed_invalidation_receiver_closed: stats
                .distributed_invalidation_receiver_closed,
            total_requests: stats.total_requests(),
            hit_ratio: stats.hit_ratio(),
            single_flight_active: stats.has_single_flight_activity(),
            stale_load_discards_seen: stats.has_stale_load_discards(),
            load_breaker_active: stats.has_load_breaker_activity(),
            event_subscriber_lag_seen: stats.has_event_subscriber_lag(),
            distributed_invalidation_active: stats.has_distributed_invalidation_activity(),
            distributed_invalidation_bus_issues: stats.has_distributed_invalidation_bus_issues(),
        }
    }
}

impl From<CacheStats> for CacheStatsSnapshot {
    fn from(stats: CacheStats) -> Self {
        Self::from_stats(stats)
    }
}

/// Serializable named diagnostic snapshot for one HydraCache instance.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CacheDiagnosticsSnapshot {
    /// Cache name inside the registry.
    pub name: String,
    /// Serializable stats snapshot.
    pub stats: CacheStatsSnapshot,
    /// Approximate number of entries currently held by the local backend.
    pub estimated_entries: u64,
    /// Whether the local backend currently appears empty.
    pub empty: bool,
}

impl CacheDiagnosticsSnapshot {
    /// Build a named snapshot from runtime diagnostics.
    pub fn from_diagnostics(name: impl Into<String>, diagnostics: CacheDiagnostics) -> Self {
        Self {
            name: name.into(),
            stats: CacheStatsSnapshot::from_stats(diagnostics.stats),
            estimated_entries: diagnostics.estimated_entries,
            empty: diagnostics.is_empty(),
        }
    }

    /// Return the number of lookup attempts represented by this snapshot.
    pub fn total_requests(&self) -> u64 {
        self.stats.total_requests
    }

    /// Return the hit ratio represented by this snapshot.
    pub fn hit_ratio(&self) -> Option<f64> {
        self.stats.hit_ratio
    }
}

/// Serializable overview of all registered caches.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct HydraCacheOverview {
    /// One diagnostic snapshot per registered cache.
    pub caches: Vec<CacheDiagnosticsSnapshot>,
    /// Admission controller snapshot for the served runtime.
    pub admission: AdmissionOverview,
    /// Aggregate grid counters across the served runtime.
    pub cluster_grid: ClusterGridCounters,
    /// Honest topology status used by management and metrics surfaces.
    pub topology: ClusterTopologyOverview,
    /// Worst backup/checkpoint age in seconds, if any snapshot exists.
    pub backup_age_seconds: Option<u64>,
}

/// Serializable admission snapshot used by operator metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct AdmissionOverview {
    /// Current admitted operation count.
    pub in_flight: u64,
    /// Current admitted bytes.
    pub memory_bytes: u64,
    /// Waiting FIFO backlog depth.
    pub queue_depth: u64,
    /// Total rejected operations.
    pub rejected_total: u64,
}

impl From<AdmissionSnapshot> for AdmissionOverview {
    fn from(snapshot: AdmissionSnapshot) -> Self {
        Self {
            in_flight: snapshot.in_flight as u64,
            memory_bytes: snapshot.memory_bytes as u64,
            queue_depth: snapshot.queue_depth as u64,
            rejected_total: snapshot.rejected_total,
        }
    }
}

/// Whether a topology reading is live or modeled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TopologyStatusSource {
    /// Status came from a live grid/control-plane handle.
    Live,
    /// Status is modeled and must not be rendered as live.
    #[default]
    Modeled,
}

impl TopologyStatusSource {
    /// Stable Prometheus label value.
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Modeled => "modeled",
        }
    }
}

/// Bounded reshard phase for topology gauges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TopologyReshardPhase {
    /// No reshard is active.
    #[default]
    Idle,
    /// A reshard is being planned.
    Planning,
    /// Partitions or replicas are moving.
    Moving,
    /// The reshard is finalizing.
    Finalizing,
}

impl TopologyReshardPhase {
    /// Stable Prometheus label value.
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Planning => "planning",
            Self::Moving => "moving",
            Self::Finalizing => "finalizing",
        }
    }
}

/// Honest topology snapshot used by Management Center metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClusterTopologyOverview {
    /// Whether the topology values are live or modeled.
    pub source: TopologyStatusSource,
    /// Visible member count.
    pub members: u64,
    /// Current leader id, if known.
    pub leader: Option<String>,
    /// Current control-plane epoch.
    pub epoch: u64,
    /// Current bounded reshard phase.
    pub reshard_phase: TopologyReshardPhase,
}

impl ClusterTopologyOverview {
    /// Build a topology snapshot.
    pub fn new(
        source: TopologyStatusSource,
        members: u64,
        leader: Option<String>,
        epoch: u64,
        reshard_phase: TopologyReshardPhase,
    ) -> Self {
        Self {
            source,
            members,
            leader,
            epoch,
            reshard_phase,
        }
    }
}

impl Default for ClusterTopologyOverview {
    fn default() -> Self {
        Self {
            source: TopologyStatusSource::Modeled,
            members: 0,
            leader: None,
            epoch: 0,
            reshard_phase: TopologyReshardPhase::Idle,
        }
    }
}

/// Read-only cluster snapshot for the Management Center.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClusterOverview {
    /// Whether the snapshot is live or modeled.
    pub source: TopologyStatusSource,
    /// Visible members. Unreachable members remain present.
    pub members: Vec<ClusterMemberView>,
    /// Current leader, if a live source knows one.
    pub leader: Option<LeaderView>,
    /// Aggregate partition health.
    pub partitions: PartitionSummary,
    /// Configured and observed consistency-level summary.
    pub consistency: ConsistencyView,
    /// Oldest known backup/checkpoint age in seconds.
    pub backup_age_seconds: Option<u64>,
    /// Current cluster lifecycle phases.
    pub lifecycle: LifecycleView,
}

impl ClusterOverview {
    /// Build a read-only cluster overview.
    pub fn new(
        source: TopologyStatusSource,
        members: Vec<ClusterMemberView>,
        leader: Option<LeaderView>,
        partitions: PartitionSummary,
        consistency: ConsistencyView,
        backup_age_seconds: Option<u64>,
        lifecycle: LifecycleView,
    ) -> Self {
        Self {
            source,
            members,
            leader,
            partitions,
            consistency,
            backup_age_seconds,
            lifecycle,
        }
    }
}

/// Per-member read model rendered by the Management Center.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClusterMemberView {
    /// Stable logical member id.
    pub node_id: String,
    /// Runtime role label.
    pub role: String,
    /// Whether the member is currently reachable.
    pub reachable: bool,
    /// Full reachability label, preserving suspect vs unreachable.
    pub reachability: String,
    /// Process generation reported by the cluster-status source.
    pub generation: u64,
}

impl ClusterMemberView {
    /// Build a member view.
    pub fn new(
        node_id: impl Into<String>,
        role: impl Into<String>,
        reachable: bool,
        reachability: impl Into<String>,
        generation: u64,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            role: role.into(),
            reachable,
            reachability: reachability.into(),
            generation,
        }
    }
}

/// Leader view carried only when the leader is known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeaderView {
    /// Stable logical member id.
    pub node_id: String,
    /// Current control-plane term.
    pub term: u64,
    /// Current authority epoch.
    pub epoch: u64,
}

impl LeaderView {
    /// Build a leader view.
    pub fn new(node_id: impl Into<String>, term: u64, epoch: u64) -> Self {
        Self {
            node_id: node_id.into(),
            term,
            epoch,
        }
    }
}

/// Aggregate partition status for the cluster overview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct PartitionSummary {
    /// Aggregate under-replicated partition/key count.
    pub under_replicated: u64,
    /// Effective partition count, or zero when no live map is attached yet.
    pub count: u64,
}

impl PartitionSummary {
    /// Build partition status from grid counters and the current effective map size.
    pub fn from_grid_counters(counters: ClusterGridCounters, count: u64) -> Self {
        Self {
            under_replicated: counters.under_replicated_keys,
            count,
        }
    }
}

/// Consistency-level summary for the cluster overview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct ConsistencyView {
    /// Configured default consistency label, if the host exposes one.
    pub configured_default: Option<String>,
    /// Observed per-level operation counts.
    ///
    /// Current counters only expose an aggregate total, so the server uses an
    /// explicit `aggregate` bucket until a per-level source is wired.
    pub op_counts_by_level: Vec<ConsistencyLevelCount>,
}

impl ConsistencyView {
    /// Build a consistency view from aggregate grid counters.
    pub fn from_grid_counters(
        configured_default: Option<String>,
        counters: ClusterGridCounters,
    ) -> Self {
        let mut op_counts_by_level = Vec::new();
        if counters.consistency_level_operations_total > 0 {
            op_counts_by_level.push(ConsistencyLevelCount {
                level: "aggregate".to_owned(),
                count: counters.consistency_level_operations_total,
            });
        }
        Self {
            configured_default,
            op_counts_by_level,
        }
    }
}

/// One consistency-level counter bucket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConsistencyLevelCount {
    /// Consistency level label, or `aggregate` when only aggregate data exists.
    pub level: String,
    /// Operation count for the level.
    pub count: u64,
}

/// Cluster lifecycle phases rendered by the Management Center.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleView {
    /// Current reshard phase.
    pub reshard_phase: String,
    /// Current graceful-upgrade phase.
    pub upgrade_phase: String,
}

impl LifecycleView {
    /// Build lifecycle status.
    pub fn new(reshard_phase: impl Into<String>, upgrade_phase: impl Into<String>) -> Self {
        Self {
            reshard_phase: reshard_phase.into(),
            upgrade_phase: upgrade_phase.into(),
        }
    }
}

/// Per-member status entry for the grid operator surface.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MemberStatus {
    /// Logical member id.
    pub node_id: ClusterNodeId,
    /// Whether the member is currently reachable according to diagnostics.
    pub reachable: bool,
}

/// Read-only production-grid status snapshot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClusterStatus {
    /// Committed authority epoch.
    pub committed_epoch: ClusterEpoch,
    /// Current raft/grid leader when known.
    pub leader: Option<ClusterNodeId>,
    /// Members in the committed topology.
    pub members: Vec<MemberStatus>,
    /// Aggregate under-replicated partition/key count.
    pub partitions_under_replicated: u64,
    /// Aggregate tombstone/repair debt.
    pub repair_debt: u64,
    /// Strong or degraded read-your-writes posture.
    pub quorum_posture: QuorumPosture,
    /// Last split-brain report retained in diagnostics.
    pub last_split_brain: Option<SplitBrainReport>,
    /// Prominent non-goal surfaced to operators.
    pub still_not_distributed_transactions: bool,
}

impl ClusterStatus {
    /// Build a status snapshot from bounded diagnostics.
    pub fn from_grid_diagnostics(
        committed_epoch: ClusterEpoch,
        leader: Option<ClusterNodeId>,
        members: Vec<MemberStatus>,
        grid: ClusterGridDiagnostics,
        quorum_posture: QuorumPosture,
    ) -> Self {
        Self {
            committed_epoch,
            leader,
            members,
            partitions_under_replicated: grid.counters.under_replicated_keys,
            repair_debt: grid.counters.tombstone_repair_debt,
            quorum_posture,
            last_split_brain: grid.last_split_brain,
            still_not_distributed_transactions: true,
        }
    }
}

/// Per-region active-active health used by `/cluster/geo` style endpoints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegionHealth {
    /// Region id.
    pub region: RegionId,
    /// Region failover state.
    pub state: RegionState,
    /// Aggregated replication lag observed for this region.
    pub replication_lag: u64,
    /// Observed bounded-staleness window in milliseconds.
    pub staleness_window_ms: u64,
}

impl RegionHealth {
    /// Create a region health entry.
    pub fn new(
        region: impl Into<RegionId>,
        state: RegionState,
        replication_lag: u64,
        staleness_window_ms: u64,
    ) -> Self {
        Self {
            region: region.into(),
            state,
            replication_lag,
            staleness_window_ms,
        }
    }
}

/// Per-link WAN replication health used by geo status.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LinkHealth {
    /// Source region.
    pub from: RegionId,
    /// Target region.
    pub to: RegionId,
    /// Lag in queued batches/records, depending on exporter resolution.
    pub lag: u64,
    /// Current adaptive replication window.
    pub window: usize,
    /// Compressed bytes replicated across this link.
    pub bytes_total: u64,
    /// Whether the link is currently under backpressure.
    pub backpressured: bool,
}

impl LinkHealth {
    /// Create a link health entry.
    pub fn new(
        from: impl Into<RegionId>,
        to: impl Into<RegionId>,
        lag: u64,
        window: usize,
        bytes_total: u64,
        backpressured: bool,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            lag,
            window,
            bytes_total,
            backpressured,
        }
    }

    /// Return a bounded label for this region link.
    pub fn link_label(&self) -> String {
        format!("{}->{}", self.from.as_str(), self.to.as_str())
    }
}

/// Staleness SLO definition for active-active geo status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GeoStalenessSlo {
    /// Target maximum observed staleness window.
    pub target_window_ms: u64,
}

impl GeoStalenessSlo {
    /// Create an SLO with a normalized non-zero target.
    pub fn new(target_window_ms: u64) -> Self {
        Self {
            target_window_ms: target_window_ms.max(1),
        }
    }

    /// Evaluate one status snapshot.
    pub fn evaluate(&self, status: &GeoStatus) -> GeoSloEvaluation {
        GeoSloEvaluation {
            target_window_ms: self.target_window_ms,
            observed_window_ms: status.worst_staleness_window_ms,
            breached: status.worst_staleness_window_ms > self.target_window_ms,
        }
    }
}

/// Result of evaluating a geo staleness SLO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GeoSloEvaluation {
    /// Target maximum staleness window.
    pub target_window_ms: u64,
    /// Observed worst region staleness window.
    pub observed_window_ms: u64,
    /// Whether the SLO is breached.
    pub breached: bool,
}

/// Read-only active-active geo status snapshot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeoStatus {
    /// Per-region state, lag, and staleness.
    pub regions: Vec<RegionHealth>,
    /// Per-WAN-link replication health.
    pub links: Vec<LinkHealth>,
    /// Whether active-active was explicitly acknowledged.
    pub active_active_acked: bool,
    /// Worst observed staleness window across regions.
    pub worst_staleness_window_ms: u64,
    /// CRDT metadata size retained for convergence/GC safety.
    pub crdt_metadata_bytes: u64,
    /// Configured staleness SLO target.
    pub staleness_slo_target_ms: u64,
    /// Whether the staleness SLO is breached.
    pub staleness_slo_breached: bool,
}

impl GeoStatus {
    /// Assemble a read-only geo status snapshot from bounded signals.
    pub fn from_signals(
        mut regions: Vec<RegionHealth>,
        mut links: Vec<LinkHealth>,
        active_active_acked: bool,
        crdt_metadata_bytes: u64,
        slo: GeoStalenessSlo,
    ) -> Self {
        regions.sort_by(|left, right| left.region.as_str().cmp(right.region.as_str()));
        links.sort_by_key(LinkHealth::link_label);
        let worst_staleness_window_ms = regions
            .iter()
            .map(|region| region.staleness_window_ms)
            .max()
            .unwrap_or_default();
        let staleness_slo_breached = worst_staleness_window_ms > slo.target_window_ms;
        Self {
            regions,
            links,
            active_active_acked,
            worst_staleness_window_ms,
            crdt_metadata_bytes,
            staleness_slo_target_ms: slo.target_window_ms,
            staleness_slo_breached,
        }
    }

    /// Return whether every region is up and no SLO is breached.
    pub fn is_healthy(&self) -> bool {
        self.active_active_acked
            && !self.staleness_slo_breached
            && self
                .regions
                .iter()
                .all(|region| region.state == RegionState::Up)
    }
}

/// Aggregate causal+ session status for read-only operator endpoints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionStats {
    /// Active session count. This is an aggregate gauge, never a session-id label.
    pub active_sessions: u64,
    /// P99 retained watermark entries across active sessions.
    pub p99_watermark_entries: u64,
    /// Ratio of guarantee-unmet events to session reads represented by this snapshot.
    pub guarantee_unmet_rate: f64,
    /// Worst observed session staleness bound.
    pub worst_session_staleness: StalenessBound,
    /// Current retained watermark entries.
    pub watermark_entries: u64,
    /// RYW escalations observed by the session layer.
    pub read_your_writes_escalations: u64,
    /// Causal writes currently deferred/missing dependencies.
    pub causal_writes_deferred: u64,
}

impl SessionStats {
    /// Build session stats from bounded grid counters.
    pub fn from_grid_counters(counters: ClusterGridCounters, total_session_reads: u64) -> Self {
        let guarantee_unmet_rate = if total_session_reads == 0 {
            0.0
        } else {
            counters.session_guarantee_unmet_total as f64 / total_session_reads as f64
        };
        Self {
            active_sessions: counters.session_active_sessions,
            p99_watermark_entries: counters.session_watermark_entries_p99,
            guarantee_unmet_rate,
            worst_session_staleness: StalenessBound::versions(
                counters.session_worst_staleness_versions,
            ),
            watermark_entries: counters.session_watermark_entries,
            read_your_writes_escalations: counters.session_ryw_escalations_total,
            causal_writes_deferred: counters.causal_writes_deferred_total,
        }
    }

    /// Return whether session guarantees currently look healthy.
    pub fn is_healthy(&self) -> bool {
        self.guarantee_unmet_rate == 0.0 && self.causal_writes_deferred == 0
    }
}

/// Metric names that make up the session observability surface.
pub fn session_metric_names() -> &'static [&'static str] {
    &[
        "hydracache_session_active_sessions",
        "hydracache_session_watermark_entries",
        "hydracache_session_watermark_entries_p99",
        "hydracache_session_worst_staleness_versions",
        "hydracache_session_watermark_coarsened_total",
        "hydracache_session_token_rejected_total",
        "hydracache_session_ryw_escalations_total",
        "hydracache_session_guarantee_unmet_total",
        "hydracache_monotonic_read_violations_prevented_total",
        "hydracache_monotonic_write_reorders_prevented_total",
        "hydracache_causal_writes_deferred_total",
        "hydracache_causal_summary_coarsened_total",
        "hydracache_causal_dependency_bytes",
        "hydracache_bounded_staleness_fast_serves_total",
        "hydracache_bounded_staleness_escalations_total",
    ]
}

/// Session alert metrics that must stay registered in the grid descriptor catalog.
pub fn session_alert_metric_names() -> &'static [&'static str] {
    &[
        "hydracache_session_token_rejected_total",
        "hydracache_session_guarantee_unmet_total",
        "hydracache_causal_writes_deferred_total",
        "hydracache_session_worst_staleness_versions",
    ]
}

/// Geo metric names shipped with the alert and dashboard artifacts.
pub fn geo_metric_names() -> &'static [&'static str] {
    &[
        "hydracache_region_staleness_window_ms",
        "hydracache_region_link_lag",
        "hydracache_region_link_bytes_total",
        "hydracache_region_link_window",
        "hydracache_region_state",
        "hydracache_region_restore_duration_ms",
        "hydracache_crdt_metadata_bytes",
    ]
}

/// Repair-debt degraded mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairDebtMode {
    /// Normal replication admission.
    Healthy,
    /// Replication admission should throttle and anti-entropy should be prioritized.
    Degraded,
}

/// Threshold-driven repair-debt controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairDebtController {
    threshold: u64,
}

impl RepairDebtController {
    /// Create a controller with a normalized threshold.
    pub fn new(threshold: u64) -> Self {
        Self {
            threshold: threshold.max(1),
        }
    }

    /// Observe diagnostics and return the resulting mode.
    pub fn observe(&self, diagnostics: &ClusterGridDiagnostics) -> RepairDebtMode {
        if diagnostics.counters.tombstone_repair_debt >= self.threshold {
            RepairDebtMode::Degraded
        } else {
            RepairDebtMode::Healthy
        }
    }
}

impl HydraCacheOverview {
    /// Build an overview from cache diagnostics with default zero operator signals.
    pub fn new(caches: Vec<CacheDiagnosticsSnapshot>) -> Self {
        Self {
            caches,
            ..Self::default()
        }
    }

    /// Attach an admission snapshot.
    pub fn with_admission_snapshot(mut self, admission: AdmissionSnapshot) -> Self {
        self.admission = AdmissionOverview::from(admission);
        self
    }

    /// Attach aggregate cluster-grid counters.
    pub fn with_cluster_grid_counters(mut self, counters: ClusterGridCounters) -> Self {
        self.cluster_grid = counters;
        self
    }

    /// Attach an honest topology snapshot.
    pub fn with_topology(mut self, topology: ClusterTopologyOverview) -> Self {
        self.topology = topology;
        self
    }

    /// Attach the worst known backup age.
    pub fn with_backup_age_seconds(mut self, seconds: u64) -> Self {
        self.backup_age_seconds = Some(seconds);
        self
    }

    /// Return the number of caches represented by this overview.
    pub fn cache_count(&self) -> usize {
        self.caches.len()
    }

    /// Return whether no cache is registered.
    pub fn is_empty(&self) -> bool {
        self.caches.is_empty()
    }
}

/// Named diagnostic source stored inside [`HydraCacheRegistry`].
#[async_trait]
pub trait CacheProbe: Send + Sync {
    /// Return the stable registry name for this cache.
    fn name(&self) -> &str;

    /// Return a serializable diagnostic snapshot.
    async fn diagnostics(&self) -> CacheDiagnosticsSnapshot;

    /// Return cluster staging health when this probe wraps a cluster cache.
    fn cluster_staging_health(&self) -> Option<ClusterStagingHealth>;

    /// Return aggregate grid counters for this probe.
    fn cluster_grid_counters(&self) -> ClusterGridCounters {
        ClusterGridCounters::default()
    }

    /// Return the pilot report for this probe when available.
    fn cluster_pilot_report(&self) -> Option<ClusterPilotReport> {
        None
    }
}

/// [`HydraCache`] implementation of [`CacheProbe`].
#[derive(Debug, Clone)]
pub struct HydraCacheProbe<C = PostcardCodec>
where
    C: CacheCodec,
{
    name: String,
    cache: HydraCache<C>,
}

impl<C> HydraCacheProbe<C>
where
    C: CacheCodec,
{
    /// Create a named probe for a cache instance.
    pub fn new(name: impl Into<String>, cache: HydraCache<C>) -> Self {
        Self {
            name: name.into(),
            cache,
        }
    }

    /// Return the underlying cache handle.
    pub fn cache(&self) -> &HydraCache<C> {
        &self.cache
    }
}

#[async_trait]
impl<C> CacheProbe for HydraCacheProbe<C>
where
    C: CacheCodec,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn diagnostics(&self) -> CacheDiagnosticsSnapshot {
        CacheDiagnosticsSnapshot::from_diagnostics(
            self.name.clone(),
            self.cache.diagnostics().await,
        )
    }

    fn cluster_staging_health(&self) -> Option<ClusterStagingHealth> {
        self.cache.cluster_staging_health()
    }

    fn cluster_grid_counters(&self) -> ClusterGridCounters {
        self.cache.cluster_grid_counters()
    }

    fn cluster_pilot_report(&self) -> Option<ClusterPilotReport> {
        Some(self.cache.cluster_pilot_report())
    }
}

/// Registry of named HydraCache instances.
///
/// The registry is intentionally read-only from an observability perspective:
/// it can produce snapshots, but it cannot mutate cache contents. HTTP adapters
/// can safely build read-only actuator endpoints on top of it.
#[derive(Clone, Default)]
pub struct HydraCacheRegistry {
    probes: BTreeMap<String, Arc<dyn CacheProbe>>,
    admission: AdmissionOverview,
    cluster_grid: ClusterGridCounters,
    topology: ClusterTopologyOverview,
    backup_age_seconds: Option<u64>,
}

impl HydraCacheRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a cache and return the updated registry.
    ///
    /// This builder-style method is handy when wiring an actuator in app setup.
    pub fn with_cache<C>(mut self, name: impl Into<String>, cache: HydraCache<C>) -> Self
    where
        C: CacheCodec,
    {
        self.insert_cache(name, cache);
        self
    }

    /// Attach the current admission snapshot.
    pub fn with_admission_snapshot(mut self, admission: AdmissionSnapshot) -> Self {
        self.admission = AdmissionOverview::from(admission);
        self
    }

    /// Attach aggregate grid counters supplied by the hosting runtime.
    pub fn with_cluster_grid_counters(mut self, counters: ClusterGridCounters) -> Self {
        self.cluster_grid = add_cluster_grid_counters(self.cluster_grid, counters);
        self
    }

    /// Attach an honest topology snapshot.
    pub fn with_topology(mut self, topology: ClusterTopologyOverview) -> Self {
        self.topology = topology;
        self
    }

    /// Attach the worst known backup age.
    pub fn with_backup_age_seconds(mut self, seconds: u64) -> Self {
        self.backup_age_seconds = Some(seconds);
        self
    }

    /// Register or replace a named cache.
    pub fn insert_cache<C>(
        &mut self,
        name: impl Into<String>,
        cache: HydraCache<C>,
    ) -> Option<Arc<dyn CacheProbe>>
    where
        C: CacheCodec,
    {
        self.insert_probe(HydraCacheProbe::new(name, cache))
    }

    /// Register or replace a custom probe.
    pub fn insert_probe<P>(&mut self, probe: P) -> Option<Arc<dyn CacheProbe>>
    where
        P: CacheProbe + 'static,
    {
        self.probes.insert(probe.name().to_owned(), Arc::new(probe))
    }

    /// Return the number of registered caches.
    pub fn len(&self) -> usize {
        self.probes.len()
    }

    /// Return whether the registry has no caches.
    pub fn is_empty(&self) -> bool {
        self.probes.is_empty()
    }

    /// Return registered cache names in stable sorted order.
    pub fn cache_names(&self) -> Vec<String> {
        self.probes.keys().cloned().collect()
    }

    /// Return a diagnostic snapshot for one registered cache.
    pub async fn diagnostics(&self, name: &str) -> Option<CacheDiagnosticsSnapshot> {
        let probe = self.probes.get(name)?;
        Some(probe.diagnostics().await)
    }

    /// Return cluster staging health for one registered cache.
    pub fn cluster_staging_health(&self, name: &str) -> Option<ClusterStagingHealth> {
        self.probes.get(name)?.cluster_staging_health()
    }

    /// Return staging health for every registered cluster cache.
    pub fn cluster_staging_healths(&self) -> Vec<(String, ClusterStagingHealth)> {
        self.probes
            .iter()
            .filter_map(|(name, probe)| {
                probe
                    .cluster_staging_health()
                    .map(|health| (name.clone(), health))
            })
            .collect()
    }

    /// Return pilot reports for every registered cache.
    pub fn cluster_pilot_reports(&self) -> Vec<(String, ClusterPilotReport)> {
        self.probes
            .iter()
            .filter_map(|(name, probe)| {
                probe
                    .cluster_pilot_report()
                    .map(|report| (name.clone(), report))
            })
            .collect()
    }

    /// Return diagnostic snapshots for all registered caches.
    pub async fn overview(&self) -> HydraCacheOverview {
        let mut caches = Vec::with_capacity(self.probes.len());
        let mut cluster_grid = self.cluster_grid;
        for probe in self.probes.values() {
            caches.push(probe.diagnostics().await);
            cluster_grid = add_cluster_grid_counters(cluster_grid, probe.cluster_grid_counters());
        }
        HydraCacheOverview {
            caches,
            admission: self.admission,
            cluster_grid,
            topology: self.topology.clone(),
            backup_age_seconds: self.backup_age_seconds,
        }
    }
}

macro_rules! add_cluster_counter_fields {
    ($left:ident, $right:ident, [$($field:ident),+ $(,)?]) => {
        $(
            $left.$field = $left.$field.saturating_add($right.$field);
        )+
    };
}

fn add_cluster_grid_counters(
    mut left: ClusterGridCounters,
    right: ClusterGridCounters,
) -> ClusterGridCounters {
    add_cluster_counter_fields!(
        left,
        right,
        [
            replication_success_total,
            replication_failure_total,
            bytes_replicated_total,
            replication_backpressure_total,
            replication_oversized_rejected_total,
            replication_decrypt_failure_total,
            under_replicated_keys,
            failover_total,
            repair_task_total,
            repair_failure_total,
            rebalance_plan_total,
            rebalance_task_ack_total,
            topology_fence_rejected_total,
            tombstone_repair_debt,
            replicated_value_rejected_total,
            split_brain_detected_total,
            merge_discarded_entries_total,
            merge_unresolved_conflicts_total,
            cluster_auth_rejected_total,
            repair_debt_degraded_mode,
            placement_zone_underspread,
            reshard_moves_inflight,
            reshard_backfill_lag,
            read_local_zone_total,
            read_hedged_total,
            read_hedge_win_total,
            value_tier_promotions_total,
            value_tier_demotions_total,
            invalidate_batch_total,
            invalidation_saga_pending,
            auto_repair_active_total,
            auto_repair_advisory_total,
            consistency_level_operations_total,
            consistency_unsatisfiable_total,
            hints_stored_total,
            hints_replayed_total,
            hints_dropped_total,
            hint_store_bytes,
            repair_ranges_exchanged_total,
            read_repair_total,
            repair_progress_ratio,
            peer_phi_scaled,
            false_suspect_total,
            cas_applied_total,
            cas_mismatch_total,
            lock_acquired_total,
            lock_stale_token_rejected_total,
            invalidation_ring_depth,
            invalidation_replayed_total,
            invalidation_fell_behind_total,
            invalidation_ring_overrun_total,
            session_watermark_entries,
            session_active_sessions,
            session_watermark_entries_p99,
            session_worst_staleness_versions,
            session_watermark_coarsened_total,
            session_token_rejected_total,
            session_ryw_escalations_total,
            session_guarantee_unmet_total,
            monotonic_read_violations_prevented_total,
            monotonic_write_reorders_prevented_total,
            causal_writes_deferred_total,
            causal_summary_coarsened_total,
            causal_dependency_bytes,
            bounded_staleness_fast_serves_total,
            bounded_staleness_escalations_total,
        ]
    );
    left
}

impl fmt::Debug for HydraCacheRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HydraCacheRegistry")
            .field("cache_names", &self.cache_names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hydracache::{
        CacheOptions, ClusterGeneration, ClusterHealthState, HydraCache, InMemoryCluster,
    };
    use serde_json::Value;

    use super::{
        CacheDiagnosticsSnapshot, CacheStatsSnapshot, HydraCacheOverview, HydraCacheProbe,
        HydraCacheRegistry,
    };

    #[tokio::test]
    async fn registry_reports_named_cache_diagnostics() {
        let cache = HydraCache::local().build();
        cache
            .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
            .await
            .unwrap();
        cache
            .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
            .await
            .unwrap();

        let registry = HydraCacheRegistry::new().with_cache("main", cache);
        let diagnostics = registry.diagnostics("main").await.unwrap();

        assert_eq!(registry.cache_names(), vec!["main".to_owned()]);
        assert_eq!(diagnostics.name, "main");
        assert_eq!(diagnostics.stats.loads, 1);
        assert_eq!(diagnostics.stats.hits, 1);
        assert_eq!(diagnostics.total_requests(), 2);
        assert_eq!(diagnostics.hit_ratio(), Some(0.5));
        assert!(!diagnostics.empty);
        assert!(registry.diagnostics("missing").await.is_none());
    }

    #[tokio::test]
    async fn overview_returns_sorted_cache_snapshots() {
        let first = HydraCache::local().build();
        let second = HydraCache::local().build();

        first.put("one", 1_u64, CacheOptions::new()).await.unwrap();
        second.put("two", 2_u64, CacheOptions::new()).await.unwrap();

        let registry = HydraCacheRegistry::new()
            .with_cache("zeta", second)
            .with_cache("alpha", first);
        let overview = registry.overview().await;

        assert_eq!(overview.cache_count(), 2);
        assert!(!overview.is_empty());
        assert_eq!(overview.caches[0].name, "alpha");
        assert_eq!(overview.caches[1].name, "zeta");
    }

    #[tokio::test]
    async fn insert_cache_replaces_existing_probe() {
        let mut registry = HydraCacheRegistry::new();

        assert!(registry
            .insert_cache("main", HydraCache::local().build())
            .is_none());
        assert!(registry
            .insert_cache("main", HydraCache::local().build())
            .is_some());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(format!("{registry:?}").contains("main"));
    }

    #[tokio::test]
    async fn registry_exposes_cluster_staging_health_for_cluster_caches() {
        let cluster = Arc::new(InMemoryCluster::new("orders"));
        let member = HydraCache::member()
            .shared_cluster(cluster)
            .node_id("member-a")
            .generation(ClusterGeneration::new(3))
            .start()
            .await
            .unwrap();
        member.record_cluster_owner_load_success();
        member.record_cluster_remote_fetch_success();
        member.record_cluster_hot_cache_hit();

        let registry = HydraCacheRegistry::new()
            .with_cache("local", HydraCache::local().build())
            .with_cache("member", member);

        assert!(registry.cluster_staging_health("local").is_none());

        let health = registry
            .cluster_staging_health("member")
            .expect("member staging health");
        assert_eq!(health.state, ClusterHealthState::Healthy);
        assert_eq!(health.node_id, "member-a");
        assert_eq!(health.generation, 3);
        assert_eq!(health.owner_load_success, 1);
        assert_eq!(health.remote_fetch_success, 1);
        assert_eq!(health.hot_cache_hits, 1);

        let all = registry.cluster_staging_healths();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "member");
    }

    #[test]
    fn stats_snapshot_contains_computed_values() {
        let stats = hydracache_core::CacheStats {
            hits: 2,
            misses: 1,
            single_flight_joins: 1,
            stale_load_discards: 1,
            load_breaker_open_total: 1,
            load_breaker_half_open_total: 1,
            load_breaker_recovered_total: 1,
            load_breaker_rejected_total: 1,
            distributed_invalidations_received: 1,
            distributed_invalidation_lagged: 1,
            distributed_invalidation_decode_errors: 1,
            distributed_invalidation_publish_failures: 1,
            distributed_invalidation_receiver_closed: 1,
            ..hydracache_core::CacheStats::default()
        };

        let snapshot = CacheStatsSnapshot::from_stats(stats);

        assert_eq!(snapshot.total_requests, 3);
        assert_eq!(snapshot.hit_ratio, Some(2.0 / 3.0));
        assert!(snapshot.single_flight_active);
        assert!(snapshot.stale_load_discards_seen);
        assert!(snapshot.load_breaker_active);
        assert!(!snapshot.event_subscriber_lag_seen);
        assert!(snapshot.distributed_invalidation_active);
        assert!(snapshot.distributed_invalidation_bus_issues);
        assert_eq!(snapshot.distributed_invalidations_received, 1);
        assert_eq!(snapshot.distributed_invalidation_lagged, 1);
        assert_eq!(snapshot.distributed_invalidation_decode_errors, 1);
        assert_eq!(snapshot.distributed_invalidation_publish_failures, 1);
        assert_eq!(snapshot.distributed_invalidation_receiver_closed, 1);

        let via_from: CacheStatsSnapshot = stats.into();
        assert_eq!(via_from.total_requests, 3);
    }

    #[test]
    fn serializable_snapshot_contract_contains_required_fields() {
        let stats = CacheStatsSnapshot::from_stats(hydracache_core::CacheStats {
            hits: 2,
            misses: 1,
            loads: 1,
            single_flight_joins: 1,
            stale_load_discards: 1,
            load_breaker_open_total: 1,
            load_breaker_rejected_total: 1,
            invalidations: 1,
            events_published: 1,
            distributed_invalidation_publish_failures: 1,
            ..hydracache_core::CacheStats::default()
        });
        let diagnostics = CacheDiagnosticsSnapshot {
            name: "main".to_owned(),
            stats,
            estimated_entries: 1,
            empty: false,
        };
        let overview = HydraCacheOverview::new(vec![diagnostics.clone()]);

        assert_json_fields(
            serde_json::to_value(&diagnostics.stats).unwrap(),
            &[
                "hits",
                "misses",
                "loads",
                "single_flight_joins",
                "stale_load_discards",
                "load_breaker_open_total",
                "load_breaker_half_open_total",
                "load_breaker_recovered_total",
                "load_breaker_rejected_total",
                "invalidations",
                "evictions",
                "oversize_rejections",
                "events_published",
                "event_subscriber_lagged",
                "distributed_invalidations_published",
                "distributed_invalidations_received",
                "distributed_invalidations_applied",
                "distributed_invalidation_lagged",
                "distributed_invalidation_decode_errors",
                "distributed_invalidation_publish_failures",
                "distributed_invalidation_receiver_closed",
                "total_requests",
                "hit_ratio",
                "single_flight_active",
                "stale_load_discards_seen",
                "load_breaker_active",
                "event_subscriber_lag_seen",
                "distributed_invalidation_active",
                "distributed_invalidation_bus_issues",
            ],
        );
        assert_json_fields(
            serde_json::to_value(&diagnostics).unwrap(),
            &["name", "stats", "estimated_entries", "empty"],
        );
        assert_json_fields(
            serde_json::to_value(&overview).unwrap(),
            &[
                "caches",
                "admission",
                "cluster_grid",
                "topology",
                "backup_age_seconds",
            ],
        );
    }

    #[test]
    fn hydra_cache_probe_exposes_underlying_cache_handle() {
        let cache = HydraCache::local().build();
        let probe = HydraCacheProbe::new("main", cache);

        assert_eq!(probe.cache().stats().total_requests(), 0);
    }

    fn assert_json_fields(value: Value, fields: &[&str]) {
        let object = value
            .as_object()
            .expect("snapshot should serialize as object");
        for field in fields {
            assert!(
                object.contains_key(*field),
                "snapshot is missing required field `{field}`"
            );
        }
    }
}
