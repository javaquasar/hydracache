//! Axum route boundary for HydraCache external client traffic.
//!
//! This crate owns the public `/client/v1/*` surface. It is intentionally
//! separate from member-to-member cluster transport so public compatibility
//! cannot accidentally inherit private cluster route semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hydracache::{
    AdmissionRejection, CasResult, ClusterEpoch, ConditionalError, ConsistencyLevel,
    ConsumerIsolation, FenceToken, LockOwner, LogicalDuration, LogicalTime,
    SingleKeyConditionalStore, TenantId, TenantMetricsSnapshot,
};
use hydracache_client_protocol::{
    protocol_version_supported, BatchItemStatus, BatchPutEntry, CasExpectation, ClientErrorCode,
    ClientErrorEnvelope, ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, ClientWireMessage, InvalidationEvent, LockConsistency, Namespace,
    StructuredKey, VersionHandshake, PROTOCOL_VERSION,
};
use hydracache_observability::{AuditEvent, AuditRecorder, InMemoryAuditSink, TenantStatus};
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

/// Optional test/admin marker for privileged client operations.
pub const HYDRACACHE_ADMIN_HEADER: &str = "x-hydracache-admin";

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

    fn from_headers(headers: &HeaderMap) -> Result<Self, ClientSurfaceError> {
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

/// Shared state for the public client surface.
#[derive(Debug)]
pub struct ClientSurfaceState {
    limits: ClientSurfaceLimits,
    dispatch_attempts: AtomicU64,
    state_mutations: AtomicU64,
    rejected_anonymous: AtomicU64,
    rejected_oversized: AtomicU64,
    active_subscriptions: AtomicU64,
    next_message_id: AtomicU64,
    store: Mutex<BTreeMap<(String, String), Vec<u8>>>,
    events: Mutex<Vec<InvalidationEvent>>,
    idempotency_keys: Mutex<BTreeSet<String>>,
    lock_service: Mutex<ClientLockService>,
    audit_sink: Arc<InMemoryAuditSink>,
    audit: Mutex<AuditRecorder<Arc<InMemoryAuditSink>>>,
    isolation: Option<Mutex<ConsumerIsolation>>,
}

impl ClientSurfaceState {
    /// Create state with validated limits.
    pub fn new(limits: ClientSurfaceLimits) -> Result<Self, ClientSurfaceError> {
        limits.validate()?;
        let audit_sink = Arc::new(InMemoryAuditSink::new());
        Ok(Self {
            limits,
            dispatch_attempts: AtomicU64::new(0),
            state_mutations: AtomicU64::new(0),
            rejected_anonymous: AtomicU64::new(0),
            rejected_oversized: AtomicU64::new(0),
            active_subscriptions: AtomicU64::new(0),
            next_message_id: AtomicU64::new(1),
            store: Mutex::new(BTreeMap::new()),
            events: Mutex::new(Vec::new()),
            idempotency_keys: Mutex::new(BTreeSet::new()),
            lock_service: Mutex::new(ClientLockService::new()),
            audit: Mutex::new(AuditRecorder::new(Arc::clone(&audit_sink))),
            audit_sink,
            isolation: None,
        })
    }

    /// Create state with tenant isolation.
    pub fn with_isolation(
        limits: ClientSurfaceLimits,
        isolation: ConsumerIsolation,
    ) -> Result<Self, ClientSurfaceError> {
        limits.validate()?;
        let audit_sink = Arc::new(InMemoryAuditSink::new());
        Ok(Self {
            limits,
            dispatch_attempts: AtomicU64::new(0),
            state_mutations: AtomicU64::new(0),
            rejected_anonymous: AtomicU64::new(0),
            rejected_oversized: AtomicU64::new(0),
            active_subscriptions: AtomicU64::new(0),
            next_message_id: AtomicU64::new(1),
            store: Mutex::new(BTreeMap::new()),
            events: Mutex::new(Vec::new()),
            idempotency_keys: Mutex::new(BTreeSet::new()),
            lock_service: Mutex::new(ClientLockService::new()),
            audit: Mutex::new(AuditRecorder::new(Arc::clone(&audit_sink))),
            audit_sink,
            isolation: Some(Mutex::new(isolation)),
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

    /// Configure the modeled lock leader for integration tests.
    pub fn set_lock_leader_for_tests(&self, local_node_id: u64, leader_node_id: u64) {
        self.lock_service
            .lock()
            .expect("lock service mutex")
            .set_leader_for_tests(local_node_id, leader_node_id);
    }

    /// Advance the modeled logical lock time for integration tests.
    pub fn advance_lock_logical_time_for_tests(&self, millis: u64) {
        self.lock_service
            .lock()
            .expect("lock service mutex")
            .advance_time_for_tests(millis);
    }

    /// Return recorded consumer audit events for integration tests.
    pub fn audit_events_for_tests(&self) -> Vec<AuditEvent> {
        self.audit_sink.events()
    }

    /// Return a tenant-scoped consumer status.
    pub fn tenant_status(
        &self,
        identity: &ClientIdentity,
    ) -> Result<TenantStatus, ClientSurfaceError> {
        self.validate_tenant_identity(identity)?;
        let metrics = if let Some(isolation) = &self.isolation {
            let tenant_id =
                TenantId::new(identity.tenant()).map_err(|_| ClientSurfaceError::Unauthorized)?;
            isolation
                .lock()
                .expect("isolation mutex")
                .metrics_snapshot_for_tenant(&tenant_id)
                .ok_or(ClientSurfaceError::Unauthorized)?
        } else {
            TenantMetricsSnapshot::default()
        };
        Ok(TenantStatus::from_metrics(
            identity.tenant(),
            &metrics,
            self.active_subscriptions(),
            0,
        ))
    }

    fn validate_tenant_identity(
        &self,
        identity: &ClientIdentity,
    ) -> Result<(), ClientSurfaceError> {
        if let Some(isolation) = &self.isolation {
            let tenant = isolation
                .lock()
                .expect("isolation mutex")
                .resolve_tenant(identity.client_id())
                .map_err(|_| ClientSurfaceError::Unauthorized)?;
            if tenant.as_str() != identity.tenant() {
                return Err(ClientSurfaceError::Unauthorized);
            }
        }
        Ok(())
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

    fn handle_wire_message(
        &self,
        identity: &ClientIdentity,
        message: ClientWireMessage,
    ) -> Result<ClientWireMessage, ClientSurfaceError> {
        match message {
            ClientWireMessage::Handshake(client) => {
                let version = client
                    .negotiate(VersionHandshake::default())
                    .map_err(ClientSurfaceError::Protocol)?;
                Ok(ClientWireMessage::Handshake(VersionHandshake::new(
                    version, version,
                )))
            }
            ClientWireMessage::Request(envelope) => {
                let response = self.handle_request(identity, envelope);
                Ok(ClientWireMessage::Response(response))
            }
            ClientWireMessage::Response(_)
            | ClientWireMessage::Invalidation(_)
            | ClientWireMessage::Heartbeat(_) => Err(ClientSurfaceError::MalformedFrame(
                "client data route accepts handshake or request frames".to_owned(),
            )),
        }
    }

    fn handle_request(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> ClientResponseEnvelope {
        let request_protocol_version = envelope.protocol_version;
        let response_protocol_version = if protocol_version_supported(request_protocol_version) {
            request_protocol_version
        } else {
            PROTOCOL_VERSION
        };
        if let Err(error) = envelope.validate_protocol() {
            return ClientResponseEnvelope::error(envelope.request_id, error)
                .with_protocol_version(response_protocol_version);
        }
        if envelope.deadline_expired(0) {
            return ClientResponseEnvelope::error(
                envelope.request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::DeadlineExceeded,
                    true,
                    "request deadline expired",
                )
                .with_retry_after_ms(1),
            )
            .with_protocol_version(response_protocol_version);
        }

        let response = match envelope.request {
            ClientRequest::Get { ns, key } => {
                if let Err(error) = self.admit_request(identity) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    let value = self
                        .store
                        .lock()
                        .expect("store mutex")
                        .get(&store_key(&ns, &key))
                        .cloned();
                    ClientResponseEnvelope::ok(envelope.request_id, ClientResponse::Value { value })
                }
            }
            ClientRequest::Put {
                ns,
                key,
                value,
                ttl_ms: _,
                dimensions: _,
            } => self.handle_put(
                identity,
                envelope.request_id,
                envelope.idempotency_key,
                ns,
                key,
                value,
            ),
            ClientRequest::Invalidate { ns, key } => {
                if let Err(error) = self.admit_request(identity) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    self.store
                        .lock()
                        .expect("store mutex")
                        .remove(&store_key(&ns, &key));
                    self.state_mutations.fetch_add(1, Ordering::SeqCst);
                    self.record_invalidation(ns, key);
                    ClientResponseEnvelope::ok(envelope.request_id, ClientResponse::Invalidated)
                }
            }
            ClientRequest::BatchGet { ns, keys } => {
                if let Err(error) = self.admit_request(identity) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    let store = self.store.lock().expect("store mutex");
                    let items = keys
                        .iter()
                        .enumerate()
                        .map(|(index, key)| BatchItemStatus {
                            index,
                            result: Ok(store.get(&store_key(&ns, key)).cloned()),
                        })
                        .collect();
                    ClientResponseEnvelope::ok(envelope.request_id, ClientResponse::Batch { items })
                }
            }
            ClientRequest::BatchPut { ns, entries } => {
                if let Err(error) = self.admit_batch_put(identity, &ns, &entries) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    let mut store = self.store.lock().expect("store mutex");
                    let items = entries
                        .into_iter()
                        .enumerate()
                        .map(|(index, entry)| {
                            if entry.value.len() > self.limits.max_value_bytes {
                                return BatchItemStatus {
                                    index,
                                    result: Err(ClientErrorEnvelope::new(
                                        ClientErrorCode::TooLarge,
                                        false,
                                        "batch item value too large",
                                    )),
                                };
                            }
                            store.insert(store_key(&ns, &entry.key), entry.value);
                            self.state_mutations.fetch_add(1, Ordering::SeqCst);
                            BatchItemStatus {
                                index,
                                result: Ok(None),
                            }
                        })
                        .collect();
                    ClientResponseEnvelope::ok(envelope.request_id, ClientResponse::Batch { items })
                }
            }
            ClientRequest::EvictRegion { ns } => {
                if let Err(error) = self.admit_request(identity) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    self.store
                        .lock()
                        .expect("store mutex")
                        .retain(|(stored_ns, _), _| stored_ns != ns.as_str());
                    self.state_mutations.fetch_add(1, Ordering::SeqCst);
                    ClientResponseEnvelope::ok(envelope.request_id, ClientResponse::Evicted)
                }
            }
            ClientRequest::SubscribeInvalidations {
                ns: _,
                region: _,
                from,
                include_value: _,
            } => {
                if let Err(error) = self.begin_tenant_subscription(identity) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    ClientResponseEnvelope::ok(
                        envelope.request_id,
                        ClientResponse::Subscribed { from },
                    )
                }
            }
            ClientRequest::SubscribeEntryEvents {
                ns: _,
                region: _,
                from,
                include_value: _,
                projection: _,
            } => {
                if let Err(error) = self.begin_tenant_subscription(identity) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    ClientResponseEnvelope::ok(
                        envelope.request_id,
                        ClientResponse::Subscribed { from },
                    )
                }
            }
            ClientRequest::TryLock {
                ns,
                key,
                lease_ms,
                wait_ms: _,
                level,
            } => self.handle_try_lock(identity, envelope.request_id, ns, key, lease_ms, level),
            ClientRequest::Unlock { ns, key, fence } => {
                self.handle_unlock(identity, envelope.request_id, ns, key, fence)
            }
            ClientRequest::RenewLockLease {
                ns,
                key,
                fence,
                lease_ms,
            } => self.handle_renew_lock(identity, envelope.request_id, ns, key, fence, lease_ms),
            ClientRequest::ForceUnlock { ns, key } => {
                self.handle_force_unlock(identity, envelope.request_id, ns, key)
            }
            ClientRequest::GetLockOwnership { ns, key } => {
                self.handle_lock_ownership(envelope.request_id, ns, key)
            }
            ClientRequest::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            } => self.handle_compare_and_set(
                identity,
                envelope.request_id,
                CompareAndSetCommand {
                    ns,
                    key,
                    expected,
                    new_value,
                    level,
                },
            ),
            ClientRequest::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            } => {
                self.handle_remove_if_value(identity, envelope.request_id, ns, key, expected, level)
            }
        };
        response.with_protocol_version(response_protocol_version)
    }

    fn handle_try_lock(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        lease_ms: u64,
        level: LockConsistency,
    ) -> ClientResponseEnvelope {
        let lock_key = lock_key(&ns, &key);
        let owner = lock_owner(identity);
        let lease = LogicalDuration::from_millis(lease_ms.max(1));
        let level = lock_consistency(level);
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, now| {
                let acquired = store.try_acquire_lock(&lock_key, level, owner, lease, now)?;
                Ok(match acquired {
                    Some(fence) => ClientResponse::LockAcquired {
                        fence: fence.value(),
                    },
                    None => ClientResponse::LockBusy,
                })
            });
        lock_response(request_id, result)
    }

    fn handle_unlock(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
    ) -> ClientResponseEnvelope {
        let lock_key = lock_key(&ns, &key);
        let owner = lock_owner(identity);
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, _now| {
                store.release_lock(&lock_key, &owner, FenceToken::new(fence))?;
                Ok(ClientResponse::LockReleased)
            });
        lock_response(request_id, result)
    }

    fn handle_renew_lock(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
        lease_ms: u64,
    ) -> ClientResponseEnvelope {
        let lock_key = lock_key(&ns, &key);
        let owner = lock_owner(identity);
        let lease = LogicalDuration::from_millis(lease_ms.max(1));
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, now| {
                store.renew_lease(
                    &lock_key,
                    &owner,
                    FenceToken::new(fence),
                    now.saturating_add(lease),
                )?;
                Ok(ClientResponse::LockLeaseRenewed)
            });
        lock_response(request_id, result)
    }

    fn handle_force_unlock(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
    ) -> ClientResponseEnvelope {
        if !identity.is_admin() {
            self.record_audit(AuditEvent::AuthFailure {
                tenant: Some(identity.tenant().to_owned()),
                route: CLIENT_DATA_PATH.to_owned(),
                request_id: Some(request_id.clone()),
            });
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::Unauthorized,
                    false,
                    "force_unlock requires admin privileges",
                ),
            );
        }
        self.record_audit(AuditEvent::PolicyChanged {
            namespace: ns.as_str().to_owned(),
            policy_epoch: ClusterEpoch::new(1),
            summary: "force_unlock".to_owned(),
        });
        let lock_key = lock_key(&ns, &key);
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, _now| {
                store.force_unlock(&lock_key);
                Ok(ClientResponse::LockReleased)
            });
        lock_response(request_id, result)
    }

    fn handle_lock_ownership(
        &self,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
    ) -> ClientResponseEnvelope {
        let lock_key = lock_key(&ns, &key);
        let mut service = self.lock_service.lock().expect("lock service mutex");
        if !service.is_leader() {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::BackendUnavailable,
                    true,
                    format!(
                        "not leader for lock partition; leader={}",
                        service.leader_node_id
                    ),
                )
                .with_retry_after_ms(1),
            );
        }
        ClientResponseEnvelope::ok(request_id, service.current_ownership(&lock_key))
    }

    fn handle_compare_and_set(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        command: CompareAndSetCommand,
    ) -> ClientResponseEnvelope {
        let CompareAndSetCommand {
            ns,
            key,
            expected,
            new_value,
            level,
        } = command;
        if new_value.len() > self.limits.max_value_bytes {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(ClientErrorCode::TooLarge, false, "value too large"),
            );
        }
        if let Err(error) = self.admit_put(identity, &ns, &key, new_value.len() as u64) {
            return ClientResponseEnvelope::error(request_id, error);
        }

        let map_key = store_key(&ns, &key);
        let live_value = self
            .store
            .lock()
            .expect("store mutex")
            .get(&map_key)
            .cloned();
        let cas_key = lock_key(&ns, &key);
        let value_for_response = new_value.clone();
        let level = lock_consistency(level);
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, _now| {
                sync_conditional_record(store, &cas_key, live_value.as_deref())?;
                let result = match expected {
                    CasExpectation::Exact(expected) => {
                        store.compare_and_set(&cas_key, Some(&expected), new_value, level)?
                    }
                    CasExpectation::Present => {
                        store.replace_if_present(&cas_key, new_value, level)?
                    }
                };
                Ok(cas_response(result))
            });

        if matches!(&result, Ok(ClientResponse::CasApplied { .. })) {
            self.store
                .lock()
                .expect("store mutex")
                .insert(map_key, value_for_response);
            self.state_mutations.fetch_add(1, Ordering::SeqCst);
            self.record_invalidation(ns, key);
        }
        lock_response(request_id, result)
    }

    fn handle_remove_if_value(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        expected: Vec<u8>,
        level: LockConsistency,
    ) -> ClientResponseEnvelope {
        if let Err(error) = self.admit_request(identity) {
            return ClientResponseEnvelope::error(request_id, error);
        }

        let map_key = store_key(&ns, &key);
        let live_value = self
            .store
            .lock()
            .expect("store mutex")
            .get(&map_key)
            .cloned();
        let cas_key = lock_key(&ns, &key);
        let level = lock_consistency(level);
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, _now| {
                sync_conditional_record(store, &cas_key, live_value.as_deref())?;
                let result = store.remove_if_value(&cas_key, &expected, level)?;
                Ok(cas_response(result))
            });

        if matches!(&result, Ok(ClientResponse::CasApplied { .. })) {
            self.store.lock().expect("store mutex").remove(&map_key);
            self.state_mutations.fetch_add(1, Ordering::SeqCst);
            self.record_invalidation(ns, key);
        }
        lock_response(request_id, result)
    }

    fn record_audit(&self, event: AuditEvent) {
        let _ = self.audit.lock().expect("audit mutex").record(&event);
    }

    fn handle_put(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        idempotency_key: Option<String>,
        ns: Namespace,
        key: StructuredKey,
        value: Vec<u8>,
    ) -> ClientResponseEnvelope {
        if value.len() > self.limits.max_value_bytes {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(ClientErrorCode::TooLarge, false, "value too large"),
            );
        }
        if let Some(idempotency_key) = idempotency_key.as_ref() {
            let keys = self.idempotency_keys.lock().expect("idempotency mutex");
            if keys.contains(idempotency_key) {
                return ClientResponseEnvelope::ok(request_id, ClientResponse::Stored);
            }
        }
        if let Err(error) = self.admit_put(identity, &ns, &key, value.len() as u64) {
            return ClientResponseEnvelope::error(request_id, error);
        }
        if let Some(idempotency_key) = idempotency_key {
            self.idempotency_keys
                .lock()
                .expect("idempotency mutex")
                .insert(idempotency_key);
        }
        self.store
            .lock()
            .expect("store mutex")
            .insert(store_key(&ns, &key), value);
        self.state_mutations.fetch_add(1, Ordering::SeqCst);
        self.record_invalidation(ns, key);
        ClientResponseEnvelope::ok(request_id, ClientResponse::Stored)
    }

    fn admit_request(&self, identity: &ClientIdentity) -> Result<(), ClientErrorEnvelope> {
        if let Some(isolation) = &self.isolation {
            isolation
                .lock()
                .expect("isolation mutex")
                .admit_request(identity.client_id())
                .map(|_| ())
                .map_err(admission_error)
        } else {
            Ok(())
        }
    }

    fn admit_put(
        &self,
        identity: &ClientIdentity,
        ns: &Namespace,
        key: &StructuredKey,
        value_bytes: u64,
    ) -> Result<(), ClientErrorEnvelope> {
        if let Some(isolation) = &self.isolation {
            isolation
                .lock()
                .expect("isolation mutex")
                .admit_put(
                    identity.client_id(),
                    ns.as_str(),
                    &key.stable_key(),
                    value_bytes,
                )
                .map_err(admission_error)
        } else {
            Ok(())
        }
    }

    fn admit_batch_put(
        &self,
        identity: &ClientIdentity,
        ns: &Namespace,
        entries: &[BatchPutEntry],
    ) -> Result<(), ClientErrorEnvelope> {
        if let Some(isolation) = &self.isolation {
            let entries = entries
                .iter()
                .map(|entry| (entry.key.stable_key(), entry.value.len() as u64))
                .collect::<Vec<_>>();
            isolation
                .lock()
                .expect("isolation mutex")
                .admit_batch_put(identity.client_id(), ns.as_str(), &entries)
                .map_err(admission_error)
        } else {
            Ok(())
        }
    }

    fn begin_tenant_subscription(
        &self,
        identity: &ClientIdentity,
    ) -> Result<(), ClientErrorEnvelope> {
        if let Some(isolation) = &self.isolation {
            isolation
                .lock()
                .expect("isolation mutex")
                .begin_subscription(identity.client_id())
                .map_err(admission_error)
        } else {
            Ok(())
        }
    }

    fn record_invalidation(&self, ns: Namespace, key: StructuredKey) {
        let message_id = self.next_message_id.fetch_add(1, Ordering::SeqCst);
        self.events
            .lock()
            .expect("events mutex")
            .push(InvalidationEvent::new(ns, key, 1, message_id));
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
        Ok(identity) => {
            state.record_dispatch();
            match ClientFrame::decode(&body, state.limits().max_frame_bytes)
                .and_then(|frame| frame.decode_message())
                .map_err(|error| ClientSurfaceError::MalformedFrame(error.to_string()))
                .and_then(|message| state.handle_wire_message(&identity, message))
                .and_then(|response| encode_wire_message(&response))
            {
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
    State(state): State<Arc<ClientSurfaceState>>,
    headers: HeaderMap,
) -> Response {
    match ClientIdentity::from_headers(&headers) {
        Ok(identity) => match state.tenant_status(&identity) {
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
    state.validate_tenant_identity(&identity)?;
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

fn lock_response(
    request_id: String,
    result: Result<ClientResponse, ClientErrorEnvelope>,
) -> ClientResponseEnvelope {
    match result {
        Ok(response) => ClientResponseEnvelope::ok(request_id, response),
        Err(error) => ClientResponseEnvelope::error(request_id, error),
    }
}

fn lock_key(ns: &Namespace, key: &StructuredKey) -> String {
    format!("{}:{}", ns.as_str(), key.stable_key())
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

fn store_key(ns: &Namespace, key: &StructuredKey) -> (String, String) {
    (ns.as_str().to_owned(), key.stable_key())
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
            Self::InvalidLimit(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Protocol(_) | Self::MalformedFrame(_) => StatusCode::BAD_REQUEST,
        };
        (status, Json(ClientSurfaceReply::rejected(self.to_string()))).into_response()
    }
}
