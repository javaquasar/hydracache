use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer, RespDecodeLimits};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DuplexStream, ReadBuf};

fn listener() -> Arc<RedisRespServer> {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let server = Arc::new(RedisRespServer::new(state, RedisListenerConfig::default()).unwrap());
    server.set_pipeline_instrumentation_enabled(true);
    server
}

#[tokio::test]
async fn pipeline_replies_are_byte_identical_and_coalesced_per_read_batch() {
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
    assert_eq!(metrics.output_frames, 100);
    assert_eq!(metrics.output_bytes, output.len() as u64);
    assert_eq!(metrics.output_buffer_high_water_bytes, output.len() as u64);
    assert_eq!(metrics.write_calls, 1);
    assert_eq!(metrics.flush_calls, 1);
}

#[tokio::test]
async fn reply_quantum_flushes_and_then_makes_bounded_progress() {
    let server = listener();
    let request = b"*1\r\n$4\r\nPING\r\n".repeat(300);
    let (mut client, server_io) = tokio::io::duplex(64 * 1024);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });

    client.write_all(&request).await.unwrap();
    let mut output = vec![0; b"+PONG\r\n".len() * 300];
    client.read_exact(&mut output).await.unwrap();
    assert_eq!(output, b"+PONG\r\n".repeat(300));
    client.shutdown().await.unwrap();
    serve.await.unwrap();

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.output_frames, 300);
    assert_eq!(metrics.write_calls, 2);
    assert_eq!(metrics.flush_calls, 2);
    assert_eq!(
        metrics.output_buffer_high_water_bytes,
        (b"+PONG\r\n".len() * 256) as u64
    );
}

#[tokio::test]
async fn partial_socket_writes_do_not_split_high_level_batch_or_reorder_replies() {
    let server = listener();
    let set = b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n";
    let get = b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n";
    let request = [set.as_slice(), get.as_slice(), get.as_slice()].concat();
    let (mut client, server_io) = tokio::io::duplex(4096);
    let server_io = ChunkedIo {
        inner: server_io,
        max_write: 3,
    };
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });

    client.write_all(&request).await.unwrap();
    let expected = b"+OK\r\n$1\r\nv\r\n$1\r\nv\r\n";
    let mut output = vec![0; expected.len()];
    client.read_exact(&mut output).await.unwrap();
    assert_eq!(output, expected);
    client.shutdown().await.unwrap();
    serve.await.unwrap();

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.output_frames, 3);
    assert_eq!(metrics.write_calls, 1);
    assert_eq!(metrics.flush_calls, 1);
}

#[tokio::test]
async fn malformed_middle_frame_flushes_committed_reply_before_error() {
    let server = listener();
    let request = b"*1\r\n$4\r\nPING\r\n!invalid\r\n";
    let (mut client, server_io) = tokio::io::duplex(4096);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });

    client.write_all(request).await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = Vec::new();
    client.read_to_end(&mut output).await.unwrap();
    serve.await.unwrap();
    assert!(output.starts_with(b"+PONG\r\n-ERR "));

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.output_frames, 2);
    assert_eq!(metrics.write_calls, 2);
    assert_eq!(metrics.flush_calls, 2);
}

#[tokio::test]
async fn slow_reader_with_small_socket_buffer_eventually_receives_complete_batch() {
    let server = listener();
    let request = b"*1\r\n$4\r\nPING\r\n".repeat(100);
    let (client, server_io) = tokio::io::duplex(32);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });
    let (mut reader, mut writer) = tokio::io::split(client);
    let write_task = tokio::spawn(async move {
        writer.write_all(&request).await.unwrap();
        writer.shutdown().await.unwrap();
    });
    let read_task = tokio::spawn(async move {
        let mut output = Vec::new();
        let mut chunk = [0; 13];
        loop {
            let read = reader.read(&mut chunk).await.unwrap();
            if read == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..read]);
            tokio::task::yield_now().await;
        }
        output
    });

    write_task.await.unwrap();
    let output = read_task.await.unwrap();
    serve.await.unwrap();
    assert_eq!(output, b"+PONG\r\n".repeat(100));
    assert!(server.pipeline_metrics().write_calls < 100);
}

#[tokio::test]
async fn oversized_legal_reply_uses_bounded_one_item_progress() {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let server = Arc::new(
        RedisRespServer::new(
            state,
            RedisListenerConfig {
                decode_limits: RespDecodeLimits {
                    max_frame_bytes: 2 * 1024 * 1024,
                    max_bulk_string_bytes: 2 * 1024 * 1024,
                    ..RespDecodeLimits::default()
                },
                ..RedisListenerConfig::default()
            },
        )
        .unwrap(),
    );
    server.set_pipeline_instrumentation_enabled(true);
    let payload = vec![b'x'; 1024 * 1024 + 1];
    let mut request = format!("*2\r\n$4\r\nECHO\r\n${}\r\n", payload.len()).into_bytes();
    request.extend_from_slice(&payload);
    request.extend_from_slice(b"\r\n");
    let expected_prefix = format!("${}\r\n", payload.len()).into_bytes();
    let expected_len = expected_prefix.len() + payload.len() + 2;
    let (mut client, server_io) = tokio::io::duplex(3 * 1024 * 1024);
    let owned = Arc::clone(&server);
    let serve = tokio::spawn(async move { owned.serve_connection(server_io).await.unwrap() });

    client.write_all(&request).await.unwrap();
    let mut output = vec![0; expected_len];
    client.read_exact(&mut output).await.unwrap();
    assert!(output.starts_with(&expected_prefix));
    assert_eq!(&output[expected_prefix.len()..output.len() - 2], payload);
    assert!(output.ends_with(b"\r\n"));
    client.shutdown().await.unwrap();
    serve.await.unwrap();

    let metrics = server.pipeline_metrics();
    assert_eq!(metrics.output_frames, 1);
    assert_eq!(metrics.write_calls, 1);
    assert_eq!(metrics.flush_calls, 1);
    assert_eq!(metrics.output_buffer_high_water_bytes, expected_len as u64);
}

struct ChunkedIo {
    inner: DuplexStream,
    max_write: usize,
}

impl AsyncRead for ChunkedIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(context, buffer)
    }
}

impl AsyncWrite for ChunkedIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let length = bytes.len().min(self.max_write);
        Pin::new(&mut self.inner).poll_write(context, &bytes[..length])
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}
