use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hydracache::ClusterNodeId;
use hydracache_cluster_raft::{InMemoryRaftLogStore, RaftLogStore, RaftWireMessage};
use protobuf::Message as ProtobufMessage;
use raft::eraftpb::{ConfChange, ConfChangeType, ConfChangeV2, Entry, EntryType, Message};
use raft::{Config, Error as RaftError, RawNode, StateRole};
use slog::{o, Discard, Logger};

use crate::{
    ElectionDriverSnapshot, ElectionNodeState, ElectionSignal, ElectionSignalKind, ElectionSource,
    FormationPhase, NodeFsmState, SimNetwork,
};

const DEFAULT_ELECTION_TICK: usize = 10;
const DEFAULT_HEARTBEAT_TICK: usize = 3;
const MAX_RAFT_INFLIGHT: usize = 4096;

/// Result type returned by the deterministic raft lab harness.
pub type SimRaftResult<T> = Result<T, SimRaftError>;

/// Error returned by the deterministic raft lab harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimRaftError {
    message: String,
}

impl SimRaftError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SimRaftError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SimRaftError {}

impl From<RaftError> for SimRaftError {
    fn from(error: RaftError) -> Self {
        Self::new(format!("raft error: {error}"))
    }
}

impl From<hydracache_cluster_raft::RaftStoreError> for SimRaftError {
    fn from(error: hydracache_cluster_raft::RaftStoreError) -> Self {
        Self::new(format!("raft store error: {error}"))
    }
}

impl From<hydracache::CacheError> for SimRaftError {
    fn from(error: hydracache::CacheError) -> Self {
        Self::new(format!("raft wire error: {error}"))
    }
}

impl From<protobuf::ProtobufError> for SimRaftError {
    fn from(error: protobuf::ProtobufError) -> Self {
        Self::new(format!("protobuf error: {error}"))
    }
}

struct SimRaftNode {
    raw: RawNode<InMemoryRaftLogStore>,
    applied_index: u64,
}

/// Stable key for raft messages currently riding the simulator network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimRaftInFlightKey {
    /// Logical simulator step at which delivery is attempted.
    pub deliver_at_step: u64,
    /// Source raft node id.
    pub from: u64,
    /// Destination raft node id.
    pub to: u64,
    /// Stable route sequence.
    pub seq: u64,
}

/// Deterministic, synchronous raft-rs election harness used by the lab.
pub struct SimRaftCluster {
    seed: u64,
    election_tick: usize,
    heartbeat_tick: usize,
    nodes: BTreeMap<u64, SimRaftNode>,
    names: BTreeMap<u64, ClusterNodeId>,
    ids: BTreeMap<ClusterNodeId, u64>,
    inflight: BTreeMap<SimRaftInFlightKey, RaftWireMessage>,
    last_live: BTreeSet<ClusterNodeId>,
    next_raft_id: u64,
    next_seq: u64,
    dropped_messages: u64,
    trace: Vec<String>,
    logger: Logger,
}

impl fmt::Debug for SimRaftCluster {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimRaftCluster")
            .field("seed", &self.seed)
            .field("election_tick", &self.election_tick)
            .field("heartbeat_tick", &self.heartbeat_tick)
            .field("nodes", &self.names)
            .field("inflight", &self.inflight.keys().collect::<Vec<_>>())
            .field("last_live", &self.last_live)
            .field("next_raft_id", &self.next_raft_id)
            .field("next_seq", &self.next_seq)
            .field("dropped_messages", &self.dropped_messages)
            .finish()
    }
}

impl SimRaftCluster {
    /// Build a deterministic real-raft cluster over the provided node names.
    pub fn new(
        seed: u64,
        node_ids: impl IntoIterator<Item = ClusterNodeId>,
    ) -> SimRaftResult<Self> {
        let logger = Logger::root(Discard, o!());
        let mut names_by_id = BTreeMap::new();
        let mut ids = BTreeMap::new();
        let ordered_names = node_ids.into_iter().collect::<BTreeSet<_>>();
        for (index, node_id) in ordered_names.iter().enumerate() {
            let raft_id = index as u64 + 1;
            names_by_id.insert(raft_id, node_id.clone());
            ids.insert(node_id.clone(), raft_id);
        }
        let peers = names_by_id.keys().copied().collect::<Vec<_>>();
        let mut nodes = BTreeMap::new();
        for raft_id in names_by_id.keys().copied().collect::<Vec<_>>() {
            nodes.insert(
                raft_id,
                SimRaftNode {
                    raw: build_raw_node(
                        raft_id,
                        &peers,
                        DEFAULT_ELECTION_TICK,
                        DEFAULT_HEARTBEAT_TICK,
                        &logger,
                    )?,
                    applied_index: 0,
                },
            );
        }
        let mut cluster = Self {
            seed,
            election_tick: DEFAULT_ELECTION_TICK,
            heartbeat_tick: DEFAULT_HEARTBEAT_TICK,
            nodes,
            names: names_by_id,
            ids,
            inflight: BTreeMap::new(),
            last_live: ordered_names,
            next_raft_id: peers.len() as u64 + 1,
            next_seq: 1,
            dropped_messages: 0,
            trace: Vec::new(),
            logger,
        };
        cluster.seed_all_timeouts();
        Ok(cluster)
    }

