//! Tool-only bounded RESP2/RESP3 controls with FIFO owners and timestamps.
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
const MAX_ARRAY: usize = 128;
const MAX_FRAME: usize = MAX_REPLY + MAX_HEADER + 4;
const MAX_HELLO_FRAME: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    Resp2,
    Resp3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Get,
    GetMissing,
    /// Write the same preloaded value; no cross-connection last-writer claim.
    Set,
    Mget {
        batch_size: usize,
    },
    Mset {
        batch_size: usize,
    },
    Exists {
        batch_size: usize,
    },
    /// Repeat an absent key: real live-removal semantics use separate fixtures.
    DelMissing {
        batch_size: usize,
    },
}
impl Operation {
    pub fn batch_size(self) -> usize {
        match self {
            Self::Get | Self::GetMissing | Self::Set => 1,
            Self::Mget { batch_size }
            | Self::Mset { batch_size }
            | Self::Exists { batch_size }
            | Self::DelMissing { batch_size } => batch_size,
        }
    }
    pub fn frame_kind(self) -> &'static str {
        match self {
            Self::Get => "bulk",
            Self::GetMissing => "null",
            Self::Set | Self::Mset { .. } => "simple",
            Self::Mget { .. } => "array",
            Self::Exists { .. } | Self::DelMissing { .. } => "integer",
        }
    }
}

