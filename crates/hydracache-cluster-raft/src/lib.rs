//! raft-rs metadata control-plane runtime for HydraCache cluster mode.
//!
//! The base `hydracache` crate exposes a transport-neutral
//! [`hydracache::ClusterControlPlane`] trait. This crate plugs a real
//! `raft-rs` [`raft::RawNode`] behind that trait while keeping the local cache
//! crate free from Raft dependencies.
//!
//! The current runtime is intentionally single-node and in-memory. It still
//! drives the real raft-rs lifecycle: campaign, propose, `Ready`, stable-log
//! append, and committed-entry application.
//!
//! ## Bridging Discovery To Raft Metadata
//!
//! The cluster composition is deliberately split:
//!
//! - `hydracache-cluster-chitchat` discovers member/client candidates.
//! - [`hydracache::ClusterAdmissionBridge`] polls those candidates.
//! - `RaftMetadataRuntime` commits accepted membership metadata.
//!
//! ```no_run
//! use std::net::SocketAddr;
//! use std::sync::Arc;
//!
//! use hydracache::{
//!     ClusterAdmissionBridge, ClusterCandidate, ClusterGeneration,
//!     ClusterDiscovery,
//! };
//! use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
//! use hydracache_cluster_raft::RaftMetadataRuntime;
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let discovery = Arc::new(
//!     ChitchatDiscovery::spawn_udp(ChitchatDiscoveryConfig::new(
//!         "orders",
//!         "member-a",
//!         ClusterGeneration::new(1),
//!         "127.0.0.1:7000".parse::<SocketAddr>().unwrap(),
//!     ))
//!     .await?,
//! );
//! let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1)?);
//! let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());
//!
//! discovery
//!     .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
//!     .await?;
//! bridge.run_once().await;
//!
//! assert_eq!(bridge.diagnostics().candidates_admitted, 1);
//! assert_eq!(control_plane.snapshot().commands_committed, 1);
//! # Ok(())
//! # }
//! ```
//!
//! # Example
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use hydracache::{ClusterGeneration, HydraCache};
//! use hydracache_cluster_raft::RaftMetadataRuntime;
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1)?);
//!
//! let member = HydraCache::member()
//!     .control_plane(control_plane.clone())
//!     .node_id("member-a")
//!     .generation(ClusterGeneration::new(1))
//!     .start()
//!     .await?;
//!
//! assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
//! assert_eq!(control_plane.snapshot().commands_committed, 1);
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::sync::{Arc, Mutex};

use hydracache::{
    CacheError, CacheInvalidationBus, CacheResult, ClusterCandidate, ClusterControlPlane,
    ClusterDiagnostics, ClusterGeneration, ClusterMember, ClusterMembershipEvent, ClusterNodeId,
    ClusterRole, InMemoryCluster, RaftMetadataCommand,
};
use raft::eraftpb::{Entry, EntryType};
use raft::storage::MemStorage;
use raft::{Config, RawNode, StateRole};
use slog::{o, Logger};

/// Configuration for an embedded raft-rs metadata runtime.
#[derive(Debug, Clone)]
pub struct RaftMetadataRuntimeConfig {
    cluster_name: String,
    raft_node_id: u64,
    election_tick: usize,
    heartbeat_tick: usize,
    max_size_per_msg: u64,
    max_inflight_msgs: usize,
}

impl RaftMetadataRuntimeConfig {
    /// Build a single-node runtime configuration.
    pub fn single_node(cluster_name: impl Into<String>, raft_node_id: u64) -> Self {
        Self {
            cluster_name: cluster_name.into(),
            raft_node_id: raft_node_id.max(1),
            election_tick: 10,
            heartbeat_tick: 3,
            max_size_per_msg: 1024 * 1024,
            max_inflight_msgs: 256,
        }
    }

    /// Set Raft election and heartbeat ticks.
    pub fn ticks(mut self, election_tick: usize, heartbeat_tick: usize) -> Self {
        self.election_tick = election_tick.max(2);
        self.heartbeat_tick = heartbeat_tick.max(1);
        self
    }

