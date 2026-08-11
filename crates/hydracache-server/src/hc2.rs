//! Production HC/2 listener backed by the existing verified client dispatch.

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use futures_util::Stream;
use hydracache_client_hc2::wire::client_envelope;
use hydracache_client_hc2::wire::client_plane_alpha_server::{
    ClientPlaneAlpha, ClientPlaneAlphaServer,
};
use hydracache_client_hc2::wire::invocation_request;
use hydracache_client_hc2::wire::invocation_response;
use hydracache_client_hc2::wire::server_envelope;
use hydracache_client_hc2::wire::{
    BatchResult, CacheEvent, ClientEnvelope, EventGap, HandshakeAck, InvocationRequest,
    InvocationResponse, MutationResult, ResponseMeta, ServerEnvelope, SessionHeartbeat,
    SessionLost, StableErrorCode, SubscriptionAck, ValueResult,
};
use hydracache_client_hc2::HC2_GENERATION;
use hydracache_client_protocol::{
    CasExpectation, ClientErrorCode, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, LockConsistency, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceState};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::transport::server::{TcpConnectInfo, TlsConnectInfo};
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status, Streaming};

use crate::config::TlsConfig;

/// Opaque, eagerly validated TLS policy for the production HC/2 listener.
#[derive(Clone)]
pub struct Hc2ListenerTls(ServerTlsConfig);

impl std::fmt::Debug for Hc2ListenerTls {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Hc2ListenerTls")
            .finish_non_exhaustive()
    }
}

impl Hc2ListenerTls {
    /// Read and validate mandatory server identity and client trust roots.
    pub fn from_server_config(tls: &TlsConfig) -> Result<Self, Hc2ServeError> {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let cert = std::fs::read(tls.cert_path.as_deref().ok_or(Hc2ServeError::MissingTls)?)?;
        let key = std::fs::read(tls.key_path.as_deref().ok_or(Hc2ServeError::MissingTls)?)?;
        let ca = std::fs::read(tls.ca_path.as_deref().ok_or(Hc2ServeError::MissingTls)?)?;
        let config = ServerTlsConfig::new()
            .identity(Identity::from_pem(cert, key))
            .client_ca_root(Certificate::from_pem(ca));
        // Tonic parses the PEM material here. Do this before any listener task
        // starts so malformed identity/trust material cannot follow readiness.
        let _ = Server::builder().tls_config(config.clone())?;
        Ok(Self(config))
    }
}

/// Privacy-safe HC/2 listener accounting used by readiness/drain proofs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hc2AccountingSnapshot {
    /// Currently open authenticated gRPC streams.
    pub active_connections: u64,
    /// Active subscriptions across all streams.
    pub active_subscriptions: u64,
    /// Active fenced sessions across all streams.
    pub active_sessions: u64,
    /// Invocations currently executing through shared dispatch.
    pub pending_invocations: u64,
    /// Frames rejected before dispatch.
    pub rejected_frames: u64,
}

#[derive(Debug, Default)]
struct Accounting {
    active_connections: AtomicU64,
    active_subscriptions: AtomicU64,
    active_sessions: AtomicU64,
    pending_invocations: AtomicU64,
    rejected_frames: AtomicU64,
    next_session: AtomicU64,
    next_watermark: AtomicU64,
}

/// Production gRPC service. Construction requires existing verified dispatch state.
#[derive(Debug, Clone)]
pub struct Hc2ClientPlaneService {
    state: Arc<ClientSurfaceState>,
    cluster_id: Arc<str>,
    accounting: Arc<Accounting>,
}

impl Hc2ClientPlaneService {
    /// Construct a service over the shared HC/1/RESP dispatch state.
    pub fn new(state: Arc<ClientSurfaceState>, cluster_id: impl Into<String>) -> Self {
        Self {
            state,
            cluster_id: Arc::from(cluster_id.into()),
            accounting: Arc::new(Accounting::default()),
        }
    }