    /// Advance raft by one deterministic simulator step.
    pub fn step(
        &mut self,
        now_step: u64,
        live: &BTreeSet<ClusterNodeId>,
        network: &SimNetwork,
    ) -> SimRaftResult<()> {
        self.last_live = live.clone();
        self.deliver_due(now_step, live, network)?;
        for raft_id in self.live_raft_ids(live) {
            if let Some(node) = self.nodes.get_mut(&raft_id) {
                seed_election_timeout(&mut node.raw, self.seed, raft_id);
                node.raw.tick();
            }
        }
        self.drain_ready_until_idle(now_step, live, network)?;
        Ok(())
    }

    /// Add or re-add one voting node via a real raft conf change.
    pub fn add_node(&mut self, name: ClusterNodeId, now_step: u64) -> SimRaftResult<()> {
        let raft_id = if let Some(raft_id) = self.ids.get(&name).copied() {
            raft_id
        } else {
            let raft_id = self.next_raft_id;
            self.next_raft_id = self.next_raft_id.saturating_add(1);
            self.ids.insert(name.clone(), raft_id);
            self.names.insert(raft_id, name.clone());
            raft_id
        };
        if !self.nodes.contains_key(&raft_id) {
            let peers = self.current_voters();
            self.nodes.insert(
                raft_id,
                SimRaftNode {
                    raw: build_raw_node(
                        raft_id,
                        &peers,
                        self.election_tick,
                        self.heartbeat_tick,
                        &self.logger,
                    )?,
                    applied_index: 0,
                },
            );
        }
        if self.current_voters().contains(&raft_id) {
            return Ok(());
        }
        self.propose_conf_change(raft_id, ConfChangeType::AddNode, now_step)
    }

    /// Remove one voting node via a real raft conf change.
    pub fn remove_node(&mut self, name: &ClusterNodeId, now_step: u64) -> SimRaftResult<()> {
        let Some(raft_id) = self.ids.get(name).copied() else {
            return Ok(());
        };
        if !self.current_voters().contains(&raft_id) {
            return Ok(());
        }
        self.propose_conf_change(raft_id, ConfChangeType::RemoveNode, now_step)
    }

    /// Mark a node as available again. Temporary crashes/isolation keep raft membership.
    pub fn restore_node(&mut self, _name: &ClusterNodeId) {}

    /// Current live leader, if raft has elected one.
    pub fn leader(&self) -> Option<ClusterNodeId> {
        self.nodes
            .iter()
            .filter(|(raft_id, _)| self.is_live_raft_id(**raft_id))
            .find(|(_, node)| node.raw.raft.state == StateRole::Leader)
            .and_then(|(raft_id, _)| self.names.get(raft_id).cloned())
    }

    /// Current term, using the live leader term when present or the max live term.
    pub fn term(&self) -> u64 {
        if let Some(leader) = self.leader().and_then(|name| self.ids.get(&name).copied()) {
            return self
                .nodes
                .get(&leader)
                .map(|node| node.raw.raft.term)
                .unwrap_or_default();
        }
        self.nodes
            .iter()
            .filter(|(raft_id, _)| self.is_live_raft_id(**raft_id))
            .map(|(_, node)| node.raw.raft.term)
            .max()
            .unwrap_or_default()
    }

