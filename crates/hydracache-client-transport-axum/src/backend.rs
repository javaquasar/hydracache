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
        let envelope = match self.admit(identity, envelope) {
            Ok(envelope) => envelope,
            Err(response) => return response,
        };
        self.dispatch_admitted_request(identity, envelope).await
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
        let envelope = match self.admit(identity, envelope) {
            Ok(envelope) => envelope,
            Err(response) => return response,
        };
        let (request, context) = match self.prepare(identity, envelope) {
            Ok(prepared) => prepared,
            Err(response) => return response,
        };
        let outcome = local.execute_inline(request);
        self.finish(identity, context, outcome)
    }

    fn admit(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> Result<ClientRequestEnvelope, ClientResponseEnvelope> {
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
            let version = response_version(envelope.protocol_version);
            return Err(ClientResponseEnvelope::error(
                envelope.request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::Unauthorized,
                    false,
                    "client identity is not authorized for this tenant",
                ),
            )
            .with_protocol_version(version));
        }
        self.edge.record_dispatch();
        Ok(envelope)
    }

    fn prepare<'a>(
        &'a self,
        identity: &'a ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> Result<(VerifiedClientRequest<'a>, ResponseContext), ClientResponseEnvelope> {
        let profiling = self.edge.profile_instrumentation.enabled();
        if profiling {
            self.edge
                .profile_instrumentation
                .audit_context_bytes_cloned
                .fetch_add(
                    envelope
                        .request_id
                        .len()
                        .saturating_add(request_namespace(&envelope.request).as_str().len())
                        as u64,
                    Ordering::Relaxed,
                );
        }
        let context = ResponseContext {
            request_id: envelope.request_id.clone(),
            namespace: request_namespace(&envelope.request).as_str().to_owned(),
            audit_auth_failure: !matches!(&envelope.request, ClientRequest::ForceUnlock { .. }),
            protocol_version: response_version(envelope.protocol_version),
        };
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
        if let Err(error) = validation {
            return Err(ClientResponseEnvelope::error(envelope.request_id, error)
                .with_protocol_version(context.protocol_version));
        }
        if envelope.deadline_expired(0) {
            return Err(ClientResponseEnvelope::error(
                envelope.request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::DeadlineExceeded,
                    true,
                    "request deadline expired",
                )
                .with_retry_after_ms(1),
            )
            .with_protocol_version(context.protocol_version));
        }
        Ok((
            VerifiedClientRequest {
                edge: &self.edge,
                identity,
                envelope,
                profiling,
            },
            context,
        ))
    }

    async fn dispatch_admitted_request(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> ClientResponseEnvelope {
        let (request, context) = match self.prepare(identity, envelope) {
            Ok(prepared) => prepared,
            Err(response) => return response,
        };
        let outcome = if is_control_request(&request.envelope.request) {
            let local = match &self.backend {
                BackendDispatch::Local(local) => Some(local),
                BackendDispatch::Async {
                    local_maintenance, ..
                } => local_maintenance.as_ref(),
            };
            if let Some(local) = local {
                let execution = LocalBackendExecution::new(local, &self.edge);
                execution.sweep_expired_entries(execution.now_ms(), false);
            }
            self.edge
                .handle_control_request(identity, request.envelope)
                .result
        } else {
            match &self.backend {
                BackendDispatch::Local(local) => local.execute_inline(request),
                BackendDispatch::Async { backend, .. } => backend.execute(request).await,
            }
        };
        self.finish(identity, context, outcome)
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
                self.dispatch_admitted_request(identity, envelope).await,
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