    /// Return bounded, secret-free listener accounting.
    pub fn accounting(&self) -> Hc2AccountingSnapshot {
        Hc2AccountingSnapshot {
            active_connections: self.accounting.active_connections.load(Ordering::Acquire),
            active_subscriptions: self.accounting.active_subscriptions.load(Ordering::Acquire),
            active_sessions: self.accounting.active_sessions.load(Ordering::Acquire),
            pending_invocations: self.accounting.pending_invocations.load(Ordering::Acquire),
            rejected_frames: self.accounting.rejected_frames.load(Ordering::Acquire),
        }
    }
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<ServerEnvelope, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl ClientPlaneAlpha for Hc2ClientPlaneService {
    type OpenStream = ResponseStream;

    async fn open(
        &self,
        request: Request<Streaming<ClientEnvelope>>,
    ) -> Result<Response<Self::OpenStream>, Status> {
        let peer_id = verified_peer_id(&request)?;
        let mut inbound = request.into_inner();
        let (outbound, receiver) = mpsc::channel(self.state.limits().max_streams_per_connection);
        let service = self.clone();
        tokio::spawn(async move {
            let guard = ConnectionGuard::new(Arc::clone(&service.accounting));
            if let Err(status) = serve_connection(&service, &peer_id, &mut inbound, &outbound).await
            {
                let _ = outbound.send(Err(status)).await;
            }
            drop(guard);
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(receiver))))
    }
}

struct ConnectionGuard {
    accounting: Arc<Accounting>,
}

struct StreamResourceGuard {
    accounting: Arc<Accounting>,
    subscriptions: u64,
    sessions: u64,
}

impl StreamResourceGuard {
    fn new(accounting: Arc<Accounting>) -> Self {
        Self {
            accounting,
            subscriptions: 0,
            sessions: 0,
        }
    }

    fn add_subscription(&mut self) {
        self.subscriptions += 1;
        self.accounting
            .active_subscriptions
            .fetch_add(1, Ordering::AcqRel);
    }

    fn remove_subscription(&mut self) {
        self.subscriptions = self.subscriptions.saturating_sub(1);
        self.accounting
            .active_subscriptions
            .fetch_sub(1, Ordering::AcqRel);
    }

    fn add_session(&mut self) {
        self.sessions += 1;
        self.accounting
            .active_sessions
            .fetch_add(1, Ordering::AcqRel);
    }

    fn remove_session(&mut self) {
        self.sessions = self.sessions.saturating_sub(1);
        self.accounting
            .active_sessions
            .fetch_sub(1, Ordering::AcqRel);
    }
}

impl Drop for StreamResourceGuard {
    fn drop(&mut self) {
        self.accounting
            .active_subscriptions
            .fetch_sub(self.subscriptions, Ordering::AcqRel);
        self.accounting
            .active_sessions
            .fetch_sub(self.sessions, Ordering::AcqRel);
    }
}

impl ConnectionGuard {
    fn new(accounting: Arc<Accounting>) -> Self {
        accounting.active_connections.fetch_add(1, Ordering::AcqRel);
        Self { accounting }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.accounting
            .active_connections
            .fetch_sub(1, Ordering::AcqRel);
    }
}

