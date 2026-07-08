//! Deterministic cluster-correctness harnesses shared by HydraCache tests.
//!
//! This crate is intentionally `publish = false` and is consumed only from
//! dev-dependencies. It owns the fault-injection vocabulary so production crates
//! do not grow test-only transport types.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use hydracache::{CacheResult, ClusterControlPlane, ClusterGeneration};
use hydracache_cluster_raft::{
    RaftMessageSink, RaftMetadataRuntime, RaftMetadataRuntimeConfig, RaftRuntimeRole,
    RaftWireMessage,
};
use raft::eraftpb::MessageType;

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

/// Deterministic in-process raft runtime cluster used by PR-tier tests.
pub struct RuntimeRaftCluster {
    nodes: BTreeMap<u64, Arc<RaftMetadataRuntime>>,
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
        let nodes = voters
            .iter()
            .copied()
            .map(|id| {
                let config = RaftMetadataRuntimeConfig::multi_voter("orders", id, voters.clone())
                    .ticks(5, 1);
                (
                    id,
                    Arc::new(RaftMetadataRuntime::with_config(config).unwrap()),
                )
            })
            .collect();
        Self {
            nodes,
            filters: RaftFilterSet::default(),
            delivered: Vec::new(),
        }
    }

    /// Create a cluster where selected nodes override the default pre-vote setting.
    pub fn with_prevote_overrides<I>(voters: I, overrides: BTreeMap<u64, bool>) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        let voters = voters.into_iter().collect::<Vec<_>>();
        let nodes = voters
            .iter()
            .copied()
            .map(|id| {
                let pre_vote = overrides.get(&id).copied().unwrap_or(true);
                let config = RaftMetadataRuntimeConfig::multi_voter("orders", id, voters.clone())
                    .pre_vote(pre_vote)
                    .ticks(5, 1);
                (
                    id,
                    Arc::new(RaftMetadataRuntime::with_config(config).unwrap()),
                )
            })
            .collect();
        Self {
            nodes,
            filters: RaftFilterSet::default(),
            delivered: Vec::new(),
        }
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