    /// Stable snapshot compatible with the existing election model snapshot.
    pub fn snapshot(&self) -> ElectionDriverSnapshot {
        let leader = self.leader();
        let phase = if leader.is_some() {
            FormationPhase::Formed
        } else if self.last_live.len() < self.quorum() {
            FormationPhase::Degraded
        } else if self.term() > 0 {
            FormationPhase::Electing
        } else {
            FormationPhase::Bootstrapping
        };
        ElectionDriverSnapshot {
            source: ElectionSource::Raft,
            phase,
            term: self.term(),
            leader,
            nodes: self
                .names
                .values()
                .filter_map(|name| self.node_state(name).map(|state| (name, state)))
                .map(
                    |(name, (state, term, voted_for, votes_received))| ElectionNodeState {
                        node_id: name.clone(),
                        state,
                        term,
                        voted_for,
                        votes_received,
                    },
                )
                .collect(),
            trace: self.trace.clone(),
            signals: self.signals(),
        }
    }

    /// Per-node raft election state used by snapshots and tests.
    pub fn node_state(
        &self,
        name: &ClusterNodeId,
    ) -> Option<(NodeFsmState, u64, Option<ClusterNodeId>, usize)> {
        let raft_id = self.ids.get(name).copied()?;
        let node = self.nodes.get(&raft_id)?;
        if !self.is_live_raft_id(raft_id) {
            return Some((
                NodeFsmState::Disconnected,
                node.raw.raft.term,
                vote_name(node.raw.raft.vote, &self.names),
                0,
            ));
        }
        let state = match node.raw.raft.state {
            StateRole::Follower => NodeFsmState::Follower,
            StateRole::Candidate | StateRole::PreCandidate => NodeFsmState::Candidate,
            StateRole::Leader => NodeFsmState::Leader,
        };
        let votes_received = match state {
            NodeFsmState::Leader => self.voter_count_for(raft_id),
            NodeFsmState::Candidate => node.raw.raft.prs().tally_votes().0,
            _ => 0,
        };
        Some((
            state,
            node.raw.raft.term,
            vote_name(node.raw.raft.vote, &self.names),
            votes_received,
        ))
    }

    /// Stable debug view of in-flight raft message order.
    pub fn inflight_order(&self) -> Vec<(u64, u64, u64, u64)> {
        self.inflight
            .keys()
            .map(|key| (key.deliver_at_step, key.from, key.to, key.seq))
            .collect()
    }

    /// Number of raft messages dropped by simulated topology gates.
    pub fn dropped_messages(&self) -> u64 {
        self.dropped_messages
    }

    /// Stable trace emitted by the raft harness.
    pub fn trace(&self) -> &[String] {
        &self.trace
    }

