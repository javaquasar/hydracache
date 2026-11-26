//! Shared edge admission and asynchronous data dispatch. Backends never own HTTP routes.
use std::future::Future;
use std::pin::Pin;

use super::*;

/// Data result without transport-owned correlation or protocol metadata.
pub type ClientBackendOutcome = Result<ClientResponse, ClientErrorEnvelope>;
/// Object-safe asynchronous backend result used by the dynamic D1 candidate.
pub type ClientBackendFuture<'a> = Pin<Box<dyn Future<Output = ClientBackendOutcome> + Send + 'a>>;

/// A request whose identity, protocol and deadline passed the common edge checks.
///
/// Only the dispatcher can create one. The backend owns data admission and mutation;
/// the edge retains audit, correlation metadata, subscriptions and the CP lock engine.
pub struct VerifiedClientRequest<'a> {
    pub(super) edge: &'a ClientSurfaceEdgeState,
    pub(super) identity: &'a ClientIdentity,
    pub(super) envelope: ClientRequestEnvelope,
    pub(super) profiling: bool,
    transport_deadline: Option<tokio::time::Instant>,
}

impl VerifiedClientRequest<'_> {
    /// Identity admitted by the common tenant boundary.
    pub fn identity(&self) -> &ClientIdentity {
        self.identity
    }
    /// Read the admitted operation and its original metadata.
    pub fn envelope(&self) -> &ClientRequestEnvelope {
        &self.envelope
    }
    /// Data bounds frozen at the edge's construction.
    pub fn limits(&self) -> ClientSurfaceLimits {
        self.edge.limits()
    }

    /// Monotonic deadline translated by the transport from its admitted request metadata.
    pub fn transport_deadline(&self) -> Option<tokio::time::Instant> {
        self.transport_deadline
    }

    /// Transfer the admitted payload to a backend implementation.
    pub fn into_envelope(self) -> ClientRequestEnvelope {
        self.envelope
    }
}

/// Transport-neutral asynchronous data execution boundary.
pub trait ClientDataBackend: std::fmt::Debug + Send + Sync {
    /// Execute an admitted data request. Never substitute a local map for an unavailable owner.
    fn execute<'a>(&'a self, request: VerifiedClientRequest<'a>) -> ClientBackendFuture<'a>;
}

/// The two D1 candidates. Selecting a candidate does not admit its performance or distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocalDispatchVariant {
    /// Static enum dispatch without allocating a future per operation.
    #[default]
    Static,
    /// Trait-object dispatch with one boxed future per data operation.
    BoxedFuture,
}

#[derive(Debug)]
enum BackendDispatch {
    Local(Arc<LocalClientDataBackend>),
    Async {
        backend: Arc<dyn ClientDataBackend>,
        // Only the explicit local factory supplies this same data owner.
        // External/member backends never construct an unused local map.
        local_maintenance: Option<Arc<LocalClientDataBackend>>,
    },
}

/// Shared admission and asynchronous dispatcher for HC/1, HC/2 and RESP.
#[derive(Debug)]
pub struct ClientSurfaceDispatcher {
    pub(super) edge: ClientSurfaceEdgeState,
    backend: BackendDispatch,
}

struct ResponseContext {
    request_id: String,
    namespace: String,
    audit_auth_failure: bool,
    protocol_version: u16,
}

impl ClientSurfaceDispatcher {
    pub(super) fn local(
        edge: ClientSurfaceEdgeState,
        local: Arc<LocalClientDataBackend>,
        variant: LocalDispatchVariant,
    ) -> Self {
        let backend = match variant {
            LocalDispatchVariant::Static => BackendDispatch::Local(local),
            LocalDispatchVariant::BoxedFuture => BackendDispatch::Async {
                backend: local.clone(),
                local_maintenance: Some(local),
            },
        };
        Self { edge, backend }
    }

