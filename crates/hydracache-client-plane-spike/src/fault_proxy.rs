//! Deterministic, bounded byte-stream fault scheduling for HC/2 transport tests.
//!
//! The scheduler is deliberately transport-neutral. Adapters feed it logical
//! read chunks, then deliver the returned chunks through a real socket or an
//! in-memory test. Replay artifacts retain plans, synthetic input shape, and
//! privacy-safe hashes; they never retain application payload bytes.

use std::cmp::max;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const FAULT_PLAN_SCHEMA: &str = "hydracache.hc2.fault-plan.v1";
pub const FAULT_TRACE_SCHEMA: &str = "hydracache.hc2.fault-trace.v1";
pub const FAULT_REPLAY_SCHEMA: &str = "hydracache.hc2.fault-replay.v1";

const MAX_ACTIONS: usize = 64;
const MAX_COPIES: usize = 4;
const MAX_TICKS: u64 = 1_000_000;
const MAX_CHUNKS: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyDirection {
    ClientToServer,
    ServerToClient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaultAction {
    Pass,
    Fragment {
        max_chunk_bytes: usize,
    },
    Coalesce {
        max_chunks: usize,
    },
    Delay {
        ticks: u64,
    },
    ReorderAdjacent,
    Duplicate {
        copies: usize,
    },
    Drop {
        every_nth: usize,
    },
    BlockDirection,
    HalfOpen,
    Reset,
    LateDelivery {
        ticks: u64,
    },
    BandwidthPressure {
        bytes_per_tick: usize,
        window_bytes: usize,
    },
    CloseAfterBytes {
        bytes: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultPlan {
    pub schema: String,
    pub case_id: String,
    pub seed: u64,
    pub direction: ProxyDirection,
    pub max_buffered_bytes: usize,
    pub max_trace_events: usize,
    pub actions: Vec<FaultAction>,
}

impl FaultPlan {
    pub fn new(
        case_id: impl Into<String>,
        seed: u64,
        direction: ProxyDirection,
        actions: Vec<FaultAction>,
    ) -> Self {
        Self {
            schema: FAULT_PLAN_SCHEMA.to_owned(),
            case_id: case_id.into(),
            seed,
            direction,
            max_buffered_bytes: 1024 * 1024,
            max_trace_events: 512,
            actions,
        }
    }

    pub fn validate(&self) -> Result<(), FaultProxyError> {
        if self.schema != FAULT_PLAN_SCHEMA {
            return Err(FaultProxyError::UnsupportedSchema(self.schema.clone()));
        }
        if self.case_id.trim().is_empty() || self.case_id.len() > 128 {
            return Err(FaultProxyError::InvalidPlan("case_id"));
        }
        if self.max_buffered_bytes == 0 || self.max_trace_events == 0 {
            return Err(FaultProxyError::InvalidPlan("zero_limit"));
        }
        if self.actions.is_empty() || self.actions.len() > MAX_ACTIONS {
            return Err(FaultProxyError::InvalidPlan("actions"));
        }
        if self.actions.len() > self.max_trace_events {
            return Err(FaultProxyError::InvalidPlan("trace_budget"));
        }
        let mut hard_terminal = false;
        for action in &self.actions {
            if hard_terminal {
                return Err(FaultProxyError::InvalidPlan("action_after_terminal"));
            }
            match action {
                FaultAction::Fragment { max_chunk_bytes }
                    if *max_chunk_bytes == 0 || *max_chunk_bytes > self.max_buffered_bytes =>
                {
                    return Err(FaultProxyError::InvalidPlan("fragment"));
                }
                FaultAction::Coalesce { max_chunks }
                    if *max_chunks == 0 || *max_chunks > MAX_CHUNKS =>
                {
                    return Err(FaultProxyError::InvalidPlan("coalesce"));
                }
                FaultAction::Delay { ticks } | FaultAction::LateDelivery { ticks }
                    if *ticks > MAX_TICKS =>
                {
                    return Err(FaultProxyError::InvalidPlan("ticks"));
                }
                FaultAction::Duplicate { copies } if *copies == 0 || *copies > MAX_COPIES => {
                    return Err(FaultProxyError::InvalidPlan("copies"));
                }
                FaultAction::Drop { every_nth } if *every_nth == 0 => {
                    return Err(FaultProxyError::InvalidPlan("drop"));
                }
                FaultAction::BandwidthPressure {
                    bytes_per_tick,
                    window_bytes,
                } if *bytes_per_tick == 0
                    || *window_bytes == 0
                    || *bytes_per_tick > *window_bytes
                    || *window_bytes > self.max_buffered_bytes =>
                {
                    return Err(FaultProxyError::InvalidPlan("bandwidth_pressure"));
                }
                FaultAction::CloseAfterBytes { bytes } if *bytes == 0 => {
                    return Err(FaultProxyError::InvalidPlan("close_after_bytes"));
                }
                _ => {}
            }
            hard_terminal = matches!(
                action,
                FaultAction::BlockDirection
                    | FaultAction::Reset
                    | FaultAction::CloseAfterBytes { .. }
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyTerminal {
    Open,
    Blocked,
    HalfOpen,
    Reset,
    ClosedAfterBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionTrace {
    pub index: usize,
    pub action: FaultAction,
    pub chunks_before: usize,
    pub chunks_after: usize,
    pub bytes_before: usize,
    pub bytes_after: usize,
    pub max_delivery_tick: u64,
    pub terminal: ProxyTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryTrace {
    pub sequence: usize,
    pub tick: u64,
    pub len: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultTrace {
    pub schema: String,
    pub case_id: String,
    pub seed: u64,
    pub direction: ProxyDirection,
    pub input_chunks: usize,
    pub input_bytes: usize,
    pub input_sha256: String,
    pub actions: Vec<ActionTrace>,
    pub deliveries: Vec<DeliveryTrace>,
    pub output_bytes: usize,
    pub output_sha256: String,
    pub terminal: ProxyTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledChunk {
    pub tick: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultExecution {
    pub deliveries: Vec<ScheduledChunk>,
    pub trace: FaultTrace,
}

impl FaultExecution {
    pub fn output_bytes(&self) -> Vec<u8> {
        self.deliveries
            .iter()
            .flat_map(|delivery| delivery.bytes.iter().copied())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntheticInput {
    pub chunk_sizes: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultReplayArtifact {
    pub schema: String,
    pub plan: FaultPlan,
    pub synthetic_input: SyntheticInput,
    pub trace: FaultTrace,
}

impl FaultReplayArtifact {
    pub fn create(plan: FaultPlan, chunk_sizes: Vec<usize>) -> Result<Self, FaultProxyError> {
        let input = synthetic_chunks(plan.seed, &chunk_sizes)?;
        let trace = execute(&plan, input)?.trace;
        Ok(Self {
            schema: FAULT_REPLAY_SCHEMA.to_owned(),
            plan,
            synthetic_input: SyntheticInput { chunk_sizes },
            trace,
        })
    }

    pub fn verify(&self) -> Result<(), FaultProxyError> {
        if self.schema != FAULT_REPLAY_SCHEMA {
            return Err(FaultProxyError::UnsupportedSchema(self.schema.clone()));
        }
        let replayed = Self::create(self.plan.clone(), self.synthetic_input.chunk_sizes.clone())?;
        if replayed.trace != self.trace {
            return Err(FaultProxyError::ReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum FaultProxyError {
    #[error("unsupported fault artifact schema: {0}")]
    UnsupportedSchema(String),
    #[error("invalid fault plan field: {0}")]
    InvalidPlan(&'static str),
    #[error("fault proxy buffered-byte limit exceeded")]
    BufferLimitExceeded,
    #[error("fault proxy trace-event limit exceeded")]
    TraceLimitExceeded,
    #[error("fault replay does not reproduce the retained trace")]
    ReplayMismatch,
    #[error("fault proxy I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
struct Packet {
    tick: u64,
    bytes: Vec<u8>,
}

pub fn execute(
    plan: &FaultPlan,
    input_chunks: Vec<Vec<u8>>,
) -> Result<FaultExecution, FaultProxyError> {
    plan.validate()?;
    if input_chunks.len() > MAX_CHUNKS || input_chunks.iter().any(Vec::is_empty) {
        return Err(FaultProxyError::InvalidPlan("input_chunks"));
    }
    let input_chunk_count = input_chunks.len();
    let input_bytes = checked_total(&input_chunks, plan.max_buffered_bytes)?;
    let input_flat: Vec<u8> = input_chunks.iter().flatten().copied().collect();
    let mut packets: Vec<Packet> = input_chunks
        .into_iter()
        .map(|bytes| Packet { tick: 0, bytes })
        .collect();
    let mut terminal = ProxyTerminal::Open;
    let mut rng = DeterministicRng::new(plan.seed);
    let mut action_trace = Vec::with_capacity(plan.actions.len());

    for (index, action) in plan.actions.iter().cloned().enumerate() {
        let chunks_before = packets.len();
        let bytes_before = packet_bytes(&packets)?;
        ensure_transform_bounds(
            &action,
            chunks_before,
            bytes_before,
            plan.max_buffered_bytes,
        )?;
        packets = apply_action(packets, &action, &mut terminal, &mut rng)?;
        if packets.len() > MAX_CHUNKS {
            return Err(FaultProxyError::BufferLimitExceeded);
        }
        let bytes_after = packet_bytes(&packets)?;
        if bytes_after > plan.max_buffered_bytes {
            return Err(FaultProxyError::BufferLimitExceeded);
        }
        let max_delivery_tick = packets.iter().map(|packet| packet.tick).max().unwrap_or(0);
        action_trace.push(ActionTrace {
            index,
            action,
            chunks_before,
            chunks_after: packets.len(),
            bytes_before,
            bytes_after,
            max_delivery_tick,
            terminal,
        });
    }

    packets.sort_by_key(|packet| packet.tick);
    let deliveries: Vec<ScheduledChunk> = packets
        .into_iter()
        .map(|packet| ScheduledChunk {
            tick: packet.tick,
            bytes: packet.bytes,
        })
        .collect();
    if action_trace.len().saturating_add(deliveries.len()) > plan.max_trace_events {
        return Err(FaultProxyError::TraceLimitExceeded);
    }
    let output: Vec<u8> = deliveries
        .iter()
        .flat_map(|delivery| delivery.bytes.iter().copied())
        .collect();
    let delivery_trace: Vec<DeliveryTrace> = deliveries
        .iter()
        .enumerate()
        .map(|(sequence, delivery)| DeliveryTrace {
            sequence,
            tick: delivery.tick,
            len: delivery.bytes.len(),
            sha256: sha256_hex(&delivery.bytes),
        })
        .collect();
    let trace = FaultTrace {
        schema: FAULT_TRACE_SCHEMA.to_owned(),
        case_id: plan.case_id.clone(),
        seed: plan.seed,
        direction: plan.direction,
        input_chunks: input_chunk_count,
        input_bytes,
        input_sha256: sha256_hex(&input_flat),
        actions: action_trace,
        deliveries: delivery_trace,
        output_bytes: output.len(),
        output_sha256: sha256_hex(&output),
        terminal,
    };
    Ok(FaultExecution { deliveries, trace })
}

fn apply_action(
    packets: Vec<Packet>,
    action: &FaultAction,
    terminal: &mut ProxyTerminal,
    rng: &mut DeterministicRng,
) -> Result<Vec<Packet>, FaultProxyError> {
    match *action {
        FaultAction::Pass => Ok(packets),
        FaultAction::Fragment { max_chunk_bytes } => {
            let mut fragmented = Vec::new();
            for packet in packets {
                let mut offset = 0;
                while offset < packet.bytes.len() {
                    if fragmented.len() == MAX_CHUNKS {
                        return Err(FaultProxyError::BufferLimitExceeded);
                    }
                    let remaining = packet.bytes.len() - offset;
                    let bound = max_chunk_bytes.min(remaining) as u64;
                    let width = 1 + (rng.next_u64() % bound) as usize;
                    fragmented.push(Packet {
                        tick: packet.tick,
                        bytes: packet.bytes[offset..offset + width].to_vec(),
                    });
                    offset += width;
                }
            }
            Ok(fragmented)
        }
        FaultAction::Coalesce { max_chunks } => {
            let mut coalesced = Vec::new();
            let mut current = Vec::new();
            let mut tick = 0;
            let mut count = 0;
            for packet in packets {
                tick = max(tick, packet.tick);
                current.extend_from_slice(&packet.bytes);
                count += 1;
                if count == max_chunks {
                    coalesced.push(Packet {
                        tick,
                        bytes: std::mem::take(&mut current),
                    });
                    tick = 0;
                    count = 0;
                }
            }
            if !current.is_empty() {
                coalesced.push(Packet {
                    tick,
                    bytes: current,
                });
            }
            Ok(coalesced)
        }
        FaultAction::Delay { ticks } | FaultAction::LateDelivery { ticks } => packets
            .into_iter()
            .map(|mut packet| {
                packet.tick = packet
                    .tick
                    .checked_add(ticks)
                    .ok_or(FaultProxyError::InvalidPlan("tick_overflow"))?;
                Ok(packet)
            })
            .collect(),
        FaultAction::ReorderAdjacent => {
            let mut reordered = packets;
            for pair in reordered.chunks_mut(2) {
                if pair.len() == 2 && pair[0].tick == pair[1].tick {
                    pair.swap(0, 1);
                }
            }
            Ok(reordered)
        }
        FaultAction::Duplicate { copies } => {
            let mut duplicated = Vec::with_capacity(packets.len().saturating_mul(copies + 1));
            for packet in packets {
                for _ in 0..=copies {
                    duplicated.push(packet.clone());
                }
            }
            Ok(duplicated)
        }
        FaultAction::Drop { every_nth } => Ok(packets
            .into_iter()
            .enumerate()
            .filter_map(|(index, packet)| ((index + 1) % every_nth != 0).then_some(packet))
            .collect()),
        FaultAction::BlockDirection => {
            *terminal = ProxyTerminal::Blocked;
            Ok(Vec::new())
        }
        FaultAction::HalfOpen => {
            *terminal = ProxyTerminal::HalfOpen;
            Ok(packets)
        }
        FaultAction::Reset => {
            *terminal = ProxyTerminal::Reset;
            Ok(Vec::new())
        }
        FaultAction::BandwidthPressure {
            bytes_per_tick,
            window_bytes,
        } => {
            let slots_per_window = (window_bytes / bytes_per_tick).max(1);
            let mut pressured = Vec::new();
            let mut slot = 0usize;
            for packet in packets {
                for bytes in packet.bytes.chunks(bytes_per_tick) {
                    if pressured.len() == MAX_CHUNKS {
                        return Err(FaultProxyError::BufferLimitExceeded);
                    }
                    let window_pause = slot / slots_per_window;
                    let tick = packet
                        .tick
                        .checked_add(slot as u64)
                        .and_then(|value| value.checked_add(window_pause as u64))
                        .ok_or(FaultProxyError::InvalidPlan("tick_overflow"))?;
                    pressured.push(Packet {
                        tick,
                        bytes: bytes.to_vec(),
                    });
                    slot += 1;
                }
            }
            Ok(pressured)
        }
        FaultAction::CloseAfterBytes { bytes } => {
            let available = packet_bytes(&packets)?;
            let mut remaining = bytes;
            let mut closed = Vec::new();
            for packet in packets {
                if remaining == 0 {
                    break;
                }
                let retained = packet.bytes.len().min(remaining);
                closed.push(Packet {
                    tick: packet.tick,
                    bytes: packet.bytes[..retained].to_vec(),
                });
                remaining -= retained;
            }
            if available >= bytes {
                *terminal = ProxyTerminal::ClosedAfterBytes;
            }
            Ok(closed)
        }
    }
}

fn ensure_transform_bounds(
    action: &FaultAction,
    chunks_before: usize,
    bytes_before: usize,
    byte_limit: usize,
) -> Result<(), FaultProxyError> {
    let (projected_chunks, projected_bytes) = match action {
        FaultAction::Duplicate { copies } => (
            chunks_before
                .checked_mul(copies + 1)
                .ok_or(FaultProxyError::BufferLimitExceeded)?,
            bytes_before
                .checked_mul(copies + 1)
                .ok_or(FaultProxyError::BufferLimitExceeded)?,
        ),
        _ => (chunks_before, bytes_before),
    };
    if projected_chunks > MAX_CHUNKS || projected_bytes > byte_limit {
        return Err(FaultProxyError::BufferLimitExceeded);
    }
    Ok(())
}

/// Read one bounded direction to EOF, apply the deterministic schedule, and
/// deliver it to a real async writer. A non-open terminal shuts down the writer.
pub async fn proxy_one_way<R, W>(
    reader: &mut R,
    writer: &mut W,
    plan: &FaultPlan,
    read_chunk_bytes: usize,
    tick_duration: Duration,
) -> Result<FaultTrace, FaultProxyError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if read_chunk_bytes == 0 || read_chunk_bytes > plan.max_buffered_bytes {
        return Err(FaultProxyError::InvalidPlan("read_chunk_bytes"));
    }
    let mut input = Vec::new();
    let mut retained = 0usize;
    loop {
        let mut buffer = vec![0; read_chunk_bytes];
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        buffer.truncate(read);
        retained = retained
            .checked_add(read)
            .ok_or(FaultProxyError::BufferLimitExceeded)?;
        if retained > plan.max_buffered_bytes {
            return Err(FaultProxyError::BufferLimitExceeded);
        }
        input.push(buffer);
    }
    let execution = execute(plan, input)?;
    let mut last_tick = 0;
    for delivery in &execution.deliveries {
        let delta = delivery.tick.saturating_sub(last_tick);
        if delta > 0 && !tick_duration.is_zero() {
            tokio::time::sleep(tick_duration.saturating_mul(delta as u32)).await;
        }
        writer.write_all(&delivery.bytes).await?;
        last_tick = delivery.tick;
    }
    writer.flush().await?;
    if matches!(
        execution.trace.terminal,
        ProxyTerminal::HalfOpen | ProxyTerminal::ClosedAfterBytes
    ) {
        writer.shutdown().await?;
    }
    Ok(execution.trace)
}

fn synthetic_chunks(seed: u64, sizes: &[usize]) -> Result<Vec<Vec<u8>>, FaultProxyError> {
    if sizes.is_empty() || sizes.len() > MAX_CHUNKS || sizes.contains(&0) {
        return Err(FaultProxyError::InvalidPlan("synthetic_input"));
    }
    let mut rng = DeterministicRng::new(seed ^ 0xa076_1d64_78bd_642f);
    Ok(sizes
        .iter()
        .map(|size| (0..*size).map(|_| (rng.next_u64() & 0xff) as u8).collect())
        .collect())
}

fn checked_total(chunks: &[Vec<u8>], limit: usize) -> Result<usize, FaultProxyError> {
    let mut total = 0usize;
    for chunk in chunks {
        total = total
            .checked_add(chunk.len())
            .ok_or(FaultProxyError::BufferLimitExceeded)?;
        if total > limit {
            return Err(FaultProxyError::BufferLimitExceeded);
        }
    }
    Ok(total)
}

fn packet_bytes(packets: &[Packet]) -> Result<usize, FaultProxyError> {
    packets.iter().try_fold(0usize, |total, packet| {
        total
            .checked_add(packet.bytes.len())
            .ok_or(FaultProxyError::BufferLimitExceeded)
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        })
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
}