async fn serve_connection(
    service: &Hc2ClientPlaneService,
    peer_id: &str,
    inbound: &mut Streaming<ClientEnvelope>,
    outbound: &mpsc::Sender<Result<ServerEnvelope, Status>>,
) -> Result<(), Status> {
    let first = inbound
        .message()
        .await?
        .ok_or_else(|| Status::unauthenticated("HC/2 handshake is required"))?;
    let Some(client_envelope::Message::Handshake(handshake)) = first.message else {
        reject(&service.accounting);
        return Err(Status::failed_precondition("HC/2 handshake must be first"));
    };
    if first.generation != HC2_GENERATION
        || handshake.generation != HC2_GENERATION
        || first.connection_generation == 0
        || first.connection_generation != handshake.connection_generation
    {
        reject(&service.accounting);
        return Err(Status::failed_precondition("unsupported HC/2 generation"));
    }
    let generation = first.connection_generation;
    outbound
        .send(Ok(server_envelope(
            generation,
            first.correlation_id,
            server_envelope::Message::Handshake(HandshakeAck {
                generation: HC2_GENERATION,
                cluster_id: service.cluster_id.to_string(),
                accepted: handshake.requested,
                topology_epoch: 1,
                connection_generation: generation,
            }),
        )))
        .await
        .map_err(|_| Status::cancelled("HC/2 response stream closed"))?;

    let mut subscriptions = BTreeMap::<u64, (Bytes, u64)>::new();
    let mut sessions = BTreeMap::<Bytes, u64>::new();
    let mut resources = StreamResourceGuard::new(Arc::clone(&service.accounting));
    while let Some(envelope) = inbound.message().await? {
        if envelope.generation != HC2_GENERATION
            || envelope.connection_generation != generation
            || envelope.correlation_id == 0
        {
            reject(&service.accounting);
            return Err(Status::failed_precondition(
                "invalid HC/2 envelope identity",
            ));
        }
        match envelope.message {
            Some(client_envelope::Message::Invocation(invocation)) => {
                let event = mutation_event(&invocation);
                let response =
                    dispatch_invocation(service, peer_id, envelope.correlation_id, invocation);
                let mutation_applied = matches!(
                    response.result.as_ref(),
                    Some(invocation_response::Result::Mutation(MutationResult {
                        applied: true
                    }))
                );
                outbound
                    .send(Ok(server_envelope(
                        generation,
                        envelope.correlation_id,
                        server_envelope::Message::Invocation(response),
                    )))
                    .await
                    .map_err(|_| Status::cancelled("HC/2 response stream closed"))?;
                if let Some((key, value, removed)) = event.filter(|_| mutation_applied) {
                    emit_matching_events(
                        service,
                        generation,
                        &key,
                        &value,
                        removed,
                        &mut subscriptions,
                        outbound,
                    )
                    .await?;
                }
            }
            Some(client_envelope::Message::Subscribe(subscribe)) => {
                if subscriptions.contains_key(&subscribe.subscription_id)
                    || subscriptions.len() >= service.state.limits().max_streams_per_connection
                {
                    reject(&service.accounting);
                    return Err(Status::resource_exhausted(
                        "HC/2 subscription bound exceeded",
                    ));
                }
                subscriptions.insert(
                    subscribe.subscription_id,
                    (subscribe.key_prefix, subscribe.resume_watermark),
                );
                resources.add_subscription();
                outbound
                    .send(Ok(server_envelope(
                        generation,
                        envelope.correlation_id,
                        server_envelope::Message::Subscribed(SubscriptionAck {
                            subscription_id: subscribe.subscription_id,
                            watermark: subscribe.resume_watermark,
                        }),
                    )))
                    .await
                    .map_err(|_| Status::cancelled("HC/2 response stream closed"))?;
            }
            Some(client_envelope::Message::Unsubscribe(unsubscribe)) => {
                if subscriptions.remove(&unsubscribe.subscription_id).is_some() {
                    resources.remove_subscription();
                }
            }
            Some(client_envelope::Message::SessionOpen(_)) => {
                let sequence = service
                    .accounting
                    .next_session
                    .fetch_add(1, Ordering::AcqRel)
                    + 1;
                let session_id = Bytes::copy_from_slice(&sequence.to_be_bytes());
                sessions.insert(session_id.clone(), 1);
                resources.add_session();
                outbound
                    .send(Ok(server_envelope(
                        generation,
                        envelope.correlation_id,
                        server_envelope::Message::SessionHeartbeat(SessionHeartbeat {
                            session_id,
                            fence: 1,
                        }),
                    )))
                    .await
                    .map_err(|_| Status::cancelled("HC/2 response stream closed"))?;
            }
            Some(client_envelope::Message::SessionHeartbeat(heartbeat)) => {
                let message = if sessions.get(&heartbeat.session_id) == Some(&heartbeat.fence) {
                    server_envelope::Message::SessionHeartbeat(heartbeat)
                } else {
                    server_envelope::Message::SessionLost(SessionLost {
                        session_id: heartbeat.session_id,
                        last_fence: heartbeat.fence,
                    })
                };
                outbound
                    .send(Ok(server_envelope(
                        generation,
                        envelope.correlation_id,
                        message,
                    )))
                    .await
                    .map_err(|_| Status::cancelled("HC/2 response stream closed"))?;
            }
            Some(client_envelope::Message::SessionClose(close)) => {
                if sessions.remove(&close.session_id).is_some() {
                    resources.remove_session();
                }
            }
            Some(client_envelope::Message::Cancel(_)) => {}
            Some(client_envelope::Message::Handshake(_)) | None => {
                reject(&service.accounting);
                return Err(Status::failed_precondition("unexpected HC/2 frame"));
            }
        }
    }
    Ok(())
}

