use hydracache::LogicalTime;

use crate::rng::SimRng;
use crate::storage::checksum;

/// Workload generator configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadConfig {
    /// Number of simulated clients.
    pub clients: u64,
    /// Number of logical keys.
    pub key_count: u64,
    /// Generated value size in bytes.
    pub value_bytes: usize,
    /// Include compare-and-set operations in generated workloads.
    pub include_compare_and_set: bool,
    /// Include session-read operations in generated workloads.
    pub include_session_reads: bool,
}

impl Default for WorkloadConfig {
    fn default() -> Self {
        Self {
            clients: 2,
            key_count: 4,
            value_bytes: 8,
            include_compare_and_set: false,
            include_session_reads: true,
        }
    }
}

impl WorkloadConfig {
    fn normalized(mut self) -> Self {
        self.clients = self.clients.max(1);
        self.key_count = self.key_count.max(1);
        self.value_bytes = self.value_bytes.max(1);
        self
    }
}

/// Simulated client operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadOp {
    /// Read one key.
    Get { key: String },
    /// Write one key.
    Put { key: String, value: Vec<u8> },
    /// Invalidate one key.
    Invalidate { key: String },
    /// Conditional write.
    CompareAndSet {
        key: String,
        expected: Option<Vec<u8>>,
        value: Vec<u8>,
    },
    /// Session read used by monotonic/read-your-writes checkers.
    SessionRead { key: String },
}

/// Result recorded for a completed workload operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadResult {
    /// The operation was accepted.
    Accepted { sequence: u64 },
    /// A read returned this value.
    Value(Option<Vec<u8>>),
    /// A compare-and-set operation was rejected by current state.
    Rejected,
    /// The operation failed with a deterministic error description.
    Error(String),
}

/// Stable id for an event stored in [`History`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(usize);

impl EventId {
    /// Return the zero-based event index.
    pub fn index(self) -> usize {
        self.0
    }
}

/// Invocation/response record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEvent {
    /// Simulated client id.
    pub client: u64,
    /// Operation invoked by the client.
    pub op: WorkloadOp,
    /// Invocation timestamp.
    pub invoked_at: LogicalTime,
    /// Response timestamp, when completed.
    pub returned_at: Option<LogicalTime>,
    /// Operation result, when completed.
    pub result: Option<WorkloadResult>,
}

impl HistoryEvent {
    /// Return whether the event has a response.
    pub fn is_complete(&self) -> bool {
        self.returned_at.is_some() && self.result.is_some()
    }

    fn stable_line(&self) -> String {
        format!(
            "client={} invoked={} returned={} op={} result={}",
            self.client,
            self.invoked_at.as_millis(),
            self.returned_at
                .map(LogicalTime::as_millis)
                .map(|millis| millis.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            stable_op(&self.op),
            self.result
                .as_ref()
                .map(stable_result)
                .unwrap_or_else(|| "-".to_owned())
        )
    }
}

/// Ordered workload history.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    events: Vec<HistoryEvent>,
}

impl History {
    /// Create an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an operation invocation and return its event id.
    pub fn record_invocation(
        &mut self,
        client: u64,
        op: WorkloadOp,
        invoked_at: LogicalTime,
    ) -> EventId {
        let id = EventId(self.events.len());
        self.events.push(HistoryEvent {
            client,
            op,
            invoked_at,
            returned_at: None,
            result: None,
        });
        id
    }

    /// Record a response for a previous invocation.
    pub fn record_response(
        &mut self,
        id: EventId,
        returned_at: LogicalTime,
        result: WorkloadResult,
    ) {
        let event = self
            .events
            .get_mut(id.index())
            .expect("history event id must exist");
        event.returned_at = Some(returned_at);
        event.result = Some(result);
    }

    /// Return all events in invocation order.
    pub fn events(&self) -> &[HistoryEvent] {
        &self.events
    }

