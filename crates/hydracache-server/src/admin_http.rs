use std::sync::{Arc, Mutex};

use axum::extract::{Path as AxumPath, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_client_transport_axum::{
    ClientSurfaceDiagnosticReset, HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use serde::Serialize;
use thiserror::Error;

use crate::bootstrap::{ServerAdminActionError, ServerRuntime};
use crate::cluster_status::RaftCompactionError;
use crate::hc2::Hc2ClientPlaneService;
use crate::management_security::ManagementReadLimiter;
use crate::services::DrainOutcome;
use hydracache_observability::PrometheusExporter;

/// Liveness path used by Kubernetes probes.
pub const ADMIN_HEALTHZ_PATH: &str = "/healthz";
/// Readiness path used by Kubernetes probes.
pub const ADMIN_READYZ_PATH: &str = "/readyz";
/// Prometheus metrics path on the internal admin surface.
pub const ADMIN_METRICS_PATH: &str = "/metrics";
/// Read-only Management Center console path on the internal admin surface.
pub const ADMIN_CONSOLE_PATH: &str = "/console";
/// Read-only cluster overview path on the internal admin surface.
pub const ADMIN_CLUSTER_OVERVIEW_PATH: &str = "/cluster/overview";
/// Read-only per-cache actuator path on the internal admin surface.
pub const ADMIN_ACTUATOR_PATH: &str = "/actuator/hydracache";
/// Operator status path.
pub const ADMIN_STATUS_PATH: &str = "/admin/status";
/// Operator drain action path.
pub const ADMIN_DRAIN_PATH: &str = "/admin/drain";
/// Operator reshard action path.
pub const ADMIN_RESHARD_PATH: &str = "/admin/reshard";
/// Operator backup action path.
pub const ADMIN_BACKUP_PATH: &str = "/admin/backup";
/// Explicit, off-by-default disk-backed Raft compaction control and status path.
pub const ADMIN_RAFT_COMPACTION_PATH: &str = "/admin/raft/compaction";
/// Off-by-default, local-only destructive diagnostic reset path.
pub const ADMIN_DIAGNOSTIC_RESET_PATH: &str = "/admin/diagnostics/reset";
/// Privileged, read-only aggregate memory-footprint path.
pub const ADMIN_MEMORY_FOOTPRINT_PATH: &str = "/admin/memory-footprint";
/// Explicit read-only management capability header used after identity verification.
pub const HYDRACACHE_MANAGEMENT_READ_HEADER: &str = "x-hydracache-management-read";

/// Shared runtime state for the admin HTTP surface.
pub type SharedServerRuntime = Arc<Mutex<ServerRuntime>>;

/// Axum route owner for the internal admin/operator surface.
#[derive(Debug, Clone)]
pub struct AdminHttpSurface {
    runtime: SharedServerRuntime,
    hc2_metrics: Option<Hc2ClientPlaneService>,
    management_cursors: Arc<Mutex<crate::management_http::ManagementCursorStore>>,
    management_aggregator: Option<Arc<crate::management_aggregation::ManagementSnapshotAggregator>>,
    management_history: Option<crate::management_history::ManagementHistoryService>,
    management_read_limiter: ManagementReadLimiter,
}

impl AdminHttpSurface {
    /// Create an admin surface from a server runtime.
    pub fn new(runtime: ServerRuntime) -> Self {
        let management_aggregator = runtime.management_peer_transport().map(|transport| {
            Arc::new(crate::management_aggregation::ManagementSnapshotAggregator::new(transport))
        });
        let management_history = crate::management_history::ManagementHistoryService::from_config(
            &runtime.config().management_history,
        )
        .expect("validated management history config");
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            hc2_metrics: None,
            management_cursors: Arc::new(Mutex::new(Default::default())),
            management_aggregator,
            management_history,
            management_read_limiter: ManagementReadLimiter::default(),
        }
    }

    /// Create an admin surface from shared runtime state.
    pub fn from_shared(runtime: SharedServerRuntime) -> Self {
        let management_aggregator = runtime
            .lock()
            .expect("server runtime mutex")
            .management_peer_transport()
            .map(|transport| {
                Arc::new(
                    crate::management_aggregation::ManagementSnapshotAggregator::new(transport),
                )
            });
        let management_history = {
            let runtime = runtime.lock().expect("server runtime mutex");
            crate::management_history::ManagementHistoryService::from_config(
                &runtime.config().management_history,
            )
            .expect("validated management history config")
        };
        Self {
            runtime,
            hc2_metrics: None,
            management_cursors: Arc::new(Mutex::new(Default::default())),
            management_aggregator,
            management_history,
            management_read_limiter: ManagementReadLimiter::default(),
        }
    }

    /// Attach the selected production HC/2 listener to the internal metrics
    /// surface. This does not expose metrics on either public client port.
    pub fn with_hc2_metrics(mut self, service: Hc2ClientPlaneService) -> Self {
        self.hc2_metrics = Some(service);
        self
    }

    /// Return shared runtime state for tests and embedding code.
    pub fn runtime(&self) -> SharedServerRuntime {
        Arc::clone(&self.runtime)
    }

    /// Return the independent management-read limiter for diagnostics and cleanup proofs.
    pub fn management_read_limiter(&self) -> ManagementReadLimiter {
        self.management_read_limiter.clone()
    }

    /// Return the axum router for `/healthz`, `/readyz`, and `/admin/*`.
    pub fn routes(&self) -> Router {
        let runtime = self.runtime.lock().expect("server runtime mutex");
        let actuator_registry = runtime.metrics_registry();
        let management_api_enabled = runtime.config().management_api_enabled;
        drop(runtime);
        let mut routes = Router::new()
            .route(ADMIN_HEALTHZ_PATH, get(healthz))
            .route(ADMIN_READYZ_PATH, get(readyz))
            .route(ADMIN_METRICS_PATH, get(metrics))
            .route(ADMIN_CLUSTER_OVERVIEW_PATH, get(cluster_overview))
            .route(ADMIN_STATUS_PATH, get(admin_status))
            .route(ADMIN_DRAIN_PATH, get(admin_drain).post(admin_drain))
            .route(ADMIN_RESHARD_PATH, post(admin_reshard))
            .route(ADMIN_BACKUP_PATH, post(admin_backup))
            .route(ADMIN_DIAGNOSTIC_RESET_PATH, post(admin_diagnostic_reset))
            .route(ADMIN_MEMORY_FOOTPRINT_PATH, get(admin_memory_footprint))
            .route(
                ADMIN_RAFT_COMPACTION_PATH,
                get(admin_raft_compaction_status).post(admin_raft_compaction),
            )
            .with_state(Arc::clone(&self.runtime))
            .nest(
                ADMIN_ACTUATOR_PATH,
                HydraCacheActuator::routes_for(actuator_registry),
            );
        if management_api_enabled {
            routes = routes
                .route(ADMIN_CONSOLE_PATH, get(console_index))
                .route("/console/", get(console_index))
                .route("/console/index.html", get(console_index))
                .route("/console/{*asset}", get(console_dist_asset))
                .merge(crate::management_http::routes(
                    Arc::clone(&self.runtime),
                    Arc::clone(&self.management_cursors),
                    self.management_aggregator.clone(),
                    self.hc2_metrics.clone(),
                    self.management_history.clone(),
                    self.management_read_limiter.clone(),
                ));
        }
        if let Some(service) = self.hc2_metrics.clone() {
            routes.layer(Extension(service))
        } else {
            routes
        }
    }
}

async fn healthz(State(runtime): State<SharedServerRuntime>) -> Response {
    let health = runtime.lock().expect("server runtime mutex").health();
    (StatusCode::OK, Json(health)).into_response()
}

async fn readyz(State(runtime): State<SharedServerRuntime>) -> Response {
    let ready = runtime.lock().expect("server runtime mutex").ready();
    let status = if ready.ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(ready)).into_response()
}

