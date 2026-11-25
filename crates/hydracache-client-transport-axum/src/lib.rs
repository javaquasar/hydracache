//! Axum route boundary for HydraCache external client traffic.
//!
//! This crate owns the public `/client/v1/*` surface. It is intentionally
//! separate from member-to-member cluster transport so public compatibility
//! cannot accidentally inherit private cluster route semantics.

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hydracache::{
    AdmissionRejection, CasResult, ClusterEpoch, ConditionalError, ConditionalRetainedState,
    ConsistencyLevel, ConsumerIsolation, FenceToken, LockOwner, LogicalDuration, LogicalTime,
    SingleKeyConditionalStore, TenantId, TenantMetricsSnapshot,
};
use hydracache_client_protocol::{
    protocol_version_supported, BatchItemStatus, CasExpectation, ClientErrorCode,
    ClientErrorEnvelope, ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, ClientWireMessage, CompareValueExpireMode, ConditionalPutCondition,
    LockConsistency, Namespace, StructuredKey, TtlState, VersionHandshake, PROTOCOL_VERSION,
};
use hydracache_observability::{AuditEvent, AuditRecorder, InMemoryAuditSink, TenantStatus};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;

mod backend;
mod edge_state;
mod local_backend;
pub use backend::{
    ClientBackendFuture, ClientBackendOutcome, ClientDataBackend, ClientSurfaceDispatcher,
    LocalDispatchVariant, VerifiedClientRequest,
};
use edge_state::ClientSurfaceEdgeState;
use local_backend::LocalBackendExecution;
pub use local_backend::LocalClientDataBackend;
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

/// Optional test/admin marker for privileged client operations.
pub const HYDRACACHE_ADMIN_HEADER: &str = "x-hydracache-admin";

/// Bounded capacity of the in-process client-surface mutation signal bus.
pub const CLIENT_SURFACE_MUTATION_EVENT_CAPACITY: usize = 1024;

/// Maximum number of successful idempotent outcomes retained by one surface.
pub const CLIENT_SURFACE_IDEMPOTENCY_CAPACITY: usize = 4_096;

/// Retention window for a successful idempotent outcome.
pub const CLIENT_SURFACE_IDEMPOTENCY_TTL_MS: u64 = 24 * 60 * 60 * 1_000;

/// Maximum interval between opportunistic expired-entry sweeps while traffic is active.
pub const CLIENT_SURFACE_EXPIRY_SWEEP_INTERVAL_MS: u64 = 1_000;

/// Maximum entries examined by one opportunistic request-path expiry sweep.
pub const CLIENT_SURFACE_EXPIRY_SWEEP_SCAN_LIMIT: usize = 256;

/// Mutation kind emitted after a verified client-surface state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientSurfaceMutationKind {
    /// A value was stored or replaced.
    Stored,
    /// A value was explicitly removed after a successful conditional check.
    Removed,
    /// A key was invalidated without a stronger transition reason.
    Invalidated,
    /// A key expiry was installed or changed.
    Expire,
    /// A key expiry was removed.
    Persist,
    /// A whole namespace was evicted.
    NamespaceEvicted,
}

/// Metadata-only mutation signal shared by HC/1, HC/2, and compatibility edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSurfaceMutationEvent {
    tenant: String,
    namespace: Namespace,
    key: Option<StructuredKey>,
    kind: ClientSurfaceMutationKind,
    message_id: u64,
}

impl ClientSurfaceMutationEvent {
    /// Return the verified tenant that owns the mutation.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    /// Return the affected namespace.
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Return the affected key for key-scoped mutations.
    pub fn key(&self) -> Option<&StructuredKey> {
        self.key.as_ref()
    }

    /// Return the mutation kind.
    pub fn kind(&self) -> ClientSurfaceMutationKind {
        self.kind
    }

    /// Return the process-local monotonic message id.
    pub fn message_id(&self) -> u64 {
        self.message_id
    }
}

/// Receive failure for the bounded client-surface mutation bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ClientSurfaceMutationRecvError {
    /// The subscriber fell behind and one or more signals were overwritten.
    #[error("client-surface mutation subscriber lagged by {0} events")]
    Lagged(u64),
    /// The mutation bus was closed.
    #[error("client-surface mutation bus closed")]
    Closed,
}

/// Tenant-fenced receiver for verified client-surface mutation signals.
#[derive(Debug)]
pub struct ClientSurfaceMutationSubscriber {
    tenant: String,
    receiver: broadcast::Receiver<ClientSurfaceMutationEvent>,
}

impl ClientSurfaceMutationSubscriber {
    /// Receive the next mutation owned by this subscriber's verified tenant.
    pub async fn recv(
        &mut self,
    ) -> Result<ClientSurfaceMutationEvent, ClientSurfaceMutationRecvError> {
        loop {
            match self.receiver.recv().await {
                Ok(event) if event.tenant == self.tenant => return Ok(event),
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    return Err(ClientSurfaceMutationRecvError::Lagged(count));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(ClientSurfaceMutationRecvError::Closed);
                }
            }
        }
    }
}

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
    /// Maximum encoded HC/1 envelope bytes accepted before protocol dispatch.
    ///
    /// RESP and HC/2 apply their own transport-frame limits before using the
    /// shared decoded operation limits below.
    pub max_frame_bytes: usize,
    /// Maximum decoded value bytes accepted by HC/1, HC/2, and RESP operations.
    pub max_value_bytes: usize,
    /// Maximum batch entries accepted by future W1 batch operations.
    pub max_batch_entries: usize,
    /// Maximum decoded aggregate batch bytes across client protocols.
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
    admin: bool,
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
        Ok(Self {
            client_id,
            tenant,
            admin: false,
        })
    }

    /// Consumer id.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Tenant id bound to the consumer.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    /// Return whether this identity can call privileged admin operations.
    pub fn is_admin(&self) -> bool {
        self.admin
    }

    /// Parse the verified identity headers for an internal adapter that applies its own authz.
    pub fn from_headers(headers: &HeaderMap) -> Result<Self, ClientSurfaceError> {
        let client_id = header_value(headers, HYDRACACHE_CLIENT_ID_HEADER)?;
        let tenant = header_value(headers, HYDRACACHE_TENANT_HEADER)?;
        let mut identity = Self::new(client_id, tenant)?;
        identity.admin = headers
            .get(HYDRACACHE_ADMIN_HEADER)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| matches!(value, "true" | "1"));
        Ok(identity)
    }
}

#[derive(Debug)]
struct ClientLockService {
    store: SingleKeyConditionalStore,
    local_node_id: u64,
    leader_node_id: u64,
    logical_now_ms: u64,
    applied_commands: u64,
}

struct CompareAndSetCommand {
    ns: Namespace,
    key: StructuredKey,
    expected: CasExpectation,
    new_value: Vec<u8>,
    level: LockConsistency,
}

struct PutCommand {
    request_id: String,
    idempotency_key: Option<String>,
    ns: Namespace,
    key: StructuredKey,
    value: Vec<u8>,
    ttl_ms: Option<u64>,
}

impl ClientLockService {
    fn new() -> Self {
        Self {
            store: SingleKeyConditionalStore::new(ClusterEpoch::new(1), 64),
            local_node_id: 1,
            leader_node_id: 1,
            logical_now_ms: 0,
            applied_commands: 0,
        }
    }

    fn set_leader_for_tests(&mut self, local_node_id: u64, leader_node_id: u64) {
        self.local_node_id = local_node_id.max(1);
        self.leader_node_id = leader_node_id.max(1);
    }

    fn advance_time_for_tests(&mut self, millis: u64) {
        self.logical_now_ms = self.logical_now_ms.saturating_add(millis);
        self.store
            .expire_due(LogicalTime::from_millis(self.logical_now_ms));
    }

    fn is_leader(&self) -> bool {
        self.local_node_id == self.leader_node_id
    }

    fn apply_lock_command<F>(&mut self, apply: F) -> Result<ClientResponse, ClientErrorEnvelope>
    where
        F: FnOnce(
            &mut SingleKeyConditionalStore,
            LogicalTime,
        ) -> Result<ClientResponse, ConditionalError>,
    {
        if !self.is_leader() {
            return Err(ClientErrorEnvelope::new(
                ClientErrorCode::BackendUnavailable,
                true,
                format!(
                    "not leader for lock partition; leader={}",
                    self.leader_node_id
                ),
            )
            .with_retry_after_ms(1));
        }
        self.applied_commands = self.applied_commands.saturating_add(1);
        self.logical_now_ms = self.logical_now_ms.saturating_add(1);
        let now = LogicalTime::from_millis(self.logical_now_ms);
        self.store.expire_due(now);
        apply(&mut self.store, now).map_err(lock_error_envelope)
    }

    fn current_ownership(&mut self, lock_key: &str) -> ClientResponse {
        self.store
            .expire_due(LogicalTime::from_millis(self.logical_now_ms));
        let fence = self.store.current_fence(lock_key).map(FenceToken::value);
        ClientResponse::LockOwnership {
            fence,
            locked: fence.is_some(),
        }
    }
}