fn dispatch_invocation(
    service: &Hc2ClientPlaneService,
    peer_id: &str,
    correlation_id: u64,
    invocation: InvocationRequest,
) -> InvocationResponse {
    service
        .accounting
        .pending_invocations
        .fetch_add(1, Ordering::AcqRel);
    let response = dispatch_invocation_inner(service, peer_id, correlation_id, invocation);
    service
        .accounting
        .pending_invocations
        .fetch_sub(1, Ordering::AcqRel);
    response
}

fn dispatch_invocation_inner(
    service: &Hc2ClientPlaneService,
    peer_id: &str,
    correlation_id: u64,
    invocation: InvocationRequest,
) -> InvocationResponse {
    let Some(meta) = invocation.meta else {
        return wire_error(
            StableErrorCode::StableErrorInvalidRequest,
            "request metadata is required",
        );
    };
    if meta.tenant.trim().is_empty() {
        return wire_error(
            StableErrorCode::StableErrorUnauthenticated,
            "tenant is required",
        );
    }
    if meta.deadline_unix_ms != 0 && meta.deadline_unix_ms <= unix_time_ms() {
        return wire_error(
            StableErrorCode::StableErrorDeadlineExceeded,
            "deadline expired",
        );
    }
    let identity = match ClientIdentity::new(peer_id, meta.tenant.clone()) {
        Ok(identity) => identity,
        Err(_) => {
            return wire_error(
                StableErrorCode::StableErrorUnauthenticated,
                "invalid identity",
            )
        }
    };
    let Some(operation) = invocation.operation else {
        return wire_error(
            StableErrorCode::StableErrorInvalidRequest,
            "operation is required",
        );
    };
    if let invocation_request::Operation::Batch(batch) = operation {
        let items = batch
            .items
            .into_iter()
            .map(|item| {
                let nested = InvocationRequest {
                    meta: Some(meta.clone()),
                    operation: item.operation.map(batch_operation),
                };
                dispatch_invocation_inner(service, peer_id, correlation_id, nested)
            })
            .collect();
        return wire_success(Some(invocation_response::Result::Batch(BatchResult {
            items,
        })));
    }
    let request = match to_client_request(operation) {
        Ok(request) => request,
        Err(detail) => return wire_error(StableErrorCode::StableErrorInvalidRequest, detail),
    };
    let mut envelope = ClientRequestEnvelope::new(correlation_id.to_string(), request);
    if meta.deadline_unix_ms != 0 {
        envelope = envelope.with_deadline_ms(meta.deadline_unix_ms);
    }
    if !meta.idempotency_key.is_empty() {
        envelope = envelope.with_idempotency_key(hex(&meta.idempotency_key));
    }
    from_dispatch_response(service.state.dispatch_verified_request(&identity, envelope))
}

fn batch_operation(
    operation: hydracache_client_hc2::wire::batch_item::Operation,
) -> invocation_request::Operation {
    match operation {
        hydracache_client_hc2::wire::batch_item::Operation::Get(value) => {
            invocation_request::Operation::Get(value)
        }
        hydracache_client_hc2::wire::batch_item::Operation::Put(value) => {
            invocation_request::Operation::Put(value)
        }
        hydracache_client_hc2::wire::batch_item::Operation::Delete(value) => {
            invocation_request::Operation::Delete(value)
        }
        hydracache_client_hc2::wire::batch_item::Operation::CompareAndSet(value) => {
            invocation_request::Operation::CompareAndSet(value)
        }
    }
}

