use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::ClusterNodeId;

/// Logical simulator time in milliseconds.
///
/// Production code can adapt wall-clock time into this type at the driver edge,
/// while deterministic tests and simulators advance it explicitly.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct LogicalTime(u64);

impl LogicalTime {
    /// Construct a logical timestamp from milliseconds.
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// Return the timestamp as milliseconds.
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// Return a timestamp advanced by `duration`.
    pub const fn saturating_add(self, duration: LogicalDuration) -> Self {
        Self(self.0.saturating_add(duration.as_millis()))
    }
}

/// Logical simulator duration in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LogicalDuration(u64);

impl LogicalDuration {
    /// Construct a logical duration from milliseconds.
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// Return the duration as milliseconds.
    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

impl Default for LogicalDuration {
    fn default() -> Self {
        Self(1_000)
    }
}

/// Deterministic clock seam used by the sans-IO cluster node.
pub trait ClusterClock {
    /// Return the current logical time.
    fn now(&self) -> LogicalTime;
}

/// Manually advanced clock for tests and simulators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ManualClusterClock {
    now: LogicalTime,
}

impl ManualClusterClock {
    /// Create a clock at `now`.
    pub const fn new(now: LogicalTime) -> Self {
        Self { now }
    }

    /// Move the clock to an absolute timestamp.
    pub fn set(&mut self, now: LogicalTime) {
        self.now = now;
    }

    /// Advance the clock by `duration`.
    pub fn advance(&mut self, duration: LogicalDuration) {
        self.now = self.now.saturating_add(duration);
    }
}

impl ClusterClock for ManualClusterClock {
    fn now(&self) -> LogicalTime {
        self.now
    }
}

/// Client operation accepted by the sans-IO node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientOp {
    /// Store a value and replicate it to peers.
    Put { key: String, value: Vec<u8> },
    /// Read a value through the storage seam.
    Get { key: String },
    /// Remove one key and replicate the invalidation to peers.
    Invalidate { key: String },
    /// Remove all local keys and replicate the flush to peers.
    Flush,
}

/// Immediate acknowledgement returned by the sans-IO node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientAck {
    /// The operation has been accepted and assigned a deterministic sequence.
    Accepted { sequence: u64 },
    /// The operation is waiting for a storage result.
    PendingStorage { request_id: u64 },
}

/// Transport-neutral cluster message emitted by a sans-IO node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClusterNodeMessage {
    /// Heartbeat emitted by `tick`.
    Heartbeat { at: LogicalTime, sequence: u64 },
    /// A replicated write.
    ReplicatePut {
        key: String,
        value: Vec<u8>,
        sequence: u64,
    },
    /// A replicated key invalidation.
    ReplicateInvalidate { key: String, sequence: u64 },
    /// A replicated flush.
    ReplicateFlush { sequence: u64 },
    /// Acknowledgement for a received message.
    Ack { sequence: u64 },
}

/// Message ready to be sent by a production or simulator driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundClusterMessage {
    /// Source node.
    pub from: ClusterNodeId,
    /// Destination node.
    pub to: ClusterNodeId,
    /// Message payload.
    pub message: ClusterNodeMessage,
}

/// Storage request kind emitted by a sans-IO node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StorageOpKind {
    /// Read one key.
    Read { key: String },
    /// Write one key.
    Write { key: String, value: Vec<u8> },
    /// Delete one key.
    Delete { key: String },
    /// Delete all keys.
    Flush,
}

/// Storage request ready to be performed by the driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageOp {
    /// Monotonic request id.
    pub request_id: u64,
    /// Request payload.
    pub kind: StorageOpKind,
}

/// Storage result returned by the driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageResult {
    /// Request id originally emitted by [`StorageOp`].
    pub request_id: u64,
    /// Read value, when the request was a read.
    pub value: Option<Vec<u8>>,
}

/// Deterministic storage seam for production and simulation drivers.
pub trait ClusterStorage {
    /// Apply one storage operation and return its result.
    fn apply(&mut self, op: StorageOp) -> StorageResult;
}

/// In-memory deterministic storage implementation useful for tests.
#[derive(Debug, Clone, Default)]
pub struct InMemoryClusterStorage {
    values: BTreeMap<String, Vec<u8>>,
}

impl InMemoryClusterStorage {
    /// Return a stored value.
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.values.get(key).map(Vec::as_slice)
    }
}

impl ClusterStorage for InMemoryClusterStorage {
    fn apply(&mut self, op: StorageOp) -> StorageResult {
        let value = match op.kind {
            StorageOpKind::Read { key } => self.values.get(&key).cloned(),
            StorageOpKind::Write { key, value } => {
                self.values.insert(key, value);
                None
            }
            StorageOpKind::Delete { key } => {
                self.values.remove(&key);
                None
            }
            StorageOpKind::Flush => {
                self.values.clear();
                None
            }
        };
        StorageResult {
            request_id: op.request_id,
            value,
        }
    }
}

/// Configuration for a sans-IO cluster node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterNodeConfig {
    /// Local node id.
    pub node_id: ClusterNodeId,
    /// Stable peer set.
    pub peers: Vec<ClusterNodeId>,
    /// Heartbeat interval used by [`ClusterNode::tick`].
    pub heartbeat_interval: LogicalDuration,
}

impl ClusterNodeConfig {
    /// Build a config and normalize peers into deterministic order.
    pub fn new(node_id: impl Into<ClusterNodeId>, peers: Vec<ClusterNodeId>) -> Self {
        let node_id = node_id.into();
        let peers = peers
            .into_iter()
            .filter(|peer| peer != &node_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            node_id,
            peers,
            heartbeat_interval: LogicalDuration::default(),
        }
    }

