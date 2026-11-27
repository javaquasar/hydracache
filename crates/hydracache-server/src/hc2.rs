//! Production HC/2 listener backed by the existing verified client dispatch.

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use hydracache_client_hc2::wire::client_envelope;
use hydracache_client_hc2::wire::client_plane_alpha_server::{
    ClientPlaneAlpha, ClientPlaneAlphaServer,
};
use hydracache_client_hc2::wire::invocation_request;
use hydracache_client_hc2::wire::invocation_response;
use hydracache_client_hc2::wire::server_envelope;
use hydracache_client_hc2::wire::{
    BatchResult, CacheEvent, Capability, ClientEnvelope, EventGap, HandshakeAck, InvocationRequest,
    InvocationResponse, LockOwnershipResult, LockResult, MutationResult, RequestMeta, ResponseMeta,
    ServerEnvelope, SessionHeartbeat, SessionLost, StableErrorCode, SubscriptionAck, ValueResult,
};
use hydracache_client_hc2::{is_supported_hc2_generation, HC2_GENERATION, HC2_MINIMUM_GENERATION};
use hydracache_client_protocol::{
    CasExpectation, ClientErrorCode, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, LockConsistency, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceDispatcher, ClientSurfaceState,
};
use prost::Message;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch, OwnedSemaphorePermit, Semaphore};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::transport::server::{TcpConnectInfo, TlsConnectInfo};
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status, Streaming};

use crate::cluster_status::GridControlPlaneHandle;
use crate::config::TlsConfig;
use crate::value_plane_subscription::{
    ClusterEvent, ClusterSubscriptionCoordinator, MemberValuePlane, MAX_CLUSTER_SUBSCRIPTIONS,
    MAX_PREFIX_BYTES,
};

#[path = "hc2_bulk.rs"]
mod bulk;

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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Hc2AccountingSnapshot {
    /// Currently open authenticated gRPC streams.
    pub active_connections: u64,
    /// Transport streams accepted since process start.
    pub accepted_connections: u64,
    /// Accepted streams closed since process start.
    pub closed_connections: u64,
    /// Active subscriptions across all streams.
    pub active_subscriptions: u64,
    /// Active fenced sessions across all streams.
    pub active_sessions: u64,
    /// Invocations currently executing through shared dispatch.
    pub pending_invocations: u64,
    /// Frames rejected before dispatch.
    pub rejected_frames: u64,
}

impl Hc2AccountingSnapshot {
    /// Whether every live owner has been released; lifetime totals are intentionally retained.
    pub fn live_resources_zero(self) -> bool {
        self.active_connections == 0
            && self.active_subscriptions == 0
            && self.active_sessions == 0
            && self.pending_invocations == 0
    }
}

#[derive(Debug, Default)]
struct Accounting {
    active_connections: AtomicU64,
    accepted_connections: AtomicU64,
    closed_connections: AtomicU64,
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
    state: Arc<ClientSurfaceDispatcher>,
    cluster_id: Arc<str>,
    accounting: Arc<Accounting>,
    member: Option<DistributedMember>,
}

#[derive(Debug, Clone)]
struct DistributedMember {
    grid: Arc<dyn GridControlPlaneHandle>,
    principal: String,
    plane: MemberValuePlane,
    subscriptions: Arc<Semaphore>,
    bulk: Arc<Semaphore>,
}

impl Hc2ClientPlaneService {
    /// Construct a service over the shared HC/1/RESP dispatch state.
    pub fn new(state: Arc<ClientSurfaceState>, cluster_id: impl Into<String>) -> Self {
        Self::from_dispatcher(state.dispatcher(), cluster_id)
    }

    /// Attach the canonical asynchronous dispatcher without constructing another local map.
    pub fn from_dispatcher(
        state: Arc<ClientSurfaceDispatcher>,
        cluster_id: impl Into<String>,
    ) -> Self {
        Self {
            state,
            cluster_id: Arc::from(cluster_id.into()),
            accounting: Arc::new(Accounting::default()),
            member: None,
        }
    }

    pub(crate) fn distributed(
        dispatcher: Arc<ClientSurfaceDispatcher>,
        cluster_id: impl Into<String>,
        grid: Arc<dyn GridControlPlaneHandle>,
        principal: String,
        plane: MemberValuePlane,
    ) -> Self {
        Self {
            member: Some(DistributedMember {
                grid,
                principal,
                plane,
                subscriptions: Arc::new(Semaphore::new(MAX_CLUSTER_SUBSCRIPTIONS)),
                bulk: Arc::new(Semaphore::new(bulk::MAX_ACTIVE_BATCHES)),
            }),
            ..Self::from_dispatcher(dispatcher, cluster_id)
        }
    }

    fn topology_epoch(&self) -> u64 {
        self.member.as_ref().map_or(1, |member| {
            member
                .grid
                .partition_snapshot()
                .filter(|snapshot| snapshot.applied_index == snapshot.commit_index)
                .map_or(0, |snapshot| snapshot.epoch.value())
        })
    }

    pub(crate) fn operations(&self) -> Option<&crate::imap_ops::IMapOps> {
        self.member.as_ref()?.plane.operations()
    }

    pub(crate) fn begin_member_drain(&self) {
        if let Some(ops) = self.operations() {
            ops.begin_drain();
        }
    }

    pub(crate) fn imap_evidence(&self) -> Option<crate::imap_ops::IMapEvidence> {
        let current = self
            .member
            .as_ref()?
            .grid
            .partition_snapshot()
            .map(|snapshot| {
                (
                    snapshot.epoch,
                    snapshot.applied_index == snapshot.commit_index,
                )
            });
        Some(self.operations()?.evidence(current))
    }

    pub(crate) fn imap_metrics(&self) -> String {
        self.operations()
            .zip(self.imap_evidence())
            .map_or_else(String::new, |(ops, evidence)| ops.prometheus(&evidence))
    }