async fn metrics(
    State(runtime): State<SharedServerRuntime>,
    hc2: Option<Extension<Hc2ClientPlaneService>>,
) -> Response {
    let registry = runtime
        .lock()
        .expect("server runtime mutex")
        .metrics_registry();
    let mut text = PrometheusExporter::new(registry).render().await;
    if let Some(Extension(service)) = hc2 {
        text.push_str(&service.prometheus_metrics());
    }
    ([(CONTENT_TYPE, "text/plain; version=0.0.4")], text).into_response()
}

async fn console_index() -> Response {
    console_asset_response("index.html")
}

async fn console_dist_asset(AxumPath(asset): AxumPath<String>) -> Response {
    console_asset_response(&asset)
}

fn console_asset_response(path: &str) -> Response {
    let Some((content_type, body)) = crate::generated_console_assets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut response = ([(CONTENT_TYPE, content_type)], body).into_response();
    let headers = response.headers_mut();
    headers.insert(
        "content-security-policy",
        "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; font-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'; object-src 'none'"
            .parse()
            .expect("static CSP is valid"),
    );
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert("referrer-policy", "no-referrer".parse().unwrap());
    headers.insert(
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()".parse().unwrap(),
    );
    headers.insert(
        "cross-origin-resource-policy",
        "same-origin".parse().unwrap(),
    );
    headers.insert("cache-control", "no-store".parse().unwrap());
    response
}

