//! Ergonomic cluster composition helpers for HydraCache.
//!
//! This crate intentionally lives outside the base `hydracache` crate so
//! local-only applications do not pull in chitchat or raft-rs dependencies.
//! It is a convenience layer over public HydraCache traits and builders.
//!
//! # Example
//!
//! ```no_run
//! use hydracache::{ClusterGeneration, HydraCache};
//! use hydracache_cluster::HydraCluster;
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cluster = HydraCluster::builder("orders")
//!     .node_id("member-a")
//!     .generation(ClusterGeneration::new(1))
//!     .chitchat_udp("127.0.0.1:7000")
//!     .seed("127.0.0.1:7001")
//!     .raft_single_node(1)
//!     .build()
//!     .await?;
//!
//! let member = cluster.member_cache().start().await?;
//!
//! assert_eq!(member.cluster_diagnostics().unwrap().cluster_name, "orders");
//! # Ok(())
//! # }
//! ```

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hydracache::{
    CacheError, CacheResult, ClusterAdmissionBridge, ClusterControlPlane, ClusterDiscovery,
    ClusterGeneration, ClusterNodeId, HydraCache, HydraCacheClientBuilder, HydraCacheMemberBuilder,
};
use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
use hydracache_cluster_raft::RaftMetadataRuntime;

/// Builder entry point for a composed HydraCache cluster runtime.
#[derive(Debug, Clone)]
pub struct HydraClusterBuilder {
    cluster_name: String,
    node_id: Option<ClusterNodeId>,
    generation: ClusterGeneration,
    bootstrap: Vec<String>,
    chitchat_udp: Option<String>,
    chitchat_seeds: Vec<String>,
    chitchat_gossip_interval: Option<Duration>,
    raft_node_id: u64,
}

impl HydraClusterBuilder {
    fn new(cluster_name: impl Into<String>) -> Self {
        Self {
            cluster_name: cluster_name.into(),
            node_id: None,
            generation: ClusterGeneration::new(1),
            bootstrap: Vec::new(),
            chitchat_udp: None,
            chitchat_seeds: Vec::new(),
            chitchat_gossip_interval: None,
            raft_node_id: 1,
        }
    }

    /// Set the logical HydraCache node id used by the member/client builder.
    pub fn node_id(mut self, node_id: impl Into<ClusterNodeId>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    /// Set the process generation used by the member/client builder.
    pub fn generation(mut self, generation: ClusterGeneration) -> Self {
        self.generation = generation;
        self
    }

    /// Enable real chitchat UDP discovery.
    ///
    /// The address is parsed during [`build`](Self::build), so callers can keep
    /// a fluent builder chain with string literals.
    pub fn chitchat_udp(mut self, listen_addr: impl Into<String>) -> Self {
        self.chitchat_udp = Some(listen_addr.into());
        self
    }

    /// Add one chitchat seed and also record it as bootstrap diagnostics.
    pub fn seed(mut self, seed: impl Into<String>) -> Self {
        let seed = seed.into();
        self.bootstrap.push(seed.clone());
        self.chitchat_seeds.push(seed);
        self
    }

    /// Set the chitchat gossip interval.
    pub fn gossip_interval(mut self, interval: Duration) -> Self {
        self.chitchat_gossip_interval = Some(interval);
        self
    }

    /// Select a single-node raft-rs metadata runtime.
    pub fn raft_single_node(mut self, raft_node_id: u64) -> Self {
        self.raft_node_id = raft_node_id.max(1);
        self
    }

    /// Build the composed cluster handles.
    pub async fn build(self) -> CacheResult<HydraCluster> {
        let node_id = self
            .node_id
            .unwrap_or_else(|| ClusterNodeId::from("hydracache-node"));
        let control_plane = Arc::new(RaftMetadataRuntime::single_node(
            self.cluster_name.clone(),
            self.raft_node_id,
        )?);
        let discovery = match self.chitchat_udp {
            Some(addr) => {
                let addr = parse_socket_addr(&addr)?;
                let mut config = ChitchatDiscoveryConfig::new(
                    self.cluster_name.clone(),
                    node_id.clone(),
                    self.generation,
                    addr,
                )
                .seed_nodes(self.chitchat_seeds);
                if let Some(interval) = self.chitchat_gossip_interval {
                    config = config.gossip_interval(interval);
                }
                Some(Arc::new(ChitchatDiscovery::spawn_udp(config).await?))
            }
            None => None,
        };

        Ok(HydraCluster {
            cluster_name: self.cluster_name,
            node_id,
            generation: self.generation,
            bootstrap: self.bootstrap,
            discovery,
            control_plane,
        })
    }
}

/// Composed optional cluster handles.
#[derive(Debug, Clone)]
pub struct HydraCluster {
    cluster_name: String,
    node_id: ClusterNodeId,
    generation: ClusterGeneration,
    bootstrap: Vec<String>,
    discovery: Option<Arc<ChitchatDiscovery>>,
    control_plane: Arc<RaftMetadataRuntime>,
}

impl HydraCluster {
    /// Start building a composed cluster runtime.
    pub fn builder(cluster_name: impl Into<String>) -> HydraClusterBuilder {
        HydraClusterBuilder::new(cluster_name)
    }

