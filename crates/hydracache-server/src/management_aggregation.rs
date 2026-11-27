//! Authenticated, bounded member-snapshot collection for Management Center.
//!
//! This module is deliberately read-only.  The caller supplies a committed
//! roster; browser input can neither add endpoints nor select a destination.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, StreamExt};
use hydracache_cluster_transport_axum::{ClusterRoute, ClusterRouteAuth};
use hydracache_observability::MANAGEMENT_API_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;

use crate::imap_ops::{
    IMapEvidence, IMapPartitionCounts, IMapReadiness, IMapReadinessReason, HEALTH_TTL,
};
use crate::imap_replicated_read::D2_PARTITION_COUNT;

/// Negotiated management snapshot schema with a bounded IMap observation.
pub const IMAP_SNAPSHOT_SCHEMA_VERSION: u16 = 2;

/// Protected member-to-member snapshot path on the cluster listener.
pub const CLUSTER_MANAGEMENT_SNAPSHOT_PATH: &str = "/cluster/management/v1/snapshot";
/// Protected capability probe used before sending the 0.72 snapshot RPC.
pub const CLUSTER_MANAGEMENT_CAPABILITIES_PATH: &str = "/cluster/management/capabilities";
/// Maximum request bytes accepted by the protected snapshot route.
pub const MANAGEMENT_SNAPSHOT_MAX_REQUEST_BYTES: usize = 4 * 1024;
/// Maximum response bytes retained from one peer.
pub const MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES: usize = 256 * 1024;
/// Maximum committed members considered by one refresh.
pub const MANAGEMENT_SNAPSHOT_MAX_PEERS: usize = 100;
/// Maximum simultaneous peer requests.
pub const MANAGEMENT_SNAPSHOT_MAX_CONCURRENCY: usize = 8;
/// Per-peer deadline.
pub const MANAGEMENT_SNAPSHOT_PEER_TIMEOUT: Duration = Duration::from_millis(500);
/// Whole-refresh deadline.
pub const MANAGEMENT_SNAPSHOT_REFRESH_TIMEOUT: Duration = Duration::from_millis(1_500);
/// Immutable aggregate cache lifetime.
pub const MANAGEMENT_SNAPSHOT_CACHE_TTL: Duration = Duration::from_millis(1_000);

/// Request sent over the authenticated cluster channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementSnapshotRequest {
    pub schema_version: u16,
    pub authority_epoch: u64,
}

/// Raw local consensus positions carried only inside the cluster trust boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementConsensusObservation {
    pub commit_index: u64,
    pub applied_index: u64,
    pub last_snapshot_index: Option<u64>,
    pub catch_up_target: u64,
    pub voter: bool,
}

/// One immutable local observation returned by a member.
///
/// `node_id` is intentionally internal. Public management routes must convert it
/// to `OpaqueNodeIdentity` before serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementMemberSnapshot {
    pub schema_version: u16,
    pub node_id: String,
    pub generation: u64,
    pub authority_epoch: u64,
    pub observation_seq: u64,
    pub consensus: Option<ManagementConsensusObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imap: Option<ManagementIMapObservation>,
}

/// Evidence for one admitted process; partition counts describe that process's view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementIMapObservation {
    pub readiness: IMapReadiness,
    pub observation_sequence: u64,
    pub age_ms: u64,
    pub complete: bool,
    pub partitions: Option<IMapPartitionCounts>,
}

impl From<IMapEvidence> for ManagementIMapObservation {
    fn from(evidence: IMapEvidence) -> Self {
        Self {
            readiness: evidence.readiness,
            observation_sequence: evidence.sequence,
            age_ms: evidence.age_ms,
            complete: evidence.complete,
            partitions: evidence
                .complete
                .then(|| IMapPartitionCounts::from_rows(&evidence.observation.rows)),
        }
    }
}

impl ManagementIMapObservation {
    pub(crate) fn aged(mut self, elapsed: Duration) -> Self {
        self.age_ms = self.age_ms.saturating_add(elapsed.as_millis() as u64);
        if self.age_ms > HEALTH_TTL.as_millis() as u64 {
            self.complete = false;
            self.partitions = None;
            self.readiness.ready = false;
            self.readiness.reason = IMapReadinessReason::EpochStale;
        }
        self
    }
}

impl ManagementMemberSnapshot {
    fn validate(&self, schema_version: u16) -> Result<(), ManagementAggregationIssue> {
        if self.schema_version != schema_version || !supports_snapshot_schema(schema_version) {
            return Err(ManagementAggregationIssue::Incompatible);
        }
        if self.imap.is_some() && schema_version == MANAGEMENT_API_SCHEMA_VERSION {
            return Err(ManagementAggregationIssue::Malformed);
        }
        if self.node_id.is_empty() || self.node_id.len() > 256 {
            return Err(ManagementAggregationIssue::Malformed);
        }
        if let Some(progress) = &self.consensus {
            if progress.applied_index > progress.commit_index
                || progress
                    .last_snapshot_index
                    .is_some_and(|index| index > progress.applied_index)
                || progress.catch_up_target < progress.applied_index
            {
                return Err(ManagementAggregationIssue::Malformed);
            }
        }
        if let Some(imap) = &self.imap {
            if imap.readiness.epoch != self.authority_epoch {
                return Err(ManagementAggregationIssue::EpochMismatch);
            }
            if imap.readiness.ready != (imap.readiness.reason == IMapReadinessReason::Ready)
                || imap.readiness.ready && !imap.complete
                || imap.complete != imap.partitions.is_some()
                || imap
                    .partitions
                    .is_some_and(|counts| counts.total() != u64::from(D2_PARTITION_COUNT.get()))
            {
                return Err(ManagementAggregationIssue::Malformed);
            }
        }
        Ok(())
    }
}