    /// Assemble an asynchronous backend without creating any local data owner.
    /// Backend injection is not a distributed-capability or member-readiness grant.
    pub fn from_backend(
        limits: ClientSurfaceLimits,
        backend: Arc<dyn ClientDataBackend>,
    ) -> Result<Self, ClientSurfaceError> {
        Self::assemble_backend(limits, None, backend)
    }

    /// Assemble an asynchronous backend with the existing tenant identity boundary.
    pub fn from_backend_with_isolation(
        limits: ClientSurfaceLimits,
        isolation: ConsumerIsolation,
        backend: Arc<dyn ClientDataBackend>,
    ) -> Result<Self, ClientSurfaceError> {
        Self::assemble_backend(limits, Some(isolation), backend)
    }

    fn assemble_backend(
        limits: ClientSurfaceLimits,
        isolation: Option<ConsumerIsolation>,
        backend: Arc<dyn ClientDataBackend>,
    ) -> Result<Self, ClientSurfaceError> {
        Ok(Self {
            edge: ClientSurfaceEdgeState::new(limits, isolation)?,
            backend: BackendDispatch::Async {
                backend,
                local_maintenance: None,
            },
        })
    }

    /// Return configured edge bounds.
    pub fn limits(&self) -> ClientSurfaceLimits {
        self.edge.limits()
    }

    /// Requests admitted to canonical dispatch across all attached adapters.
    pub fn dispatch_attempts(&self) -> u64 {
        self.edge.dispatch_attempts()
    }

    /// Successful state transitions observed at this edge.
    pub fn state_mutations(&self) -> u64 {
        self.edge.state_mutations()
    }

    /// Active subscriptions owned by this dispatcher.
    pub fn active_subscriptions(&self) -> u64 {
        self.edge.active_subscriptions()
    }

    /// Subscribe to this dispatcher's bounded tenant-scoped event channel.
    pub fn subscribe_mutations(
        &self,
        identity: &ClientIdentity,
    ) -> Result<ClientSurfaceMutationSubscriber, ClientSurfaceError> {
        self.edge.subscribe_mutations(identity)
    }
    /// Return the selected local candidate, or `None` for an external backend.
    pub fn local_dispatch_variant(&self) -> Option<LocalDispatchVariant> {
        match &self.backend {
            BackendDispatch::Local(_) => Some(LocalDispatchVariant::Static),
            BackendDispatch::Async {
                local_maintenance: Some(_),
                ..
            } => Some(LocalDispatchVariant::BoxedFuture),
            BackendDispatch::Async { .. } => None,
        }
    }

