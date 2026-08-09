use hydracache_client_plane_spike::{
    FrameKind, PeerIdentity, ResourceSnapshot, SpikeConnection, SpikeFrame, SpikeLimits,
    TransportCandidate, HC2_GENERATION,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

const PREFIX_BYTES: usize = 8;

async fn read_exact_frame(stream: &mut TcpStream, max: usize) -> Vec<u8> {
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

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut connection = ready_connection();
        let bytes = read_exact_frame(&mut socket, SpikeLimits::default().max_frame_bytes).await;
        let request = connection.receive(&bytes).expect("strict frame accepted");
        assert_eq!(request.kind, FrameKind::Invocation);
        connection
            .begin_invocation(request.correlation_id)
            .expect("pending invocation");
        connection
            .complete_invocation(request.correlation_id, b"value".to_vec())
            .expect("completion");
        let reply = connection.drain_next().expect("queued reply");
        let encoded = connection.encode(&reply).expect("encoded reply");
        socket.write_all(&encoded).await.expect("reply write");
        socket.shutdown().await.expect("server half-close");
        closed_tx
            .send(connection.disconnect())
            .expect("resource snapshot receiver");
    });

    let mut client = TcpStream::connect(address).await.unwrap();
    let client_codec = ready_connection();
    let request = client_codec
        .encode(&SpikeFrame::current(
            FrameKind::Invocation,
            19,
            b"get:key".to_vec(),
        ))
        .unwrap();
    client.write_all(&request).await.unwrap();
    let reply = read_exact_frame(&mut client, SpikeLimits::default().max_frame_bytes).await;
    let mut decoder = ready_connection();
    let reply = decoder.receive(&reply).expect("reply decoded");
    assert_eq!(reply.kind, FrameKind::Reply);
    assert_eq!(reply.correlation_id, 19);
    assert_eq!(reply.payload, b"value");

    server.await.unwrap();
    assert_eq!(closed_rx.await.unwrap(), ResourceSnapshot::default());
}