    fn propose_conf_change(
        &mut self,
        node_id: u64,
        change_type: ConfChangeType,
        now_step: u64,
    ) -> SimRaftResult<()> {
        let Some(leader_id) = self.leader().and_then(|name| self.ids.get(&name).copied()) else {
            self.trace.push(format!(
                "raft:{now_step}:conf-change-deferred:{node_id}:{change_type:?}"
            ));
            return Ok(());
        };
        let mut change = ConfChange::default();
        change.set_node_id(node_id);
        change.set_change_type(change_type);
        let leader = self
            .nodes
            .get_mut(&leader_id)
            .expect("leader id must resolve to a raw node");
        match leader.raw.propose_conf_change(Vec::new(), change) {
            Ok(()) => {
                self.trace.push(format!(
                    "raft:{now_step}:conf-change:{leader_id}:{node_id}:{change_type:?}"
                ));
                Ok(())
            }
            Err(RaftError::ProposalDropped) => {
                self.trace.push(format!(
                    "raft:{now_step}:conf-change-dropped:{leader_id}:{node_id}:{change_type:?}"
                ));
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn deliver_due(
        &mut self,
        now_step: u64,
        live: &BTreeSet<ClusterNodeId>,
        network: &SimNetwork,
    ) -> SimRaftResult<()> {
        let due = self
            .inflight
            .keys()
            .take_while(|key| key.deliver_at_step <= now_step)
            .copied()
            .collect::<Vec<_>>();
        for key in due {
            let Some(wire) = self.inflight.remove(&key) else {
                continue;
            };
            let Some(from_name) = self.names.get(&key.from) else {
                self.dropped_messages = self.dropped_messages.saturating_add(1);
                continue;
            };
            let Some(to_name) = self.names.get(&key.to) else {
                self.dropped_messages = self.dropped_messages.saturating_add(1);
                continue;
            };
            if !live.contains(to_name) || !network.can_deliver(from_name, to_name) {
                self.dropped_messages = self.dropped_messages.saturating_add(1);
                continue;
            }
            let message = wire.decode()?;
            self.step_message(key.to, message)?;
        }
        Ok(())
    }

    fn drain_ready_until_idle(
        &mut self,
        now_step: u64,
        live: &BTreeSet<ClusterNodeId>,
        network: &SimNetwork,
    ) -> SimRaftResult<()> {
        loop {
            let mut drained_any = false;
            let ids = self.nodes.keys().copied().collect::<Vec<_>>();
            for raft_id in ids {
                if self.drain_ready_for(raft_id, now_step, live, network)? {
                    drained_any = true;
                }
            }
            if !drained_any {
                break;
            }
        }
        Ok(())
    }

    fn drain_ready_for(
        &mut self,
        raft_id: u64,
        now_step: u64,
        live: &BTreeSet<ClusterNodeId>,
        network: &SimNetwork,
    ) -> SimRaftResult<bool> {
        let Some(node) = self.nodes.get_mut(&raft_id) else {
            return Ok(false);
        };
        if !node.raw.has_ready() {
            return Ok(false);
        }

        let mut outbound = Vec::new();
        {
            let store = node.raw.raft.raft_log.store.clone();
            let mut ready = node.raw.ready();
            outbound.extend(ready.take_messages());

            if !ready.snapshot().is_empty() {
                store.save_snapshot(ready.snapshot(), 0)?;
            }

            let committed_entries = ready.take_committed_entries();
            if !ready.entries().is_empty() {
                store.append(ready.entries())?;
            }
            if let Some(hard_state) = ready.hs() {
                store.save_hard_state(hard_state)?;
            }
            apply_committed_entries(
                &mut node.raw,
                &store,
                &mut node.applied_index,
                committed_entries,
            )?;
            outbound.extend(ready.take_persisted_messages());

            let mut light_ready = node.raw.advance(ready);
            if let Some(commit) = light_ready.commit_index() {
                RaftLogStore::set_commit(&store, commit)?;
            }
            outbound.extend(light_ready.take_messages());
            apply_committed_entries(
                &mut node.raw,
                &store,
                &mut node.applied_index,
                light_ready.take_committed_entries(),
            )?;
            store.mark_applied(node.applied_index);
            node.raw.advance_apply();
        }

        self.route_messages(raft_id, outbound, now_step, live, network)?;
        Ok(true)
    }

    fn route_messages(
        &mut self,
        from_id: u64,
        messages: Vec<Message>,
        now_step: u64,
        live: &BTreeSet<ClusterNodeId>,
        network: &SimNetwork,
    ) -> SimRaftResult<()> {
        for message in messages {
            let to_id = message.to;
            if to_id == 0 {
                self.dropped_messages = self.dropped_messages.saturating_add(1);
                continue;
            }
            if from_id == to_id {
                self.step_message(to_id, message)?;
                continue;
            }
            let Some(from_name) = self.names.get(&from_id) else {
                self.dropped_messages = self.dropped_messages.saturating_add(1);
                continue;
            };
            let Some(to_name) = self.names.get(&to_id) else {
                self.dropped_messages = self.dropped_messages.saturating_add(1);
                continue;
            };
            if !live.contains(to_name) || !network.can_deliver(from_name, to_name) {
                self.dropped_messages = self.dropped_messages.saturating_add(1);
                continue;
            }
            if self.inflight.len() >= MAX_RAFT_INFLIGHT {
                return Err(SimRaftError::new(format!(
                    "raft inflight budget exceeded: {MAX_RAFT_INFLIGHT}"
                )));
            }
            let seq = self.next_seq;
            self.next_seq = self.next_seq.saturating_add(1);
            let key = SimRaftInFlightKey {
                deliver_at_step: now_step.saturating_add(1),
                from: from_id,
                to: to_id,
                seq,
            };
            self.inflight
                .insert(key, RaftWireMessage::encode(&message)?);
        }
        Ok(())
    }

    fn step_message(&mut self, to_id: u64, message: Message) -> SimRaftResult<()> {
        let Some(node) = self.nodes.get_mut(&to_id) else {
            self.dropped_messages = self.dropped_messages.saturating_add(1);
            return Ok(());
        };
        seed_election_timeout(&mut node.raw, self.seed, to_id);
        match node.raw.step(message) {
            Ok(()) | Err(RaftError::StepPeerNotFound) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn live_raft_ids(&self, live: &BTreeSet<ClusterNodeId>) -> Vec<u64> {
        live.iter()
            .filter_map(|name| self.ids.get(name).copied())
            .filter(|raft_id| self.nodes.contains_key(raft_id))
            .collect()
    }

    fn is_live_raft_id(&self, raft_id: u64) -> bool {
        self.names
            .get(&raft_id)
            .is_some_and(|name| self.last_live.contains(name))
    }

    fn current_voters(&self) -> Vec<u64> {
        self.nodes
            .values()
            .next()
            .map(|node| {
                let mut voters = node
                    .raw
                    .raft
                    .prs()
                    .conf()
                    .voters()
                    .ids()
                    .iter()
                    .collect::<Vec<_>>();
                voters.sort_unstable();
                voters
            })
            .unwrap_or_default()
    }

    fn voter_count_for(&self, raft_id: u64) -> usize {
        self.nodes
            .get(&raft_id)
            .map(|node| node.raw.raft.prs().conf().voters().ids().len())
            .unwrap_or_default()
    }

    fn quorum(&self) -> usize {
        let voters = self.current_voters().len().max(self.nodes.len());
        voters / 2 + 1
    }

    fn seed_all_timeouts(&mut self) {
        for (raft_id, node) in self.nodes.iter_mut() {
            seed_election_timeout(&mut node.raw, self.seed, *raft_id);
        }
    }

    fn signals(&self) -> Vec<ElectionSignal> {
        self.inflight
            .iter()
            .filter_map(|(key, wire)| {
                let from = self.names.get(&key.from)?.clone();
                let to = self.names.get(&key.to)?.clone();
                let message = wire.decode().ok()?;
                Some(ElectionSignal {
                    id: key.seq,
                    from,
                    to,
                    kind: signal_kind(&message),
                    term: message.term,
                })
            })
            .collect()
    }
}

fn build_raw_node(
    raft_id: u64,
    peers: &[u64],
    election_tick: usize,
    heartbeat_tick: usize,
    logger: &Logger,
) -> SimRaftResult<RawNode<InMemoryRaftLogStore>> {
    let storage = InMemoryRaftLogStore::new_with_conf_state((peers.to_vec(), vec![]));
    let config = Config {
        id: raft_id,
        election_tick,
        heartbeat_tick,
        check_quorum: true,
        pre_vote: true,
        ..Default::default()
    };
    config.validate()?;
    let mut raw = RawNode::new(&config, storage, logger)?;
    seed_election_timeout(&mut raw, 0, raft_id);
    Ok(raw)
}

fn seed_election_timeout(raw: &mut RawNode<InMemoryRaftLogStore>, seed: u64, raft_id: u64) {
    let base = raw.raft.election_timeout();
    let span = base.max(1);
    let mut h = seed ^ raft_id.rotate_left(17) ^ raw.raft.term.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    h ^= h >> 29;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let timeout = base + (h as usize % span);
    raw.raft.set_randomized_election_timeout(timeout);
}

fn apply_committed_entries(
    raw: &mut RawNode<InMemoryRaftLogStore>,
    store: &InMemoryRaftLogStore,
    applied_index: &mut u64,
    entries: Vec<Entry>,
) -> SimRaftResult<()> {
    for entry in entries {
        *applied_index = (*applied_index).max(entry.index);
        if entry.data.is_empty() {
            continue;
        }
        match entry.get_entry_type() {
            EntryType::EntryNormal => {}
            EntryType::EntryConfChange => {
                let mut change = ConfChange::default();
                change.merge_from_bytes(entry.data.as_ref())?;
                let conf_state = raw.apply_conf_change(&change)?;
                store.initialize_with_conf_state(conf_state);
            }
            EntryType::EntryConfChangeV2 => {
                let mut change = ConfChangeV2::default();
                change.merge_from_bytes(entry.data.as_ref())?;
                let conf_state = raw.apply_conf_change(&change)?;
                store.initialize_with_conf_state(conf_state);
            }
        }
    }
    Ok(())
}

fn vote_name(vote: u64, names: &BTreeMap<u64, ClusterNodeId>) -> Option<ClusterNodeId> {
    (vote != 0).then(|| names.get(&vote).cloned()).flatten()
}

fn signal_kind(message: &Message) -> ElectionSignalKind {
    match message.get_msg_type() {
        raft::eraftpb::MessageType::MsgRequestVote
        | raft::eraftpb::MessageType::MsgRequestPreVote => ElectionSignalKind::VoteRequest,
        raft::eraftpb::MessageType::MsgRequestVoteResponse
        | raft::eraftpb::MessageType::MsgRequestPreVoteResponse => ElectionSignalKind::VoteResponse,
        raft::eraftpb::MessageType::MsgHeartbeatResponse
        | raft::eraftpb::MessageType::MsgAppendResponse => ElectionSignalKind::HeartbeatAck,
        _ => ElectionSignalKind::LeaderHeartbeat,
    }
}
