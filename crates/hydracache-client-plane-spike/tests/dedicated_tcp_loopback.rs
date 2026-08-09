use std::collections::BTreeSet;
use std::time::Duration;

use hydracache_client_plane_spike::{
    FrameKind, PeerIdentity, ResourceSnapshot, SpikeConnection, SpikeFrame, SpikeLimits,
    TransportCandidate, HC2_GENERATION,
};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

mod support;

const PREFIX_BYTES: usize = 8;

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
    let mut connection =
        SpikeConnection::new(TransportCandidate::DedicatedTcpTls, SpikeLimits::default());
    connection
        .authenticate(PeerIdentity::verified("loopback-client", "loopback-tenant"))
        .unwrap();
    connection.negotiate(HC2_GENERATION).unwrap();
    connection
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
                .complete_invocation(correlation_id, b"value".to_vec())
                .expect("completion");
        }
        connection.heartbeat().unwrap();
        connection
            .push_event(44, b"tcp-entry-event".to_vec())
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
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        acceptor.accept(socket).await
    });
    let socket = TcpStream::connect(address).await.unwrap();
    let client = connector
        .connect(ServerName::try_from("localhost").unwrap(), socket)
        .await;
    let server = server.await.unwrap();
    assert!(
        server.is_err(),
        "server unexpectedly accepted invalid TLS identity"
    );
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
}

#[tokio::test]
async fn rustls_candidates_reject_an_untrusted_ca_before_protocol_dispatch() {
    let server_pki = support::TestPki::generate();
    let untrusted_pki = support::TestPki::generate();
    assert_rustls_handshake_rejected(
        server_pki.server_acceptor(),
        untrusted_pki.client_connector(),
    )
    .await;
}

#[tokio::test]
async fn rustls_candidates_reject_a_missing_client_certificate_before_protocol_dispatch() {
    let pki = support::TestPki::generate();
    assert_rustls_handshake_rejected(
        pki.server_acceptor(),
        pki.client_connector_without_identity(),
    )
    .await;
}
