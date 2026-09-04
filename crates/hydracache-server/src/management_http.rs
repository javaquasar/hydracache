//! Authenticated, bounded read-only Management Center HTTP surface.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use hydracache_client_transport_axum::ClientIdentity;
use hydracache_observability::{
    AdmissionState, BoundedCount, BoundedPage, CatchUpState, ClientProtocolLifecycle,
    ClusterFormationSnapshot, ConsensusProgressSnapshot, DiscoveryState, DurableRecoveryStatus,
    FormationReasonCode, ManagementCacheDetail, ManagementCapabilities, ManagementCapability,
    ManagementCapabilityAvailability, ManagementCapabilityId, ManagementClientProtocol,
    ManagementClientsSnapshot, ManagementCompleteness, ManagementConsensusRole, ManagementEnvelope,
    ManagementMeasurementQuality, ManagementMemberDetail, ManagementNamespaceSummary,
    ManagementObservationSource, ManagementPartitionSnapshot, ManagementWarning,
    ManagementWarningCode, MemberFormationTransition, MemberReachabilityReason, OpaqueCursor,
    OpaqueNodeIdentity, PlacementDecisionTrace, RecoveryArtifactKind, RecoveryOutcome,
    RecoveryPhase, RecoveryReasonCode, RecoveryScope, RepairState, ServingState, TransportState,
    MANAGEMENT_API_SCHEMA_VERSION, MAX_MANAGEMENT_CURSOR_BYTES, MAX_MANAGEMENT_PAGE_ITEMS,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::admin_http::{require_admin, SharedServerRuntime};
use crate::cluster_status::{ClusterStatus, LocalConsensusStatus, Reachability, StatusSource};
use crate::management_aggregation::{
    AggregatedManagementSnapshot, ManagementAggregationIssue, ManagementMemberSnapshot,
    ManagementSnapshotAggregator,
};

/// Management capability negotiation path.
pub const MANAGEMENT_CAPABILITIES_PATH: &str = "/management/v1/capabilities";
/// Bounded typed dashboard snapshot path.
pub const MANAGEMENT_DASHBOARD_PATH: &str = "/management/v1/dashboard";
/// Bounded cluster-formation observations path.
pub const MANAGEMENT_FORMATION_PATH: &str = "/management/v1/formation";
/// Epoch-consistent committed-member detail path.
pub const MANAGEMENT_CLUSTER_MEMBERS_PATH: &str = "/management/v1/cluster/members";
/// Cluster-scoped alias for the formation state machine.
pub const MANAGEMENT_CLUSTER_FORMATION_PATH: &str = "/management/v1/cluster/formation";
/// Honest aggregate partition ownership and repair path.
pub const MANAGEMENT_CLUSTER_PARTITIONS_PATH: &str = "/management/v1/cluster/partitions";
/// Bounded node-local client lifecycle summary path.
pub const MANAGEMENT_CLIENTS_PATH: &str = "/management/v1/clients";
/// Caller-authorized namespace summary path.
pub const MANAGEMENT_NAMESPACES_PATH: &str = "/management/v1/namespaces";
/// Prefix for caller-authorized namespace cache detail.
pub const MANAGEMENT_NAMESPACE_CACHES_PREFIX: &str = "/management/v1/namespaces";
/// Prefix for non-enumerable placement-decision trace resources.
pub const MANAGEMENT_PLACEMENT_TRACE_PREFIX: &str = "/management/v1/cluster/placement-traces";
/// Honest durable-recovery observation path.
pub const MANAGEMENT_RECOVERY_PATH: &str = "/management/v1/persistence/recovery";
/// Local authoritative Raft progress path.
pub const MANAGEMENT_CONSENSUS_PROGRESS_PATH: &str = "/management/v1/consensus/progress";

/// Freshness window attached to local management observations.
pub const MANAGEMENT_STALE_AFTER_MS: u64 = 2_000;
/// Hard byte ceiling for every serialized management response.
pub const MANAGEMENT_MAX_RESPONSE_BYTES: usize = 256 * 1024;
/// Lifetime of an opaque pagination cursor.
pub const MANAGEMENT_CURSOR_TTL_MS: u64 = 30_000;
/// Hard bound for server-side retained cursor records.
pub const MANAGEMENT_MAX_RETAINED_CURSORS: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorKind {
    Formation,
    Members,
    Namespaces,
    ConsensusProgress,
    Recovery,
}

#[derive(Debug, Clone, Copy)]
struct CursorRecord {
    kind: CursorKind,
    authority_epoch: u64,
    observation_seq: u64,
    offset: usize,
    expires_at_unix_ms: u64,
    issued_seq: u64,
}

/// Bounded server-side cursor registry. Tokens expose neither offsets nor source identities.
#[derive(Debug, Default)]
pub(crate) struct ManagementCursorStore {
    entries: BTreeMap<String, CursorRecord>,
    issued_seq: u64,
}

impl ManagementCursorStore {
    fn issue(
        &mut self,
        kind: CursorKind,
        authority_epoch: u64,
        observation_seq: u64,
        offset: usize,
        now_unix_ms: u64,
    ) -> OpaqueCursor {
        self.remove_expired(now_unix_ms);
        self.issued_seq = self.issued_seq.saturating_add(1);
        if self.entries.len() >= MANAGEMENT_MAX_RETAINED_CURSORS {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, record)| record.issued_seq)
                .map(|(token, _)| token.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        let mut hasher = Sha256::new();
        hasher.update(b"hydracache-management-cursor-v1\0");
        hasher.update(format!(
            "{kind:?}:{authority_epoch}:{observation_seq}:{offset}:{}:{now_unix_ms}",
            self.issued_seq
        ));
        let token = hex_prefix(&hasher.finalize(), 32);
        self.entries.insert(
            token.clone(),
            CursorRecord {
                kind,
                authority_epoch,
                observation_seq,
                offset,
                expires_at_unix_ms: now_unix_ms.saturating_add(MANAGEMENT_CURSOR_TTL_MS),
                issued_seq: self.issued_seq,
            },
        );
        OpaqueCursor::new(token)
    }

    fn resolve(
        &mut self,
        token: &str,
        kind: CursorKind,
        authority_epoch: u64,
        observation_seq: u64,
        now_unix_ms: u64,
    ) -> Result<usize, ManagementHttpError> {
        self.remove_expired(now_unix_ms);
        let record = self
            .entries
            .get(token)
            .ok_or(ManagementHttpError::InvalidCursor)?;
        if record.kind != kind
            || record.authority_epoch != authority_epoch
            || record.observation_seq != observation_seq
        {
            return Err(ManagementHttpError::SnapshotChanged);
        }
        Ok(record.offset)
    }

    fn remove_expired(&mut self, now_unix_ms: u64) {
        self.entries
            .retain(|_, record| record.expires_at_unix_ms > now_unix_ms);
    }
}

#[derive(Debug, Clone)]
struct ManagementHttpState {
    runtime: SharedServerRuntime,
    cursors: Arc<Mutex<ManagementCursorStore>>,
    aggregator: Option<Arc<ManagementSnapshotAggregator>>,
    hc2: Option<crate::hc2::Hc2ClientPlaneService>,
}

/// Build the management routes with state shared across repeated router construction in tests.
pub(crate) fn routes(
    runtime: SharedServerRuntime,
    cursors: Arc<Mutex<ManagementCursorStore>>,
    aggregator: Option<Arc<ManagementSnapshotAggregator>>,
    hc2: Option<crate::hc2::Hc2ClientPlaneService>,
) -> Router {
    Router::new()
        .route(MANAGEMENT_CAPABILITIES_PATH, get(capabilities))
        .route(MANAGEMENT_DASHBOARD_PATH, get(dashboard))
        .route(MANAGEMENT_FORMATION_PATH, get(formation))
        .route(MANAGEMENT_CLUSTER_FORMATION_PATH, get(formation))
        .route(MANAGEMENT_CLUSTER_MEMBERS_PATH, get(members))
        .route(MANAGEMENT_CLUSTER_PARTITIONS_PATH, get(partitions))
        .route(MANAGEMENT_CLIENTS_PATH, get(clients))
        .route(MANAGEMENT_NAMESPACES_PATH, get(namespaces))
        .route(
            "/management/v1/namespaces/{namespace}/caches",
            get(namespace_caches),
        )
        .route(
            "/management/v1/cluster/placement-traces/{opaque_id}",
            get(placement_trace),
        )
        .route(MANAGEMENT_RECOVERY_PATH, get(recovery))
        .route(MANAGEMENT_CONSENSUS_PROGRESS_PATH, get(consensus_progress))
        .with_state(Arc::new(ManagementHttpState {
            runtime,
            cursors,
            aggregator,
            hc2,
        }))
}

#[derive(Debug, Serialize)]
struct DashboardData {
    cluster: DashboardCluster,
    replication: DashboardReplication,
    partitions: DashboardPartitions,
    reshard: DashboardReshard,
    cache: DashboardCache,
    consensus: DashboardConsensus,
    placement: DashboardPlacement,
    members: Vec<DashboardMember>,
    unavailable_fields: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct DashboardCluster {
    state: &'static str,
    quorum_ok: bool,
    leader: Option<OpaqueNodeIdentity>,
    term: u64,
    authority_epoch: u64,
    metadata_authoritative: bool,
    member_count: u64,
    voter_count: u32,
}

#[derive(Debug, Serialize)]
struct DashboardReplication {
    success_total: u64,
    failure_total: u64,
    backpressure_total: u64,
    under_replicated: u64,
    repair_debt: u64,
    degraded: bool,
    zone_underspread: u64,
}

#[derive(Debug, Serialize)]
struct DashboardPartitions {
    total: u64,
    assigned: Option<u64>,
    unassigned: Option<u64>,
    distribution: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct DashboardReshard {
    phase: String,
    moves_inflight: u64,
    backfill_lag: u64,
}

#[derive(Debug, Serialize)]
struct DashboardCache {
    entries: u64,
    retained_bytes: Option<u64>,
    hits_total: u64,
    misses_total: u64,
    loads_total: u64,
    hit_ratio: Option<f64>,
    admission_queue_depth: u64,
    admission_rejected_total: u64,
    ttl_backlog: Option<u64>,
}

#[derive(Debug, Serialize)]
struct DashboardConsensus {
    commit_index: Option<u64>,
    applied_index: Option<u64>,
    apply_lag: Option<u64>,
}

#[derive(Debug, Serialize)]
struct DashboardPlacement {
    outcome: Option<&'static str>,
    selected: Option<u64>,
    rejected: Option<u64>,
    latest_committed_epoch: Option<u64>,
    latest_applied_epoch: Option<u64>,
}

#[derive(Debug, Serialize)]
struct DashboardMember {
    node: OpaqueNodeIdentity,
    generation: u64,
    role: &'static str,
    reachability: &'static str,
    cpu_percent: Option<f64>,
    rss_bytes: Option<u64>,
    retained_bytes: Option<u64>,
    uptime_seconds: Option<u64>,
}

async fn dashboard(State(state): State<Arc<ManagementHttpState>>, headers: HeaderMap) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let (status, registry, cluster_overview) = {
        let runtime = state.runtime.lock().expect("server runtime mutex");
        (
            runtime.cluster_status_snapshot(),
            runtime.metrics_registry(),
            runtime.cluster_overview(),
        )
    };
    let overview = registry.overview().await;
    let counters = overview.cluster_grid;
    let entries = overview
        .caches
        .iter()
        .map(|cache| cache.estimated_entries)
        .fold(0_u64, u64::saturating_add);
    let hits = overview
        .caches
        .iter()
        .map(|cache| cache.stats.hits)
        .fold(0_u64, u64::saturating_add);
    let misses = overview
        .caches
        .iter()
        .map(|cache| cache.stats.misses)
        .fold(0_u64, u64::saturating_add);
    let loads = overview
        .caches
        .iter()
        .map(|cache| cache.stats.loads)
        .fold(0_u64, u64::saturating_add);
    let requests = hits.saturating_add(misses);
    let progress = status.local_consensus.as_ref();
    let data = DashboardData {
        cluster: DashboardCluster {
            state: if status.draining {
                "draining"
            } else if status.quorum_ok {
                "active"
            } else {
                "degraded"
            },
            quorum_ok: status.quorum_ok,
            leader: status.leader.as_deref().map(opaque_node_identity),
            term: status.term,
            authority_epoch: status.epoch,
            metadata_authoritative: status.metadata_authoritative,
            member_count: status.members.len() as u64,
            voter_count: status.voters,
        },
        replication: DashboardReplication {
            success_total: counters.replication_success_total,
            failure_total: counters.replication_failure_total,
            backpressure_total: counters.replication_backpressure_total,
            under_replicated: counters.under_replicated_keys,
            repair_debt: counters.tombstone_repair_debt,
            degraded: counters.repair_debt_degraded_mode > 0,
            zone_underspread: counters.placement_zone_underspread,
        },
        partitions: DashboardPartitions {
            total: cluster_overview.partitions.count,
            assigned: None,
            unassigned: None,
            distribution: Vec::new(),
        },
        reshard: DashboardReshard {
            phase: status.reshard_phase.to_string(),
            moves_inflight: counters.reshard_moves_inflight,
            backfill_lag: counters.reshard_backfill_lag,
        },
        cache: DashboardCache {
            entries,
            retained_bytes: None,
            hits_total: hits,
            misses_total: misses,
            loads_total: loads,
            hit_ratio: (requests > 0).then_some(hits as f64 / requests as f64),
            admission_queue_depth: overview.admission.queue_depth,
            admission_rejected_total: overview.admission.rejected_total,
            ttl_backlog: None,
        },
        consensus: DashboardConsensus {
            commit_index: progress.map(|value| value.commit_index),
            applied_index: progress.map(|value| value.applied_index),
            apply_lag: progress.map(|value| value.commit_index.saturating_sub(value.applied_index)),
        },
        placement: DashboardPlacement {
            outcome: None,
            selected: None,
            rejected: None,
            latest_committed_epoch: None,
            latest_applied_epoch: None,
        },
        members: status
            .members
            .iter()
            .take(MAX_MANAGEMENT_PAGE_ITEMS)
            .map(|member| DashboardMember {
                node: opaque_node_identity(&member.node_id),
                generation: member.generation,
                role: member_role(member.role),
                reachability: member_reachability(member.reachable),
                cpu_percent: None,
                rss_bytes: None,
                retained_bytes: None,
                uptime_seconds: None,
            })
            .collect(),
        unavailable_fields: vec![
            "cache.retained_bytes",
            "cache.ttl_backlog",
            "partitions.assigned",
            "partitions.unassigned",
            "partitions.distribution",
            "members.cpu_percent",
            "members.rss_bytes",
            "members.retained_bytes",
            "members.uptime_seconds",
            "placement.outcome",
        ],
    };
    let completeness = ManagementCompleteness::Partial;
    let mut warnings = vec![ManagementWarning {
        code: ManagementWarningCode::PartialObservation,
        affected_count: Some(data.unavailable_fields.len() as u64),
    }];
    if status.members.len() > MAX_MANAGEMENT_PAGE_ITEMS {
        warnings.push(ManagementWarning {
            code: ManagementWarningCode::ResultTruncated,
            affected_count: Some((status.members.len() - MAX_MANAGEMENT_PAGE_ITEMS) as u64),
        });
    }
    let envelope = envelope(&status, completeness, warnings, data);
    management_response(&envelope)
}

fn member_role(role: crate::cluster_status::MemberRole) -> &'static str {
    match role {
        crate::cluster_status::MemberRole::Local => "local",
        crate::cluster_status::MemberRole::Client => "client",
        crate::cluster_status::MemberRole::Member => "member",
    }
}

fn member_reachability(reachability: Reachability) -> &'static str {
    match reachability {
        Reachability::Reachable => "reachable",
        Reachability::Suspect => "suspect",
        Reachability::Unreachable => "unreachable",
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageQuery {
    limit: Option<usize>,
    cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct ManagementErrorReply {
    schema_version: u16,
    code: &'static str,
    detail: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagementHttpError {
    InvalidQuery,
    InvalidLimit,
    InvalidCursor,
    SnapshotChanged,
    ContractViolation,
    ResponseTooLarge,
}

impl ManagementHttpError {
    fn into_response(self) -> Response {
        let (status, code, detail) = match self {
            Self::InvalidQuery => (
                StatusCode::BAD_REQUEST,
                "invalid-query",
                "query parameters are malformed or unsupported",
            ),
            Self::InvalidLimit => (
                StatusCode::BAD_REQUEST,
                "invalid-limit",
                "limit must be between 1 and 100",
            ),
            Self::InvalidCursor => (
                StatusCode::BAD_REQUEST,
                "invalid-cursor",
                "cursor is unknown, malformed, or expired",
            ),
            Self::SnapshotChanged => (
                StatusCode::CONFLICT,
                "snapshot-changed",
                "the source observation changed; restart pagination",
            ),
            Self::ContractViolation => (
                StatusCode::SERVICE_UNAVAILABLE,
                "source-contract-violation",
                "the source observation failed management validation",
            ),
            Self::ResponseTooLarge => (
                StatusCode::SERVICE_UNAVAILABLE,
                "response-too-large",
                "the bounded management response exceeded its byte budget",
            ),
        };
        bounded_json(
            status,
            &ManagementErrorReply {
                schema_version: MANAGEMENT_API_SCHEMA_VERSION,
                code,
                detail,
            },
        )
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}

async fn capabilities(
    State(state): State<Arc<ManagementHttpState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let status = snapshot(&state);
    let formation = match status.source {
        StatusSource::Live => ManagementCapabilityAvailability::Partial,
        StatusSource::Modeled => ManagementCapabilityAvailability::Unavailable,
    };
    let consensus = match (&status.local_consensus, status.metadata_authoritative) {
        (Some(_), true) => ManagementCapabilityAvailability::Available,
        (Some(_), false) => ManagementCapabilityAvailability::Partial,
        (None, _) => ManagementCapabilityAvailability::Unavailable,
    };
    let data = ManagementCapabilities {
        capabilities: vec![
            ManagementCapability {
                id: ManagementCapabilityId::ClusterFormation,
                availability: formation,
                reason: match formation {
                    ManagementCapabilityAvailability::Partial => {
                        Some(ManagementWarningCode::PartialObservation)
                    }
                    ManagementCapabilityAvailability::Unavailable => {
                        Some(ManagementWarningCode::AuthorityUnavailable)
                    }
                    _ => None,
                },
            },
            ManagementCapability {
                id: ManagementCapabilityId::ConsensusProgress,
                availability: consensus,
                reason: match consensus {
                    ManagementCapabilityAvailability::Partial => {
                        Some(ManagementWarningCode::AuthorityUnavailable)
                    }
                    ManagementCapabilityAvailability::Unavailable => {
                        Some(ManagementWarningCode::SourceUnavailable)
                    }
                    _ => None,
                },
            },
            ManagementCapability {
                id: ManagementCapabilityId::PersistenceRecovery,
                availability: ManagementCapabilityAvailability::Unavailable,
                reason: Some(ManagementWarningCode::StatusNotRetained),
            },
        ],
    };
    if data.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    let envelope = envelope(&status, ManagementCompleteness::Complete, Vec::new(), data);
    management_response(&envelope)
}

async fn formation(
    State(state): State<Arc<ManagementHttpState>>,
    headers: HeaderMap,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(error) => return error.into_response(),
    };
    let (status, can_serve, aggregate) = request_snapshot(&state).await;
    let now = unix_time_ms();
    let start = match page_start(&state, &query, CursorKind::Formation, &status, now) {
        Ok(start) => start,
        Err(error) => return error.into_response(),
    };

    let mut members = status.members.clone();
    members.sort_by(|left, right| {
        left.node_id
            .cmp(&right.node_id)
            .then(left.generation.cmp(&right.generation))
    });
    let end = start.saturating_add(query.limit).min(members.len());
    if start > members.len() {
        return ManagementHttpError::SnapshotChanged.into_response();
    }
    let mut items = Vec::with_capacity(end.saturating_sub(start));
    for member in &members[start..end] {
        let peer = aggregate.as_ref().and_then(|aggregate| {
            aggregate.members.iter().find(|snapshot| {
                snapshot.node_id == member.node_id && snapshot.generation == member.generation
            })
        });
        let item = formation_item(&status, member, peer, can_serve);
        if item.validate().is_err() {
            return ManagementHttpError::ContractViolation.into_response();
        }
        items.push(item);
    }
    let truncated = end < members.len();
    let next_cursor = truncated.then(|| {
        state
            .cursors
            .lock()
            .expect("management cursor store mutex")
            .issue(
                CursorKind::Formation,
                status.epoch,
                status.observation_seq,
                end,
                now,
            )
    });
    let page = BoundedPage {
        items,
        next_cursor,
        truncated,
    };
    if page.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    let complete = page.items.iter().all(|item| {
        item.completeness == ManagementCompleteness::Complete
            && item.source == ManagementObservationSource::Live
    }) && !truncated
        && aggregate
            .as_ref()
            .is_none_or(|aggregate| aggregate.complete());
    let completeness = if complete {
        ManagementCompleteness::Complete
    } else {
        ManagementCompleteness::Partial
    };
    let mut warnings =
        response_warnings(completeness, truncated, members.len().saturating_sub(end));
    if let Some(aggregate) = &aggregate {
        append_aggregation_warnings(&mut warnings, aggregate);
    }
    let envelope = envelope(&status, completeness, warnings, page);
    management_response(&envelope)
}

async fn members(
    State(state): State<Arc<ManagementHttpState>>,
    headers: HeaderMap,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(error) => return error.into_response(),
    };
    let (status, can_serve, aggregate) = request_snapshot(&state).await;
    let (diagnostics, cache) = {
        let runtime = state.runtime.lock().expect("server runtime mutex");
        (
            runtime.management_local_diagnostics(),
            runtime.management_cache(),
        )
    };
    let retained_bytes = cache
        .memory_footprint_snapshot(hydracache::MemorySnapshotRequest::Admin)
        .await
        .ok()
        .map(|snapshot| snapshot.estimated_retained_bytes);
    let now = unix_time_ms();
    let start = match page_start(&state, &query, CursorKind::Members, &status, now) {
        Ok(start) => start,
        Err(error) => return error.into_response(),
    };
    let mut committed = status.members.clone();
    committed.sort_by(|left, right| {
        left.node_id
            .cmp(&right.node_id)
            .then(left.generation.cmp(&right.generation))
    });
    if start > committed.len() {
        return ManagementHttpError::SnapshotChanged.into_response();
    }
    let end = start.saturating_add(query.limit).min(committed.len());
    let mut items = Vec::with_capacity(end.saturating_sub(start));
    for member in &committed[start..end] {
        let peer = aggregate.as_ref().and_then(|aggregate| {
            aggregate.members.iter().find(|candidate| {
                candidate.node_id == member.node_id && candidate.generation == member.generation
            })
        });
        let formation = formation_item(&status, member, peer, can_serve);
        let local = status.local_consensus.as_ref().is_some_and(|progress| {
            progress.node_id == member.node_id && progress.generation == member.generation
        });
        let reachability_reason = match member.reachable {
            Reachability::Reachable => MemberReachabilityReason::Healthy,
            Reachability::Suspect => MemberReachabilityReason::Suspected,
            Reachability::Unreachable => MemberReachabilityReason::TransportUnreachable,
        };
        let timeline = formation
            .authority_epoch
            .map(|authority_epoch| {
                vec![MemberFormationTransition {
                    observation_seq: status.observation_seq,
                    authority_epoch,
                    serving: formation.serving,
                    blocker: formation.blocker,
                }]
            })
            .unwrap_or_default();
        let item = ManagementMemberDetail {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            node: formation.node.clone(),
            generation: member.generation,
            authority_epoch: formation.authority_epoch,
            observation_seq: status.observation_seq,
            consensus_role: formation.consensus_role,
            product_version: local.then(|| diagnostics.product_version.clone()),
            protocol_compatible: local.then_some(true),
            reachability: formation.transport,
            reachability_reason,
            draining: local.then_some(status.draining),
            uptime_seconds: local.then_some(diagnostics.uptime_seconds),
            cpu_percent: None,
            rss_bytes: local.then_some(diagnostics.rss_bytes).flatten(),
            retained_bytes: local.then_some(retained_bytes).flatten(),
            open_fds: local.then_some(diagnostics.open_fds).flatten(),
            thread_count: local.then_some(diagnostics.thread_count).flatten(),
            task_count: None,
            client_count: local.then_some(diagnostics.client_count),
            partition_count: None,
            config_digest: local.then(|| diagnostics.config_digest.clone()),
            formation,
            timeline,
            source: source(status.source),
            completeness: ManagementCompleteness::Partial,
        };
        if item.validate().is_err() {
            return ManagementHttpError::ContractViolation.into_response();
        }
        items.push(item);
    }
    let truncated = end < committed.len();
    let next_cursor = truncated.then(|| {
        state
            .cursors
            .lock()
            .expect("management cursor store mutex")
            .issue(
                CursorKind::Members,
                status.epoch,
                status.observation_seq,
                end,
                now,
            )
    });
    let page = BoundedPage {
        items,
        next_cursor,
        truncated,
    };
    if page.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    let mut warnings = response_warnings(
        ManagementCompleteness::Partial,
        truncated,
        committed.len().saturating_sub(end),
    );
    if let Some(aggregate) = &aggregate {
        append_aggregation_warnings(&mut warnings, aggregate);
    }
    management_response(&envelope(
        &status,
        ManagementCompleteness::Partial,
        warnings,
        page,
    ))
}

async fn partitions(State(state): State<Arc<ManagementHttpState>>, headers: HeaderMap) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let (status, topology, cluster_overview, registry) = {
        let runtime = state.runtime.lock().expect("server runtime mutex");
        (
            runtime.cluster_status_snapshot(),
            runtime.management_topology_model(),
            runtime.cluster_overview(),
            runtime.metrics_registry(),
        )
    };
    let overview = registry.overview().await;
    let stale = topology.partition_observation().is_some()
        && topology
            .partition_for(status.epoch, status.observation_seq)
            .is_none();
    let (data, warnings) =
        if let Some(snapshot) = topology.partition_for(status.epoch, status.observation_seq) {
            (snapshot, Vec::new())
        } else {
            let counters = overview.cluster_grid;
            (
                ManagementPartitionSnapshot {
                    schema_version: MANAGEMENT_API_SCHEMA_VERSION,
                    authority_epoch: status.metadata_authoritative.then_some(status.epoch),
                    observation_seq: status.observation_seq,
                    total: cluster_overview.partitions.count,
                    assigned: None,
                    unassigned: None,
                    distribution: Vec::new(),
                    under_replicated: counters.under_replicated_keys,
                    zone_underspread: counters.placement_zone_underspread,
                    repair_debt: counters.tombstone_repair_debt,
                    reshard_phase: status.reshard_phase.to_string(),
                    reshard_moves_inflight: counters.reshard_moves_inflight,
                    backfill_lag: counters.reshard_backfill_lag,
                    placement_trace_id: None,
                    source: source(status.source),
                    completeness: ManagementCompleteness::Partial,
                },
                vec![ManagementWarning {
                    code: if stale {
                        ManagementWarningCode::StaleObservation
                    } else {
                        ManagementWarningCode::SourceUnavailable
                    },
                    affected_count: Some(1),
                }],
            )
        };
    if data.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    let completeness = data.completeness;
    management_response(&envelope(&status, completeness, warnings, data))
}

async fn namespaces(
    State(state): State<Arc<ManagementHttpState>>,
    headers: HeaderMap,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let identity = match ClientIdentity::from_headers(&headers) {
        Ok(identity) => identity,
        Err(_) => return StatusCode::FORBIDDEN.into_response(),
    };
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(error) => return error.into_response(),
    };
    let (status, client_state) = {
        let runtime = state.runtime.lock().expect("server runtime mutex");
        (
            runtime.cluster_status_snapshot(),
            runtime.management_client_state(),
        )
    };
    let Some(client_state) = client_state else {
        let page = BoundedPage::<ManagementNamespaceSummary> {
            items: Vec::new(),
            next_cursor: None,
            truncated: false,
        };
        let mut result = envelope(
            &status,
            ManagementCompleteness::Partial,
            vec![ManagementWarning {
                code: ManagementWarningCode::SourceUnavailable,
                affected_count: Some(1),
            }],
            page,
        );
        result.source = ManagementObservationSource::Unavailable;
        return management_response(&result);
    };
    let tenant = match client_state.tenant_status(&identity) {
        Ok(tenant) => tenant,
        Err(_) => return StatusCode::FORBIDDEN.into_response(),
    };
    let now = unix_time_ms();
    let start = match page_start(&state, &query, CursorKind::Namespaces, &status, now) {
        Ok(start) => start,
        Err(error) => return error.into_response(),
    };
    if start > tenant.namespaces.len() {
        return ManagementHttpError::SnapshotChanged.into_response();
    }
    let end = start
        .saturating_add(query.limit)
        .min(tenant.namespaces.len());
    let mut items = Vec::with_capacity(end.saturating_sub(start));
    for namespace in &tenant.namespaces[start..end] {
        let item = ManagementNamespaceSummary {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            namespace: namespace.namespace.clone(),
            cache_count: 1,
            entries: namespace.entries,
            logical_bytes: namespace.bytes,
            retained_bytes: None,
            max_entries: namespace.max_entries,
            max_bytes: namespace.max_bytes,
            admitted_requests: tenant.rate_limit.request_count,
            rate_limit_per_window: tenant.rate_limit.rate_limit_per_window,
            fair_share_count: tenant.rate_limit.fair_share_count,
            fair_share_per_window: tenant.rate_limit.fair_share_per_window,
            admission_rejected_total: tenant.rate_limit.admission_rejected_total,
            active_subscriptions: tenant.near_cache.active_subscriptions,
            near_cache_repairs_total: tenant.near_cache.repairs_total,
            persistence_status: ManagementMeasurementQuality::Unavailable,
            usage_quality: ManagementMeasurementQuality::Exact,
            source: source(status.source),
            completeness: ManagementCompleteness::Partial,
        };
        if item.validate().is_err() {
            return ManagementHttpError::ContractViolation.into_response();
        }
        items.push(item);
    }
    let truncated = end < tenant.namespaces.len();
    let next_cursor = truncated.then(|| {
        state
            .cursors
            .lock()
            .expect("management cursor store mutex")
            .issue(
                CursorKind::Namespaces,
                status.epoch,
                status.observation_seq,
                end,
                now,
            )
    });
    let page = BoundedPage {
        items,
        next_cursor,
        truncated,
    };
    if page.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    let mut warnings = response_warnings(
        ManagementCompleteness::Partial,
        truncated,
        tenant.namespaces.len().saturating_sub(end),
    );
    warnings.push(ManagementWarning {
        code: ManagementWarningCode::SourceUnavailable,
        affected_count: Some(page.items.len() as u64),
    });
    management_response(&envelope(
        &status,
        ManagementCompleteness::Partial,
        warnings,
        page,
    ))
}

async fn namespace_caches(
    State(state): State<Arc<ManagementHttpState>>,
    headers: HeaderMap,
    Path(namespace): Path<String>,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let identity = match ClientIdentity::from_headers(&headers) {
        Ok(identity) => identity,
        Err(_) => return StatusCode::FORBIDDEN.into_response(),
    };
    if namespace.is_empty() || namespace.len() > 128 || namespace.chars().any(char::is_control) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (status, client_state) = {
        let runtime = state.runtime.lock().expect("server runtime mutex");
        (
            runtime.cluster_status_snapshot(),
            runtime.management_client_state(),
        )
    };
    let Some(client_state) = client_state else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let tenant = match client_state.tenant_status(&identity) {
        Ok(tenant) => tenant,
        Err(_) => return StatusCode::FORBIDDEN.into_response(),
    };
    let Some(namespace) = tenant
        .namespaces
        .iter()
        .find(|candidate| candidate.namespace == namespace)
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let item = ManagementCacheDetail {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        namespace: namespace.namespace.clone(),
        cache: "client-surface".to_owned(),
        entries: namespace.entries,
        logical_bytes: namespace.bytes,
        retained_bytes: None,
        max_entries: namespace.max_entries,
        max_bytes: namespace.max_bytes,
        hit_total: None,
        miss_total: None,
        load_total: None,
        ttl_backlog: None,
        tag_index_bytes: None,
        conditional_records: None,
        idempotency_records: None,
        audit_records: None,
        backup_age_seconds: None,
        load_breaker_active: None,
        usage_quality: ManagementMeasurementQuality::Exact,
        source: source(status.source),
        completeness: ManagementCompleteness::Partial,
    };
    if item.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    let page = BoundedPage {
        items: vec![item],
        next_cursor: None,
        truncated: false,
    };
    management_response(&envelope(
        &status,
        ManagementCompleteness::Partial,
        vec![ManagementWarning {
            code: ManagementWarningCode::SourceUnavailable,
            affected_count: Some(1),
        }],
        page,
    ))
}

async fn clients(State(state): State<Arc<ManagementHttpState>>, headers: HeaderMap) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let (status, local) = {
        let runtime = state.runtime.lock().expect("server runtime mutex");
        (
            runtime.cluster_status_snapshot(),
            runtime.management_client_lifecycle(),
        )
    };
    let mut protocols = Vec::new();
    if let (Some(rejected), Some(subscriptions)) =
        (local.hc1_rejected_total, local.hc1_active_subscriptions)
    {
        protocols.push(ClientProtocolLifecycle {
            protocol: ManagementClientProtocol::Hc1,
            version: Some("hc-1".to_owned()),
            active_connections: None,
            accepted_total: None,
            closed_total: None,
            rejected_total: Some(rejected),
            pending_invocations: None,
            active_subscriptions: Some(subscriptions),
            active_sessions: None,
            buffered_bytes: None,
        });
    }
    if let Some(service) = &state.hc2 {
        let accounting = service.accounting();
        protocols.push(ClientProtocolLifecycle {
            protocol: ManagementClientProtocol::Hc2,
            version: Some("hc-2-alpha".to_owned()),
            active_connections: Some(accounting.active_connections),
            accepted_total: Some(accounting.accepted_connections),
            closed_total: Some(accounting.closed_connections),
            rejected_total: Some(accounting.rejected_frames),
            pending_invocations: Some(accounting.pending_invocations),
            active_subscriptions: Some(accounting.active_subscriptions),
            active_sessions: Some(accounting.active_sessions),
            buffered_bytes: None,
        });
    }
    if let (Some(active), Some(accepted), Some(closed), Some(rejected)) = (
        local.resp_active,
        local.resp_accepted,
        local.resp_closed,
        local.resp_rejected,
    ) {
        protocols.push(ClientProtocolLifecycle {
            protocol: ManagementClientProtocol::Resp,
            version: Some("resp-2-3".to_owned()),
            active_connections: Some(active),
            accepted_total: Some(accepted),
            closed_total: Some(closed),
            rejected_total: Some(rejected),
            pending_invocations: Some(0),
            active_subscriptions: Some(0),
            active_sessions: Some(0),
            buffered_bytes: None,
        });
    }
    protocols.sort_by_key(|protocol| protocol.protocol);
    let active_connections = sum_protocol_field(&protocols, |item| item.active_connections);
    let accepted_total = sum_protocol_field(&protocols, |item| item.accepted_total);
    let closed_total = sum_protocol_field(&protocols, |item| item.closed_total);
    let pending_invocations = sum_protocol_field(&protocols, |item| item.pending_invocations);
    let active_sessions = sum_protocol_field(&protocols, |item| item.active_sessions);
    let active_subscriptions = protocols
        .iter()
        .filter_map(|item| item.active_subscriptions)
        .fold(0_u64, u64::saturating_add);
    let rejected_total = protocols
        .iter()
        .filter_map(|item| item.rejected_total)
        .fold(0_u64, u64::saturating_add);
    let complete = !protocols.is_empty()
        && protocols.iter().all(|item| {
            item.active_connections.is_some()
                && item.accepted_total.is_some()
                && item.closed_total.is_some()
                && item.pending_invocations.is_some()
                && item.active_sessions.is_some()
                && item.buffered_bytes.is_some()
        });
    let completeness = if complete {
        ManagementCompleteness::Complete
    } else {
        ManagementCompleteness::Partial
    };
    let data = ManagementClientsSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        authority_epoch: status.metadata_authoritative.then_some(status.epoch),
        observation_seq: status.observation_seq,
        active_connections,
        accepted_total,
        closed_total,
        rejected_total,
        pending_invocations,
        active_subscriptions,
        active_sessions,
        buffered_bytes: None,
        reconnecting: None,
        slow: None,
        quota_rejected_total: None,
        cleanup_lag: None,
        protocols,
        detail_available: false,
        source: source(status.source),
        completeness,
    };
    if data.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    let warnings = (!complete)
        .then_some(ManagementWarning {
            code: ManagementWarningCode::PartialObservation,
            affected_count: Some(1),
        })
        .into_iter()
        .collect();
    management_response(&envelope(&status, completeness, warnings, data))
}

