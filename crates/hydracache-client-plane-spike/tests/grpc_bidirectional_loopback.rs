use std::collections::BTreeSet;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures_util::Stream;
use hydracache_client_plane_spike::{
    BootstrapConnection, FrameKind, PeerIdentity, ResourceSnapshot, SpikeConnection, SpikeFrame,
    SpikeLimits, TransportCandidate, HC2_GENERATION,
};
use prost::Message;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity, Server, ServerTlsConfig};
use tonic::{Code, Request, Response, Status, Streaming};

mod support;

mod proto {
    tonic::include_proto!("hydracache.client.v2");
}

use proto::client_plane_spike_client::ClientPlaneSpikeClient;
use proto::client_plane_spike_server::{ClientPlaneSpike, ClientPlaneSpikeServer};
use proto::SpikeEnvelope;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<SpikeEnvelope, Status>> + Send>>;

const GOLDEN_ENVELOPE: &[u8] = &[
    0x08, 0x05, 0x10, 0x02, 0x18, 0x13, 0x22, 0x03, 0x67, 0x65, 0x74,
];

#[derive(Clone)]
struct SpikeService {
    closed: Arc<Mutex<Option<oneshot::Sender<ResourceSnapshot>>>>,
}

fn ready_connection() -> SpikeConnection {
    BootstrapConnection::new(
        TransportCandidate::GrpcBidirectional,
        SpikeLimits::default(),
    )
    .verify_tls(true)
    .unwrap()
    .authenticate(PeerIdentity::verified("grpc-client", "grpc-tenant"))
    .unwrap()
    .negotiate(HC2_GENERATION)
    .unwrap()
}

fn to_proto(frame: SpikeFrame) -> SpikeEnvelope {
    SpikeEnvelope {
        generation: u32::from(frame.generation),
        kind: u32::from(frame.kind as u8),
        correlation_id: frame.correlation_id,
        payload: frame.payload,
    }
}

fn from_proto(frame: SpikeEnvelope) -> Result<SpikeFrame, Status> {
    let generation = u16::try_from(frame.generation)
        .map_err(|_| Status::failed_precondition("generation exceeds u16"))?;
    let kind = u8::try_from(frame.kind)
        .map_err(|_| Status::invalid_argument("message kind exceeds u8"))?
        .try_into()
        .map_err(|_| Status::invalid_argument("unknown message kind"))?;
    Ok(SpikeFrame {
        generation,
        kind,
        correlation_id: frame.correlation_id,
        payload: frame.payload,
    })
}

#[tonic::async_trait]
impl ClientPlaneSpike for SpikeService {
    type OpenStream = ResponseStream;