fn to_client_request(
    operation: invocation_request::Operation,
) -> Result<ClientRequest, &'static str> {
    let ns = Namespace::new("hc2").map_err(|_| "invalid namespace")?;
    Ok(match operation {
        invocation_request::Operation::Get(value) => ClientRequest::Get {
            ns,
            key: structured_key(&value.key)?,
        },
        invocation_request::Operation::Put(value) => ClientRequest::Put {
            ns,
            key: structured_key(&value.key)?,
            value: value.value.to_vec(),
            ttl_ms: (value.ttl_ms != 0).then_some(value.ttl_ms),
            dimensions: Vec::new(),
        },
        invocation_request::Operation::Delete(value) => ClientRequest::Invalidate {
            ns,
            key: structured_key(&value.key)?,
        },
        invocation_request::Operation::CompareAndSet(value) => ClientRequest::CompareAndSet {
            ns,
            key: structured_key(&value.key)?,
            expected: CasExpectation::Exact(value.expected.to_vec()),
            new_value: value.replacement.to_vec(),
            level: LockConsistency::Quorum,
        },
        invocation_request::Operation::Batch(_) => return Err("nested batch is not allowed"),
    })
}

fn from_dispatch_response(response: ClientResponseEnvelope) -> InvocationResponse {
    match response.result {
        Ok(ClientResponse::Value { value }) => {
            wire_success(Some(invocation_response::Result::Value(ValueResult {
                found: value.is_some(),
                value: value.unwrap_or_default().into(),
                expires_at_unix_ms: 0,
            })))
        }
        Ok(ClientResponse::Stored | ClientResponse::Invalidated) => wire_success(Some(
            invocation_response::Result::Mutation(MutationResult { applied: true }),
        )),
        Ok(ClientResponse::CasApplied { .. }) => wire_success(Some(
            invocation_response::Result::Mutation(MutationResult { applied: true }),
        )),
        Ok(ClientResponse::CasMismatch { .. }) => wire_success(Some(
            invocation_response::Result::Mutation(MutationResult { applied: false }),
        )),
        Ok(_) => wire_error(
            StableErrorCode::StableErrorUnsupported,
            "unsupported HC/2 result",
        ),
        Err(error) => wire_error(map_client_error(error.code), "request rejected"),
    }
}

fn map_client_error(code: ClientErrorCode) -> StableErrorCode {
    match code {
        ClientErrorCode::Unauthenticated => StableErrorCode::StableErrorUnauthenticated,
        ClientErrorCode::Unauthorized | ClientErrorCode::ResidencyDenied => {
            StableErrorCode::StableErrorUnauthorized
        }
        ClientErrorCode::TenantQuota | ClientErrorCode::RateLimited | ClientErrorCode::TooLarge => {
            StableErrorCode::StableErrorQuotaExceeded
        }
        ClientErrorCode::DeadlineExceeded => StableErrorCode::StableErrorDeadlineExceeded,
        ClientErrorCode::Conflict => StableErrorCode::StableErrorConflict,
        ClientErrorCode::BackendUnavailable => StableErrorCode::StableErrorUnavailable,
        ClientErrorCode::IncompatibleVersion | ClientErrorCode::MalformedFrame => {
            StableErrorCode::StableErrorInvalidRequest
        }
    }
}

fn wire_success(result: Option<invocation_response::Result>) -> InvocationResponse {
    InvocationResponse {
        meta: Some(ResponseMeta {
            error: StableErrorCode::StableErrorUnspecified as i32,
            retry: hydracache_client_hc2::wire::RetryDirective::Never as i32,
            safe_detail: String::new(),
            topology_epoch: 1,
        }),
        result,
    }
}

fn wire_error(code: StableErrorCode, detail: &'static str) -> InvocationResponse {
    InvocationResponse {
        meta: Some(ResponseMeta {
            error: code as i32,
            retry: hydracache_client_hc2::wire::RetryDirective::Never as i32,
            safe_detail: detail.to_owned(),
            topology_epoch: 1,
        }),
        result: None,
    }
}