    fn admit_public(
        &self,
    ) -> Result<Option<tokio::sync::SemaphorePermit<'_>>, (StableErrorCode, &'static str)> {
        use crate::imap_ops::{compatible_roster, Denial};
        let Some(ops) = self.operations() else {
            return Ok(None);
        };
        let permit = ops.enter(false).map_err(|denial| match denial {
            Denial::Draining => (
                StableErrorCode::StableErrorUnavailable,
                "IMap member is draining",
            ),
            Denial::Saturated => (
                StableErrorCode::StableErrorQuotaExceeded,
                "IMap member admission exhausted",
            ),
        })?;
        let member = self
            .member
            .as_ref()
            .expect("value control belongs to a member");
        let Some(snapshot) = member.grid.partition_snapshot() else {
            return Err((
                StableErrorCode::StableErrorUnavailable,
                "IMap topology unavailable",
            ));
        };
        if !compatible_roster(&snapshot) {
            ops.counters.incompatible.fetch_add(1, Ordering::Relaxed);
            return Err((
                StableErrorCode::StableErrorUnavailable,
                "IMap value wire generation is incompatible",
            ));
        }
        if snapshot.applied_index != snapshot.commit_index
            || snapshot.members.len() < ops.config.replication_factor
            || !snapshot.members.iter().any(|record| {
                record.node_id == member.plane.local_member().node
                    && record.generation == member.plane.local_member().generation
            })
        {
            return Err((
                StableErrorCode::StableErrorUnavailable,
                "IMap topology is not applied",
            ));
        }
        Ok(Some(permit))
    }

    /// Return bounded, secret-free listener accounting.
    pub fn accounting(&self) -> Hc2AccountingSnapshot {
        Hc2AccountingSnapshot {
            active_connections: self.accounting.active_connections.load(Ordering::Acquire),
            accepted_connections: self.accounting.accepted_connections.load(Ordering::Acquire),
            closed_connections: self.accounting.closed_connections.load(Ordering::Acquire),
            active_subscriptions: self.accounting.active_subscriptions.load(Ordering::Acquire),
            active_sessions: self.accounting.active_sessions.load(Ordering::Acquire),
            pending_invocations: self.accounting.pending_invocations.load(Ordering::Acquire),
            rejected_frames: self.accounting.rejected_frames.load(Ordering::Acquire),
        }
    }

    /// Render the production listener's bounded aggregate accounting for the
    /// existing internal Prometheus endpoint.
    ///
    /// The only label has a closed domain. Peer identities, tenant IDs,
    /// authorities, certificate material, keys, and values are never exposed.
    pub fn prometheus_metrics(&self) -> String {
        let snapshot = self.accounting();
        format!(
            concat!(
                "# TYPE hydracache_hc2_connections gauge\n",
                "hydracache_hc2_connections{{transport=\"grpc_bidirectional\"}} {}\n",
                "# TYPE hydracache_hc2_pending_invocations gauge\n",
                "hydracache_hc2_pending_invocations{{transport=\"grpc_bidirectional\"}} {}\n",
                "# TYPE hydracache_hc2_subscriptions gauge\n",
                "hydracache_hc2_subscriptions{{transport=\"grpc_bidirectional\"}} {}\n",
                "# TYPE hydracache_hc2_sessions gauge\n",
                "hydracache_hc2_sessions{{transport=\"grpc_bidirectional\"}} {}\n",
                "# TYPE hydracache_hc2_rejected_frames_total counter\n",
                "hydracache_hc2_rejected_frames_total{{transport=\"grpc_bidirectional\"}} {}\n",
            ),
            snapshot.active_connections,
            snapshot.pending_invocations,
            snapshot.active_subscriptions,
            snapshot.active_sessions,
            snapshot.rejected_frames,
        )
    }
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<ServerEnvelope, Status>> + Send + 'static>>;

const HC2_OUTBOUND_QUEUE_BYTE_BUDGET: usize = 1024 * 1024;

struct QueuedResponse {
    frame: Result<ServerEnvelope, Status>,
    _byte_permit: Option<OwnedSemaphorePermit>,
}

type OutboundSender = mpsc::Sender<QueuedResponse>;

#[derive(Clone)]
struct OutboundQueue {
    sender: OutboundSender,
    byte_budget: Arc<Semaphore>,
}

impl OutboundQueue {
    fn new(item_capacity: usize) -> (Self, mpsc::Receiver<QueuedResponse>) {
        let (sender, receiver) = mpsc::channel(item_capacity);
        (
            Self {
                sender,
                byte_budget: Arc::new(Semaphore::new(HC2_OUTBOUND_QUEUE_BYTE_BUDGET)),
            },
            receiver,
        )
    }

    async fn send(&self, frame: Result<ServerEnvelope, Status>) -> Result<(), Status> {
        let byte_permit = match frame.as_ref() {
            Ok(envelope) => {
                let charge = envelope
                    .encoded_len()
                    .clamp(1, HC2_OUTBOUND_QUEUE_BYTE_BUDGET);
                let charge =
                    u32::try_from(charge).expect("HC/2 outbound queue byte budget must fit in u32");
                Some(
                    Arc::clone(&self.byte_budget)
                        .acquire_many_owned(charge)
                        .await
                        .map_err(|_| Status::cancelled("HC/2 response stream closed"))?,
                )
            }
            Err(_) => None,
        };
        self.sender
            .send(QueuedResponse {
                frame,
                _byte_permit: byte_permit,
            })
            .await
            .map_err(|_| Status::cancelled("HC/2 response stream closed"))
    }
}

#[tonic::async_trait]
impl ClientPlaneAlpha for Hc2ClientPlaneService {
    type OpenStream = ResponseStream;

