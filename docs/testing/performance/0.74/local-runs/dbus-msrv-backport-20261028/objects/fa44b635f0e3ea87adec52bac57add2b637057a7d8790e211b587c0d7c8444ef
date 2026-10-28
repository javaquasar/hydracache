//! What a D-Bus connection costs on this runtime, with the protocol taken out: its setup and
//! teardown, a method call's round trip, a large body, a signal, and a burst of 1000 concurrent
//! calls to a peer on another thread, over an in-process channel and over a unix socket pair.
//! These were zbus's connection benchmarks, whose cost is mostly the runtime's, and they keep
//! zbus's ids.
//!
//! Instead of zbus, which depends on this crate, a connection's traffic is recreated here: the
//! same tasks, wakes, timers and I/O. A connection writes framed messages through a writer behind
//! an async lock; a socket reader task routes each reply to the call waiting on its serial and
//! hands each call to an object server task, which spawns and detaches a task per call; every
//! call races its reply against a 30-second timeout, as a zbus method call does; building a pair
//! over a socket takes a handshake of two round trips, as zbus's peer-to-peer one does; and
//! shutting a connection down gracefully drops it and waits for it to be gone.
//!
//! Left out are D-Bus's marshalling (a message here is a small fixed header and a body of raw
//! bytes), the handshake's actual text, and match rules (a connection has one signal queue, and
//! only one subscriber is ever exercised).
//!
//! The socket transport and every id built on it are unix-only; `method-call/1000-concurrent-p2p`
//! needs only the in-process channel and runs everywhere.

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    hint::black_box,
    io,
    pin::{Pin, pin},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

use cpus::Cpus;
use criterion::{
    BenchmarkGroup, Criterion, Throughput, async_executor::AsyncExecutor, criterion_group,
    criterion_main, measurement::Measurement,
};
use futures::future::{self, try_join_all};
use zruntime::{Event, Shared, SharedRuntime, Task, lock};

#[path = "common/cpus.rs"]
mod cpus;

/// Runs a routine's future to completion inside `zruntime::block_on`.
struct ZruntimeExecutor;

impl AsyncExecutor for ZruntimeExecutor {
    fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        zruntime::block_on(future)
    }
}

/// How long a call waits for its reply before giving up, matching the `method_timeout` zbus's
/// own benchmark pairs are built with.
const METHOD_TIMEOUT: Duration = Duration::from_secs(30);

/// The `Ping` method id: the call body is `[METHOD_PING, value: u32 LE]`, the reply body is the
/// same four bytes.
const METHOD_PING: u8 = 0;
/// The `Echo` method id: the call body is `[METHOD_ECHO, ..payload]`, the reply body is the
/// payload unchanged.
const METHOD_ECHO: u8 = 1;

/// Computes a call's reply body synchronously, as the methods of the interface zbus's
/// benchmarks served are; only the task that runs this and the write of the reply are async.
fn dispatch(body: &[u8]) -> Vec<u8> {
    match body.first().copied() {
        Some(METHOD_PING) => body[1..5].to_vec(),
        Some(METHOD_ECHO) => body[1..].to_vec(),
        _ => Vec::new(),
    }
}

/// A server's per-call dispatch function. A plain function pointer, since the two methods above
/// need no captured state; `Copy` makes it cheap to hand to every dispatched task.
type Handler = fn(&[u8]) -> Vec<u8>;

async fn ping(connection: &Connection, value: u32) -> io::Result<u32> {
    let mut body = Vec::with_capacity(5);
    body.push(METHOD_PING);
    body.extend_from_slice(&value.to_le_bytes());
    let reply = connection.call(body).await?;

    Ok(u32::from_le_bytes(reply.body[..4].try_into().unwrap()))
}

/// Only the unix `method-call/1MiB-body` bench calls this.
#[cfg(unix)]
async fn echo(connection: &Connection, payload: &[u8]) -> io::Result<Vec<u8>> {
    let mut body = Vec::with_capacity(1 + payload.len());
    body.push(METHOD_ECHO);
    body.extend_from_slice(payload);

    Ok(connection.call(body).await?.body)
}

/// How many calls `method-call/1000-concurrent-p2p*` has in flight at once.
const CONCURRENT_METHOD_CALLS: usize = 1000;

