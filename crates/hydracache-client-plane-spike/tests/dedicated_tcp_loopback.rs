use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache_client_plane_spike::{
    BootstrapConnection, ClientAuthorizationPolicy, ConnectionGeneration, FrameKind, PeerIdentity,
    ResourceSnapshot, SpikeConnection, SpikeFrame, SpikeLimits, TransportCandidate, HC2_GENERATION,
};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

mod support;

const PREFIX_BYTES: usize = 8;
const CONNECTION_GENERATION: ConnectionGeneration = ConnectionGeneration::FIRST;

async fn read_exact_frame<S>(stream: &mut S, max: usize) -> Vec<u8>
where
    S: AsyncRead + Unpin,
{
    let mut prefix = [0_u8; PREFIX_BYTES];
    stream.read_exact(&mut prefix).await.expect("frame prefix");
    let body_len = u32::from_be_bytes(prefix[4..8].try_into().expect("length slice")) as usize;
    assert!(body_len + PREFIX_BYTES <= max, "bounded frame");
    let mut frame = Vec::with_capacity(PREFIX_BYTES + body_len);
    frame.extend_from_slice(&prefix);
    frame.resize(PREFIX_BYTES + body_len, 0);
    stream
        .read_exact(&mut frame[PREFIX_BYTES..])
        .await
        .expect("frame body");
    frame
}

fn ready_connection() -> SpikeConnection {
    BootstrapConnection::new(
        TransportCandidate::DedicatedTcpTls,
        CONNECTION_GENERATION,
        SpikeLimits::default(),
    )
    .verify_tls(true)
    .unwrap()
    .authenticate(PeerIdentity::verified("loopback-client", "loopback-tenant"))
    .unwrap()
    .authorize(&ClientAuthorizationPolicy::exact("loopback-client", "loopback-tenant").unwrap())
    .unwrap()
    .negotiate(HC2_GENERATION)
    .unwrap()
}

#[tokio::test]
async fn dedicated_tcp_candidate_round_trips_on_a_real_loopback_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (closed_tx, closed_rx) = oneshot::channel();
    let pki = support::TestPki::generate();
    let acceptor = pki.server_acceptor();
    let connector = pki.client_connector();

    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio::time::timeout(Duration::from_secs(5), acceptor.accept(socket))
            .await
            .expect("server mTLS handshake deadline")
            .expect("server mTLS handshake");
        let mut connection = ready_connection();
        connection.register_subscription(44).unwrap();
        for expected in 1..=256 {
            let bytes = tokio::time::timeout(
                Duration::from_secs(5),
                read_exact_frame(&mut socket, SpikeLimits::default().max_frame_bytes),
            )
            .await
            .expect("server request read deadline");
            let request = connection.receive(&bytes).expect("strict frame accepted");
            assert_eq!(request.kind, FrameKind::Invocation);
            assert_eq!(request.correlation_id, expected);
            connection
                .begin_invocation(request.correlation_id)
                .expect("pending invocation");
        }
        for correlation_id in 1..=256 {
            connection
                .complete_invocation(CONNECTION_GENERATION, correlation_id, b"value".to_vec())
                .expect("completion");
        }
        connection.heartbeat(CONNECTION_GENERATION).unwrap();
        connection
            .push_event(CONNECTION_GENERATION, 44, b"tcp-entry-event".to_vec())
            .unwrap();
        while let Some(frame) = connection.drain_next() {
            let encoded = connection.encode(&frame).expect("encoded frame");
            tokio::time::timeout(Duration::from_secs(5), socket.write_all(&encoded))
                .await
                .expect("server frame write deadline")
                .expect("server frame write");
        }
        tokio::time::timeout(Duration::from_secs(5), socket.flush())
            .await
            .expect("server frame flush deadline")
            .expect("server frame flush");
        tokio::time::timeout(Duration::from_secs(5), socket.shutdown())
            .await
            .expect("server half-close deadline")
            .expect("server half-close");
        closed_tx
            .send(connection.disconnect())
            .expect("resource snapshot receiver");
    });

    let client = TcpStream::connect(address).await.unwrap();
    let mut client = tokio::time::timeout(
        Duration::from_secs(5),
        connector.connect(ServerName::try_from("localhost").unwrap(), client),
    )
    .await
    .expect("client mTLS handshake deadline")
    .expect("verified server and client certificates");
    let client_codec = ready_connection();
    for correlation_id in 1..=256 {
        let request = client_codec
            .encode(&SpikeFrame::current(
                CONNECTION_GENERATION,
                FrameKind::Invocation,
                correlation_id,
                b"get:key".to_vec(),
            ))
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), client.write_all(&request))
            .await
            .expect("client invocation write deadline")
            .unwrap();
    }
    tokio::time::timeout(Duration::from_secs(5), client.flush())
        .await
        .expect("client invocation flush deadline")
        .unwrap();
    let mut decoder = ready_connection();
    let mut replies = BTreeSet::new();
    let mut saw_heartbeat = false;
    let mut saw_event = false;
    loop {
        let read = tokio::time::timeout(
            Duration::from_secs(5),
            read_exact_frame(&mut client, SpikeLimits::default().max_frame_bytes),
        )
        .await;
        let frame = match read {
            Err(_) => panic!(
                "client response read deadline after replies={}, heartbeat={}, event={}",
                replies.len(),
                saw_heartbeat,
                saw_event
            ),
            Ok(frame) => match frame {
                frame if !frame.is_empty() => decoder.receive(&frame).expect("frame decoded"),
                _ => unreachable!(),
            },
        };
        match frame.kind {
            FrameKind::Reply => {
                assert_eq!(frame.payload, b"value");
                replies.insert(frame.correlation_id);
            }
            FrameKind::Heartbeat => saw_heartbeat = true,
            FrameKind::Event => {
                assert_eq!(frame.correlation_id, 44);
                saw_event = true;
            }
            other => panic!("unexpected server frame: {other:?}"),
        }
        if replies.len() == 256 && saw_heartbeat && saw_event {
            break;
        }
    }
    assert_eq!(replies, (1..=256).collect());
    drop(client);

    server.await.unwrap();
    assert_eq!(closed_rx.await.unwrap(), ResourceSnapshot::default());
}

