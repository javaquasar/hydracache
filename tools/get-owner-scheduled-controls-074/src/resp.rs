//! Tool-only RESP2 GET pipeline with bounded FIFO owners and response timestamps.
//! This is not a general Redis client, security-matched cohort, or timing CLI.
use crate::{
    native::Dataset,
    rate::FixedRateSchedule,
    scheduled::{self, Config, Observation},
    target::{Target, TargetError, TargetOutcome, TargetRequest},
};
use async_trait::async_trait;
use bytes::Bytes;
use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer, RespDecodeLimits};
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::Instant;

type Result<T> = std::result::Result<T, String>;
const MAX_REPLY: usize = 1_048_576;
const MAX_HEADER: usize = 128;
const MAX_HISTORY: usize = 10_256;

#[derive(Clone)]
enum Expected {
    Bulk(Option<Bytes>),
    Stored,
}
enum Frame<'a> {
    Bulk(Option<&'a [u8]>),
    Simple(&'a [u8]),
    Error,
}
impl Frame<'_> {
    fn kind(&self) -> &'static str {
        match self {
            Self::Bulk(Some(_)) => "bulk",
            Self::Bulk(None) => "null",
            Self::Simple(_) => "simple",
            Self::Error => "error",
        }
    }
    fn matches(&self, expected: &Expected) -> bool {
        match (self, expected) {
            (Self::Bulk(actual), Expected::Bulk(value)) => *actual == value.as_deref(),
            (Self::Simple(b"OK"), Expected::Stored) => true,
            _ => false,
        }
    }
}

/// Incremental bounded parser: only the response types used by this control.
fn decode(bytes: &[u8]) -> Result<Option<(Frame<'_>, usize)>> {
    if bytes.is_empty() {
        return Ok(None);
    }
    if ![b'$', b'+', b'-'].contains(&bytes[0]) {
        return Err("unsupported RESP2 reply type".to_owned());
    }
    let Some(end) = bytes.windows(2).position(|part| part == b"\r\n") else {
        if bytes.len() > MAX_HEADER + 1 {
            return Err("RESP2 reply header over budget".to_owned());
        }
        return Ok(None);
    };
    if end > MAX_HEADER {
        return Err("RESP2 reply header over budget".to_owned());
    }
    if bytes[0] == b'+' {
        return Ok(Some((Frame::Simple(&bytes[1..end]), end + 2)));
    }
    if bytes[0] == b'-' {
        return Ok(Some((Frame::Error, end + 2)));
    }
    let length = &bytes[1..end];
    if length == b"-1" {
        return Ok(Some((Frame::Bulk(None), end + 2)));
    }
    if length.is_empty() || !length.iter().all(u8::is_ascii_digit) {
        return Err("invalid RESP2 bulk length".to_owned());
    }
    let size = std::str::from_utf8(length)
        .map_err(|_| "invalid length")?
        .parse::<usize>()
        .map_err(|_| "bulk length overflow")?;
    if size > MAX_REPLY {
        return Err("RESP2 bulk reply over budget".to_owned());
    }
    let total = end + 2 + size + 2;
    if bytes.len() < total {
        return Ok(None);
    }
    if &bytes[total - 2..total] != b"\r\n" {
        return Err("invalid RESP2 bulk terminator".to_owned());
    }
    Ok(Some((Frame::Bulk(Some(&bytes[end + 2..total - 2])), total)))
}