type StoreKey = (String, String, String);
type IdempotencyKey = (String, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IdempotencyRecord {
    expires_at_ms: u64,
}

/// Aggregate, secret-free retained-state snapshot for diagnostic memory runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSurfaceRetainedState {
    /// Live cache entries retained by the external dispatch surface.
    pub store_entries: usize,
    /// Value bytes retained by live cache entries.
    pub value_bytes: usize,
    /// Conservative tenant/namespace/key bytes retained by store keys.
    pub store_identity_bytes: usize,
    /// Successful idempotency outcomes retained for replay.
    pub idempotency_outcomes: usize,
    /// Conservative tenant/idempotency identity bytes.
    pub idempotency_identity_bytes: usize,
    /// Audit records retained by the bounded in-memory adapter.
    pub audit_events: usize,
    /// Conditional value, tombstone, lock and session owners.
    pub conditional: ConditionalRetainedState,
}

/// Before/after owner counts from an explicit diagnostic reset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSurfaceDiagnosticReset {
    /// Owner counts immediately before cleanup.
    pub before: ClientSurfaceRetainedState,
    /// Owner counts after synchronous cleanup.
    pub after: ClientSurfaceRetainedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredValue {
    value: Vec<u8>,
    expires_at_ms: Option<u64>,
}

/// Label-free counters for W1 direct client-surface attribution.
///
/// Collection is disabled by default and intended for isolated profiling
/// processes. The counters never retain tenant, namespace, key, request, or
/// value data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClientSurfaceProfileMetrics {
    /// Verified requests that reached canonical dispatch.
    pub dispatches: u64,
    /// Tenant/identity validation attempts at the alternate transport seam.
    pub identity_validations: u64,
    /// Aggregate time spent validating tenant/identity bindings.
    pub identity_validation_nanoseconds: u64,
    /// Protocol and operation-version validation attempts.
    pub protocol_validations: u64,
    /// Aggregate time spent validating protocol and operation versions.
    pub protocol_validation_nanoseconds: u64,
    /// Request-id and namespace bytes cloned for conditional rejection audit.
    pub audit_context_bytes_cloned: u64,
    /// PUT payload bytes moved into client-surface storage without cloning.
    pub put_value_bytes_moved: u64,
    /// GET payload bytes cloned from client-surface storage into responses.
    pub get_value_bytes_cloned: u64,
    /// Reads of the request/expiry clock.
    pub clock_reads: u64,
    /// Aggregate time spent obtaining and monotonically fencing the request clock.
    pub clock_nanoseconds: u64,
    /// Ordinary or forced expiry sweep checks.
    pub expiry_sweep_checks: u64,
    /// Sweep checks that acquired the expiry cursor and scanned the store.
    pub expiry_sweeps_claimed: u64,
    /// Store entries examined by claimed expiry sweeps.
    pub expiry_sweep_entries_examined: u64,
    /// Expired store entries removed by claimed expiry sweeps.
    pub expiry_sweep_entries_removed: u64,
    /// Canonical `(tenant, namespace, key)` identities materialized for store access.
    pub structured_key_materializations: u64,
    /// Identity bytes copied into canonical store keys.
    pub structured_key_bytes_copied: u64,
    /// Native multi-key requests reaching canonical dispatch.
    pub batch_operations: u64,
    /// Native multi-key items presented to canonical dispatch.
    pub batch_items: u64,
    /// Sequentially consistent dispatch-counter operations on the request path.
    pub dispatch_counter_seqcst_operations: u64,
    /// Sequentially consistent monotonic-clock fence operations on the request path.
    pub clock_floor_seqcst_operations: u64,
    /// Sequentially consistent expiry-gate operations on the request path.
    pub expiry_gate_seqcst_operations: u64,
    /// Sequentially consistent mutation-counter operations on the request path.
    pub mutation_counter_seqcst_operations: u64,
    /// Sequentially consistent event message-id operations on the request path.
    pub message_id_seqcst_operations: u64,
    /// Store mutex acquisitions.
    pub store_lock_acquisitions: u64,
    /// Aggregate time waiting for the store mutex.
    pub store_lock_wait_nanoseconds: u64,
    /// Aggregate time holding the store mutex.
    pub store_lock_hold_nanoseconds: u64,
    /// Live values returned by single-key GET.
    pub live_get_hits: u64,
    /// Expired values removed by the single-key GET lookup itself.
    pub expired_get_removals: u64,
    /// Single-key GET misses that were not removed by that lookup.
    pub missing_gets: u64,
}

#[derive(Debug, Default)]
struct ClientSurfaceProfileInstrumentation {
    enabled: AtomicBool,
    dispatches: AtomicU64,
    identity_validations: AtomicU64,
    identity_validation_nanoseconds: AtomicU64,
    protocol_validations: AtomicU64,
    protocol_validation_nanoseconds: AtomicU64,
    audit_context_bytes_cloned: AtomicU64,
    put_value_bytes_moved: AtomicU64,
    get_value_bytes_cloned: AtomicU64,
    clock_reads: AtomicU64,
    clock_nanoseconds: AtomicU64,
    expiry_sweep_checks: AtomicU64,
    expiry_sweeps_claimed: AtomicU64,
    expiry_sweep_entries_examined: AtomicU64,
    expiry_sweep_entries_removed: AtomicU64,
    structured_key_materializations: AtomicU64,
    structured_key_bytes_copied: AtomicU64,
    batch_operations: AtomicU64,
    batch_items: AtomicU64,
    dispatch_counter_seqcst_operations: AtomicU64,
    clock_floor_seqcst_operations: AtomicU64,
    expiry_gate_seqcst_operations: AtomicU64,
    mutation_counter_seqcst_operations: AtomicU64,
    message_id_seqcst_operations: AtomicU64,
    store_lock_acquisitions: AtomicU64,
    store_lock_wait_nanoseconds: AtomicU64,
    store_lock_hold_nanoseconds: AtomicU64,
    live_get_hits: AtomicU64,
    expired_get_removals: AtomicU64,
    missing_gets: AtomicU64,
}

impl ClientSurfaceProfileInstrumentation {
    fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    fn reset(&self) {
        for counter in [
            &self.dispatches,
            &self.identity_validations,
            &self.identity_validation_nanoseconds,
            &self.protocol_validations,
            &self.protocol_validation_nanoseconds,
            &self.audit_context_bytes_cloned,
            &self.put_value_bytes_moved,
            &self.get_value_bytes_cloned,
            &self.clock_reads,
            &self.clock_nanoseconds,
            &self.expiry_sweep_checks,
            &self.expiry_sweeps_claimed,
            &self.expiry_sweep_entries_examined,
            &self.expiry_sweep_entries_removed,
            &self.structured_key_materializations,
            &self.structured_key_bytes_copied,
            &self.batch_operations,
            &self.batch_items,
            &self.dispatch_counter_seqcst_operations,
            &self.clock_floor_seqcst_operations,
            &self.expiry_gate_seqcst_operations,
            &self.mutation_counter_seqcst_operations,
            &self.message_id_seqcst_operations,
            &self.store_lock_acquisitions,
            &self.store_lock_wait_nanoseconds,
            &self.store_lock_hold_nanoseconds,
            &self.live_get_hits,
            &self.expired_get_removals,
            &self.missing_gets,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
    }

    fn snapshot(&self) -> ClientSurfaceProfileMetrics {
        ClientSurfaceProfileMetrics {
            dispatches: self.dispatches.load(Ordering::Relaxed),
            identity_validations: self.identity_validations.load(Ordering::Relaxed),
            identity_validation_nanoseconds: self
                .identity_validation_nanoseconds
                .load(Ordering::Relaxed),
            protocol_validations: self.protocol_validations.load(Ordering::Relaxed),
            protocol_validation_nanoseconds: self
                .protocol_validation_nanoseconds
                .load(Ordering::Relaxed),
            audit_context_bytes_cloned: self.audit_context_bytes_cloned.load(Ordering::Relaxed),
            put_value_bytes_moved: self.put_value_bytes_moved.load(Ordering::Relaxed),
            get_value_bytes_cloned: self.get_value_bytes_cloned.load(Ordering::Relaxed),
            clock_reads: self.clock_reads.load(Ordering::Relaxed),
            clock_nanoseconds: self.clock_nanoseconds.load(Ordering::Relaxed),
            expiry_sweep_checks: self.expiry_sweep_checks.load(Ordering::Relaxed),
            expiry_sweeps_claimed: self.expiry_sweeps_claimed.load(Ordering::Relaxed),
            expiry_sweep_entries_examined: self
                .expiry_sweep_entries_examined
                .load(Ordering::Relaxed),
            expiry_sweep_entries_removed: self.expiry_sweep_entries_removed.load(Ordering::Relaxed),
            structured_key_materializations: self
                .structured_key_materializations
                .load(Ordering::Relaxed),
            structured_key_bytes_copied: self.structured_key_bytes_copied.load(Ordering::Relaxed),
            batch_operations: self.batch_operations.load(Ordering::Relaxed),
            batch_items: self.batch_items.load(Ordering::Relaxed),
            dispatch_counter_seqcst_operations: self
                .dispatch_counter_seqcst_operations
                .load(Ordering::Relaxed),
            clock_floor_seqcst_operations: self
                .clock_floor_seqcst_operations
                .load(Ordering::Relaxed),
            expiry_gate_seqcst_operations: self
                .expiry_gate_seqcst_operations
                .load(Ordering::Relaxed),
            mutation_counter_seqcst_operations: self
                .mutation_counter_seqcst_operations
                .load(Ordering::Relaxed),
            message_id_seqcst_operations: self.message_id_seqcst_operations.load(Ordering::Relaxed),
            store_lock_acquisitions: self.store_lock_acquisitions.load(Ordering::Relaxed),
            store_lock_wait_nanoseconds: self.store_lock_wait_nanoseconds.load(Ordering::Relaxed),
            store_lock_hold_nanoseconds: self.store_lock_hold_nanoseconds.load(Ordering::Relaxed),
            live_get_hits: self.live_get_hits.load(Ordering::Relaxed),
            expired_get_removals: self.expired_get_removals.load(Ordering::Relaxed),
            missing_gets: self.missing_gets.load(Ordering::Relaxed),
        }
    }
}

struct ProfiledStoreGuard<'a> {
    guard: MutexGuard<'a, BTreeMap<StoreKey, StoredValue>>,
    hold_started: Option<Instant>,
    instrumentation: &'a ClientSurfaceProfileInstrumentation,
}