fn supports_snapshot_schema(version: u16) -> bool {
    matches!(
        version,
        MANAGEMENT_API_SCHEMA_VERSION | IMAP_SNAPSHOT_SCHEMA_VERSION
    )
}

/// Runtime seam used by the protected route. Implementations must only read.
pub trait LocalManagementSnapshotProvider: fmt::Debug + Send + Sync + 'static {
    fn local_snapshot(&self) -> Option<ManagementMemberSnapshot>;

    fn local_imap_snapshot(&self) -> Option<ManagementMemberSnapshot> {
        self.local_snapshot().map(|mut snapshot| {
            snapshot.schema_version = IMAP_SNAPSHOT_SCHEMA_VERSION;
            snapshot
        })
    }
}

#[derive(Clone)]
struct SnapshotRpcState {
    provider: Arc<dyn LocalManagementSnapshotProvider>,
    auth: ClusterRouteAuth,
}

/// Protected, read-only member snapshot route factory.
#[derive(Clone)]
pub struct ManagementSnapshotRpcService {
    state: SnapshotRpcState,
}

impl fmt::Debug for ManagementSnapshotRpcService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagementSnapshotRpcService")
            .field("provider", &self.state.provider)
            .field("auth", &self.state.auth)
            .finish()
    }
}

impl ManagementSnapshotRpcService {
    pub fn new(provider: Arc<dyn LocalManagementSnapshotProvider>, auth: ClusterRouteAuth) -> Self {
        Self {
            state: SnapshotRpcState { provider, auth },
        }
    }

    pub fn routes(&self) -> Router {
        Router::new()
            .route(
                CLUSTER_MANAGEMENT_CAPABILITIES_PATH,
                get(snapshot_capabilities_rpc),
            )
            .route(CLUSTER_MANAGEMENT_SNAPSHOT_PATH, post(snapshot_rpc))
            .layer(DefaultBodyLimit::max(MANAGEMENT_SNAPSHOT_MAX_REQUEST_BYTES))
            .with_state(self.state.clone())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagementSnapshotCapabilities {
    snapshot_schema_versions: Vec<u16>,
}

async fn snapshot_capabilities_rpc(
    State(state): State<SnapshotRpcState>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = state
        .auth
        .verify(ClusterRoute::ManagementSnapshot, &headers)
    {
        return (
            error.status,
            Json(SnapshotRpcError {
                schema_version: MANAGEMENT_API_SCHEMA_VERSION,
                code: error.code,
            }),
        )
            .into_response();
    }
    Json(ManagementSnapshotCapabilities {
        snapshot_schema_versions: vec![MANAGEMENT_API_SCHEMA_VERSION, IMAP_SNAPSHOT_SCHEMA_VERSION],
    })
    .into_response()
}

#[derive(Debug, Serialize)]
struct SnapshotRpcError {
    schema_version: u16,
    code: &'static str,
}

async fn snapshot_rpc(
    State(state): State<SnapshotRpcState>,
    headers: HeaderMap,
    Json(request): Json<ManagementSnapshotRequest>,
) -> Response {
    if let Err(error) = state
        .auth
        .verify(ClusterRoute::ManagementSnapshot, &headers)
    {
        return (
            error.status,
            Json(SnapshotRpcError {
                schema_version: MANAGEMENT_API_SCHEMA_VERSION,
                code: error.code,
            }),
        )
            .into_response();
    }
    let snapshot = match request.schema_version {
        MANAGEMENT_API_SCHEMA_VERSION => state.provider.local_snapshot(),
        IMAP_SNAPSHOT_SCHEMA_VERSION => state.provider.local_imap_snapshot(),
        _ => return rpc_error(StatusCode::NOT_ACCEPTABLE, "incompatible-schema"),
    };
    let Some(snapshot) = snapshot else {
        return rpc_error(StatusCode::SERVICE_UNAVAILABLE, "source-unavailable");
    };
    if snapshot.authority_epoch != request.authority_epoch {
        return rpc_error(StatusCode::CONFLICT, "authority-epoch-mismatch");
    }
    if snapshot.validate(request.schema_version).is_err() {
        return rpc_error(StatusCode::SERVICE_UNAVAILABLE, "invalid-local-snapshot");
    }
    let Ok(body) = serde_json::to_vec(&snapshot) else {
        return rpc_error(StatusCode::INTERNAL_SERVER_ERROR, "serialization-failed");
    };
    if body.len() > MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES {
        return rpc_error(StatusCode::SERVICE_UNAVAILABLE, "response-too-large");
    }
    (StatusCode::OK, [("content-type", "application/json")], body).into_response()
}

fn rpc_error(status: StatusCode, code: &'static str) -> Response {
    (
        status,
        Json(SnapshotRpcError {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            code,
        }),
    )
        .into_response()
}

/// One peer selected from the committed roster and resolved server-side.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManagementPeerTarget {
    pub node_id: String,
    pub generation: u64,
    pub endpoint: String,
    /// Snapshot schema advertised by membership capability discovery.
    /// `None` represents a pre-0.72 member and is never sent the new RPC.
    pub management_schema_version: Option<u16>,
}

/// Stable reason why a committed member did not contribute to an aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManagementAggregationIssue {
    MissingEndpoint,
    Timeout,
    Transport,
    Oversize,
    Malformed,
    Incompatible,
    IdentityMismatch,
    GenerationMismatch,
    EpochMismatch,
    Duplicate,
    Stale,
    TruncatedRoster,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagementPeerFailure {
    pub node_id: String,
    pub generation: u64,
    pub issue: ManagementAggregationIssue,
}

/// Immutable deterministic aggregate. Raw IDs never leave this internal type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedManagementSnapshot {
    pub authority_epoch: u64,
    pub observation_seq: u64,
    pub members: Vec<ManagementMemberSnapshot>,
    pub failures: Vec<ManagementPeerFailure>,
    pub truncated: bool,
}

