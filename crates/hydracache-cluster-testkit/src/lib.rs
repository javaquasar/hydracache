//! Deterministic cluster-correctness harnesses shared by HydraCache tests.
//!
//! This crate is intentionally `publish = false` and is consumed only from
//! dev-dependencies. It owns the fault-injection vocabulary so production crates
//! do not grow test-only transport types.

pub mod invariants;

use std::collections::{BTreeMap, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use chitchat::transport::{ChannelTransport, Socket, Transport};
use chitchat::{ChitchatMessage, Deserializable, Serializable};
use hydracache::{
    CacheError, CacheResult, ClusterControlPlane, ClusterDiscovery, ClusterDiscoveryLiveness,
    ClusterGeneration, ClusterNodeId,
};
use hydracache_cluster_raft::{
    InMemoryRaftLogStore, RaftLogStore, RaftMessageSink, RaftMetadataRuntime,
    RaftMetadataRuntimeConfig, RaftRuntimeRole, RaftWireMessage,
};
use raft::eraftpb::{Message as RaftMessage, MessageType, Snapshot};
use raft::Storage;

/// Delivery direction used by packet filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftFilterDirection {
    /// Match messages sent from the selected node.
    Send,
    /// Match messages received by the selected node.
    Recv,
    /// Match both directions.
    Both,
}

/// Action taken when a message filter matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftFilterAction {
    /// Deliver the message unchanged.
    Pass,
    /// Drop the message.
    Drop,
    /// Deliver the message after the given logical tick count.
    Delay(u64),
    /// Deliver the original plus `extra` duplicates.
    Duplicate(usize),
    /// Retain the message until the test explicitly releases it.
    Hold,
}

/// Deterministic trace event emitted by the filter harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftMessageTraceEvent {
    /// Logical filter tick.
    pub tick: u64,
    /// Source raft node id.
    pub from: u64,
    /// Destination raft node id.
    pub to: u64,
    /// Decoded raft message type.
    pub message_type: Option<MessageType>,
    /// Harness action label.
    pub action: &'static str,
}

/// Test-only predicate over one outbound raft message.
pub trait RaftMessageFilter: Send + Sync {
    /// Decide how one message should be handled at the current logical tick.
    fn filter(&self, tick: u64, message: &RaftWireMessage) -> RaftFilterAction;
}

/// A TiKV-style packet filter with optional direction, endpoint, and type selectors.
#[derive(Debug)]
pub struct RaftPacketFilter {
    node: Option<u64>,
    peer: Option<u64>,
    direction: RaftFilterDirection,
    message_type: Option<MessageType>,
    allow_remaining: AtomicU64,
    action: RaftFilterAction,
}

impl RaftPacketFilter {
    /// Drop every message from `from` to `to`.
    pub fn drop_between(from: u64, to: u64) -> Self {
        Self::new().from(from).to(to).action(RaftFilterAction::Drop)
    }

    /// Start a pass-through filter builder.
    pub fn new() -> Self {
        Self {
            node: None,
            peer: None,
            direction: RaftFilterDirection::Send,
            message_type: None,
            allow_remaining: AtomicU64::new(0),
            action: RaftFilterAction::Pass,
        }
    }

    /// Match messages sent by this raft node.
    pub fn from(mut self, node: u64) -> Self {
        self.node = Some(node);
        self.direction = RaftFilterDirection::Send;
        self
    }

    /// Match messages received by this raft node.
    pub fn recv(mut self, node: u64) -> Self {
        self.node = Some(node);
        self.direction = RaftFilterDirection::Recv;
        self
    }

    /// Match messages between the selected node and this peer.
    pub fn to(mut self, peer: u64) -> Self {
        self.peer = Some(peer);
        self
    }

    /// Match both send and receive directions for the selected node/peer pair.
    pub fn both_directions(mut self) -> Self {
        self.direction = RaftFilterDirection::Both;
        self
    }

    /// Match only a specific raft message type.
    pub fn message_type(mut self, message_type: MessageType) -> Self {
        self.message_type = Some(message_type);
        self
    }

    /// Allow the first `count` matching messages before applying the action.
    pub fn allow(mut self, count: u64) -> Self {
        self.allow_remaining = AtomicU64::new(count);
        self
    }

    /// Set the action for matching messages.
    pub fn action(mut self, action: RaftFilterAction) -> Self {
        self.action = action;
        self
    }

    fn matches(&self, message: &RaftWireMessage) -> bool {
        if let Some(message_type) = self.message_type {
            let Ok(decoded) = message.decode() else {
                return false;
            };
            if decoded.get_msg_type() != message_type {
                return false;
            }
        }

        let Some(node) = self.node else {
            return true;
        };
        let peer_matches = |peer: u64| self.peer.is_none_or(|expected| expected == peer);
        match self.direction {
            RaftFilterDirection::Send => message.from == node && peer_matches(message.to),
            RaftFilterDirection::Recv => message.to == node && peer_matches(message.from),
            RaftFilterDirection::Both => {
                (message.from == node && peer_matches(message.to))
                    || (message.to == node && peer_matches(message.from))
            }
        }
    }
}

