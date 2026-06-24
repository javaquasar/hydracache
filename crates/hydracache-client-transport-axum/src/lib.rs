//! Axum route boundary for HydraCache external client traffic.
//!
//! This crate owns the public `/client/v1/*` surface. It is intentionally
//! separate from member-to-member cluster transport so public compatibility
//! cannot accidentally inherit private cluster route semantics.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hydracache_client_protocol::ClientFrame;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable external client API prefix.
pub const CLIENT_API_PREFIX: &str = "/client/v1";

/// Minimal data route reserved for W1 protocol dispatch.
pub const CLIENT_DATA_PATH: &str = "/client/v1/data";

/// Client status route reserved for W6.
pub const CLIENT_STATUS_PATH: &str = "/client/v1/status";

/// Subscription route reserved for W1 invalidation streams.
pub const CLIENT_SUBSCRIPTIONS_PATH: &str = "/client/v1/subscriptions";

/// Header carrying a verified external consumer id.
pub const HYDRACACHE_CLIENT_ID_HEADER: &str = "x-hydracache-client-id";

/// Header carrying a verified tenant id.
pub const HYDRACACHE_TENANT_HEADER: &str = "x-hydracache-tenant";

/// External client route boundary helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientRouteBoundary;

impl ClientRouteBoundary {
    /// Return whether a path belongs to the external client route namespace.
    pub fn is_client_route(path: &str) -> bool {
        path == CLIENT_API_PREFIX
            || path
                .strip_prefix(CLIENT_API_PREFIX)
                .is_some_and(|suffix| suffix.starts_with('/'))
    }

    /// Return whether a path belongs to the internal member namespace.
    pub fn is_internal_member_route(path: &str) -> bool {
        path == "/cluster" || path.starts_with("/cluster/")
    }
}

/// Request and stream limits for the external client surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSurfaceLimits {
    /// Maximum encoded frame bytes accepted before protocol dispatch.
    pub max_frame_bytes: usize,
    /// Maximum value bytes accepted by future W1 Put operations.
    pub max_value_bytes: usize,
    /// Maximum batch entries accepted by future W1 batch operations.
    pub max_batch_entries: usize,
    /// Maximum serialized batch bytes.
    pub max_batch_bytes: usize,
    /// Maximum concurrently active subscription streams per connection.
    pub max_streams_per_connection: usize,
    /// Heartbeat interval reserved for SubscribeInvalidations.
    pub heartbeat_interval_ms: u64,
    /// Idle timeout reserved for SubscribeInvalidations.
    pub idle_timeout_ms: u64,
}

impl Default for ClientSurfaceLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 1024 * 1024,
            max_value_bytes: 16 * 1024 * 1024,
            max_batch_entries: 128,
            max_batch_bytes: 8 * 1024 * 1024,
            max_streams_per_connection: 16,
            heartbeat_interval_ms: 10_000,
            idle_timeout_ms: 60_000,
        }
    }
}

impl ClientSurfaceLimits {
    /// Validate that every limit is non-zero and internally coherent.
    pub fn validate(&self) -> Result<(), ClientSurfaceError> {
        if self.max_frame_bytes == 0 {
            return Err(ClientSurfaceError::InvalidLimit("max_frame_bytes"));
        }
        if self.max_value_bytes == 0 {
            return Err(ClientSurfaceError::InvalidLimit("max_value_bytes"));
        }
        if self.max_batch_entries == 0 {
            return Err(ClientSurfaceError::InvalidLimit("max_batch_entries"));
        }
        if self.max_batch_bytes == 0 {
            return Err(ClientSurfaceError::InvalidLimit("max_batch_bytes"));
        }
        if self.max_streams_per_connection == 0 {
            return Err(ClientSurfaceError::InvalidLimit(
                "max_streams_per_connection",
            ));
        }
        if self.heartbeat_interval_ms == 0 {
            return Err(ClientSurfaceError::InvalidLimit("heartbeat_interval_ms"));
        }
        if self.idle_timeout_ms == 0 {
            return Err(ClientSurfaceError::InvalidLimit("idle_timeout_ms"));
        }
        Ok(())
    }

    /// Return the heartbeat interval as a duration.
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval_ms)
    }

    /// Return the idle timeout as a duration.
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_millis(self.idle_timeout_ms)
    }
}

/// Verified identity extracted before protocol payload dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientIdentity {
    client_id: String,
    tenant: String,
}

impl ClientIdentity {
    /// Create a verified identity.
    pub fn new(
        client_id: impl Into<String>,
        tenant: impl Into<String>,
    ) -> Result<Self, ClientSurfaceError> {
        let client_id = client_id.into();
        let tenant = tenant.into();
        if client_id.trim().is_empty() {
            return Err(ClientSurfaceError::Unauthenticated);
        }
        if tenant.trim().is_empty() {
            return Err(ClientSurfaceError::Unauthenticated);
        }
        Ok(Self { client_id, tenant })
    }