/// Awaits every one of `CONCURRENT_METHOD_CALLS` `Ping` calls together, through
/// `futures::future::try_join_all`.
async fn call_ping_concurrently(client: &Connection) {
    let replies =
        try_join_all((0..CONCURRENT_METHOD_CALLS as u32).map(|value| ping(client, value)))
            .await
            .unwrap();
    black_box(replies);
}

/// What building a connection's transport hands back: a reader and a writer, already past
/// whatever handshake that transport needs.
type BuildFuture = Pin<Box<dyn Future<Output = (BoxedReader, BoxedWriter)>>>;

/// Builds the server on a thread of its own and the client on the caller's, benches one burst of
/// concurrent calls per iteration, then drops the client and joins the server thread.
/// `build_server` and `build_client` build either end over any transport, behind
/// [`BoxedReader`]/[`BoxedWriter`].
///
/// The server's `block_on` runs until the client hangs up, which is when [`Connection::closed`]
/// resolves. The two threads are pinned to a CPU each until then, so that every process measures
/// the same placement of the two (see `common/cpus.rs`).
fn run_burst_bench<M>(
    group: &mut BenchmarkGroup<'_, M>,
    id: &str,
    build_server: impl FnOnce(&SharedRuntime) -> BuildFuture + Send + 'static,
    build_client: impl FnOnce(&SharedRuntime) -> BuildFuture,
) where
    M: Measurement,
{
    let cpus = Cpus::allowed();
    let server_cpu = cpus.nth(1);
    let _pinned = cpus.nth(0).pin();

    let server_thread = thread::spawn(move || {
        let _pinned = server_cpu.pin();
        zruntime::block_on(async move {
            let runtime = SharedRuntime::current().expect("a runtime for this thread");
            let (reader, writer) = build_server(&runtime).await;
            let connection = build_connection(reader, writer, &runtime, Some(dispatch));
            connection.closed().await;
        })
    });
    let client = zruntime::block_on(async {
        let runtime = SharedRuntime::current().expect("a runtime for this thread");
        let (reader, writer) = build_client(&runtime).await;

        build_connection(reader, writer, &runtime, None)
    });

    group.bench_function(id, |b| {
        b.to_async(ZruntimeExecutor)
            .iter(|| call_ping_concurrently(&client));
    });

    drop(client);
    server_thread.join().unwrap();
}

/// How many frames a channel pair buffers before its writer waits, matching the capacity of
/// zbus's own in-process `Channel` transport (`CHANNEL_CAPACITY` in
/// `zbus/src/connection/socket/channel.rs`).
const CHANNEL_CAPACITY: usize = 32;

fn concurrent_channel_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("method-call");
    group.sample_size(10);
    group.throughput(Throughput::Elements(CONCURRENT_METHOD_CALLS as u64));

    // Both ends of an in-process channel, with no handshake: zbus's benchmark built these
    // already authenticated.
    let client_to_server = Arc::new(Queue::bounded(CHANNEL_CAPACITY));
    let server_to_client = Arc::new(Queue::bounded(CHANNEL_CAPACITY));
    let (server_reads, server_writes) = (client_to_server.clone(), server_to_client.clone());

    run_burst_bench(
        &mut group,
        "1000-concurrent-p2p",
        move |_runtime| {
            let reader: BoxedReader = Box::new(ChannelReader(server_reads));
            let writer: BoxedWriter = Box::new(ChannelWriter(server_writes));

            Box::pin(std::future::ready((reader, writer)))
        },
        move |_runtime| {
            let reader: BoxedReader = Box::new(ChannelReader(server_to_client));
            let writer: BoxedWriter = Box::new(ChannelWriter(client_to_server));

            Box::pin(std::future::ready((reader, writer)))
        },
    );

    group.finish();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameKind {
    Call,
    Reply,
    // Only ever constructed by `Connection::emit_signal`, which — like the rest of signal
    // support below — only the unix `signal/emit-receive` bench exercises; see the note on
    // `Connection::signals`.
    #[cfg(unix)]
    Signal,
}

#[derive(Clone, Debug)]
struct Frame {
    kind: FrameKind,
    serial: u32,
    reply_serial: u32,
    body: Vec<u8>,
}

/// One end of a transport's read side. Implemented for a unix socket end and for a channel
/// queue; boxed as [`BoxedReader`] so [`Connection`] is the same type over either.
trait FrameReader: Send + 'static {
    /// Reads one frame, or `None` once the peer has gone away.
    fn read_frame(&self) -> Pin<Box<dyn Future<Output = Option<Frame>> + Send + '_>>;
}