impl Default for RaftPacketFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl RaftMessageFilter for RaftPacketFilter {
    fn filter(&self, _tick: u64, message: &RaftWireMessage) -> RaftFilterAction {
        if !self.matches(message) {
            return RaftFilterAction::Pass;
        }
        let mut remaining = self.allow_remaining.load(Ordering::SeqCst);
        while remaining > 0 {
            match self.allow_remaining.compare_exchange(
                remaining,
                remaining - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return RaftFilterAction::Pass,
                Err(actual) => remaining = actual,
            }
        }
        self.action
    }
}

/// Shared deterministic filter state.
#[derive(Clone, Default)]
pub struct RaftFilterSet {
    filters: Arc<RwLock<Vec<Arc<dyn RaftMessageFilter>>>>,
    delayed: Arc<Mutex<BTreeMap<u64, Vec<RaftWireMessage>>>>,
    dropped: Arc<Mutex<Vec<RaftWireMessage>>>,
    held: Arc<Mutex<Vec<RaftWireMessage>>>,
    trace: Arc<Mutex<Vec<RaftMessageTraceEvent>>>,
    tick: Arc<AtomicU64>,
}

impl RaftFilterSet {
    /// Add one filter.
    pub fn add_filter(&self, filter: impl RaftMessageFilter + 'static) {
        self.filters
            .write()
            .expect("raft filter set poisoned")
            .push(Arc::new(filter));
    }

    /// Drop both directions between two raft nodes.
    pub fn cut(&self, left: u64, right: u64) {
        self.add_filter(RaftPacketFilter::drop_between(left, right));
        self.add_filter(RaftPacketFilter::drop_between(right, left));
    }

    /// Drop every message to and from one raft node and its known peers.
    pub fn isolate(&self, node: u64, peers: impl IntoIterator<Item = u64>) {
        for peer in peers {
            if peer != node {
                self.cut(node, peer);
            }
        }
    }

    /// Clear filters and delayed messages.
    pub fn recover(&self) {
        self.filters
            .write()
            .expect("raft filter set poisoned")
            .clear();
        self.delayed
            .lock()
            .expect("raft delayed queue poisoned")
            .clear();
    }

    /// Return dropped messages reserved for assertions.
    pub fn dropped(&self) -> Vec<RaftWireMessage> {
        self.dropped
            .lock()
            .expect("raft dropped queue poisoned")
            .clone()
    }

    /// Return held messages without releasing them.
    pub fn held(&self) -> Vec<RaftWireMessage> {
        self.held.lock().expect("raft held queue poisoned").clone()
    }

    /// Release every explicitly held message in insertion order.
    pub fn release_held(&self) -> Vec<RaftWireMessage> {
        std::mem::take(&mut *self.held.lock().expect("raft held queue poisoned"))
    }

    /// Return the deterministic trace.
    pub fn trace(&self) -> Vec<RaftMessageTraceEvent> {
        self.trace
            .lock()
            .expect("raft trace queue poisoned")
            .clone()
    }

    /// Advance the logical filter tick and return messages whose delay expired.
    pub fn advance_tick(&self) -> Vec<RaftWireMessage> {
        let tick = self.tick.fetch_add(1, Ordering::SeqCst) + 1;
        self.take_due(tick)
    }

    /// Filter one message and return messages deliverable at the current tick.
    pub fn apply(&self, message: RaftWireMessage) -> Vec<RaftWireMessage> {
        let tick = self.tick.load(Ordering::SeqCst);
        let action = self
            .filters
            .read()
            .expect("raft filter set poisoned")
            .iter()
            .map(|filter| filter.filter(tick, &message))
            .find(|action| *action != RaftFilterAction::Pass)
            .unwrap_or(RaftFilterAction::Pass);
        self.apply_action(tick, message, action)
    }

    fn take_due(&self, tick: u64) -> Vec<RaftWireMessage> {
        let mut delayed = self.delayed.lock().expect("raft delayed queue poisoned");
        let due_ticks = delayed
            .range(..=tick)
            .map(|(due_tick, _)| *due_tick)
            .collect::<Vec<_>>();
        let mut due = Vec::new();
        for due_tick in due_ticks {
            if let Some(messages) = delayed.remove(&due_tick) {
                for message in messages {
                    self.record(tick, &message, "delivered");
                    due.push(message);
                }
            }
        }
        due
    }

    fn apply_action(
        &self,
        tick: u64,
        message: RaftWireMessage,
        action: RaftFilterAction,
    ) -> Vec<RaftWireMessage> {
        match action {
            RaftFilterAction::Pass => {
                self.record(tick, &message, "delivered");
                vec![message]
            }
            RaftFilterAction::Drop => {
                self.record(tick, &message, "dropped");
                self.dropped
                    .lock()
                    .expect("raft dropped queue poisoned")
                    .push(message);
                Vec::new()
            }
            RaftFilterAction::Delay(delay) => {
                self.record(tick, &message, "delayed");
                self.delayed
                    .lock()
                    .expect("raft delayed queue poisoned")
                    .entry(tick.saturating_add(delay.max(1)))
                    .or_default()
                    .push(message);
                Vec::new()
            }
            RaftFilterAction::Duplicate(extra) => {
                self.record(tick, &message, "delivered");
                let mut messages = Vec::with_capacity(extra.saturating_add(1));
                messages.push(message.clone());
                for _ in 0..extra {
                    self.record(tick, &message, "duplicated");
                    messages.push(message.clone());
                }
                messages
            }
            RaftFilterAction::Hold => {
                self.record(tick, &message, "held");
                self.held
                    .lock()
                    .expect("raft held queue poisoned")
                    .push(message);
                Vec::new()
            }
        }
    }