    async fn open(
        &self,
        request: Request<Streaming<ClientEnvelope>>,
    ) -> Result<Response<Self::OpenStream>, Status> {
        let peer_id = verified_peer_id(&request)?;
        if self
            .member
            .as_ref()
            .is_some_and(|member| member.principal != peer_id)
        {
            return Err(Status::permission_denied(
                "HC/2 client certificate is outside the member profile",
            ));
        }
        if self.operations().is_some_and(|ops| ops.is_draining()) {
            return Err(Status::unavailable("IMap member is draining"));
        }
        let mut inbound = request.into_inner();
        let (outbound, receiver) =
            OutboundQueue::new(self.state.limits().max_streams_per_connection);
        let service = self.clone();
        tokio::spawn(async move {
            let guard = ConnectionGuard::new(Arc::clone(&service.accounting));
            if let Err(status) = serve_connection(&service, &peer_id, &mut inbound, &outbound).await
            {
                let _ = outbound.send(Err(status)).await;
            }
            drop(guard);
        });
        Ok(Response::new(Box::pin(
            ReceiverStream::new(receiver).map(|queued| queued.frame),
        )))
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
        accounting
            .accepted_connections
            .fetch_add(1, Ordering::AcqRel);
        Self { accounting }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.accounting
            .active_connections
            .fetch_sub(1, Ordering::AcqRel);
        self.accounting
            .closed_connections
            .fetch_add(1, Ordering::AcqRel);
    }
}

#[derive(Clone, Copy)]
struct SessionIdentity {
    protocol_generation: u32,
    connection_generation: u64,
}

async fn serve_connection(
    service: &Hc2ClientPlaneService,
    peer_id: &str,
    inbound: &mut Streaming<ClientEnvelope>,
    outbound: &OutboundQueue,
) -> Result<(), Status> {
    let mut draining = service.operations().map(|ops| ops.drain_signal());
    let first = tokio::select! {
        biased;
        _ = wait_for_member_drain(&mut draining) => return Ok(()),
        first = inbound.message() => first?.ok_or_else(|| Status::unauthenticated("HC/2 handshake is required"))?,
    };
    let Some(client_envelope::Message::Handshake(handshake)) = first.message else {
        reject(&service.accounting);
        return Err(Status::failed_precondition("HC/2 handshake must be first"));
    };
    if !is_supported_hc2_generation(first.generation)
        || handshake.generation != first.generation
        || first.connection_generation == 0
        || first.connection_generation != handshake.connection_generation
    {
        reject(&service.accounting);
        return Err(Status::failed_precondition("unsupported HC/2 generation"));
    }
    let identity = SessionIdentity {
        protocol_generation: first.generation,
        connection_generation: first.connection_generation,
    };
    outbound
        .send(Ok(server_envelope(
            identity,
            first.correlation_id,
            server_envelope::Message::Handshake(HandshakeAck {
                generation: identity.protocol_generation,
                cluster_id: service.cluster_id.to_string(),
                accepted: handshake
                    .requested
                    .into_iter()
                    .filter(|capability| {
                        service.member.is_none()
                            || matches!(
                                Capability::try_from(*capability),
                                Ok(Capability::Data
                                    | Capability::Batch
                                    | Capability::Subscriptions)
                            )
                    })
                    .collect(),
                topology_epoch: service.topology_epoch(),
                connection_generation: identity.connection_generation,
                minimum_generation: HC2_MINIMUM_GENERATION,
                preferred_generation: HC2_GENERATION,
                negotiated_generation_deprecated: identity.protocol_generation < HC2_GENERATION,
            }),
        )))
        .await?;

    let mut subscriptions = BTreeMap::<u64, (Bytes, u64)>::new();
    let mut clustered = BTreeMap::<u64, ClusterSubscriptionTask>::new();
    let mut sessions = BTreeMap::<Bytes, u64>::new();
    let mut resources = StreamResourceGuard::new(Arc::clone(&service.accounting));
    loop {
        let Some(envelope) = (tokio::select! {
            biased;
            _ = wait_for_member_drain(&mut draining) => return Ok(()),
            next = inbound.message() => next?,
        }) else {
            return Ok(());
        };
        if envelope.generation != identity.protocol_generation
            || envelope.connection_generation != identity.connection_generation
            || envelope.correlation_id == 0
        {
            reject(&service.accounting);
            return Err(Status::failed_precondition(
                "invalid HC/2 envelope identity",
            ));
        }
        match envelope.message {
            Some(client_envelope::Message::Invocation(invocation)) => {
                let event = service
                    .member
                    .is_none()
                    .then(|| mutation_event(&invocation))
                    .flatten();
                let mut response = dispatch_invocation_until_closed(
                    service,
                    peer_id,
                    envelope.correlation_id,
                    invocation,
                    outbound,
                )
                .await?;
                if let Some(meta) = &mut response.meta {
                    meta.topology_epoch = service.topology_epoch();
                }
                let mutation_applied = matches!(
                    response.result.as_ref(),
                    Some(invocation_response::Result::Mutation(MutationResult {
                        applied: true
                    }))
                );
                outbound
                    .send(Ok(server_envelope(
                        identity,
                        envelope.correlation_id,
                        server_envelope::Message::Invocation(response),
                    )))
                    .await?;
                if let Some((key, value, removed)) = event.filter(|_| mutation_applied) {
                    emit_matching_events(
                        service,
                        identity,
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
                if let Some(member) = &service.member {
                    let _admission = service.admit_public().map_err(|(code, detail)| {
                        if code == StableErrorCode::StableErrorQuotaExceeded {
                            Status::resource_exhausted(detail)
                        } else {
                            Status::unavailable(detail)
                        }
                    })?;
                    clustered.retain(|_, task| !task.0.is_finished());
                    if clustered.contains_key(&subscribe.subscription_id)
                        || clustered.len() >= service.state.limits().max_streams_per_connection
                    {
                        reject(&service.accounting);
                        return Err(Status::resource_exhausted(
                            "HC/2 subscription bound exceeded",
                        ));
                    }
                    let id = subscribe.subscription_id;
                    let task = start_cluster_subscription(
                        member,
                        Arc::clone(&service.accounting),
                        identity,
                        envelope.correlation_id,
                        subscribe,
                        outbound,
                    )
                    .await?;
                    clustered.insert(id, task);
                    continue;
                }
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
                        identity,
                        envelope.correlation_id,
                        server_envelope::Message::Subscribed(SubscriptionAck {
                            subscription_id: subscribe.subscription_id,
                            watermark: subscribe.resume_watermark,
                        }),
                    )))
                    .await?;
            }
            Some(client_envelope::Message::Unsubscribe(unsubscribe)) => {
                clustered.remove(&unsubscribe.subscription_id);
                if subscriptions.remove(&unsubscribe.subscription_id).is_some() {
                    resources.remove_subscription();
                }
            }
            Some(client_envelope::Message::SessionOpen(_)) => {
                if service.member.is_some() {
                    return Err(Status::unimplemented(
                        "clustered HC/2 fenced sessions are unavailable",
                    ));
                }
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
                        identity,
                        envelope.correlation_id,
                        server_envelope::Message::SessionHeartbeat(SessionHeartbeat {
                            session_id,
                            fence: 1,
                        }),
                    )))
                    .await?;
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
                        identity,
                        envelope.correlation_id,
                        message,
                    )))
                    .await?;
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
}