/// One end of a transport's write side. See [`FrameReader`].
trait FrameWriter: Send + 'static {
    /// Writes one frame to the peer.
    fn write_frame<'a>(
        &'a self,
        frame: &'a Frame,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + 'a>>;
}

type BoxedReader = Box<dyn FrameReader>;
type BoxedWriter = Box<dyn FrameWriter>;

/// A small in-process queue: unbounded where it is just a handoff between this bench's own
/// tasks (a connection's signals and, on a server, its incoming calls), and bounded where it
/// stands in for a transport (`channel_pair`'s in-process channel), so that a full queue
/// backpressures its writer exactly as zbus's own `Channel`, built on
/// `zruntime::broadcast`, does.
struct Queue<T> {
    items: Mutex<VecDeque<T>>,
    capacity: Option<usize>,
    closed: AtomicBool,
    not_empty: Event,
    not_full: Event,
}

impl<T> Queue<T> {
    fn unbounded() -> Self {
        Self::new(None)
    }

    fn bounded(capacity: usize) -> Self {
        Self::new(Some(capacity))
    }

    fn new(capacity: Option<usize>) -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            capacity,
            closed: AtomicBool::new(false),
            not_empty: Event::new(),
            not_full: Event::new(),
        }
    }

    /// Queues `item` for a reader, waiting out a full bounded queue first.
    async fn push(&self, item: T) {
        let mut item = Some(item);
        loop {
            if self.try_push(&mut item) {
                return;
            }
            let listener = self.not_full.listen();
            if self.try_push(&mut item) {
                return;
            }
            listener.await;
        }
    }

    fn try_push(&self, item: &mut Option<T>) -> bool {
        let mut items = self.items.lock().unwrap();
        if self
            .capacity
            .is_some_and(|capacity| items.len() >= capacity)
        {
            return false;
        }
        items.push_back(item.take().expect("pushed twice"));
        drop(items);
        self.not_empty.notify(1);

        true
    }

    /// Takes the next item, or `None` once [`Queue::close`] has been called and nothing is left.
    async fn pop(&self) -> Option<T> {
        loop {
            if let Some(outcome) = self.try_pop() {
                return outcome;
            }
            let listener = self.not_empty.listen();
            if let Some(outcome) = self.try_pop() {
                return outcome;
            }
            listener.await;
        }
    }

    fn try_pop(&self) -> Option<Option<T>> {
        let mut items = self.items.lock().unwrap();
        if let Some(item) = items.pop_front() {
            drop(items);
            self.not_full.notify(1);

            return Some(Some(item));
        }
        self.closed.load(Ordering::Acquire).then_some(None)
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.not_empty.notify(usize::MAX);
        self.not_full.notify(usize::MAX);
    }
}

/// The writer half of an in-process channel pair, pushing into the peer's inbound queue.
struct ChannelWriter(Arc<Queue<Frame>>);

impl FrameWriter for ChannelWriter {
    fn write_frame<'a>(
        &'a self,
        frame: &'a Frame,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            self.0.push(frame.clone()).await;

            Ok(())
        })
    }
}

impl Drop for ChannelWriter {
    fn drop(&mut self) {
        // The peer's reader is popping from this same queue; closing it here is what lets that
        // side notice this end has gone, the way its socket peer would see an EOF.
        self.0.close();
    }
}

/// The reader half of an in-process channel pair, popping from this end's own inbound queue.
struct ChannelReader(Arc<Queue<Frame>>);

impl FrameReader for ChannelReader {
    fn read_frame(&self) -> Pin<Box<dyn Future<Output = Option<Frame>> + Send + '_>> {
        Box::pin(async move { self.0.pop().await })
    }
}

/// A method call whose reply is still outstanding, keyed by serial. Mirrors the shape of zbus's
/// own `PendingMethodCalls` (`zbus/src/connection/pending_method_calls.rs`) without its
/// broadcast-channel machinery, since only one waiter per call is ever needed here.
#[derive(Default)]
struct PendingCalls {
    calls: Mutex<HashMap<u32, PendingSlot>>,
    closed: AtomicBool,
}

#[derive(Default)]
struct PendingSlot {
    reply: Option<Frame>,
    waker: Option<Waker>,
}

impl PendingCalls {
    fn register(self: &Arc<Self>, serial: u32) -> PendingCall {
        self.calls
            .lock()
            .unwrap()
            .insert(serial, PendingSlot::default());

        PendingCall {
            serial,
            pending: self.clone(),
        }
    }