#[derive(Clone)]
enum Expected {
    Bulk(Option<Bytes>),
    Stored,
    Array(Vec<Option<Bytes>>),
    Integer(i64),
    #[cfg(test)]
    ErrorContaining(&'static [u8]),
    #[cfg(test)]
    OneOfArrays(Vec<Vec<Option<Bytes>>>),
}
enum Frame<'a> {
    Bulk(Option<&'a [u8]>),
    Simple(&'a [u8]),
    Error(&'a [u8]),
    Array {
        body: &'a [u8],
        count: usize,
        dialect: Dialect,
    },
    Integer(i64),
}
impl Frame<'_> {
    fn items(&self) -> usize {
        match self {
            Self::Array { count, .. } => *count,
            _ => 1,
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Self::Bulk(Some(_)) => "bulk",
            Self::Bulk(None) => "null",
            Self::Simple(_) => "simple",
            Self::Error(_) => "error",
            Self::Array { .. } => "array",
            Self::Integer(_) => "integer",
        }
    }
    fn matches(&self, expected: &Expected) -> bool {
        match (self, expected) {
            #[cfg(test)]
            (_, Expected::OneOfArrays(arrays)) => arrays
                .iter()
                .any(|values| self.matches(&Expected::Array(values.clone()))),
            (Self::Bulk(actual), Expected::Bulk(value)) => *actual == value.as_deref(),
            (Self::Simple(b"OK"), Expected::Stored) => true,
            (Self::Integer(actual), Expected::Integer(expected)) => actual == expected,
            #[cfg(test)]
            (Self::Error(message), Expected::ErrorContaining(expected)) => {
                !expected.is_empty()
                    && message
                        .windows(expected.len())
                        .any(|part| part == *expected)
            }
            (
                Self::Array {
                    body,
                    count,
                    dialect,
                },
                Expected::Array(values),
            ) => {
                if *count != values.len() {
                    return false;
                }
                let mut remaining = *body;
                for value in values {
                    match decode_scalar(remaining, MAX_FRAME, *dialect) {
                        Ok(Some((Self::Bulk(actual), consumed))) if actual == value.as_deref() => {
                            remaining = &remaining[consumed..];
                        }
                        _ => return false,
                    }
                }
                remaining.is_empty()
            }
            _ => false,
        }
    }
}

/// Incremental bounded parser: only the response types used by this control.
#[cfg(test)]
fn decode(bytes: &[u8]) -> Result<Option<(Frame<'_>, usize)>> {
    decode_dialect(bytes, Dialect::Resp2)
}
fn decode_dialect(bytes: &[u8], dialect: Dialect) -> Result<Option<(Frame<'_>, usize)>> {
    if bytes.first() != Some(&b'*') {
        return decode_scalar(bytes, MAX_FRAME, dialect);
    }
    let Some(end) = header_end(bytes)? else {
        return Ok(None);
    };
    let count = unsigned_length(&bytes[1..end])?;
    if count > MAX_ARRAY {
        return Err("RESP2 flat array over budget".to_owned());
    }
    let body_start = end + 2;
    let mut offset = body_start;
    for _ in 0..count {
        if offset >= MAX_FRAME {
            return Err("RESP2 aggregate reply over budget".to_owned());
        }
        if bytes
            .get(offset)
            .is_some_and(|kind| *kind != b'$' && !(dialect == Dialect::Resp3 && *kind == b'_'))
        {
            return Err("RESP2 array requires flat bulk/null items".to_owned());
        }
        let Some((_, consumed)) = decode_scalar(&bytes[offset..], MAX_FRAME - offset, dialect)?
        else {
            return Ok(None);
        };
        offset += consumed;
    }
    Ok(Some((
        Frame::Array {
            body: &bytes[body_start..offset],
            count,
            dialect,
        },
        offset,
    )))
}
fn header_end(bytes: &[u8]) -> Result<Option<usize>> {
    let Some(end) = bytes.windows(2).position(|part| part == b"\r\n") else {
        if bytes.len() > MAX_HEADER + 1 {
            return Err("RESP2 reply header over budget".to_owned());
        }
        return Ok(None);
    };
    if end > MAX_HEADER {
        return Err("RESP2 reply header over budget".to_owned());
    }
    Ok(Some(end))
}
fn unsigned_length(length: &[u8]) -> Result<usize> {
    if length.is_empty() || !length.iter().all(u8::is_ascii_digit) {
        return Err("invalid RESP2 length".to_owned());
    }
    std::str::from_utf8(length)
        .map_err(|_| "invalid length")?
        .parse::<usize>()
        .map_err(|_| "RESP2 length overflow".to_owned())
}
fn decode_scalar(
    bytes: &[u8],
    budget: usize,
    dialect: Dialect,
) -> Result<Option<(Frame<'_>, usize)>> {
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes[0] == b'_' && dialect == Dialect::Resp3 {
        if budget < 3 {
            return Err("RESP3 null over aggregate budget".to_owned());
        }
        if bytes.len() < 3 {
            return Ok(None);
        }
        if &bytes[1..3] != b"\r\n" {
            return Err("invalid RESP3 null".to_owned());
        }
        return Ok(Some((Frame::Bulk(None), 3)));
    }
    if ![b'$', b'+', b'-', b':'].contains(&bytes[0]) {
        return Err("unsupported RESP2 reply type".to_owned());
    }
    let Some(end) = header_end(bytes)? else {
        return Ok(None);
    };
    if end + 2 > budget {
        return Err("RESP2 aggregate reply over budget".to_owned());
    }
    if bytes[0] == b':' {
        let integer = &bytes[1..end];
        let digits = integer.strip_prefix(b"-").unwrap_or(integer);
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
            return Err("invalid RESP2 integer".to_owned());
        }
        let value = std::str::from_utf8(integer)
            .map_err(|_| "invalid integer")?
            .parse::<i64>()
            .map_err(|_| "RESP2 integer overflow")?;
        return Ok(Some((Frame::Integer(value), end + 2)));
    }
    if bytes[0] == b'+' {
        return Ok(Some((Frame::Simple(&bytes[1..end]), end + 2)));
    }
    if bytes[0] == b'-' {
        return Ok(Some((Frame::Error(&bytes[1..end]), end + 2)));
    }
    let length = &bytes[1..end];
    if length == b"-1" {
        if dialect == Dialect::Resp3 {
            return Err("RESP2 null in a RESP3 reply".to_owned());
        }
        return Ok(Some((Frame::Bulk(None), end + 2)));
    }
    let size = unsigned_length(length)?;
    if size > MAX_REPLY {
        return Err("RESP2 bulk reply over budget".to_owned());
    }
    let total = end + 2 + size + 2;
    if total > budget {
        return Err("RESP2 aggregate reply over budget".to_owned());
    }
    if bytes.len() < total {
        return Ok(None);
    }
    if &bytes[total - 2..total] != b"\r\n" {
        return Err("invalid RESP2 bulk terminator".to_owned());
    }
    Ok(Some((Frame::Bulk(Some(&bytes[end + 2..total - 2])), total)))
}

// HELLO has one fixed shallow metadata shape. It is deliberately not part of
// the scheduled reply grammar: no general recursive map/array parser is added.
fn decode_hello(bytes: &[u8], dialect: Dialect) -> Result<Option<(usize, &[u8])>> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let expected_prefix = if dialect == Dialect::Resp3 {
        b'%'
    } else {
        b'*'
    };
    if bytes[0] != expected_prefix {
        return Err("HELLO metadata container mismatch".to_owned());
    }
    let Some(end) = header_end(bytes)? else {
        return Ok(None);
    };
    let expected_count = if dialect == Dialect::Resp3 { 7 } else { 14 };
    if unsigned_length(&bytes[1..end])? != expected_count {
        return Err("HELLO metadata count mismatch".to_owned());
    }
    let mut offset = end + 2;
    let mut version = None;
    let keys = [
        b"server".as_slice(),
        b"version",
        b"proto",
        b"id",
        b"mode",
        b"role",
        b"modules",
    ];
    let mut seen = [false; 7];
    for _ in 0..7 {
        let Some((name, consumed)) =
            decode_scalar(&bytes[offset..], MAX_HELLO_FRAME - offset, dialect)?
        else {
            return Ok(None);
        };
        let name = match (dialect, name) {
            (Dialect::Resp3, Frame::Simple(name)) | (Dialect::Resp2, Frame::Bulk(Some(name))) => {
                name
            }
            _ => return Err("HELLO metadata key type mismatch".to_owned()),
        };
        let index = keys
            .iter()
            .position(|key| *key == name)
            .ok_or("unknown HELLO metadata key")?;
        if seen[index] {
            return Err("duplicate HELLO metadata key".to_owned());
        }
        seen[index] = true;
        offset += consumed;
        if index == 6 {
            let remaining = &bytes[offset..];
            if remaining.len() < 4 {
                if !b"*0\r\n".starts_with(remaining) {
                    return Err("HELLO modules mismatch".to_owned());
                }
                return Ok(None);
            }
            if &remaining[..4] != b"*0\r\n" {
                return Err("HELLO modules mismatch".to_owned());
            }
            offset += 4;
            continue;
        }
        let Some((value, consumed)) =
            decode_scalar(&bytes[offset..], MAX_HELLO_FRAME - offset, dialect)?
        else {
            return Ok(None);
        };
        let valid = match (index, dialect, value) {
            (2, _, Frame::Integer(proto)) => proto == if dialect == Dialect::Resp3 { 3 } else { 2 },
            (3, _, Frame::Integer(id)) => id == 0,
            (field, Dialect::Resp3, Frame::Simple(text))
            | (field, Dialect::Resp2, Frame::Bulk(Some(text))) => match field {
                0 => text == b"hydracache",
                1 => {
                    version = Some(text);
                    valid_hello_version(text)
                }
                4 => text == b"standalone",
                5 => text == b"master",
                _ => false,
            },
            _ => false,
        };
        if !valid {
            return Err("HELLO metadata value mismatch".to_owned());
        }
        offset += consumed;
        if offset >= MAX_HELLO_FRAME {
            return Err("HELLO metadata over budget".to_owned());
        }
    }
    if offset > MAX_HELLO_FRAME {
        return Err("HELLO metadata over budget".to_owned());
    }
    Ok(Some((offset, version.ok_or("HELLO version missing")?)))
}
fn valid_hello_version(version: &[u8]) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-+".contains(byte))
}
async fn negotiate_resp3<S: AsyncRead + AsyncWrite + Unpin>(stream: &mut S) -> Result<String> {
    tokio::time::timeout(Duration::from_secs(5), negotiate_resp3_inner(stream))
        .await
        .map_err(|_| "HELLO negotiation timed out")?
}
async fn negotiate_resp3_inner<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
) -> Result<String> {
    stream
        .write_all(&command(&[b"HELLO", b"3"]))
        .await
        .map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    let mut scratch = [0u8; 256];
    loop {
        if let Some((consumed, version)) = decode_hello(&buffer, Dialect::Resp3)? {
            if consumed != buffer.len() {
                return Err("unsolicited bytes after HELLO".to_owned());
            }
            return String::from_utf8(version.to_vec()).map_err(|e| e.to_string());
        }
        let available = (MAX_HELLO_FRAME - buffer.len()).min(scratch.len());
        if available == 0 {
            return Err("HELLO metadata buffer over budget".to_owned());
        }
        let count = stream
            .read(&mut scratch[..available])
            .await
            .map_err(|e| e.to_string())?;
        if count == 0 {
            return Err("disconnect during HELLO".to_owned());
        }
        buffer.extend_from_slice(&scratch[..count]);
    }
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

fn make_request(
    entries: &[(Bytes, Bytes)],
    operation: Operation,
    sequence: u64,
) -> Result<(Bytes, Expected)> {
    let count = operation.batch_size();
    if entries.is_empty() || ![1, 8, 32, 128].contains(&count) {
        return Err("unsupported RESP2 batch size".to_owned());
    }
    let selected = (0..count)
        .map(|index| &entries[(sequence as usize + index) % entries.len()])
        .collect::<Vec<_>>();
    let mut parts: Vec<&[u8]> = Vec::new();
    let expected = match operation {
        Operation::Get => {
            parts.extend([b"GET".as_slice(), selected[0].0.as_ref()]);
            Expected::Bulk(Some(selected[0].1.clone()))
        }
        Operation::GetMissing => {
            parts.extend([b"GET".as_slice(), b"missing-key"]);
            Expected::Bulk(None)
        }
        Operation::Set => {
            parts.extend([
                b"SET".as_slice(),
                selected[0].0.as_ref(),
                selected[0].1.as_ref(),
            ]);
            Expected::Stored
        }
        Operation::Mget { .. } => {
            let reply_size =
                selected
                    .iter()
                    .fold(format!("*{count}\r\n").len(), |total, (_, value)| {
                        total
                            .saturating_add(format!("${}\r\n", value.len()).len() + value.len() + 2)
                    });
            if reply_size > MAX_FRAME {
                return Err("RESP2 aggregate reply over budget".to_owned());
            }
            parts.push(b"MGET");
            parts.extend(selected.iter().map(|(key, _)| key.as_ref()));
            Expected::Array(
                selected
                    .iter()
                    .map(|(_, value)| Some(value.clone()))
                    .collect(),
            )
        }
        Operation::Mset { .. } => {
            parts.push(b"MSET");
            for (key, value) in &selected {
                parts.extend([key.as_ref(), value.as_ref()]);
            }
            Expected::Stored
        }
        Operation::Exists { .. } => {
            parts.push(b"EXISTS");
            parts.extend(selected.iter().map(|(key, _)| key.as_ref()));
            Expected::Integer(count as i64)
        }
        Operation::DelMissing { .. } => {
            parts.push(b"DEL");
            parts.extend(std::iter::repeat_n(b"missing-key".as_slice(), count));
            Expected::Integer(0)
        }
    };
    // Size the borrowed parts before copying potentially repeated large payloads.
    let request_size = parts
        .iter()
        .fold(format!("*{}\r\n", parts.len()).len(), |total, part| {
            total.saturating_add(format!("${}\r\n", part.len()).len() + part.len() + 2)
        });
    if request_size > MAX_REPLY + 4096 {
        return Err("RESP2 aggregate request over budget".to_owned());
    }
    Ok((command(&parts), expected))
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
    items: Option<usize>,
    error_bytes: Option<Vec<u8>>,
    verified: bool,
    cancelled: bool,
    failure: Option<&'static str>,
}
fn record(
    pending: Pending,
    received: Option<Instant>,
    frame: Option<&Frame<'_>>,
    verified: bool,
    failure: Option<&'static str>,
    records: &Mutex<Vec<Record>>,
) {
    let kind = frame.map(Frame::kind);
    let items = frame.map(Frame::items);
    let error_bytes = frame.and_then(|value| match value {
        Frame::Error(message) => Some(message.to_vec()),
        _ => None,
    });
    let cancelled = pending.submission.reply.is_closed();
    records.lock().expect("tool response records").push(Record {
        sequence: pending.submission.sequence,
        ordinal: pending.ordinal,
        accepted: pending.submission.accepted,
        written: pending.written,
        received,
        kind,
        items,
        error_bytes,
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
    dialect: Dialect,
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
            while let Some((frame, consumed)) = decode_dialect(&buffer, dialect)? {
                let received = Instant::now();
                if pending.front().ok_or("unsolicited RESP2 reply")?.written.is_none() { return Err("reply before full request write".to_owned()); }
                let owner = pending.pop_front().expect("checked FIFO owner");
                let verified = frame.matches(&owner.submission.expected);
                record(owner, Some(received), Some(&frame), verified, None, &records);
                buffer.drain(..consumed);
            }
            let available = (MAX_FRAME - buffer.len()).min(scratch.len());
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
    dialect: Dialect,
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
        Self::new_dialect(stream, depth, Dialect::Resp2)
    }
    fn new_dialect<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        stream: S,
        depth: usize,
        dialect: Dialect,
    ) -> Result<Self> {
        if ![1, 10, 50].contains(&depth) {
            return Err("unsupported RESP2 pipeline depth".to_owned());
        }
        let (requests, receiver) = mpsc::channel(depth);
        let (stop, shutdown) = watch::channel(false);
        let records = Arc::new(Mutex::new(Vec::new()));
        let actor = tokio::spawn(pump(
            stream,
            dialect,
            receiver,
            shutdown,
            Arc::clone(&records),
        ));
        Ok(Self {
            dialect,
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

async fn drain_pipelines(pipelines: &[Pipeline]) -> Result<()> {
    // One group budget; a late socket cannot multiply the allowed drain window.
    tokio::time::timeout(Duration::from_secs(5), async {
        for pipeline in pipelines {
            pipeline.drain().await?;
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|_| "RESP2 connection group wire drain timed out")?
}

#[derive(Serialize)]
pub struct WireSample {
    pub dialect: Dialect,
    pub sequence: u64,
    pub connection_id: usize,
    pub wire_ordinal: u64,
    pub scheduled_ns: u64,
    pub accepted_ns: u64,
    pub write_completed_ns: Option<u64>,
    pub response_complete_ns: Option<u64>,
    pub scheduled_frame_latency_ns: Option<u64>,
    pub frame_kind: Option<&'static str>,
    pub response_items: Option<usize>,
    pub protocol_error_bytes: Option<Vec<u8>>,
    pub byte_oracle_verified: bool,
    pub waiting_caller_cancelled: bool,
    pub transport_failure: Option<&'static str>,
    pub operation_outcome: scheduled::Outcome,
}
#[derive(Serialize)]
pub struct RespObservation {
    pub dialect: Dialect,
    pub hello_connections: usize,
    pub hello_server_version: Option<String>,
    pub operations: Observation,
    pub wire_samples: Vec<WireSample>,
    pub pipeline_limit: usize,
    pub physical_connections: usize,
    pub operation: Operation,
    pub batch_size: usize,
    pub product_performance_claim: bool,
}
/// Bounded real TCP connections, independent FIFOs, one shared fixture store.
pub struct RespControl {
    dialect: Dialect,
    hello_server_version: Option<String>,
    pipelines: Vec<Pipeline>,
    entries: Vec<(Bytes, Bytes)>,
    state: Arc<ClientSurfaceState>,
    server: Arc<RedisRespServer>,
    listeners: Vec<JoinHandle<Result<()>>>,
    digest: String,
    depth: usize,
    operation: Operation,
    setup_id: std::sync::atomic::AtomicU64,
    used: std::sync::atomic::AtomicBool,
}
impl Drop for RespControl {
    fn drop(&mut self) {
        for listener in &self.listeners {
            listener.abort();
        }
    }
}
impl RespControl {
    pub async fn start(dataset: Dataset, depth: usize) -> Result<Self> {
        Self::start_connections(dataset, depth, 1, Operation::Get).await
    }
    pub async fn start_connections(
        dataset: Dataset,
        depth: usize,
        connections: usize,
        operation: Operation,
    ) -> Result<Self> {
        Self::start_connections_dialect(dataset, depth, connections, operation, Dialect::Resp2)
            .await
    }
    pub async fn start_connections_dialect(
        dataset: Dataset,
        depth: usize,
        connections: usize,
        operation: Operation,
        dialect: Dialect,
    ) -> Result<Self> {
        if ![1, 10, 50].contains(&depth) {
            return Err("unsupported RESP2 pipeline depth".to_owned());
        }
        if ![1, 8, 32, 128].contains(&connections) {
            return Err("unsupported RESP2 connection count".to_owned());
        }
        // Reject unsupported count/aggregate payload before opening any socket.
        make_request(&dataset.entries(), operation, 0)?;
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
        // Construct the owner before connecting: every error path drops/aborts
        // already-created client actors and server tasks instead of detaching them.
        let mut control = Self {
            dialect,
            hello_server_version: None,
            pipelines: Vec::new(),
            entries: dataset.entries(),
            state,
            server,
            listeners: Vec::new(),
            digest: dataset.digest(),
            depth,
            operation,
            setup_id: std::sync::atomic::AtomicU64::new(u64::MAX),
            used: std::sync::atomic::AtomicBool::new(false),
        };
        let address = tcp.local_addr().map_err(|e| e.to_string())?;
        for _ in 0..connections {
            let mut client = tokio::net::TcpStream::connect(address)
                .await
                .map_err(|e| e.to_string())?;
            client.set_nodelay(true).map_err(|e| e.to_string())?;
            let (stream, _) = tcp.accept().await.map_err(|e| e.to_string())?;
            stream.set_nodelay(true).map_err(|e| e.to_string())?;
            let observed = Arc::clone(&control.server);
            control.listeners.push(tokio::spawn(async move {
                observed
                    .serve_connection(stream)
                    .await
                    .map_err(|e| e.to_string())
            }));
            if dialect == Dialect::Resp3 {
                let version = negotiate_resp3(&mut client).await?;
                if control
                    .hello_server_version
                    .as_ref()
                    .is_some_and(|previous| previous != &version)
                {
                    return Err("HELLO version differs across sockets".to_owned());
                }
                control.hello_server_version = Some(version);
                control
                    .pipelines
                    .push(Pipeline::new_dialect(client, depth, dialect)?);
            } else {
                control.pipelines.push(Pipeline::new(client, depth)?);
            }
        }
        // Preload once through TCP, then verify visibility through EVERY socket.
        for (key, value) in &control.entries {
            if control.pipelines[0]
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
    pub fn physical_connections(&self) -> usize {
        self.pipelines.len()
    }
    fn setup_id(&self) -> u64 {
        self.setup_id
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
    }
    pub async fn verify(&self) -> Result<String> {
        for pipeline in &self.pipelines {
            for (key, value) in &self.entries {
                if pipeline
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
            if pipeline
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
        drain_pipelines(&self.pipelines).await?;
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
        let mut wire_samples = Vec::new();
        for (connection_id, pipeline) in self.pipelines.iter().enumerate() {
            let records = pipeline
                .records
                .lock()
                .map_err(|_| "record lock poisoned")?;
            wire_samples.extend(
                records
                    .iter()
                    .filter(|r| r.sequence < config.operations)
                    .map(|r| {
                        let scheduled_ns = schedule.scheduled_ns(r.sequence);
                        WireSample {
                            dialect: pipeline.dialect,
                            sequence: r.sequence,
                            connection_id,
                            wire_ordinal: r.ordinal,
                            scheduled_ns,
                            accepted_ns: offset(r.accepted),
                            write_completed_ns: r.written.map(offset),
                            response_complete_ns: r.received.map(offset),
                            scheduled_frame_latency_ns: r
                                .received
                                .map(|at| offset(at).saturating_sub(scheduled_ns)),
                            frame_kind: r.kind,
                            response_items: r.items,
                            protocol_error_bytes: r.error_bytes.clone(),
                            byte_oracle_verified: r.verified,
                            waiting_caller_cancelled: r.cancelled,
                            transport_failure: r.failure,
                            operation_outcome: operations.samples[r.sequence as usize].outcome,
                        }
                    }),
            );
        }
        let result = RespObservation {
            dialect: self.dialect,
            hello_connections: if self.dialect == Dialect::Resp3 {
                self.pipelines.len()
            } else {
                0
            },
            hello_server_version: self.hello_server_version.clone(),
            operations,
            wire_samples,
            pipeline_limit: self.depth,
            physical_connections: self.pipelines.len(),
            operation: self.operation,
            batch_size: self.operation.batch_size(),
            product_performance_claim: false,
        };
        validate_wire(&result)?;
        Ok(result)
    }
    pub async fn shutdown(mut self) -> Result<()> {
        let mut failures = Vec::new();
        for pipeline in &self.pipelines {
            if let Err(error) = pipeline.shutdown().await {
                failures.push(error);
            }
        }
        // Visit every owned server task even if an earlier connection failed.
        for mut listener in self.listeners.drain(..) {
            match tokio::time::timeout(Duration::from_secs(5), &mut listener).await {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(error))) => failures.push(error),
                Ok(Err(error)) => failures.push(error.to_string()),
                Err(_) => {
                    listener.abort();
                    let _ = listener.await;
                    failures.push("RESP2 connection drain timed out".to_owned());
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
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
        let sequence = request.sequence;
        let (frame, expected) = match make_request(&self.entries, self.operation, sequence) {
            Ok(request) => request,
            Err(_) => return TargetOutcome::Rejected,
        };
        self.pipelines[sequence as usize % self.pipelines.len()]
            .submit(sequence, frame, expected)
            .await
    }
}

/// Reject response reassignment, timestamp rebasing and fabricated wire replies.
pub fn validate_wire(observation: &RespObservation) -> Result<()> {
    match observation.dialect {
        Dialect::Resp2
            if observation.hello_connections != 0 || observation.hello_server_version.is_some() =>
        {
            return Err("unexpected RESP2 HELLO receipt".to_owned());
        }
        Dialect::Resp3
            if observation.hello_connections != observation.physical_connections
                || observation
                    .hello_server_version
                    .as_ref()
                    .is_none_or(|v| !valid_hello_version(v.as_bytes())) =>
        {
            return Err("missing or invalid RESP3 HELLO receipt".to_owned());
        }
        _ => {}
    }
    if ![1, 8, 32, 128].contains(&observation.physical_connections)
        || ![1, 10, 50].contains(&observation.pipeline_limit)
        || observation.product_performance_claim
        || ![1, 8, 32, 128].contains(&observation.batch_size)
        || observation.batch_size != observation.operation.batch_size()
    {
        return Err("unsupported RESP2 wire topology or claim".to_owned());
    }
    let schedule =
        FixedRateSchedule::new(0, observation.operations.config.offered_rate_per_second)?;
    let mut seen = HashSet::new();
    let mut previous_ordinals = vec![None; observation.physical_connections];
    let mut previous_responses = vec![None; observation.physical_connections];
    for sample in &observation.wire_samples {
        if sample.dialect != observation.dialect
            || sample.connection_id >= observation.physical_connections
            || sample.sequence as usize % observation.physical_connections != sample.connection_id
        {
            return Err("wire connection or deterministic route drift".to_owned());
        }
        let previous_ordinal = &mut previous_ordinals[sample.connection_id];
        let previous_response = &mut previous_responses[sample.connection_id];
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
        *previous_ordinal = Some(sample.wire_ordinal);
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
                    || sample.response_items.is_none()
                    || match sample.frame_kind {
                        Some("array") => {
                            sample.response_items.is_none_or(|items| items > MAX_ARRAY)
                        }
                        Some("bulk" | "null" | "simple" | "integer" | "error") => {
                            sample.response_items != Some(1)
                        }
                        _ => true,
                    }
                    || (sample.frame_kind == Some("error")) != sample.protocol_error_bytes.is_some()
                    || sample
                        .protocol_error_bytes
                        .as_ref()
                        .is_some_and(|bytes| bytes.len() > MAX_HEADER)
                    || sample.transport_failure.is_some()
                    || sample.scheduled_frame_latency_ns != Some(response - sample.scheduled_ns)
                {
                    return Err("wire response boundary drift".to_owned());
                }
                *previous_response = Some(response);
            }
            None => {
                if sample.byte_oracle_verified
                    || sample.frame_kind.is_some()
                    || sample.response_items.is_some()
                    || sample.protocol_error_bytes.is_some()
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
                || sample.frame_kind != Some(observation.operation.frame_kind())
                || sample.response_items
                    != Some(if matches!(observation.operation, Operation::Mget { .. }) {
                        observation.batch_size
                    } else {
                        1
                    })
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

    fn hello3_fixture(rotation: usize) -> Vec<u8> {
        let fields = [
            (b"server".as_slice(), b"+hydracache\r\n".as_slice()),
            (b"version".as_slice(), b"+0.73.0\r\n".as_slice()),
            (b"proto".as_slice(), b":3\r\n".as_slice()),
            (b"id".as_slice(), b":0\r\n".as_slice()),
            (b"mode".as_slice(), b"+standalone\r\n".as_slice()),
            (b"role".as_slice(), b"+master\r\n".as_slice()),
            (b"modules".as_slice(), b"*0\r\n".as_slice()),
        ];
        let mut bytes = b"%7\r\n".to_vec();
        for position in 0..7 {
            let (key, value) = fields[(position + rotation) % 7];
            bytes.push(b'+');
            bytes.extend_from_slice(key);
            bytes.extend_from_slice(b"\r\n");
            bytes.extend_from_slice(value);
        }
        bytes
    }

    #[test]
    fn resp3_hello_metadata_is_shallow_bounded_and_map_order_independent() {
        for rotation in 0..7 {
            let bytes = hello3_fixture(rotation);
            for split in 0..bytes.len() {
                assert!(
                    decode_hello(&bytes[..split], Dialect::Resp3)
                        .unwrap()
                        .is_none(),
                    "rotation={rotation}, split={split}"
                );
            }
            let (used, version) = decode_hello(&bytes, Dialect::Resp3).unwrap().unwrap();
            assert_eq!(used, bytes.len());
            assert_eq!(version, b"0.73.0");
            assert!(decode_dialect(&bytes, Dialect::Resp3).is_err()); // Maps are setup-only.
            assert!(decode_hello(&bytes, Dialect::Resp2).is_err());
        }
        for bytes in [
            b"%8\r\n".as_slice(),
            b"%7\r\n+unknown\r\n",
            b"%7\r\n$6\r\nserver\r\n",
            b"%7\r\n+server\r\n+hydracache\r\n+server\r\n",
            b"%7\r\n+proto\r\n:2\r\n",
            b"%7\r\n+modules\r\n*1\r\n",
            b"%7\r\n+version\r\n+\r\n",
            b"%7\r\n+server\r\n$1048576\r\n",
        ] {
            assert!(decode_hello(bytes, Dialect::Resp3).is_err(), "{bytes:?}");
        }
        let mut header = b"%7\r\n+".to_vec();
        header.extend(std::iter::repeat_n(b'a', MAX_HEADER + 1));
        assert!(decode_hello(&header, Dialect::Resp3).is_err());
        assert!(!valid_hello_version(b"\0"));
        assert!(!valid_hello_version(&[b'a'; 65]));
    }

    #[test]
    fn resp3_null_arrays_are_fragmented_without_legacy_null_fallback() {
        let bytes = b"*4\r\n$3\r\n\0\xffb\r\n_\r\n$0\r\n\r\n_\r\n";
        for split in 0..bytes.len() {
            assert!(decode_dialect(&bytes[..split], Dialect::Resp3)
                .unwrap()
                .is_none());
        }
        assert!(decode_dialect(bytes, Dialect::Resp3)
            .unwrap()
            .unwrap()
            .0
            .matches(&Expected::Array(vec![
                Some(Bytes::from_static(b"\0\xffb")),
                None,
                Some(Bytes::new()),
                None,
            ])));
        for split in 0..3 {
            assert!(decode_dialect(&b"_\r\n"[..split], Dialect::Resp3)
                .unwrap()
                .is_none());
        }
        assert!(decode_dialect(b"_\r\n", Dialect::Resp3)
            .unwrap()
            .unwrap()
            .0
            .matches(&Expected::Bulk(None)));
        assert!(decode(b"_\r\n").is_err());
        assert!(decode(bytes).is_err());
        for bytes in [
            b"$-1\r\n".as_slice(),
            b"*1\r\n$-1\r\n",
            b"_x\n",
            b"*1\r\n*0\r\n",
            b">0\r\n",
            b"%0\r\n",
            b"*129\r\n",
        ] {
            assert!(decode_dialect(bytes, Dialect::Resp3).is_err(), "{bytes:?}");
        }
        assert!(decode_scalar(b"_\r\n", 2, Dialect::Resp3).is_err());
        let mut maximum = b"*128\r\n".to_vec();
        maximum.extend_from_slice(&b"_\r\n".repeat(128));
        maximum.extend_from_slice(b":2\r\n");
        let (frame, used) = decode_dialect(&maximum, Dialect::Resp3).unwrap().unwrap();
        assert!(frame.matches(&Expected::Array(vec![None; 128])));
        assert!(decode_dialect(&maximum[used..], Dialect::Resp3)
            .unwrap()
            .unwrap()
            .0
            .matches(&Expected::Integer(2)));
    }

    #[tokio::test]
    async fn hello_negotiation_refuses_error_disconnect_and_unsolicited_tail() {
        let mut tail = hello3_fixture(0);
        tail.extend_from_slice(b"+OK\r\n");
        for reply in [Vec::new(), b"-ERR unsupported\r\n".to_vec(), tail] {
            let (mut client, mut peer) = tokio::io::duplex(4096);
            let server = tokio::spawn(async move {
                let hello = command(&[b"HELLO", b"3"]);
                let mut input = vec![0; hello.len()];
                peer.read_exact(&mut input).await.unwrap();
                assert_eq!(input, hello);
                peer.write_all(&reply).await.unwrap();
                peer.shutdown().await.unwrap();
            });
            assert!(negotiate_resp3(&mut client).await.is_err());
            server.await.unwrap();
        }
    }

    #[tokio::test(start_paused = true)]
    async fn hello_negotiation_timeout_drops_socket_without_fallback() {
        let (mut client, mut peer) = tokio::io::duplex(64);
        let handshake = tokio::spawn(async move { negotiate_resp3(&mut client).await });
        let hello = command(&[b"HELLO", b"3"]);
        let mut request = vec![0; hello.len()];
        peer.read_exact(&mut request).await.unwrap();
        assert_eq!(request, hello);
        tokio::time::advance(Duration::from_secs(5)).await;
        assert_eq!(
            handshake.await.unwrap().unwrap_err(),
            "HELLO negotiation timed out"
        );
        assert!(peer.write_all(b"+late\r\n").await.is_err());
    }

    #[tokio::test]
    async fn real_resp2_resp3_mget_preserves_null_empty_binary_and_duplicate_positions() {
        for dialect in [Dialect::Resp2, Dialect::Resp3] {
            let control = RespControl::start_connections_dialect(
                Dataset::new(1, 16).unwrap(),
                10,
                1,
                Operation::Get,
                dialect,
            )
            .await
            .unwrap();
            let key = &control.entries[0].0;
            let value = control.entries[0].1.clone();
            assert_eq!(
                control.pipelines[0]
                    .submit(
                        control.setup_id(),
                        command(&[b"SET", b"empty", b""]),
                        Expected::Stored
                    )
                    .await,
                TargetOutcome::Success
            );
            assert_eq!(
                control.pipelines[0]
                    .submit(
                        control.setup_id(),
                        command(&[b"MGET", key, b"missing-key", b"empty", key]),
                        Expected::Array(vec![
                            Some(value.clone()),
                            None,
                            Some(Bytes::new()),
                            Some(value)
                        ])
                    )
                    .await,
                TargetOutcome::Success
            );
            assert_eq!(
                control.pipelines[0]
                    .submit(
                        control.setup_id(),
                        command(&[b"DEL", b"empty"]),
                        Expected::Integer(1)
                    )
                    .await,
                TargetOutcome::Success
            );
            control.verify().await.unwrap();
            control.shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn real_hello_transitions_apply_to_the_next_pipelined_reply() {
        let control = RespControl::start(Dataset::new(1, 16).unwrap(), 1)
            .await
            .unwrap();
        let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = tokio::net::TcpStream::connect(tcp.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, _) = tcp.accept().await.unwrap();
        let observed = Arc::clone(&control.server);
        let server = tokio::spawn(async move { observed.serve_connection(stream).await });
        let mut requests = Vec::new();
        for parts in [
            &[b"HELLO".as_slice(), b"3"][..],
            &[b"GET".as_slice(), b"missing-key"][..],
            &[b"HELLO".as_slice(), b"2"][..],
            &[b"GET".as_slice(), b"missing-key"][..],
        ] {
            requests.extend_from_slice(&command(parts));
        }
        client.write_all(&requests).await.unwrap();
        let mut buffer = Vec::new();
        for (hello, dialect) in [
            (true, Dialect::Resp3),
            (false, Dialect::Resp3),
            (true, Dialect::Resp2),
            (false, Dialect::Resp2),
        ] {
            loop {
                let complete = if hello {
                    decode_hello(&buffer, dialect)
                        .unwrap()
                        .map(|(used, _)| used)
                } else {
                    decode_dialect(&buffer, dialect)
                        .unwrap()
                        .map(|(frame, used)| {
                            assert!(frame.matches(&Expected::Bulk(None)));
                            used
                        })
                };
                if let Some(used) = complete {
                    buffer.drain(..used);
                    break;
                }
                let mut scratch = [0; 64];
                let count = client.read(&mut scratch).await.unwrap();
                assert_ne!(count, 0);
                buffer.extend_from_slice(&scratch[..count]);
                assert!(buffer.len() <= MAX_HELLO_FRAME);
            }
        }
        assert!(buffer.is_empty());
        client.shutdown().await.unwrap();
        drop(client);
        server.await.unwrap().unwrap();
        control.verify().await.unwrap();
        control.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn seeded_real_batch_sequence_matches_independent_reference_map() {
        let seed = crate::native::SEED;
        eprintln!("RESP batch reference seed={seed}, replay rounds=64");
        let mut random = seed;
        let mut next = || {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            random
        };
        let control = RespControl::start(Dataset::new(1, 16).unwrap(), 10)
            .await
            .unwrap();
        let keys = (0..4)
            .map(|i| Bytes::from(vec![b'm', 0, 255, i]))
            .collect::<Vec<_>>();
        let mut model = std::collections::BTreeMap::<Bytes, Bytes>::new();
        let mut covered = [false; 4];
        for round in 0..64 {
            let operation = next() as usize % 4;
            covered[operation] = true;
            let positions = (0..8)
                .map(|_| next() as usize % keys.len())
                .collect::<Vec<_>>();
            let values = (0..8)
                .map(|_| match next() % 3 {
                    0 => Bytes::new(),
                    1 => Bytes::from_static(b"\0\xff\r\n"),
                    _ => Bytes::from_static(b"value"),
                })
                .collect::<Vec<_>>();
            let mut parts: Vec<&[u8]> = vec![match operation {
                0 => b"MSET",
                1 => b"MGET",
                2 => b"DEL",
                _ => b"EXISTS",
            }];
            let expected = match operation {
                0 => {
                    for (index, value) in positions.iter().zip(&values) {
                        parts.extend([keys[*index].as_ref(), value.as_ref()]);
                        model.insert(keys[*index].clone(), value.clone());
                    }
                    Expected::Stored
                }
                1 => {
                    parts.extend(positions.iter().map(|index| keys[*index].as_ref()));
                    Expected::Array(
                        positions
                            .iter()
                            .map(|index| model.get(&keys[*index]).cloned())
                            .collect(),
                    )
                }
                2 => {
                    parts.extend(positions.iter().map(|index| keys[*index].as_ref()));
                    let removed = positions
                        .iter()
                        .filter(|index| model.remove(&keys[**index]).is_some())
                        .count();
                    Expected::Integer(removed as i64)
                }
                _ => {
                    parts.extend(positions.iter().map(|index| keys[*index].as_ref()));
                    Expected::Integer(
                        positions
                            .iter()
                            .filter(|index| model.contains_key(&keys[**index]))
                            .count() as i64,
                    )
                }
            };
            assert_eq!(
                control.pipelines[0]
                    .submit(control.setup_id(), command(&parts), expected)
                    .await,
                TargetOutcome::Success,
                "seed={seed}, round={round}, op={operation}"
            );
        }
        assert!(covered.into_iter().all(|value| value));
        let mut parts = vec![b"MGET".as_slice()];
        parts.extend(keys.iter().map(Bytes::as_ref));
        assert_eq!(
            control.pipelines[0]
                .submit(
                    control.setup_id(),
                    command(&parts),
                    Expected::Array(keys.iter().map(|key| model.get(key).cloned()).collect())
                )
                .await,
            TargetOutcome::Success
        );
        control.verify().await.unwrap();
        control.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn real_batch_duplicate_order_and_oversized_mset_are_atomic() {
        let control = RespControl::start(Dataset::new(1, 16).unwrap(), 10)
            .await
            .unwrap();
        let pipeline = &control.pipelines[0];
        let a = b"\0\xffa".as_slice();
        let b = b"\0\xffb".as_slice();
        let missing = b"missing-model".as_slice();
        let binary = Bytes::from_static(b"\0\xff\r\nvalue");
        assert_eq!(
            pipeline
                .submit(
                    control.setup_id(),
                    command(&[b"MSET", a, b"old", b, &binary, a, b"",]),
                    Expected::Stored
                )
                .await,
            TargetOutcome::Success
        );
        assert_eq!(
            pipeline
                .submit(
                    control.setup_id(),
                    command(&[b"MGET", b, a, missing, b]),
                    Expected::Array(vec![
                        Some(binary.clone()),
                        Some(Bytes::new()),
                        None,
                        Some(binary)
                    ])
                )
                .await,
            TargetOutcome::Success
        );
        assert_eq!(
            pipeline
                .submit(
                    control.setup_id(),
                    command(&[b"EXISTS", a, missing, a, b]),
                    Expected::Integer(3)
                )
                .await,
            TargetOutcome::Success
        );
        assert_eq!(
            pipeline
                .submit(
                    control.setup_id(),
                    command(&[b"DEL", a, a, missing, b]),
                    Expected::Integer(2)
                )
                .await,
            TargetOutcome::Success
        );
        assert_eq!(
            pipeline
                .submit(
                    control.setup_id(),
                    command(&[b"MGET", a, b]),
                    Expected::Array(vec![None, None])
                )
                .await,
            TargetOutcome::Success
        );
        assert_eq!(
            pipeline
                .submit(
                    control.setup_id(),
                    command(&[b"SET", a, b"kept"]),
                    Expected::Stored
                )
                .await,
            TargetOutcome::Success
        );
        let mut keys = (0..255)
            .map(|i| format!("oversized:{i}").into_bytes())
            .collect::<Vec<_>>();
        keys.insert(0, a.to_vec());
        let mut parts = vec![b"MSET".as_slice()];
        for key in &keys {
            parts.extend([key.as_slice(), b"changed".as_slice()]);
        }
        assert_eq!(
            pipeline
                .submit(
                    control.setup_id(),
                    command(&parts),
                    Expected::ErrorContaining(b"request too large")
                )
                .await,
            TargetOutcome::Success
        );
        for (chunk_index, chunk) in keys.chunks(128).enumerate() {
            let mut parts = vec![b"MGET".as_slice()];
            parts.extend(chunk.iter().map(Vec::as_slice));
            let mut expected = vec![None; chunk.len()];
            if chunk_index == 0 {
                expected[0] = Some(Bytes::from_static(b"kept"));
            }
            assert_eq!(
                pipeline
                    .submit(
                        control.setup_id(),
                        command(&parts),
                        Expected::Array(expected)
                    )
                    .await,
                TargetOutcome::Success
            );
        }
        assert_eq!(
            pipeline
                .submit(
                    control.setup_id(),
                    command(&[b"DEL", a]),
                    Expected::Integer(1)
                )
                .await,
            TargetOutcome::Success
        );
        control.verify().await.unwrap();
        control.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_real_mset_mget_never_observes_partial_pair() {
        let control =
            RespControl::start_connections(Dataset::new(1, 16).unwrap(), 10, 8, Operation::Get)
                .await
                .unwrap();
        let a = b"atomic:a".as_slice();
        let b = b"atomic:b".as_slice();
        let values = [Bytes::from_static(b"v0"), Bytes::from_static(b"v1")];
        assert_eq!(
            control.pipelines[0]
                .submit(
                    control.setup_id(),
                    command(&[b"MSET", a, &values[0], b, &values[0]]),
                    Expected::Stored
                )
                .await,
            TargetOutcome::Success
        );
        for round in 1..=32 {
            let current = &values[round % 2];
            let previous = &values[(round - 1) % 2];
            let allowed = Expected::OneOfArrays(vec![
                vec![Some(previous.clone()), Some(previous.clone())],
                vec![Some(current.clone()), Some(current.clone())],
            ]);
            let (write, read) = tokio::join!(
                control.pipelines[0].submit(
                    control.setup_id(),
                    command(&[b"MSET", a, current, b, current]),
                    Expected::Stored
                ),
                control.pipelines[1].submit(control.setup_id(), command(&[b"MGET", a, b]), allowed),
            );
            assert_eq!(write, TargetOutcome::Success);
            assert_eq!(read, TargetOutcome::Success, "mixed pair at round={round}");
        }
        assert_eq!(
            control.pipelines[0]
                .submit(
                    control.setup_id(),
                    command(&[b"DEL", a, b]),
                    Expected::Integer(2)
                )
                .await,
            TargetOutcome::Success
        );
        control.verify().await.unwrap();
        control.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn cancelled_fragmented_array_keeps_one_command_owner() {
        for dialect in [Dialect::Resp2, Dialect::Resp3] {
            let (client, mut peer) = tokio::io::duplex(128);
            let pipeline = Arc::new(Pipeline::new_dialect(client, 1, dialect).unwrap());
            let request = command(&[b"MGET", b"a", b"b"]);
            let p = Arc::clone(&pipeline);
            let first = request.clone();
            let waiter = tokio::spawn(async move {
                p.submit(
                    0,
                    first,
                    Expected::Array(vec![Some(Bytes::from_static(b"a")), None]),
                )
                .await
            });
            let mut input = vec![0; request.len()];
            peer.read_exact(&mut input).await.unwrap();
            let fragmented = if dialect == Dialect::Resp3 {
                b"*2\r\n$1\r\na\r\n_\r".as_slice()
            } else {
                b"*2\r\n$1\r\na\r\n$-1\r".as_slice()
            };
            peer.write_all(fragmented).await.unwrap();
            waiter.abort();
            assert!(waiter.await.unwrap_err().is_cancelled());
            assert_eq!(pipeline.slots.available_permits(), 0);
            let p = Arc::clone(&pipeline);
            let second = command(&[b"EXISTS", b"a", b"a"]);
            let next_size = second.len();
            let next = tokio::spawn(async move { p.submit(1, second, Expected::Integer(2)).await });
            peer.write_all(b"\n").await.unwrap();
            peer.read_exact(&mut vec![0; next_size]).await.unwrap();
            peer.write_all(b":2\r\n").await.unwrap();
            assert_eq!(next.await.unwrap(), TargetOutcome::Success);
            pipeline.drain().await.unwrap();
            let records = pipeline.records.lock().unwrap().clone();
            assert_eq!(records.len(), 2);
            assert!(records[0].cancelled && records[0].verified);
            assert_eq!(records[0].items, Some(2));
            assert_eq!(records[1].items, Some(1));
            assert_eq!(records[1].sequence, 1);
            pipeline.shutdown().await.unwrap();
        }
    }

    #[test]
    fn flat_array_integer_fragmentation_and_aggregate_budget_are_explicit() {
        let encoded = b"*4\r\n$1\r\na\r\n$-1\r\n$0\r\n\r\n$3\r\n\0\xffb\r\n";
        for split in 0..encoded.len() {
            assert!(
                decode(&encoded[..split]).unwrap().is_none(),
                "split={split}"
            );
        }
        let expected = Expected::Array(vec![
            Some(Bytes::from_static(b"a")),
            None,
            Some(Bytes::new()),
            Some(Bytes::from_static(b"\0\xffb")),
        ]);
        let (frame, consumed) = decode(encoded).unwrap().unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(frame.kind(), "array");
        assert!(frame.matches(&expected));
        assert!(!frame.matches(&Expected::Array(vec![None; 4])));
        assert!(!frame.matches(&Expected::Array(Vec::new())));
        assert!(decode(b"*0\r\n")
            .unwrap()
            .unwrap()
            .0
            .matches(&Expected::Array(Vec::new())));
        for (bytes, integer) in [
            (b":0\r\n".as_slice(), 0),
            (b":-12\r\n", -12),
            (b":9223372036854775807\r\n", i64::MAX),
        ] {
            for split in 0..bytes.len() {
                assert!(decode(&bytes[..split]).unwrap().is_none());
            }
            assert!(decode(bytes)
                .unwrap()
                .unwrap()
                .0
                .matches(&Expected::Integer(integer)));
        }
        for bad in [
            b"*129\r\n".as_slice(),
            b"*-1\r\n",
            b"*1\r\n*0\r\n",
            b"*1\r\n+OK\r\n",
            b":+1\r\n",
            b":-\r\n",
            b":9223372036854775808\r\n",
            b"*\r\n",
        ] {
            assert!(decode(bad).is_err(), "{bad:?}");
        }
        let mut too_large = b"*2\r\n$1048576\r\n".to_vec();
        too_large.extend(std::iter::repeat_n(b'x', MAX_REPLY));
        too_large.extend_from_slice(b"\r\n$128\r\n");
        assert!(decode(&too_large).is_err());
        let mut maximum = b"*128\r\n".to_vec();
        for _ in 0..128 {
            maximum.extend_from_slice(b"$-1\r\n");
        }
        maximum.extend_from_slice(b":2\r\n");
        let (frame, used) = decode(&maximum).unwrap().unwrap();
        assert!(frame.matches(&Expected::Array(vec![None; 128])));
        assert!(decode(&maximum[used..])
            .unwrap()
            .unwrap()
            .0
            .matches(&Expected::Integer(2)));
    }

    #[tokio::test]
    async fn one_connection_failure_still_joins_every_owned_task() {
        let control = Arc::new(
            RespControl::start_connections(Dataset::new(1, 16).unwrap(), 1, 8, Operation::Get)
                .await
                .unwrap(),
        );
        let server_tasks = control
            .listeners
            .iter()
            .map(|task| task.abort_handle())
            .collect::<Vec<_>>();
        let client_tasks = control
            .pipelines
            .iter()
            .map(|pipeline| {
                pipeline
                    .actor
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .abort_handle()
            })
            .collect::<Vec<_>>();
        // Abort only this fixture-owned server connection, not a process/service.
        control.listeners[0].abort();
        let result = control
            .run(&Config {
                operations: 8,
                offered_rate_per_second: 10000,
                concurrency: 8,
                maximum_queued: 8,
                operation_timeout_ns: 5_000_000_000,
                drain_timeout_ns: 5_000_000_000,
                slo_ns: 5_000_000_000,
                highest_trackable_ns: 10_000_000_000,
            })
            .await
            .unwrap();
        assert_eq!(result.operations.successes, 7);
        assert_eq!(result.operations.errors, 1);
        assert_eq!(
            result.operations.samples[0].outcome,
            scheduled::Outcome::Error
        );
        assert!(result.operations.samples[0].started_ns.is_some());
        let failed = result
            .wire_samples
            .iter()
            .filter(|s| s.connection_id == 0)
            .collect::<Vec<_>>();
        assert!(failed.len() <= 1);
        // If the actor closes before channel admission there is no FIFO owner.
        // The driver error remains counted; never invent a wire timestamp.
        for sample in failed {
            assert!(sample.transport_failure.is_some());
            assert!(sample.response_complete_ns.is_none());
        }
        let control = Arc::try_unwrap(control).unwrap_or_else(|_| panic!("control still owned"));
        assert!(control.shutdown().await.is_err());
        assert!(server_tasks.iter().all(|task| task.is_finished()));
        assert!(client_tasks.iter().all(|task| task.is_finished()));
    }

    #[tokio::test(start_paused = true)]
    async fn independent_connections_do_not_share_cancelled_owners_or_drain_budget() {
        let (client_a, mut peer_a) = tokio::io::duplex(64);
        let (client_b, mut peer_b) = tokio::io::duplex(64);
        let pipelines = Arc::new(vec![
            Pipeline::new(client_a, 1).unwrap(),
            Pipeline::new(client_b, 1).unwrap(),
        ]);
        let request = command(&[b"GET", b"x"]);
        let p = Arc::clone(&pipelines);
        let bytes = request.clone();
        let waiter_a = tokio::spawn(async move {
            p[0].submit(0, bytes, Expected::Bulk(Some(Bytes::from_static(b"x"))))
                .await
        });
        let mut input = vec![0; request.len()];
        peer_a.read_exact(&mut input).await.unwrap();
        waiter_a.abort();
        assert!(waiter_a.await.unwrap_err().is_cancelled());
        assert_eq!(pipelines[0].slots.available_permits(), 0);
        assert_eq!(pipelines[1].slots.available_permits(), 1);
        let p = Arc::clone(&pipelines);
        let waiter_b = tokio::spawn(async move {
            p[1].submit(1, request, Expected::Bulk(Some(Bytes::from_static(b"x"))))
                .await
        });
        peer_b.read_exact(&mut input).await.unwrap();
        peer_b.write_all(b"$1\r\nx\r\n").await.unwrap();
        assert_eq!(waiter_b.await.unwrap(), TargetOutcome::Success);
        assert_eq!(pipelines[1].slots.available_permits(), 1);
        let start = Instant::now();
        assert!(drain_pipelines(&pipelines).await.is_err());
        assert_eq!(start.elapsed(), Duration::from_secs(5));
        assert!(pipelines[0].records.lock().unwrap().is_empty());
        assert_eq!(pipelines[1].records.lock().unwrap()[0].ordinal, 0);
        peer_a.write_all(b"$1\r\nx\r\n").await.unwrap();
        drain_pipelines(&pipelines).await.unwrap();
        let record = pipelines[0].records.lock().unwrap()[0].clone();
        assert_eq!(record.ordinal, 0); // Identical ordinal on different sockets is valid.
        assert!(record.cancelled && record.verified);
        for pipeline in pipelines.iter() {
            pipeline.shutdown().await.unwrap();
            assert_eq!(pipeline.slots.available_permits(), 1);
        }
    }

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
            b"%0\r\n",
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
