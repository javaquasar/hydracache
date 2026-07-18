//! Strict RESP2-over-TCP target for release-0.67 performance characterization.
//!
//! The adapter is binary-safe, incremental, and exact about pipeline reply
//! counts. It connects to one explicitly selected endpoint and never sums
//! independent node-local stores into a cluster-capacity claim.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};

use crate::target::{PreloadOutcome, Target, TargetError, TargetOutcome, TargetRequest};

const STATE_DIGEST_VERSION: &str = "hydracache-resp-node-local-logical-state-v2";
const KEY_PREFIX: &[u8] = b"hc:perf:w3:";

/// Bounds applied by the incremental RESP2 parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resp2Limits {
    pub max_frame_bytes: usize,
    pub max_bulk_bytes: usize,
    pub max_array_entries: usize,
    pub max_nesting_depth: usize,
}

impl Default for Resp2Limits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 16 * 1024 * 1024,
            max_bulk_bytes: 16 * 1024 * 1024,
            max_array_entries: 1024,
            max_nesting_depth: 16,
        }
    }
}

/// Binary-safe RESP2 value returned by the strict parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resp2Value {
    Simple(Vec<u8>),
    Error(Vec<u8>),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
    Array(Option<Vec<Resp2Value>>),
}

/// Result of parsing one value from an incrementally filled buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resp2ParseStatus {
    Incomplete,
    Complete { value: Resp2Value, consumed: usize },
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum Resp2ParseError {
    #[error("RESP2 frame exceeds {max} bytes")]
    FrameTooLarge { max: usize },
    #[error("RESP2 bulk payload {actual} exceeds {max} bytes")]
    BulkTooLarge { actual: usize, max: usize },
    #[error("RESP2 array has {actual} entries, limit is {max}")]
    ArrayTooLarge { actual: usize, max: usize },
    #[error("RESP2 nesting exceeds {max} levels")]
    NestingTooDeep { max: usize },
    #[error("RESP2 value has an unknown type prefix 0x{0:02x}")]
    UnknownType(u8),
    #[error("RESP2 length/integer is malformed")]
    InvalidInteger,
    #[error("RESP2 length cannot be less than -1")]
    InvalidNegativeLength,
    #[error("RESP2 value is missing its CRLF terminator")]
    InvalidTerminator,
    #[error("RESP2 size arithmetic overflowed")]
    SizeOverflow,
}

/// Encode one command as a binary-safe RESP2 array of bulk strings.
pub fn encode_resp2_command<I, B>(arguments: I) -> Vec<u8>
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    let mut encoded = Vec::new();
    encoded.extend_from_slice(format!("*{}\r\n", arguments.len()).as_bytes());
    for argument in arguments {
        let bytes = argument.as_ref();
        encoded.extend_from_slice(format!("${}\r\n", bytes.len()).as_bytes());
        encoded.extend_from_slice(bytes);
        encoded.extend_from_slice(b"\r\n");
    }
    encoded
}

/// Parse one incremental RESP2 value without accepting trailing bytes as part
/// of that value.
pub fn parse_resp2(input: &[u8], limits: Resp2Limits) -> Result<Resp2ParseStatus, Resp2ParseError> {
    match parse_value(input, limits, 0)? {
        Some((value, consumed)) => Ok(Resp2ParseStatus::Complete { value, consumed }),
        None => Ok(Resp2ParseStatus::Incomplete),
    }
}

/// Parse exactly `expected` complete replies and reject truncation, surplus
/// replies, or trailing garbage.
pub fn parse_exact_resp2_replies(
    input: &[u8],
    expected: usize,
    limits: Resp2Limits,
) -> Result<Vec<Resp2Value>, RespExactReplyError> {
    let mut offset = 0;
    let mut replies = Vec::with_capacity(expected);
    while replies.len() < expected {
        match parse_resp2(&input[offset..], limits)? {
            Resp2ParseStatus::Incomplete => {
                return Err(RespExactReplyError::Truncated {
                    expected,
                    received: replies.len(),
                })
            }
            Resp2ParseStatus::Complete { value, consumed } => {
                offset = offset
                    .checked_add(consumed)
                    .ok_or(Resp2ParseError::SizeOverflow)?;
                replies.push(value);
            }
        }
    }
    if offset != input.len() {
        return Err(RespExactReplyError::SurplusBytes {
            expected,
            trailing: input.len() - offset,
        });
    }
    Ok(replies)
}