async fn emit_matching_events(
    service: &Hc2ClientPlaneService,
    generation: u64,
    key: &[u8],
    value: &[u8],
    removed: bool,
    subscriptions: &mut BTreeMap<u64, (Bytes, u64)>,
    outbound: &mpsc::Sender<Result<ServerEnvelope, Status>>,
) -> Result<(), Status> {
    for (subscription_id, (prefix, watermark)) in subscriptions.iter_mut() {
        if key.starts_with(prefix) {
            let next = service
                .accounting
                .next_watermark
                .fetch_add(1, Ordering::AcqRel)
                + 1;
            if *watermark != 0 && next > watermark.saturating_add(1) {
                outbound
                    .send(Ok(server_envelope(
                        generation,
                        0,
                        server_envelope::Message::Gap(EventGap {
                            subscription_id: *subscription_id,
                            after_watermark: *watermark,
                        }),
                    )))
                    .await
                    .map_err(|_| Status::cancelled("HC/2 response stream closed"))?;
            }
            *watermark = next;
            outbound
                .send(Ok(server_envelope(
                    generation,
                    0,
                    server_envelope::Message::Event(CacheEvent {
                        subscription_id: *subscription_id,
                        watermark: next,
                        key: Bytes::copy_from_slice(key),
                        value: Bytes::copy_from_slice(value),
                        removed,
                    }),
                )))
                .await
                .map_err(|_| Status::cancelled("HC/2 response stream closed"))?;
        }
    }
    Ok(())
}

fn mutation_event(invocation: &InvocationRequest) -> Option<(Bytes, Bytes, bool)> {
    match invocation.operation.as_ref()? {
        invocation_request::Operation::Put(value) => {
            Some((value.key.clone(), value.value.clone(), false))
        }
        invocation_request::Operation::Delete(value) => {
            Some((value.key.clone(), Bytes::new(), true))
        }
        invocation_request::Operation::CompareAndSet(value) => {
            Some((value.key.clone(), value.replacement.clone(), false))
        }
        _ => None,
    }
}

fn structured_key(value: &[u8]) -> Result<StructuredKey, &'static str> {
    if value.is_empty() {
        return Err("key is required");
    }
    StructuredKey::new(vec![hex(value)]).map_err(|_| "invalid key")
}

fn server_envelope(
    connection_generation: u64,
    correlation_id: u64,
    message: server_envelope::Message,
) -> ServerEnvelope {
    ServerEnvelope {
        generation: HC2_GENERATION,
        connection_generation,
        correlation_id,
        message: Some(message),
    }
}

fn reject(accounting: &Accounting) {
    accounting.rejected_frames.fetch_add(1, Ordering::Relaxed);
}

fn verified_peer_id(request: &Request<Streaming<ClientEnvelope>>) -> Result<String, Status> {
    let connect = request
        .extensions()
        .get::<TlsConnectInfo<TcpConnectInfo>>()
        .ok_or_else(|| Status::unauthenticated("verified mTLS peer is required"))?;
    let certificates = connect
        .peer_certs()
        .ok_or_else(|| Status::unauthenticated("client certificate is required"))?;
    let certificate = certificates
        .first()
        .ok_or_else(|| Status::unauthenticated("client certificate is required"))?;
    Ok(format!(
        "mtls-sha256:{}",
        hex(&Sha256::digest(certificate.as_ref()))
    ))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// Serve the already-bound production listener with mandatory client certificates.
pub async fn serve_hc2_listener(
    listener: TcpListener,
    service: Hc2ClientPlaneService,
    tls: Hc2ListenerTls,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), Hc2ServeError> {
    Server::builder()
        .tls_config(tls.0)?
        .add_service(
            ClientPlaneAlphaServer::new(service)
                .max_decoding_message_size(8 * 1024 * 1024)
                .max_encoding_message_size(8 * 1024 * 1024),
        )
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async move {
            while shutdown.changed().await.is_ok() {
                if *shutdown.borrow() {
                    break;
                }
            }
        })
        .await?;
    Ok(())
}