impl Deref for ProfiledStoreGuard<'_> {
    type Target = BTreeMap<StoreKey, StoredValue>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl DerefMut for ProfiledStoreGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl Drop for ProfiledStoreGuard<'_> {
    fn drop(&mut self) {
        if let Some(started) = self.hold_started {
            self.instrumentation
                .store_lock_hold_nanoseconds
                .fetch_add(elapsed_nanoseconds(started), Ordering::Relaxed);
        }
    }
}

struct ConditionalPutArgs {
    request_id: String,
    ns: Namespace,
    key: StructuredKey,
    value: Vec<u8>,
    ttl_ms: Option<u64>,
    condition: ConditionalPutCondition,
}

struct CompareValueExpireArgs {
    request_id: String,
    ns: Namespace,
    key: StructuredKey,
    expected_value: Vec<u8>,
    ttl_ms: u64,
    mode: CompareValueExpireMode,
}

impl StoredValue {
    fn persistent(value: Vec<u8>) -> Self {
        Self {
            value,
            expires_at_ms: None,
        }
    }

    fn with_ttl(value: Vec<u8>, now_ms: u64, ttl_ms: u64) -> Self {
        Self {
            value,
            expires_at_ms: Some(now_ms.saturating_add(ttl_ms)),
        }
    }

    fn is_expired(&self, now_ms: u64) -> bool {
        self.expires_at_ms
            .is_some_and(|expires_at_ms| expires_at_ms <= now_ms)
    }

    fn ttl_state(&self, now_ms: u64) -> TtlState {
        match self.expires_at_ms {
            Some(expires_at_ms) if expires_at_ms <= now_ms => TtlState::Missing,
            Some(expires_at_ms) => TtlState::ExpiresIn {
                ttl_ms: expires_at_ms.saturating_sub(now_ms),
            },
            None => TtlState::Persistent,
        }
    }
}

struct ExpirySweepCandidates {
    #[cfg_attr(not(any(test, feature = "performance-profile")), allow(dead_code))]
    examined: usize,
    expired: Vec<StoreKey>,
    next_cursor: Option<StoreKey>,
}

fn expiry_sweep_candidates(
    store: &BTreeMap<StoreKey, StoredValue>,
    now_ms: u64,
    cursor: Option<&StoreKey>,
    scan_limit: usize,
    capture_cursor: bool,
) -> ExpirySweepCandidates {
    let mut examined = 0;
    let mut expired = Vec::new();
    let mut last_examined = None;
    if let Some(cursor) = cursor {
        for (key, value) in store
            .range((
                std::ops::Bound::Excluded(cursor),
                std::ops::Bound::Unbounded,
            ))
            .take(scan_limit)
        {
            examined += 1;
            if value.is_expired(now_ms) {
                expired.push(key.clone());
            }
            last_examined = Some(key);
        }
        if examined < scan_limit {
            for (key, value) in store
                .range((
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Included(cursor),
                ))
                .take(scan_limit - examined)
            {
                examined += 1;
                if value.is_expired(now_ms) {
                    expired.push(key.clone());
                }
                last_examined = Some(key);
            }
        }
    } else {
        for (key, value) in store.iter().take(scan_limit) {
            examined += 1;
            if value.is_expired(now_ms) {
                expired.push(key.clone());
            }
            last_examined = Some(key);
        }
    }
    ExpirySweepCandidates {
        examined,
        expired,
        next_cursor: if capture_cursor {
            last_examined.cloned()
        } else {
            None
        },
    }
}

/// Profile-only fixtures for attributing expiry-sweep allocation without
/// adding counters or timers to the production request path.
#[cfg(feature = "performance-profile")]
pub mod performance_profile {
    use super::{
        expiry_sweep_candidates, store_key, ClientIdentity, ClientRequest, ClientRequestEnvelope,
        ClientResponse, ClientSurfaceLimits, ClientSurfaceState, Namespace, StoreKey, StoredValue,
        StructuredKey,
    };
    use std::collections::BTreeMap;

    const FIXTURE_ENTRIES: usize = 512;
    const SCAN_LIMIT: usize = 256;
    const NOW_MS: u64 = 1_000;
    const TENANT_BYTES: usize = 16;
    const NAMESPACE_BYTES: usize = 16;
    const KEY_BYTES: usize = 64;