async fn wait_for_member_drain(signal: &mut Option<tokio::sync::watch::Receiver<bool>>) {
    match signal {
        Some(signal) => {
            let _ = signal.wait_for(|draining| *draining).await;
        }
        None => std::future::pending().await,
    }
}

struct ClusterSubscriptionTask(tokio::task::JoinHandle<()>);

impl Drop for ClusterSubscriptionTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn start_cluster_subscription(
    member: &DistributedMember,
    accounting: Arc<Accounting>,
    identity: SessionIdentity,
    correlation: u64,
    request: hydracache_client_hc2::wire::Subscribe,
    outbound: &OutboundQueue,
) -> Result<ClusterSubscriptionTask, Status> {
    if request.subscription_id == 0
        || request.resume_watermark == u64::MAX
        || request.key_prefix.len() > MAX_PREFIX_BYTES
    {
        return Err(Status::invalid_argument(
            "invalid HC/2 cluster subscription",
        ));
    }
    let permit = member
        .subscriptions
        .clone()
        .try_acquire_owned()
        .map_err(|_| Status::resource_exhausted("HC/2 member subscription bound exceeded"))?;
    let view = member
        .plane
        .listener_view()
        .ok_or_else(|| Status::unavailable("HC/2 subscription topology unavailable"))?;
    let coordinator = ClusterSubscriptionCoordinator::new(
        member.plane.clone(),
        &view,
        request.key_prefix.to_vec(),
        request.include_value,
    );
    let subscription_id = request.subscription_id;
    let mut watermark = request.resume_watermark;
    outbound
        .send(Ok(server_envelope(
            identity,
            correlation,
            server_envelope::Message::Subscribed(SubscriptionAck {
                subscription_id,
                watermark,
            }),
        )))
        .await?;
    let outbound = outbound.clone();
    let plane = member.plane.clone();
    Ok(ClusterSubscriptionTask(tokio::spawn(async move {
        let _permit = permit;
        let mut resource = StreamResourceGuard::new(accounting);
        resource.add_subscription();
        let mut draining = plane.operations().map(|ops| ops.drain_signal());
        let gap = |after_watermark, details| {
            if let Some(operations) = plane.operations() {
                operations
                    .counters
                    .listener_gaps
                    .fetch_add(1, Ordering::Relaxed);
            }
            server_envelope(
                identity,
                0,
                server_envelope::Message::Gap(EventGap {
                    subscription_id,
                    after_watermark,
                    details: Some(details),
                }),
            )
        };
        if watermark != 0 {
            let details = hydracache_client_hc2::wire::EventGapDetails {
                reason: hydracache_client_hc2::wire::EventGapReason::ResumeUnbound as i32,
                partitions: Vec::new(),
            };
            if outbound.send(Ok(gap(watermark, details))).await.is_err() {
                return;
            }
        }
        let forward = coordinator.run(|event| {
            let frame = match event {
                ClusterEvent::Gap(details) => Some(gap(watermark, details)),
                ClusterEvent::Mutation(event) => watermark.checked_add(1).map(|next| {
                    watermark = next;
                    server_envelope(
                        identity,
                        0,
                        server_envelope::Message::Event(CacheEvent {
                            subscription_id,
                            watermark,
                            key: event.key.into(),
                            value: event.value.unwrap_or_default().into(),
                            removed: event.removed,
                        }),
                    )
                }),
            };
            let outbound = outbound.clone();
            async move {
                outbound
                    .send(Ok(frame.ok_or_else(|| {
                        Status::resource_exhausted("HC/2 watermark exhausted")
                    })?))
                    .await
            }
        });
        tokio::select! {
            _ = wait_for_member_drain(&mut draining) => {}
            _ = outbound.sender.closed() => {}
            _ = forward => {}
        }
    })))
}

struct PendingInvocationGuard<'a>(&'a AtomicU64);

impl<'a> PendingInvocationGuard<'a> {
    fn new(counter: &'a AtomicU64) -> Self {
        counter.fetch_add(1, Ordering::AcqRel);
        Self(counter)
    }
}

impl Drop for PendingInvocationGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

async fn dispatch_invocation_until_closed(
    service: &Hc2ClientPlaneService,
    peer_id: &str,
    correlation_id: u64,
    invocation: InvocationRequest,
    outbound: &OutboundQueue,
) -> Result<InvocationResponse, Status> {
    tokio::select! {
        biased;
        _ = outbound.sender.closed() => Err(Status::cancelled("HC/2 response stream closed")),
        response = dispatch_invocation_async(service, peer_id, correlation_id, invocation) => Ok(response),
    }
}

