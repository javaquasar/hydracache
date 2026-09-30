use std::sync::Arc;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn listener() -> Arc<RedisRespServer> {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let server = Arc::new(RedisRespServer::new(state, RedisListenerConfig::default()).unwrap());
    server.set_pipeline_instrumentation_enabled(true);
    server
}

#[tokio::test]
async fn deep_pipeline_reclaims_input_once_without_moving_suffixes() {
    let server = listener();
    let ping = b"*1\r\n$4\r\nPING\r\n";
    let request = ping.repeat(100);
    let (mut client, server_io) = tokio::io::duplex(64 * 1024);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });

    client.write_all(&request).await.unwrap();
    let mut output = vec![0; b"+PONG\r\n".len() * 100];
    client.read_exact(&mut output).await.unwrap();
    assert_eq!(output, b"+PONG\r\n".repeat(100));
    client.shutdown().await.unwrap();
    serve.await.unwrap();

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.decoded_commands, 100);
    assert_eq!(metrics.parser_consumed_bytes, request.len() as u64);
    assert_eq!(metrics.input_compactions, 0);
    assert_eq!(metrics.input_compaction_moved_bytes, 0);
}

#[tokio::test]
async fn partial_pipeline_moves_only_the_incomplete_suffix_once() {
    let server = listener();
    let ping = b"*1\r\n$4\r\nPING\r\n";
    let echo = b"*2\r\n$4\r\nECHO\r\n$5\r\nvalue\r\n";
    let split = 11;
    let mut first = ping.to_vec();
    first.extend_from_slice(&echo[..split]);
    let (mut client, server_io) = tokio::io::duplex(4096);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });

    client.write_all(&first).await.unwrap();
    let mut pong = [0; 7];
    client.read_exact(&mut pong).await.unwrap();
    assert_eq!(&pong, b"+PONG\r\n");
    client.write_all(&echo[split..]).await.unwrap();
    let mut echoed = [0; 11];
    client.read_exact(&mut echoed).await.unwrap();
    assert_eq!(&echoed, b"$5\r\nvalue\r\n");
    client.shutdown().await.unwrap();
    serve.await.unwrap();

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.decoded_commands, 2);
    assert_eq!(metrics.input_compactions, 1);
    assert_eq!(metrics.input_compaction_moved_bytes, split as u64);
}

#[tokio::test]
async fn every_single_frame_split_preserves_wire_semantics() {
    let command = b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n";
    for split in 1..command.len() {
        let server = listener();
        let (mut client, server_io) = tokio::io::duplex(4096);
        let owned = Arc::clone(&server);
        let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });
        client.write_all(&command[..split]).await.unwrap();
        tokio::task::yield_now().await;
        client.write_all(&command[split..]).await.unwrap();
        let mut output = [0; 5];
        client.read_exact(&mut output).await.unwrap();
        assert_eq!(&output, b"+OK\r\n", "split at byte {split}");
        client.shutdown().await.unwrap();
        serve.await.unwrap();
    }
}