    /// Set the max bytes per Raft message.
    pub fn max_size_per_msg(mut self, size: u64) -> Self {
        self.max_size_per_msg = size.max(1);
        self
    }

    /// Set max inflight messages.
    pub fn max_inflight_msgs(mut self, value: usize) -> Self {
        self.max_inflight_msgs = value.max(1);
        self
    }

    fn raft_config(&self) -> Config {
        Config {
            id: self.raft_node_id,
            election_tick: self.election_tick,
            heartbeat_tick: self.heartbeat_tick,
            max_size_per_msg: self.max_size_per_msg,
            max_inflight_msgs: self.max_inflight_msgs,
            applied: 0,
            ..Default::default()
        }
    }
}

/// Point-in-time raft-rs metadata runtime snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftMetadataRuntimeSnapshot {
    /// Local raft node id.
    pub raft_node_id: u64,
    /// Current raft term.
    pub term: u64,
    /// Current committed index.
    pub commit_index: u64,
    /// Last applied index tracked by the metadata state machine.
    pub applied_index: u64,
    /// Current raft role.
    pub role: RaftRuntimeRole,
    /// Number of metadata commands applied from committed Raft entries.
    pub commands_committed: usize,
    /// Last applied metadata command, if any.
    pub last_command: Option<RaftMetadataCommand>,
}

/// Stable debug-friendly view of raft-rs [`StateRole`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftRuntimeRole {
    /// Node is follower.
    Follower,
    /// Node is candidate or pre-candidate.
    Candidate,
    /// Node is leader.
    Leader,
}

impl From<StateRole> for RaftRuntimeRole {
    fn from(role: StateRole) -> Self {
        match role {
            StateRole::Follower => Self::Follower,
            StateRole::PreCandidate | StateRole::Candidate => Self::Candidate,
            StateRole::Leader => Self::Leader,
        }
    }
}

struct RaftRuntimeState {
    raw_node: RawNode<MemStorage>,
    commands: Vec<RaftMetadataCommand>,
    applied_index: u64,
}

impl fmt::Debug for RaftRuntimeState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RaftRuntimeState")
            .field("commands", &self.commands)
            .field("applied_index", &self.applied_index)
            .finish_non_exhaustive()
    }
}

/// Single-node raft-rs metadata control plane.
#[derive(Debug)]
pub struct RaftMetadataRuntime {
    cluster: InMemoryCluster,
    raft_node_id: u64,
    raft: Mutex<RaftRuntimeState>,
}

impl RaftMetadataRuntime {
    /// Start a single-node raft-rs metadata runtime.
    pub fn single_node(cluster_name: impl Into<String>, raft_node_id: u64) -> CacheResult<Self> {
        Self::with_config(RaftMetadataRuntimeConfig::single_node(
            cluster_name,
            raft_node_id,
        ))
    }

    /// Start a single-node raft-rs metadata runtime with explicit config.
    pub fn with_config(config: RaftMetadataRuntimeConfig) -> CacheResult<Self> {
        let cluster_name = config.cluster_name.clone();
        let raft_node_id = config.raft_node_id;
        let storage = MemStorage::new_with_conf_state((vec![raft_node_id], vec![]));
        let logger = Logger::root(slog::Discard, o!());
        let mut raw_node =
            RawNode::new(&config.raft_config(), storage, &logger).map_err(to_cache_error)?;
        raw_node.campaign().map_err(to_cache_error)?;

        let mut state = RaftRuntimeState {
            raw_node,
            commands: Vec::new(),
            applied_index: 0,
        };
        state.drain_ready()?;

        Ok(Self {
            cluster: InMemoryCluster::new(cluster_name),
            raft_node_id,
            raft: Mutex::new(state),
        })
    }

    /// Return applied metadata commands.
    pub fn commands(&self) -> Vec<RaftMetadataCommand> {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .commands
            .clone()
    }