    fn complete(&self, serial: u32, reply: Frame) {
        let mut calls = self.calls.lock().unwrap();
        let Some(slot) = calls.get_mut(&serial) else {
            return;
        };
        slot.reply = Some(reply);
        let waker = slot.waker.take();
        drop(calls);
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Wakes every call still waiting, once the reader that would ever complete them is gone.
    fn fail_all(&self) {
        self.closed.store(true, Ordering::Release);
        let wakers: Vec<_> = self
            .calls
            .lock()
            .unwrap()
            .values_mut()
            .filter_map(|slot| slot.waker.take())
            .collect();
        for waker in wakers {
            waker.wake();
        }
    }

    fn remove(&self, serial: u32) {
        self.calls.lock().unwrap().remove(&serial);
    }
}

struct PendingCall {
    serial: u32,
    pending: Arc<PendingCalls>,
}

impl Future for PendingCall {
    type Output = Option<Frame>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut calls = self.pending.calls.lock().unwrap();
        let slot = calls.get_mut(&self.serial).expect("this call's own slot");
        if let Some(reply) = slot.reply.take() {
            calls.remove(&self.serial);

            return Poll::Ready(Some(reply));
        }
        if self.pending.closed.load(Ordering::Acquire) {
            calls.remove(&self.serial);

            return Poll::Ready(None);
        }
        slot.waker = Some(cx.waker().clone());

        Poll::Pending
    }
}

impl Drop for PendingCall {
    fn drop(&mut self) {
        self.pending.remove(self.serial);
    }
}

/// A connection over either transport: a unix socket pair or an in-process channel pair, built
/// by [`build_connection`]. Mirrors the shape of `zbus::Connection`, minus everything specific
/// to D-Bus (bus names, introspection, multiple registered paths).
struct Connection {
    write: Arc<lock::Mutex<BoxedWriter>>,
    pending: Arc<PendingCalls>,
    // Only `Connection::emit_signal`/`next_signal` read or write this, and only the unix
    // `signal/emit-receive` bench ever calls either, so this whole path is unix-only.
    #[cfg(unix)]
    signals: Arc<Queue<Frame>>,
    next_serial: AtomicU32,
    closed: Arc<AtomicBool>,
    closed_event: Arc<Event>,
    runtime: SharedRuntime,
    // Neither of these is reachable from `pending`, `signals` or `write`, so the tasks they hold
    // can never (even indirectly) end up owning a handle on themselves; dropping `Connection`
    // drops both fields and so cancels both tasks outright. Named with a leading underscore
    // because nothing ever reads them back — they are kept only for that drop.
    _reader_task: Task<(), Shared>,
    _object_server_task: Option<Task<(), Shared>>,
    drop_event: Arc<Event>,
}

impl Connection {
    async fn call(&self, body: Vec<u8>) -> io::Result<Frame> {
        let serial = self.next_serial.fetch_add(1, Ordering::Relaxed);
        // Registered before the write goes out, so a reply that arrives while the write is still
        // in flight is never missed — as zbus's own `register_call` does before it sends.
        let pending = self.pending.register(serial);
        let frame = Frame {
            kind: FrameKind::Call,
            serial,
            reply_serial: 0,
            body,
        };
        self.write.lock().await.write_frame(&frame).await?;

        enum Outcome {
            Replied(Option<Frame>),
            TimedOut,
        }

        // Races the reply against `METHOD_TIMEOUT`, the way `Connection::call_method` races it
        // against `method_timeout` in `zbus/src/connection/mod.rs`: a timer is armed on the
        // reactor for every call and dropped, cancelled, the moment the reply wins.
        let outcome = future::select(
            pin!(async { Outcome::Replied(pending.await) }),
            pin!(async {
                self.runtime.sleep(METHOD_TIMEOUT).await;
                Outcome::TimedOut
            }),
        )
        .await
        .factor_first()
        .0;

        match outcome {
            Outcome::Replied(Some(frame)) => Ok(frame),
            Outcome::Replied(None) => Err(io::Error::other(
                "the connection closed before a reply arrived",
            )),
            Outcome::TimedOut => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "method call timed out",
            )),
        }
    }

    #[cfg(unix)]
    async fn emit_signal(&self, body: Vec<u8>) {
        let frame = Frame {
            kind: FrameKind::Signal,
            serial: 0,
            reply_serial: 0,
            body,
        };
        self.write
            .lock()
            .await
            .write_frame(&frame)
            .await
            .expect("emit a signal");
    }

    #[cfg(unix)]
    async fn next_signal(&self) -> Frame {
        self.signals
            .pop()
            .await
            .expect("the connection closed while waiting for a signal")
    }

    /// Resolves once the peer has gone away, whichever side notices first: the reader returning
    /// `None`. Mirrors `Connection::closed` in `zbus/src/connection/mod.rs`.
    async fn closed(&self) {
        loop {
            if self.closed.load(Ordering::Acquire) {
                return;
            }
            let listener = self.closed_event.listen();
            if self.closed.load(Ordering::Acquire) {
                return;
            }
            listener.await;
        }
    }

    /// Drops this connection and waits for that to have happened. Mirrors zbus's actual
    /// `graceful_shutdown` (`zbus/src/connection/mod.rs`) — an immediate, `Drop`-driven
    /// teardown, not a wait for the peer to see a closed socket — since that is what is really
    /// benchmarked. Only the unix `connection/graceful-shutdown` bench calls this.
    #[cfg(unix)]
    async fn graceful_shutdown(self) {
        let listener = self.drop_event.listen();
        drop(self);
        listener.await;
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.drop_event.notify(usize::MAX);
    }
}