impl AggregatedManagementSnapshot {
    pub fn complete(&self) -> bool {
        self.failures.is_empty() && !self.truncated
    }
}

/// Transport seam used by deterministic and process tests.
#[async_trait]
pub trait ManagementPeerTransport: fmt::Debug + Send + Sync + 'static {
    async fn fetch(
        &self,
        target: &ManagementPeerTarget,
        request: ManagementSnapshotRequest,
    ) -> Result<Vec<u8>, ManagementAggregationIssue>;
}

/// Bounded authenticated HTTP implementation of the peer snapshot transport.
#[derive(Clone)]
pub struct HttpManagementPeerTransport {
    client: reqwest::Client,
    auth: ClusterRouteAuth,
    scheme: &'static str,
}

impl fmt::Debug for HttpManagementPeerTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpManagementPeerTransport")
            .field("auth", &self.auth)
            .field("scheme", &self.scheme)
            .finish_non_exhaustive()
    }
}

impl HttpManagementPeerTransport {
    pub fn new(client: reqwest::Client, auth: ClusterRouteAuth, tls: bool) -> Self {
        Self {
            client,
            auth,
            scheme: if tls { "https" } else { "http" },
        }
    }
}

#[async_trait]
impl ManagementPeerTransport for HttpManagementPeerTransport {
    async fn fetch(
        &self,
        target: &ManagementPeerTarget,
        request: ManagementSnapshotRequest,
    ) -> Result<Vec<u8>, ManagementAggregationIssue> {
        let mut headers = HeaderMap::new();
        self.auth
            .apply_outbound_headers(&mut headers)
            .map_err(|_| ManagementAggregationIssue::Transport)?;
        let capability_response = self
            .client
            .get(format!(
                "{}://{}{}",
                self.scheme, target.endpoint, CLUSTER_MANAGEMENT_CAPABILITIES_PATH
            ))
            .headers(headers.clone())
            .send()
            .await
            .map_err(|_| ManagementAggregationIssue::Transport)?;
        if !capability_response.status().is_success() {
            return Err(match capability_response.status() {
                StatusCode::NOT_FOUND | StatusCode::NOT_ACCEPTABLE => {
                    ManagementAggregationIssue::Incompatible
                }
                _ => ManagementAggregationIssue::Transport,
            });
        }
        let capability_bytes =
            read_bounded(capability_response, MANAGEMENT_SNAPSHOT_MAX_REQUEST_BYTES).await?;
        let capabilities: ManagementSnapshotCapabilities =
            serde_json::from_slice(&capability_bytes)
                .map_err(|_| ManagementAggregationIssue::Malformed)?;
        if !capabilities
            .snapshot_schema_versions
            .contains(&request.schema_version)
        {
            return Err(ManagementAggregationIssue::Incompatible);
        }
        let response = self
            .client
            .post(format!(
                "{}://{}{}",
                self.scheme, target.endpoint, CLUSTER_MANAGEMENT_SNAPSHOT_PATH
            ))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|_| ManagementAggregationIssue::Transport)?;
        if !response.status().is_success() {
            return Err(match response.status() {
                StatusCode::CONFLICT => ManagementAggregationIssue::EpochMismatch,
                StatusCode::NOT_ACCEPTABLE => ManagementAggregationIssue::Incompatible,
                _ => ManagementAggregationIssue::Transport,
            });
        }
        read_bounded(response, MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES).await
    }
}

