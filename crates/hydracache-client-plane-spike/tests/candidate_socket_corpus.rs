use std::time::Duration;

use hydracache_client_plane_spike::{
    BootstrapConnection, ClientAuthorizationPolicy, ConnectionGeneration, FrameKind, PeerIdentity,
    SpikeCodec, SpikeFrame, SpikeLimits, TransportCandidate, HC2_GENERATION,
};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod support;

const GENERATION: ConnectionGeneration = ConnectionGeneration::FIRST;
const MAX_FRAME_BYTES: usize = 64;

fn ready(
    candidate: TransportCandidate,
    limits: SpikeLimits,
) -> hydracache_client_plane_spike::SpikeConnection {
    BootstrapConnection::new(candidate, GENERATION, limits)
        .verify_tls(true)
        .unwrap()
        .authenticate(PeerIdentity::verified("corpus-client", "corpus-tenant"))
        .unwrap()
        .authorize(&ClientAuthorizationPolicy::exact("corpus-client", "corpus-tenant").unwrap())
        .unwrap()
        .negotiate(HC2_GENERATION)
        .unwrap()
}

fn hostile_cases(candidate: TransportCandidate) -> Vec<(Vec<u8>, bool)> {
    let codec = SpikeCodec::new(candidate, MAX_FRAME_BYTES);
    let valid = codec
        .encode(&SpikeFrame::current(
            GENERATION,
            FrameKind::Invocation,
            7,
            b"get".to_vec(),
        ))
        .unwrap();
    let other = TransportCandidate::ALL
        .into_iter()
        .find(|value| *value != candidate)
        .unwrap();
    let cross_candidate = SpikeCodec::new(other, MAX_FRAME_BYTES)
        .encode(&SpikeFrame::current(
            GENERATION,
            FrameKind::Invocation,
            8,
            b"get".to_vec(),
        ))
        .unwrap();
    let mut unknown_generation = valid.clone();
    unknown_generation[8..10].copy_from_slice(&(HC2_GENERATION + 1).to_be_bytes());
    let mut unknown_kind = valid.clone();
    unknown_kind[18] = u8::MAX;
    let mut trailing = valid.clone();
    trailing.push(0);
    vec![
        (valid.clone(), true),
        (cross_candidate, false),
        (valid[..valid.len() - 1].to_vec(), false),
        (vec![0_u8; MAX_FRAME_BYTES + 1], false),
        (unknown_generation, false),
        (unknown_kind, false),
        (trailing, false),
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hostile_corpus_reaches_no_dispatch_over_real_mtls_sockets() {
    for candidate in TransportCandidate::ALL {
        let cases = hostile_cases(candidate);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let pki = support::TestPki::generate();
        let acceptor = pki.server_acceptor();
        let connector = pki.client_connector();
        let case_count = cases.len();
        let server = tokio::spawn(async move {
            let codec = SpikeCodec::new(candidate, MAX_FRAME_BYTES);
            let mut accepted = 0_u64;
            for _ in 0..case_count {
                let (socket, _) = listener.accept().await.unwrap();
                let mut socket = acceptor.accept(socket).await.unwrap();
                let mut bytes = Vec::new();
                tokio::time::timeout(Duration::from_secs(2), socket.read_to_end(&mut bytes))
                    .await
                    .expect("hostile corpus read deadline")
                    .unwrap();
                let dispatched = codec.decode(&bytes).is_ok_and(|frame| {
                    let mut connection = ready(
                        candidate,
                        SpikeLimits {
                            max_frame_bytes: MAX_FRAME_BYTES,
                            ..SpikeLimits::default()
                        },
                    );
                    connection.receive(&codec.encode(&frame).unwrap()).is_ok()
                        && connection.dispatch_count() == 1
                });
                accepted += u64::from(dispatched);
                socket.write_all(&[u8::from(dispatched)]).await.unwrap();
                socket.shutdown().await.unwrap();
            }
            accepted
        });

        for (bytes, expected) in cases {
            let socket = TcpStream::connect(address).await.unwrap();
            let mut socket = connector
                .connect(ServerName::try_from("localhost").unwrap(), socket)
                .await
                .unwrap();
            socket.write_all(&bytes).await.unwrap();
            socket.shutdown().await.unwrap();
            let mut verdict = [0_u8; 1];
            socket.read_exact(&mut verdict).await.unwrap();
            assert_eq!(verdict[0] == 1, expected, "candidate={candidate:?}");
        }
        assert_eq!(server.await.unwrap(), 1, "candidate={candidate:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slow_consumer_receives_explicit_gap_over_each_candidate_socket_codec() {
    for candidate in TransportCandidate::ALL {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let pki = support::TestPki::generate();
        let acceptor = pki.server_acceptor();
        let connector = pki.client_connector();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = acceptor.accept(socket).await.unwrap();
            let mut connection = ready(
                candidate,
                SpikeLimits {
                    max_event_frames: 2,
                    ..SpikeLimits::default()
                },
            );
            connection.register_subscription(41).unwrap();
            connection
                .push_event(GENERATION, 41, b"one".to_vec())
                .unwrap();
            connection
                .push_event(GENERATION, 41, b"two".to_vec())
                .unwrap();
            connection
                .push_event(GENERATION, 41, b"three".to_vec())
                .unwrap();
            assert_eq!(connection.resources().event_frames, 1);
            let gap = connection.drain_next().unwrap();
            assert_eq!(gap.kind, FrameKind::Gap);
            socket
                .write_all(&connection.encode(&gap).unwrap())
                .await
                .unwrap();
            socket.shutdown().await.unwrap();
            connection.disconnect()
        });

        let socket = TcpStream::connect(address).await.unwrap();
        let mut socket = connector
            .connect(ServerName::try_from("localhost").unwrap(), socket)
            .await
            .unwrap();
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).await.unwrap();
        let gap = SpikeCodec::new(candidate, SpikeLimits::default().max_frame_bytes)
            .decode(&bytes)
            .unwrap();
        assert_eq!(gap.kind, FrameKind::Gap);
        assert_eq!(gap.correlation_id, 41);
        assert_eq!(
            server.await.unwrap(),
            hydracache_client_plane_spike::ResourceSnapshot::default()
        );
    }
}