/// Assembles a [`Connection`] from an already-built reader and writer: the writer behind its
/// async lock, a socket-reader task that routes frames by kind, and — only where `handler` is
/// given, i.e. only on a server — a persistent object-server task that spawns and detaches a
/// task per incoming call. That extra task and hop mirrors zbus's own reader-to-`MessageStream`-
/// to-`obj_server_task`-to-detached-handler chain (`start_object_server` in
/// `zbus/src/connection/mod.rs`), rather than flattening it into a single hop.
fn build_connection(
    reader: BoxedReader,
    writer: BoxedWriter,
    runtime: &SharedRuntime,
    handler: Option<Handler>,
) -> Connection {
    let write = Arc::new(lock::Mutex::new(writer));
    let pending = Arc::new(PendingCalls::default());
    #[cfg(unix)]
    let signals = Arc::new(Queue::unbounded());
    let closed = Arc::new(AtomicBool::new(false));
    let closed_event = Arc::new(Event::new());
    let calls = handler.map(|_| Arc::new(Queue::unbounded()));

    let reader_task = runtime.spawn(
        "bench socket reader",
        run_reader(
            reader,
            pending.clone(),
            #[cfg(unix)]
            signals.clone(),
            calls.clone(),
            closed.clone(),
            closed_event.clone(),
        ),
    );
    let object_server_task = match (calls, handler) {
        (Some(calls), Some(handler)) => Some(runtime.spawn(
            "bench object server",
            run_object_server(calls, write.clone(), runtime.clone(), handler),
        )),
        _ => None,
    };

    Connection {
        write,
        pending,
        #[cfg(unix)]
        signals,
        next_serial: AtomicU32::new(1),
        closed,
        closed_event,
        runtime: runtime.clone(),
        _reader_task: reader_task,
        _object_server_task: object_server_task,
        drop_event: Arc::new(Event::new()),
    }
}

/// Reads frames until the peer is gone, routing each one: a reply to the call waiting on its
/// serial, a signal onto the signal queue, a call onto `calls` (if this side has one — only a
/// server does) for the object-server task to pick up. Mirrors `SocketReader::receive_msg` in
/// `zbus/src/connection/socket_reader.rs`.
async fn run_reader(
    reader: BoxedReader,
    pending: Arc<PendingCalls>,
    #[cfg(unix)] signals: Arc<Queue<Frame>>,
    calls: Option<Arc<Queue<Frame>>>,
    closed: Arc<AtomicBool>,
    closed_event: Arc<Event>,
) {
    while let Some(frame) = reader.read_frame().await {
        match frame.kind {
            FrameKind::Reply => pending.complete(frame.reply_serial, frame),
            #[cfg(unix)]
            FrameKind::Signal => signals.push(frame).await,
            FrameKind::Call => {
                if let Some(calls) = &calls {
                    calls.push(frame).await;
                }
            }
        }
    }

    // Ends the connection whenever the reader stops, however it stopped — the same job `Drop for
    // SocketReader` does in `zbus/src/connection/socket_reader.rs`.
    closed.store(true, Ordering::Release);
    closed_event.notify(usize::MAX);
    pending.fail_all();
    #[cfg(unix)]
    signals.close();
    if let Some(calls) = &calls {
        calls.close();
    }
}