    /// Frozen expiry shape used by the W2 local profile process.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ExpirySweepProfileScenario {
        /// No examined entry is expired.
        NoneExpired,
        /// Every other examined entry is expired.
        HalfExpired,
        /// Every examined entry is expired.
        AllExpired,
        /// The scan crosses the end of the ordered map without expired entries.
        CursorWrapNoneExpired,
    }

    impl ExpirySweepProfileScenario {
        /// Parse the stable command-line spelling used by the profile tool.
        pub fn parse(value: &str) -> Option<Self> {
            match value {
                "none-expired" => Some(Self::NoneExpired),
                "half-expired" => Some(Self::HalfExpired),
                "all-expired" => Some(Self::AllExpired),
                "cursor-wrap-none-expired" => Some(Self::CursorWrapNoneExpired),
                _ => None,
            }
        }

        /// Return the stable command-line spelling.
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::NoneExpired => "none-expired",
                Self::HalfExpired => "half-expired",
                Self::AllExpired => "all-expired",
                Self::CursorWrapNoneExpired => "cursor-wrap-none-expired",
            }
        }
    }

    /// Logical copy volume observed while executing one frozen W2 scan.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ExpirySweepProfileObservation {
        /// Entries inspected by the bounded scan.
        pub examined_keys: usize,
        /// Inspected entries copied into the expired work list.
        pub expired_keys: usize,
        /// Tuple-key clones, including the next cursor clone.
        pub cloned_keys: usize,
        /// String payload bytes copied by all tuple-key clones.
        pub cloned_identity_bytes: usize,
        /// Whether the scan retained a cursor for the next bounded pass.
        pub next_cursor_present: bool,
    }

    /// Prebuilt store and cursor. Fixture construction is intentionally outside
    /// the allocation measurement window.
    pub struct ExpirySweepProfileFixture {
        scenario: ExpirySweepProfileScenario,
        store: BTreeMap<StoreKey, StoredValue>,
        cursor: Option<StoreKey>,
    }

    /// Pointer and lifetime observations for the direct client-surface value path.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ClientValueOwnershipObservation {
        /// Payload bytes moved through the fixture.
        pub payload_bytes: usize,
        /// PUT moved the request allocation into the store without copying it.
        pub request_to_store_pointer_preserved: bool,
        /// GET returned the same allocation retained by the store.
        pub store_to_response_pointer_shared: bool,
        /// A returned response remained valid after the stored value was replaced.
        pub response_survived_store_replacement: bool,
    }

    /// Exercise the real direct client-surface PUT/GET path and expose only pointer identity.
    ///
    /// This hook is compiled only for the local performance profiler. It does not expose stored
    /// values, tenant identities, or mutable backing storage.
    pub fn observe_client_value_ownership(
        payload_bytes: usize,
    ) -> Result<ClientValueOwnershipObservation, String> {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default())
            .map_err(|error| error.to_string())?;
        let identity = ClientIdentity::new("ownership-profile", "ownership-profile")
            .map_err(|error| error.to_string())?;
        let namespace = Namespace::new("ownership-profile").map_err(|error| error.to_string())?;
        let key =
            StructuredKey::new(vec!["value".to_owned()]).map_err(|error| error.to_string())?;
        let map_key = store_key(&identity, &namespace, &key);
        let value = vec![0x5a; payload_bytes];
        let expected = value.clone();
        let request_pointer = value.as_ptr();

        let put = state.dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new(
                "ownership-put",
                ClientRequest::Put {
                    ns: namespace.clone(),
                    key: key.clone(),
                    value,
                    ttl_ms: None,
                    dimensions: Vec::new(),
                },
            ),
        );
        if put.result != Ok(ClientResponse::Stored) {
            return Err("ownership PUT was not stored".to_owned());
        }
        let stored_pointer = state
            .local_execution()
            .lock_store()
            .get(&map_key)
            .map(|stored| stored.value.as_ptr())
            .ok_or_else(|| "ownership PUT did not retain a value".to_owned())?;

        let get = state.dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new(
                "ownership-get",
                ClientRequest::Get {
                    ns: namespace.clone(),
                    key: key.clone(),
                },
            ),
        );
        let response_value = match get.result {
            Ok(ClientResponse::Value { value: Some(value) }) => value,
            _ => return Err("ownership GET did not return the stored value".to_owned()),
        };
        let response_pointer = response_value.as_ptr();

        let replacement = state.dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new(
                "ownership-replace",
                ClientRequest::Put {
                    ns: namespace,
                    key,
                    value: vec![0xa5; payload_bytes],
                    ttl_ms: None,
                    dimensions: Vec::new(),
                },
            ),
        );
        if replacement.result != Ok(ClientResponse::Stored) {
            return Err("ownership replacement was not stored".to_owned());
        }

        Ok(ClientValueOwnershipObservation {
            payload_bytes,
            request_to_store_pointer_preserved: payload_bytes == 0
                || request_pointer == stored_pointer,
            store_to_response_pointer_shared: payload_bytes == 0
                || stored_pointer == response_pointer,
            response_survived_store_replacement: response_value == expected,
        })
    }

    impl ExpirySweepProfileFixture {
        /// Build the fixed-cardinality W2 fixture.
        pub fn new(scenario: ExpirySweepProfileScenario) -> Self {
            let mut store = BTreeMap::new();
            for index in 0..FIXTURE_ENTRIES {
                let expires_at_ms = match scenario {
                    ExpirySweepProfileScenario::HalfExpired if index % 2 == 0 => NOW_MS,
                    ExpirySweepProfileScenario::AllExpired => NOW_MS,
                    _ => NOW_MS + 1,
                };
                store.insert(
                    fixed_store_key(index),
                    StoredValue {
                        value: Vec::new(),
                        expires_at_ms: Some(expires_at_ms),
                    },
                );
            }
            let cursor = (scenario == ExpirySweepProfileScenario::CursorWrapNoneExpired)
                .then(|| fixed_store_key(400));
            Self {
                scenario,
                store,
                cursor,
            }
        }

        /// Execute the real candidate selection and the request-path cursor
        /// clone, returning only allocation-free aggregate observations.
        pub fn run(&self) -> ExpirySweepProfileObservation {
            let candidates = expiry_sweep_candidates(
                &self.store,
                NOW_MS,
                self.cursor.as_ref(),
                SCAN_LIMIT,
                true,
            );
            let expired_identity_bytes =
                candidates.expired.iter().map(identity_bytes).sum::<usize>();
            let cursor_identity_bytes = candidates.next_cursor.as_ref().map_or(0, identity_bytes);
            let observation = ExpirySweepProfileObservation {
                examined_keys: candidates.examined,
                expired_keys: candidates.expired.len(),
                cloned_keys: candidates.expired.len()
                    + usize::from(candidates.next_cursor.is_some()),
                cloned_identity_bytes: expired_identity_bytes + cursor_identity_bytes,
                next_cursor_present: candidates.next_cursor.is_some(),
            };
            std::hint::black_box((&candidates, self.scenario));
            observation
        }
    }

    fn fixed_store_key(index: usize) -> StoreKey {
        (
            fixed_component("tenant", 0, TENANT_BYTES),
            fixed_component("ns", 0, NAMESPACE_BYTES),
            fixed_component("key", index, KEY_BYTES),
        )
    }

    fn fixed_component(prefix: &str, index: usize, bytes: usize) -> String {
        let mut value = format!("{prefix}-{index:08}");
        value.extend(std::iter::repeat_n('x', bytes - value.len()));
        value
    }

    fn identity_bytes((tenant, namespace, key): &StoreKey) -> usize {
        tenant.len() + namespace.len() + key.len()
    }
}

/// Shared state for the public client surface.
#[derive(Debug)]
pub struct ClientSurfaceState {
    dispatcher: Arc<ClientSurfaceDispatcher>,
    local_backend: Arc<LocalClientDataBackend>,
}