async fn cluster_overview(State(runtime): State<SharedServerRuntime>) -> Response {
    let overview = runtime
        .lock()
        .expect("server runtime mutex")
        .cluster_overview();
    (StatusCode::OK, Json(overview)).into_response()
}

async fn admin_status(State(runtime): State<SharedServerRuntime>, headers: HeaderMap) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let status = runtime.lock().expect("server runtime mutex").admin_status();
    (StatusCode::OK, Json(status)).into_response()
}

#[derive(Debug, Serialize)]
struct AdminMemoryFootprintReply {
    schema_version: &'static str,
    embedded_cache: hydracache::MemoryFootprintSnapshot,
    client_surface: Option<hydracache_client_transport_axum::ClientSurfaceRetainedState>,
    resp_active_connections: u64,
    hc2: Option<crate::hc2::Hc2AccountingSnapshot>,
}

async fn admin_memory_footprint(
    State(runtime): State<SharedServerRuntime>,
    headers: HeaderMap,
    hc2: Option<Extension<Hc2ClientPlaneService>>,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let (cache, client_surface, resp_active_connections) = {
        let runtime = runtime.lock().expect("server runtime mutex");
        (
            runtime.cache().clone(),
            runtime.client_dispatch_state(),
            runtime.redis_active_connections(),
        )
    };
    match cache
        .memory_footprint_snapshot(hydracache::MemorySnapshotRequest::Admin)
        .await
    {
        Ok(mut embedded_cache) => {
            let client_surface = client_surface.map(|state| state.retained_state_for_diagnostics());
            let hc2 = hc2.map(|Extension(service)| service.accounting());
            if let Some(client) = client_surface {
                embedded_cache.external.client_surface_entries = Some(client.store_entries as u64);
                embedded_cache.external.client_surface_bytes = Some(
                    client
                        .value_bytes
                        .saturating_add(client.store_identity_bytes) as u64,
                );
                embedded_cache.external.idempotency_records =
                    Some(client.idempotency_outcomes as u64);
                embedded_cache.external.idempotency_bytes =
                    Some(client.idempotency_identity_bytes as u64);
                embedded_cache.external.audit_records = Some(client.audit_events as u64);
                embedded_cache.external.sessions =
                    Some(client.conditional.session_heartbeats as u64);
            }
            embedded_cache.external.connections = Some(
                resp_active_connections
                    .saturating_add(hc2.map_or(0, |value| value.active_connections)),
            );
            embedded_cache.external.subscriptions = hc2.map(|value| value.active_subscriptions);
            embedded_cache.external.unavailable_reason = Some(
                "age, audit byte/failure, outbound byte, durable, and file-backed counters unavailable"
                    .to_owned(),
            );
            (
                StatusCode::OK,
                Json(AdminMemoryFootprintReply {
                    schema_version: "hydracache-admin-memory-footprint-v1",
                    embedded_cache,
                    client_surface,
                    resp_active_connections,
                    hc2,
                }),
            )
                .into_response()
        }
        Err(error) => AdminHttpError::MemorySnapshotFailed(error.to_string()).into_response(),
    }
}

async fn admin_drain(State(runtime): State<SharedServerRuntime>, headers: HeaderMap) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let drain = runtime
        .lock()
        .expect("server runtime mutex")
        .request_admin_drain();
    (
        StatusCode::OK,
        Json(AdminDrainReply {
            action: "drain",
            outcome: "accepted",
            drain,
        }),
    )
        .into_response()
}