    /// Return a runtime snapshot.
    pub fn snapshot(&self) -> RaftMetadataRuntimeSnapshot {
        let state = self.raft.lock().expect("raft metadata state poisoned");
        let hard_state = state.raw_node.raft.hard_state();
        RaftMetadataRuntimeSnapshot {
            raft_node_id: self.raft_node_id,
            term: state.raw_node.raft.term,
            commit_index: hard_state.commit,
            applied_index: state.applied_index,
            role: state.raw_node.raft.state.into(),
            commands_committed: state.commands.len(),
            last_command: state.commands.last().cloned(),
        }
    }

    fn commit_command(&self, command: RaftMetadataCommand) -> CacheResult<()> {
        self.raft
            .lock()
            .expect("raft metadata state poisoned")
            .commit_command(command)
    }
}

impl RaftRuntimeState {
    fn commit_command(&mut self, command: RaftMetadataCommand) -> CacheResult<()> {
        self.raw_node
            .propose(vec![], encode_command(&command))
            .map_err(to_cache_error)?;
        self.drain_ready()
    }

    fn drain_ready(&mut self) -> CacheResult<()> {
        while self.raw_node.has_ready() {
            let store = self.raw_node.raft.raft_log.store.clone();
            let mut ready = self.raw_node.ready();

            if !ready.snapshot().is_empty() {
                store
                    .wl()
                    .apply_snapshot(ready.snapshot().clone())
                    .map_err(to_cache_error)?;
            }

            let committed_entries = ready.take_committed_entries();

            if !ready.entries().is_empty() {
                store.wl().append(ready.entries()).map_err(to_cache_error)?;
            }

            if let Some(hard_state) = ready.hs() {
                store.wl().set_hardstate(hard_state.clone());
            }

            self.apply_committed_entries(committed_entries)?;

            let mut light_ready = self.raw_node.advance(ready);
            if let Some(commit) = light_ready.commit_index() {
                store.wl().mut_hard_state().set_commit(commit);
            }
            self.apply_committed_entries(light_ready.take_committed_entries())?;
            self.raw_node.advance_apply();
        }
        Ok(())
    }