/// Pops incoming calls and, for each, spawns and detaches a task that computes the reply and
/// writes it — the persistent task and per-call hand-off `start_object_server` in
/// `zbus/src/connection/mod.rs` does through a `MessageStream`.
async fn run_object_server(
    calls: Arc<Queue<Frame>>,
    write: Arc<lock::Mutex<BoxedWriter>>,
    runtime: SharedRuntime,
    handler: Handler,
) {
    while let Some(call) = calls.pop().await {
        let write = write.clone();
        runtime
            .spawn("bench call dispatcher", async move {
                let reply_body = handler(&call.body);
                let reply = Frame {
                    kind: FrameKind::Reply,
                    serial: 0,
                    reply_serial: call.serial,
                    body: reply_body,
                };
                let _ = write.lock().await.write_frame(&reply).await;
            })
            .detach();
    }
}

#[cfg(unix)]
mod unix {
    use std::{
        hint::black_box,
        io::{Read, Write},
        os::unix::net::UnixStream,
        sync::Arc,
        time::{Duration, Instant},
    };

    use criterion::{Criterion, Throughput};
    use futures::future;
    use zruntime::{Interest, Registration, Shared, SharedRuntime};

    use super::{
        BoxedReader, BoxedWriter, Connection, Frame, FrameKind, ZruntimeExecutor, build_connection,
        dispatch, echo, ping, run_burst_bench,
    };

    /// How big a body `method-call/1MiB-body` sends and receives.
    const BIG: usize = 1024 * 1024;

    /// A handle on the runtime this thread's `block_on` calls drive, brought into being in a
    /// `block_on` of its own and kept alive by the caller so that every later `block_on` on this
    /// thread resolves to the same runtime rather than a fresh one. Mirrors `runtime_handle` in
    /// `benches/common/executor.rs`.
    fn runtime_handle() -> SharedRuntime {
        zruntime::block_on(async { SharedRuntime::current().expect("a runtime for this thread") })
    }

    /// A small fixed header (kind, serial, reply serial, body length) plus a body of raw bytes —
    /// the framing this bench needs in place of D-Bus's own wire format. Only the unix socket
    /// transport needs actual byte encoding: the in-process channel transport passes `Frame`
    /// values directly, the way zbus's own `Channel` (`zbus/src/connection/socket/channel.rs`)
    /// passes `Message` values over a broadcast channel with no serialization at all.
    const HEADER_LEN: usize = 1 + 4 + 4 + 4;

    impl FrameKind {
        fn to_byte(self) -> u8 {
            match self {
                FrameKind::Call => 0,
                FrameKind::Reply => 1,
                FrameKind::Signal => 2,
            }
        }

        fn from_byte(byte: u8) -> Self {
            match byte {
                0 => FrameKind::Call,
                1 => FrameKind::Reply,
                2 => FrameKind::Signal,
                _ => panic!("not a frame kind: {byte}"),
            }
        }
    }

    impl Frame {
        /// Encodes this frame as one contiguous buffer, the way zbus's own `send_message`
        /// (`zbus/src/connection/socket/mod.rs`) writes a whole message from one buffer rather
        /// than its header and body separately.
        fn encode(&self) -> Vec<u8> {
            let mut buf = Vec::with_capacity(HEADER_LEN + self.body.len());
            buf.push(self.kind.to_byte());
            buf.extend_from_slice(&self.serial.to_le_bytes());
            buf.extend_from_slice(&self.reply_serial.to_le_bytes());
            buf.extend_from_slice(&(self.body.len() as u32).to_le_bytes());
            buf.extend_from_slice(&self.body);

            buf
        }
    }

    /// Decodes a frame's header, read on its own the way zbus's `receive_message` reads a fixed
    /// minimum header before it knows how much more to read.
    fn decode_header(header: &[u8; HEADER_LEN]) -> (FrameKind, u32, u32, usize) {
        let kind = FrameKind::from_byte(header[0]);
        let serial = u32::from_le_bytes(header[1..5].try_into().unwrap());
        let reply_serial = u32::from_le_bytes(header[5..9].try_into().unwrap());
        let body_len = u32::from_le_bytes(header[9..13].try_into().unwrap()) as usize;

        (kind, serial, reply_serial, body_len)
    }