fn sum_protocol_field(
    protocols: &[ClientProtocolLifecycle],
    field: impl Fn(&ClientProtocolLifecycle) -> Option<u64>,
) -> Option<u64> {
    protocols.iter().try_fold(0_u64, |sum, protocol| {
        field(protocol).map(|value| sum.saturating_add(value))
    })
}

async fn placement_trace(
    State(state): State<Arc<ManagementHttpState>>,
    headers: HeaderMap,
    Path(opaque_id): Path<String>,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    if opaque_id.is_empty()
        || opaque_id.len() > 128
        || !opaque_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (status, trace): (ClusterStatus, Option<PlacementDecisionTrace>) = {
        let runtime = state.runtime.lock().expect("server runtime mutex");
        let status = runtime.cluster_status_snapshot();
        let trace = runtime
            .management_topology_model()
            .placement_trace_for(&opaque_id, status.epoch);
        (status, trace)
    };
    let Some(trace) = trace else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if trace.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    management_response(&envelope(&status, trace.completeness, Vec::new(), trace))
}

async fn consensus_progress(
    State(state): State<Arc<ManagementHttpState>>,
    headers: HeaderMap,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(error) => return error.into_response(),
    };
    let (status, _can_serve, aggregate) = request_snapshot(&state).await;
    let now = unix_time_ms();
    let start = match page_start(&state, &query, CursorKind::ConsensusProgress, &status, now) {
        Ok(start) => start,
        Err(error) => return error.into_response(),
    };
    let mut all = if let Some(aggregate) = &aggregate {
        aggregate
            .members
            .iter()
            .filter_map(|member| consensus_aggregate_item(&status, member))
            .collect::<Vec<_>>()
    } else {
        status
            .local_consensus
            .as_ref()
            .map(|progress| consensus_item(&status, progress))
            .into_iter()
            .collect::<Vec<_>>()
    };
    all.sort_by(|left, right| {
        left.node
            .cmp(&right.node)
            .then(left.generation.cmp(&right.generation))
    });
    if all.iter().any(|item| item.validate().is_err()) {
        return ManagementHttpError::ContractViolation.into_response();
    }
    if start > all.len() {
        return ManagementHttpError::SnapshotChanged.into_response();
    }
    let total = all.len();
    let end = start.saturating_add(query.limit).min(total);
    let items = all.drain(start..end).collect::<Vec<_>>();
    let truncated = end < total;
    let next_cursor = truncated.then(|| {
        state
            .cursors
            .lock()
            .expect("management cursor store mutex")
            .issue(
                CursorKind::ConsensusProgress,
                status.epoch,
                status.observation_seq,
                end,
                now,
            )
    });
    let page = BoundedPage {
        items,
        next_cursor,
        truncated,
    };
    let available = total > 0;
    let complete = available
        && status.metadata_authoritative
        && !truncated
        && aggregate
            .as_ref()
            .is_none_or(|aggregate| aggregate.complete());
    let source = if available {
        source(status.source)
    } else {
        ManagementObservationSource::Unavailable
    };
    let completeness = if complete {
        ManagementCompleteness::Complete
    } else {
        ManagementCompleteness::Partial
    };
    let mut warnings = (!complete)
        .then_some(ManagementWarning {
            code: if available {
                ManagementWarningCode::AuthorityUnavailable
            } else {
                ManagementWarningCode::SourceUnavailable
            },
            affected_count: Some(1),
        })
        .into_iter()
        .collect();
    if let Some(aggregate) = &aggregate {
        append_aggregation_warnings(&mut warnings, aggregate);
    }
    let mut envelope = envelope(&status, completeness, warnings, page);
    envelope.source = source;
    management_response(&envelope)
}