    fn record(&self, tick: u64, message: &RaftWireMessage, action: &'static str) {
        let message_type = message.decode().ok().map(|decoded| decoded.get_msg_type());
        self.trace
            .lock()
            .expect("raft trace queue poisoned")
            .push(RaftMessageTraceEvent {
                tick,
                from: message.from,
                to: message.to,
                message_type,
                action,
            });
    }
}

/// `RaftMessageSink` decorator backed by [`RaftFilterSet`].
pub struct FilteredRaftMessageSink {
    inner: Arc<dyn RaftMessageSink>,
    filters: RaftFilterSet,
}

impl FilteredRaftMessageSink {
    /// Wrap an existing sink.
    pub fn new(inner: Arc<dyn RaftMessageSink>, filters: RaftFilterSet) -> Self {
        Self { inner, filters }
    }

    /// Return the shared filter set.
    pub fn filters(&self) -> RaftFilterSet {
        self.filters.clone()
    }

    /// Deliver messages whose logical delay expired.
    pub async fn deliver_delayed(&self) -> CacheResult<()> {
        for message in self.filters.advance_tick() {
            self.inner.send(message).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl RaftMessageSink for FilteredRaftMessageSink {
    async fn send(&self, message: RaftWireMessage) -> CacheResult<()> {
        for message in self.filters.apply(message) {
            self.inner.send(message).await?;
        }
        Ok(())
    }
}

/// Delivery direction used by gossip packet filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipFilterDirection {
    /// Match messages sent from the selected address.
    Send,
    /// Match messages received by the selected address.
    Recv,
    /// Match both directions.
    Both,
}

/// Action taken when a gossip packet filter matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipFilterAction {
    /// Deliver the message unchanged.
    Pass,
    /// Drop the message.
    Drop,
    /// Deliver the message after the given logical tick count.
    Delay(u64),
    /// Deliver the original plus `extra` duplicates.
    Duplicate(usize),
}

/// Public message-kind label for chitchat's three-way gossip exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipMessageType {
    /// Scuttlebutt SYN.
    Syn,
    /// Scuttlebutt SYN-ACK.
    SynAck,
    /// Scuttlebutt ACK.
    Ack,
    /// Bad-cluster rejection.
    BadCluster,
}

impl GossipMessageType {
    fn from_message(message: &ChitchatMessage) -> Self {
        match message {
            ChitchatMessage::Syn { .. } => Self::Syn,
            ChitchatMessage::SynAck { .. } => Self::SynAck,
            ChitchatMessage::Ack { .. } => Self::Ack,
            ChitchatMessage::BadCluster => Self::BadCluster,
        }
    }
}

/// Deterministic trace event emitted by the gossip filter harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipMessageTraceEvent {
    /// Logical filter tick.
    pub tick: u64,
    /// Source gossip address.
    pub from: SocketAddr,
    /// Destination gossip address.
    pub to: SocketAddr,
    /// Chitchat message kind.
    pub message_type: GossipMessageType,
    /// Harness action label.
    pub action: &'static str,
}

#[derive(Debug, Clone)]
struct SerializedGossipMessage {
    from: SocketAddr,
    to: SocketAddr,
    payload: Vec<u8>,
    message_type: GossipMessageType,
}

impl SerializedGossipMessage {
    fn new(from: SocketAddr, to: SocketAddr, message: &ChitchatMessage) -> Self {
        Self {
            from,
            to,
            payload: message.serialize_to_vec(),
            message_type: GossipMessageType::from_message(message),
        }
    }

    fn decode(&self) -> ChitchatMessage {
        let mut bytes = self.payload.as_slice();
        ChitchatMessage::deserialize(&mut bytes).expect("serialized chitchat message should decode")
    }
}

fn clone_chitchat_message(message: &ChitchatMessage) -> ChitchatMessage {
    let serialized = message.serialize_to_vec();
    let mut bytes = serialized.as_slice();
    ChitchatMessage::deserialize(&mut bytes).expect("serialized chitchat message should decode")
}

fn message_contains_prefix(message: &ChitchatMessage, prefix: &[u8]) -> bool {
    if prefix.is_empty() {
        return true;
    }
    if message
        .serialize_to_vec()
        .windows(prefix.len())
        .any(|window| window == prefix)
    {
        return true;
    }
    format!("{message:?}")
        .as_bytes()
        .windows(prefix.len())
        .any(|window| window == prefix)
}

/// Test-only predicate over one outbound gossip message.
pub trait GossipMessageFilter: Send + Sync {
    /// Decide how one message should be handled at the current logical tick.
    fn filter(
        &self,
        tick: u64,
        from: SocketAddr,
        to: SocketAddr,
        message: &ChitchatMessage,
    ) -> GossipFilterAction;
}