    pub(super) fn connection_benches(c: &mut Criterion) {
        let runtime = runtime_handle();

        let mut group = c.benchmark_group("connection");
        group.sample_size(20);
        group.bench_function("build-and-drop", |b| {
            let runtime = &runtime;
            b.to_async(ZruntimeExecutor)
                .iter(|| async { drop(black_box(pair(runtime).await)) });
        });
        group.bench_function("graceful-shutdown", |b| {
            b.iter_custom(|iters| {
                zruntime::block_on(async {
                    let mut total = Duration::ZERO;
                    for _ in 0..iters {
                        let (server, client) = pair(&runtime).await;
                        let started = Instant::now();
                        future::join(server.graceful_shutdown(), client.graceful_shutdown()).await;
                        total += started.elapsed();
                    }

                    total
                })
            });
        });
        group.finish();
    }

    pub(super) fn method_call_benches(c: &mut Criterion) {
        let runtime = runtime_handle();

        let mut group = c.benchmark_group("method-call");
        {
            let (_server, client) = zruntime::block_on(pair(&runtime));
            group.bench_function("roundtrip", |b| {
                b.to_async(ZruntimeExecutor)
                    .iter(|| async { black_box(ping(&client, 1).await.unwrap()) });
            });
        }
        group.sample_size(10);
        group.throughput(Throughput::Bytes(BIG as u64));
        {
            let (_server, client) = zruntime::block_on(pair(&runtime));
            let body = vec![7u8; BIG];
            group.bench_function("1MiB-body", |b| {
                b.to_async(ZruntimeExecutor)
                    .iter(|| async { black_box(echo(&client, &body).await.unwrap()) });
            });
        }
        group.finish();
    }

    pub(super) fn signal_benches(c: &mut Criterion) {
        let runtime = runtime_handle();

        let mut group = c.benchmark_group("signal");
        let (server, client) = zruntime::block_on(pair(&runtime));
        group.bench_function("emit-receive", |b| {
            b.to_async(ZruntimeExecutor).iter(|| async {
                server.emit_signal(Vec::new()).await;
                black_box(client.next_signal().await);
            });
        });
        group.finish();
    }

    pub(super) fn concurrent_socket_bench(c: &mut Criterion) {
        let mut group = c.benchmark_group("method-call");
        group.sample_size(10);
        group.throughput(Throughput::Elements(super::CONCURRENT_METHOD_CALLS as u64));

        // Both ends of a unix socket pair, handshake included — mirrors `socket_pair` in
        // `zbus/benches/concurrent_method_calls.rs`. Default buffer sizes on purpose: with a
        // reader task on each side that never blocks on a write, the two ends cannot deadlock
        // the way a single read-then-write task could, so the 1000-concurrent body (each call a
        // few bytes) legitimately exercises the reactor's readiness path a plain 1MiB echo would
        // need enlarged buffers for instead.
        let (server_stream, client_stream) = UnixStream::pair().unwrap();

        run_burst_bench(
            &mut group,
            "1000-concurrent-p2p-socket",
            move |runtime: &SharedRuntime| {
                let end = Arc::new(UnixEnd::register(runtime, server_stream));
                Box::pin(async move {
                    server_handshake(&end).await;
                    let reader: BoxedReader = Box::new(end.clone());
                    let writer: BoxedWriter = Box::new(end);

                    (reader, writer)
                })
            },
            move |runtime: &SharedRuntime| {
                let end = Arc::new(UnixEnd::register(runtime, client_stream));
                Box::pin(async move {
                    client_handshake(&end).await;
                    let reader: BoxedReader = Box::new(end.clone());
                    let writer: BoxedWriter = Box::new(end);

                    (reader, writer)
                })
            },
        );

        group.finish();
    }

    /// A server and a client over a fresh socket pair, handshake included. Mirrors `pair` in
    /// `zbus/benches/runtime.rs`.
    async fn pair(runtime: &SharedRuntime) -> (Connection, Connection) {
        let (server_stream, client_stream) = UnixStream::pair().unwrap();
        let server_end = Arc::new(UnixEnd::register(runtime, server_stream));
        let client_end = Arc::new(UnixEnd::register(runtime, client_stream));

        future::join(server_handshake(&server_end), client_handshake(&client_end)).await;

        let server = build_connection(
            Box::new(server_end.clone()),
            Box::new(server_end),
            runtime,
            Some(dispatch),
        );
        let client = build_connection(
            Box::new(client_end.clone()),
            Box::new(client_end),
            runtime,
            None,
        );

        (server, client)
    }