async fn recovery(
    State(state): State<Arc<ManagementHttpState>>,
    headers: HeaderMap,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(error) => return error.into_response(),
    };
    let status = snapshot(&state);
    let now = unix_time_ms();
    let start = match page_start(&state, &query, CursorKind::Recovery, &status, now) {
        Ok(start) => start,
        Err(error) => return error.into_response(),
    };
    let item = DurableRecoveryStatus {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        scope: RecoveryScope::Node,
        outcome: RecoveryOutcome::Unknown,
        phase: RecoveryPhase::Unknown,
        artifact: RecoveryArtifactKind::DurableValueStore,
        artifact_format_version: None,
        validated_watermark: None,
        records_checked: BoundedCount::exact(0),
        records_recovered: BoundedCount::exact(0),
        records_discarded: BoundedCount::exact(0),
        corrupt_records: BoundedCount::exact(0),
        repair: RepairState::Unknown,
        reason: Some(RecoveryReasonCode::StatusNotRetained),
        started_at_unix_ms: None,
        completed_at_unix_ms: None,
        source: ManagementObservationSource::Unavailable,
        completeness: ManagementCompleteness::Partial,
    };
    if item.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    let all = [item];
    if start > all.len() {
        return ManagementHttpError::SnapshotChanged.into_response();
    }
    let end = start.saturating_add(query.limit).min(all.len());
    let page = BoundedPage {
        items: all[start..end].to_vec(),
        next_cursor: None,
        truncated: false,
    };
    let mut envelope = envelope(
        &status,
        ManagementCompleteness::Partial,
        vec![ManagementWarning {
            code: ManagementWarningCode::StatusNotRetained,
            affected_count: Some(1),
        }],
        page,
    );
    envelope.source = ManagementObservationSource::Unavailable;
    management_response(&envelope)
}