/// A TiKV-style gossip packet filter with optional endpoint, type, and key selectors.
#[derive(Debug)]
pub struct GossipPacketFilter {
    node: Option<SocketAddr>,
    peer: Option<SocketAddr>,
    direction: GossipFilterDirection,
    message_type: Option<GossipMessageType>,
    key_prefix: Option<Vec<u8>>,
    allow_remaining: AtomicU64,
    action: GossipFilterAction,
}

impl GossipPacketFilter {
    /// Drop every message from `from` to `to`.
    pub fn drop_between(from: SocketAddr, to: SocketAddr) -> Self {
        Self::new()
            .from(from)
            .to(to)
            .action(GossipFilterAction::Drop)
    }

    /// Start a pass-through filter builder.
    pub fn new() -> Self {
        Self {
            node: None,
            peer: None,
            direction: GossipFilterDirection::Send,
            message_type: None,
            key_prefix: None,
            allow_remaining: AtomicU64::new(0),
            action: GossipFilterAction::Pass,
        }
    }

    /// Match messages sent by this gossip address.
    pub fn from(mut self, node: SocketAddr) -> Self {
        self.node = Some(node);
        self.direction = GossipFilterDirection::Send;
        self
    }

    /// Match messages received by this gossip address.
    pub fn recv(mut self, node: SocketAddr) -> Self {
        self.node = Some(node);
        self.direction = GossipFilterDirection::Recv;
        self
    }

    /// Match messages between the selected node and this peer.
    pub fn to(mut self, peer: SocketAddr) -> Self {
        self.peer = Some(peer);
        self
    }

    /// Match both send and receive directions for the selected address/peer pair.
    pub fn both_directions(mut self) -> Self {
        self.direction = GossipFilterDirection::Both;
        self
    }

    /// Match only a specific chitchat message type.
    pub fn message_type(mut self, message_type: GossipMessageType) -> Self {
        self.message_type = Some(message_type);
        self
    }

    /// Match serialized gossip payloads that contain this key prefix.
    pub fn key_prefix(mut self, prefix: impl Into<Vec<u8>>) -> Self {
        self.key_prefix = Some(prefix.into());
        self
    }

    /// Allow the first `count` matching messages before applying the action.
    pub fn allow(mut self, count: u64) -> Self {
        self.allow_remaining = AtomicU64::new(count);
        self
    }

    /// Set the action for matching messages.
    pub fn action(mut self, action: GossipFilterAction) -> Self {
        self.action = action;
        self
    }

    fn matches(&self, from: SocketAddr, to: SocketAddr, message: &ChitchatMessage) -> bool {
        if let Some(message_type) = self.message_type {
            if GossipMessageType::from_message(message) != message_type {
                return false;
            }
        }
        if let Some(prefix) = &self.key_prefix {
            if !message_contains_prefix(message, prefix) {
                return false;
            }
        }

        let Some(node) = self.node else {
            return true;
        };
        let peer_matches = |peer: SocketAddr| self.peer.is_none_or(|expected| expected == peer);
        match self.direction {
            GossipFilterDirection::Send => from == node && peer_matches(to),
            GossipFilterDirection::Recv => to == node && peer_matches(from),
            GossipFilterDirection::Both => {
                (from == node && peer_matches(to)) || (to == node && peer_matches(from))
            }
        }
    }
}

impl Default for GossipPacketFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl GossipMessageFilter for GossipPacketFilter {
    fn filter(
        &self,
        _tick: u64,
        from: SocketAddr,
        to: SocketAddr,
        message: &ChitchatMessage,
    ) -> GossipFilterAction {
        if !self.matches(from, to, message) {
            return GossipFilterAction::Pass;
        }
        let mut remaining = self.allow_remaining.load(Ordering::SeqCst);
        while remaining > 0 {
            match self.allow_remaining.compare_exchange(
                remaining,
                remaining - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return GossipFilterAction::Pass,
                Err(actual) => remaining = actual,
            }
        }
        self.action
    }
}

/// Shared deterministic gossip filter state.
#[derive(Clone, Default)]
pub struct GossipFilterSet {
    filters: Arc<RwLock<Vec<Arc<dyn GossipMessageFilter>>>>,
    delayed: Arc<Mutex<BTreeMap<u64, Vec<SerializedGossipMessage>>>>,
    dropped: Arc<Mutex<Vec<SerializedGossipMessage>>>,
    trace: Arc<Mutex<Vec<GossipMessageTraceEvent>>>,
    tick: Arc<AtomicU64>,
}

impl GossipFilterSet {
    /// Add one filter.
    pub fn add_filter(&self, filter: impl GossipMessageFilter + 'static) {
        self.filters
            .write()
            .expect("gossip filter set poisoned")
            .push(Arc::new(filter));
    }

    /// Drop both directions between two gossip addresses.
    pub fn cut(&self, left: SocketAddr, right: SocketAddr) {
        self.add_filter(GossipPacketFilter::drop_between(left, right));
        self.add_filter(GossipPacketFilter::drop_between(right, left));
    }

    /// Drop every message to and from one gossip address and its known peers.
    pub fn isolate(&self, node: SocketAddr, peers: impl IntoIterator<Item = SocketAddr>) {
        for peer in peers {
            if peer != node {
                self.cut(node, peer);
            }
        }
    }