fn command(parts: &[&[u8]]) -> Bytes {
    let mut frame = format!("*{}\r\n", parts.len()).into_bytes();
    for part in parts {
        frame.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        frame.extend_from_slice(part);
        frame.extend_from_slice(b"\r\n");
    }
    Bytes::from(frame)
}
struct Submission {
    sequence: u64,
    request: Bytes,
    expected: Expected,
    reply: oneshot::Sender<TargetOutcome>,
    accepted: Instant,
    _slot: OwnedSemaphorePermit,
}
struct Pending {
    submission: Submission,
    ordinal: u64,
    written: Option<Instant>,
}
struct Write {
    bytes: Bytes,
    offset: usize,
    ordinal: u64,
}
#[derive(Clone)]
struct Record {
    sequence: u64,
    ordinal: u64,
    accepted: Instant,
    written: Option<Instant>,
    received: Option<Instant>,
    kind: Option<&'static str>,
    verified: bool,
    cancelled: bool,
    failure: Option<&'static str>,
}
fn record(
    pending: Pending,
    received: Option<Instant>,
    kind: Option<&'static str>,
    verified: bool,
    failure: Option<&'static str>,
    records: &Mutex<Vec<Record>>,
) {
    let cancelled = pending.submission.reply.is_closed();
    records.lock().expect("tool response records").push(Record {
        sequence: pending.submission.sequence,
        ordinal: pending.ordinal,
        accepted: pending.submission.accepted,
        written: pending.written,
        received,
        kind,
        verified,
        cancelled,
        failure,
    });
    let _ = pending.submission.reply.send(if verified {
        TargetOutcome::Success
    } else {
        TargetOutcome::Error
    });
    // The slot belongs to the FIFO owner, NOT the cancelled waiting caller.
}

async fn pump<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    mut requests: mpsc::Receiver<Submission>,
    mut shutdown: watch::Receiver<bool>,
    records: Arc<Mutex<Vec<Record>>>,
) -> Result<()> {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let mut pending: VecDeque<Pending> = VecDeque::new();
    let mut writes: VecDeque<Write> = VecDeque::new();
    let mut buffer = Vec::new();
    let mut scratch = [0u8; 4096];
    let mut ordinal = 0;
    let result:Result<()> = async {
        loop {
            while let Some((frame, consumed)) = decode(&buffer)? {
                let received = Instant::now();
                if pending.front().ok_or("unsolicited RESP2 reply")?.written.is_none() { return Err("reply before full request write".to_owned()); }
                let owner = pending.pop_front().expect("checked FIFO owner");
                let kind = frame.kind(); let verified = frame.matches(&owner.submission.expected);
                record(owner, Some(received), Some(kind), verified, None, &records);
                buffer.drain(..consumed);
            }
            let available = (MAX_REPLY + MAX_HEADER + 4 - buffer.len()).min(scratch.len());
            if available == 0 { return Err("RESP2 reply buffer over budget".to_owned()); }
            let write = writes.front().map(|w| w.bytes.slice(w.offset..));
            tokio::select! {
                changed = shutdown.changed() => { if changed.is_err() || *shutdown.borrow() { return Ok(()); } }
                submission = requests.recv() => {
                    let Some(submission) = submission else { return Ok(()); };
                    writes.push_back(Write {bytes:submission.request.clone(), offset:0, ordinal});
                    pending.push_back(Pending {submission, ordinal, written:None}); ordinal += 1;
                }
                written = async { writer.write(write.as_ref().expect("guarded write")).await }, if write.is_some() => {
                    let count = written.map_err(|_| "RESP2 write failure")?;
                    if count == 0 { return Err("RESP2 zero write".to_owned()); }
                    let front = writes.front_mut().expect("write owner"); front.offset += count;
                    if front.offset == front.bytes.len() {
                        let done = writes.pop_front().expect("write owner");
                        let owner = pending.iter_mut().find(|p| p.ordinal == done.ordinal).ok_or("write owner lost")?;
                        owner.written = Some(Instant::now());
                    }
                }
                read = reader.read(&mut scratch[..available]) => {
                    let count = read.map_err(|_| "RESP2 read failure")?;
                    if count == 0 { return Err("RESP2 disconnect".to_owned()); }
                    buffer.extend_from_slice(&scratch[..count]);
                }
            }
        }
    }.await;
    // Pending and not-yet-written owners all get failure, never a response time.
    for owner in pending {
        record(
            owner,
            None,
            None,
            false,
            Some("closed-or-protocol-failure"),
            &records,
        );
    }
    requests.close();
    while let Some(submission) = requests.recv().await {
        record(
            Pending {
                submission,
                ordinal,
                written: None,
            },
            None,
            None,
            false,
            Some("closed-before-write"),
            &records,
        );
        ordinal += 1;
    }
    result
}