fn formation_item(
    status: &ClusterStatus,
    member: &crate::cluster_status::MemberStatus,
    aggregated: Option<&ManagementMemberSnapshot>,
    can_serve: bool,
) -> ClusterFormationSnapshot {
    let local = status.local_consensus.as_ref().filter(|progress| {
        progress.node_id == member.node_id && progress.generation == member.generation
    });
    let aggregated = aggregated.filter(|snapshot| {
        snapshot.authority_epoch == status.epoch
            && snapshot.node_id == member.node_id
            && snapshot.generation == member.generation
    });
    let observed = aggregated.is_some() || local.is_some();
    let authoritative = status.metadata_authoritative
        && status.source == StatusSource::Live
        && aggregated.is_none_or(|snapshot| snapshot.authority_epoch == status.epoch);
    let transport = match (observed, member.reachable) {
        (_, Reachability::Unreachable) => TransportState::Unreachable,
        (true, _) => TransportState::Authenticated,
        (false, _) => TransportState::Reachable,
    };
    let admission = if authoritative {
        AdmissionState::Admitted
    } else {
        AdmissionState::Unknown
    };
    let consensus_role = if admission == AdmissionState::Admitted {
        aggregated
            .and_then(|snapshot| snapshot.consensus.as_ref())
            .map(|progress| progress.voter)
            .or_else(|| local.map(|progress| progress.voter))
            .map_or(ManagementConsensusRole::Unknown, |voter| {
                if voter {
                    ManagementConsensusRole::Voter
                } else {
                    ManagementConsensusRole::Learner
                }
            })
    } else {
        ManagementConsensusRole::Unknown
    };
    let progress = aggregated
        .and_then(|snapshot| snapshot.consensus.as_ref())
        .map(|progress| (progress.applied_index, progress.catch_up_target))
        .or_else(|| local.map(|progress| (progress.applied_index, progress.catch_up_target)));
    let catch_up = progress.map_or(CatchUpState::Unknown, |(applied, target)| {
        if applied >= target {
            CatchUpState::Current
        } else {
            CatchUpState::Behind
        }
    });
    let serving = if local.is_some() && status.draining {
        ServingState::Draining
    } else if local.is_some()
        && can_serve
        && authoritative
        && transport == TransportState::Authenticated
        && catch_up == CatchUpState::Current
    {
        ServingState::Serving
    } else {
        ServingState::Blocked
    };
    let blocker = match serving {
        ServingState::Serving => None,
        ServingState::Draining => Some(FormationReasonCode::Draining),
        _ if member.reachable == Reachability::Unreachable => {
            Some(FormationReasonCode::TransportUnreachable)
        }
        _ if !authoritative => Some(FormationReasonCode::AuthorityUnknown),
        _ if progress.is_some_and(|(applied, target)| applied < target) => {
            Some(FormationReasonCode::LearnerBehind)
        }
        _ => Some(FormationReasonCode::SourceUnavailable),
    };
    let completeness = if local.is_some()
        && authoritative
        && !matches!(consensus_role, ManagementConsensusRole::Unknown)
        && !matches!(catch_up, CatchUpState::Unknown)
    {
        ManagementCompleteness::Complete
    } else {
        ManagementCompleteness::Partial
    };
    ClusterFormationSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node: opaque_node_identity(&member.node_id),
        generation: member.generation,
        candidate_observation_seq: aggregated
            .map(|snapshot| snapshot.observation_seq)
            .or(Some(status.observation_seq)),
        authority_epoch: authoritative.then_some(status.epoch),
        discovery: DiscoveryState::Discovered,
        transport,
        admission,
        consensus_role,
        catch_up,
        serving,
        blocker,
        source: source(status.source),
        completeness,
    }
}