async fn admin_reshard(State(runtime): State<SharedServerRuntime>, headers: HeaderMap) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    runtime
        .lock()
        .expect("server runtime mutex")
        .request_reshard()
        .map(|action| (StatusCode::OK, Json(action)).into_response())
        .unwrap_or_else(|error| AdminHttpError::from(error).into_response())
}

async fn admin_backup(State(runtime): State<SharedServerRuntime>, headers: HeaderMap) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    runtime
        .lock()
        .expect("server runtime mutex")
        .request_backup()
        .map(|action| {
            (
                StatusCode::ACCEPTED,
                Json(AdminBackupRequestAcceptance {
                    action: action.action,
                    outcome: action.outcome,
                    detail: action.detail,
                    authority: "request_only",
                    durable_artifact_created: false,
                    restore_point_available: false,
                }),
            )
                .into_response()
        })
        .unwrap_or_else(|error| AdminHttpError::from(error).into_response())
}

async fn admin_diagnostic_reset(
    State(runtime): State<SharedServerRuntime>,
    hc2: Option<Extension<Hc2ClientPlaneService>>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let (cache, client_state) = {
        let runtime = runtime.lock().expect("server runtime mutex");
        if !runtime.diagnostic_reset_enabled() {
            return AdminHttpError::DiagnosticResetDisabled.into_response();
        }
        runtime.diagnostic_reset_targets()
    };
    if client_state
        .as_ref()
        .is_some_and(|state| state.active_subscriptions() != 0)
        || hc2.as_ref().is_some_and(|Extension(service)| {
            let accounting = service.accounting();
            accounting.active_connections != 0
                || accounting.pending_invocations != 0
                || accounting.active_subscriptions != 0
                || accounting.active_sessions != 0
        })
    {
        return AdminHttpError::DiagnosticResetBusy.into_response();
    }

    let embedded_before = cache.diagnostics().await.estimated_entries;
    if let Err(error) = cache.flush().await {
        return AdminHttpError::DiagnosticResetFailed(error.to_string()).into_response();
    }
    let client = match client_state
        .map(|state| state.reset_retained_state_for_diagnostics())
        .transpose()
    {
        Ok(reset) => reset,
        Err(error) => {
            return AdminHttpError::DiagnosticResetFailed(error.to_string()).into_response();
        }
    };
    let embedded_after = cache.diagnostics().await.estimated_entries;
    let client_is_zero = client.as_ref().is_none_or(|reset| {
        reset.after.store_entries == 0
            && reset.after.idempotency_outcomes == 0
            && reset.after.conditional.records == 0
            && reset.after.conditional.locks == 0
            && reset.after.conditional.session_heartbeats == 0
    });
    if embedded_after != 0 || !client_is_zero {
        return AdminHttpError::DiagnosticResetFailed(
            "owner counts remained non-zero after reset".to_owned(),
        )
        .into_response();
    }

    (
        StatusCode::OK,
        Json(AdminDiagnosticResetReply {
            action: "diagnostic_reset",
            outcome: "completed",
            embedded_before,
            embedded_after,
            client,
        }),
    )
        .into_response()
}

async fn admin_raft_compaction_status(
    State(runtime): State<SharedServerRuntime>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    runtime
        .lock()
        .expect("server runtime mutex")
        .raft_compaction_status()
        .map(|status| (StatusCode::OK, Json(status)).into_response())
        .unwrap_or_else(|error| {
            AdminHttpError::from(ServerAdminActionError::from(error)).into_response()
        })
}

async fn admin_raft_compaction(
    State(runtime): State<SharedServerRuntime>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    runtime
        .lock()
        .expect("server runtime mutex")
        .request_raft_compaction()
        .map(|status| (StatusCode::OK, Json(status)).into_response())
        .unwrap_or_else(|error| AdminHttpError::from(error).into_response())
}

/// Honest response boundary for the currently request-only backup admin seam.
///
/// A successful HTTP response confirms only that configuration and runtime
/// preconditions accepted the request. The daemon does not yet own a live
/// value-plane backup source, a durable object-store writer, or restore-point
/// authority, so neither boolean may be inferred from `outcome = "accepted"`.
#[derive(Debug, Serialize)]
struct AdminBackupRequestAcceptance {
    action: &'static str,
    outcome: &'static str,
    detail: String,
    authority: &'static str,
    durable_artifact_created: bool,
    restore_point_available: bool,
}