async fn assert_rustls_handshake_rejected(
    acceptor: tokio_rustls::TlsAcceptor,
    connector: tokio_rustls::TlsConnector,
    server_name: &'static str,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let dispatch_count = Arc::new(AtomicU64::new(0));
    let server_dispatch_count = Arc::clone(&dispatch_count);
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        if let Ok(mut stream) = acceptor.accept(socket).await {
            let mut byte = [0_u8; 1];
            if matches!(
                tokio::time::timeout(Duration::from_secs(1), stream.read(&mut byte)).await,
                Ok(Ok(1))
            ) {
                server_dispatch_count.fetch_add(1, Ordering::SeqCst);
            }
        }
    });
    let socket = TcpStream::connect(address).await.unwrap();
    let client = connector
        .connect(ServerName::try_from(server_name).unwrap(), socket)
        .await;
    let client_rejected = match client {
        Err(_) => true,
        Ok(mut stream) => {
            let _ = stream.write_all(b"must-not-dispatch").await;
            let mut byte = [0_u8; 1];
            matches!(
                tokio::time::timeout(Duration::from_secs(1), stream.read(&mut byte)).await,
                Ok(Err(_)) | Ok(Ok(0))
            )
        }
    };
    assert!(client_rejected, "client did not observe the TLS rejection");
    server.await.unwrap();
    assert_eq!(
        dispatch_count.load(Ordering::SeqCst),
        0,
        "invalid TLS attempt reached protocol dispatch"
    );
}

async fn assert_rustls_handshake_accepted(
    acceptor: tokio_rustls::TlsAcceptor,
    connector: tokio_rustls::TlsConnector,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut stream = acceptor.accept(socket).await.unwrap();
        let mut byte = [0_u8; 1];
        stream.read_exact(&mut byte).await.unwrap();
        byte
    });
    let socket = TcpStream::connect(address).await.unwrap();
    let mut client = connector
        .connect(ServerName::try_from("localhost").unwrap(), socket)
        .await
        .unwrap();
    client.write_all(b"R").await.unwrap();
    assert_eq!(server.await.unwrap(), [b'R']);
}

#[tokio::test]
async fn rustls_candidates_reject_an_untrusted_ca_before_protocol_dispatch() {
    let server_pki = support::TestPki::generate();
    let untrusted_pki = support::TestPki::generate();
    assert_rustls_handshake_rejected(
        server_pki.server_acceptor(),
        untrusted_pki.client_connector(),
        "localhost",
    )
    .await;
}

#[tokio::test]
async fn rustls_candidates_reject_a_missing_client_certificate_before_protocol_dispatch() {
    let pki = support::TestPki::generate();
    assert_rustls_handshake_rejected(
        pki.server_acceptor(),
        pki.client_connector_without_identity(),
        "localhost",
    )
    .await;
}

#[tokio::test]
async fn rustls_candidates_reject_hostname_time_eku_ca_and_protocol_policy_failures() {
    use support::{LeafPurpose, LeafValidity, PkiProfile, TestPki};

    let valid = TestPki::generate();
    assert_rustls_handshake_rejected(
        valid.server_acceptor(),
        valid.client_connector(),
        "wrong.example",
    )
    .await;

    for profile in [
        PkiProfile {
            server_validity: LeafValidity::Expired,
            ..PkiProfile::default()
        },
        PkiProfile {
            server_validity: LeafValidity::NotYetValid,
            ..PkiProfile::default()
        },
        PkiProfile {
            client_validity: LeafValidity::Expired,
            ..PkiProfile::default()
        },
        PkiProfile {
            client_validity: LeafValidity::NotYetValid,
            ..PkiProfile::default()
        },
        PkiProfile {
            server_purpose: LeafPurpose::Wrong,
            ..PkiProfile::default()
        },
        PkiProfile {
            client_purpose: LeafPurpose::Wrong,
            ..PkiProfile::default()
        },
    ] {
        let pki = TestPki::generate_with(profile);
        assert_rustls_handshake_rejected(
            pki.server_acceptor(),
            pki.client_connector(),
            "localhost",
        )
        .await;
    }

    let server_pki = TestPki::generate();
    let foreign_client = TestPki::generate();
    assert_rustls_handshake_rejected(
        server_pki.server_acceptor(),
        server_pki.client_connector_with_identity(&foreign_client),
        "localhost",
    )
    .await;

    let version_pki = TestPki::generate();
    assert_rustls_handshake_rejected(
        version_pki.server_acceptor_tls13_only(),
        version_pki.client_connector_tls12_only(),
        "localhost",
    )
    .await;

    let cipher_pki = TestPki::generate();
    assert_rustls_handshake_rejected(
        cipher_pki.server_acceptor_aes256_only(),
        cipher_pki.client_connector_aes128_only(),
        "localhost",
    )
    .await;
}

#[tokio::test]
async fn rustls_candidates_accept_ca_overlap_then_reject_retired_client_identity() {
    let old = support::TestPki::generate();
    let current = support::TestPki::generate();

    assert_rustls_handshake_accepted(
        current.server_acceptor_trusting(&[&old, &current]),
        current.client_connector_with_identity(&old),
    )
    .await;
    assert_rustls_handshake_rejected(
        current.server_acceptor(),
        current.client_connector_with_identity(&old),
        "localhost",
    )
    .await;
}