    /// One end of a unix socket pair, registered on the reactor once and shared, via `Arc`,
    /// between its [`super::FrameReader`] and [`super::FrameWriter`] impls — a single fd and a
    /// single registration either way, never one dup per half, so the peer sees an EOF as soon
    /// as this end drops.
    struct UnixEnd {
        registration: Registration<Shared>,
        stream: UnixStream,
    }

    impl UnixEnd {
        fn register(runtime: &SharedRuntime, stream: UnixStream) -> Self {
            stream.set_nonblocking(true).expect("nonblocking");
            let registration = runtime
                .register(
                    stream
                        .try_clone()
                        .expect("clone the fd for the registration"),
                )
                .expect("register the socket");

            Self {
                registration,
                stream,
            }
        }

        /// Reads until `buf` is full, or `None` once the peer has closed.
        async fn read_exact(&self, mut buf: &mut [u8]) -> Option<()> {
            while !buf.is_empty() {
                let read = std::future::poll_fn(|cx| {
                    self.registration
                        .poll_io(cx, Interest::Readable, || (&self.stream).read(buf))
                })
                .await
                .expect("read from the bench socket");
                if read == 0 {
                    return None;
                }
                buf = &mut buf[read..];
            }

            Some(())
        }

        /// Writes every byte of `buf`, waiting out `WouldBlock` on the registration in between.
        async fn write_all(&self, mut buf: &[u8]) {
            while !buf.is_empty() {
                let written = std::future::poll_fn(|cx| {
                    self.registration
                        .poll_io(cx, Interest::Writable, || (&self.stream).write(buf))
                })
                .await
                .expect("write to the bench socket");
                buf = &buf[written..];
            }
        }
    }

    impl super::FrameReader for Arc<UnixEnd> {
        fn read_frame(
            &self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Frame>> + Send + '_>>
        {
            Box::pin(async move {
                let mut header = [0u8; HEADER_LEN];
                self.read_exact(&mut header).await?;
                let (kind, serial, reply_serial, body_len) = decode_header(&header);
                let mut body = vec![0u8; body_len];
                self.read_exact(&mut body).await?;

                Some(Frame {
                    kind,
                    serial,
                    reply_serial,
                    body,
                })
            })
        }
    }

    impl super::FrameWriter for Arc<UnixEnd> {
        fn write_frame<'a>(
            &'a self,
            frame: &'a Frame,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>>
        {
            Box::pin(async move {
                self.write_all(&frame.encode()).await;

                Ok(())
            })
        }
    }

    /// One line-sized round trip of this bench's own protocol: a fixed request answered by a
    /// fixed response. Two of these in a row keep the same shape as zbus's own non-bus p2p
    /// handshake — an AUTH/OK exchange, then a NEGOTIATE_UNIX_FD+BEGIN write (pipelined, so it
    /// costs one round trip, not two) answered by one AGREE_UNIX_FD — without any of its actual
    /// SASL text; see `Client::perform` in `zbus/src/connection/handshake/client.rs`.
    const HANDSHAKE_LINE: &[u8] = b"AUTH EXTERNAL 0\r\n";
    const HANDSHAKE_ROUND_TRIPS: usize = 2;

    async fn client_handshake(end: &UnixEnd) {
        for _ in 0..HANDSHAKE_ROUND_TRIPS {
            end.write_all(HANDSHAKE_LINE).await;
            let mut reply = [0u8; HANDSHAKE_LINE.len()];
            end.read_exact(&mut reply)
                .await
                .expect("the server closed the connection during the handshake");
        }
    }

    async fn server_handshake(end: &UnixEnd) {
        for _ in 0..HANDSHAKE_ROUND_TRIPS {
            let mut request = [0u8; HANDSHAKE_LINE.len()];
            end.read_exact(&mut request)
                .await
                .expect("the client closed the connection during the handshake");
            end.write_all(HANDSHAKE_LINE).await;
        }
    }
}

#[cfg(unix)]
criterion_group!(
    benches,
    unix::connection_benches,
    unix::method_call_benches,
    unix::signal_benches,
    concurrent_channel_bench,
    unix::concurrent_socket_bench,
);
#[cfg(not(unix))]
criterion_group!(benches, concurrent_channel_bench);

criterion_main!(benches);
