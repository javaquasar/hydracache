use bytes::Bytes;
use h2::{client, server};
use http::{Request, Response};
use hydracache_client_plane_spike::{
    BootstrapConnection, ConnectionGeneration, FrameKind, PeerIdentity, ResourceSnapshot,
    SpikeConnection, SpikeFrame, SpikeLimits, TransportCandidate, HC2_GENERATION,
};
use rustls::pki_types::ServerName;
use std::collections::BTreeSet;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

mod support;

const CONNECTION_GENERATION: ConnectionGeneration = ConnectionGeneration::FIRST;

fn ready_connection() -> SpikeConnection {
    BootstrapConnection::new(
        TransportCandidate::Http2Bidirectional,
        CONNECTION_GENERATION,
        SpikeLimits::default(),
    )
    .verify_tls(true)
    .unwrap()
    .authenticate(PeerIdentity::verified("h2-client", "h2-tenant"))
    .unwrap()
    .negotiate(HC2_GENERATION)
    .unwrap()
}

#[tokio::test]
async fn http2_candidate_interleaves_reply_and_event_on_a_real_stream() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (closed_tx, closed_rx) = oneshot::channel();
    let pki = support::TestPki::generate();
    let acceptor = pki.server_acceptor();
    let connector = pki.client_connector();

    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let socket = acceptor.accept(socket).await.expect("mTLS handshake");
        let mut h2 = server::handshake(socket).await.unwrap();
        let (request, mut respond) = h2.accept().await.expect("request stream").unwrap();
        let mut inbound = request.into_body();
        let mut request_bytes = Vec::new();
        while let Some(chunk) = inbound.data().await {
            let chunk = chunk.unwrap();
            inbound
                .flow_control()
                .release_capacity(chunk.len())
                .unwrap();
            request_bytes.extend_from_slice(&chunk);
        }

        let mut connection = ready_connection();
        let codec = ready_connection();
        let mut cursor = request_bytes.as_slice();
        for expected in 1..=256 {
            let frame_len = 8 + u32::from_be_bytes(cursor[4..8].try_into().unwrap()) as usize;
            let invocation = connection.receive(&cursor[..frame_len]).unwrap();
            assert_eq!(invocation.correlation_id, expected);
            connection
                .begin_invocation(invocation.correlation_id)
                .unwrap();
            cursor = &cursor[frame_len..];
        }
        assert!(cursor.is_empty(), "no trailing request bytes");
        connection.register_subscription(44).unwrap();
        for correlation_id in 1..=256 {
            connection
                .complete_invocation(CONNECTION_GENERATION, correlation_id, b"h2-value".to_vec())
                .unwrap();
        }
        connection.heartbeat(CONNECTION_GENERATION).unwrap();
        connection
            .push_event(CONNECTION_GENERATION, 44, b"h2-entry-event".to_vec())
            .unwrap();

        let response = Response::builder().status(200).body(()).unwrap();
        let mut outbound = respond.send_response(response, false).unwrap();
        let mut response_bytes = Vec::new();
        while let Some(frame) = connection.drain_next() {
            response_bytes.extend_from_slice(&codec.encode(&frame).unwrap());
        }
        outbound
            .send_data(Bytes::from(response_bytes), true)
            .unwrap();
        drop(outbound);
        drop(respond);
        drop(inbound);

        closed_tx.send(connection.disconnect()).unwrap();
        h2.graceful_shutdown();
        while h2.accept().await.is_some() {}
    });

    let socket = TcpStream::connect(address).await.unwrap();
    let socket = connector
        .connect(ServerName::try_from("localhost").unwrap(), socket)
        .await
        .expect("verified server and client certificates");
    let (mut client, h2_connection) = client::handshake(socket).await.unwrap();
    let driver = tokio::spawn(h2_connection);
    let request = Request::builder()
        .method("POST")
        .uri("https://localhost/client/v2/stream")
        .body(())
        .unwrap();
    let (response, mut request_body) = client.send_request(request, false).unwrap();
    let codec = ready_connection();
    let mut invocations = Vec::new();
    for correlation_id in 1..=256 {
        invocations.extend_from_slice(
            &codec
                .encode(&SpikeFrame::current(
                    CONNECTION_GENERATION,
                    FrameKind::Invocation,
                    correlation_id,
                    b"get:key".to_vec(),
                ))
                .unwrap(),
        );
    }
    request_body
        .send_data(Bytes::from(invocations), true)
        .unwrap();

    let response = response.await.unwrap();
    assert_eq!(response.status(), 200);
    let mut body = response.into_body();
    let mut response_bytes = Vec::new();
    while let Some(chunk) = body.data().await {
        let chunk = chunk.unwrap();
        body.flow_control().release_capacity(chunk.len()).unwrap();
        response_bytes.extend_from_slice(&chunk);
    }

    let mut decoder = ready_connection();
    let mut replies = BTreeSet::new();
    let mut saw_heartbeat = false;
    let mut saw_event = false;
    let mut cursor = response_bytes.as_slice();
    while !cursor.is_empty() {
        let frame_len = 8 + u32::from_be_bytes(cursor[4..8].try_into().unwrap()) as usize;
        let frame = decoder.receive(&cursor[..frame_len]).unwrap();
        cursor = &cursor[frame_len..];
        match frame.kind {
            FrameKind::Reply => {
                replies.insert(frame.correlation_id);
            }
            FrameKind::Heartbeat => saw_heartbeat = true,
            FrameKind::Event => {
                assert_eq!(frame.correlation_id, 44);
                saw_event = true;
            }
            other => panic!("unexpected server frame: {other:?}"),
        }
    }
    assert_eq!(replies, (1..=256).collect());
    assert!(saw_heartbeat);
    assert!(saw_event);

    drop(request_body);
    drop(body);
    drop(client);
    assert_eq!(closed_rx.await.unwrap(), ResourceSnapshot::default());
    server_task.abort();
    driver.abort();
    let _ = server_task.await;
    let _ = driver.await;
}