impl ClientSurfaceState {
    /// Create local state with validated limits and static dispatch.
    pub fn new(limits: ClientSurfaceLimits) -> Result<Self, ClientSurfaceError> {
        Self::assemble(limits, None, LocalDispatchVariant::Static)
    }
    /// Create local state with tenant isolation and static dispatch.
    pub fn with_isolation(
        limits: ClientSurfaceLimits,
        isolation: ConsumerIsolation,
    ) -> Result<Self, ClientSurfaceError> {
        Self::assemble(limits, Some(isolation), LocalDispatchVariant::Static)
    }
    /// Select one of D1's two local dispatch candidates without creating another map.
    pub fn with_dispatch_variant(
        limits: ClientSurfaceLimits,
        variant: LocalDispatchVariant,
    ) -> Result<Self, ClientSurfaceError> {
        Self::assemble(limits, None, variant)
    }
    /// Select a local dispatch candidate with the existing tenant admission policy.
    pub fn with_isolation_and_dispatch(
        limits: ClientSurfaceLimits,
        isolation: ConsumerIsolation,
        variant: LocalDispatchVariant,
    ) -> Result<Self, ClientSurfaceError> {
        Self::assemble(limits, Some(isolation), variant)
    }
    fn assemble(
        limits: ClientSurfaceLimits,
        isolation: Option<ConsumerIsolation>,
        variant: LocalDispatchVariant,
    ) -> Result<Self, ClientSurfaceError> {
        let edge = ClientSurfaceEdgeState::new(limits, isolation)?;
        let local_backend = Arc::new(LocalClientDataBackend::new());
        let dispatcher = Arc::new(ClientSurfaceDispatcher::local(
            edge,
            Arc::clone(&local_backend),
            variant,
        ));
        Ok(Self {
            dispatcher,
            local_backend,
        })
    }
    /// Return the shared asynchronous edge dispatcher used by every production adapter.
    pub fn dispatcher(&self) -> Arc<ClientSurfaceDispatcher> {
        Arc::clone(&self.dispatcher)
    }
    /// Return the actual local data owner for contract checks and local diagnostics.
    pub fn local_backend(&self) -> Arc<LocalClientDataBackend> {
        Arc::clone(&self.local_backend)
    }
    fn local_execution(&self) -> LocalBackendExecution<'_> {
        LocalBackendExecution::new(&self.local_backend, &self.dispatcher.edge)
    }
    /// Local-only synchronous compatibility helper. Transport handlers use the dispatcher.
    pub fn dispatch_verified_request(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> ClientResponseEnvelope {
        self.dispatcher.dispatch_local_request(identity, envelope)
    }
    /// Configured edge limits.
    pub fn limits(&self) -> ClientSurfaceLimits {
        self.dispatcher.limits()
    }
    /// Requests reaching canonical dispatch.
    pub fn dispatch_attempts(&self) -> u64 {
        self.dispatcher.edge.dispatch_attempts()
    }
    /// Successful state transitions.
    pub fn state_mutations(&self) -> u64 {
        self.dispatcher.edge.state_mutations()
    }
    /// Anonymous admission refusals.
    pub fn rejected_anonymous(&self) -> u64 {
        self.dispatcher.edge.rejected_anonymous()
    }
    /// Oversized admission refusals.
    pub fn rejected_oversized(&self) -> u64 {
        self.dispatcher.edge.rejected_oversized()
    }
    /// Active subscription reservations.
    pub fn active_subscriptions(&self) -> u64 {
        self.dispatcher.edge.active_subscriptions()
    }
    /// Enable the existing optional performance instrumentation.
    pub fn set_profile_instrumentation_enabled(&self, enabled: bool) {
        self.dispatcher
            .edge
            .set_profile_instrumentation_enabled(enabled);
    }
    /// Reset aggregate instrumentation at a quiescent boundary.
    pub fn reset_profile_metrics(&self) {
        self.dispatcher.edge.reset_profile_metrics();
    }
    /// Snapshot the existing instrumentation counters.
    pub fn profile_metrics(&self) -> ClientSurfaceProfileMetrics {
        self.dispatcher.edge.profile_metrics()
    }
    /// Subscribe to this edge's bounded tenant-scoped mutation stream.
    pub fn subscribe_mutations(
        &self,
        identity: &ClientIdentity,
    ) -> Result<ClientSurfaceMutationSubscriber, ClientSurfaceError> {
        self.dispatcher.edge.subscribe_mutations(identity)
    }
    /// Configure the separate modeled CP lock leader for tests.
    pub fn set_lock_leader_for_tests(&self, local: u64, leader: u64) {
        self.dispatcher
            .edge
            .set_lock_leader_for_tests(local, leader);
    }
    /// Advance the separate modeled CP lock clock for tests.
    pub fn advance_lock_logical_time_for_tests(&self, millis: u64) {
        self.dispatcher
            .edge
            .advance_lock_logical_time_for_tests(millis);
    }
    /// Override this local owner's cache clock for deterministic tests.
    pub fn set_cache_time_for_tests(&self, now: Option<u64>) {
        self.local_execution().set_cache_time_for_tests(now);
    }
    /// Advance this local owner's cache clock for deterministic tests.
    pub fn advance_cache_time_for_tests(&self, millis: u64) {
        self.local_execution().advance_cache_time_for_tests(millis);
    }
    /// Return the retained consumer audit history.
    pub fn audit_events_for_tests(&self) -> Vec<AuditEvent> {
        self.dispatcher.edge.audit_events_for_tests()
    }
    /// Snapshot local owners at an explicitly quiescent diagnostic checkpoint.
    pub fn retained_state_for_diagnostics(&self) -> ClientSurfaceRetainedState {
        self.local_execution().retained_state_for_diagnostics()
    }
    /// Run the unchanged bounded local expiry maintenance tick.
    pub fn reap_expired_entries_for_maintenance(&self) -> usize {
        self.local_execution()
            .reap_expired_entries_for_maintenance()
    }
    /// Reset the explicitly enabled local diagnostic owners; tenant-isolated state refuses.
    pub fn reset_retained_state_for_diagnostics(
        &self,
    ) -> Result<ClientSurfaceDiagnosticReset, ClientSurfaceError> {
        self.local_execution()
            .reset_retained_state_for_diagnostics()
    }
    /// Return tenant-scoped admission and subscription status.
    pub fn tenant_status(
        &self,
        identity: &ClientIdentity,
    ) -> Result<TenantStatus, ClientSurfaceError> {
        self.dispatcher.edge.tenant_status(identity)
    }
    fn begin_subscription(&self) {
        self.dispatcher.edge.begin_subscription();
    }
    fn drain_subscriptions(&self) -> u64 {
        self.dispatcher.edge.drain_subscriptions()
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

    /// Create a route owner with tenant isolation.
    pub fn with_isolation(
        limits: ClientSurfaceLimits,
        isolation: ConsumerIsolation,
    ) -> Result<Self, ClientSurfaceError> {
        Ok(Self {
            state: Arc::new(ClientSurfaceState::with_isolation(limits, isolation)?),
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
        self.state.dispatcher.routes()
    }
}

async fn client_data(
    State(state): State<Arc<ClientSurfaceDispatcher>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match validate_before_dispatch(&state, &headers, body.len(), CLIENT_DATA_PATH) {
        Ok(identity) => {
            state.edge.record_dispatch();
            let decoded = ClientFrame::decode(&body, state.limits().max_frame_bytes)
                .and_then(|frame| frame.decode_message())
                .map_err(|error| ClientSurfaceError::MalformedFrame(error.to_string()));
            let result = match decoded {
                Ok(message) => state
                    .handle_wire_message(&identity, message)
                    .await
                    .and_then(|response| encode_wire_message(&response)),
                Err(error) => Err(error),
            };
            match result {
                Ok(bytes) => (StatusCode::OK, bytes).into_response(),
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
    State(state): State<Arc<ClientSurfaceDispatcher>>,
    headers: HeaderMap,
) -> Response {
    match ClientIdentity::from_headers(&headers) {
        Ok(identity) => match state.edge.tenant_status(&identity) {
            Ok(tenant_status) => (
                StatusCode::OK,
                Json(ClientStatusReply::from_identity(identity, tenant_status)),
            )
                .into_response(),
            Err(error) => error.into_response(),
        },
        Err(error) => error.into_response(),
    }
}

async fn client_subscription(
    State(state): State<Arc<ClientSurfaceDispatcher>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match validate_before_dispatch(&state, &headers, body.len(), CLIENT_SUBSCRIPTIONS_PATH) {
        Ok(_identity) => {
            if state.edge.active_subscriptions() as usize
                >= state.limits().max_streams_per_connection
            {
                return ClientSurfaceError::TooManyStreams.into_response();
            }
            state.edge.begin_subscription();
            state.edge.record_dispatch();
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
    state: &ClientSurfaceDispatcher,
    headers: &HeaderMap,
    body_len: usize,
    route: &str,
) -> Result<ClientIdentity, ClientSurfaceError> {
    let identity = match ClientIdentity::from_headers(headers) {
        Ok(identity) => identity,
        Err(error) => {
            state.edge.reject_anonymous();
            return Err(error);
        }
    };
    if body_len > state.limits().max_frame_bytes {
        state.edge.reject_oversized();
        return Err(ClientSurfaceError::FrameTooLarge {
            actual: body_len,
            max: state.limits().max_frame_bytes,
        });
    }
    state
        .edge
        .validate_tenant_identity(&identity, route, None)?;
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

fn encode_wire_message(message: &ClientWireMessage) -> Result<Vec<u8>, ClientSurfaceError> {
    Ok(
        ClientFrame::from_message_with_version(message.protocol_version(), message)
            .map_err(|error| ClientSurfaceError::MalformedFrame(error.to_string()))?
            .encode()
            .map_err(|error| ClientSurfaceError::MalformedFrame(error.to_string()))?
            .to_vec(),
    )
}

fn admission_error(error: AdmissionRejection) -> ClientErrorEnvelope {
    match error {
        AdmissionRejection::RejectQuota {
            tenant,
            namespace,
            retry_after,
        } => ClientErrorEnvelope::new(
            ClientErrorCode::TenantQuota,
            true,
            format!(
                "tenant {} namespace {namespace} quota exceeded",
                tenant.as_str()
            ),
        )
        .with_retry_after_ms(duration_millis_saturating(retry_after)),
        AdmissionRejection::RejectRate {
            tenant,
            retry_after,
        } => ClientErrorEnvelope::new(
            ClientErrorCode::RateLimited,
            true,
            format!("tenant {} rate limited", tenant.as_str()),
        )
        .with_retry_after_ms(duration_millis_saturating(retry_after)),
        AdmissionRejection::UnknownTenant | AdmissionRejection::UnknownNamespace { .. } => {
            ClientErrorEnvelope::new(
                ClientErrorCode::Unauthorized,
                false,
                "tenant is not authorized for this namespace",
            )
        }
        AdmissionRejection::GlobalLimit { reason } => ClientErrorEnvelope::new(
            ClientErrorCode::TooLarge,
            false,
            format!("request exceeds {reason}"),
        ),
    }
}

fn duration_millis_saturating(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn request_namespace(request: &ClientRequest) -> &Namespace {
    match request {
        ClientRequest::Get { ns, .. }
        | ClientRequest::Put { ns, .. }
        | ClientRequest::Invalidate { ns, .. }
        | ClientRequest::BatchGet { ns, .. }
        | ClientRequest::BatchPut { ns, .. }
        | ClientRequest::Expire { ns, .. }
        | ClientRequest::Persist { ns, .. }
        | ClientRequest::GetTtl { ns, .. }
        | ClientRequest::ConditionalPut { ns, .. }
        | ClientRequest::CompareValueAndInvalidate { ns, .. }
        | ClientRequest::CompareValueAndExpire { ns, .. }
        | ClientRequest::EvictRegion { ns }
        | ClientRequest::SubscribeInvalidations { ns, .. }
        | ClientRequest::SubscribeEntryEvents { ns, .. }
        | ClientRequest::TryLock { ns, .. }
        | ClientRequest::Unlock { ns, .. }
        | ClientRequest::RenewLockLease { ns, .. }
        | ClientRequest::ForceUnlock { ns, .. }
        | ClientRequest::GetLockOwnership { ns, .. }
        | ClientRequest::CompareAndSet { ns, .. }
        | ClientRequest::RemoveIfValue { ns, .. } => ns,
    }
}

fn lock_response(
    request_id: String,
    result: Result<ClientResponse, ClientErrorEnvelope>,
) -> ClientResponseEnvelope {
    match result {
        Ok(response) => ClientResponseEnvelope::ok(request_id, response),
        Err(error) => ClientResponseEnvelope::error(request_id, error),
    }
}

fn lock_key(identity: &ClientIdentity, ns: &Namespace, key: &StructuredKey) -> String {
    let tenant = identity.tenant();
    let namespace = ns.as_str();
    let stable_key = key.stable_key();
    format!(
        "{}:{}|{}:{}|{}:{}",
        tenant.len(),
        tenant,
        namespace.len(),
        namespace,
        stable_key.len(),
        stable_key,
    )
}

fn lock_owner(identity: &ClientIdentity) -> LockOwner {
    LockOwner::new(format!("{}:{}", identity.tenant(), identity.client_id()), 0)
}

fn lock_consistency(level: LockConsistency) -> ConsistencyLevel {
    match level {
        LockConsistency::One => ConsistencyLevel::One,
        LockConsistency::Quorum => ConsistencyLevel::Quorum,
        LockConsistency::EachQuorum => ConsistencyLevel::EachQuorum,
        LockConsistency::All => ConsistencyLevel::All,
    }
}

fn sync_conditional_record(
    store: &mut SingleKeyConditionalStore,
    key: &str,
    live_value: Option<&[u8]>,
) -> Result<(), ConditionalError> {
    let current = store.current_value(key);
    if current.as_deref() == live_value {
        return Ok(());
    }
    match live_value {
        Some(value) => {
            store.compare_and_set(
                key,
                current.as_deref(),
                value.to_vec(),
                ConsistencyLevel::Quorum,
            )?;
        }
        None => {
            if let Some(current) = current {
                store.remove_if_value(key, &current, ConsistencyLevel::Quorum)?;
            }
        }
    }
    Ok(())
}

fn apply_compare_and_set(
    service: &mut ClientLockService,
    key: &str,
    live_value: Option<&[u8]>,
    expected: &CasExpectation,
    new_value: Vec<u8>,
    level: ConsistencyLevel,
) -> Result<ClientResponse, ClientErrorEnvelope> {
    service.apply_lock_command(|store, _now| {
        sync_conditional_record(store, key, live_value)?;
        let result = match expected {
            CasExpectation::Exact(expected) => {
                store.compare_and_set(key, Some(expected), new_value, level)?
            }
            CasExpectation::Present => store.replace_if_present(key, new_value, level)?,
        };
        Ok(cas_response(result))
    })
}

fn cas_response(result: CasResult) -> ClientResponse {
    match result {
        CasResult::Applied { new_version } => ClientResponse::CasApplied { new_version },
        CasResult::Mismatch { current } => ClientResponse::CasMismatch { current },
    }
}

fn lock_error_envelope(error: ConditionalError) -> ClientErrorEnvelope {
    let code = match error {
        ConditionalError::WeakConsistency { .. }
        | ConditionalError::MultiKeyRejected { .. }
        | ConditionalError::StaleFenceToken { .. }
        | ConditionalError::LockNotHeld
        | ConditionalError::LeaseExpired { .. }
        | ConditionalError::NotOwner { .. }
        | ConditionalError::ReentrancyLimit { .. } => ClientErrorCode::Conflict,
    };
    ClientErrorEnvelope::new(code, false, error.to_string())
}

fn live_value(
    store: &mut BTreeMap<StoreKey, StoredValue>,
    key: &StoreKey,
    now_ms: u64,
) -> (Option<Vec<u8>>, bool) {
    if store.get(key).is_some_and(|entry| entry.is_expired(now_ms)) {
        store.remove(key);
        return (None, true);
    }
    (store.get(key).map(|entry| entry.value.clone()), false)
}

fn stored_value(value: Vec<u8>, now_ms: u64, ttl_ms: Option<u64>) -> StoredValue {
    match ttl_ms {
        Some(ttl_ms) => StoredValue::with_ttl(value, now_ms, ttl_ms),
        None => StoredValue::persistent(value),
    }
}

fn live_entry_mut<'a>(
    store: &'a mut BTreeMap<StoreKey, StoredValue>,
    key: &StoreKey,
    now_ms: u64,
) -> (Option<&'a mut StoredValue>, bool) {
    if store.get(key).is_some_and(|entry| entry.is_expired(now_ms)) {
        store.remove(key);
        return (None, true);
    }
    (store.get_mut(key), false)
}

fn ttl_state(
    store: &mut BTreeMap<StoreKey, StoredValue>,
    key: &StoreKey,
    now_ms: u64,
) -> (TtlState, bool) {
    let Some(entry) = store.get(key) else {
        return (TtlState::Missing, false);
    };
    let state = entry.ttl_state(now_ms);
    if matches!(state, TtlState::Missing) {
        store.remove(key);
        return (state, true);
    }
    (state, false)
}

fn system_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
std::thread_local! {
    static PROFILE_ELAPSED_NANOSECONDS_FOR_TESTS: std::cell::Cell<Option<u64>> = const {
        std::cell::Cell::new(None)
    };
}

fn elapsed_nanoseconds(started: Instant) -> u64 {
    #[cfg(test)]
    if let Some(elapsed) = PROFILE_ELAPSED_NANOSECONDS_FOR_TESTS.with(std::cell::Cell::get) {
        return elapsed;
    }
    started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
}

#[cfg(feature = "performance-profile")]
fn store_key(identity: &ClientIdentity, ns: &Namespace, key: &StructuredKey) -> StoreKey {
    (
        identity.tenant().to_owned(),
        ns.as_str().to_owned(),
        key.stable_key(),
    )
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
    /// Tenant-scoped consumer status.
    pub tenant_status: TenantStatus,
}

impl ClientStatusReply {
    fn from_identity(identity: ClientIdentity, tenant_status: TenantStatus) -> Self {
        Self {
            client_id: identity.client_id,
            tenant: identity.tenant,
            route_version: 1,
            tenant_status,
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
    /// Client identity is not allowed by the tenant roster.
    #[error("client identity is not authorized for this tenant")]
    Unauthorized,
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
    /// Diagnostic reset cannot safely clear tenant-isolated quota state.
    #[error("diagnostic reset is unavailable for tenant-isolated state")]
    DiagnosticResetUnavailable,
    /// Invalid zero limit.
    #[error("client surface limit {0} must be greater than zero")]
    InvalidLimit(&'static str),
    /// Stable protocol request error.
    #[error("client protocol error: {0:?}")]
    Protocol(ClientErrorEnvelope),
    /// Malformed frame or unexpected wire message.
    #[error("malformed client frame: {0}")]
    MalformedFrame(String),
}

impl IntoResponse for ClientSurfaceError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::Unauthorized => StatusCode::FORBIDDEN,
            Self::FrameTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            Self::TooManyStreams => StatusCode::TOO_MANY_REQUESTS,
            Self::Draining => StatusCode::SERVICE_UNAVAILABLE,
            Self::DiagnosticResetUnavailable => StatusCode::CONFLICT,
            Self::InvalidLimit(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Protocol(_) | Self::MalformedFrame(_) => StatusCode::BAD_REQUEST,
        };
        (status, Json(ClientSurfaceReply::rejected(self.to_string()))).into_response()
    }
}

#[cfg(test)]
mod retention_tests {
    use super::*;

    #[cfg(feature = "performance-profile")]
    #[test]
    fn client_value_ownership_profile_covers_registered_payload_sizes() {
        for payload_bytes in [0, 64, 4 * 1024, 1024 * 1024] {
            let observation =
                performance_profile::observe_client_value_ownership(payload_bytes).unwrap();
            assert_eq!(observation.payload_bytes, payload_bytes);
            assert!(observation.request_to_store_pointer_preserved);
            assert_eq!(
                observation.store_to_response_pointer_shared,
                payload_bytes == 0
            );
            assert!(observation.response_survived_store_replacement);
        }
    }

    #[test]
    fn profile_metrics_attribute_identity_audit_and_value_ownership() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        let identity = ClientIdentity::new("profile-client", "profile-tenant").unwrap();
        let namespace = Namespace::new("profile").unwrap();
        let key = StructuredKey::new(vec!["key".to_owned()]).unwrap();
        state.set_profile_instrumentation_enabled(true);
        state.reset_profile_metrics();

        let put = state.dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new(
                "put",
                ClientRequest::Put {
                    ns: namespace.clone(),
                    key: key.clone(),
                    value: vec![0x5a; 64],
                    ttl_ms: None,
                    dimensions: Vec::new(),
                },
            ),
        );
        assert_eq!(put.result, Ok(ClientResponse::Stored));

        let get = state.dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new("get", ClientRequest::Get { ns: namespace, key }),
        );
        assert_eq!(
            get.result,
            Ok(ClientResponse::Value {
                value: Some(vec![0x5a; 64])
            })
        );

        let metrics = state.profile_metrics();
        assert_eq!(metrics.dispatches, 2);
        assert_eq!(metrics.identity_validations, 2);
        assert_eq!(metrics.protocol_validations, 2);
        assert_eq!(metrics.audit_context_bytes_cloned, 20);
        assert_eq!(metrics.put_value_bytes_moved, 64);
        assert_eq!(metrics.get_value_bytes_cloned, 64);
        assert_eq!(metrics.live_get_hits, 1);
    }

    fn put(
        state: &ClientSurfaceState,
        identity: &ClientIdentity,
        request_id: String,
        idempotency_key: Option<String>,
    ) -> ClientResponseEnvelope {
        put_for_key(
            state,
            identity,
            request_id,
            "shared-key".to_owned(),
            idempotency_key,
        )
    }

    fn put_for_key(
        state: &ClientSurfaceState,
        identity: &ClientIdentity,
        request_id: String,
        key: String,
        idempotency_key: Option<String>,
    ) -> ClientResponseEnvelope {
        put_for_key_with_ttl(state, identity, request_id, key, idempotency_key, None)
    }

    fn put_for_key_with_ttl(
        state: &ClientSurfaceState,
        identity: &ClientIdentity,
        request_id: String,
        key: String,
        idempotency_key: Option<String>,
        ttl_ms: Option<u64>,
    ) -> ClientResponseEnvelope {
        let mut request = ClientRequestEnvelope::new(
            request_id,
            ClientRequest::Put {
                ns: Namespace::new("retention").unwrap(),
                key: StructuredKey::new(vec![key]).unwrap(),
                value: b"value".to_vec(),
                ttl_ms,
                dimensions: Vec::new(),
            },
        );
        request.idempotency_key = idempotency_key;
        state.dispatch_verified_request(identity, request)
    }

    #[test]
    fn expired_entries_are_reaped_without_key_access() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        let identity = ClientIdentity::new("client", "tenant").unwrap();
        state.set_cache_time_for_tests(Some(1_000));
        for index in 0..128 {
            let response = put_for_key_with_ttl(
                &state,
                &identity,
                format!("put-{index}"),
                format!("ttl-{index}"),
                None,
                Some(500),
            );
            assert!(response.result.is_ok());
        }
        assert_eq!(state.retained_state_for_diagnostics().store_entries, 128);

        state.advance_cache_time_for_tests(501);
        let reclaimed = state.retained_state_for_diagnostics();

        assert_eq!(reclaimed.store_entries, 0);
        assert_eq!(reclaimed.value_bytes, 0);
        assert_eq!(reclaimed.store_identity_bytes, 0);
    }

    #[test]
    fn request_path_expiry_sweep_has_a_fixed_scan_budget_and_makes_progress() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        let identity = ClientIdentity::new("client", "tenant").unwrap();
        state.set_cache_time_for_tests(Some(1_000));
        let entries = CLIENT_SURFACE_EXPIRY_SWEEP_SCAN_LIMIT + 44;
        for index in 0..entries {
            let response = put_for_key_with_ttl(
                &state,
                &identity,
                format!("put-{index}"),
                format!("ttl-{index:04}"),
                None,
                Some(500),
            );
            assert!(response.result.is_ok());
        }

        state.advance_cache_time_for_tests(1_001);
        assert_eq!(
            state.reap_expired_entries_for_maintenance(),
            CLIENT_SURFACE_EXPIRY_SWEEP_SCAN_LIMIT
        );
        assert_eq!(
            state.local_backend.store.lock().expect("store mutex").len(),
            44
        );

        state.advance_cache_time_for_tests(CLIENT_SURFACE_EXPIRY_SWEEP_INTERVAL_MS);
        assert_eq!(state.reap_expired_entries_for_maintenance(), 44);
        assert!(state
            .local_backend
            .store
            .lock()
            .expect("store mutex")
            .is_empty());

        state.advance_cache_time_for_tests(CLIENT_SURFACE_EXPIRY_SWEEP_INTERVAL_MS);
        assert_eq!(state.reap_expired_entries_for_maintenance(), 0);
        assert!(state
            .local_backend
            .expiry_sweep_cursor
            .lock()
            .expect("expiry cursor mutex")
            .is_none());
    }

    #[test]
    fn expiry_sweep_candidate_wraps_once_and_full_scan_retains_no_cursor() {
        let store = ["a", "b", "c", "d"]
            .into_iter()
            .map(|key| {
                (
                    ("tenant".to_owned(), "namespace".to_owned(), key.to_owned()),
                    StoredValue::persistent(Vec::new()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let cursor = ("tenant".to_owned(), "namespace".to_owned(), "c".to_owned());

        let bounded = expiry_sweep_candidates(&store, 1_000, Some(&cursor), 3, true);
        assert_eq!(bounded.examined, 3);
        assert!(bounded.expired.is_empty());
        assert_eq!(
            bounded.next_cursor.as_ref().map(|(_, _, key)| key.as_str()),
            Some("b")
        );

        let full = expiry_sweep_candidates(&store, 1_000, None, usize::MAX, false);
        assert_eq!(full.examined, store.len());
        assert!(full.expired.is_empty());
        assert!(full.next_cursor.is_none());
    }

    #[test]
    fn expiry_reaper_releases_tenant_quota_accounting() {
        let roster = hydracache::TenantRoster::new(vec![hydracache::Tenant::new("tenant")
            .unwrap()
            .allow_client("client")
            .namespace("retention", hydracache::NamespaceQuota::new(5, 1))])
        .unwrap();
        let state = ClientSurfaceState::with_isolation(
            ClientSurfaceLimits::default(),
            ConsumerIsolation::new(roster, hydracache::ConsumerIsolationConfig::default()),
        )
        .unwrap();
        let identity = ClientIdentity::new("client", "tenant").unwrap();
        state.set_cache_time_for_tests(Some(1_000));
        assert!(put_for_key_with_ttl(
            &state,
            &identity,
            "first".to_owned(),
            "first".to_owned(),
            None,
            Some(500),
        )
        .result
        .is_ok());

        state.advance_cache_time_for_tests(501);
        assert_eq!(state.retained_state_for_diagnostics().store_entries, 0);
        assert!(put_for_key(
            &state,
            &identity,
            "replacement".to_owned(),
            "replacement".to_owned(),
            None,
        )
        .result
        .is_ok());
    }

    #[test]
    fn mutations_without_subscribers_do_not_retain_or_number_events() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        let identity = ClientIdentity::new("client", "tenant").unwrap();

        assert!(put(&state, &identity, "put".to_owned(), None)
            .result
            .is_ok());

        assert_eq!(
            state.dispatcher.edge.next_message_id.load(Ordering::SeqCst),
            1
        );
    }

    #[test]
    fn idempotency_outcomes_are_bounded_fail_loud_and_expire() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        let identity = ClientIdentity::new("client", "tenant").unwrap();
        state.set_cache_time_for_tests(Some(1_000));

        for index in 0..CLIENT_SURFACE_IDEMPOTENCY_CAPACITY {
            let response = put(
                &state,
                &identity,
                format!("put-{index}"),
                Some(format!("idempotency-{index}")),
            );
            assert!(response.result.is_ok());
        }
        assert_eq!(
            state
                .local_backend
                .idempotency_keys
                .lock()
                .expect("idempotency mutex")
                .len(),
            CLIENT_SURFACE_IDEMPOTENCY_CAPACITY
        );

        let saturated = put(
            &state,
            &identity,
            "saturated".to_owned(),
            Some("idempotency-overflow".to_owned()),
        );
        let error = saturated.result.unwrap_err();
        assert_eq!(error.code, ClientErrorCode::RateLimited);
        assert!(error.retryable);
        assert!(error.retry_after_ms.is_some());
        assert_eq!(
            state.state_mutations(),
            CLIENT_SURFACE_IDEMPOTENCY_CAPACITY as u64
        );

        state.advance_cache_time_for_tests(CLIENT_SURFACE_IDEMPOTENCY_TTL_MS + 1);
        let after_expiry = put(
            &state,
            &identity,
            "after-expiry".to_owned(),
            Some("idempotency-after-expiry".to_owned()),
        );
        assert!(after_expiry.result.is_ok());
        assert_eq!(
            state
                .local_backend
                .idempotency_keys
                .lock()
                .expect("idempotency mutex")
                .len(),
            1
        );
    }

    #[test]
    fn audit_pressure_is_bounded_and_blocks_mandatory_admin_mutation() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        for index in 0..hydracache_observability::IN_MEMORY_AUDIT_CAPACITY {
            state
                .dispatcher
                .edge
                .record_audit(AuditEvent::AuthFailure {
                    tenant: Some("tenant".to_owned()),
                    route: CLIENT_DATA_PATH.to_owned(),
                    request_id: Some(format!("request-{index}")),
                })
                .unwrap();
        }
        assert_eq!(
            state.audit_events_for_tests().len(),
            hydracache_observability::IN_MEMORY_AUDIT_CAPACITY
        );

        let mut admin = ClientIdentity::new("admin", "tenant").unwrap();
        admin.admin = true;
        let response = state.dispatcher.edge.handle_force_unlock(
            &admin,
            "force-unlock".to_owned(),
            Namespace::new("retention").unwrap(),
            StructuredKey::new(vec!["key".to_owned()]).unwrap(),
        );

        let error = response.result.unwrap_err();
        assert_eq!(error.code, ClientErrorCode::BackendUnavailable);
        assert!(!error.retryable);
        assert_eq!(
            state
                .dispatcher
                .edge
                .lock_service
                .lock()
                .expect("lock service mutex")
                .applied_commands,
            0
        );
        assert_eq!(
            state.audit_events_for_tests().len(),
            hydracache_observability::IN_MEMORY_AUDIT_CAPACITY
        );
    }

    #[test]
    fn diagnostic_reset_reports_and_clears_every_mutable_data_owner() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        let identity = ClientIdentity::new("client", "tenant").unwrap();
        assert!(put(
            &state,
            &identity,
            "put".to_owned(),
            Some("idempotency".to_owned()),
        )
        .result
        .is_ok());
        {
            let mut locks = state
                .dispatcher
                .edge
                .lock_service
                .lock()
                .expect("lock service mutex");
            let owner = LockOwner::new("session", 1);
            locks
                .store
                .try_acquire_lock(
                    "lock",
                    ConsistencyLevel::Quorum,
                    owner,
                    LogicalDuration::from_millis(100),
                    LogicalTime::from_millis(0),
                )
                .unwrap()
                .expect("lock acquired");
        }
        state
            .dispatcher
            .edge
            .record_audit(AuditEvent::AuthFailure {
                tenant: Some("tenant".to_owned()),
                route: CLIENT_DATA_PATH.to_owned(),
                request_id: Some("audit".to_owned()),
            })
            .unwrap();

        let reset = state.reset_retained_state_for_diagnostics().unwrap();

        assert_eq!(reset.before.store_entries, 1);
        assert_eq!(reset.before.idempotency_outcomes, 1);
        assert_eq!(reset.before.conditional.locks, 1);
        assert_eq!(reset.before.conditional.session_heartbeats, 1);
        assert_eq!(reset.before.audit_events, 1);
        assert_eq!(reset.after.store_entries, 0);
        assert_eq!(reset.after.value_bytes, 0);
        assert_eq!(reset.after.store_identity_bytes, 0);
        assert_eq!(reset.after.idempotency_outcomes, 0);
        assert_eq!(reset.after.idempotency_identity_bytes, 0);
        assert_eq!(reset.after.conditional, ConditionalRetainedState::default());
        assert_eq!(reset.after.audit_events, 1);
        assert_eq!(state.retained_state_for_diagnostics(), reset.after);
    }

    #[test]
    #[ignore = "0.70 million-mutation retention soak: exercised by scheduled/release evidence"]
    fn retention_soak_million_fixed_keyspace_mutations_plateau_and_reset() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
        let identity = ClientIdentity::new("client", "tenant").unwrap();
        let inject_append_only =
            std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W3_APPEND_ONLY");
        let mutations = if inject_append_only {
            10_000
        } else {
            1_000_000
        };
        let keyspace = if inject_append_only { mutations } else { 64 };

        for index in 0..mutations {
            let response = put_for_key(
                &state,
                &identity,
                format!("soak-{index}"),
                format!("key-{}", index % keyspace),
                None,
            );
            assert!(response.result.is_ok());
            if index > 0 && index % 100_000 == 0 {
                let retained = state.retained_state_for_diagnostics();
                assert!(retained.store_entries <= 64);
                assert_eq!(
                    state.dispatcher.edge.next_message_id.load(Ordering::SeqCst),
                    1
                );
            }
        }

        let plateau = state.retained_state_for_diagnostics();
        assert!(
            plateau.store_entries <= 64,
            "HC-CANARY-RED:W3: fixed-cardinality mutations retained {} store owners",
            plateau.store_entries
        );
        assert_eq!(plateau.idempotency_outcomes, 0);
        assert_eq!(plateau.audit_events, 0);
        assert_eq!(
            state.dispatcher.edge.next_message_id.load(Ordering::SeqCst),
            1
        );

        let reset = state.reset_retained_state_for_diagnostics().unwrap();
        assert_eq!(reset.after.store_entries, 0);
        assert_eq!(reset.after.value_bytes, 0);
        assert_eq!(reset.after.store_identity_bytes, 0);
        assert_eq!(reset.after.idempotency_outcomes, 0);
        assert_eq!(reset.after.conditional, ConditionalRetainedState::default());
    }
}

#[cfg(test)]
mod profiled_store_guard_tests {
    use super::*;
    use std::sync::TryLockError;

    #[test]
    fn profile_timing_accumulates_exact_samples_including_zero() {
        struct ResetElapsedSample;
        impl Drop for ResetElapsedSample {
            fn drop(&mut self) {
                PROFILE_ELAPSED_NANOSECONDS_FOR_TESTS.with(|sample| sample.set(None));
            }
        }

        for elapsed in [0, 37] {
            PROFILE_ELAPSED_NANOSECONDS_FOR_TESTS.with(|sample| sample.set(Some(elapsed)));
            let _reset_sample = ResetElapsedSample;
            let state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
            let identity = ClientIdentity::new("profile-client", "profile-tenant").unwrap();
            let ns = Namespace::new("profile-clock").unwrap();
            let key = StructuredKey::new(vec!["key".to_owned()]).unwrap();
            let put = || {
                state.dispatch_verified_request(
                    &identity,
                    ClientRequestEnvelope::new(
                        "put",
                        ClientRequest::Put {
                            ns: ns.clone(),
                            key: key.clone(),
                            value: vec![7; 256],
                            ttl_ms: None,
                            dimensions: Vec::new(),
                        },
                    ),
                )
            };
            assert!(matches!(put().result, Ok(ClientResponse::Stored)));
            assert_eq!(
                state.profile_metrics(),
                ClientSurfaceProfileMetrics::default()
            );

            state.set_profile_instrumentation_enabled(true);
            state.reset_profile_metrics();
            assert!(matches!(put().result, Ok(ClientResponse::Stored)));
            let get = state.dispatch_verified_request(
                &identity,
                ClientRequestEnvelope::new("get", ClientRequest::Get { ns, key }),
            );
            assert!(matches!(
                get.result,
                Ok(ClientResponse::Value { value: Some(ref value) }) if value == &vec![7; 256]
            ));
            let metrics = state.profile_metrics();
            assert_eq!(metrics.dispatches, 2);
            assert_eq!(metrics.identity_validations, 2);
            assert_eq!(metrics.identity_validation_nanoseconds, 2 * elapsed);
            assert_eq!(metrics.protocol_validations, 2);
            assert_eq!(metrics.protocol_validation_nanoseconds, 2 * elapsed);
            assert!(metrics.clock_reads >= 2);
            assert_eq!(metrics.clock_nanoseconds, metrics.clock_reads * elapsed);
            assert!(metrics.store_lock_acquisitions >= 2);
            assert_eq!(
                metrics.store_lock_wait_nanoseconds,
                metrics.store_lock_acquisitions * elapsed
            );
            assert_eq!(
                metrics.store_lock_hold_nanoseconds,
                metrics.store_lock_acquisitions * elapsed
            );

            state.set_profile_instrumentation_enabled(false);
            state.reset_profile_metrics();
            drop(state.local_execution().lock_store());
            state.local_execution().now_ms();
            assert_eq!(
                state.profile_metrics(),
                ClientSurfaceProfileMetrics::default()
            );
        }
    }

    #[test]
    fn profiled_store_guard_releases_borrow_on_drop_and_unwind() {
        for profiling in [false, true] {
            let store = Mutex::new(BTreeMap::<StoreKey, StoredValue>::new());
            let instrumentation = ClientSurfaceProfileInstrumentation::default();
            let make_guard = || ProfiledStoreGuard {
                guard: store.lock().unwrap(),
                hold_started: profiling.then(|| Instant::now() - Duration::from_millis(1)),
                instrumentation: &instrumentation,
            };
            {
                let guard = make_guard();
                assert!(guard.is_empty());
                assert!(matches!(store.try_lock(), Err(TryLockError::WouldBlock)));
            }
            assert!(store.try_lock().unwrap().is_empty());
            let normal_hold = instrumentation
                .store_lock_hold_nanoseconds
                .load(Ordering::Relaxed);
            assert_eq!(normal_hold > 0, profiling);

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _guard = make_guard();
                assert!(matches!(store.try_lock(), Err(TryLockError::WouldBlock)));
                panic!("intentional guard unwind");
            }));
            assert!(result.is_err());
            // Poisoning is preserved. Poisoned means the mutex was released, not leaked.
            match store.try_lock() {
                Err(TryLockError::Poisoned(error)) => assert!(error.into_inner().is_empty()),
                other => panic!("expected released poisoned store, got {other:?}"),
            }
            let final_hold = instrumentation
                .store_lock_hold_nanoseconds
                .load(Ordering::Relaxed);
            assert_eq!(final_hold > normal_hold, profiling);
        }
    }
}