    /// Clear filters and delayed messages.
    pub fn recover(&self) {
        self.filters
            .write()
            .expect("gossip filter set poisoned")
            .clear();
        self.delayed
            .lock()
            .expect("gossip delayed queue poisoned")
            .clear();
    }

    /// Return the deterministic trace.
    pub fn trace(&self) -> Vec<GossipMessageTraceEvent> {
        self.trace
            .lock()
            .expect("gossip trace queue poisoned")
            .clone()
    }

    /// Return the number of dropped gossip messages.
    pub fn dropped_count(&self) -> usize {
        self.dropped
            .lock()
            .expect("gossip dropped queue poisoned")
            .len()
    }

    fn apply(
        &self,
        from: SocketAddr,
        to: SocketAddr,
        message: ChitchatMessage,
    ) -> Vec<(SocketAddr, ChitchatMessage)> {
        let tick = self.tick.load(Ordering::SeqCst);
        let action = self
            .filters
            .read()
            .expect("gossip filter set poisoned")
            .iter()
            .map(|filter| filter.filter(tick, from, to, &message))
            .find(|action| *action != GossipFilterAction::Pass)
            .unwrap_or(GossipFilterAction::Pass);
        self.apply_action(tick, from, to, message, action)
    }

    fn advance_tick_for(&self, from: SocketAddr) -> Vec<(SocketAddr, ChitchatMessage)> {
        let tick = self.tick.fetch_add(1, Ordering::SeqCst) + 1;
        let mut delayed = self.delayed.lock().expect("gossip delayed queue poisoned");
        let due_ticks = delayed
            .range(..=tick)
            .map(|(due_tick, _)| *due_tick)
            .collect::<Vec<_>>();
        let mut deliverable = Vec::new();
        for due_tick in due_ticks {
            let Some(messages) = delayed.remove(&due_tick) else {
                continue;
            };
            let mut retained = Vec::new();
            for message in messages {
                if message.from == from {
                    self.record_serialized(tick, &message, "delivered");
                    deliverable.push((message.to, message.decode()));
                } else {
                    retained.push(message);
                }
            }
            if !retained.is_empty() {
                delayed.insert(due_tick, retained);
            }
        }
        deliverable
    }

    fn apply_action(
        &self,
        tick: u64,
        from: SocketAddr,
        to: SocketAddr,
        message: ChitchatMessage,
        action: GossipFilterAction,
    ) -> Vec<(SocketAddr, ChitchatMessage)> {
        match action {
            GossipFilterAction::Pass => {
                self.record(tick, from, to, &message, "delivered");
                vec![(to, message)]
            }
            GossipFilterAction::Drop => {
                self.record(tick, from, to, &message, "dropped");
                self.dropped
                    .lock()
                    .expect("gossip dropped queue poisoned")
                    .push(SerializedGossipMessage::new(from, to, &message));
                Vec::new()
            }
            GossipFilterAction::Delay(delay) => {
                self.record(tick, from, to, &message, "delayed");
                self.delayed
                    .lock()
                    .expect("gossip delayed queue poisoned")
                    .entry(tick.saturating_add(delay.max(1)))
                    .or_default()
                    .push(SerializedGossipMessage::new(from, to, &message));
                Vec::new()
            }
            GossipFilterAction::Duplicate(extra) => {
                self.record(tick, from, to, &message, "delivered");
                let mut messages = Vec::with_capacity(extra.saturating_add(1));
                messages.push((to, clone_chitchat_message(&message)));
                for _ in 0..extra {
                    self.record(tick, from, to, &message, "duplicated");
                    messages.push((to, clone_chitchat_message(&message)));
                }
                messages
            }
        }
    }

    fn record(
        &self,
        tick: u64,
        from: SocketAddr,
        to: SocketAddr,
        message: &ChitchatMessage,
        action: &'static str,
    ) {
        self.trace
            .lock()
            .expect("gossip trace queue poisoned")
            .push(GossipMessageTraceEvent {
                tick,
                from,
                to,
                message_type: GossipMessageType::from_message(message),
                action,
            });
    }

    fn record_serialized(
        &self,
        tick: u64,
        message: &SerializedGossipMessage,
        action: &'static str,
    ) {
        self.trace
            .lock()
            .expect("gossip trace queue poisoned")
            .push(GossipMessageTraceEvent {
                tick,
                from: message.from,
                to: message.to,
                message_type: message.message_type,
                action,
            });
    }
}

/// Chitchat `Transport` decorator backed by [`GossipFilterSet`].
pub struct FilteredChitchatTransport {
    inner: ChannelTransport,
    filters: GossipFilterSet,
}

impl FilteredChitchatTransport {
    /// Create a filterable in-process chitchat transport.
    pub fn new(inner: ChannelTransport, filters: GossipFilterSet) -> Self {
        Self { inner, filters }
    }

    /// Create a filterable transport backed by `ChannelTransport::default()`.
    pub fn channel(filters: GossipFilterSet) -> Self {
        Self::new(ChannelTransport::default(), filters)
    }

    /// Return the shared filter set.
    pub fn filters(&self) -> GossipFilterSet {
        self.filters.clone()
    }
}

impl Default for FilteredChitchatTransport {
    fn default() -> Self {
        Self::channel(GossipFilterSet::default())
    }
}