fn consensus_item(
    status: &ClusterStatus,
    progress: &LocalConsensusStatus,
) -> ConsensusProgressSnapshot {
    ConsensusProgressSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node: opaque_node_identity(&progress.node_id),
        generation: progress.generation,
        authority_epoch: status.metadata_authoritative.then_some(status.epoch),
        commit_index: Some(progress.commit_index),
        applied_index: Some(progress.applied_index),
        last_snapshot_index: progress.last_snapshot_index,
        catch_up_target: Some(progress.catch_up_target),
        source: source(status.source),
        completeness: if status.metadata_authoritative {
            ManagementCompleteness::Complete
        } else {
            ManagementCompleteness::Partial
        },
    }
}

fn consensus_aggregate_item(
    status: &ClusterStatus,
    member: &ManagementMemberSnapshot,
) -> Option<ConsensusProgressSnapshot> {
    let progress = member.consensus.as_ref()?;
    Some(ConsensusProgressSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node: opaque_node_identity(&member.node_id),
        generation: member.generation,
        authority_epoch: (member.authority_epoch == status.epoch).then_some(status.epoch),
        commit_index: Some(progress.commit_index),
        applied_index: Some(progress.applied_index),
        last_snapshot_index: progress.last_snapshot_index,
        catch_up_target: Some(progress.catch_up_target),
        source: ManagementObservationSource::Live,
        completeness: if member.authority_epoch == status.epoch {
            ManagementCompleteness::Complete
        } else {
            ManagementCompleteness::Partial
        },
    })
}