    /// Return the configured logical cluster name.
    pub fn cluster_name(&self) -> &str {
        &self.cluster_name
    }

    /// Return the configured node id.
    pub fn node_id(&self) -> &ClusterNodeId {
        &self.node_id
    }

    /// Return the configured generation.
    pub fn generation(&self) -> ClusterGeneration {
        self.generation
    }

    /// Return the raft metadata runtime.
    pub fn raft(&self) -> Arc<RaftMetadataRuntime> {
        self.control_plane.clone()
    }

    /// Return the control plane as a trait object.
    pub fn control_plane(&self) -> Arc<dyn ClusterControlPlane> {
        self.control_plane.clone()
    }

    /// Return the chitchat discovery adapter, when configured.
    pub fn chitchat(&self) -> Option<Arc<ChitchatDiscovery>> {
        self.discovery.clone()
    }

    /// Return discovery as a trait object, when configured.
    pub fn discovery(&self) -> Option<Arc<dyn ClusterDiscovery>> {
        self.discovery
            .as_ref()
            .map(|discovery| discovery.clone() as Arc<dyn ClusterDiscovery>)
    }

    /// Create an admission bridge from configured discovery to raft metadata.
    pub fn admission_bridge(&self) -> Option<ClusterAdmissionBridge> {
        self.discovery()
            .map(|discovery| ClusterAdmissionBridge::new(discovery, self.control_plane()))
    }

    /// Return a member cache builder wired to this cluster.
    pub fn member_cache(&self) -> HydraCacheMemberBuilder {
        let mut builder = HydraCache::member()
            .cluster(self.cluster_name.clone())
            .control_plane(self.control_plane())
            .node_id(self.node_id.clone())
            .generation(self.generation);
        if let Some(discovery) = self.discovery() {
            builder = builder.discovery(discovery);
        }
        for bootstrap in &self.bootstrap {
            builder = builder.bootstrap(bootstrap.clone());
        }
        builder
    }

    /// Return a client near-cache builder wired to this cluster.
    pub fn client_cache(&self) -> HydraCacheClientBuilder {
        let mut builder = HydraCache::client()
            .cluster(self.cluster_name.clone())
            .control_plane(self.control_plane())
            .node_id(self.node_id.clone())
            .generation(self.generation);
        if let Some(discovery) = self.discovery() {
            builder = builder.discovery(discovery);
        }
        for bootstrap in &self.bootstrap {
            builder = builder.bootstrap(bootstrap.clone());
        }
        builder
    }
}

fn parse_socket_addr(value: &str) -> CacheResult<SocketAddr> {
    value.parse::<SocketAddr>().map_err(|error| {
        CacheError::Backend(format!("invalid chitchat UDP address '{value}': {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builder_creates_member_cache_with_raft_control_plane() {
        let cluster = HydraCluster::builder("orders")
            .node_id("member-a")
            .generation(ClusterGeneration::new(3))
            .raft_single_node(1)
            .build()
            .await
            .unwrap();

        let member = cluster.member_cache().start().await.unwrap();
        let diagnostics = member.cluster_diagnostics().unwrap();

        assert_eq!(cluster.cluster_name(), "orders");
        assert_eq!(cluster.node_id().as_str(), "member-a");
        assert_eq!(cluster.generation(), ClusterGeneration::new(3));
        assert_eq!(diagnostics.cluster_name, "orders");
        assert_eq!(diagnostics.node_id.as_str(), "member-a");
        assert_eq!(cluster.raft().snapshot().commands_committed, 1);
        assert!(cluster.admission_bridge().is_none());
    }

    #[tokio::test]
    async fn builder_creates_client_cache_with_raft_control_plane() {
        let cluster = HydraCluster::builder("orders")
            .node_id("client-a")
            .generation(ClusterGeneration::new(1))
            .build()
            .await
            .unwrap();

        let client = cluster.client_cache().connect().await.unwrap();
        let diagnostics = client.cluster_diagnostics().unwrap();

        assert_eq!(diagnostics.client_count, 1);
        assert_eq!(diagnostics.node_id.as_str(), "client-a");
        assert_eq!(cluster.raft().snapshot().commands_committed, 1);
    }

    #[tokio::test]
    async fn builder_can_start_chitchat_discovery_on_ephemeral_udp_port() {
        let cluster = HydraCluster::builder("orders")
            .node_id("member-a")
            .generation(ClusterGeneration::new(1))
            .chitchat_udp("127.0.0.1:0")
            .seed("127.0.0.1:7001")
            .gossip_interval(Duration::from_millis(20))
            .build()
            .await
            .unwrap();

        assert!(cluster.chitchat().is_some());
        assert!(cluster.discovery().is_some());
        assert!(cluster.admission_bridge().is_some());
    }

    #[tokio::test]
    async fn builder_rejects_invalid_chitchat_udp_address() {
        let error = HydraCluster::builder("orders")
            .node_id("member-a")
            .chitchat_udp("not-an-address")
            .build()
            .await
            .unwrap_err();

        assert!(error.to_string().contains("invalid chitchat UDP address"));
    }
}
