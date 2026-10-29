use std::sync::Arc;
use std::time::Duration;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const PING: &[u8] = b"*1\r\n$4\r\nPING\r\n";
const QUIT: &[u8] = b"*1\r\n$4\r\nQUIT\r\n";

fn listener() -> Arc<RedisRespServer> {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let server = Arc::new(RedisRespServer::new(state, RedisListenerConfig::default()).unwrap());
    server.set_pipeline_instrumentation_enabled(true);
    server
}

#[tokio::test]
async fn sequential_pipeline_one_keeps_one_write_and_flush_per_reply() {
    let server = listener();
    let (mut client, server_io) = tokio::io::duplex(4096);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });

    for request in [PING, PING, QUIT] {
        client.write_all(request).await.unwrap();
        let expected: &[u8] = if request == QUIT {
            b"+OK\r\n"
        } else {
            b"+PONG\r\n"
        };
        let mut reply = vec![0; expected.len()];
        tokio::time::timeout(Duration::from_secs(1), client.read_exact(&mut reply))
            .await
            .expect("pipeline-one reply must not wait for a later request")
            .unwrap();
        assert_eq!(reply, expected);
    }
    serve.await.unwrap();

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.output_frames, 3);
    assert_eq!(metrics.write_calls, 3);
    assert_eq!(metrics.flush_calls, 3);
}

#[tokio::test]
async fn incomplete_second_frame_does_not_delay_the_first_reply() {
    let server = listener();
    let (mut client, server_io) = tokio::io::duplex(4096);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });

    client.write_all(PING).await.unwrap();
    client.write_all(&QUIT[..QUIT.len() - 2]).await.unwrap();
    let mut pong = [0; 7];
    tokio::time::timeout(Duration::from_secs(1), client.read_exact(&mut pong))
        .await
        .expect("a partial lookahead frame must not hold the completed reply")
        .unwrap();
    assert_eq!(&pong, b"+PONG\r\n");

    client.write_all(&QUIT[QUIT.len() - 2..]).await.unwrap();
    let mut ok = [0; 5];
    client.read_exact(&mut ok).await.unwrap();
    assert_eq!(&ok, b"+OK\r\n");
    serve.await.unwrap();

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.write_calls, 2);
    assert_eq!(metrics.flush_calls, 2);
}

#[tokio::test]
async fn deep_pipeline_retains_per_reply_writes_after_rejected_adaptive_candidate() {
    let server = listener();
    let mut request = Vec::new();
    let mut expected = Vec::new();
    for _ in 0..9 {
        request.extend_from_slice(PING);
        expected.extend_from_slice(b"+PONG\r\n");
    }
    request.extend_from_slice(QUIT);
    expected.extend_from_slice(b"+OK\r\n");

    let (mut client, server_io) = tokio::io::duplex(4096);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });
    client.write_all(&request).await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = Vec::new();
    client.read_to_end(&mut output).await.unwrap();
    serve.await.unwrap();
    assert_eq!(output, expected);

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.output_frames, 10);
    assert_eq!(metrics.write_calls, 10);
    assert_eq!(metrics.flush_calls, 10);
}