#[async_trait::async_trait]
impl Transport for FilteredChitchatTransport {
    async fn open(&self, listen_addr: SocketAddr) -> anyhow::Result<Box<dyn Socket>> {
        let inner = self.inner.open(listen_addr).await?;
        Ok(Box::new(FilteredChitchatSocket {
            listen_addr,
            inner,
            filters: self.filters.clone(),
        }))
    }
}

struct FilteredChitchatSocket {
    listen_addr: SocketAddr,
    inner: Box<dyn Socket>,
    filters: GossipFilterSet,
}

#[async_trait::async_trait]
impl Socket for FilteredChitchatSocket {
    async fn send(&mut self, to: SocketAddr, message: ChitchatMessage) -> anyhow::Result<()> {
        for (to, message) in self.filters.advance_tick_for(self.listen_addr) {
            self.inner.send(to, message).await?;
        }
        for (to, message) in self.filters.apply(self.listen_addr, to, message) {
            self.inner.send(to, message).await?;
        }
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<(SocketAddr, ChitchatMessage)> {
        self.inner.recv().await
    }
}

/// One deterministic liveness transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipLivenessStep {
    /// Affected HydraCache node id.
    pub node_id: ClusterNodeId,
    /// Liveness state to publish.
    pub state: ClusterDiscoveryLiveness,
}

/// Deterministic trace emitted by [`GossipLivenessScript`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipLivenessTraceEvent {
    /// Logical liveness-script tick.
    pub tick: u64,
    /// Affected HydraCache node id.
    pub node_id: ClusterNodeId,
    /// Published liveness state.
    pub state: ClusterDiscoveryLiveness,
}

/// Scripted discovery liveness changes without wall-clock sleeps.
pub struct GossipLivenessScript {
    steps: VecDeque<GossipLivenessStep>,
    trace: Vec<GossipLivenessTraceEvent>,
    tick: u64,
}

impl GossipLivenessScript {
    /// Build a script from explicit steps.
    pub fn new(steps: impl IntoIterator<Item = GossipLivenessStep>) -> Self {
        Self {
            steps: VecDeque::from_iter(steps),
            trace: Vec::new(),
            tick: 0,
        }
    }

    /// Build a Live -> Suspect -> Dead -> Live flap for one node.
    pub fn live_suspect_dead_live(node_id: impl Into<ClusterNodeId>) -> Self {
        let node_id = node_id.into();
        Self::new([
            GossipLivenessStep {
                node_id: node_id.clone(),
                state: ClusterDiscoveryLiveness::Live,
            },
            GossipLivenessStep {
                node_id: node_id.clone(),
                state: ClusterDiscoveryLiveness::Suspect,
            },
            GossipLivenessStep {
                node_id: node_id.clone(),
                state: ClusterDiscoveryLiveness::Dead,
            },
            GossipLivenessStep {
                node_id,
                state: ClusterDiscoveryLiveness::Live,
            },
        ])
    }

    /// Apply the next liveness step. Returns `false` when the script is empty.
    pub async fn apply_next<D>(&mut self, discovery: &D) -> CacheResult<bool>
    where
        D: ClusterDiscovery + ?Sized,
    {
        let Some(step) = self.steps.pop_front() else {
            return Ok(false);
        };
        self.tick = self.tick.saturating_add(1);
        match step.state {
            ClusterDiscoveryLiveness::Live => discovery.mark_live(step.node_id.clone()).await?,
            ClusterDiscoveryLiveness::Suspect => {
                discovery.mark_suspect(step.node_id.clone()).await?
            }
            ClusterDiscoveryLiveness::Dead => discovery.mark_dead(step.node_id.clone()).await?,
        }
        self.trace.push(GossipLivenessTraceEvent {
            tick: self.tick,
            node_id: step.node_id,
            state: step.state,
        });
        Ok(true)
    }

    /// Apply every remaining liveness step.
    pub async fn apply_all<D>(&mut self, discovery: &D) -> CacheResult<()>
    where
        D: ClusterDiscovery + ?Sized,
    {
        while self.apply_next(discovery).await? {}
        Ok(())
    }

    /// Return the deterministic liveness trace.
    pub fn trace(&self) -> &[GossipLivenessTraceEvent] {
        &self.trace
    }
}

/// Deterministic in-process raft runtime cluster used by PR-tier tests.
pub struct RuntimeRaftCluster {
    nodes: BTreeMap<u64, Arc<RaftMetadataRuntime>>,
    configs: BTreeMap<u64, RaftMetadataRuntimeConfig>,
    stores: BTreeMap<u64, InMemoryRaftLogStore>,
    filters: RaftFilterSet,
    delivered: Vec<RaftWireMessage>,
}

impl RuntimeRaftCluster {
    /// Create a three-node cluster.
    pub fn three_node() -> Self {
        Self::with_voters([1, 2, 3])
    }

    /// Create a cluster with explicit voter ids.
    pub fn with_voters<I>(voters: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        let voters = voters.into_iter().collect::<Vec<_>>();
        Self::with_voters_and_prevote(voters, BTreeMap::new())
    }