    /// Consumer id.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Tenant id bound to the consumer.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    fn from_headers(headers: &HeaderMap) -> Result<Self, ClientSurfaceError> {
        let client_id = header_value(headers, HYDRACACHE_CLIENT_ID_HEADER)?;
        let tenant = header_value(headers, HYDRACACHE_TENANT_HEADER)?;
        Self::new(client_id, tenant)
    }
}

/// Shared state for the public client surface.
#[derive(Debug)]
pub struct ClientSurfaceState {
    limits: ClientSurfaceLimits,
    dispatch_attempts: AtomicU64,
    state_mutations: AtomicU64,
    rejected_anonymous: AtomicU64,
    rejected_oversized: AtomicU64,
    active_subscriptions: AtomicU64,
}

impl ClientSurfaceState {
    /// Create state with validated limits.
    pub fn new(limits: ClientSurfaceLimits) -> Result<Self, ClientSurfaceError> {
        limits.validate()?;
        Ok(Self {
            limits,
            dispatch_attempts: AtomicU64::new(0),
            state_mutations: AtomicU64::new(0),
            rejected_anonymous: AtomicU64::new(0),
            rejected_oversized: AtomicU64::new(0),
            active_subscriptions: AtomicU64::new(0),
        })
    }

    /// Return configured limits.
    pub fn limits(&self) -> ClientSurfaceLimits {
        self.limits
    }

    /// Count of requests that reached protocol dispatch.
    pub fn dispatch_attempts(&self) -> u64 {
        self.dispatch_attempts.load(Ordering::SeqCst)
    }

    /// Count of modeled cache mutations.
    pub fn state_mutations(&self) -> u64 {
        self.state_mutations.load(Ordering::SeqCst)
    }

    /// Count of anonymous requests rejected before dispatch.
    pub fn rejected_anonymous(&self) -> u64 {
        self.rejected_anonymous.load(Ordering::SeqCst)
    }

    /// Count of oversized frames rejected before mutation.
    pub fn rejected_oversized(&self) -> u64 {
        self.rejected_oversized.load(Ordering::SeqCst)
    }

    /// Count of active subscription streams.
    pub fn active_subscriptions(&self) -> u64 {
        self.active_subscriptions.load(Ordering::SeqCst)
    }

    fn reject_anonymous(&self) {
        self.rejected_anonymous.fetch_add(1, Ordering::SeqCst);
    }

    fn reject_oversized(&self) {
        self.rejected_oversized.fetch_add(1, Ordering::SeqCst);
    }

    fn record_dispatch(&self) {
        self.dispatch_attempts.fetch_add(1, Ordering::SeqCst);
    }

    fn begin_subscription(&self) {
        self.active_subscriptions.fetch_add(1, Ordering::SeqCst);
    }

    fn drain_subscriptions(&self) -> u64 {
        self.active_subscriptions.swap(0, Ordering::SeqCst)
    }
}

/// Axum route owner for the external client surface.
#[derive(Debug, Clone)]
pub struct AxumClientSurface {
    state: Arc<ClientSurfaceState>,
}

impl AxumClientSurface {
    /// Create a route owner with validated limits.
    pub fn new(limits: ClientSurfaceLimits) -> Result<Self, ClientSurfaceError> {
        Ok(Self {
            state: Arc::new(ClientSurfaceState::new(limits)?),
        })
    }

    /// Create a route owner from shared state.
    pub fn from_state(state: Arc<ClientSurfaceState>) -> Self {
        Self { state }
    }

    /// Return shared surface state.
    pub fn state(&self) -> Arc<ClientSurfaceState> {
        Arc::clone(&self.state)
    }

    /// Return the axum router for `/client/v1/*`.
    pub fn routes(&self) -> Router {
        Router::new()
            .route("/client/v1/data", post(client_data))
            .route("/client/v1/status", get(client_status))
            .route("/client/v1/subscriptions", post(client_subscription))
            .with_state(Arc::clone(&self.state))
    }
}

async fn client_data(
    State(state): State<Arc<ClientSurfaceState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match validate_before_dispatch(&state, &headers, body.len()) {
        Ok(_identity) => {
            state.record_dispatch();
            match ClientFrame::decode(&body, state.limits().max_frame_bytes) {
                Ok(_frame) => (
                    StatusCode::ACCEPTED,
                    Json(ClientSurfaceReply::accepted("protocol_dispatch_reserved")),
                )
                    .into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(ClientSurfaceReply::rejected(error.to_string())),
                )
                    .into_response(),
            }
        }
        Err(error) => error.into_response(),
    }
}

async fn client_status(
    State(_state): State<Arc<ClientSurfaceState>>,
    headers: HeaderMap,
) -> Response {
    match ClientIdentity::from_headers(&headers) {
        Ok(identity) => (StatusCode::OK, Json(ClientStatusReply::from(identity))).into_response(),
        Err(error) => error.into_response(),
    }
}