    /// Dispatch after the adapter has verified its transport identity.
    pub async fn dispatch_verified_request(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> ClientResponseEnvelope {
        self.dispatch_verified_request_with_deadline(identity, envelope, None)
            .await
    }

    /// Preserve the transport's monotonic deadline for an asynchronous backend.
    pub async fn dispatch_verified_request_with_deadline(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
        deadline: Option<tokio::time::Instant>,
    ) -> ClientResponseEnvelope {
        if !self.admit(identity, &envelope) {
            return unauthorized_response(envelope);
        }
        self.dispatch_admitted_request(identity, envelope, deadline).await
    }

    /// Local-only synchronous compatibility entry point; asynchronous modes refuse before effects.
    pub fn dispatch_local_request(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> ClientResponseEnvelope {
        let BackendDispatch::Local(local) = &self.backend else {
            return ClientResponseEnvelope::error(
                envelope.request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::BackendUnavailable,
                    false,
                    "synchronous local dispatch is unavailable for an asynchronous backend",
                ),
            )
            .with_protocol_version(response_version(envelope.protocol_version));
        };
        if !self.admit(identity, &envelope) {
            return unauthorized_response(envelope);
        }
        self.dispatch_static_admitted(local, identity, envelope)
    }

    fn admit(&self, identity: &ClientIdentity, envelope: &ClientRequestEnvelope) -> bool {
        let profiling = self.edge.profile_instrumentation.enabled();
        let started = profiling.then(Instant::now);
        let validation = self.edge.validate_tenant_identity(
            identity,
            CLIENT_DATA_PATH,
            Some(envelope.request_id.as_str()),
        );
        if let Some(started) = started {
            self.edge
                .profile_instrumentation
                .identity_validations
                .fetch_add(1, Ordering::Relaxed);
            self.edge
                .profile_instrumentation
                .identity_validation_nanoseconds
                .fetch_add(elapsed_nanoseconds(started), Ordering::Relaxed);
        }
        if validation.is_err() {
            return false;
        }
        self.edge.record_dispatch();
        true
    }

    fn validate_request(
        &self,
        envelope: &ClientRequestEnvelope,
        profiling: bool,
    ) -> Result<(), ClientErrorEnvelope> {
        let started = profiling.then(Instant::now);
        let validation = envelope.validate_protocol();
        if let Some(started) = started {
            self.edge
                .profile_instrumentation
                .protocol_validations
                .fetch_add(1, Ordering::Relaxed);
            self.edge
                .profile_instrumentation
                .protocol_validation_nanoseconds
                .fetch_add(elapsed_nanoseconds(started), Ordering::Relaxed);
        }
        validation?;
        if envelope.deadline_expired(0) {
            return Err(ClientErrorEnvelope::new(
                ClientErrorCode::DeadlineExceeded,
                true,
                "request deadline expired",
            )
            .with_retry_after_ms(1));
        }
        Ok(())
    }

    fn dispatch_static_admitted(
        &self,
        local: &LocalClientDataBackend,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> ClientResponseEnvelope {
        let profiling = self.edge.profile_instrumentation.enabled();
        if let Err(error) = self.validate_request(&envelope, profiling) {
            return rejected_response(envelope, error);
        }
        let version = envelope.protocol_version;
        // Unisolated reads and ordinary writes have no quota, rate or authorization
        // rejection to audit. The write exception is a retained idempotency outcome,
        // whose capacity can be exhausted even when tenant isolation is disabled.
        let needs_audit = self.edge.isolation.is_some()
            || !matches!(
                &envelope.request,
                ClientRequest::Get { .. } | ClientRequest::Put { .. }
            )
            || (matches!(&envelope.request, ClientRequest::Put { .. })
                && envelope.idempotency_key.is_some());
        let audit = needs_audit.then(|| {
            let namespace = request_namespace(&envelope.request).as_str();
            self.record_audit_context_copy(profiling, namespace.len());
            (
                namespace.to_owned(),
                !matches!(&envelope.request, ClientRequest::ForceUnlock { .. }),
            )
        });
        let response = local.execute_inline(VerifiedClientRequest {
            edge: &self.edge,
            identity,
            envelope,
            profiling,
            transport_deadline: None,
        });
        if let Some((namespace, audit_auth_failure)) = audit {
            self.edge.record_rejection_audit(
                identity,
                &namespace,
                &response.request_id,
                &response,
                audit_auth_failure,
            );
        }
        response.with_protocol_version(version)
    }

    async fn dispatch_admitted_request(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
        deadline: Option<tokio::time::Instant>,
    ) -> ClientResponseEnvelope {
        match &self.backend {
            BackendDispatch::Local(local) => {
                self.dispatch_static_admitted(local, identity, envelope)
            }
            BackendDispatch::Async {
                backend,
                local_maintenance,
            } => {
                self.dispatch_async_admitted(
                    backend.as_ref(),
                    local_maintenance.as_deref(),
                    identity,
                    envelope,
                    deadline,
                )
                .await
            }
        }
    }

    async fn dispatch_async_admitted(
        &self,
        backend: &dyn ClientDataBackend,
        local_maintenance: Option<&LocalClientDataBackend>,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
        deadline: Option<tokio::time::Instant>,
    ) -> ClientResponseEnvelope {
        let profiling = self.edge.profile_instrumentation.enabled();
        if let Err(error) = self.validate_request(&envelope, profiling) {
            return rejected_response(envelope, error);
        }
        let namespace = request_namespace(&envelope.request).as_str();
        self.record_audit_context_copy(
            profiling,
            namespace.len().saturating_add(envelope.request_id.len()),
        );
        let context = ResponseContext {
            request_id: envelope.request_id.clone(),
            namespace: namespace.to_owned(),
            audit_auth_failure: !matches!(&envelope.request, ClientRequest::ForceUnlock { .. }),
            protocol_version: envelope.protocol_version,
        };
        let outcome = if is_control_request(&envelope.request) {
            if let Some(local) = local_maintenance {
                let execution = LocalBackendExecution::new(local, &self.edge);
                execution.sweep_expired_entries(execution.now_ms(), false);
            }
            self.edge.handle_control_request(identity, envelope).result
        } else {
            backend
                .execute(VerifiedClientRequest {
                    edge: &self.edge,
                    identity,
                    envelope,
                    profiling,
                    transport_deadline: deadline,
                })
                .await
        };
        self.finish(identity, context, outcome)
    }

    fn record_audit_context_copy(&self, profiling: bool, bytes: usize) {
        if profiling {
            self.edge
                .profile_instrumentation
                .audit_context_bytes_cloned
                .fetch_add(bytes as u64, Ordering::Relaxed);
        }
    }

    fn finish(
        &self,
        identity: &ClientIdentity,
        context: ResponseContext,
        outcome: ClientBackendOutcome,
    ) -> ClientResponseEnvelope {
        let response = match outcome {
            Ok(value) => ClientResponseEnvelope::ok(context.request_id, value),
            Err(error) => ClientResponseEnvelope::error(context.request_id, error),
        };
        self.edge.record_rejection_audit(
            identity,
            &context.namespace,
            &response.request_id,
            &response,
            context.audit_auth_failure,
        );
        response.with_protocol_version(context.protocol_version)
    }

    pub(super) async fn handle_wire_message(
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
            ClientWireMessage::Request(envelope) => Ok(ClientWireMessage::Response(
                self.dispatch_admitted_request(identity, envelope, None).await,
            )),
            ClientWireMessage::Response(_)
            | ClientWireMessage::Invalidation(_)
            | ClientWireMessage::Heartbeat(_) => Err(ClientSurfaceError::MalformedFrame(
                "client data route accepts handshake or request frames".to_owned(),
            )),
        }
    }

    /// Bind all HC/1 routes to this same edge and selected data backend.
    pub fn routes(self: &Arc<Self>) -> Router {
        Router::new()
            .route(CLIENT_DATA_PATH, post(client_data))
            .route(CLIENT_STATUS_PATH, get(client_status))
            .route(CLIENT_SUBSCRIPTIONS_PATH, post(client_subscription))
            .with_state(Arc::clone(self))
    }
}

fn unauthorized_response(envelope: ClientRequestEnvelope) -> ClientResponseEnvelope {
    rejected_response(
        envelope,
        ClientErrorEnvelope::new(
            ClientErrorCode::Unauthorized,
            false,
            "client identity is not authorized for this tenant",
        ),
    )
}

fn rejected_response(
    envelope: ClientRequestEnvelope,
    error: ClientErrorEnvelope,
) -> ClientResponseEnvelope {
    ClientResponseEnvelope::error(envelope.request_id, error)
        .with_protocol_version(response_version(envelope.protocol_version))
}

fn response_version(version: u16) -> u16 {
    if protocol_version_supported(version) {
        version
    } else {
        PROTOCOL_VERSION
    }
}

fn is_control_request(request: &ClientRequest) -> bool {
    matches!(
        request,
        ClientRequest::SubscribeInvalidations { .. }
            | ClientRequest::SubscribeEntryEvents { .. }
            | ClientRequest::TryLock { .. }
            | ClientRequest::Unlock { .. }
            | ClientRequest::RenewLockLease { .. }
            | ClientRequest::ForceUnlock { .. }
            | ClientRequest::GetLockOwnership { .. }
    )
}