fn parse_query(
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Result<NormalizedPageQuery, ManagementHttpError> {
    let Query(query) = query.map_err(|_| ManagementHttpError::InvalidQuery)?;
    let limit = query.limit.unwrap_or(MAX_MANAGEMENT_PAGE_ITEMS);
    if !(1..=MAX_MANAGEMENT_PAGE_ITEMS).contains(&limit) {
        return Err(ManagementHttpError::InvalidLimit);
    }
    if query
        .cursor
        .as_ref()
        .is_some_and(|cursor| cursor.is_empty() || cursor.len() > MAX_MANAGEMENT_CURSOR_BYTES)
    {
        return Err(ManagementHttpError::InvalidCursor);
    }
    Ok(NormalizedPageQuery {
        limit,
        cursor: query.cursor,
    })
}

#[derive(Debug)]
struct NormalizedPageQuery {
    limit: usize,
    cursor: Option<String>,
}

fn page_start(
    state: &ManagementHttpState,
    query: &NormalizedPageQuery,
    kind: CursorKind,
    status: &ClusterStatus,
    now_unix_ms: u64,
) -> Result<usize, ManagementHttpError> {
    query.cursor.as_deref().map_or(Ok(0), |cursor| {
        state
            .cursors
            .lock()
            .expect("management cursor store mutex")
            .resolve(
                cursor,
                kind,
                status.epoch,
                status.observation_seq,
                now_unix_ms,
            )
    })
}

fn snapshot(state: &ManagementHttpState) -> ClusterStatus {
    state
        .runtime
        .lock()
        .expect("server runtime mutex")
        .cluster_status_snapshot()
}

async fn request_snapshot(
    state: &ManagementHttpState,
) -> (
    ClusterStatus,
    bool,
    Option<Arc<AggregatedManagementSnapshot>>,
) {
    let (status, can_serve, input) = {
        let runtime = state.runtime.lock().expect("server runtime mutex");
        (
            runtime.cluster_status_snapshot(),
            runtime.can_serve(),
            runtime.management_snapshot_input(),
        )
    };
    let aggregate = match (&state.aggregator, input) {
        (Some(aggregator), Some((local, targets)))
            if local.authority_epoch == status.epoch
                && local.observation_seq == status.observation_seq =>
        {
            Some(aggregator.refresh(local, targets).await)
        }
        _ => None,
    };
    (status, can_serve, aggregate)
}

fn append_aggregation_warnings(
    warnings: &mut Vec<ManagementWarning>,
    aggregate: &AggregatedManagementSnapshot,
) {
    let mut counts = BTreeMap::<ManagementWarningCode, u64>::new();
    for failure in &aggregate.failures {
        let code = match failure.issue {
            ManagementAggregationIssue::Timeout => ManagementWarningCode::PeerTimeout,
            ManagementAggregationIssue::Incompatible => ManagementWarningCode::PeerIncompatible,
            ManagementAggregationIssue::Duplicate => ManagementWarningCode::DuplicateObservation,
            ManagementAggregationIssue::IdentityMismatch
            | ManagementAggregationIssue::GenerationMismatch => {
                ManagementWarningCode::PeerIdentityMismatch
            }
            ManagementAggregationIssue::EpochMismatch | ManagementAggregationIssue::Stale => {
                ManagementWarningCode::StaleObservation
            }
            ManagementAggregationIssue::TruncatedRoster => ManagementWarningCode::ResultTruncated,
            ManagementAggregationIssue::MissingEndpoint
            | ManagementAggregationIssue::Transport
            | ManagementAggregationIssue::Oversize
            | ManagementAggregationIssue::Malformed => ManagementWarningCode::SourceUnavailable,
        };
        let count = counts.entry(code).or_default();
        *count = count.saturating_add(1);
    }
    for (code, affected_count) in counts {
        if warnings.len() >= hydracache_observability::MAX_MANAGEMENT_WARNINGS {
            break;
        }
        warnings.push(ManagementWarning {
            code,
            affected_count: Some(affected_count),
        });
    }
}

fn envelope<T>(
    status: &ClusterStatus,
    completeness: ManagementCompleteness,
    warnings: Vec<ManagementWarning>,
    data: T,
) -> ManagementEnvelope<T> {
    ManagementEnvelope {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        observation_seq: status.observation_seq,
        authority_epoch: (status.source == StatusSource::Live).then_some(status.epoch),
        captured_at_unix_ms: unix_time_ms(),
        source: source(status.source),
        completeness,
        stale_after_ms: MANAGEMENT_STALE_AFTER_MS,
        warnings,
        data,
    }
}

fn response_warnings(
    completeness: ManagementCompleteness,
    truncated: bool,
    omitted: usize,
) -> Vec<ManagementWarning> {
    let mut warnings = Vec::with_capacity(2);
    if completeness == ManagementCompleteness::Partial {
        warnings.push(ManagementWarning {
            code: ManagementWarningCode::PartialObservation,
            affected_count: None,
        });
    }
    if truncated {
        warnings.push(ManagementWarning {
            code: ManagementWarningCode::ResultTruncated,
            affected_count: Some(omitted as u64),
        });
    }
    warnings
}

fn source(value: StatusSource) -> ManagementObservationSource {
    match value {
        StatusSource::Live => ManagementObservationSource::Live,
        StatusSource::Modeled => ManagementObservationSource::Modeled,
    }
}

fn opaque_node_identity(node_id: &str) -> OpaqueNodeIdentity {
    let mut hasher = Sha256::new();
    hasher.update(b"hydracache-management-node-v1\0");
    hasher.update(node_id.as_bytes());
    OpaqueNodeIdentity::new(format!("node-{}", hex_prefix(&hasher.finalize(), 16)))
}

fn hex_prefix(bytes: &[u8], length: usize) -> String {
    let mut output = String::with_capacity(length.saturating_mul(2));
    for byte in bytes.iter().take(length) {
        write!(&mut output, "{byte:02x}").expect("writing hex to String cannot fail");
    }
    output
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn management_response<T: Serialize>(envelope: &ManagementEnvelope<T>) -> Response {
    if envelope.validate().is_err() {
        return ManagementHttpError::ContractViolation.into_response();
    }
    bounded_json(StatusCode::OK, envelope).unwrap_or_else(|error| error.into_response())
}

fn bounded_json<T: Serialize>(
    status: StatusCode,
    value: &T,
) -> Result<Response, ManagementHttpError> {
    let body = serde_json::to_vec(value).map_err(|_| ManagementHttpError::ContractViolation)?;
    if body.len() > MANAGEMENT_MAX_RESPONSE_BYTES {
        return Err(ManagementHttpError::ResponseTooLarge);
    }
    Ok((status, [(CONTENT_TYPE, "application/json")], body).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursors_are_opaque_bounded_and_expire() {
        let mut store = ManagementCursorStore::default();
        let cursor = store.issue(CursorKind::Formation, 7, 9, 11, 1_000);
        assert!(cursor.as_str().len() <= MAX_MANAGEMENT_CURSOR_BYTES);
        assert_ne!(cursor.as_str(), "11");
        assert_eq!(cursor.as_str().len(), 64);
        assert_eq!(
            store.resolve(cursor.as_str(), CursorKind::Formation, 7, 9, 1_001),
            Ok(11)
        );
        assert_eq!(
            store.resolve(
                cursor.as_str(),
                CursorKind::Formation,
                7,
                9,
                1_000 + MANAGEMENT_CURSOR_TTL_MS
            ),
            Err(ManagementHttpError::InvalidCursor)
        );
    }

    #[test]
    fn cursor_is_bound_to_kind_epoch_and_observation() {
        let mut store = ManagementCursorStore::default();
        let cursor = store.issue(CursorKind::Formation, 7, 9, 11, 1_000);
        assert_eq!(
            store.resolve(cursor.as_str(), CursorKind::Recovery, 7, 9, 1_001),
            Err(ManagementHttpError::SnapshotChanged)
        );
        assert_eq!(
            store.resolve(cursor.as_str(), CursorKind::Formation, 8, 9, 1_001),
            Err(ManagementHttpError::SnapshotChanged)
        );
        assert_eq!(
            store.resolve(cursor.as_str(), CursorKind::Formation, 7, 10, 1_001),
            Err(ManagementHttpError::SnapshotChanged)
        );
    }

    #[test]
    fn cursor_store_evicts_oldest_entry_at_hard_limit() {
        let mut store = ManagementCursorStore::default();
        let first = store.issue(CursorKind::Formation, 1, 1, 1, 1_000);
        for offset in 2..=MANAGEMENT_MAX_RETAINED_CURSORS + 1 {
            store.issue(CursorKind::Formation, 1, 1, offset, 1_000);
        }
        assert_eq!(store.entries.len(), MANAGEMENT_MAX_RETAINED_CURSORS);
        assert_eq!(
            store.resolve(first.as_str(), CursorKind::Formation, 1, 1, 1_001),
            Err(ManagementHttpError::InvalidCursor)
        );
    }
}
