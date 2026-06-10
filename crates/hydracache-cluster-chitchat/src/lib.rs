//! Chitchat-backed discovery adapter for HydraCache cluster mode.
//!
//! This crate is intentionally separate from `hydracache` so local-only users
//! do not pay for gossip dependencies. `ChitchatDiscovery` implements
//! [`hydracache::ClusterDiscovery`] and stores HydraCache candidate metadata in
//! real `chitchat` node state.
//!
//! # Example
//!
//! ```no_run
//! use std::net::SocketAddr;
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use hydracache::{ClusterGeneration, HydraCache, InMemoryCluster};
//! use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cluster = Arc::new(InMemoryCluster::new("orders"));
//! let discovery = Arc::new(
//!     ChitchatDiscovery::spawn_udp(
//!         ChitchatDiscoveryConfig::new(
//!             "orders",
//!             "member-a",
//!             ClusterGeneration::new(1),
//!             "127.0.0.1:7000".parse::<SocketAddr>().unwrap(),
//!         )
//!         .gossip_interval(Duration::from_millis(200)),
//!     )
//!     .await?,
//! );
//!
//! let member = HydraCache::member()
//!     .shared_cluster(cluster)
//!     .discovery(discovery)
//!     .node_id("member-a")
//!     .generation(ClusterGeneration::new(1))
//!     .start()
//!     .await?;
//!
//! assert!(member.cluster_discovery_diagnostics().unwrap().has_candidates());
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chitchat::transport::{Transport, UdpTransport};
use chitchat::{
    spawn_chitchat, Chitchat, ChitchatConfig, ChitchatHandle, ChitchatId, FailureDetectorConfig,
    NodeState,
};
use hydracache::{
    CacheError, CacheResult, ClusterCandidate, ClusterDiscovery, ClusterDiscoveryEvent,
    ClusterEndpoints, ClusterGeneration, ClusterNodeId, ClusterRole,
};
use tokio::sync::Mutex as TokioMutex;

const KEY_ADAPTER: &str = "hydracache.discovery.adapter";
const KEY_ROLE: &str = "hydracache.role";
const KEY_GENERATION: &str = "hydracache.generation";
const KEY_ENDPOINT_CONTROL: &str = "hydracache.endpoint.control";
const KEY_ENDPOINT_INVALIDATION: &str = "hydracache.endpoint.invalidation";
const KEY_ENDPOINT_DIAGNOSTICS: &str = "hydracache.endpoint.diagnostics";
const KEY_METADATA_PREFIX: &str = "hydracache.metadata.";

/// Configuration for a chitchat-backed HydraCache discovery node.
#[derive(Debug, Clone)]
pub struct ChitchatDiscoveryConfig {
    cluster_id: String,
    node_id: ClusterNodeId,
    generation: ClusterGeneration,
    listen_addr: SocketAddr,
    gossip_advertise_addr: SocketAddr,
    seed_nodes: Vec<String>,
    gossip_interval: Duration,
    marked_for_deletion_grace_period: Duration,
    failure_detector_config: FailureDetectorConfig,
}