/// Fail-loud HC/2 listener startup/runtime errors.
#[derive(Debug, Error)]
pub enum Hc2ServeError {
    /// HC/2 was enabled without complete TLS paths.
    #[error("HC/2 requires certificate, key, and client CA paths")]
    MissingTls,
    /// TLS material could not be read.
    #[error("failed to read HC/2 TLS material: {0}")]
    Io(#[from] std::io::Error),
    /// Tonic rejected TLS or serving configuration.
    #[error("HC/2 gRPC serving failed: {0}")]
    Transport(#[from] tonic::transport::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    use hydracache_client_hc2::wire::{GetRequest, PutRequest, RequestMeta};
    use hydracache_client_hc2::{
        ClientConfig, ErrorCode, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client, SubscriptionEvent,
    };
    use rcgen::{
        BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa,
        KeyPair,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn service() -> Hc2ClientPlaneService {
        Hc2ClientPlaneService::new(
            Arc::new(ClientSurfaceState::new(Default::default()).unwrap()),
            "test-cluster",
        )
    }

    fn meta() -> RequestMeta {
        RequestMeta {
            deadline_unix_ms: 0,
            idempotency_key: Bytes::new(),
            tenant: "tenant-a".to_owned(),
            topology_epoch: 1,
        }
    }

    #[test]
    fn operations_use_the_existing_verified_dispatch_state() {
        let service = service();
        let put = dispatch_invocation(
            &service,
            "verified-peer",
            1,
            InvocationRequest {
                meta: Some(meta()),
                operation: Some(invocation_request::Operation::Put(PutRequest {
                    key: Bytes::from_static(b"key"),
                    value: Bytes::from_static(b"value"),
                    ttl_ms: 0,
                })),
            },
        );
        assert!(matches!(
            put.result,
            Some(invocation_response::Result::Mutation(MutationResult {
                applied: true
            }))
        ));
        let get = dispatch_invocation(
            &service,
            "verified-peer",
            2,
            InvocationRequest {
                meta: Some(meta()),
                operation: Some(invocation_request::Operation::Get(GetRequest {
                    key: Bytes::from_static(b"key"),
                })),
            },
        );
        assert!(matches!(
            get.result,
            Some(invocation_response::Result::Value(ValueResult { found: true, ref value, .. }))
                if value.as_ref() == b"value"
        ));
        assert_eq!(service.state.dispatch_attempts(), 2);
        assert_eq!(service.state.state_mutations(), 1);
        assert_eq!(service.accounting(), Hc2AccountingSnapshot::default());
    }

    #[test]
    fn missing_tenant_and_expired_deadline_fail_before_dispatch() {
        let service = service();
        let mut missing = meta();
        missing.tenant.clear();
        let response = dispatch_invocation(
            &service,
            "verified-peer",
            1,
            InvocationRequest {
                meta: Some(missing),
                operation: Some(invocation_request::Operation::Get(GetRequest {
                    key: Bytes::from_static(b"key"),
                })),
            },
        );
        assert_eq!(
            response.meta.unwrap().error,
            StableErrorCode::StableErrorUnauthenticated as i32
        );
        let mut expired = meta();
        expired.deadline_unix_ms = 1;
        let response = dispatch_invocation(
            &service,
            "verified-peer",
            2,
            InvocationRequest {
                meta: Some(expired),
                operation: Some(invocation_request::Operation::Get(GetRequest {
                    key: Bytes::from_static(b"key"),
                })),
            },
        );
        assert_eq!(
            response.meta.unwrap().error,
            StableErrorCode::StableErrorDeadlineExceeded as i32
        );
        assert_eq!(service.state.dispatch_attempts(), 0);
    }

    struct TestPki {
        ca: String,
        server_cert: String,
        server_key: String,
        client_cert: String,
        client_key: String,
    }

    fn test_pki() -> TestPki {
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();

        let server_key = KeyPair::generate().unwrap();
        let mut server_params = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
        server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let server_cert = server_params.signed_by(&server_key, &ca).unwrap();

        let client_key = KeyPair::generate().unwrap();
        let mut client_params = CertificateParams::new(vec!["hc2-client".to_owned()]).unwrap();
        client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        let client_cert = client_params.signed_by(&client_key, &ca).unwrap();
        TestPki {
            ca: ca.pem(),
            server_cert: server_cert.pem(),
            server_key: server_key.serialize_pem(),
            client_cert: client_cert.pem(),
            client_key: client_key.serialize_pem(),
        }
    }

    fn write_tls(root: &PathBuf, pki: &TestPki) -> TlsConfig {
        std::fs::create_dir_all(root).unwrap();
        let cert = root.join("server.pem");
        let key = root.join("server.key");
        let ca = root.join("clients.pem");
        std::fs::write(&cert, &pki.server_cert).unwrap();
        std::fs::write(&key, &pki.server_key).unwrap();
        std::fs::write(&ca, &pki.ca).unwrap();
        TlsConfig {
            enabled: true,
            cert_path: Some(cert),
            key_path: Some(key),
            ca_path: Some(ca),
            acknowledge_insecure: false,
        }
    }

    fn adapter(addr: std::net::SocketAddr, server: &TestPki, client: &TestPki) -> GrpcMtlsAdapter {
        GrpcMtlsAdapter::new(
            GrpcMtlsConfig::new(
                format!("https://{addr}"),
                "localhost",
                server.ca.as_bytes(),
                client.client_cert.as_bytes(),
                client.client_key.as_bytes(),
            )
            .unwrap(),
        )
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_socket_requires_mtls_dispatches_pushes_and_drains_to_zero() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let unique = unix_time_ms();
        let root = PathBuf::from(format!(
            "target/test-hc2-production-{}-{unique}",
            std::process::id()
        ));
        let trusted = test_pki();
        let untrusted = test_pki();
        let tls = Hc2ListenerTls::from_server_config(&write_tls(&root, &trusted)).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let service = service();
        let observed = service.clone();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let serving =
            tokio::spawn(
                async move { serve_hc2_listener(listener, service, tls, shutdown_rx).await },
            );

        let mut plaintext = tokio::net::TcpStream::connect(addr).await.unwrap();
        plaintext
            .write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
            .await
            .unwrap();
        let mut byte = [0_u8; 1];
        let plaintext_result =
            tokio::time::timeout(Duration::from_secs(1), plaintext.read(&mut byte)).await;
        assert!(
            matches!(plaintext_result, Ok(Ok(0)) | Ok(Err(_)))
                || matches!(plaintext_result, Ok(Ok(1))) && byte[0] == 21,
            "plaintext must be rejected before protocol dispatch: {plaintext_result:?}"
        );

        let rejected = Hc2Client::connect(
            &adapter(addr, &trusted, &untrusted),
            ClientConfig::new("untrusted", "tenant-a"),
        )
        .await
        .expect_err("foreign client CA must fail closed");
        assert_eq!(rejected.code(), ErrorCode::Unavailable);

        let client = Hc2Client::connect(
            &adapter(addr, &trusted, &trusted),
            ClientConfig::new("claimed-name-is-not-identity", "tenant-a"),
        )
        .await
        .unwrap();
        assert_eq!(client.cluster_id(), "test-cluster");
        let mut subscription = client
            .subscribe(Bytes::from_static(b"event/"), 0)
            .await
            .unwrap();
        let session = client.open_session(Duration::from_secs(2)).await.unwrap();
        assert_eq!(session.fence().unwrap(), 1);
        client
            .put(
                Bytes::from_static(b"event/key"),
                Bytes::from_static(b"value"),
                None,
                None,
            )
            .await
            .unwrap();
        let value = client
            .get(Bytes::from_static(b"event/key"), None)
            .await
            .unwrap()
            .expect("stored value");
        assert_eq!(value.value, Bytes::from_static(b"value"));
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(1), subscription.next())
                .await
                .unwrap(),
            Some(SubscriptionEvent::Event(_))
        ));
        // Tear down the transport while stream-owned resources are still live.
        // Server accounting must be released by the stream guard rather than
        // depending on cooperative unsubscribe/session-close frames.
        client.close();
        drop(subscription);
        drop(session);

        for _ in 0..50 {
            if observed.accounting() == Hc2AccountingSnapshot::default() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(observed.accounting(), Hc2AccountingSnapshot::default());
        shutdown_tx.send(true).unwrap();
        serving.await.unwrap().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
