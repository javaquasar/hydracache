use std::sync::Arc;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisPipelineMetrics, RedisRespServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn listener() -> Arc<RedisRespServer> {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    Arc::new(RedisRespServer::new(state, RedisListenerConfig::default()).unwrap())
}

async fn exchange(server: &Arc<RedisRespServer>, request: &[u8]) -> Vec<u8> {
    let (mut client, server_io) = tokio::io::duplex(16 * 1024);
    client.write_all(request).await.unwrap();
    client.shutdown().await.unwrap();
    let owned = Arc::clone(server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });
    let mut output = Vec::new();
    client.read_to_end(&mut output).await.unwrap();
    serve.await.unwrap();
    output
}

#[tokio::test]
async fn pipeline_counters_reconcile_compaction_and_coalesced_flush() {
    let server = listener();
    server.set_pipeline_instrumentation_enabled(true);
    let set = b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n";
    let get = b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n";
    let quit = b"*1\r\n$4\r\nQUIT\r\n";
    let request = [set.as_slice(), get.as_slice(), quit.as_slice()].concat();

    let output = exchange(&server, &request).await;
    assert_eq!(output, b"+OK\r\n$1\r\nv\r\n+OK\r\n");

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.read_calls, 1);
    assert_eq!(metrics.input_bytes, request.len() as u64);
    assert_eq!(metrics.input_buffer_high_water_bytes, request.len() as u64);
    assert_eq!(metrics.decoded_commands, 3);
    assert_eq!(metrics.parser_consumed_bytes, request.len() as u64);
    assert_eq!(metrics.input_compactions, 3);
    assert_eq!(
        metrics.input_compaction_moved_bytes,
        (get.len() + quit.len() + quit.len()) as u64
    );
    assert_eq!(metrics.translation_contexts, 3);
    assert!(metrics.request_id_bytes > 0);
    assert_eq!(metrics.script_cache_entries_cloned, 0);
    assert_eq!(metrics.output_frames, 3);
    assert_eq!(metrics.output_bytes, output.len() as u64);
    assert_eq!(metrics.output_buffer_high_water_bytes, output.len() as u64);
    assert_eq!(metrics.write_calls, 1);
    assert_eq!(metrics.flush_calls, 1);
}

#[tokio::test]
async fn pipeline_counters_are_off_by_default_and_reset_only_when_requested() {
    let server = listener();
    let request = b"*1\r\n$4\r\nPING\r\n*1\r\n$4\r\nQUIT\r\n";
    assert_eq!(exchange(&server, request).await, b"+PONG\r\n+OK\r\n");
    assert_eq!(server.pipeline_metrics(), RedisPipelineMetrics::default());

    server.set_pipeline_instrumentation_enabled(true);
    assert_eq!(exchange(&server, request).await, b"+PONG\r\n+OK\r\n");
    assert_eq!(server.pipeline_metrics().decoded_commands, 2);

    server.set_pipeline_instrumentation_enabled(false);
    server.reset_pipeline_metrics();
    assert_eq!(server.pipeline_metrics(), RedisPipelineMetrics::default());
}