    fn apply_committed_entries(&mut self, entries: Vec<Entry>) -> CacheResult<()> {
        for entry in entries {
            self.applied_index = entry.index;
            if entry.data.is_empty() || entry.get_entry_type() != EntryType::EntryNormal {
                continue;
            }
            self.commands.push(decode_command(entry.data.as_ref())?);
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ClusterControlPlane for RaftMetadataRuntime {
    fn name(&self) -> String {
        self.cluster.name().to_owned()
    }

    fn invalidation_bus(&self) -> Arc<dyn CacheInvalidationBus> {
        self.cluster.invalidation_bus()
    }

    async fn join_member(&self, candidate: ClusterCandidate) -> CacheResult<ClusterMember> {
        let member = self.cluster.join_member(candidate)?;
        self.commit_command(RaftMetadataCommand::MemberUpsert {
            node_id: member.node_id.clone(),
            generation: member.generation,
            epoch: member.epoch,
        })?;
        Ok(member)
    }

    async fn join_client(&self, candidate: ClusterCandidate) -> CacheResult<ClusterMember> {
        let member = self.cluster.join_client(candidate)?;
        self.commit_command(RaftMetadataCommand::ClientUpsert {
            node_id: member.node_id.clone(),
            generation: member.generation,
            epoch: member.epoch,
        })?;
        Ok(member)
    }

    async fn validate_generation(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> CacheResult<()> {
        self.cluster.validate_generation(node_id, generation)
    }

    async fn leave(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> CacheResult<Option<ClusterMembershipEvent>> {
        let Some(event) = self.cluster.leave(node_id, generation)? else {
            return Ok(None);
        };
        if let ClusterMembershipEvent::NodeLeft {
            node_id,
            role,
            epoch,
        } = &event
        {
            self.commit_command(RaftMetadataCommand::NodeLeft {
                node_id: node_id.clone(),
                role: *role,
                epoch: *epoch,
            })?;
        }
        Ok(Some(event))
    }

    fn diagnostics_for(
        &self,
        role: ClusterRole,
        node_id: ClusterNodeId,
        generation: ClusterGeneration,
        bootstrap: Vec<String>,
    ) -> ClusterDiagnostics {
        self.cluster
            .diagnostics_for(role, node_id, generation, bootstrap)
    }
}

fn encode_command(command: &RaftMetadataCommand) -> Vec<u8> {
    match command {
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            epoch,
        } => format!("member|{node_id}|{}|{}", generation.value(), epoch.value()).into_bytes(),
        RaftMetadataCommand::ClientUpsert {
            node_id,
            generation,
            epoch,
        } => format!("client|{node_id}|{}|{}", generation.value(), epoch.value()).into_bytes(),
        RaftMetadataCommand::NodeLeft {
            node_id,
            role,
            epoch,
        } => format!("left|{node_id}|{}|{}", role_to_str(*role), epoch.value()).into_bytes(),
    }
}

fn decode_command(data: &[u8]) -> CacheResult<RaftMetadataCommand> {
    let text = std::str::from_utf8(data)
        .map_err(|error| CacheError::Backend(format!("invalid raft command utf8: {error}")))?;
    let parts = text.split('|').collect::<Vec<_>>();
    match parts.as_slice() {
        ["member", node_id, generation, epoch] => Ok(RaftMetadataCommand::MemberUpsert {
            node_id: ClusterNodeId::from((*node_id).to_owned()),
            generation: ClusterGeneration::new(parse_u64(generation, "generation")?),
            epoch: hydracache::ClusterEpoch::new(parse_u64(epoch, "epoch")?),
        }),
        ["client", node_id, generation, epoch] => Ok(RaftMetadataCommand::ClientUpsert {
            node_id: ClusterNodeId::from((*node_id).to_owned()),
            generation: ClusterGeneration::new(parse_u64(generation, "generation")?),
            epoch: hydracache::ClusterEpoch::new(parse_u64(epoch, "epoch")?),
        }),
        ["left", node_id, role, epoch] => Ok(RaftMetadataCommand::NodeLeft {
            node_id: ClusterNodeId::from((*node_id).to_owned()),
            role: parse_role(role)?,
            epoch: hydracache::ClusterEpoch::new(parse_u64(epoch, "epoch")?),
        }),
        _ => Err(CacheError::Backend(format!(
            "invalid raft metadata command: {text}"
        ))),
    }
}

fn parse_u64(value: &str, label: &str) -> CacheResult<u64> {
    value
        .parse::<u64>()
        .map_err(|error| CacheError::Backend(format!("invalid {label} in raft command: {error}")))
}

fn role_to_str(role: ClusterRole) -> &'static str {
    match role {
        ClusterRole::Local => "local",
        ClusterRole::Client => "client",
        ClusterRole::Member => "member",
    }
}

fn parse_role(value: &str) -> CacheResult<ClusterRole> {
    match value {
        "local" => Ok(ClusterRole::Local),
        "client" => Ok(ClusterRole::Client),
        "member" => Ok(ClusterRole::Member),
        _ => Err(CacheError::Backend(format!(
            "invalid raft metadata role: {value}"
        ))),
    }
}

fn to_cache_error(error: impl fmt::Display) -> CacheError {
    CacheError::Backend(format!("raft metadata runtime failed: {error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hydracache::{HydraCache, InMemoryCluster};

    use super::*;

    #[test]
    fn runtime_campaigns_single_node_to_leader() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        let snapshot = runtime.snapshot();

        assert_eq!(snapshot.raft_node_id, 1);
        assert_eq!(snapshot.role, RaftRuntimeRole::Leader);
        assert_eq!(snapshot.commands_committed, 0);
    }

    #[tokio::test]
    async fn member_and_client_admission_are_committed_through_raft() {
        let runtime = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());

        let member = HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(1))
            .start()
            .await
            .unwrap();
        let client = HydraCache::client()
            .control_plane(runtime.clone())
            .node_id("client-a")
            .generation(ClusterGeneration::new(1))
            .connect()
            .await
            .unwrap();

        let snapshot = runtime.snapshot();
        assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
        assert_eq!(client.cluster_diagnostics().unwrap().client_count, 1);
        assert_eq!(snapshot.role, RaftRuntimeRole::Leader);
        assert_eq!(snapshot.commands_committed, 2);
        assert!(snapshot.commit_index >= 3);
        assert!(matches!(
            &runtime.commands()[0],
            RaftMetadataCommand::MemberUpsert { node_id, .. } if node_id.as_str() == "member-a"
        ));
        assert!(matches!(
            snapshot.last_command,
            Some(RaftMetadataCommand::ClientUpsert { ref node_id, .. })
                if node_id.as_str() == "client-a"
        ));
    }

    #[tokio::test]
    async fn stale_generation_rejection_does_not_commit_raft_command() {
        let runtime = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());

        HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(2))
            .start()
            .await
            .unwrap();

        let error = HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(1))
            .start()
            .await
            .unwrap_err();