impl ChitchatDiscoveryConfig {
    /// Build a config using the same listen and advertised gossip address.
    pub fn new(
        cluster_id: impl Into<String>,
        node_id: impl Into<ClusterNodeId>,
        generation: ClusterGeneration,
        listen_addr: SocketAddr,
    ) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            node_id: node_id.into(),
            generation,
            listen_addr,
            gossip_advertise_addr: listen_addr,
            seed_nodes: Vec::new(),
            gossip_interval: Duration::from_millis(250),
            marked_for_deletion_grace_period: Duration::from_secs(60),
            failure_detector_config: FailureDetectorConfig::default(),
        }
    }

    /// Advertise a different gossip address than the local bind address.
    pub fn gossip_advertise_addr(mut self, addr: SocketAddr) -> Self {
        self.gossip_advertise_addr = addr;
        self
    }

    /// Add one seed node address.
    pub fn seed_node(mut self, seed: impl Into<String>) -> Self {
        self.seed_nodes.push(seed.into());
        self
    }

    /// Replace all seed node addresses.
    pub fn seed_nodes<I, S>(mut self, seeds: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.seed_nodes = seeds.into_iter().map(Into::into).collect();
        self
    }

    /// Set the gossip interval.
    pub fn gossip_interval(mut self, interval: Duration) -> Self {
        self.gossip_interval = interval;
        self
    }

    /// Set the tombstone grace period for chitchat node-state keys.
    pub fn marked_for_deletion_grace_period(mut self, period: Duration) -> Self {
        self.marked_for_deletion_grace_period = period;
        self
    }

    /// Set chitchat's failure detector configuration.
    pub fn failure_detector_config(mut self, config: FailureDetectorConfig) -> Self {
        self.failure_detector_config = config;
        self
    }

    /// Return the logical HydraCache cluster id.
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Return the stable HydraCache node id.
    pub fn node_id(&self) -> &ClusterNodeId {
        &self.node_id
    }

    /// Return the process generation advertised in chitchat.
    pub fn generation(&self) -> ClusterGeneration {
        self.generation
    }

    /// Return the UDP bind address.
    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    /// Return configured seed addresses.
    pub fn seed_nodes_value(&self) -> &[String] {
        &self.seed_nodes
    }

    fn chitchat_id(&self) -> ChitchatId {
        ChitchatId::new(
            self.node_id.as_str().to_owned(),
            self.generation.value(),
            self.gossip_advertise_addr,
        )
    }

    fn into_chitchat_config(self) -> ChitchatConfig {
        ChitchatConfig {
            chitchat_id: self.chitchat_id(),
            cluster_id: self.cluster_id,
            gossip_interval: self.gossip_interval,
            listen_addr: self.listen_addr,
            seed_nodes: self.seed_nodes,
            failure_detector_config: self.failure_detector_config,
            marked_for_deletion_grace_period: self.marked_for_deletion_grace_period,
            catchup_callback: None,
            extra_liveness_predicate: None,
        }
    }
}

#[derive(Debug, Default)]
struct DiscoveryState {
    candidates: BTreeMap<ClusterNodeId, ClusterCandidate>,
    events: Vec<ClusterDiscoveryEvent>,
}

/// Real chitchat-backed implementation of [`ClusterDiscovery`].
pub struct ChitchatDiscovery {
    chitchat_id: ChitchatId,
    chitchat: Arc<TokioMutex<Chitchat>>,
    handle: ChitchatHandle,
    state: Arc<Mutex<DiscoveryState>>,
}

impl fmt::Debug for ChitchatDiscovery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChitchatDiscovery")
            .field("chitchat_id", &self.chitchat_id)
            .field("candidate_count", &self.candidates().len())
            .field("event_count", &self.events().len())
            .finish_non_exhaustive()
    }
}

impl Drop for ChitchatDiscovery {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl ChitchatDiscovery {
    /// Spawn a discovery node using chitchat's UDP transport.
    pub async fn spawn_udp(config: ChitchatDiscoveryConfig) -> CacheResult<Self> {
        Self::spawn_with_transport(config, &UdpTransport).await
    }

    /// Spawn a discovery node using a caller-provided chitchat transport.
    ///
    /// Tests can use `chitchat::transport::ChannelTransport`; production code
    /// usually uses [`spawn_udp`](Self::spawn_udp).
    pub async fn spawn_with_transport(
        config: ChitchatDiscoveryConfig,
        transport: &dyn Transport,
    ) -> CacheResult<Self> {
        let handle = spawn_chitchat(
            config.into_chitchat_config(),
            vec![(KEY_ADAPTER.to_owned(), "chitchat".to_owned())],
            transport,
        )
        .await
        .map_err(to_cache_error)?;
        let chitchat_id = handle.chitchat_id().clone();
        let chitchat = handle.chitchat();
        let state = Arc::new(Mutex::new(DiscoveryState::default()));

        spawn_live_node_watcher(chitchat.clone(), state.clone());

        Ok(Self {
            chitchat_id,
            chitchat,
            handle,
            state,
        })
    }