async fn dispatch_invocation_async(
    service: &Hc2ClientPlaneService,
    peer_id: &str,
    correlation_id: u64,
    invocation: InvocationRequest,
) -> InvocationResponse {
    let _pending = PendingInvocationGuard::new(&service.accounting.pending_invocations);
    let _admission = match service.admit_public() {
        Ok(permit) => permit,
        Err((code, detail)) => return wire_error(code, detail),
    };
    let (meta, identity, operation) = match prepare_invocation(peer_id, invocation) {
        Ok(prepared) => prepared,
        Err(response) => return response,
    };
    if let Some(member) = &service.member {
        if let invocation_request::Operation::Batch(batch) = operation {
            return bulk::dispatch(service, member, correlation_id, meta, identity, batch).await;
        }
        if !admitted_distributed_operation(&operation) {
            return wire_error(
                StableErrorCode::StableErrorUnsupported,
                "operation is outside the distributed HC/2 profile",
            );
        }
    }
    if let invocation_request::Operation::Batch(batch) = operation {
        let mut items = Vec::with_capacity(batch.items.len());
        for item in batch.items {
            let nested = InvocationRequest {
                meta: Some(meta.clone()),
                operation: item.operation.map(batch_operation),
            };
            let response = match prepare_invocation(peer_id, nested) {
                Ok((meta, identity, operation)) => {
                    execute_invocation(service, correlation_id, meta, identity, operation).await
                }
                Err(response) => response,
            };
            items.push(response);
        }
        return wire_success(Some(invocation_response::Result::Batch(BatchResult {
            items,
            receipts: Vec::new(),
        })));
    }
    execute_invocation(service, correlation_id, meta, identity, operation).await
}

fn prepare_invocation(
    peer_id: &str,
    invocation: InvocationRequest,
) -> Result<(RequestMeta, ClientIdentity, invocation_request::Operation), InvocationResponse> {
    let Some(meta) = invocation.meta else {
        return Err(wire_error(
            StableErrorCode::StableErrorInvalidRequest,
            "request metadata is required",
        ));
    };
    if meta.tenant.trim().is_empty() {
        return Err(wire_error(
            StableErrorCode::StableErrorUnauthenticated,
            "tenant is required",
        ));
    }
    if meta.deadline_unix_ms != 0 && meta.deadline_unix_ms <= unix_time_ms() {
        return Err(wire_error(
            StableErrorCode::StableErrorDeadlineExceeded,
            "deadline expired",
        ));
    }
    let identity = ClientIdentity::new(peer_id, meta.tenant.clone()).map_err(|_| {
        wire_error(
            StableErrorCode::StableErrorUnauthenticated,
            "invalid identity",
        )
    })?;
    let operation = invocation.operation.ok_or_else(|| {
        wire_error(
            StableErrorCode::StableErrorInvalidRequest,
            "operation is required",
        )
    })?;
    Ok((meta, identity, operation))
}

async fn execute_invocation(
    service: &Hc2ClientPlaneService,
    correlation_id: u64,
    meta: RequestMeta,
    identity: ClientIdentity,
    operation: invocation_request::Operation,
) -> InvocationResponse {
    let request = match to_client_request(operation) {
        Ok(request) => request,
        Err(detail) => return wire_error(StableErrorCode::StableErrorInvalidRequest, detail),
    };
    let issued = service
        .member
        .as_ref()
        .and_then(|_| scoped_identity(&meta.idempotency_key));
    let item_id = (!meta.idempotency_key.is_empty())
        .then(|| issued.unwrap_or_else(|| hex(&meta.idempotency_key)));
    execute_normalized(
        service,
        correlation_id,
        meta.deadline_unix_ms,
        &identity,
        request,
        item_id,
    )
    .await
}

async fn execute_normalized(
    service: &Hc2ClientPlaneService,
    correlation: u64,
    deadline_unix_ms: u64,
    identity: &ClientIdentity,
    request: ClientRequest,
    request_identity: Option<String>,
) -> InvocationResponse {
    let mut envelope = ClientRequestEnvelope::new(correlation.to_string(), request);
    let deadline = if deadline_unix_ms == 0 {
        None
    } else {
        let remaining_ms = deadline_unix_ms.saturating_sub(unix_time_ms());
        if remaining_ms == 0 {
            return wire_error(
                StableErrorCode::StableErrorDeadlineExceeded,
                "deadline expired",
            );
        }
        envelope = envelope.with_deadline_ms(deadline_unix_ms);
        tokio::time::Instant::now().checked_add(std::time::Duration::from_millis(remaining_ms))
    };
    if let Some(request_identity) = request_identity {
        envelope = envelope.with_idempotency_key(request_identity);
    }
    from_dispatch_response(
        service
            .state
            .dispatch_verified_request_with_deadline(identity, envelope, deadline)
            .await,
    )
}

fn scoped_identity(bytes: &[u8]) -> Option<String> {
    let token = std::str::from_utf8(bytes).ok()?;
    let (epoch, nonce) = token.strip_prefix("w10e:")?.split_once(':')?;
    if epoch.parse::<u64>().ok()? == 0
        || nonce.is_empty()
        || token.len() > 128
        || !nonce
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return None;
    }
    Some(token.to_owned())
}

fn admitted_distributed_operation(operation: &invocation_request::Operation) -> bool {
    use invocation_request::Operation;
    match operation {
        Operation::Get(_) | Operation::Delete(_) | Operation::RemoveIfValue(_) => true,
        Operation::Put(value) => value.ttl_ms == 0,
        Operation::CompareAndSet(value) => value.ttl_ms == 0,
        _ => false,
    }
}