        assert!(error.to_string().contains("stale cluster generation"));
        assert_eq!(runtime.snapshot().commands_committed, 1);
    }

    #[tokio::test]
    async fn leave_is_generation_safe_and_committed_through_raft() {
        let runtime = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());
        let stale = HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(1))
            .start()
            .await
            .unwrap();
        let current = HydraCache::member()
            .control_plane(runtime.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(2))
            .start()
            .await
            .unwrap();

        let error = stale.leave_cluster().await.unwrap_err();
        assert!(error.to_string().contains("stale cluster generation"));
        assert_eq!(runtime.snapshot().commands_committed, 2);

        let left = current.leave_cluster().await.unwrap();
        assert!(left.is_some());
        assert_eq!(runtime.snapshot().commands_committed, 3);
        assert!(matches!(
            runtime.snapshot().last_command,
            Some(RaftMetadataCommand::NodeLeft { .. })
        ));
    }

    #[test]
    fn command_encoding_round_trips_without_json_dependency() {
        let commands = [
            RaftMetadataCommand::MemberUpsert {
                node_id: ClusterNodeId::from("member-a"),
                generation: ClusterGeneration::new(1),
                epoch: hydracache::ClusterEpoch::new(2),
            },
            RaftMetadataCommand::ClientUpsert {
                node_id: ClusterNodeId::from("client-a"),
                generation: ClusterGeneration::new(3),
                epoch: hydracache::ClusterEpoch::new(4),
            },
            RaftMetadataCommand::NodeLeft {
                node_id: ClusterNodeId::from("member-a"),
                role: ClusterRole::Member,
                epoch: hydracache::ClusterEpoch::new(5),
            },
        ];

        for command in commands {
            assert_eq!(decode_command(&encode_command(&command)).unwrap(), command);
        }
    }

    #[test]
    fn runtime_keeps_invalidation_bus_from_inner_cluster() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        let _subscriber = runtime.invalidation_bus().subscribe();

        let diagnostics = runtime.diagnostics_for(
            ClusterRole::Member,
            ClusterNodeId::from("member-a"),
            ClusterGeneration::new(1),
            Vec::new(),
        );

        assert_eq!(diagnostics.invalidation_subscribers, 1);
    }

    #[test]
    fn config_clamps_small_values() {
        let config = RaftMetadataRuntimeConfig::single_node("orders", 0)
            .ticks(0, 0)
            .max_size_per_msg(0)
            .max_inflight_msgs(0);

        assert_eq!(config.raft_node_id, 1);
        assert_eq!(config.election_tick, 2);
        assert_eq!(config.heartbeat_tick, 1);
        assert_eq!(config.max_size_per_msg, 1);
        assert_eq!(config.max_inflight_msgs, 1);
    }

    #[test]
    fn can_still_use_in_memory_cluster_for_comparison() {
        let cluster = InMemoryCluster::new("orders");
        assert_eq!(cluster.name(), "orders");
    }
}