struct Pipeline {
    requests: mpsc::Sender<Submission>,
    slots: Arc<Semaphore>,
    seen: Mutex<HashSet<u64>>,
    records: Arc<Mutex<Vec<Record>>>,
    stop: watch::Sender<bool>,
    actor: Mutex<Option<JoinHandle<Result<()>>>>,
    depth: usize,
}
impl Drop for Pipeline {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
        if let Some(actor) = self.actor.get_mut().expect("tool actor lock").take() {
            actor.abort();
        }
    }
}
impl Pipeline {
    fn new<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        stream: S,
        depth: usize,
    ) -> Result<Self> {
        if ![1, 10, 50].contains(&depth) {
            return Err("unsupported RESP2 pipeline depth".to_owned());
        }
        let (requests, receiver) = mpsc::channel(depth);
        let (stop, shutdown) = watch::channel(false);
        let records = Arc::new(Mutex::new(Vec::new()));
        let actor = tokio::spawn(pump(stream, receiver, shutdown, Arc::clone(&records)));
        Ok(Self {
            requests,
            slots: Arc::new(Semaphore::new(depth)),
            seen: Mutex::new(HashSet::new()),
            records,
            stop,
            actor: Mutex::new(Some(actor)),
            depth,
        })
    }
    async fn submit(&self, sequence: u64, request: Bytes, expected: Expected) -> TargetOutcome {
        if request.is_empty() || request.len() > MAX_REPLY + 4096 {
            return TargetOutcome::Rejected;
        }
        {
            let mut seen = self.seen.lock().expect("tool request identities");
            if seen.len() == MAX_HISTORY || !seen.insert(sequence) {
                return TargetOutcome::Rejected;
            }
        }
        let Ok(slot) = Arc::clone(&self.slots).acquire_owned().await else {
            return TargetOutcome::Error;
        };
        let (reply, received) = oneshot::channel();
        let submission = Submission {
            sequence,
            request,
            expected,
            reply,
            accepted: Instant::now(),
            _slot: slot,
        };
        if self.requests.send(submission).await.is_err() {
            return TargetOutcome::Error;
        }
        received.await.unwrap_or(TargetOutcome::Error)
    }
    async fn shutdown(&self) -> Result<()> {
        let _ = self.stop.send(true);
        let actor = self.actor.lock().map_err(|_| "actor lock poisoned")?.take();
        if let Some(mut actor) = actor {
            match tokio::time::timeout(Duration::from_secs(5), &mut actor).await {
                Ok(result) => result.map_err(|e| e.to_string())??,
                Err(_) => {
                    actor.abort();
                    let _ = actor.await;
                    return Err("RESP2 actor drain timed out".to_owned());
                }
            }
        }
        Ok(())
    }
    async fn drain(&self) -> Result<()> {
        let slots = Arc::clone(&self.slots).acquire_many_owned(self.depth as u32);
        let _all = tokio::time::timeout(Duration::from_secs(5), slots)
            .await
            .map_err(|_| "RESP2 wire owners still pending after drain")?
            .map_err(|_| "RESP2 slots closed")?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct WireSample {
    pub sequence: u64,
    pub wire_ordinal: u64,
    pub scheduled_ns: u64,
    pub accepted_ns: u64,
    pub write_completed_ns: Option<u64>,
    pub response_complete_ns: Option<u64>,
    pub scheduled_frame_latency_ns: Option<u64>,
    pub frame_kind: Option<&'static str>,
    pub byte_oracle_verified: bool,
    pub waiting_caller_cancelled: bool,
    pub transport_failure: Option<&'static str>,
    pub operation_outcome: scheduled::Outcome,
}
#[derive(Serialize)]
pub struct RespObservation {
    pub operations: Observation,
    pub wire_samples: Vec<WireSample>,
    pub pipeline_limit: usize,
    pub product_performance_claim: bool,
}
/// One real TCP connection, production RESP connection owner, independent store.
pub struct RespControl {
    pipeline: Pipeline,
    entries: Vec<(Bytes, Bytes)>,
    state: Arc<ClientSurfaceState>,
    server: Arc<RedisRespServer>,
    listener: Option<JoinHandle<Result<()>>>,
    digest: String,
    depth: usize,
    setup_id: std::sync::atomic::AtomicU64,
    used: std::sync::atomic::AtomicBool,
}
impl Drop for RespControl {
    fn drop(&mut self) {
        if let Some(listener) = self.listener.take() {
            listener.abort();
        }
    }
}
impl RespControl {
    pub async fn start(dataset: Dataset, depth: usize) -> Result<Self> {
        if ![1, 10, 50].contains(&depth) {
            return Err("unsupported RESP2 pipeline depth".to_owned());
        }
        let state = Arc::new(
            ClientSurfaceState::new(ClientSurfaceLimits::default()).map_err(|e| e.to_string())?,
        );
        state.set_profile_instrumentation_enabled(false);
        let server = Arc::new(
            RedisRespServer::new(
                Arc::clone(&state),
                RedisListenerConfig {
                    namespace: "default".to_owned(),
                    tenant: "scheduled-resp".to_owned(),
                    client_id: "scheduled-resp".to_owned(),
                    decode_limits: RespDecodeLimits {
                        max_frame_bytes: 8 * 1024 * 1024,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?,
        );
        server.set_pipeline_instrumentation_enabled(false);
        let tcp = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| e.to_string())?;
        let stream = tokio::net::TcpStream::connect(tcp.local_addr().map_err(|e| e.to_string())?)
            .await
            .map_err(|e| e.to_string())?;
        stream.set_nodelay(true).map_err(|e| e.to_string())?;
        let pipeline = Pipeline::new(stream, depth)?;
        let observed = Arc::clone(&server);
        let listener = tokio::spawn(async move {
            let (stream, _) = tcp.accept().await.map_err(|e| e.to_string())?;
            stream.set_nodelay(true).map_err(|e| e.to_string())?;
            observed
                .serve_connection(stream)
                .await
                .map_err(|e| e.to_string())
        });
        let control = Self {
            pipeline,
            entries: dataset.entries(),
            state,
            server,
            listener: Some(listener),
            digest: dataset.digest(),
            depth,
            setup_id: std::sync::atomic::AtomicU64::new(u64::MAX),
            used: std::sync::atomic::AtomicBool::new(false),
        };
        for (key, value) in &control.entries {
            if control
                .pipeline
                .submit(
                    control.setup_id(),
                    command(&[b"SET", key, value]),
                    Expected::Stored,
                )
                .await
                != TargetOutcome::Success
            {
                return Err("RESP2 transport preload failed".to_owned());
            }
        }
        control.verify().await?;
        Ok(control)
    }
    fn setup_id(&self) -> u64 {
        self.setup_id
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
    }
    pub async fn verify(&self) -> Result<String> {
        for (key, value) in &self.entries {
            if self
                .pipeline
                .submit(
                    self.setup_id(),
                    command(&[b"GET", key]),
                    Expected::Bulk(Some(value.clone())),
                )
                .await
                != TargetOutcome::Success
            {
                return Err("RESP2 final byte oracle failed".to_owned());
            }
        }
        if self
            .pipeline
            .submit(
                self.setup_id(),
                command(&[b"GET", b"missing-key"]),
                Expected::Bulk(None),
            )
            .await
            != TargetOutcome::Success
        {
            return Err("RESP2 missing GET failed".to_owned());
        }
        if self.state.profile_metrics() != Default::default()
            || self.server.pipeline_metrics() != Default::default()
        {
            return Err("product profiling unexpectedly active".to_owned());
        }
        Ok(self.digest.clone())
    }
    pub async fn run(self: &Arc<Self>, config: &Config) -> Result<RespObservation> {
        config.validate()?;
        if self.used.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Err("RESP2 control is single-use; no retry or history rebasing".to_owned());
        }
        let origin = Instant::now();
        let operations = scheduled::run_at(Arc::clone(self), config, origin).await?;
        // A cancelled waiter is not an aborted TCP request. Await the FIFO
        // tombstones too; an incomplete drain is an error, never a clean receipt.
        self.pipeline.drain().await?;
        let elapsed = u64::try_from(origin.elapsed().as_nanos()).map_err(|_| "elapsed overflow")?;
        let operations = scheduled::project(
            config,
            operations.samples,
            elapsed,
            operations.pending_high_water,
            operations.owned_tasks_drained,
        )?;
        let schedule = FixedRateSchedule::new(0, config.offered_rate_per_second)?;
        let offset = |at: Instant| {
            u64::try_from(at.saturating_duration_since(origin).as_nanos()).unwrap_or(u64::MAX)
        };
        let records = self
            .pipeline
            .records
            .lock()
            .map_err(|_| "record lock poisoned")?;
        let wire_samples = records
            .iter()
            .filter(|r| r.sequence < config.operations)
            .map(|r| {
                let scheduled_ns = schedule.scheduled_ns(r.sequence);
                WireSample {
                    sequence: r.sequence,
                    wire_ordinal: r.ordinal,
                    scheduled_ns,
                    accepted_ns: offset(r.accepted),
                    write_completed_ns: r.written.map(offset),
                    response_complete_ns: r.received.map(offset),
                    scheduled_frame_latency_ns: r
                        .received
                        .map(|at| offset(at).saturating_sub(scheduled_ns)),
                    frame_kind: r.kind,
                    byte_oracle_verified: r.verified,
                    waiting_caller_cancelled: r.cancelled,
                    transport_failure: r.failure,
                    operation_outcome: operations.samples[r.sequence as usize].outcome,
                }
            })
            .collect();
        let result = RespObservation {
            operations,
            wire_samples,
            pipeline_limit: self.depth,
            product_performance_claim: false,
        };
        validate_wire(&result)?;
        Ok(result)
    }
    pub async fn shutdown(mut self) -> Result<()> {
        let result = self.pipeline.shutdown().await;
        if let Some(mut listener) = self.listener.take() {
            match tokio::time::timeout(Duration::from_secs(5), &mut listener).await {
                Ok(joined) => joined.map_err(|e| e.to_string())??,
                Err(_) => {
                    listener.abort();
                    let _ = listener.await;
                    return Err("RESP2 connection drain timed out".to_owned());
                }
            }
        }
        result
    }
}
#[async_trait]
impl Target for RespControl {
    async fn reset(&self) -> std::result::Result<String, TargetError> {
        self.verify().await.map_err(TargetError::Reset)
    }
    async fn state_digest(&self) -> std::result::Result<String, TargetError> {
        self.verify().await.map_err(TargetError::Measurement)
    }
    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        if request.sequence >= 10_000 {
            return TargetOutcome::Rejected;
        }
        let (key, value) = &self.entries[request.sequence as usize % self.entries.len()];
        self.pipeline
            .submit(
                request.sequence,
                command(&[b"GET", key]),
                Expected::Bulk(Some(value.clone())),
            )
            .await
    }
}

/// Reject response reassignment, timestamp rebasing and fabricated wire replies.
pub fn validate_wire(observation: &RespObservation) -> Result<()> {
    let schedule =
        FixedRateSchedule::new(0, observation.operations.config.offered_rate_per_second)?;
    let mut seen = HashSet::new();
    let mut previous_ordinal = None;
    let mut previous_response = None;
    for sample in &observation.wire_samples {
        let operation = observation
            .operations
            .samples
            .get(sample.sequence as usize)
            .ok_or("orphan wire sequence")?;
        if !seen.insert(sample.sequence)
            || sample.operation_outcome != operation.outcome
            || sample.scheduled_ns != schedule.scheduled_ns(sample.sequence)
            || sample.accepted_ns < sample.scheduled_ns
            || operation
                .started_ns
                .is_none_or(|start| sample.accepted_ns < start)
            || previous_ordinal.is_some_and(|ordinal| ordinal >= sample.wire_ordinal)
        {
            return Err("wire identity, schedule or FIFO drift".to_owned());
        }
        previous_ordinal = Some(sample.wire_ordinal);
        if sample.write_completed_ns.is_some_and(|write| {
            write < sample.accepted_ns || write > observation.operations.elapsed_ns
        }) {
            return Err("wire write timestamp drift".to_owned());
        }
        match sample.response_complete_ns {
            Some(response) => {
                if sample
                    .write_completed_ns
                    .is_none_or(|write| response < write)
                    || response > observation.operations.elapsed_ns
                    || previous_response.is_some_and(|prior| prior > response)
                    || sample.frame_kind.is_none()
                    || sample.transport_failure.is_some()
                    || sample.scheduled_frame_latency_ns != Some(response - sample.scheduled_ns)
                {
                    return Err("wire response boundary drift".to_owned());
                }
                previous_response = Some(response);
            }
            None => {
                if sample.byte_oracle_verified
                    || sample.frame_kind.is_some()
                    || sample.scheduled_frame_latency_ns.is_some()
                    || sample.transport_failure.is_none()
                {
                    return Err("fabricated wire response".to_owned());
                }
            }
        }
        if operation.outcome == scheduled::Outcome::Success
            && (!sample.byte_oracle_verified
                || sample.waiting_caller_cancelled
                || sample
                    .response_complete_ns
                    .is_none_or(|time| time > operation.terminal_ns))
        {
            return Err("success without verified response".to_owned());
        }
    }
    if observation.operations.samples.iter().any(|sample| {
        sample.outcome == scheduled::Outcome::Success && !seen.contains(&sample.sequence)
    }) {
        return Err("successful offer omitted from wire evidence".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        pin::Pin,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Poll, Waker},
    };
    use tokio::io::{DuplexStream, ReadBuf};
    use tokio::sync::Notify;

    #[test]
    fn fragmented_binary_empty_null_error_and_coalesced_frames_are_distinct() {
        for frame in [
            &b"$6\r\n\0\xffa\r\nb\r\n"[..],
            &b"$0\r\n\r\n"[..],
            &b"$-1\r\n"[..],
            &b"+OK\r\n"[..],
            &b"-ERR denied\r\n"[..],
        ] {
            for length in 0..frame.len() {
                assert!(
                    decode(&frame[..length]).unwrap().is_none(),
                    "split={length}"
                );
            }
            assert_eq!(decode(frame).unwrap().unwrap().1, frame.len());
        }
        let (empty, _) = decode(b"$0\r\n\r\n").unwrap().unwrap();
        assert!(empty.matches(&Expected::Bulk(Some(Bytes::new()))));
        assert!(!empty.matches(&Expected::Bulk(None)));
        let (null, _) = decode(b"$-1\r\n$1\r\nx\r\n").unwrap().unwrap();
        assert!(null.matches(&Expected::Bulk(None)));
        assert!(!null.matches(&Expected::Bulk(Some(Bytes::new()))));
        assert_eq!(
            decode(b"-ERR denied\r\n").unwrap().unwrap().0.kind(),
            "error"
        );
    }

    #[test]
    fn malformed_oversized_and_future_reply_types_fail_loud() {
        for frame in [
            b"$-2\r\n".as_slice(),
            b"$+1\r\n",
            b"$\r\n",
            b"$1048577\r\n",
            b"$999999999999999999999999999\r\n",
            b"$1\r\nxZZ",
            b"*0\r\n",
        ] {
            assert!(decode(frame).is_err());
        }
        assert!(decode(&[b'+'; MAX_HEADER + 2]).is_err());
        let mut header = vec![b'+'; MAX_HEADER + 1];
        header.extend_from_slice(b"\r\n");
        assert!(decode(&header).is_err());
        let mut boundary = vec![b'+'; MAX_HEADER];
        boundary.extend_from_slice(b"\r\n");
        assert!(decode(&boundary[..boundary.len() - 1]).unwrap().is_none());
        assert!(decode(&boundary).unwrap().is_some());
    }

    struct Gate {
        closed: AtomicBool,
        waker: Mutex<Option<Waker>>,
    }
    impl Gate {
        fn open(&self) {
            self.closed.store(false, Ordering::SeqCst);
            if let Some(waker) = self.waker.lock().unwrap().take() {
                waker.wake();
            }
        }
    }
    struct LimitedIo {
        stream: DuplexStream,
        gate: Arc<Gate>,
    }
    impl AsyncRead for LimitedIo {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            if self.gate.closed.load(Ordering::SeqCst) {
                *self.gate.waker.lock().unwrap() = Some(cx.waker().clone());
                return Poll::Pending;
            }
            Pin::new(&mut self.stream).poll_read(cx, buffer)
        }
    }
    impl AsyncWrite for LimitedIo {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.stream).poll_write(cx, &bytes[..bytes.len().min(2)])
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.stream).poll_flush(cx)
        }
        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.stream).poll_shutdown(cx)
        }
    }

    #[tokio::test]
    async fn slow_reader_and_partial_writes_preserve_cancelled_fifo_tombstone() {
        let (client, mut peer) = tokio::io::duplex(1);
        let gate = Arc::new(Gate {
            closed: AtomicBool::new(true),
            waker: Mutex::new(None),
        });
        let pipeline = Arc::new(
            Pipeline::new(
                LimitedIo {
                    stream: client,
                    gate: Arc::clone(&gate),
                },
                1,
            )
            .unwrap(),
        );
        let first = command(&[b"GET", b"a"]);
        let second = command(&[b"GET", b"b"]);
        let first_seen = Arc::new(Notify::new());
        let observed = Arc::clone(&first_seen);
        let hold_peer = Arc::new(Notify::new());
        let held = Arc::clone(&hold_peer);
        let first_bytes = first.clone();
        let second_bytes = second.clone();
        let server = tokio::spawn(async move {
            let mut input = vec![0; first_bytes.len()];
            peer.read_exact(&mut input).await.unwrap();
            assert_eq!(input, first_bytes);
            peer.write_all(b"$").await.unwrap();
            observed.notify_one();
            peer.write_all(b"1\r\na\r\n").await.unwrap();
            let mut input = vec![0; second_bytes.len()];
            peer.read_exact(&mut input).await.unwrap();
            assert_eq!(input, second_bytes);
            for byte in b"$1\r\nb\r\n" {
                peer.write_all(&[*byte]).await.unwrap();
            }
            held.notified().await;
        });
        let p = Arc::clone(&pipeline);
        let waiter = tokio::spawn(async move {
            p.submit(0, first, Expected::Bulk(Some(Bytes::from_static(b"a"))))
                .await
        });
        first_seen.notified().await;
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert_eq!(pipeline.slots.available_permits(), 0);
        assert!(pipeline.records.lock().unwrap().is_empty());
        let p = Arc::clone(&pipeline);
        let next = tokio::spawn(async move {
            p.submit(1, second, Expected::Bulk(Some(Bytes::from_static(b"b"))))
                .await
        });
        gate.open();
        assert_eq!(next.await.unwrap(), TargetOutcome::Success);
        pipeline.drain().await.unwrap();
        {
            let records = pipeline.records.lock().unwrap();
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].sequence, 0);
            assert!(records[0].cancelled);
            assert!(records[0].verified);
            assert_eq!(records[1].sequence, 1);
            assert!(!records[1].cancelled);
            assert!(records[1].verified);
            assert!(records.iter().all(|r| r.received >= r.written));
        }
        pipeline.shutdown().await.unwrap();
        hold_peer.notify_one();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn reads_progress_while_later_pipeline_write_is_backpressured() {
        let (client, mut peer) = tokio::io::duplex(1);
        let pipeline = Arc::new(Pipeline::new(client, 10).unwrap());
        let first = command(&[b"GET", b"a"]);
        let second = command(&[b"GET", b"b"]);
        let seen = Arc::new(Notify::new());
        let ready = Arc::clone(&seen);
        let gate = Arc::new(Notify::new());
        let release = Arc::clone(&gate);
        let done = Arc::new(Notify::new());
        let held = Arc::clone(&done);
        let a = first.clone();
        let b = second.clone();
        let peer = tokio::spawn(async move {
            let mut input = vec![0; a.len()];
            peer.read_exact(&mut input).await.unwrap();
            assert_eq!(input, a);
            ready.notify_one();
            release.notified().await;
            peer.write_all(b"$1\r\na\r\n").await.unwrap();
            let mut input = vec![0; b.len()];
            peer.read_exact(&mut input).await.unwrap();
            assert_eq!(input, b);
            peer.write_all(b"$1\r\nb\r\n").await.unwrap();
            held.notified().await;
        });
        let p = Arc::clone(&pipeline);
        let a = tokio::spawn(async move {
            p.submit(0, first, Expected::Bulk(Some(Bytes::from_static(b"a"))))
                .await
        });
        seen.notified().await;
        let p = Arc::clone(&pipeline);
        let b = tokio::spawn(async move {
            p.submit(1, second, Expected::Bulk(Some(Bytes::from_static(b"b"))))
                .await
        });
        gate.notify_one();
        assert_eq!(a.await.unwrap(), TargetOutcome::Success);
        assert_eq!(b.await.unwrap(), TargetOutcome::Success);
        pipeline.shutdown().await.unwrap();
        done.notify_one();
        peer.await.unwrap();
    }

    #[tokio::test]
    async fn disconnect_and_malformed_reply_release_owners_without_fake_response_time() {
        for reply in [b"$2\r\nx".as_slice(), b"$1048577\r\n"] {
            let (client, mut peer) = tokio::io::duplex(64);
            let pipeline = Pipeline::new(client, 1).unwrap();
            let request = command(&[b"GET", b"x"]);
            let expected = request.clone();
            let reply = reply.to_vec();
            let server = tokio::spawn(async move {
                let mut input = vec![0; expected.len()];
                peer.read_exact(&mut input).await.unwrap();
                peer.write_all(&reply).await.unwrap();
            });
            assert_eq!(
                pipeline
                    .submit(0, request, Expected::Bulk(Some(Bytes::from_static(b"x"))))
                    .await,
                TargetOutcome::Error
            );
            server.await.unwrap();
            pipeline.drain().await.unwrap();
            let record = pipeline.records.lock().unwrap()[0].clone();
            assert!(record.received.is_none());
            assert!(!record.verified);
            assert!(record.failure.is_some());
            assert!(pipeline.shutdown().await.is_err());
        }
    }

    #[tokio::test]
    async fn duplicates_and_bounds_are_rejected_without_implicit_retry() {
        let (client, mut peer) = tokio::io::duplex(64);
        let pipeline = Pipeline::new(client, 1).unwrap();
        let request = command(&[b"GET", b"x"]);
        let expected = request.clone();
        let done = Arc::new(Notify::new());
        let held = Arc::clone(&done);
        let peer = tokio::spawn(async move {
            let mut input = vec![0; expected.len()];
            peer.read_exact(&mut input).await.unwrap();
            peer.write_all(b"-ERR denied\r\n").await.unwrap();
            held.notified().await;
        });
        assert_eq!(
            pipeline
                .submit(0, request.clone(), Expected::Bulk(None))
                .await,
            TargetOutcome::Error
        );
        assert_eq!(pipeline.records.lock().unwrap()[0].kind, Some("error"));
        assert_eq!(
            pipeline.submit(0, request, Expected::Bulk(None)).await,
            TargetOutcome::Rejected
        );
        assert_eq!(
            pipeline.submit(1, Bytes::new(), Expected::Bulk(None)).await,
            TargetOutcome::Rejected
        );
        pipeline.shutdown().await.unwrap();
        done.notify_one();
        peer.await.unwrap();
        let (client, _) = tokio::io::duplex(1);
        assert!(Pipeline::new(client, 2).is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn unresolved_wire_owner_refuses_clean_drain_then_shutdown_releases_it() {
        let (client, mut peer) = tokio::io::duplex(64);
        let pipeline = Arc::new(Pipeline::new(client, 1).unwrap());
        let request = command(&[b"GET", b"held"]);
        let expected = request.clone();
        let seen = Arc::new(Notify::new());
        let observed = Arc::clone(&seen);
        let finish = Arc::new(Notify::new());
        let held = Arc::clone(&finish);
        let server = tokio::spawn(async move {
            let mut input = vec![0; expected.len()];
            peer.read_exact(&mut input).await.unwrap();
            observed.notify_one();
            held.notified().await;
        });
        let p = Arc::clone(&pipeline);
        let caller = tokio::spawn(async move { p.submit(0, request, Expected::Bulk(None)).await });
        seen.notified().await;
        assert!(pipeline
            .drain()
            .await
            .unwrap_err()
            .contains("still pending"));
        assert_eq!(pipeline.slots.available_permits(), 0);
        pipeline.shutdown().await.unwrap();
        assert_eq!(caller.await.unwrap(), TargetOutcome::Error);
        assert_eq!(pipeline.slots.available_permits(), 1);
        assert!(pipeline.records.lock().unwrap()[0].received.is_none());
        finish.notify_one();
        server.await.unwrap();
    }
}