    /// Create a cluster where selected nodes override the default pre-vote setting.
    pub fn with_prevote_overrides<I>(voters: I, overrides: BTreeMap<u64, bool>) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        let voters = voters.into_iter().collect::<Vec<_>>();
        Self::with_voters_and_prevote(voters, overrides)
    }

    fn with_voters_and_prevote(voters: Vec<u64>, overrides: BTreeMap<u64, bool>) -> Self {
        let mut nodes = BTreeMap::new();
        let mut configs = BTreeMap::new();
        let mut stores = BTreeMap::new();
        for id in voters.iter().copied() {
            let pre_vote = overrides.get(&id).copied().unwrap_or(true);
            let config = RaftMetadataRuntimeConfig::multi_voter("orders", id, voters.clone())
                .pre_vote(pre_vote)
                .ticks(5, 1);
            let store = InMemoryRaftLogStore::new_with_conf_state((voters.clone(), vec![]));
            let runtime = RaftMetadataRuntime::with_storage(config.clone(), store.clone()).unwrap();
            nodes.insert(id, Arc::new(runtime));
            configs.insert(id, config);
            stores.insert(id, store);
        }
        Self {
            nodes,
            configs,
            stores,
            filters: RaftFilterSet::default(),
            delivered: Vec::new(),
        }
    }

    /// Restart one runtime over its existing in-memory raft log store.
    pub fn restart_node(&mut self, node_id: u64) -> CacheResult<()> {
        let config = self
            .configs
            .get(&node_id)
            .cloned()
            .ok_or_else(|| CacheError::Backend(format!("unknown raft node {node_id}")))?;
        let store =
            self.stores.get(&node_id).cloned().ok_or_else(|| {
                CacheError::Backend(format!("missing raft store for node {node_id}"))
            })?;
        let runtime = RaftMetadataRuntime::with_storage(config, store)?;
        self.nodes.insert(node_id, Arc::new(runtime));
        Ok(())
    }

    /// Persist a raft snapshot for one node using its current applied index and conf state.
    pub fn save_snapshot_for_node(&self, node_id: u64) -> CacheResult<u64> {
        let runtime = self.node(node_id);
        let runtime_snapshot = runtime.snapshot();
        let store = self
            .stores
            .get(&node_id)
            .ok_or_else(|| CacheError::Backend(format!("missing raft store for node {node_id}")))?;
        let mut snapshot = Snapshot::default();
        snapshot.mut_metadata().index = runtime_snapshot.applied_index;
        snapshot.mut_metadata().term = runtime_snapshot.term;
        snapshot
            .mut_metadata()
            .mut_conf_state()
            .set_voters(runtime.voter_ids()?);
        store
            .save_snapshot(&snapshot, usize::MAX)
            .map_err(|error| CacheError::Backend(error.to_string()))?;
        Ok(runtime_snapshot.applied_index)
    }

    /// Persist a metadata-bearing snapshot and compact one node's applied log.
    ///
    /// This mirrors the runtime snapshot payload contract from a dev-only crate,
    /// allowing default-feature integration tests to exercise real raft-rs
    /// `MsgSnapshot` delivery without exposing a production compaction hook.
    pub fn compact_applied_log_to_snapshot(&self, node_id: u64) -> CacheResult<u64> {
        const PAYLOAD_MAGIC: &[u8; 8] = b"HCMETA01";
        const PAYLOAD_VERSION: u32 = 1;

        let runtime = self.node(node_id);
        let export = runtime.export_snapshot();
        if export.applied_index == 0 {
            return Err(CacheError::Backend(
                "cannot compact raft metadata log before any entry is applied".to_owned(),
            ));
        }
        let store = self
            .stores
            .get(&node_id)
            .ok_or_else(|| CacheError::Backend(format!("missing raft store for node {node_id}")))?;
        let term = store
            .term(export.applied_index)
            .unwrap_or(runtime.snapshot().term);
        let conf_state = store
            .initial_state()
            .map_err(|error| CacheError::Backend(error.to_string()))?
            .conf_state;
        let payload = serde_json::json!({
            "format_version": PAYLOAD_VERSION,
            "cluster_name": export.cluster_name,
            "source_raft_node_id": export.raft_node_id,
            "applied_index": export.applied_index,
            "commands": export.commands,
        });
        let mut data = PAYLOAD_MAGIC.to_vec();
        data.extend(
            serde_json::to_vec(&payload).map_err(|error| CacheError::Backend(error.to_string()))?,
        );

        let mut snapshot = Snapshot::default();
        snapshot.mut_metadata().index = export.applied_index;
        snapshot.mut_metadata().term = term;
        snapshot.mut_metadata().set_conf_state(conf_state);
        snapshot.data = data.into();
        store
            .save_snapshot(&snapshot, usize::MAX)
            .map_err(|error| CacheError::Backend(error.to_string()))?;
        Ok(export.applied_index)
    }

    /// Return the shared filter set.
    pub fn filters(&self) -> RaftFilterSet {
        self.filters.clone()
    }

    /// Return node ids in deterministic order.
    pub fn node_ids(&self) -> Vec<u64> {
        self.nodes.keys().copied().collect()
    }

    /// Return one runtime.
    pub fn node(&self, node_id: u64) -> Arc<RaftMetadataRuntime> {
        self.nodes.get(&node_id).expect("known raft node").clone()
    }

    /// Return the current leader, if any.
    pub fn leader_id(&self) -> Option<u64> {
        self.nodes.iter().find_map(|(node_id, node)| {
            (node.snapshot().role == RaftRuntimeRole::Leader).then_some(*node_id)
        })
    }

    /// Return all delivered messages.
    pub fn delivered(&self) -> &[RaftWireMessage] {
        &self.delivered
    }

    /// Campaign one node and drain the resulting network.
    pub fn campaign(&mut self, node_id: u64) {
        let messages = self.node(node_id).campaign().unwrap();
        self.drain_until_idle(messages);
    }

    /// Request a real raft-rs leadership transfer through the deterministic network.
    pub fn request_leadership_transfer(
        &mut self,
        leader_id: u64,
        transferee_id: u64,
    ) -> CacheResult<()> {
        if !self.nodes.contains_key(&transferee_id) {
            return Err(CacheError::Backend(format!(
                "unknown leadership transferee {transferee_id}"
            )));
        }
        if !self.node(leader_id).voter_ids()?.contains(&transferee_id) {
            return Err(CacheError::Backend(format!(
                "ineligible non-voter leadership transferee {transferee_id}"
            )));
        }
        let mut message = RaftMessage::default();
        message.from = transferee_id;
        message.to = leader_id;
        message.set_msg_type(MessageType::MsgTransferLeader);
        self.drain_until_idle([RaftWireMessage::encode(&message)?]);
        Ok(())
    }

    /// Report a snapshot transport outcome and drain any immediate retry.
    pub fn report_snapshot_delivery(
        &mut self,
        leader_id: u64,
        follower_id: u64,
        delivered: bool,
    ) -> CacheResult<()> {
        let messages = self
            .node(leader_id)
            .report_snapshot_delivery(follower_id, delivered)?;
        self.drain_until_idle(messages);
        Ok(())
    }

    /// Tick one node and drain the resulting network.
    pub fn tick_node(&mut self, node_id: u64) {
        let messages = self.node(node_id).tick().unwrap();
        self.drain_until_idle(messages);
    }

    /// Tick every node `count` times.
    pub fn tick_all(&mut self, count: usize) {
        for _ in 0..count {
            for node_id in self.node_ids() {
                self.tick_node(node_id);
            }
        }
    }

    /// Propose adding a voter and drain the resulting network.
    pub fn propose_add_voter(&mut self, leader_id: u64, voter_id: u64) -> CacheResult<()> {
        let messages = self.node(leader_id).propose_add_voter(voter_id)?;
        self.drain_until_idle(messages);
        Ok(())
    }

    /// Propose removing a voter and drain the resulting network.
    pub fn propose_remove_voter(&mut self, leader_id: u64, voter_id: u64) -> CacheResult<()> {
        let messages = self.node(leader_id).propose_remove_voter(voter_id)?;
        self.drain_until_idle(messages);
        Ok(())
    }

    /// Join a member through the metadata runtime and drain until the proposal applies.
    pub async fn join_member(&mut self, leader_id: u64, node_id: &str) -> CacheResult<()> {
        let leader = self.node(leader_id);
        let join = tokio::spawn({
            let leader = leader.clone();
            let node_id = node_id.to_owned();
            async move {
                leader
                    .join_member(
                        hydracache::ClusterCandidate::member(node_id)
                            .generation(ClusterGeneration::new(1)),
                    )
                    .await
            }
        });
        for _ in 0..200 {
            self.drain_until_idle(leader.take_outbound_messages());
            if leader.command_applied(&format!("member-upsert:{node_id}:1")) {
                break;
            }
            tokio::task::yield_now().await;
        }
        join.await.expect("join task should not panic")?;
        Ok(())
    }

    /// Drain a batch and any recursively produced messages.
    pub fn drain_until_idle<I>(&mut self, messages: I)
    where
        I: IntoIterator<Item = RaftWireMessage>,
    {
        let mut queue = VecDeque::from_iter(messages);
        for _ in 0..1_000 {
            while let Some(message) = queue.pop_front() {
                for deliverable in self.filters.apply(message) {
                    queue.extend(self.deliver_now(deliverable));
                }
            }
            for delayed in self.filters.advance_tick() {
                queue.extend(self.deliver_now(delayed));
            }
            if !queue.is_empty() {
                continue;
            }
            let newly_ready = self.drain_all_ready();
            if queue.is_empty() && newly_ready.is_empty() {
                return;
            }
            queue.extend(newly_ready);
        }
        panic!("runtime raft harness did not become idle");
    }

    fn drain_all_ready(&self) -> Vec<RaftWireMessage> {
        self.nodes
            .values()
            .flat_map(|node| node.drain_ready().unwrap())
            .collect()
    }

    fn deliver_now(&mut self, message: RaftWireMessage) -> Vec<RaftWireMessage> {
        let Some(node) = self.nodes.get(&message.to).cloned() else {
            return Vec::new();
        };
        self.delivered.push(message.clone());
        match node.step(message) {
            Ok(messages) => messages,
            Err(error) if error.to_string().contains("peer not found") => Vec::new(),
            Err(error) => panic!("runtime raft harness failed to deliver message: {error}"),
        }
    }
}