pub(crate) fn require_admin(headers: &HeaderMap) -> Result<(), AdminHttpError> {
    let has_identity = header_value(headers, HYDRACACHE_CLIENT_ID_HEADER).is_some()
        && header_value(headers, HYDRACACHE_TENANT_HEADER).is_some();
    if !has_identity {
        return Err(AdminHttpError::Unauthenticated);
    }
    let admin = headers
        .get(HYDRACACHE_ADMIN_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| matches!(value, "true" | "1"));
    if !admin {
        return Err(AdminHttpError::Unauthorized);
    }
    Ok(())
}

pub(crate) fn require_management_read(headers: &HeaderMap) -> Result<(), AdminHttpError> {
    let has_identity = header_value(headers, HYDRACACHE_CLIENT_ID_HEADER).is_some()
        && header_value(headers, HYDRACACHE_TENANT_HEADER).is_some();
    if !has_identity {
        return Err(AdminHttpError::Unauthenticated);
    }
    let reader = headers
        .get(HYDRACACHE_MANAGEMENT_READ_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| matches!(value, "true" | "1"));
    let write_admin = headers
        .get(HYDRACACHE_ADMIN_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| matches!(value, "true" | "1"));
    if !reader && !write_admin {
        return Err(AdminHttpError::Unauthorized);
    }
    Ok(())
}

fn header_value(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

/// Admin drain response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdminDrainReply {
    /// Stable action name.
    pub action: &'static str,
    /// Stable outcome string.
    pub outcome: &'static str,
    /// Drain result from the runtime.
    pub drain: DrainOutcome,
}

/// Verified owner counts returned by the local diagnostic reset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdminDiagnosticResetReply {
    /// Stable destructive action name.
    pub action: &'static str,
    /// Stable successful outcome.
    pub outcome: &'static str,
    /// Embedded cache entries observed before cleanup.
    pub embedded_before: u64,
    /// Embedded cache entries observed after cleanup.
    pub embedded_after: u64,
    /// Shared HC/1, HC/2 and RESP dispatch owner counts, when configured.
    pub client: Option<ClientSurfaceDiagnosticReset>,
}

/// JSON reply for rejected admin calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdminErrorReply {
    /// Stable outcome string.
    pub outcome: &'static str,
    /// Redacted detail safe for operator Conditions.
    pub detail: String,
}

impl AdminErrorReply {
    fn rejected(detail: impl Into<String>) -> Self {
        Self {
            outcome: "rejected",
            detail: detail.into(),
        }
    }
}

/// Admin HTTP boundary errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdminHttpError {
    /// Admin identity was absent or incomplete.
    #[error("admin identity is required")]
    Unauthenticated,
    /// Caller identity is not privileged for admin actions.
    #[error("admin privileges are required")]
    Unauthorized,
    /// Diagnostic reset is not explicitly enabled.
    #[error("diagnostic reset is disabled")]
    DiagnosticResetDisabled,
    /// Active client resources make destructive reset unsafe.
    #[error("diagnostic reset requires a quiescent client surface")]
    DiagnosticResetBusy,
    /// A reset owner failed cleanup or its zero assertion.
    #[error("diagnostic reset failed: {0}")]
    DiagnosticResetFailed(String),
    /// Product counters rejected a corrupt or incoherent snapshot.
    #[error("memory snapshot failed: {0}")]
    MemorySnapshotFailed(String),
    /// Runtime refused the requested admin action.
    #[error("{0}")]
    Action(#[from] ServerAdminActionError),
}

impl IntoResponse for AdminHttpError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::Unauthorized => StatusCode::FORBIDDEN,
            Self::DiagnosticResetDisabled => StatusCode::NOT_FOUND,
            Self::DiagnosticResetBusy => StatusCode::CONFLICT,
            Self::DiagnosticResetFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::MemorySnapshotFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Action(ServerAdminActionError::NotReady(_)) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Action(
                ServerAdminActionError::RequiresMember(_) | ServerAdminActionError::BackupDisabled,
            ) => StatusCode::CONFLICT,
            Self::Action(ServerAdminActionError::RaftCompaction(
                RaftCompactionError::Disabled | RaftCompactionError::Unavailable,
            )) => StatusCode::CONFLICT,
            Self::Action(ServerAdminActionError::RaftCompaction(RaftCompactionError::Runtime(
                _,
            ))) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(AdminErrorReply::rejected(self.to_string()))).into_response()
    }
}