    /// Return completed events in invocation order.
    pub fn completed(&self) -> impl Iterator<Item = &HistoryEvent> {
        self.events.iter().filter(|event| event.is_complete())
    }

    /// Return a stable hash of all recorded events.
    pub fn hash(&self) -> u64 {
        let stable = self
            .events
            .iter()
            .map(HistoryEvent::stable_line)
            .collect::<Vec<_>>()
            .join("\n");
        checksum(stable.as_bytes())
    }
}

/// Seeded workload generator.
#[derive(Debug, Clone)]
pub struct WorkloadGenerator {
    cfg: WorkloadConfig,
    rng: SimRng,
}

impl WorkloadGenerator {
    /// Create a generator from seed and config.
    pub fn new(seed: u64, cfg: WorkloadConfig) -> Self {
        Self {
            cfg: cfg.normalized(),
            rng: SimRng::from_seed(seed),
        }
    }

    /// Pick the next client and operation.
    pub fn next_invocation(&mut self) -> (u64, WorkloadOp) {
        let client = self.rng.next_index(self.cfg.clients as usize) as u64;
        (client, self.next_operation())
    }

    /// Generate the next operation.
    pub fn next_operation(&mut self) -> WorkloadOp {
        let kinds = self.operation_kinds();
        match kinds[self.rng.next_index(kinds.len())] {
            WorkloadKind::Get => WorkloadOp::Get {
                key: self.next_key(),
            },
            WorkloadKind::Put => WorkloadOp::Put {
                key: self.next_key(),
                value: self.next_value(),
            },
            WorkloadKind::Invalidate => WorkloadOp::Invalidate {
                key: self.next_key(),
            },
            WorkloadKind::CompareAndSet => WorkloadOp::CompareAndSet {
                key: self.next_key(),
                expected: Some(self.next_value()),
                value: self.next_value(),
            },
            WorkloadKind::SessionRead => WorkloadOp::SessionRead {
                key: self.next_key(),
            },
        }
    }

    fn operation_kinds(&self) -> Vec<WorkloadKind> {
        let mut kinds = vec![
            WorkloadKind::Get,
            WorkloadKind::Put,
            WorkloadKind::Invalidate,
        ];
        if self.cfg.include_compare_and_set {
            kinds.push(WorkloadKind::CompareAndSet);
        }
        if self.cfg.include_session_reads {
            kinds.push(WorkloadKind::SessionRead);
        }
        kinds
    }

    fn next_key(&mut self) -> String {
        format!(
            "sim:key:{}",
            self.rng.next_index(self.cfg.key_count as usize)
        )
    }

    fn next_value(&mut self) -> Vec<u8> {
        (0..self.cfg.value_bytes)
            .map(|_| (self.rng.next_u64() & 0xff) as u8)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkloadKind {
    Get,
    Put,
    Invalidate,
    CompareAndSet,
    SessionRead,
}

fn stable_op(op: &WorkloadOp) -> String {
    match op {
        WorkloadOp::Get { key } => format!("get:{key}"),
        WorkloadOp::Put { key, value } => format!("put:{key}:{}", hex(value)),
        WorkloadOp::Invalidate { key } => format!("invalidate:{key}"),
        WorkloadOp::CompareAndSet {
            key,
            expected,
            value,
        } => format!(
            "cas:{key}:{}:{}",
            expected
                .as_ref()
                .map(|value| hex(value))
                .unwrap_or_else(|| "-".to_owned()),
            hex(value)
        ),
        WorkloadOp::SessionRead { key } => format!("session-read:{key}"),
    }
}

fn stable_result(result: &WorkloadResult) -> String {
    match result {
        WorkloadResult::Accepted { sequence } => format!("accepted:{sequence}"),
        WorkloadResult::Value(value) => value
            .as_ref()
            .map(|value| format!("value:{}", hex(value)))
            .unwrap_or_else(|| "value:-".to_owned()),
        WorkloadResult::Rejected => "rejected".to_owned(),
        WorkloadResult::Error(error) => format!("error:{error}"),
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