// Existing synchronous characterization cases exercise only the immediately-ready local backend.
#[cfg(test)]
fn dispatch_invocation(
    service: &Hc2ClientPlaneService,
    peer_id: &str,
    correlation_id: u64,
    invocation: InvocationRequest,
) -> InvocationResponse {
    use futures_util::FutureExt;
    dispatch_invocation_async(service, peer_id, correlation_id, invocation)
        .now_or_never()
        .expect("local characterization dispatcher must complete in one poll")
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
        invocation_request::Operation::TryLock(value) => ClientRequest::TryLock {
            ns,
            key: structured_key(&value.key)?,
            lease_ms: value.lease_ms,
            wait_ms: value.wait_ms,
            level: LockConsistency::Quorum,
        },
        invocation_request::Operation::Unlock(value) => ClientRequest::Unlock {
            ns,
            key: structured_key(&value.key)?,
            fence: value.fence,
        },
        invocation_request::Operation::RenewLock(value) => ClientRequest::RenewLockLease {
            ns,
            key: structured_key(&value.key)?,
            fence: value.fence,
            lease_ms: value.lease_ms,
        },
        invocation_request::Operation::LockOwnership(value) => ClientRequest::GetLockOwnership {
            ns,
            key: structured_key(&value.key)?,
        },
        invocation_request::Operation::RemoveIfValue(value) => ClientRequest::RemoveIfValue {
            ns,
            key: structured_key(&value.key)?,
            expected: value.expected.to_vec(),
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
        Ok(ClientResponse::LockAcquired { fence }) => {
            wire_success(Some(invocation_response::Result::Lock(LockResult {
                acquired: true,
                fence,
            })))
        }
        Ok(ClientResponse::LockBusy) => {
            wire_success(Some(invocation_response::Result::Lock(LockResult {
                acquired: false,
                fence: 0,
            })))
        }
        Ok(ClientResponse::LockReleased | ClientResponse::LockLeaseRenewed) => wire_success(Some(
            invocation_response::Result::Mutation(MutationResult { applied: true }),
        )),
        Ok(ClientResponse::LockOwnership { fence, locked }) => wire_success(Some(
            invocation_response::Result::LockOwnership(LockOwnershipResult {
                locked,
                fence: fence.unwrap_or_default(),
            }),
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
    identity: SessionIdentity,
    key: &Bytes,
    value: &Bytes,
    removed: bool,
    subscriptions: &mut BTreeMap<u64, (Bytes, u64)>,
    outbound: &OutboundQueue,
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
                        identity,
                        0,
                        server_envelope::Message::Gap(EventGap {
                            subscription_id: *subscription_id,
                            after_watermark: *watermark,
                            details: None,
                        }),
                    )))
                    .await?;
            }
            *watermark = next;
            outbound
                .send(Ok(server_envelope(
                    identity,
                    0,
                    server_envelope::Message::Event(CacheEvent {
                        subscription_id: *subscription_id,
                        watermark: next,
                        key: key.clone(),
                        value: value.clone(),
                        removed,
                    }),
                )))
                .await?;
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
        invocation_request::Operation::TryLock(_)
        | invocation_request::Operation::Unlock(_)
        | invocation_request::Operation::RenewLock(_)
        | invocation_request::Operation::LockOwnership(_) => None,
        invocation_request::Operation::RemoveIfValue(value) => {
            Some((value.key.clone(), Bytes::new(), true))
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
    identity: SessionIdentity,
    correlation_id: u64,
    message: server_envelope::Message,
) -> ServerEnvelope {
    ServerEnvelope {
        generation: identity.protocol_generation,
        connection_generation: identity.connection_generation,
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

pub(crate) fn unix_time_ms() -> u64 {
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

    use hydracache_client_hc2::wire::{
        GetRequest, LockOwnershipRequest, PutRequest, RemoveIfValueRequest, RequestMeta,
        TryLockRequest, UnlockRequest,
    };
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

    #[derive(Debug)]
    struct SuspendedBackend {
        entered: AtomicU64,
        completed: AtomicU64,
        release: Semaphore,
    }

    impl hydracache_client_transport_axum::ClientDataBackend for SuspendedBackend {
        fn execute<'a>(
            &'a self,
            _request: hydracache_client_transport_axum::VerifiedClientRequest<'a>,
        ) -> hydracache_client_transport_axum::ClientBackendFuture<'a> {
            Box::pin(async move {
                self.entered.fetch_add(1, Ordering::AcqRel);
                self.release.acquire().await.unwrap().forget();
                self.completed.fetch_add(1, Ordering::AcqRel);
                Ok(ClientResponse::Value { value: None })
            })
        }
    }

    #[tokio::test]
    async fn asynchronous_invocation_releases_pending_accounting_on_completion_and_disconnect() {
        use futures_util::FutureExt;
        let backend = Arc::new(SuspendedBackend {
            entered: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            release: Semaphore::new(0),
        });
        let dispatcher = Arc::new(
            ClientSurfaceDispatcher::from_backend(Default::default(), backend.clone()).unwrap(),
        );
        let service = Hc2ClientPlaneService::from_dispatcher(dispatcher, "test-cluster");
        let invocation = || InvocationRequest {
            meta: Some(meta()),
            operation: Some(invocation_request::Operation::Get(GetRequest {
                key: Bytes::from_static(b"key"),
            })),
        };
        let (outbound, receiver) = OutboundQueue::new(1);
        let mut request = Box::pin(dispatch_invocation_until_closed(
            &service,
            "peer",
            1,
            invocation(),
            &outbound,
        ));
        assert!(request.as_mut().now_or_never().is_none());
        assert_eq!(service.accounting().pending_invocations, 1);
        assert_eq!(backend.entered.load(Ordering::Acquire), 1);
        drop(receiver);
        assert_eq!(request.await.unwrap_err().code(), tonic::Code::Cancelled);
        assert_eq!(service.accounting().pending_invocations, 0);
        assert_eq!(backend.completed.load(Ordering::Acquire), 0);

        let (outbound, _receiver) = OutboundQueue::new(1);
        backend.release.add_permits(1);
        let response =
            dispatch_invocation_until_closed(&service, "peer", 2, invocation(), &outbound)
                .await
                .unwrap();
        assert!(matches!(
            response.result,
            Some(invocation_response::Result::Value(ValueResult {
                found: false,
                ..
            }))
        ));
        assert_eq!(service.accounting().pending_invocations, 0);
        assert_eq!(backend.completed.load(Ordering::Acquire), 1);
        let mut request = Box::pin(dispatch_invocation_async(&service, "peer", 3, invocation()));
        assert!(request.as_mut().now_or_never().is_none());
        assert_eq!(service.accounting().pending_invocations, 1);
        drop(request);
        assert_eq!(service.accounting().pending_invocations, 0);
        assert_eq!(backend.completed.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn event_fanout_shares_immutable_payload_and_preserves_frames() {
        let service = service();
        let identity = SessionIdentity {
            protocol_generation: HC2_GENERATION,
            connection_generation: 1,
        };
        let key = Bytes::from_static(b"event/key");
        let value = Bytes::from_static(b"event-value");
        let mut subscriptions = BTreeMap::from([
            (1, (Bytes::from_static(b"event/"), 0)),
            (2, (Bytes::from_static(b"event/"), 0)),
        ]);
        let (outbound, mut receiver) = OutboundQueue::new(2);

        emit_matching_events(
            &service,
            identity,
            &key,
            &value,
            false,
            &mut subscriptions,
            &outbound,
        )
        .await
        .unwrap();

        for expected_subscription in [1, 2] {
            let envelope = receiver.recv().await.unwrap().frame.unwrap();
            let Some(server_envelope::Message::Event(event)) = envelope.message else {
                panic!("expected event frame");
            };
            assert_eq!(event.subscription_id, expected_subscription);
            assert_eq!(event.key, key);
            assert_eq!(event.value, value);
            assert_eq!(event.key.as_ptr(), key.as_ptr());
            assert_eq!(event.value.as_ptr(), value.as_ptr());
        }
        assert_eq!(
            outbound.byte_budget.available_permits(),
            HC2_OUTBOUND_QUEUE_BYTE_BUDGET
        );
    }

    #[tokio::test]
    async fn outbound_byte_admission_charges_encoded_length_and_releases_on_poll() {
        let (outbound, mut receiver) = OutboundQueue::new(16);
        let envelope = server_envelope(
            SessionIdentity {
                protocol_generation: HC2_GENERATION,
                connection_generation: 1,
            },
            7,
            server_envelope::Message::Event(CacheEvent {
                subscription_id: 1,
                watermark: 1,
                key: Bytes::from_static(b"key"),
                value: Bytes::from(vec![0x5a; 4_096]),
                removed: false,
            }),
        );
        let encoded_len = envelope.encoded_len();

        outbound.send(Ok(envelope)).await.unwrap();
        assert_eq!(
            outbound.byte_budget.available_permits(),
            HC2_OUTBOUND_QUEUE_BYTE_BUDGET - encoded_len
        );
        drop(receiver.recv().await.unwrap());
        assert_eq!(
            outbound.byte_budget.available_permits(),
            HC2_OUTBOUND_QUEUE_BYTE_BUDGET
        );
    }

    #[tokio::test]
    async fn outbound_byte_admission_allows_one_oversize_and_releases_on_disconnect() {
        let (outbound, mut receiver) = OutboundQueue::new(16);
        let oversized = || {
            server_envelope(
                SessionIdentity {
                    protocol_generation: HC2_GENERATION,
                    connection_generation: 1,
                },
                9,
                server_envelope::Message::Event(CacheEvent {
                    subscription_id: 1,
                    watermark: 1,
                    key: Bytes::from_static(b"key"),
                    value: Bytes::from(vec![0x5a; HC2_OUTBOUND_QUEUE_BYTE_BUDGET + 128]),
                    removed: false,
                }),
            )
        };

        outbound.send(Ok(oversized())).await.unwrap();
        assert_eq!(outbound.byte_budget.available_permits(), 0);

        let second_outbound = outbound.clone();
        let second_frame = oversized();
        let mut second = tokio::spawn(async move { second_outbound.send(Ok(second_frame)).await });
        assert!(tokio::time::timeout(Duration::from_millis(20), &mut second)
            .await
            .is_err());

        drop(receiver.recv().await.unwrap());
        second.await.unwrap().unwrap();
        assert_eq!(outbound.byte_budget.available_permits(), 0);
        drop(receiver);
        tokio::task::yield_now().await;
        assert_eq!(
            outbound.byte_budget.available_permits(),
            HC2_OUTBOUND_QUEUE_BYTE_BUDGET
        );

        let (closed_outbound, closed_receiver) = OutboundQueue::new(1);
        drop(closed_receiver);
        assert!(closed_outbound.send(Ok(oversized())).await.is_err());
        assert_eq!(
            closed_outbound.byte_budget.available_permits(),
            HC2_OUTBOUND_QUEUE_BYTE_BUDGET
        );
    }

    #[test]
    fn distributed_hc2_rejects_unavailable_operations_and_checks_epoch_tokens() {
        use hydracache_client_hc2::wire::{CompareAndSetRequest, GetRequest, PutRequest};
        use invocation_request::Operation;
        assert!(admitted_distributed_operation(&Operation::Get(
            GetRequest {
                key: Bytes::from_static(b"k"),
            }
        )));
        assert!(!admitted_distributed_operation(&Operation::Put(
            PutRequest {
                key: Bytes::from_static(b"k"),
                value: Bytes::from_static(b"value"),
                ttl_ms: 1,
            }
        )));
        assert!(!admitted_distributed_operation(&Operation::CompareAndSet(
            CompareAndSetRequest {
                key: Bytes::from_static(b"k"),
                expected: Bytes::from_static(b"old"),
                replacement: Bytes::from_static(b"new"),
                ttl_ms: 1,
            }
        )));
        assert_eq!(
            scoped_identity(b"w10e:3:valid_0-a").as_deref(),
            Some("w10e:3:valid_0-a")
        );
        for invalid in [
            b"w10e:0:nonce".as_slice(),
            b"w10e:3:",
            b"w10e:3:bad:tag",
            b"Opaque",
        ] {
            assert_eq!(scoped_identity(invalid), None);
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
    fn conditional_remove_and_fenced_lock_use_the_shared_dispatch() {
        let service = service();
        let invoke = |correlation_id, operation| {
            dispatch_invocation(
                &service,
                "verified-peer",
                correlation_id,
                InvocationRequest {
                    meta: Some(meta()),
                    operation: Some(operation),
                },
            )
        };
        invoke(
            1,
            invocation_request::Operation::Put(PutRequest {
                key: Bytes::from_static(b"key"),
                value: Bytes::from_static(b"value"),
                ttl_ms: 0,
            }),
        );
        let mismatch = invoke(
            2,
            invocation_request::Operation::RemoveIfValue(RemoveIfValueRequest {
                key: Bytes::from_static(b"key"),
                expected: Bytes::from_static(b"wrong"),
            }),
        );
        assert!(matches!(
            mismatch.result,
            Some(invocation_response::Result::Mutation(MutationResult {
                applied: false
            }))
        ));
        let removed = invoke(
            3,
            invocation_request::Operation::RemoveIfValue(RemoveIfValueRequest {
                key: Bytes::from_static(b"key"),
                expected: Bytes::from_static(b"value"),
            }),
        );
        assert!(matches!(
            removed.result,
            Some(invocation_response::Result::Mutation(MutationResult {
                applied: true
            }))
        ));

        let acquired = invoke(
            4,
            invocation_request::Operation::TryLock(TryLockRequest {
                key: Bytes::from_static(b"lock"),
                lease_ms: 10_000,
                wait_ms: 0,
            }),
        );
        let fence = match acquired.result {
            Some(invocation_response::Result::Lock(LockResult {
                acquired: true,
                fence,
            })) => fence,
            other => panic!("expected acquired lock, got {other:?}"),
        };
        assert_ne!(fence, 0);
        let ownership = invoke(
            5,
            invocation_request::Operation::LockOwnership(LockOwnershipRequest {
                key: Bytes::from_static(b"lock"),
            }),
        );
        assert!(matches!(
            ownership.result,
            Some(invocation_response::Result::LockOwnership(
                LockOwnershipResult {
                    locked: true,
                    fence: observed,
                }
            )) if observed == fence
        ));
        let released = invoke(
            6,
            invocation_request::Operation::Unlock(UnlockRequest {
                key: Bytes::from_static(b"lock"),
                fence,
            }),
        );
        assert!(matches!(
            released.result,
            Some(invocation_response::Result::Mutation(MutationResult {
                applied: true
            }))
        ));
    }

    #[test]
    fn prometheus_accounting_has_only_bounded_privacy_safe_labels() {
        let service = service();
        service
            .accounting
            .active_connections
            .store(2, Ordering::Release);
        service
            .accounting
            .active_subscriptions
            .store(3, Ordering::Release);
        service
            .accounting
            .active_sessions
            .store(4, Ordering::Release);
        service
            .accounting
            .pending_invocations
            .store(5, Ordering::Release);
        service
            .accounting
            .rejected_frames
            .store(6, Ordering::Release);

        let metrics = service.prometheus_metrics();
        assert!(metrics.contains("hydracache_hc2_connections{transport=\"grpc_bidirectional\"} 2"));
        assert!(metrics
            .contains("hydracache_hc2_pending_invocations{transport=\"grpc_bidirectional\"} 5"));
        assert!(metrics
            .contains("hydracache_hc2_rejected_frames_total{transport=\"grpc_bidirectional\"} 6"));
        for forbidden in [
            "tenant-a",
            "verified-peer",
            "test-cluster",
            "endpoint",
            "authority",
            "certificate",
        ] {
            assert!(!metrics.contains(forbidden));
        }
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
        let active_client = client.retained_state();
        assert_eq!(active_client.active_subscriptions, 1);
        assert_eq!(active_client.active_sessions, 1);
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
        let closed_client = client.retained_state();
        assert!(closed_client.closed);
        assert_eq!(closed_client.pending_invocations, 0);
        assert_eq!(closed_client.pending_subscriptions, 0);
        assert_eq!(closed_client.active_subscriptions, 0);
        assert_eq!(closed_client.pending_sessions, 0);
        assert_eq!(closed_client.active_sessions, 0);
        assert_eq!(
            closed_client.available_invocation_permits,
            ClientConfig::new("limits", "tenant-a")
                .limits
                .max_pending_invocations
        );
        assert_eq!(
            closed_client.available_subscription_permits,
            ClientConfig::new("limits", "tenant-a")
                .limits
                .max_subscriptions
        );
        assert_eq!(
            closed_client.available_session_permits,
            ClientConfig::new("limits", "tenant-a").limits.max_sessions
        );

        for _ in 0..50 {
            if observed.accounting().live_resources_zero() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(observed.accounting().live_resources_zero());

        for cardinality in [1_usize, 10, 100, 1_000] {
            let mut clients = Vec::with_capacity(cardinality);
            for index in 0..cardinality {
                let client = Hc2Client::connect(
                    &adapter(addr, &trusted, &trusted),
                    ClientConfig::new(
                        format!("retention-series-{cardinality}-{index}"),
                        "tenant-a",
                    ),
                )
                .await
                .unwrap();
                clients.push(client);
            }
            for _ in 0..100 {
                if observed.accounting().active_connections == cardinality as u64 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            let active = observed.accounting();
            assert_eq!(active.active_connections, cardinality as u64);
            assert_eq!(
                active
                    .accepted_connections
                    .saturating_sub(active.closed_connections),
                active.active_connections
            );
            assert_eq!(active.active_subscriptions, 0);
            assert_eq!(active.active_sessions, 0);
            assert_eq!(active.pending_invocations, 0);
            for client in &clients {
                let retained = client.retained_state();
                assert_eq!(retained.pending_invocations, 0);
                assert_eq!(retained.pending_subscriptions, 0);
                assert_eq!(retained.active_subscriptions, 0);
                assert_eq!(retained.pending_sessions, 0);
                assert_eq!(retained.active_sessions, 0);
                client.close();
            }
            drop(clients);
            for _ in 0..100 {
                if observed.accounting().live_resources_zero() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert!(observed.accounting().live_resources_zero());
            let closed = observed.accounting();
            assert_eq!(closed.accepted_connections, closed.closed_connections);
        }
        shutdown_tx.send(true).unwrap();
        serving.await.unwrap().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