async fn read_bounded(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, ManagementAggregationIssue> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(ManagementAggregationIssue::Oversize);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ManagementAggregationIssue::Transport)?
    {
        if bytes.len().saturating_add(chunk.len()) > max_bytes {
            return Err(ManagementAggregationIssue::Oversize);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[derive(Debug, Clone)]
struct CachedAggregate {
    inserted: Instant,
    started: Instant,
    roster_key: ManagementRosterKey,
    value: Arc<AggregatedManagementSnapshot>,
}

impl CachedAggregate {
    fn matches_imap(&self, local: &ManagementMemberSnapshot) -> bool {
        let cached = self
            .value
            .members
            .iter()
            .find(|member| member.node_id == local.node_id)
            .and_then(|member| member.imap.as_ref());
        let identity = |imap: &ManagementIMapObservation| {
            (imap.readiness, imap.complete, imap.observation_sequence)
        };
        cached.map(identity) == local.imap.as_ref().map(identity)
    }
}

type ManagementRosterKey = Vec<(String, u64, Option<u16>)>;

#[derive(Debug, Default)]
struct AggregatorState {
    cache: Option<CachedAggregate>,
}

/// Coalescing, deadline-bound aggregate refresher.
#[derive(Debug)]
pub struct ManagementSnapshotAggregator {
    transport: Arc<dyn ManagementPeerTransport>,
    state: AsyncMutex<AggregatorState>,
    imap_state: AsyncMutex<AggregatorState>,
    ttl: Duration,
    peer_timeout: Duration,
    refresh_timeout: Duration,
    concurrency: usize,
}

impl ManagementSnapshotAggregator {
    pub fn new(transport: Arc<dyn ManagementPeerTransport>) -> Self {
        Self {
            transport,
            state: AsyncMutex::new(AggregatorState::default()),
            imap_state: AsyncMutex::new(AggregatorState::default()),
            ttl: MANAGEMENT_SNAPSHOT_CACHE_TTL,
            peer_timeout: MANAGEMENT_SNAPSHOT_PEER_TIMEOUT,
            refresh_timeout: MANAGEMENT_SNAPSHOT_REFRESH_TIMEOUT,
            concurrency: MANAGEMENT_SNAPSHOT_MAX_CONCURRENCY,
        }
    }

    /// Override bounds for deterministic tests and specialized embeddings.
    pub fn with_limits(
        mut self,
        ttl: Duration,
        peer_timeout: Duration,
        refresh_timeout: Duration,
        concurrency: usize,
    ) -> Self {
        self.ttl = ttl;
        self.peer_timeout = peer_timeout;
        self.refresh_timeout = refresh_timeout;
        self.concurrency = concurrency.max(1);
        self
    }

    pub async fn refresh(
        &self,
        local: ManagementMemberSnapshot,
        committed_targets: Vec<ManagementPeerTarget>,
    ) -> Arc<AggregatedManagementSnapshot> {
        self.refresh_schema(local, committed_targets, MANAGEMENT_API_SCHEMA_VERSION)
            .await
            .0
    }

    pub(crate) async fn refresh_imap(
        &self,
        local: ManagementMemberSnapshot,
        committed_targets: Vec<ManagementPeerTarget>,
    ) -> (Arc<AggregatedManagementSnapshot>, Instant) {
        self.refresh_schema(local, committed_targets, IMAP_SNAPSHOT_SCHEMA_VERSION)
            .await
    }

    async fn refresh_schema(
        &self,
        local: ManagementMemberSnapshot,
        committed_targets: Vec<ManagementPeerTarget>,
        schema_version: u16,
    ) -> (Arc<AggregatedManagementSnapshot>, Instant) {
        let started = Instant::now();
        let mut guard = if schema_version == IMAP_SNAPSHOT_SCHEMA_VERSION {
            self.imap_state.lock().await
        } else {
            self.state.lock().await
        };
        let (targets, roster_key, roster_truncated) = normalize_roster(&local, committed_targets);
        if let Some(cached) = guard.cache.as_ref().filter(|cached| {
            cached.inserted.elapsed() < self.ttl
                && cached.value.authority_epoch == local.authority_epoch
                && cached.value.observation_seq == local.observation_seq
                && cached.roster_key == roster_key
                && (schema_version == MANAGEMENT_API_SCHEMA_VERSION || cached.matches_imap(&local))
        }) {
            return (Arc::clone(&cached.value), cached.started);
        }

        if guard.cache.as_ref().is_some_and(|cached| {
            cached.value.authority_epoch > local.authority_epoch
                || (cached.value.authority_epoch == local.authority_epoch
                    && cached.value.observation_seq > local.observation_seq)
        }) {
            let cache = guard.cache.as_ref().expect("checked cache");
            return (Arc::clone(&cache.value), cache.started);
        }

        let previous = guard.cache.as_ref().map(|cached| Arc::clone(&cached.value));
        let local_validation = local.validate(schema_version);
        let cacheable = local_validation.is_ok();
        let value = if let Err(issue) = local_validation {
            Arc::new(AggregatedManagementSnapshot {
                authority_epoch: local.authority_epoch,
                observation_seq: local.observation_seq,
                members: Vec::new(),
                failures: vec![ManagementPeerFailure {
                    node_id: local.node_id.clone(),
                    generation: local.generation,
                    issue,
                }],
                truncated: roster_truncated,
            })
        } else {
            Arc::new(
                self.collect(local, targets, roster_truncated, previous.as_deref())
                    .await,
            )
        };
        if cacheable {
            guard.cache = Some(CachedAggregate {
                inserted: Instant::now(),
                started,
                roster_key,
                value: Arc::clone(&value),
            });
        }
        (value, started)
    }

    async fn collect(
        &self,
        local: ManagementMemberSnapshot,
        targets: Vec<ManagementPeerTarget>,
        roster_truncated: bool,
        previous: Option<&AggregatedManagementSnapshot>,
    ) -> AggregatedManagementSnapshot {
        let epoch = local.authority_epoch;
        let request = ManagementSnapshotRequest {
            schema_version: local.schema_version,
            authority_epoch: epoch,
        };
        let transport = Arc::clone(&self.transport);
        let peer_timeout = self.peer_timeout;
        let mut pending = targets
            .iter()
            .map(|target| (target.node_id.clone(), target.generation))
            .collect::<BTreeSet<_>>();
        let mut futures = stream::iter(targets.into_iter().map(move |target| {
            let transport = Arc::clone(&transport);
            async move {
                let result =
                    if target.management_schema_version != Some(MANAGEMENT_API_SCHEMA_VERSION) {
                        Err(ManagementAggregationIssue::Incompatible)
                    } else if target.endpoint.trim().is_empty() {
                        Err(ManagementAggregationIssue::MissingEndpoint)
                    } else {
                        tokio::time::timeout(peer_timeout, transport.fetch(&target, request))
                            .await
                            .map_err(|_| ManagementAggregationIssue::Timeout)
                            .and_then(|result| result)
                            .and_then(|bytes| decode_peer(&target, request, &bytes))
                    };
                (target, result)
            }
        }))
        .buffer_unordered(self.concurrency);

        let deadline = tokio::time::sleep(self.refresh_timeout);
        tokio::pin!(deadline);
        let mut members = vec![local.clone()];
        let mut failures = Vec::new();
        loop {
            tokio::select! {
                biased;
                result = futures.next() => {
                    let Some((target, result)) = result else {
                        break;
                    };
                    pending.remove(&(target.node_id.clone(), target.generation));
                    match result {
                        Ok(snapshot) => members.push(snapshot),
                        Err(issue) => failures.push(ManagementPeerFailure {
                            node_id: target.node_id,
                            generation: target.generation,
                            issue,
                        }),
                    }
                }
                () = &mut deadline => break,
            }
        }
        failures.extend(
            pending
                .into_iter()
                .map(|(node_id, generation)| ManagementPeerFailure {
                    node_id,
                    generation,
                    issue: ManagementAggregationIssue::Timeout,
                }),
        );
        if let Some(previous) = previous.filter(|previous| previous.authority_epoch == epoch) {
            let previous_members = previous
                .members
                .iter()
                .map(|snapshot| {
                    (
                        (snapshot.node_id.clone(), snapshot.generation),
                        snapshot.clone(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            for snapshot in &mut members {
                let key = (snapshot.node_id.clone(), snapshot.generation);
                if let Some(prior) = previous_members.get(&key).filter(|prior| {
                    prior.observation_seq > snapshot.observation_seq
                        || prior.imap.as_ref().zip(snapshot.imap.as_ref()).is_some_and(
                            |(prior, current)| {
                                prior.observation_sequence > current.observation_sequence
                            },
                        )
                }) {
                    failures.push(ManagementPeerFailure {
                        node_id: snapshot.node_id.clone(),
                        generation: snapshot.generation,
                        issue: ManagementAggregationIssue::Stale,
                    });
                    if request.schema_version == MANAGEMENT_API_SCHEMA_VERSION {
                        *snapshot = prior.clone();
                    } else {
                        snapshot.imap = None;
                    }
                }
            }
        }
        if roster_truncated {
            failures.push(ManagementPeerFailure {
                node_id: "roster".to_owned(),
                generation: 0,
                issue: ManagementAggregationIssue::TruncatedRoster,
            });
        }
        reduce(
            epoch,
            local.observation_seq,
            members,
            failures,
            roster_truncated,
        )
    }
}

fn normalize_roster(
    local: &ManagementMemberSnapshot,
    targets: Vec<ManagementPeerTarget>,
) -> (Vec<ManagementPeerTarget>, ManagementRosterKey, bool) {
    let mut unique = BTreeMap::<(String, u64), ManagementPeerTarget>::new();
    for target in targets {
        if target.node_id == local.node_id && target.generation == local.generation {
            continue;
        }
        unique
            .entry((target.node_id.clone(), target.generation))
            .and_modify(|current| {
                if target.endpoint < current.endpoint {
                    current.endpoint = target.endpoint.clone();
                }
                if current.management_schema_version != target.management_schema_version {
                    current.management_schema_version = None;
                }
            })
            .or_insert(target);
    }
    let mut targets = unique.into_values().collect::<Vec<_>>();
    let truncated = targets.len() > MANAGEMENT_SNAPSHOT_MAX_PEERS.saturating_sub(1);
    targets.truncate(MANAGEMENT_SNAPSHOT_MAX_PEERS.saturating_sub(1));
    let mut roster_key = vec![(
        local.node_id.clone(),
        local.generation,
        Some(MANAGEMENT_API_SCHEMA_VERSION),
    )];
    roster_key.extend(targets.iter().map(|target| {
        (
            target.node_id.clone(),
            target.generation,
            target.management_schema_version,
        )
    }));
    roster_key.sort();
    (targets, roster_key, truncated)
}

fn decode_peer(
    target: &ManagementPeerTarget,
    request: ManagementSnapshotRequest,
    bytes: &[u8],
) -> Result<ManagementMemberSnapshot, ManagementAggregationIssue> {
    if bytes.len() > MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES {
        return Err(ManagementAggregationIssue::Oversize);
    }
    let snapshot: ManagementMemberSnapshot =
        serde_json::from_slice(bytes).map_err(|_| ManagementAggregationIssue::Malformed)?;
    snapshot.validate(request.schema_version)?;
    if snapshot.node_id != target.node_id {
        return Err(ManagementAggregationIssue::IdentityMismatch);
    }
    if snapshot.generation != target.generation {
        return Err(ManagementAggregationIssue::GenerationMismatch);
    }
    if snapshot.authority_epoch != request.authority_epoch {
        return Err(ManagementAggregationIssue::EpochMismatch);
    }
    Ok(snapshot)
}

fn reduce(
    authority_epoch: u64,
    observation_seq: u64,
    snapshots: Vec<ManagementMemberSnapshot>,
    mut failures: Vec<ManagementPeerFailure>,
    truncated: bool,
) -> AggregatedManagementSnapshot {
    let mut selected = BTreeMap::<(String, u64), ManagementMemberSnapshot>::new();
    let mut duplicates = BTreeSet::new();
    for snapshot in snapshots {
        let key = (snapshot.node_id.clone(), snapshot.generation);
        match selected.get(&key) {
            Some(current) if snapshot_rank(current) >= snapshot_rank(&snapshot) => {
                duplicates.insert(key);
            }
            Some(_) => {
                duplicates.insert(key.clone());
                selected.insert(key, snapshot);
            }
            None => {
                selected.insert(key, snapshot);
            }
        }
    }
    failures.extend(
        duplicates
            .into_iter()
            .map(|(node_id, generation)| ManagementPeerFailure {
                node_id,
                generation,
                issue: ManagementAggregationIssue::Duplicate,
            }),
    );
    failures.sort_by(|left, right| {
        left.node_id
            .cmp(&right.node_id)
            .then(left.generation.cmp(&right.generation))
            .then(left.issue.cmp(&right.issue))
    });
    failures.dedup();
    AggregatedManagementSnapshot {
        authority_epoch,
        observation_seq,
        members: selected.into_values().collect(),
        failures,
        truncated,
    }
}

fn snapshot_rank(snapshot: &ManagementMemberSnapshot) -> (u64, Vec<u8>) {
    (
        snapshot.observation_seq,
        serde_json::to_vec(snapshot).unwrap_or_default(),
    )
}

/// Test and embedding provider backed by an atomically replaced snapshot.
#[derive(Debug, Default)]
pub struct RetainedLocalManagementSnapshot {
    snapshot: Mutex<Option<ManagementMemberSnapshot>>,
}

impl RetainedLocalManagementSnapshot {
    pub fn replace(&self, snapshot: ManagementMemberSnapshot) {
        *self.snapshot.lock().expect("local snapshot mutex") = Some(snapshot);
    }
}

impl LocalManagementSnapshotProvider for RetainedLocalManagementSnapshot {
    fn local_snapshot(&self) -> Option<ManagementMemberSnapshot> {
        self.snapshot.lock().expect("local snapshot mutex").clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use proptest::prelude::*;

    use super::*;

    #[derive(Debug)]
    struct FakeTransport {
        replies: BTreeMap<String, Result<Vec<u8>, ManagementAggregationIssue>>,
        calls: AtomicUsize,
        delay: Duration,
    }

    #[derive(Debug)]
    struct CancellationTransport {
        active: Arc<AtomicUsize>,
    }

    #[derive(Debug)]
    struct MutableTransport {
        reply: Mutex<Vec<u8>>,
    }

    #[async_trait]
    impl ManagementPeerTransport for MutableTransport {
        async fn fetch(
            &self,
            _target: &ManagementPeerTarget,
            _request: ManagementSnapshotRequest,
        ) -> Result<Vec<u8>, ManagementAggregationIssue> {
            Ok(self.reply.lock().expect("mutable reply mutex").clone())
        }
    }

    struct ActiveGuard(Arc<AtomicUsize>);

    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl ManagementPeerTransport for CancellationTransport {
        async fn fetch(
            &self,
            _target: &ManagementPeerTarget,
            _request: ManagementSnapshotRequest,
        ) -> Result<Vec<u8>, ManagementAggregationIssue> {
            self.active.fetch_add(1, Ordering::SeqCst);
            let _guard = ActiveGuard(Arc::clone(&self.active));
            std::future::pending().await
        }
    }

    #[async_trait]
    impl ManagementPeerTransport for FakeTransport {
        async fn fetch(
            &self,
            target: &ManagementPeerTarget,
            _request: ManagementSnapshotRequest,
        ) -> Result<Vec<u8>, ManagementAggregationIssue> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            self.replies
                .get(&target.node_id)
                .cloned()
                .unwrap_or(Err(ManagementAggregationIssue::Transport))
        }
    }

    fn member(node: &str, generation: u64, epoch: u64, seq: u64) -> ManagementMemberSnapshot {
        ManagementMemberSnapshot {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            node_id: node.to_owned(),
            generation,
            authority_epoch: epoch,
            observation_seq: seq,
            imap: None,
            consensus: Some(ManagementConsensusObservation {
                commit_index: seq,
                applied_index: seq,
                last_snapshot_index: Some(seq.saturating_sub(1)),
                catch_up_target: seq,
                voter: true,
            }),
        }
    }

    fn target(node: &str, generation: u64, endpoint: &str) -> ManagementPeerTarget {
        ManagementPeerTarget {
            node_id: node.to_owned(),
            generation,
            endpoint: endpoint.to_owned(),
            management_schema_version: Some(MANAGEMENT_API_SCHEMA_VERSION),
        }
    }

    fn encoded(snapshot: &ManagementMemberSnapshot) -> Vec<u8> {
        serde_json::to_vec(snapshot).unwrap()
    }

    fn imap_member(node: &str, generation: u64) -> ManagementMemberSnapshot {
        let mut report = member(node, generation, 7, 5);
        report.schema_version = IMAP_SNAPSHOT_SCHEMA_VERSION;
        report.consensus = None;
        report.imap = Some(ManagementIMapObservation {
            readiness: IMapReadiness {
                ready: true,
                epoch: 7,
                reason: IMapReadinessReason::Ready,
            },
            observation_sequence: 1,
            age_ms: 200,
            complete: true,
            partitions: Some(IMapPartitionCounts {
                empty: D2_PARTITION_COUNT.get(),
                ..Default::default()
            }),
        });
        report
    }

    #[test]
    fn negotiated_imap_observations_are_checked_and_age_out() {
        let request = ManagementSnapshotRequest {
            schema_version: IMAP_SNAPSHOT_SCHEMA_VERSION,
            authority_epoch: 7,
        };
        let expected = target("b", 2, "b:2");
        let legacy = member("b", 2, 7, 5);
        assert!(serde_json::to_value(&legacy).unwrap().get("imap").is_none());
        assert_eq!(
            decode_peer(&expected, request, &encoded(&legacy)),
            Err(ManagementAggregationIssue::Incompatible)
        );
        let mut report = imap_member("b", 2);
        assert!(decode_peer(&expected, request, &encoded(&report)).is_ok());
        let valid = report.imap.clone().unwrap();
        assert!(valid.clone().aged(Duration::from_millis(2_800)).complete);
        let stale = valid.aged(Duration::from_millis(2_801));
        assert_eq!(stale.readiness.reason, IMapReadinessReason::EpochStale);
        assert!(!stale.readiness.ready && !stale.complete && stale.partitions.is_none());
        report.imap.as_mut().unwrap().readiness.epoch = 6;
        assert_eq!(
            decode_peer(&expected, request, &encoded(&report)),
            Err(ManagementAggregationIssue::EpochMismatch)
        );
        report.imap.as_mut().unwrap().readiness.epoch = 7;
        report
            .imap
            .as_mut()
            .unwrap()
            .partitions
            .as_mut()
            .unwrap()
            .empty = u32::MAX;
        assert_eq!(
            decode_peer(&expected, request, &encoded(&report)),
            Err(ManagementAggregationIssue::Malformed)
        );
    }

    #[tokio::test]
    async fn imap_refresh_has_an_independent_cache_and_observation_fence() {
        let transport = Arc::new(FakeTransport {
            replies: BTreeMap::from([("b".to_owned(), Ok(encoded(&imap_member("b", 2))))]),
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
        });
        let aggregator = ManagementSnapshotAggregator::new(transport.clone());
        let targets = vec![target("b", 2, "b:2")];
        let local = imap_member("a", 1);
        let (first, started) = aggregator
            .refresh_imap(local.clone(), targets.clone())
            .await;
        assert!(first.complete());
        assert_eq!(
            first.members[1].imap.as_ref().unwrap().observation_sequence,
            1
        );
        let legacy = aggregator
            .refresh(member("a", 1, 7, 5), targets.clone())
            .await;
        assert_eq!(
            legacy.failures[0].issue,
            ManagementAggregationIssue::Incompatible
        );
        let (cached, origin) = aggregator
            .refresh_imap(local.clone(), targets.clone())
            .await;
        assert!(Arc::ptr_eq(&first, &cached));
        assert_eq!(origin, started);
        let mut local = local;
        local.imap.as_mut().unwrap().observation_sequence = 2;
        let (changed, _) = aggregator.refresh_imap(local, targets).await;
        assert_eq!(
            changed.members[0]
                .imap
                .as_ref()
                .unwrap()
                .observation_sequence,
            2
        );
        assert_eq!(transport.calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn reducer_is_order_independent_and_retains_newest_duplicate() {
        let local = member("a", 1, 7, 10);
        let old = member("b", 2, 7, 8);
        let new = member("b", 2, 7, 9);
        let one = reduce(
            7,
            10,
            vec![local.clone(), old.clone(), new.clone()],
            Vec::new(),
            false,
        );
        let two = reduce(7, 10, vec![new, local, old], Vec::new(), false);
        assert_eq!(one, two);
        assert_eq!(one.members[1].observation_seq, 9);
        assert_eq!(one.failures[0].issue, ManagementAggregationIssue::Duplicate);
    }

    proptest! {
        #[test]
        fn reducer_matches_reverse_order_for_random_duplicates(
            observations in prop::collection::vec((0_u8..8, 0_u64..100), 1..80)
        ) {
            let snapshots = observations
                .into_iter()
                .map(|(node, seq)| member(&format!("node-{node}"), u64::from(node % 3), 7, seq))
                .collect::<Vec<_>>();
            let mut reversed = snapshots.clone();
            reversed.reverse();

            let forward = reduce(7, 100, snapshots, Vec::new(), false);
            let backward = reduce(7, 100, reversed, Vec::new(), false);

            prop_assert_eq!(forward, backward);
        }
    }

    #[test]
    fn roster_deduplicates_identity_and_never_invents_browser_targets() {
        let local = member("a", 1, 7, 10);
        let (targets, key, truncated) = normalize_roster(
            &local,
            vec![
                target("a", 1, "browser-supplied:1"),
                target("b", 2, "z:2"),
                target("b", 2, "a:2"),
            ],
        );
        assert!(!truncated);
        assert_eq!(targets, vec![target("b", 2, "a:2")]);
        assert_eq!(
            key,
            vec![
                ("a".to_owned(), 1, Some(MANAGEMENT_API_SCHEMA_VERSION)),
                ("b".to_owned(), 2, Some(MANAGEMENT_API_SCHEMA_VERSION)),
            ]
        );
    }

    #[test]
    fn peer_decoder_fences_identity_generation_epoch_schema_and_size() {
        let expected = target("b", 2, "b:2");
        let request = ManagementSnapshotRequest {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            authority_epoch: 7,
        };
        assert!(decode_peer(&expected, request, &encoded(&member("b", 2, 7, 1))).is_ok());
        assert_eq!(
            decode_peer(&expected, request, &encoded(&member("x", 2, 7, 1))),
            Err(ManagementAggregationIssue::IdentityMismatch)
        );
        assert_eq!(
            decode_peer(&expected, request, &encoded(&member("b", 3, 7, 1))),
            Err(ManagementAggregationIssue::GenerationMismatch)
        );
        assert_eq!(
            decode_peer(&expected, request, &encoded(&member("b", 2, 6, 1))),
            Err(ManagementAggregationIssue::EpochMismatch)
        );
        let mut incompatible = member("b", 2, 7, 1);
        incompatible.schema_version += 1;
        assert_eq!(
            decode_peer(&expected, request, &encoded(&incompatible)),
            Err(ManagementAggregationIssue::Incompatible)
        );
        assert_eq!(
            decode_peer(
                &expected,
                request,
                &vec![b'x'; MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES + 1]
            ),
            Err(ManagementAggregationIssue::Oversize)
        );
    }

    #[tokio::test]
    async fn refresh_is_bounded_partial_and_deterministically_sorted() {
        let transport = Arc::new(FakeTransport {
            replies: BTreeMap::from([
                ("b".to_owned(), Ok(encoded(&member("b", 2, 7, 4)))),
                ("c".to_owned(), Err(ManagementAggregationIssue::Transport)),
            ]),
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
        });
        let aggregator = ManagementSnapshotAggregator::new(transport.clone());
        let result = aggregator
            .refresh(
                member("a", 1, 7, 5),
                vec![
                    target("c", 3, "c:3"),
                    target("d", 4, ""),
                    target("b", 2, "b:2"),
                ],
            )
            .await;
        assert_eq!(
            result
                .members
                .iter()
                .map(|item| item.node_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(transport.calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            result
                .failures
                .iter()
                .map(|failure| (failure.node_id.clone(), failure.issue))
                .collect::<Vec<_>>(),
            vec![
                ("c".to_owned(), ManagementAggregationIssue::Transport),
                ("d".to_owned(), ManagementAggregationIssue::MissingEndpoint),
            ]
        );
        assert!(!result.complete());
    }

    #[tokio::test]
    async fn canary_management_fanout_coalesces_and_late_epoch_cannot_replace_cache() {
        let transport = Arc::new(FakeTransport {
            replies: BTreeMap::from([("b".to_owned(), Ok(encoded(&member("b", 2, 8, 11))))]),
            calls: AtomicUsize::new(0),
            delay: Duration::from_millis(25),
        });
        let aggregator = Arc::new(ManagementSnapshotAggregator::new(transport.clone()));
        let targets = vec![target("b", 2, "b:2")];
        let first = {
            let aggregator = Arc::clone(&aggregator);
            let targets = targets.clone();
            tokio::spawn(async move { aggregator.refresh(member("a", 1, 8, 12), targets).await })
        };
        let second = {
            let aggregator = Arc::clone(&aggregator);
            let targets = targets.clone();
            tokio::spawn(async move { aggregator.refresh(member("a", 1, 8, 12), targets).await })
        };
        let (first, second) = tokio::join!(first, second);
        assert!(Arc::ptr_eq(&first.unwrap(), &second.unwrap()));
        assert_eq!(transport.calls.load(Ordering::SeqCst), 1);

        let stale = if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W3") {
            Arc::new(reduce(
                7,
                99,
                vec![member("a", 1, 7, 99)],
                Vec::new(),
                false,
            ))
        } else {
            aggregator.refresh(member("a", 1, 7, 99), targets).await
        };
        assert_eq!(
            stale.authority_epoch, 8,
            "HC-CANARY-RED:MC72-W3 late older epoch replaced current snapshot"
        );
        assert_eq!(stale.observation_seq, 12);
        assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn peer_timeout_cancels_future_and_publishes_partial_evidence() {
        let transport = Arc::new(FakeTransport {
            replies: BTreeMap::from([("b".to_owned(), Ok(encoded(&member("b", 2, 7, 1))))]),
            calls: AtomicUsize::new(0),
            delay: Duration::from_millis(50),
        });
        let aggregator = ManagementSnapshotAggregator::new(transport).with_limits(
            Duration::from_secs(1),
            Duration::from_millis(5),
            Duration::from_millis(20),
            1,
        );
        let result = aggregator
            .refresh(member("a", 1, 7, 2), vec![target("b", 2, "b:2")])
            .await;
        assert_eq!(result.members.len(), 1);
        assert_eq!(
            result.failures[0].issue,
            ManagementAggregationIssue::Timeout
        );
    }

    #[tokio::test]
    async fn timeout_cancellation_releases_all_inflight_peer_futures() {
        let active = Arc::new(AtomicUsize::new(0));
        let transport = Arc::new(CancellationTransport {
            active: Arc::clone(&active),
        });
        let aggregator = ManagementSnapshotAggregator::new(transport).with_limits(
            Duration::from_secs(1),
            Duration::from_millis(10),
            Duration::from_millis(20),
            2,
        );

        let result = aggregator
            .refresh(
                member("a", 1, 7, 2),
                vec![target("b", 2, "b:2"), target("c", 3, "c:3")],
            )
            .await;

        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(result.failures.len(), 2);
        assert!(result
            .failures
            .iter()
            .all(|failure| failure.issue == ManagementAggregationIssue::Timeout));
    }

    #[tokio::test]
    async fn late_same_epoch_peer_observation_is_marked_stale_and_cannot_regress() {
        let transport = Arc::new(MutableTransport {
            reply: Mutex::new(encoded(&member("b", 2, 7, 10))),
        });
        let aggregator = ManagementSnapshotAggregator::new(transport.clone()).with_limits(
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_secs(1),
            1,
        );
        let first = aggregator
            .refresh(member("a", 1, 7, 10), vec![target("b", 2, "b:2")])
            .await;
        assert_eq!(first.members[1].observation_seq, 10);

        *transport.reply.lock().unwrap() = encoded(&member("b", 2, 7, 9));
        let second = aggregator
            .refresh(member("a", 1, 7, 11), vec![target("b", 2, "b:2")])
            .await;

        assert_eq!(second.members[1].observation_seq, 10);
        assert!(second.failures.iter().any(|failure| {
            failure.node_id == "b" && failure.issue == ManagementAggregationIssue::Stale
        }));
    }
}