async fn client_subscription(
    State(state): State<Arc<ClientSurfaceState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match validate_before_dispatch(&state, &headers, body.len()) {
        Ok(_identity) => {
            if state.active_subscriptions() as usize >= state.limits().max_streams_per_connection {
                return ClientSurfaceError::TooManyStreams.into_response();
            }
            state.begin_subscription();
            state.record_dispatch();
            (
                StatusCode::ACCEPTED,
                Json(ClientSurfaceReply::accepted("subscription_reserved")),
            )
                .into_response()
        }
        Err(error) => error.into_response(),
    }
}

fn validate_before_dispatch(
    state: &ClientSurfaceState,
    headers: &HeaderMap,
    body_len: usize,
) -> Result<ClientIdentity, ClientSurfaceError> {
    let identity = match ClientIdentity::from_headers(headers) {
        Ok(identity) => identity,
        Err(error) => {
            state.reject_anonymous();
            return Err(error);
        }
    };
    if body_len > state.limits().max_frame_bytes {
        state.reject_oversized();
        return Err(ClientSurfaceError::FrameTooLarge {
            actual: body_len,
            max: state.limits().max_frame_bytes,
        });
    }
    Ok(identity)
}

fn header_value(headers: &HeaderMap, name: &'static str) -> Result<String, ClientSurfaceError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or(ClientSurfaceError::Unauthenticated)
}

/// Deterministic lifecycle model for long-lived client subscriptions.
#[derive(Debug, Clone)]
pub struct ClientSurfaceRuntime {
    state: Arc<ClientSurfaceState>,
    accepting: bool,
}

impl ClientSurfaceRuntime {
    /// Create a runtime from limits.
    pub fn new(limits: ClientSurfaceLimits) -> Result<Self, ClientSurfaceError> {
        Ok(Self {
            state: Arc::new(ClientSurfaceState::new(limits)?),
            accepting: false,
        })
    }

    /// Start accepting client work.
    pub fn start(&mut self) {
        self.accepting = true;
    }

    /// Return whether client routes are accepting work.
    pub fn accepting(&self) -> bool {
        self.accepting
    }

    /// Begin a modeled subscription stream.
    pub fn begin_subscription(&self) -> Result<(), ClientSurfaceError> {
        if !self.accepting {
            return Err(ClientSurfaceError::Draining);
        }
        if self.state.active_subscriptions() as usize
            >= self.state.limits().max_streams_per_connection
        {
            return Err(ClientSurfaceError::TooManyStreams);
        }
        self.state.begin_subscription();
        Ok(())
    }

    /// Gracefully stop accepting and drain active streams.
    pub fn shutdown(&mut self) -> ClientSurfaceDrain {
        self.accepting = false;
        let started_with = self.state.drain_subscriptions();
        ClientSurfaceDrain {
            started_with,
            remaining: self.state.active_subscriptions(),
        }
    }

    /// Return shared state.
    pub fn state(&self) -> Arc<ClientSurfaceState> {
        Arc::clone(&self.state)
    }
}

/// Result of draining external client streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientSurfaceDrain {
    /// Active subscriptions observed when drain started.
    pub started_with: u64,
    /// Active subscriptions after drain.
    pub remaining: u64,
}

/// JSON status response for W0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientStatusReply {
    /// Verified consumer id.
    pub client_id: String,
    /// Verified tenant id.
    pub tenant: String,
    /// Route boundary version.
    pub route_version: u16,
}

impl From<ClientIdentity> for ClientStatusReply {
    fn from(identity: ClientIdentity) -> Self {
        Self {
            client_id: identity.client_id,
            tenant: identity.tenant,
            route_version: 1,
        }
    }
}

/// JSON reply used by the W0 route boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientSurfaceReply {
    /// Outcome string.
    pub outcome: &'static str,
    /// Redacted detail.
    pub detail: String,
}

impl ClientSurfaceReply {
    fn accepted(detail: impl Into<String>) -> Self {
        Self {
            outcome: "accepted",
            detail: detail.into(),
        }
    }

    fn rejected(detail: impl Into<String>) -> Self {
        Self {
            outcome: "rejected",
            detail: detail.into(),
        }
    }
}

/// Client surface failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClientSurfaceError {
    /// Client identity was absent or incomplete.
    #[error("client identity is required before dispatch")]
    Unauthenticated,
    /// Frame exceeds configured limits.
    #[error("client frame is {actual} bytes, exceeding max_frame_bytes={max}")]
    FrameTooLarge {
        /// Observed frame length.
        actual: usize,
        /// Configured limit.
        max: usize,
    },
    /// Too many subscription streams are active.
    #[error("too many client subscription streams")]
    TooManyStreams,
    /// Surface is draining.
    #[error("client surface is draining")]
    Draining,
    /// Invalid zero limit.
    #[error("client surface limit {0} must be greater than zero")]
    InvalidLimit(&'static str),
}

impl IntoResponse for ClientSurfaceError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::FrameTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            Self::TooManyStreams => StatusCode::TOO_MANY_REQUESTS,
            Self::Draining => StatusCode::SERVICE_UNAVAILABLE,
            Self::InvalidLimit(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(ClientSurfaceReply::rejected(self.to_string()))).into_response()
    }
}