    async fn open(
        &self,
        request: Request<Streaming<SpikeEnvelope>>,
    ) -> Result<Response<Self::OpenStream>, Status> {
        let mut inbound = request.into_inner();
        let mut connection = ready_connection();
        connection
            .register_subscription(55)
            .map_err(|error| Status::resource_exhausted(error.to_string()))?;
        let mut correlations = Vec::new();
        while let Some(request) = inbound.message().await? {
            let frame = from_proto(request)?;
            let encoded = connection
                .encode(&frame)
                .map_err(|error| Status::invalid_argument(error.to_string()))?;
            let invocation = connection
                .receive(&encoded)
                .map_err(|error| Status::invalid_argument(error.to_string()))?;
            connection
                .begin_invocation(invocation.correlation_id)
                .map_err(|error| Status::resource_exhausted(error.to_string()))?;
            correlations.push(invocation.correlation_id);
        }
        if correlations.is_empty() {
            return Err(Status::invalid_argument("missing invocation"));
        }
        for correlation_id in correlations.iter().copied() {
            connection
                .complete_invocation(correlation_id, b"grpc-value".to_vec())
                .map_err(|error| Status::internal(error.to_string()))?;
        }
        connection
            .heartbeat()
            .map_err(|error| Status::internal(error.to_string()))?;
        connection
            .push_event(55, b"grpc-entry-event".to_vec())
            .map_err(|error| Status::internal(error.to_string()))?;

        let (tx, rx) = mpsc::channel(correlations.len() + 2);
        while let Some(frame) = connection.drain_next() {
            tx.send(Ok(to_proto(frame))).await.unwrap();
        }
        drop(tx);
        if let Some(closed) = self.closed.lock().expect("closed sender").take() {
            let _ = closed.send(connection.disconnect());
        }
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

#[tokio::test]
async fn grpc_candidate_interleaves_reply_and_event_on_a_generated_real_stream() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let incoming = TcpListenerStream::new(listener);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (closed_tx, closed_rx) = oneshot::channel();
    let pki = support::TestPki::generate();
    let service = SpikeService {
        closed: Arc::new(Mutex::new(Some(closed_tx))),
    };
    let server_identity =
        Identity::from_pem(pki.server_cert_pem.clone(), pki.server_key_pem.clone());
    let ca_pem = pki.ca_pem.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .tls_config(
                ServerTlsConfig::new()
                    .identity(server_identity)
                    .client_ca_root(Certificate::from_pem(ca_pem.clone())),
            )
            .unwrap()
            .add_service(ClientPlaneSpikeServer::new(service))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    let endpoint = format!("https://localhost:{}", address.port());
    let channel = Channel::from_shared(endpoint)
        .unwrap()
        .tls_config(
            ClientTlsConfig::new()
                .ca_certificate(Certificate::from_pem(pki.ca_pem))
                .identity(Identity::from_pem(pki.client_cert_pem, pki.client_key_pem))
                .domain_name("localhost"),
        )
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = ClientPlaneSpikeClient::new(channel);
    let (tx, rx) = mpsc::channel(256);
    let sender = tokio::spawn(async move {
        for correlation_id in 1..=256 {
            tx.send(to_proto(SpikeFrame::current(
                FrameKind::Invocation,
                correlation_id,
                b"get:key".to_vec(),
            )))
            .await
            .unwrap();
        }
    });

    let mut stream = client
        .open(ReceiverStream::new(rx))
        .await
        .unwrap()
        .into_inner();
    sender.await.unwrap();
    let mut replies = BTreeSet::new();
    let mut saw_heartbeat = false;
    let mut saw_event = false;
    while let Some(envelope) = stream.message().await.unwrap() {
        let frame = from_proto(envelope).unwrap();
        match frame.kind {
            FrameKind::Reply => {
                replies.insert(frame.correlation_id);
            }
            FrameKind::Heartbeat => saw_heartbeat = true,
            FrameKind::Event => {
                assert_eq!(frame.correlation_id, 55);
                saw_event = true;
            }
            other => panic!("unexpected server frame: {other:?}"),
        }
    }
    assert_eq!(replies, (1..=256).collect());
    assert!(saw_heartbeat);
    assert!(saw_event);

    assert_eq!(closed_rx.await.unwrap(), ResourceSnapshot::default());
    shutdown_tx.send(()).unwrap();
    server.await.unwrap();
}

async fn serve_rejection_probe(
    pki: &support::TestPki,
) -> (u16, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let incoming = TcpListenerStream::new(listener);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let service = SpikeService {
        closed: Arc::new(Mutex::new(None)),
    };
    let identity = Identity::from_pem(pki.server_cert_pem.clone(), pki.server_key_pem.clone());
    let ca = Certificate::from_pem(pki.ca_pem.clone());
    let server = tokio::spawn(async move {
        Server::builder()
            .tls_config(ServerTlsConfig::new().identity(identity).client_ca_root(ca))
            .unwrap()
            .add_service(ClientPlaneSpikeServer::new(service))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    (port, shutdown_tx, server)
}

async fn assert_tls_rejected(port: u16, tls: ClientTlsConfig) {
    let endpoint = format!("https://localhost:{port}");
    match Channel::from_shared(endpoint)
        .unwrap()
        .tls_config(tls)
        .unwrap()
        .connect()
        .await
    {
        Err(_) => {}
        Ok(channel) => {
            let mut client = ClientPlaneSpikeClient::new(channel);
            let (tx, rx) = mpsc::channel(1);
            tx.send(to_proto(SpikeFrame::current(
                FrameKind::Invocation,
                1,
                b"probe".to_vec(),
            )))
            .await
            .unwrap();
            drop(tx);
            let status = client.open(ReceiverStream::new(rx)).await.unwrap_err();
            assert!(
                matches!(
                    status.code(),
                    Code::Unavailable | Code::Unknown | Code::Internal
                ),
                "request reached the application instead of failing TLS: {status}"
            );
        }
    }
}

#[tokio::test]
async fn grpc_candidate_rejects_an_untrusted_server_ca_before_dispatch() {
    let server_pki = support::TestPki::generate();
    let untrusted_pki = support::TestPki::generate();
    let (port, shutdown, server) = serve_rejection_probe(&server_pki).await;
    assert_tls_rejected(
        port,
        ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(untrusted_pki.ca_pem))
            .identity(Identity::from_pem(
                untrusted_pki.client_cert_pem,
                untrusted_pki.client_key_pem,
            ))
            .domain_name("localhost"),
    )
    .await;
    shutdown.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn grpc_candidate_rejects_a_missing_client_certificate_before_dispatch() {
    let pki = support::TestPki::generate();
    let (port, shutdown, server) = serve_rejection_probe(&pki).await;
    assert_tls_rejected(
        port,
        ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(pki.ca_pem.clone()))
            .domain_name("localhost"),
    )
    .await;
    shutdown.send(()).unwrap();
    server.await.unwrap();
}

#[test]
fn generated_rust_codec_matches_the_cross_language_golden_frame() {
    let envelope = SpikeEnvelope {
        generation: u32::from(HC2_GENERATION),
        kind: u32::from(FrameKind::Invocation as u8),
        correlation_id: 19,
        payload: b"get".to_vec(),
    };
    assert_eq!(envelope.encode_to_vec(), GOLDEN_ENVELOPE);
    assert_eq!(SpikeEnvelope::decode(GOLDEN_ENVELOPE).unwrap(), envelope);
}