    /// Return this node's chitchat id.
    pub fn chitchat_id(&self) -> &ChitchatId {
        &self.chitchat_id
    }

    /// Return the latest known candidates.
    pub fn candidates(&self) -> Vec<ClusterCandidate> {
        self.state
            .lock()
            .expect("chitchat discovery state poisoned")
            .candidates
            .values()
            .cloned()
            .collect()
    }

    /// Return observed discovery events.
    pub fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        self.state
            .lock()
            .expect("chitchat discovery state poisoned")
            .events
            .clone()
    }

    /// Ask this node to gossip with a specific peer immediately.
    pub fn gossip_once(&self, addr: SocketAddr) -> CacheResult<()> {
        self.handle.gossip(addr).map_err(to_cache_error)
    }

    /// Return one local chitchat key-value for diagnostics and tests.
    pub async fn local_value(&self, key: &str) -> Option<String> {
        self.chitchat
            .lock()
            .await
            .self_node_state()
            .get(key)
            .map(ToOwned::to_owned)
    }

    async fn announce_candidate(&self, mut candidate: ClusterCandidate) -> CacheResult<()> {
        candidate
            .metadata
            .entry("discovery.adapter".to_owned())
            .or_insert_with(|| "chitchat".to_owned());

        self.chitchat
            .lock()
            .await
            .self_node_state()
            .set(KEY_ADAPTER, "chitchat");
        write_candidate_to_chitchat(self.chitchat.clone(), &candidate).await;
        record_candidate(self.state.clone(), candidate);
        Ok(())
    }

    fn push_event(&self, event: ClusterDiscoveryEvent) {
        self.state
            .lock()
            .expect("chitchat discovery state poisoned")
            .events
            .push(event);
    }
}

#[async_trait::async_trait]
impl ClusterDiscovery for ChitchatDiscovery {
    async fn announce(&self, candidate: ClusterCandidate) -> CacheResult<()> {
        self.announce_candidate(candidate).await
    }

    async fn mark_live(&self, node_id: ClusterNodeId) -> CacheResult<()> {
        self.push_event(ClusterDiscoveryEvent::MemberLive(node_id));
        Ok(())
    }

    async fn mark_suspect(&self, node_id: ClusterNodeId) -> CacheResult<()> {
        self.push_event(ClusterDiscoveryEvent::MemberSuspect(node_id));
        Ok(())
    }

    async fn mark_dead(&self, node_id: ClusterNodeId) -> CacheResult<()> {
        self.push_event(ClusterDiscoveryEvent::MemberDead(node_id));
        Ok(())
    }

    fn candidates(&self) -> Vec<ClusterCandidate> {
        ChitchatDiscovery::candidates(self)
    }

    fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        ChitchatDiscovery::events(self)
    }
}

async fn write_candidate_to_chitchat(
    chitchat: Arc<TokioMutex<Chitchat>>,
    candidate: &ClusterCandidate,
) {
    let mut chitchat = chitchat.lock().await;
    let node_state = chitchat.self_node_state();
    node_state.set(KEY_ROLE, role_to_str(candidate.role));
    node_state.set(KEY_GENERATION, candidate.generation.value().to_string());
    set_optional(
        node_state,
        KEY_ENDPOINT_CONTROL,
        candidate.endpoints.control.as_deref(),
    );
    set_optional(
        node_state,
        KEY_ENDPOINT_INVALIDATION,
        candidate.endpoints.invalidation.as_deref(),
    );
    set_optional(
        node_state,
        KEY_ENDPOINT_DIAGNOSTICS,
        candidate.endpoints.diagnostics.as_deref(),
    );
    for (key, value) in &candidate.metadata {
        node_state.set(format!("{KEY_METADATA_PREFIX}{key}"), value);
    }
}