#[derive(Debug, thiserror::Error)]
pub enum RespExactReplyError {
    #[error(transparent)]
    Parse(#[from] Resp2ParseError),
    #[error("RESP2 stream truncated: expected {expected} replies, received {received}")]
    Truncated { expected: usize, received: usize },
    #[error("RESP2 stream returned bytes after exactly {expected} replies: {trailing} bytes")]
    SurplusBytes { expected: usize, trailing: usize },
}

fn parse_value(
    input: &[u8],
    limits: Resp2Limits,
    depth: usize,
) -> Result<Option<(Resp2Value, usize)>, Resp2ParseError> {
    if depth > limits.max_nesting_depth {
        return Err(Resp2ParseError::NestingTooDeep {
            max: limits.max_nesting_depth,
        });
    }
    let Some(prefix) = input.first().copied() else {
        return Ok(None);
    };
    match prefix {
        b'+' | b'-' | b':' => {
            let Some(line_end) = find_crlf(&input[1..]) else {
                ensure_frame_size(input.len(), limits)?;
                return Ok(None);
            };
            let payload = &input[1..1 + line_end];
            let consumed = 1 + line_end + 2;
            ensure_frame_size(consumed, limits)?;
            let value = match prefix {
                b'+' => Resp2Value::Simple(payload.to_vec()),
                b'-' => Resp2Value::Error(payload.to_vec()),
                b':' => Resp2Value::Integer(parse_i64(payload)?),
                _ => unreachable!(),
            };
            Ok(Some((value, consumed)))
        }
        b'$' => parse_bulk(input, limits),
        b'*' => parse_array(input, limits, depth),
        other => Err(Resp2ParseError::UnknownType(other)),
    }
}

fn parse_bulk(
    input: &[u8],
    limits: Resp2Limits,
) -> Result<Option<(Resp2Value, usize)>, Resp2ParseError> {
    let Some(line_end) = find_crlf(&input[1..]) else {
        ensure_frame_size(input.len(), limits)?;
        return Ok(None);
    };
    let length = parse_i64(&input[1..1 + line_end])?;
    let header = 1 + line_end + 2;
    if length == -1 {
        return Ok(Some((Resp2Value::Bulk(None), header)));
    }
    if length < -1 {
        return Err(Resp2ParseError::InvalidNegativeLength);
    }
    let length = usize::try_from(length).map_err(|_| Resp2ParseError::SizeOverflow)?;
    if length > limits.max_bulk_bytes {
        return Err(Resp2ParseError::BulkTooLarge {
            actual: length,
            max: limits.max_bulk_bytes,
        });
    }
    let payload_end = header
        .checked_add(length)
        .ok_or(Resp2ParseError::SizeOverflow)?;
    let consumed = payload_end
        .checked_add(2)
        .ok_or(Resp2ParseError::SizeOverflow)?;
    ensure_frame_size(consumed, limits)?;
    if input.len() < consumed {
        return Ok(None);
    }
    if &input[payload_end..consumed] != b"\r\n" {
        return Err(Resp2ParseError::InvalidTerminator);
    }
    Ok(Some((
        Resp2Value::Bulk(Some(input[header..payload_end].to_vec())),
        consumed,
    )))
}

fn parse_array(
    input: &[u8],
    limits: Resp2Limits,
    depth: usize,
) -> Result<Option<(Resp2Value, usize)>, Resp2ParseError> {
    let Some(line_end) = find_crlf(&input[1..]) else {
        ensure_frame_size(input.len(), limits)?;
        return Ok(None);
    };
    let count = parse_i64(&input[1..1 + line_end])?;
    let mut consumed = 1 + line_end + 2;
    if count == -1 {
        return Ok(Some((Resp2Value::Array(None), consumed)));
    }
    if count < -1 {
        return Err(Resp2ParseError::InvalidNegativeLength);
    }
    let count = usize::try_from(count).map_err(|_| Resp2ParseError::SizeOverflow)?;
    if count > limits.max_array_entries {
        return Err(Resp2ParseError::ArrayTooLarge {
            actual: count,
            max: limits.max_array_entries,
        });
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let Some((value, value_bytes)) = parse_value(&input[consumed..], limits, depth + 1)? else {
            return Ok(None);
        };
        consumed = consumed
            .checked_add(value_bytes)
            .ok_or(Resp2ParseError::SizeOverflow)?;
        ensure_frame_size(consumed, limits)?;
        values.push(value);
    }
    Ok(Some((Resp2Value::Array(Some(values)), consumed)))
}

fn find_crlf(input: &[u8]) -> Option<usize> {
    input.windows(2).position(|pair| pair == b"\r\n")
}

fn parse_i64(input: &[u8]) -> Result<i64, Resp2ParseError> {
    std::str::from_utf8(input)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or(Resp2ParseError::InvalidInteger)
}

fn ensure_frame_size(size: usize, limits: Resp2Limits) -> Result<(), Resp2ParseError> {
    if size > limits.max_frame_bytes {
        Err(Resp2ParseError::FrameTooLarge {
            max: limits.max_frame_bytes,
        })
    } else {
        Ok(())
    }
}

/// Honest identity of the one RESP endpoint measured by this target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespEndpointIdentity {
    pub address: std::net::SocketAddr,
    pub selected_endpoint: String,
    pub endpoint_kind: String,
    pub state_scope: String,
}

/// YCSB-shaped operation mix supported by HydraCache's RESP facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespOperationMix {
    pub get_percent: u8,
    pub set_percent: u8,
    pub mget_percent: u8,
    pub mset_percent: u8,
}

impl RespOperationMix {
    pub const WORKLOAD_A: Self = Self {
        get_percent: 45,
        set_percent: 45,
        mget_percent: 5,
        mset_percent: 5,
    };
    pub const WORKLOAD_B: Self = Self {
        get_percent: 90,
        set_percent: 4,
        mget_percent: 5,
        mset_percent: 1,
    };
    pub const WORKLOAD_C: Self = Self {
        get_percent: 90,
        set_percent: 0,
        mget_percent: 10,
        mset_percent: 0,
    };

    pub const fn total_percent(self) -> u16 {
        self.get_percent as u16
            + self.set_percent as u16
            + self.mget_percent as u16
            + self.mset_percent as u16
    }

