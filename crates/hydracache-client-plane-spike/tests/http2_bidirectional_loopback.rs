use bytes::Bytes;
use h2::{client, server};
use http::{Request, Response};
use hydracache_client_plane_spike::{
    FrameKind, PeerIdentity, ResourceSnapshot, SpikeConnection, SpikeFrame, SpikeLimits,
    TransportCandidate, HC2_GENERATION,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

fn ready_connection() -> SpikeConnection {
    let mut connection = SpikeConnection::new(
        TransportCandidate::Http2Bidirectional,
        SpikeLimits::default(),
    );
    connection
        .authenticate(PeerIdentity::verified("h2-client", "h2-tenant"))
        .unwrap();
    connection.negotiate(HC2_GENERATION).unwrap();
    connection
}

#[tokio::test]
async fn http2_candidate_interleaves_reply_and_event_on_a_real_stream() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (closed_tx, closed_rx) = oneshot::channel();

    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut h2 = server::handshake(socket).await.unwrap();
        let (request, mut respond) = h2.accept().await.expect("request stream").unwrap();
        let mut inbound = request.into_body();
        let request_bytes = inbound.data().await.expect("request DATA").unwrap();
        inbound
            .flow_control()
            .release_capacity(request_bytes.len())
            .unwrap();

        let mut connection = ready_connection();
        let invocation = connection.receive(&request_bytes).unwrap();
        connection
            .begin_invocation(invocation.correlation_id)
            .unwrap();
        connection.register_subscription(44).unwrap();
        connection
            .complete_invocation(invocation.correlation_id, b"h2-value".to_vec())
            .unwrap();
        connection
            .push_event(44, b"h2-entry-event".to_vec())
            .unwrap();

        let response = Response::builder().status(200).body(()).unwrap();
        let mut outbound = respond.send_response(response, false).unwrap();
        let reply = connection.drain_next().expect("reply frame");
        outbound
            .send_data(Bytes::from(connection.encode(&reply).unwrap()), false)
            .unwrap();
        let event = connection.drain_next().expect("event frame");
        outbound
            .send_data(Bytes::from(connection.encode(&event).unwrap()), true)
            .unwrap();

        h2.graceful_shutdown();
        while h2.accept().await.is_some() {}
        closed_tx.send(connection.disconnect()).unwrap();
    });

    let socket = TcpStream::connect(address).await.unwrap();
    let (mut client, h2_connection) = client::handshake(socket).await.unwrap();
    let driver = tokio::spawn(async move { h2_connection.await.unwrap() });
    let request = Request::builder()
        .method("POST")
        .uri("https://localhost/client/v2/stream")
        .body(())
        .unwrap();
    let (response, mut request_body) = client.send_request(request, false).unwrap();
    let codec = ready_connection();
    let invocation = codec
        .encode(&SpikeFrame::current(
            FrameKind::Invocation,
            91,
            b"get:key".to_vec(),
        ))
        .unwrap();
    request_body
        .send_data(Bytes::from(invocation), true)
        .unwrap();

    let response = response.await.unwrap();
    assert_eq!(response.status(), 200);
    let mut body = response.into_body();
    let reply_bytes = body.data().await.expect("reply DATA").unwrap();
    body.flow_control()
        .release_capacity(reply_bytes.len())
        .unwrap();
    let event_bytes = body.data().await.expect("event DATA").unwrap();
    body.flow_control()
        .release_capacity(event_bytes.len())
        .unwrap();

    let mut decoder = ready_connection();
    let reply = decoder.receive(&reply_bytes).unwrap();
    let event = decoder.receive(&event_bytes).unwrap();
    assert_eq!(reply.kind, FrameKind::Reply);
    assert_eq!(reply.correlation_id, 91);
    assert_eq!(event.kind, FrameKind::Event);
    assert_eq!(event.correlation_id, 44);

    server_task.await.unwrap();
    driver.await.unwrap();
    assert_eq!(closed_rx.await.unwrap(), ResourceSnapshot::default());
}
