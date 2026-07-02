use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use serde::Serialize;
use thiserror::Error;

use crate::bootstrap::{ServerAdminActionError, ServerRuntime};
use crate::services::DrainOutcome;
use hydracache_observability::PrometheusExporter;

/// Liveness path used by Kubernetes probes.
pub const ADMIN_HEALTHZ_PATH: &str = "/healthz";
/// Readiness path used by Kubernetes probes.
pub const ADMIN_READYZ_PATH: &str = "/readyz";
/// Prometheus metrics path on the internal admin surface.
pub const ADMIN_METRICS_PATH: &str = "/metrics";
/// Read-only cluster overview path on the internal admin surface.
pub const ADMIN_CLUSTER_OVERVIEW_PATH: &str = "/cluster/overview";
/// Operator status path.
pub const ADMIN_STATUS_PATH: &str = "/admin/status";
/// Operator drain action path.
pub const ADMIN_DRAIN_PATH: &str = "/admin/drain";
/// Operator reshard action path.
pub const ADMIN_RESHARD_PATH: &str = "/admin/reshard";
/// Operator backup action path.
pub const ADMIN_BACKUP_PATH: &str = "/admin/backup";

/// Shared runtime state for the admin HTTP surface.
pub type SharedServerRuntime = Arc<Mutex<ServerRuntime>>;

/// Axum route owner for the internal admin/operator surface.
#[derive(Debug, Clone)]
pub struct AdminHttpSurface {
    runtime: SharedServerRuntime,
}

impl AdminHttpSurface {
    /// Create an admin surface from a server runtime.
    pub fn new(runtime: ServerRuntime) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
        }
    }

    /// Create an admin surface from shared runtime state.
    pub fn from_shared(runtime: SharedServerRuntime) -> Self {
        Self { runtime }
    }

    /// Return shared runtime state for tests and embedding code.
    pub fn runtime(&self) -> SharedServerRuntime {
        Arc::clone(&self.runtime)
    }

    /// Return the axum router for `/healthz`, `/readyz`, and `/admin/*`.
    pub fn routes(&self) -> Router {
        Router::new()
            .route(ADMIN_HEALTHZ_PATH, get(healthz))
            .route(ADMIN_READYZ_PATH, get(readyz))
            .route(ADMIN_METRICS_PATH, get(metrics))
            .route(ADMIN_CLUSTER_OVERVIEW_PATH, get(cluster_overview))
            .route(ADMIN_STATUS_PATH, get(admin_status))
            .route(ADMIN_DRAIN_PATH, post(admin_drain))
            .route(ADMIN_RESHARD_PATH, post(admin_reshard))
            .route(ADMIN_BACKUP_PATH, post(admin_backup))
            .with_state(Arc::clone(&self.runtime))
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

async fn metrics(State(runtime): State<SharedServerRuntime>) -> Response {
    let registry = runtime
        .lock()
        .expect("server runtime mutex")
        .metrics_registry();
    let text = PrometheusExporter::new(registry).render().await;
    ([(CONTENT_TYPE, "text/plain; version=0.0.4")], text).into_response()
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

async fn admin_drain(State(runtime): State<SharedServerRuntime>, headers: HeaderMap) -> Response {
    if let Err(error) = require_admin(&headers) {
        return error.into_response();
    }
    let drain = runtime
        .lock()
        .expect("server runtime mutex")
        .graceful_shutdown();
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
        .map(|action| (StatusCode::OK, Json(action)).into_response())
        .unwrap_or_else(|error| AdminHttpError::from(error).into_response())
}

fn require_admin(headers: &HeaderMap) -> Result<(), AdminHttpError> {
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
    /// Runtime refused the requested admin action.
    #[error("{0}")]
    Action(#[from] ServerAdminActionError),
}

impl IntoResponse for AdminHttpError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::Unauthorized => StatusCode::FORBIDDEN,
            Self::Action(ServerAdminActionError::NotReady(_)) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Action(
                ServerAdminActionError::RequiresMember(_) | ServerAdminActionError::BackupDisabled,
            ) => StatusCode::CONFLICT,
        };
        (status, Json(AdminErrorReply::rejected(self.to_string()))).into_response()
    }
}