    pub fn operation_for(self, sequence: u64) -> RespOperation {
        let percentile = (sequence % 100) as u16;
        let get_end = self.get_percent as u16;
        let set_end = get_end + self.set_percent as u16;
        let mget_end = set_end + self.mget_percent as u16;
        if percentile < get_end {
            RespOperation::Get
        } else if percentile < set_end {
            RespOperation::Set
        } else if percentile < mget_end {
            RespOperation::MGet
        } else {
            RespOperation::MSet
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespOperation {
    Get,
    Set,
    MGet,
    MSet,
}

#[derive(Debug, Clone)]
pub struct RespTargetConfig {
    pub endpoint: RespEndpointIdentity,
    pub require_loopback: bool,
    pub connections: usize,
    pub pipeline_depth: usize,
    pub preload_entries: u64,
    pub key_space: u64,
    pub payload_bytes: usize,
    pub batch_size: usize,
    /// Maximum keys carried by one reset `DEL`. This must not exceed the
    /// selected surface's documented batch/array admission limit.
    pub reset_batch_entries: usize,
    pub operation_mix: RespOperationMix,
    pub key_schedule: Arc<Vec<u64>>,
    pub connect_timeout: Duration,
    pub io_timeout: Duration,
    pub parser_limits: Resp2Limits,
    /// Loadgen-owned canary seam applied immediately before the real TCP
    /// exchange. It is never represented as a product listener failpoint.
    pub injected_dispatch_delay: Duration,
}

impl RespTargetConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.endpoint.selected_endpoint.trim().is_empty()
            || self.endpoint.endpoint_kind.trim().is_empty()
            || self.endpoint.state_scope.trim().is_empty()
        {
            return Err("selected RESP endpoint identity is incomplete".into());
        }
        if self.require_loopback && !self.endpoint.address.ip().is_loopback() {
            return Err("RESP smoke target requires a loopback TCP endpoint".into());
        }
        if self.connections == 0
            || self.pipeline_depth == 0
            || self.key_space == 0
            || self.payload_bytes == 0
            || self.batch_size == 0
            || self.reset_batch_entries == 0
            || self.reset_batch_entries > 128
            || self.preload_entries > self.key_space
            || self.operation_mix.total_percent() != 100
            || self.key_schedule.is_empty()
            || self.key_schedule.iter().any(|key| *key >= self.key_space)
            || self.connect_timeout.is_zero()
            || self.io_timeout.is_zero()
            || self.parser_limits.max_frame_bytes == 0
            || self.parser_limits.max_bulk_bytes == 0
            || self.parser_limits.max_array_entries == 0
            || self.parser_limits.max_nesting_depth == 0
        {
            return Err("RESP target execution contract is incomplete".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespTcpEvidence {
    pub selected_endpoint: String,
    pub endpoint_kind: String,
    pub state_scope: String,
    pub peer_addresses: Vec<std::net::SocketAddr>,
    pub local_addresses: Vec<std::net::SocketAddr>,
    pub connection_count: usize,
    pub real_tcp: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RespTargetSnapshot {
    pub pipeline_exchanges: u64,
    pub commands_sent: u64,
    pub replies_received: u64,
    pub successful_exchanges: u64,
    pub rejected_exchanges: u64,
    pub failed_exchanges: u64,
}

#[derive(Debug)]
pub struct RespDispatch {
    pub outcome: TargetOutcome,
    pub replies: Vec<Resp2Value>,
    pub logical_commands: usize,
    pub bytes_sent: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum RespTargetError {
    #[error("RESP target configuration failed: {0}")]
    Config(String),
    #[error("RESP TCP connect to {endpoint} failed: {source}")]
    Connect {
        endpoint: std::net::SocketAddr,
        source: std::io::Error,
    },
    #[error("RESP TCP IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("RESP TCP operation timed out during {0}")]
    Timeout(&'static str),
    #[error(transparent)]
    Parse(#[from] Resp2ParseError),
    #[error("RESP server truncated a pipeline: expected {expected}, received {received}")]
    Truncated { expected: usize, received: usize },
    #[error("RESP server returned unsolicited bytes after {expected} replies: {trailing}")]
    Surplus { expected: usize, trailing: usize },
    #[error("RESP server returned an unexpected reply: {0}")]
    UnexpectedReply(String),
    #[error("RESP target has not been reset/connected")]
    NotConnected,
}

#[derive(Debug)]
struct RespConnection {
    stream: TcpStream,
    read_buffer: Vec<u8>,
    limits: Resp2Limits,
    io_timeout: Duration,
    usable: bool,
}

impl RespConnection {
    async fn connect(config: &RespTargetConfig) -> Result<Self, RespTargetError> {
        let stream = tokio::time::timeout(
            config.connect_timeout,
            TcpStream::connect(config.endpoint.address),
        )
        .await
        .map_err(|_| RespTargetError::Timeout("connect"))?
        .map_err(|source| RespTargetError::Connect {
            endpoint: config.endpoint.address,
            source,
        })?;
        stream.set_nodelay(true)?;
        let mut connection = Self {
            stream,
            read_buffer: Vec::new(),
            limits: config.parser_limits,
            io_timeout: config.io_timeout,
            usable: true,
        };
        let replies = connection
            .exchange(&encode_resp2_command([b"PING".as_slice()]), 1)
            .await?;
        if replies != [Resp2Value::Simple(b"PONG".to_vec())] {
            return Err(RespTargetError::UnexpectedReply(format!(
                "PING handshake returned {replies:?}"
            )));
        }
        Ok(connection)
    }

    fn invalidate(&mut self) {
        self.usable = false;
        self.read_buffer.clear();
    }

    async fn exchange(
        &mut self,
        request: &[u8],
        expected_replies: usize,
    ) -> Result<Vec<Resp2Value>, RespTargetError> {
        tokio::time::timeout(self.io_timeout, self.stream.write_all(request))
            .await
            .map_err(|_| RespTargetError::Timeout("write"))??;
        tokio::time::timeout(self.io_timeout, self.stream.flush())
            .await
            .map_err(|_| RespTargetError::Timeout("flush"))??;
        let mut replies = Vec::with_capacity(expected_replies);
        loop {
            while replies.len() < expected_replies {
                match parse_resp2(&self.read_buffer, self.limits)? {
                    Resp2ParseStatus::Incomplete => break,
                    Resp2ParseStatus::Complete { value, consumed } => {
                        self.read_buffer.drain(..consumed);
                        replies.push(value);
                    }
                }
            }
            if replies.len() == expected_replies {
                if !self.read_buffer.is_empty() {
                    return Err(RespTargetError::Surplus {
                        expected: expected_replies,
                        trailing: self.read_buffer.len(),
                    });
                }
                return Ok(replies);
            }
            let mut chunk = [0_u8; 8192];
            let read = tokio::time::timeout(self.io_timeout, self.stream.read(&mut chunk))
                .await
                .map_err(|_| RespTargetError::Timeout("read"))??;
            if read == 0 {
                return Err(RespTargetError::Truncated {
                    expected: expected_replies,
                    received: replies.len(),
                });
            }
            self.read_buffer.extend_from_slice(&chunk[..read]);
            let maximum_pipeline_bytes = self
                .limits
                .max_frame_bytes
                .saturating_mul(expected_replies.max(1));
            if self.read_buffer.len() > maximum_pipeline_bytes {
                return Err(Resp2ParseError::FrameTooLarge {
                    max: maximum_pipeline_bytes,
                }
                .into());
            }
        }
    }
}

#[derive(Debug, Default)]
struct RespCounters {
    pipeline_exchanges: AtomicU64,
    commands_sent: AtomicU64,
    replies_received: AtomicU64,
    successful_exchanges: AtomicU64,
    rejected_exchanges: AtomicU64,
    failed_exchanges: AtomicU64,
}

impl RespCounters {
    fn reset(&self) {
        self.pipeline_exchanges.store(0, Ordering::Relaxed);
        self.commands_sent.store(0, Ordering::Relaxed);
        self.replies_received.store(0, Ordering::Relaxed);
        self.successful_exchanges.store(0, Ordering::Relaxed);
        self.rejected_exchanges.store(0, Ordering::Relaxed);
        self.failed_exchanges.store(0, Ordering::Relaxed);
    }

    fn snapshot(&self) -> RespTargetSnapshot {
        RespTargetSnapshot {
            pipeline_exchanges: self.pipeline_exchanges.load(Ordering::Relaxed),
            commands_sent: self.commands_sent.load(Ordering::Relaxed),
            replies_received: self.replies_received.load(Ordering::Relaxed),
            successful_exchanges: self.successful_exchanges.load(Ordering::Relaxed),
            rejected_exchanges: self.rejected_exchanges.load(Ordering::Relaxed),
            failed_exchanges: self.failed_exchanges.load(Ordering::Relaxed),
        }
    }
}

/// Real Tokio TCP target for one selected RESP2 endpoint.
#[derive(Debug)]
pub struct RespTcpTarget {
    config: RespTargetConfig,
    connections: RwLock<Vec<Arc<Mutex<RespConnection>>>>,
    counters: RespCounters,
}

impl RespTcpTarget {
    pub fn new(config: RespTargetConfig) -> Result<Self, RespTargetError> {
        config.validate().map_err(RespTargetError::Config)?;
        Ok(Self {
            config,
            connections: RwLock::new(Vec::new()),
            counters: RespCounters::default(),
        })
    }

    pub fn config(&self) -> &RespTargetConfig {
        &self.config
    }

    pub fn snapshot(&self) -> RespTargetSnapshot {
        self.counters.snapshot()
    }

    pub async fn tcp_evidence(&self) -> Result<RespTcpEvidence, RespTargetError> {
        let connections = self.connections.read().await;
        if connections.is_empty() {
            return Err(RespTargetError::NotConnected);
        }
        let mut peers = Vec::with_capacity(connections.len());
        let mut locals = Vec::with_capacity(connections.len());
        for connection in connections.iter() {
            let connection = connection.lock().await;
            peers.push(connection.stream.peer_addr()?);
            locals.push(connection.stream.local_addr()?);
        }
        Ok(RespTcpEvidence {
            selected_endpoint: self.config.endpoint.selected_endpoint.clone(),
            endpoint_kind: self.config.endpoint.endpoint_kind.clone(),
            state_scope: self.config.endpoint.state_scope.clone(),
            peer_addresses: peers,
            local_addresses: locals,
            connection_count: connections.len(),
            real_tcp: true,
        })
    }

    pub async fn dispatch_operation(
        &self,
        operation: RespOperation,
        sequence: u64,
    ) -> RespDispatch {
        self.dispatch_pipeline(operation, sequence, self.config.pipeline_depth)
            .await
    }

    pub async fn dispatch_pipeline(
        &self,
        operation: RespOperation,
        first_sequence: u64,
        depth: usize,
    ) -> RespDispatch {
        if depth == 0 {
            return RespDispatch {
                outcome: TargetOutcome::Error,
                replies: Vec::new(),
                logical_commands: 0,
                bytes_sent: 0,
            };
        }
        let operations = (0..depth)
            .map(|offset| (operation, first_sequence.saturating_add(offset as u64)))
            .collect::<Vec<_>>();
        self.dispatch_commands(first_sequence, &operations).await
    }

    async fn dispatch_scheduled_pipeline(&self, request_sequence: u64) -> RespDispatch {
        let depth = self.config.pipeline_depth;
        let first_sequence = request_sequence.saturating_mul(depth as u64);
        let operations = (0..depth)
            .map(|offset| {
                let sequence = first_sequence.saturating_add(offset as u64);
                (self.config.operation_mix.operation_for(sequence), sequence)
            })
            .collect::<Vec<_>>();
        self.dispatch_commands(request_sequence, &operations).await
    }

    async fn dispatch_commands(
        &self,
        connection_sequence: u64,
        operations: &[(RespOperation, u64)],
    ) -> RespDispatch {
        let depth = operations.len();
        let mut request = Vec::new();
        let mut expected = Vec::with_capacity(depth);
        for (operation, sequence) in operations {
            let (command, reply_shape) = self.command(*operation, *sequence);
            request.extend_from_slice(&command);
            expected.push(reply_shape);
        }
        self.counters
            .pipeline_exchanges
            .fetch_add(1, Ordering::Relaxed);
        self.counters
            .commands_sent
            .fetch_add(depth as u64, Ordering::Relaxed);
        let connection = match self.connection(connection_sequence).await {
            Ok(connection) => connection,
            Err(_) => {
                self.counters
                    .failed_exchanges
                    .fetch_add(1, Ordering::Relaxed);
                return RespDispatch {
                    outcome: TargetOutcome::Error,
                    replies: Vec::new(),
                    logical_commands: depth,
                    bytes_sent: request.len(),
                };
            }
        };
        if !self.config.injected_dispatch_delay.is_zero() {
            tokio::time::sleep(self.config.injected_dispatch_delay).await;
        }
        let mut connection = connection.lock().await;
        if !connection.usable {
            match RespConnection::connect(&self.config).await {
                Ok(replacement) => *connection = replacement,
                Err(_) => {
                    self.counters
                        .failed_exchanges
                        .fetch_add(1, Ordering::Relaxed);
                    return RespDispatch {
                        outcome: TargetOutcome::Error,
                        replies: Vec::new(),
                        logical_commands: depth,
                        bytes_sent: request.len(),
                    };
                }
            }
        }
        // Pessimistically poison before the cancellable IO await. A driver
        // drain timeout may abort this future at any suspension point; the
        // next request must reconnect rather than reuse partial wire state.
        connection.usable = false;
        let result = connection.exchange(&request, depth).await;
        if result.is_ok() {
            connection.usable = true;
        } else {
            // A timeout, EOF, IO failure, or parse failure can leave a partial
            // request or a late reply on this stream. Never let the next
            // scheduled operation consume those bytes as its own response.
            connection.invalidate();
        }
        drop(connection);
        match result {
            Ok(replies) => {
                self.counters
                    .replies_received
                    .fetch_add(replies.len() as u64, Ordering::Relaxed);
                let outcome = classify_replies(&replies, &expected);
                match outcome {
                    TargetOutcome::Success => {
                        self.counters
                            .successful_exchanges
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    TargetOutcome::Rejected => {
                        self.counters
                            .rejected_exchanges
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    TargetOutcome::Error | TargetOutcome::Timeout => {
                        self.counters
                            .failed_exchanges
                            .fetch_add(1, Ordering::Relaxed);
                    }
                }
                RespDispatch {
                    outcome,
                    replies,
                    logical_commands: depth,
                    bytes_sent: request.len(),
                }
            }
            Err(RespTargetError::Timeout(_)) => {
                self.counters
                    .failed_exchanges
                    .fetch_add(1, Ordering::Relaxed);
                RespDispatch {
                    outcome: TargetOutcome::Timeout,
                    replies: Vec::new(),
                    logical_commands: depth,
                    bytes_sent: request.len(),
                }
            }
            Err(_) => {
                self.counters
                    .failed_exchanges
                    .fetch_add(1, Ordering::Relaxed);
                RespDispatch {
                    outcome: TargetOutcome::Error,
                    replies: Vec::new(),
                    logical_commands: depth,
                    bytes_sent: request.len(),
                }
            }
        }
    }

    async fn connect_all(&self) -> Result<(), RespTargetError> {
        let mut replacement = Vec::with_capacity(self.config.connections);
        for _ in 0..self.config.connections {
            replacement.push(Arc::new(Mutex::new(
                RespConnection::connect(&self.config).await?,
            )));
        }
        *self.connections.write().await = replacement;
        Ok(())
    }

    async fn connection(
        &self,
        sequence: u64,
    ) -> Result<Arc<Mutex<RespConnection>>, RespTargetError> {
        let connections = self.connections.read().await;
        if connections.is_empty() {
            return Err(RespTargetError::NotConnected);
        }
        Ok(Arc::clone(
            &connections[sequence as usize % connections.len()],
        ))
    }

    fn command(&self, operation: RespOperation, sequence: u64) -> (Vec<u8>, ReplyShape) {
        let logical_key =
            self.config.key_schedule[sequence as usize % self.config.key_schedule.len()];
        match operation {
            RespOperation::Get => (
                encode_resp2_command([b"GET".as_slice(), key(logical_key).as_slice()]),
                ReplyShape::Bulk,
            ),
            RespOperation::Set => (
                encode_resp2_command([
                    b"SET".as_slice(),
                    key(logical_key).as_slice(),
                    payload(sequence, self.config.payload_bytes).as_slice(),
                ]),
                ReplyShape::Ok,
            ),
            RespOperation::MGet => {
                let mut arguments = vec![b"MGET".to_vec()];
                for offset in 0..self.config.batch_size {
                    arguments.push(key(
                        logical_key.saturating_add(offset as u64) % self.config.key_space
                    ));
                }
                (
                    encode_resp2_command(arguments),
                    ReplyShape::BulkArray(self.config.batch_size),
                )
            }
            RespOperation::MSet => {
                let mut arguments = vec![b"MSET".to_vec()];
                for offset in 0..self.config.batch_size {
                    arguments.push(key(
                        logical_key.saturating_add(offset as u64) % self.config.key_space
                    ));
                    arguments.push(payload(
                        sequence.saturating_add(offset as u64),
                        self.config.payload_bytes,
                    ));
                }
                (encode_resp2_command(arguments), ReplyShape::Ok)
            }
        }
    }

    async fn clear_known_keys(&self) -> Result<(), RespTargetError> {
        let connection = self.connection(0).await?;
        let mut connection = connection.lock().await;
        for start in (0..self.config.key_space).step_by(self.config.reset_batch_entries) {
            let end = start
                .saturating_add(self.config.reset_batch_entries as u64)
                .min(self.config.key_space);
            let mut arguments = Vec::with_capacity((end - start) as usize + 1);
            arguments.push(b"DEL".to_vec());
            for logical_key in start..end {
                arguments.push(key(logical_key));
            }
            let replies = connection
                .exchange(&encode_resp2_command(arguments), 1)
                .await?;
            if !matches!(replies.as_slice(), [Resp2Value::Integer(value)] if *value >= 0) {
                return Err(RespTargetError::UnexpectedReply(format!(
                    "DEL reset chunk {start}..{end} returned {replies:?}"
                )));
            }
        }
        Ok(())
    }

    async fn observed_values(&self) -> Result<Vec<Option<Vec<u8>>>, RespTargetError> {
        let mut request = Vec::new();
        for logical_key in 0..self.config.key_space {
            request.extend_from_slice(&encode_resp2_command([
                b"GET".as_slice(),
                key(logical_key).as_slice(),
            ]));
        }
        let connection = self.connection(0).await?;
        let replies = connection
            .lock()
            .await
            .exchange(&request, self.config.key_space as usize)
            .await?;
        replies
            .into_iter()
            .map(|reply| match reply {
                Resp2Value::Bulk(value) => Ok(value),
                other => Err(RespTargetError::UnexpectedReply(format!(
                    "GET state observation returned {other:?}"
                ))),
            })
            .collect()
    }

    async fn observed_state_digest(&self) -> Result<String, RespTargetError> {
        let values = self.observed_values().await?;
        let mut hasher = Sha256::new();
        hasher.update(STATE_DIGEST_VERSION.as_bytes());
        hasher.update(self.config.endpoint.state_scope.as_bytes());
        hasher.update(b"hydracache-redis-default-namespace");
        hasher.update(self.config.key_space.to_le_bytes());
        for (logical_key, value) in values.into_iter().enumerate() {
            hasher.update((logical_key as u64).to_le_bytes());
            match value {
                Some(value) => {
                    hasher.update([1]);
                    hasher.update((value.len() as u64).to_le_bytes());
                    hasher.update(value);
                }
                None => hasher.update([0]),
            }
        }
        Ok(hex_digest(hasher.finalize().as_ref()))
    }
}

#[async_trait]
impl Target for RespTcpTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        self.connect_all()
            .await
            .map_err(|error| TargetError::Reset(error.to_string()))?;
        self.clear_known_keys()
            .await
            .map_err(|error| TargetError::Reset(error.to_string()))?;
        self.counters.reset();
        self.observed_state_digest()
            .await
            .map_err(|error| TargetError::Reset(error.to_string()))
    }

    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        for start in (0..self.config.preload_entries).step_by(self.config.reset_batch_entries) {
            let end =
                (start + self.config.reset_batch_entries as u64).min(self.config.preload_entries);
            let mut request = Vec::new();
            for logical_key in start..end {
                request.extend_from_slice(&encode_resp2_command([
                    b"SET".as_slice(),
                    key(logical_key).as_slice(),
                    payload(logical_key, self.config.payload_bytes).as_slice(),
                ]));
            }
            let connection = self
                .connection(0)
                .await
                .map_err(|error| TargetError::Preload(error.to_string()))?;
            let replies = connection
                .lock()
                .await
                .exchange(&request, (end - start) as usize)
                .await
                .map_err(|error| TargetError::Preload(error.to_string()))?;
            if replies
                .iter()
                .any(|reply| !matches!(reply, Resp2Value::Simple(value) if value == b"OK"))
            {
                return Err(TargetError::Preload(format!(
                    "SET preload returned {replies:?}"
                )));
            }
        }
        let observed = self
            .observed_values()
            .await
            .map_err(|error| TargetError::Preload(error.to_string()))?;
        for logical_key in 0..self.config.preload_entries {
            if observed[logical_key as usize]
                != Some(payload(logical_key, self.config.payload_bytes))
            {
                return Err(TargetError::Preload(format!(
                    "GET verification failed for RESP preload key {logical_key}"
                )));
            }
        }
        Ok(PreloadOutcome {
            operations: self.config.preload_entries,
            state_digest: self
                .observed_state_digest()
                .await
                .map_err(|error| TargetError::Preload(error.to_string()))?,
        })
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        self.observed_state_digest()
            .await
            .map_err(|error| TargetError::Warmup(error.to_string()))
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        self.dispatch_scheduled_pipeline(request.sequence)
            .await
            .outcome
    }
}

#[derive(Debug, Clone, Copy)]
enum ReplyShape {
    Ok,
    Bulk,
    BulkArray(usize),
}

fn classify_replies(replies: &[Resp2Value], expected: &[ReplyShape]) -> TargetOutcome {
    if replies.len() != expected.len() {
        return TargetOutcome::Error;
    }
    if replies
        .iter()
        .any(|reply| matches!(reply, Resp2Value::Error(_)))
    {
        return TargetOutcome::Rejected;
    }
    if replies
        .iter()
        .zip(expected)
        .all(|(reply, shape)| match shape {
            ReplyShape::Ok => matches!(reply, Resp2Value::Simple(value) if value == b"OK"),
            ReplyShape::Bulk => matches!(reply, Resp2Value::Bulk(_)),
            ReplyShape::BulkArray(expected_entries) => matches!(
                reply,
                Resp2Value::Array(Some(values))
                    if values.len() == *expected_entries
                        && values.iter().all(|value| matches!(value, Resp2Value::Bulk(_)))
            ),
        })
    {
        TargetOutcome::Success
    } else {
        TargetOutcome::Error
    }
}

fn key(logical_key: u64) -> Vec<u8> {
    let mut key = KEY_PREFIX.to_vec();
    key.extend_from_slice(logical_key.to_string().as_bytes());
    key
}

fn payload(sequence: u64, bytes: usize) -> Vec<u8> {
    let mut value = vec![(sequence % 251) as u8; bytes];
    if bytes >= 4 {
        // Include RESP-looking and zero bytes so target tests prove bulk payload
        // handling is length-based rather than delimiter/string based.
        value[..4].copy_from_slice(&[0, b'\r', b'\n', b'$']);
    }
    value
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_safe_command_round_trips_incrementally() {
        let binary = [0, b'\r', b'\n', b'$', 0xff];
        let encoded = encode_resp2_command([b"SET".as_slice(), b"key".as_slice(), &binary]);
        for prefix in 0..encoded.len() {
            assert_eq!(
                parse_resp2(&encoded[..prefix], Resp2Limits::default()).unwrap(),
                Resp2ParseStatus::Incomplete
            );
        }
        let Resp2ParseStatus::Complete { value, consumed } =
            parse_resp2(&encoded, Resp2Limits::default()).unwrap()
        else {
            panic!("complete command remained incomplete");
        };
        assert_eq!(consumed, encoded.len());
        assert_eq!(
            value,
            Resp2Value::Array(Some(vec![
                Resp2Value::Bulk(Some(b"SET".to_vec())),
                Resp2Value::Bulk(Some(b"key".to_vec())),
                Resp2Value::Bulk(Some(binary.to_vec())),
            ]))
        );
    }

    #[test]
    fn exact_reply_parser_rejects_truncation_and_surplus() {
        let limits = Resp2Limits::default();
        assert!(matches!(
            parse_exact_resp2_replies(b"+OK\r\n$3\r\nab", 2, limits),
            Err(RespExactReplyError::Truncated {
                expected: 2,
                received: 1
            })
        ));
        assert!(matches!(
            parse_exact_resp2_replies(b"+OK\r\n+EXTRA\r\n", 1, limits),
            Err(RespExactReplyError::SurplusBytes {
                expected: 1,
                trailing: 8
            })
        ));
    }

    #[test]
    fn pipeline_total_may_exceed_the_per_reply_frame_limit() {
        let limits = Resp2Limits {
            max_frame_bytes: 6,
            ..Resp2Limits::default()
        };
        let replies = parse_exact_resp2_replies(b"+OK\r\n+OK\r\n", 2, limits).unwrap();
        assert_eq!(replies.len(), 2);
    }

    #[tokio::test]
    async fn timed_out_connection_is_replaced_before_late_reply_can_be_reused() {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut input = [0_u8; 1024];
                let read = stream.read(&mut input).await.unwrap();
                assert!(input[..read].windows(4).any(|bytes| bytes == b"PING"));
                stream.write_all(b"+PONG\r\n").await.unwrap();
                let read = stream.read(&mut input).await.unwrap();
                assert!(input[..read].windows(3).any(|bytes| bytes == b"GET"));
                if attempt == 0 {
                    tokio::time::sleep(Duration::from_millis(40)).await;
                }
                let _ = stream.write_all(b"$-1\r\n").await;
            }
        });
        let target = RespTcpTarget::new(RespTargetConfig {
            endpoint: RespEndpointIdentity {
                address,
                selected_endpoint: format!("timeout-test@{address}"),
                endpoint_kind: "loopback-test".to_owned(),
                state_scope: "test-local".to_owned(),
            },
            require_loopback: true,
            connections: 1,
            pipeline_depth: 1,
            preload_entries: 0,
            key_space: 1,
            payload_bytes: 8,
            batch_size: 1,
            reset_batch_entries: 128,
            operation_mix: RespOperationMix {
                get_percent: 100,
                set_percent: 0,
                mget_percent: 0,
                mset_percent: 0,
            },
            key_schedule: Arc::new(vec![0]),
            connect_timeout: Duration::from_secs(1),
            io_timeout: Duration::from_millis(10),
            parser_limits: Resp2Limits::default(),
            injected_dispatch_delay: Duration::ZERO,
        })
        .unwrap();
        target.connect_all().await.unwrap();
        assert_eq!(
            target
                .dispatch_operation(RespOperation::Get, 0)
                .await
                .outcome,
            TargetOutcome::Timeout
        );
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(
            target
                .dispatch_operation(RespOperation::Get, 1)
                .await
                .outcome,
            TargetOutcome::Success
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn cancelled_exchange_is_poisoned_before_the_next_request() {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let (request_seen, request_seen_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let mut request_seen = Some(request_seen);
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut input = [0_u8; 1024];
                let read = stream.read(&mut input).await.unwrap();
                assert!(input[..read].windows(4).any(|bytes| bytes == b"PING"));
                stream.write_all(b"+PONG\r\n").await.unwrap();
                let read = stream.read(&mut input).await.unwrap();
                assert!(input[..read].windows(3).any(|bytes| bytes == b"GET"));
                if attempt == 0 {
                    request_seen.take().unwrap().send(()).unwrap();
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                let _ = stream.write_all(b"$-1\r\n").await;
            }
        });
        let target = Arc::new(
            RespTcpTarget::new(RespTargetConfig {
                endpoint: RespEndpointIdentity {
                    address,
                    selected_endpoint: format!("cancel-test@{address}"),
                    endpoint_kind: "loopback-test".to_owned(),
                    state_scope: "test-local".to_owned(),
                },
                require_loopback: true,
                connections: 1,
                pipeline_depth: 1,
                preload_entries: 0,
                key_space: 1,
                payload_bytes: 8,
                batch_size: 1,
                reset_batch_entries: 128,
                operation_mix: RespOperationMix {
                    get_percent: 100,
                    set_percent: 0,
                    mget_percent: 0,
                    mset_percent: 0,
                },
                key_schedule: Arc::new(vec![0]),
                connect_timeout: Duration::from_secs(1),
                io_timeout: Duration::from_secs(1),
                parser_limits: Resp2Limits::default(),
                injected_dispatch_delay: Duration::ZERO,
            })
            .unwrap(),
        );
        target.connect_all().await.unwrap();
        let request = {
            let target = Arc::clone(&target);
            tokio::spawn(async move { target.dispatch_operation(RespOperation::Get, 0).await })
        };
        request_seen_rx.await.unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert_eq!(
            target
                .dispatch_operation(RespOperation::Get, 1)
                .await
                .outcome,
            TargetOutcome::Success
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn reset_chunks_large_keyspaces_within_the_product_batch_limit() {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = Vec::new();
            let mut del_widths = Vec::new();
            while del_widths.len() < 9 {
                let mut chunk = [0_u8; 8192];
                let read = stream.read(&mut chunk).await.unwrap();
                assert_ne!(read, 0);
                buffer.extend_from_slice(&chunk[..read]);
                loop {
                    let Resp2ParseStatus::Complete { value, consumed } =
                        parse_resp2(&buffer, Resp2Limits::default()).unwrap()
                    else {
                        break;
                    };
                    buffer.drain(..consumed);
                    let Resp2Value::Array(Some(arguments)) = value else {
                        panic!("command must be a RESP array")
                    };
                    let command = arguments.first().and_then(|value| match value {
                        Resp2Value::Bulk(Some(value)) => Some(value.as_slice()),
                        _ => None,
                    });
                    match command {
                        Some(b"PING") => stream.write_all(b"+PONG\r\n").await.unwrap(),
                        Some(b"DEL") => {
                            del_widths.push(arguments.len() - 1);
                            stream.write_all(b":0\r\n").await.unwrap();
                        }
                        other => panic!("unexpected reset command: {other:?}"),
                    }
                }
            }
            del_widths
        });
        let target = RespTcpTarget::new(RespTargetConfig {
            endpoint: RespEndpointIdentity {
                address,
                selected_endpoint: format!("reset-test@{address}"),
                endpoint_kind: "loopback-test".to_owned(),
                state_scope: "test-local".to_owned(),
            },
            require_loopback: true,
            connections: 1,
            pipeline_depth: 1,
            preload_entries: 0,
            key_space: 1_025,
            payload_bytes: 8,
            batch_size: 1,
            reset_batch_entries: 128,
            operation_mix: RespOperationMix {
                get_percent: 100,
                set_percent: 0,
                mget_percent: 0,
                mset_percent: 0,
            },
            key_schedule: Arc::new(vec![0]),
            connect_timeout: Duration::from_secs(1),
            io_timeout: Duration::from_secs(1),
            parser_limits: Resp2Limits::default(),
            injected_dispatch_delay: Duration::ZERO,
        })
        .unwrap();
        target.connect_all().await.unwrap();
        target.clear_known_keys().await.unwrap();
        let widths = server.await.unwrap();
        assert_eq!(widths.len(), 9);
        assert_eq!(widths.iter().sum::<usize>(), 1_025);
        assert!(widths.iter().all(|width| *width <= 128));
    }
}