fn set_optional(node_state: &mut NodeState, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        node_state.set(key, value);
    }
}

fn spawn_live_node_watcher(chitchat: Arc<TokioMutex<Chitchat>>, state: Arc<Mutex<DiscoveryState>>) {
    tokio::spawn(async move {
        let mut live_nodes = {
            let chitchat = chitchat.lock().await;
            chitchat.live_nodes_watcher()
        };

        while live_nodes.changed().await.is_ok() {
            let candidates = live_nodes
                .borrow()
                .iter()
                .filter_map(|(chitchat_id, node_state)| {
                    candidate_from_node(chitchat_id, node_state)
                })
                .collect::<Vec<_>>();

            let mut state = state.lock().expect("chitchat discovery state poisoned");
            for candidate in candidates {
                state
                    .events
                    .push(ClusterDiscoveryEvent::MemberLive(candidate.node_id.clone()));
                state
                    .candidates
                    .insert(candidate.node_id.clone(), candidate);
            }
        }
    });
}

fn record_candidate(state: Arc<Mutex<DiscoveryState>>, candidate: ClusterCandidate) {
    let mut state = state.lock().expect("chitchat discovery state poisoned");
    state
        .events
        .push(ClusterDiscoveryEvent::CandidateSeen(candidate.clone()));
    state
        .candidates
        .insert(candidate.node_id.clone(), candidate);
}

fn candidate_from_node(
    chitchat_id: &ChitchatId,
    node_state: &NodeState,
) -> Option<ClusterCandidate> {
    let role = parse_role(node_state.get(KEY_ROLE)?)?;
    let generation = node_state
        .get(KEY_GENERATION)
        .and_then(|value| value.parse::<u64>().ok())
        .map(ClusterGeneration::new)
        .unwrap_or_else(|| ClusterGeneration::new(chitchat_id.generation_id));
    let mut candidate = match role {
        ClusterRole::Member => ClusterCandidate::member(chitchat_id.node_id.to_string()),
        ClusterRole::Client => ClusterCandidate::client(chitchat_id.node_id.to_string()),
        ClusterRole::Local => return None,
    }
    .generation(generation)
    .endpoints(ClusterEndpoints {
        control: node_state.get(KEY_ENDPOINT_CONTROL).map(ToOwned::to_owned),
        invalidation: node_state
            .get(KEY_ENDPOINT_INVALIDATION)
            .map(ToOwned::to_owned),
        diagnostics: node_state
            .get(KEY_ENDPOINT_DIAGNOSTICS)
            .map(ToOwned::to_owned),
    });

    for (key, value) in node_state.key_values() {
        if let Some(metadata_key) = key.strip_prefix(KEY_METADATA_PREFIX) {
            candidate
                .metadata
                .insert(metadata_key.to_owned(), value.to_owned());
        }
    }
    candidate
        .metadata
        .entry("discovery.adapter".to_owned())
        .or_insert_with(|| "chitchat".to_owned());
    Some(candidate)
}

fn role_to_str(role: ClusterRole) -> &'static str {
    match role {
        ClusterRole::Local => "local",
        ClusterRole::Client => "client",
        ClusterRole::Member => "member",
    }
}

fn parse_role(value: &str) -> Option<ClusterRole> {
    match value {
        "client" => Some(ClusterRole::Client),
        "member" => Some(ClusterRole::Member),
        "local" => Some(ClusterRole::Local),
        _ => None,
    }
}

