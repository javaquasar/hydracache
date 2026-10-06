use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{
    ClientRequest, ClientRequestEnvelope, ClientResponse, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{
    translate_redis_command, RedisCommand, RedisListenerConfig, RedisRespServer,
    RedisTranslatedCommand, RedisTranslationContext, DEFAULT_REDIS_NAMESPACE,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

const TWO_SETS_AND_QUIT: &[u8] = b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n\
                                  *3\r\n$3\r\nSET\r\n$1\r\nb\r\n$1\r\n2\r\n\
                                  *1\r\n$4\r\nQUIT\r\n";

#[tokio::test]
async fn queued_set_nx_revalidates_after_intervening_native_invalidate() {
    for accepted_bytes in 0..=5 {
        let server = listener();
        let gate = Arc::new(WriteGate::closed());
        let output = Arc::new(Mutex::new(Vec::new()));
        let io = AdversarialIo::new(
            b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n\
              *6\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n2\r\n$2\r\nNX\r\n$2\r\nPX\r\n$4\r\n5000\r\n\
              *1\r\n$4\r\nQUIT\r\n",
            Arc::clone(&output),
        )
        .gate_after_bytes(accepted_bytes, Arc::clone(&gate));
        let serving = server.serve_connection(io);
        tokio::pin!(serving);
        assert!(futures_util::poll!(&mut serving).is_pending());
        assert_eq!(server.state().state_mutations(), 1);
        assert_eq!(native_get(&server, b"a"), Some(b"1".to_vec()));

        let (ns, key) = native_key(b"a");
        assert_eq!(
            native_dispatch(
                &server,
                "native-delete",
                ClientRequest::Invalidate { ns, key }
            ),
            ClientResponse::Invalidated
        );
        assert_eq!(native_get(&server, b"a"), None);
        assert_eq!(server.state().state_mutations(), 2);

        gate.open();
        tokio::time::timeout(Duration::from_secs(2), serving)
            .await
            .expect("released NX must finish")
            .unwrap();
        assert_eq!(&*output.lock().unwrap(), b"+OK\r\n+OK\r\n+OK\r\n");
        assert_eq!(native_get(&server, b"a"), Some(b"2".to_vec()));
        assert_eq!(server.state().state_mutations(), 3);
    }
}

#[tokio::test]
async fn queued_get_observes_intervening_native_put_without_blocking_native() {
    for accepted_bytes in 0..=5 {
        let server = listener();
        let gate = Arc::new(WriteGate::closed());
        let output = Arc::new(Mutex::new(Vec::new()));
        let io = AdversarialIo::new(
            b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\na\r\n\
              *1\r\n$4\r\nQUIT\r\n",
            Arc::clone(&output),
        )
        .gate_after_bytes(accepted_bytes, Arc::clone(&gate));
        let serving = server.serve_connection(io);
        tokio::pin!(serving);
        assert!(futures_util::poll!(&mut serving).is_pending());
        assert_eq!(native_get(&server, b"a"), Some(b"1".to_vec()));
        native_put(&server, b"a", b"N");
        assert_eq!(native_get(&server, b"a"), Some(b"N".to_vec()));

        gate.open();
        tokio::time::timeout(Duration::from_secs(2), serving)
            .await
            .expect("released GET must finish")
            .unwrap();
        assert_eq!(&*output.lock().unwrap(), b"+OK\r\n$1\r\nN\r\n+OK\r\n");
        assert_eq!(server.state().state_mutations(), 2);
    }
}

#[tokio::test]
async fn queued_set_nx_uses_expiry_and_ttl_at_its_execution_boundary() {
    for accepted_bytes in 0..=5 {
        let server = listener();
        server.state().set_cache_time_for_tests(Some(1_000));
        let gate = Arc::new(WriteGate::closed());
        let output = Arc::new(Mutex::new(Vec::new()));
        let io = AdversarialIo::new(
            b"*5\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n$2\r\nPX\r\n$2\r\n10\r\n\
              *6\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n2\r\n$2\r\nNX\r\n$2\r\nPX\r\n$2\r\n10\r\n\
              *1\r\n$4\r\nQUIT\r\n",
            Arc::clone(&output),
        )
        .gate_after_bytes(accepted_bytes, Arc::clone(&gate));
        let serving = server.serve_connection(io);
        tokio::pin!(serving);
        assert!(futures_util::poll!(&mut serving).is_pending());
        assert_eq!(native_get(&server, b"a"), Some(b"1".to_vec()));
        // Advance the injected clock to the exact first-key expiry. This is a
        // logical input change, not waiting on wall-clock time.
        server.state().advance_cache_time_for_tests(10);
        assert_eq!(native_get(&server, b"a"), None);

        gate.open();
        tokio::time::timeout(Duration::from_secs(2), serving)
            .await
            .expect("released expiring NX must finish")
            .unwrap();
        assert_eq!(&*output.lock().unwrap(), b"+OK\r\n+OK\r\n+OK\r\n");
        assert_eq!(native_get(&server, b"a"), Some(b"2".to_vec()));
        server.state().advance_cache_time_for_tests(9);
        assert_eq!(native_get(&server, b"a"), Some(b"2".to_vec()));
        server.state().advance_cache_time_for_tests(1);
        assert_eq!(native_get(&server, b"a"), None);
    }
}

#[tokio::test]
async fn queued_set_does_not_reserve_quota_ahead_of_intervening_native_put() {
    for accepted_bytes in 0..=5 {
        let roster = TenantRoster::new(vec![Tenant::new(DEFAULT_REDIS_NAMESPACE)
            .unwrap()
            .allow_client("redis-resp")
            .allow_client("native-client")
            .namespace(DEFAULT_REDIS_NAMESPACE, NamespaceQuota::new(2, 2))])
        .unwrap();
        let isolation = ConsumerIsolation::new(roster, ConsumerIsolationConfig::default());
        let state =
            ClientSurfaceState::with_isolation(ClientSurfaceLimits::default(), isolation).unwrap();
        let server = RedisRespServer::new(Arc::new(state), RedisListenerConfig::default()).unwrap();
        let gate = Arc::new(WriteGate::closed());
        let output = Arc::new(Mutex::new(Vec::new()));
        let io = AdversarialIo::new(
            b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n\
              *3\r\n$3\r\nSET\r\n$1\r\nc\r\n$1\r\n2\r\n\
              *1\r\n$4\r\nQUIT\r\n",
            Arc::clone(&output),
        )
        .gate_after_bytes(accepted_bytes, Arc::clone(&gate));
        let serving = server.serve_connection(io);
        tokio::pin!(serving);
        assert!(futures_util::poll!(&mut serving).is_pending());
        assert_eq!(server.state().state_mutations(), 1);
        assert!(server.state().audit_events_for_tests().is_empty());
        native_put(&server, b"b", b"N");
        assert_eq!(native_get(&server, b"b"), Some(b"N".to_vec()));
        assert_eq!(native_get(&server, b"c"), None);

        gate.open();
        tokio::time::timeout(Duration::from_secs(2), serving)
            .await
            .expect("quota-rejected pipeline must finish")
            .unwrap();
        assert_eq!(
            &*output.lock().unwrap(),
            b"+OK\r\n-ERR HydraCache client error: tenant quota exceeded\r\n+OK\r\n"
        );
        assert_eq!(server.state().state_mutations(), 2);
        assert_eq!(native_get(&server, b"a"), Some(b"1".to_vec()));
        assert_eq!(native_get(&server, b"b"), Some(b"N".to_vec()));
        assert_eq!(native_get(&server, b"c"), None);
        let audit = server.state().audit_events_for_tests();
        assert_eq!(audit.len(), 1);
        assert!(format!("{:?}", audit[0]).contains("QuotaRejected"));
        assert_eq!(
            server
                .state()
                .retained_state_for_diagnostics()
                .store_entries,
            2
        );
    }
}

#[tokio::test]
async fn every_partial_reply_and_pending_flush_preserve_the_mutation_frontier() {
    let first_reply = b"+OK\r\n";
    // Enumerate no progress, every partial prefix, and complete write with a
    // still-pending flush. Correctness is observed after a deterministic poll,
    // not after sleeping or racing a scheduler deadline.
    for accepted_bytes in 0..=first_reply.len() {
        let server = listener();
        let gate = Arc::new(WriteGate::closed());
        let output = Arc::new(Mutex::new(Vec::new()));
        let io = AdversarialIo::new(TWO_SETS_AND_QUIT, Arc::clone(&output))
            .write_chunk(1)
            .gate_after_bytes(accepted_bytes, Arc::clone(&gate));
        let serving = server.serve_connection(io);
        tokio::pin!(serving);

        assert!(futures_util::poll!(&mut serving).is_pending());
        assert_eq!(&*output.lock().unwrap(), &first_reply[..accepted_bytes]);
        assert_eq!(server.state().state_mutations(), 1, "cut {accepted_bytes}");
        assert_eq!(server.metrics().commands, 0, "cut {accepted_bytes}");

        let other = exchange(
            &server,
            b"*2\r\n$3\r\nGET\r\n$1\r\na\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\nb\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        assert_eq!(other, b"$1\r\n1\r\n$-1\r\n+OK\r\n");
        assert_eq!(server.state().state_mutations(), 1);

        gate.open();
        tokio::time::timeout(Duration::from_secs(2), serving)
            .await
            .expect("released frontier must finish")
            .expect("released connection must complete");
        assert_eq!(&*output.lock().unwrap(), b"+OK\r\n+OK\r\n+OK\r\n");
        assert_eq!(server.state().state_mutations(), 2);
    }
}

#[tokio::test]
async fn failed_flush_after_complete_reply_does_not_execute_the_next_set() {
    let server = listener();
    let gate = Arc::new(WriteGate::closed());
    let output = Arc::new(Mutex::new(Vec::new()));
    let io = AdversarialIo::new(TWO_SETS_AND_QUIT, Arc::clone(&output))
        .gate_after_bytes(b"+OK\r\n".len(), Arc::clone(&gate))
        .fail_flush();
    let serving = server.serve_connection(io);
    tokio::pin!(serving);

    assert!(futures_util::poll!(&mut serving).is_pending());
    assert_eq!(&*output.lock().unwrap(), b"+OK\r\n");
    assert_eq!(server.state().state_mutations(), 1);
    assert_eq!(server.metrics().commands, 0);
    gate.open();
    let error = serving.await.expect_err("flush failure must fail loudly");
    assert!(error.to_string().contains("scripted flush failure"));
    assert_eq!(&*output.lock().unwrap(), b"+OK\r\n");
    assert_eq!(server.state().state_mutations(), 1);
    assert_eq!(server.metrics().commands, 0);

    let other = exchange(
        &server,
        b"*2\r\n$3\r\nGET\r\n$1\r\na\r\n\
          *2\r\n$3\r\nGET\r\n$1\r\nb\r\n\
          *1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(other, b"$1\r\n1\r\n$-1\r\n+OK\r\n");
}

#[tokio::test]
async fn fragmented_frames_short_writes_and_pending_flush_preserve_pipeline() {
    let server = listener();
    let input = b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
                  *2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
                  *1\r\n$4\r\nQUIT\r\n";
    let output = Arc::new(Mutex::new(Vec::new()));
    let io = AdversarialIo::new(input, Arc::clone(&output))
        .read_chunk(1)
        .write_chunk(2)
        .pending_writes(7)
        .pending_flushes(3);

    tokio::time::timeout(Duration::from_secs(2), server.serve_connection(io))
        .await
        .expect("fragmented connection must not hang")
        .expect("fragmented connection must complete");

    assert_eq!(&*output.lock().unwrap(), b"+OK\r\n$1\r\nv\r\n+OK\r\n");
    assert_eq!(server.metrics().commands, 3);
}

#[tokio::test]
async fn closed_write_gate_backpressures_one_connection_without_starving_another() {
    let server = Arc::new(listener());
    let gate = Arc::new(WriteGate::closed());
    let output = Arc::new(Mutex::new(Vec::new()));
    let input = b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n\
                  *3\r\n$3\r\nSET\r\n$1\r\nb\r\n$1\r\n2\r\n\
                  *1\r\n$4\r\nQUIT\r\n";
    let io = AdversarialIo::new(input, Arc::clone(&output)).gate(Arc::clone(&gate));
    let blocked_server = Arc::clone(&server);
    let blocked = tokio::spawn(async move { blocked_server.serve_connection(io).await });

    tokio::time::timeout(Duration::from_secs(1), async {
        while server.state().state_mutations() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first mutation should reach the blocked response boundary");
    assert_eq!(server.state().state_mutations(), 1);
    assert_eq!(server.metrics().commands, 0);

    let other = exchange(&server, b"*1\r\n$4\r\nPING\r\n*1\r\n$4\r\nQUIT\r\n").await;
    assert_eq!(other, b"+PONG\r\n+OK\r\n");

    gate.open();
    tokio::time::timeout(Duration::from_secs(2), blocked)
        .await
        .expect("released slow reader must finish")
        .expect("blocked task must not panic")
        .expect("released connection must complete");
    assert_eq!(&*output.lock().unwrap(), b"+OK\r\n+OK\r\n+OK\r\n");
    assert_eq!(server.state().state_mutations(), 2);
}

#[tokio::test]
async fn disconnect_during_partial_reply_does_not_poison_committed_state_or_reconnect() {
    let server = listener();
    let output = Arc::new(Mutex::new(Vec::new()));
    let input = b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n";
    let io = AdversarialIo::new(input, Arc::clone(&output))
        .write_chunk(1)
        .disconnect_after(2);

    let result = tokio::time::timeout(Duration::from_secs(1), server.serve_connection(io))
        .await
        .expect("disconnect must be observed promptly");
    assert!(result.is_err(), "partial reply disconnect must fail loudly");
    assert_eq!(server.state().state_mutations(), 1);
    assert_eq!(&*output.lock().unwrap(), b"+O");

    let retry = exchange(
        &server,
        b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n*1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(retry, b"$1\r\nv\r\n+OK\r\n");
}

#[tokio::test]
async fn large_reply_survives_repeated_pending_and_one_byte_writes() {
    let server = listener();
    let payload = vec![b'x'; 64 * 1024];
    let mut input = format!("*3\r\n$3\r\nSET\r\n$3\r\nbig\r\n${}\r\n", payload.len()).into_bytes();
    input.extend_from_slice(&payload);
    input.extend_from_slice(b"\r\n*2\r\n$3\r\nGET\r\n$3\r\nbig\r\n*1\r\n$4\r\nQUIT\r\n");
    let output = Arc::new(Mutex::new(Vec::new()));
    let io = AdversarialIo::new(&input, Arc::clone(&output))
        .read_chunk(127)
        .write_chunk(1)
        .pending_writes(11)
        .pending_flushes(5);

    tokio::time::timeout(Duration::from_secs(5), server.serve_connection(io))
        .await
        .expect("large short-write response must not hang")
        .expect("large short-write response must complete");
    let output = output.lock().unwrap();
    assert!(output.starts_with(b"+OK\r\n$65536\r\n"));
    assert!(output.ends_with(b"\r\n+OK\r\n"));
    assert_eq!(server.state().state_mutations(), 1);
}

fn listener() -> RedisRespServer {
    RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
        RedisListenerConfig::default(),
    )
    .unwrap()
}

// Translate only to obtain the existing binary-safe identity. All interleaving
// operations themselves use the supported direct native dispatch seam, not RESP
// execution or private store access.
fn native_key(key: &[u8]) -> (Namespace, StructuredKey) {
    let RedisTranslatedCommand::Execute(plan) = translate_redis_command(
        RedisCommand::Get { key: key.to_vec() },
        &RedisTranslationContext::new(DEFAULT_REDIS_NAMESPACE, "native-key").unwrap(),
    )
    .unwrap() else {
        panic!("GET must translate to an execution plan");
    };
    let ClientRequest::Get { ns, key } = &plan.initial_requests()[0].request else {
        panic!("GET must translate to a native GET");
    };
    (ns.clone(), key.clone())
}

fn native_dispatch(server: &RedisRespServer, id: &str, request: ClientRequest) -> ClientResponse {
    server
        .state()
        .dispatch_verified_request(
            &ClientIdentity::new("native-client", DEFAULT_REDIS_NAMESPACE).unwrap(),
            ClientRequestEnvelope::new(id, request),
        )
        .result
        .expect("intervening native operation must make progress")
}

fn native_get(server: &RedisRespServer, key: &[u8]) -> Option<Vec<u8>> {
    let (ns, key) = native_key(key);
    let ClientResponse::Value { value } =
        native_dispatch(server, "native-get", ClientRequest::Get { ns, key })
    else {
        panic!("native GET must return Value");
    };
    value
}

fn native_put(server: &RedisRespServer, key: &[u8], value: &[u8]) {
    let (ns, key) = native_key(key);
    assert_eq!(
        native_dispatch(
            server,
            "native-put",
            ClientRequest::Put {
                ns,
                key,
                value: value.to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ),
        ClientResponse::Stored
    );
}

async fn exchange(server: &RedisRespServer, input: &'static [u8]) -> Vec<u8> {
    let (mut client, server_io) = tokio::io::duplex(4096);
    let serve = async { server.serve_connection(server_io).await.unwrap() };
    let client = async {
        client.write_all(input).await.unwrap();
        let mut output = Vec::new();
        client.read_to_end(&mut output).await.unwrap();
        output
    };
    let (_, output) = tokio::join!(serve, client);
    output
}

struct WriteGate {
    open: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

impl WriteGate {
    fn closed() -> Self {
        Self {
            open: AtomicBool::new(false),
            waker: Mutex::new(None),
        }
    }

    fn open(&self) {
        self.open.store(true, Ordering::Release);
        if let Some(waker) = self.waker.lock().unwrap().take() {
            waker.wake();
        }
    }

    fn poll(&self, cx: &Context<'_>) -> bool {
        if self.open.load(Ordering::Acquire) {
            true
        } else {
            *self.waker.lock().unwrap() = Some(cx.waker().clone());
            false
        }
    }
}

struct AdversarialIo {
    input: Vec<u8>,
    read_offset: usize,
    read_chunk: usize,
    output: Arc<Mutex<Vec<u8>>>,
    write_chunk: usize,
    pending_writes: usize,
    pending_flushes: usize,
    disconnect_after: Option<usize>,
    gate: Option<Arc<WriteGate>>,
    gate_after_write_bytes: usize,
    fail_flush: bool,
}

impl AdversarialIo {
    fn new(input: &[u8], output: Arc<Mutex<Vec<u8>>>) -> Self {
        Self {
            input: input.to_vec(),
            read_offset: 0,
            read_chunk: usize::MAX,
            output,
            write_chunk: usize::MAX,
            pending_writes: 0,
            pending_flushes: 0,
            disconnect_after: None,
            gate: None,
            gate_after_write_bytes: 0,
            fail_flush: false,
        }
    }

    fn read_chunk(mut self, bytes: usize) -> Self {
        self.read_chunk = bytes;
        self
    }

    fn write_chunk(mut self, bytes: usize) -> Self {
        self.write_chunk = bytes;
        self
    }

    fn pending_writes(mut self, polls: usize) -> Self {
        self.pending_writes = polls;
        self
    }

    fn pending_flushes(mut self, polls: usize) -> Self {
        self.pending_flushes = polls;
        self
    }

    fn disconnect_after(mut self, bytes: usize) -> Self {
        self.disconnect_after = Some(bytes);
        self
    }

    fn gate(mut self, gate: Arc<WriteGate>) -> Self {
        self.gate = Some(gate);
        self
    }

    fn gate_after_bytes(mut self, bytes: usize, gate: Arc<WriteGate>) -> Self {
        self.gate_after_write_bytes = bytes;
        self.gate = Some(gate);
        self
    }

    fn fail_flush(mut self) -> Self {
        self.fail_flush = true;
        self
    }
}

impl AsyncRead for AdversarialIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.read_offset == self.input.len() {
            return Poll::Ready(Ok(()));
        }
        let available = self.input.len() - self.read_offset;
        let count = available.min(self.read_chunk).min(buffer.remaining());
        let end = self.read_offset + count;
        buffer.put_slice(&self.input[self.read_offset..end]);
        self.read_offset = end;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for AdversarialIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let written = self.output.lock().unwrap().len();
        let remaining_before_gate = if self.gate.as_ref().is_some_and(|gate| !gate.poll(cx)) {
            if written >= self.gate_after_write_bytes {
                return Poll::Pending;
            }
            self.gate_after_write_bytes - written
        } else {
            usize::MAX
        };
        if self.pending_writes != 0 {
            self.pending_writes -= 1;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        if self.disconnect_after.is_some_and(|limit| written >= limit) {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "scripted disconnect",
            )));
        }
        let remaining_before_disconnect = self
            .disconnect_after
            .map_or(usize::MAX, |limit| limit.saturating_sub(written));
        let count = bytes
            .len()
            .min(self.write_chunk)
            .min(remaining_before_gate)
            .min(remaining_before_disconnect);
        if count == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "scripted zero write",
            )));
        }
        self.output
            .lock()
            .unwrap()
            .extend_from_slice(&bytes[..count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.gate.as_ref().is_some_and(|gate| !gate.poll(cx)) {
            return Poll::Pending;
        }
        if self.fail_flush {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "scripted flush failure",
            )));
        }
        if self.pending_flushes != 0 {
            self.pending_flushes -= 1;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