    /// Override the heartbeat interval.
    pub fn heartbeat_interval(mut self, interval: LogicalDuration) -> Self {
        self.heartbeat_interval = interval;
        self
    }
}

/// IO-free cluster node state machine.
///
/// The node never talks to sockets, disks, wall-clock time, or task schedulers
/// directly. Drivers call methods, then drain outbound messages and storage
/// requests in deterministic order.
#[derive(Debug, Clone)]
pub struct ClusterNode {
    config: ClusterNodeConfig,
    next_sequence: u64,
    next_storage_request: u64,
    last_heartbeat_at: Option<LogicalTime>,
    outbound: VecDeque<OutboundClusterMessage>,
    storage_requests: VecDeque<StorageOp>,
    storage_results: BTreeMap<u64, StorageResult>,
}

impl ClusterNode {
    /// Create a node from config.
    pub fn new(config: ClusterNodeConfig) -> Self {
        Self {
            config,
            next_sequence: 1,
            next_storage_request: 1,
            last_heartbeat_at: None,
            outbound: VecDeque::new(),
            storage_requests: VecDeque::new(),
            storage_results: BTreeMap::new(),
        }
    }

    /// Return the local node id.
    pub fn node_id(&self) -> &ClusterNodeId {
        &self.config.node_id
    }

    /// Return peers in deterministic order.
    pub fn peers(&self) -> &[ClusterNodeId] {
        &self.config.peers
    }

    /// Advance node timers at `now`.
    pub fn tick(&mut self, now: LogicalTime) {
        let should_heartbeat = self
            .last_heartbeat_at
            .map(|last| {
                now.as_millis().saturating_sub(last.as_millis())
                    >= self.config.heartbeat_interval.as_millis()
            })
            .unwrap_or(true);
        if should_heartbeat {
            self.last_heartbeat_at = Some(now);
            let sequence = self.next_sequence();
            self.broadcast(ClusterNodeMessage::Heartbeat { at: now, sequence });
        }
    }

    /// Handle an inbound cluster message.
    pub fn handle_message(&mut self, from: ClusterNodeId, message: ClusterNodeMessage) {
        match message {
            ClusterNodeMessage::Heartbeat { sequence, .. } => {
                self.enqueue_outbound(from, ClusterNodeMessage::Ack { sequence });
            }
            ClusterNodeMessage::Ack { .. } => {}
            ClusterNodeMessage::ReplicatePut {
                key,
                value,
                sequence,
            } => {
                self.enqueue_storage(StorageOpKind::Write { key, value });
                self.enqueue_outbound(from, ClusterNodeMessage::Ack { sequence });
            }
            ClusterNodeMessage::ReplicateInvalidate { key, sequence } => {
                self.enqueue_storage(StorageOpKind::Delete { key });
                self.enqueue_outbound(from, ClusterNodeMessage::Ack { sequence });
            }
            ClusterNodeMessage::ReplicateFlush { sequence } => {
                self.enqueue_storage(StorageOpKind::Flush);
                self.enqueue_outbound(from, ClusterNodeMessage::Ack { sequence });
            }
        }
    }

    /// Handle a client operation and emit storage/network side effects.
    pub fn handle_client(&mut self, op: ClientOp) -> ClientAck {
        match op {
            ClientOp::Put { key, value } => {
                let sequence = self.next_sequence();
                self.enqueue_storage(StorageOpKind::Write {
                    key: key.clone(),
                    value: value.clone(),
                });
                self.broadcast(ClusterNodeMessage::ReplicatePut {
                    key,
                    value,
                    sequence,
                });
                ClientAck::Accepted { sequence }
            }
            ClientOp::Get { key } => {
                let request_id = self.enqueue_storage(StorageOpKind::Read { key });
                ClientAck::PendingStorage { request_id }
            }
            ClientOp::Invalidate { key } => {
                let sequence = self.next_sequence();
                self.enqueue_storage(StorageOpKind::Delete { key: key.clone() });
                self.broadcast(ClusterNodeMessage::ReplicateInvalidate { key, sequence });
                ClientAck::Accepted { sequence }
            }
            ClientOp::Flush => {
                let sequence = self.next_sequence();
                self.enqueue_storage(StorageOpKind::Flush);
                self.broadcast(ClusterNodeMessage::ReplicateFlush { sequence });
                ClientAck::Accepted { sequence }
            }
        }
    }

    /// Drain outbound messages in deterministic order.
    pub fn take_outbound(&mut self) -> Vec<OutboundClusterMessage> {
        self.outbound.drain(..).collect()
    }

    /// Drain storage requests in deterministic order.
    pub fn storage_requests(&mut self) -> Vec<StorageOp> {
        self.storage_requests.drain(..).collect()
    }

    /// Apply a storage result returned by a driver.
    pub fn apply_storage_result(&mut self, result: StorageResult) {
        self.storage_results.insert(result.request_id, result);
    }

    /// Return a previously applied storage result.
    pub fn storage_result(&self, request_id: u64) -> Option<&StorageResult> {
        self.storage_results.get(&request_id)
    }

    fn next_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        sequence
    }

    fn enqueue_storage(&mut self, kind: StorageOpKind) -> u64 {
        let request_id = self.next_storage_request;
        self.next_storage_request = self.next_storage_request.saturating_add(1);
        self.storage_requests
            .push_back(StorageOp { request_id, kind });
        request_id
    }

    fn broadcast(&mut self, message: ClusterNodeMessage) {
        for peer in self.config.peers.clone() {
            self.enqueue_outbound(peer, message.clone());
        }
    }

    fn enqueue_outbound(&mut self, to: ClusterNodeId, message: ClusterNodeMessage) {
        self.outbound.push_back(OutboundClusterMessage {
            from: self.config.node_id.clone(),
            to,
            message,
        });
    }
}