fn to_cache_error(error: impl std::fmt::Display) -> CacheError {
    CacheError::Backend(format!("chitchat discovery failed: {error}"))
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use chitchat::transport::ChannelTransport;
    use hydracache::{ClusterDiscovery, ClusterEndpoints};
    use tokio::time::{sleep, timeout};

    use super::*;

    fn addr(port: u16) -> SocketAddr {
        ([127, 0, 0, 1], port).into()
    }

    fn config(port: u16, node: &str) -> ChitchatDiscoveryConfig {
        ChitchatDiscoveryConfig::new(
            "orders",
            node,
            ClusterGeneration::new(port as u64),
            addr(port),
        )
        .gossip_interval(Duration::from_millis(20))
    }

    #[test]
    fn config_builds_chitchat_identity_with_generation() {
        let config = ChitchatDiscoveryConfig::new(
            "orders",
            "member-a",
            ClusterGeneration::new(42),
            addr(47_001),
        )
        .seed_node("127.0.0.1:47000");

        let id = config.chitchat_id();

        assert_eq!(config.cluster_id(), "orders");
        assert_eq!(config.node_id().as_str(), "member-a");
        assert_eq!(config.generation(), ClusterGeneration::new(42));
        assert_eq!(config.listen_addr(), addr(47_001));
        assert_eq!(config.seed_nodes_value(), &["127.0.0.1:47000".to_owned()]);
        assert_eq!(id.node_id.as_ref(), "member-a");
        assert_eq!(id.generation_id, 42);
    }

    #[tokio::test]
    async fn announce_writes_candidate_to_real_chitchat_state() {
        let transport = ChannelTransport::default();
        let discovery =
            ChitchatDiscovery::spawn_with_transport(config(47_011, "member-a"), &transport)
                .await
                .unwrap();

        discovery
            .announce(
                ClusterCandidate::member("member-a")
                    .generation(ClusterGeneration::new(47_011))
                    .endpoints(ClusterEndpoints::new().control("127.0.0.1:7000"))
                    .metadata("zone", "eu"),
            )
            .await
            .unwrap();

        assert_eq!(
            discovery.local_value(KEY_ROLE).await.as_deref(),
            Some("member")
        );
        assert_eq!(
            discovery.local_value(KEY_ENDPOINT_CONTROL).await.as_deref(),
            Some("127.0.0.1:7000")
        );
        assert_eq!(
            discovery
                .local_value(&format!("{KEY_METADATA_PREFIX}zone"))
                .await
                .as_deref(),
            Some("eu")
        );
        assert_eq!(discovery.candidates().len(), 1);
        assert!(matches!(
            discovery.events().first(),
            Some(ClusterDiscoveryEvent::CandidateSeen(candidate))
                if candidate.node_id.as_str() == "member-a"
        ));
    }

    #[tokio::test]
    async fn chitchat_gossip_discovers_remote_candidate() {
        let transport = ChannelTransport::default();
        let first = ChitchatDiscovery::spawn_with_transport(config(47_021, "member-a"), &transport)
            .await
            .unwrap();
        let second = ChitchatDiscovery::spawn_with_transport(
            config(47_022, "client-a").seed_node("127.0.0.1:47021"),
            &transport,
        )
        .await
        .unwrap();

        first
            .announce(
                ClusterCandidate::member("member-a").generation(ClusterGeneration::new(47_021)),
            )
            .await
            .unwrap();
        second
            .announce(
                ClusterCandidate::client("client-a").generation(ClusterGeneration::new(47_022)),
            )
            .await
            .unwrap();
        second.gossip_once(addr(47_021)).unwrap();

        wait_until(Duration::from_secs(2), || {
            first
                .candidates()
                .iter()
                .any(|candidate| candidate.node_id.as_str() == "client-a")
        })
        .await;

        let remote = first
            .candidates()
            .into_iter()
            .find(|candidate| candidate.node_id.as_str() == "client-a")
            .expect("remote candidate should be present");
        assert_eq!(remote.role, ClusterRole::Client);
        assert_eq!(remote.generation, ClusterGeneration::new(47_022));
        assert_eq!(
            remote.metadata.get("discovery.adapter").map(String::as_str),
            Some("chitchat")
        );
    }

    async fn wait_until(timeout_after: Duration, mut condition: impl FnMut() -> bool) {
        timeout(timeout_after, async {
            let started = Instant::now();
            loop {
                if condition() {
                    return;
                }
                assert!(started.elapsed() < timeout_after);
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("condition should become true");
    }
}
