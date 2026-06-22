use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterDiscoveryEvent {
    /// A candidate was observed by discovery.
    CandidateSeen(ClusterCandidate),
    /// A member appears live.
    MemberLive(ClusterNodeId),
    /// A member or client published an intentional graceful-leave marker.
    MemberLeaving {
        /// Leaving node id.
        node_id: ClusterNodeId,
        /// Generation that published the leave marker.
        generation: ClusterGeneration,
        /// Runtime role that is leaving.
        role: ClusterRole,
    },
    /// A member is suspected unhealthy.
    MemberSuspect(ClusterNodeId),
    /// A member is considered dead.
    MemberDead(ClusterNodeId),
}

/// Transport-neutral discovery contract for cluster candidates and liveness.
///
/// This is the seam where future chitchat, DNS, mDNS, or P2P discovery
/// adapters can plug in. Discovery observes candidates and liveness; it does
/// not make authoritative membership decisions. Admission remains the
/// responsibility of [`ClusterControlPlane`].
#[async_trait::async_trait]
pub trait ClusterDiscovery: fmt::Debug + Send + Sync {
    /// Announce or update a candidate.
    async fn announce(&self, candidate: ClusterCandidate) -> Result<()>;

    /// Record that a node appears live.
    async fn mark_live(&self, node_id: ClusterNodeId) -> Result<()>;

    /// Record that a node is suspected unhealthy.
    async fn mark_suspect(&self, node_id: ClusterNodeId) -> Result<()>;

    /// Record that a node is considered dead.
    async fn mark_dead(&self, node_id: ClusterNodeId) -> Result<()>;

    /// Return the latest candidate snapshot for every discovered node id.
    fn candidates(&self) -> Vec<ClusterCandidate>;

    /// Return discovery events recorded by this adapter.
    fn events(&self) -> Vec<ClusterDiscoveryEvent>;
}

#[derive(Debug, Default)]
struct InMemoryClusterDiscoveryState {
    candidates: BTreeMap<ClusterNodeId, ClusterCandidate>,
    events: Vec<ClusterDiscoveryEvent>,
}

/// In-memory discovery journal for tests, demos, and future adapter contracts.
///
/// `InMemoryClusterDiscovery` models the chitchat side of the design without
/// depending on chitchat yet: nodes first become visible as candidates with
/// metadata, endpoints, role, and generation; authoritative admission remains
/// the responsibility of [`InMemoryCluster`].
#[derive(Debug, Default)]
pub struct InMemoryClusterDiscovery {
    state: Mutex<InMemoryClusterDiscoveryState>,
}

impl InMemoryClusterDiscovery {
    /// Create an empty in-memory discovery journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Announce or update a candidate.
    pub fn announce(&self, candidate: ClusterCandidate) {
        let mut state = self.state.lock().expect("cluster discovery poisoned");
        state
            .candidates
            .insert(candidate.node_id.clone(), candidate.clone());
        state
            .events
            .push(ClusterDiscoveryEvent::CandidateSeen(candidate));
    }

    /// Record that a node appears live.
    pub fn mark_live(&self, node_id: impl Into<ClusterNodeId>) {
        self.push_liveness(ClusterDiscoveryEvent::MemberLive(node_id.into()));
    }

    /// Record that a node is suspected unhealthy.
    pub fn mark_suspect(&self, node_id: impl Into<ClusterNodeId>) {
        self.push_liveness(ClusterDiscoveryEvent::MemberSuspect(node_id.into()));
    }

    /// Record that a node is considered dead.
    pub fn mark_dead(&self, node_id: impl Into<ClusterNodeId>) {
        self.push_liveness(ClusterDiscoveryEvent::MemberDead(node_id.into()));
    }

    fn push_liveness(&self, event: ClusterDiscoveryEvent) {
        self.state
            .lock()
            .expect("cluster discovery poisoned")
            .events
            .push(event);
    }

    /// Return the latest candidate snapshot for every discovered node id.
    pub fn candidates(&self) -> Vec<ClusterCandidate> {
        self.state
            .lock()
            .expect("cluster discovery poisoned")
            .candidates
            .values()
            .cloned()
            .collect()
    }

    /// Return discovery events recorded by the in-memory journal.
    pub fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        self.state
            .lock()
            .expect("cluster discovery poisoned")
            .events
            .clone()
    }
}

#[async_trait::async_trait]
impl ClusterDiscovery for InMemoryClusterDiscovery {
    async fn announce(&self, candidate: ClusterCandidate) -> Result<()> {
        InMemoryClusterDiscovery::announce(self, candidate);
        Ok(())
    }

    async fn mark_live(&self, node_id: ClusterNodeId) -> Result<()> {
        InMemoryClusterDiscovery::mark_live(self, node_id);
        Ok(())
    }

    async fn mark_suspect(&self, node_id: ClusterNodeId) -> Result<()> {
        InMemoryClusterDiscovery::mark_suspect(self, node_id);
        Ok(())
    }

    async fn mark_dead(&self, node_id: ClusterNodeId) -> Result<()> {
        InMemoryClusterDiscovery::mark_dead(self, node_id);
        Ok(())
    }

    fn candidates(&self) -> Vec<ClusterCandidate> {
        InMemoryClusterDiscovery::candidates(self)
    }

    fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        InMemoryClusterDiscovery::events(self)
    }
}

/// Dependency-free, chitchat-style discovery adapter for tests and API spikes.
///
/// This adapter intentionally does not run the real `chitchat` network
/// protocol yet. It models the part of chitchat that matters to HydraCache's
/// public cluster API: a node starts with seed addresses, announces itself as a
/// candidate, and records liveness transitions separately from authoritative
/// control-plane admission.
///
/// Candidate announcements are stored in-memory and annotated with adapter
/// metadata so tests, diagnostics, and the sandbox can distinguish this path
/// from the plain [`InMemoryClusterDiscovery`] journal.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use hydracache::{ChitchatStyleDiscovery, HydraCache, InMemoryCluster};
///
/// # #[tokio::main]
/// # async fn main() -> hydracache::CacheResult<()> {
/// let cluster = Arc::new(InMemoryCluster::new("orders"));
/// let discovery = Arc::new(ChitchatStyleDiscovery::new([
///     "127.0.0.1:7000",
///     "127.0.0.1:7001",
/// ]));
///
/// let member = HydraCache::member()
///     .shared_cluster(cluster)
///     .discovery(discovery.clone())
///     .node_id("member-a")
///     .start()
///     .await?;
///
/// assert_eq!(discovery.seed_count(), 2);
/// assert_eq!(discovery.candidates().len(), 1);
/// assert!(member.cluster_discovery_diagnostics().unwrap().has_candidates());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ChitchatStyleDiscovery {
    seeds: Vec<String>,
    inner: InMemoryClusterDiscovery,
}

impl ChitchatStyleDiscovery {
    /// Create a chitchat-style discovery journal with seed addresses.
    pub fn new<I, S>(seeds: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            seeds: seeds.into_iter().map(Into::into).collect(),
            inner: InMemoryClusterDiscovery::new(),
        }
    }

    /// Return the static seed addresses used to bootstrap discovery.
    pub fn seeds(&self) -> &[String] {
        &self.seeds
    }

    /// Return the number of configured seed addresses.
    pub fn seed_count(&self) -> usize {
        self.seeds.len()
    }

    /// Return whether the adapter has at least one seed address.
    pub fn has_seeds(&self) -> bool {
        !self.seeds.is_empty()
    }

    /// Return the adapter label attached to candidate metadata.
    pub fn adapter_name(&self) -> &'static str {
        "chitchat-style"
    }

    /// Announce or update a candidate with chitchat-style metadata.
    pub fn announce(&self, mut candidate: ClusterCandidate) {
        candidate
            .metadata
            .entry("discovery.adapter".to_owned())
            .or_insert_with(|| self.adapter_name().to_owned());
        if self.has_seeds() {
            candidate
                .metadata
                .entry("discovery.seeds".to_owned())
                .or_insert_with(|| self.seeds.join(","));
        }
        self.inner.announce(candidate);
    }

    /// Record that a node appears live.
    pub fn mark_live(&self, node_id: impl Into<ClusterNodeId>) {
        self.inner.mark_live(node_id);
    }

    /// Record that a node is suspected unhealthy.
    pub fn mark_suspect(&self, node_id: impl Into<ClusterNodeId>) {
        self.inner.mark_suspect(node_id);
    }

    /// Record that a node is considered dead.
    pub fn mark_dead(&self, node_id: impl Into<ClusterNodeId>) {
        self.inner.mark_dead(node_id);
    }

    /// Return the latest candidate snapshot for every discovered node id.
    pub fn candidates(&self) -> Vec<ClusterCandidate> {
        self.inner.candidates()
    }

    /// Return discovery events recorded by the adapter.
    pub fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        self.inner.events()
    }
}

impl Default for ChitchatStyleDiscovery {
    fn default() -> Self {
        Self::new(std::iter::empty::<String>())
    }
}

#[async_trait::async_trait]
impl ClusterDiscovery for ChitchatStyleDiscovery {
    async fn announce(&self, candidate: ClusterCandidate) -> Result<()> {
        ChitchatStyleDiscovery::announce(self, candidate);
        Ok(())
    }

    async fn mark_live(&self, node_id: ClusterNodeId) -> Result<()> {
        ChitchatStyleDiscovery::mark_live(self, node_id);
        Ok(())
    }

    async fn mark_suspect(&self, node_id: ClusterNodeId) -> Result<()> {
        ChitchatStyleDiscovery::mark_suspect(self, node_id);
        Ok(())
    }

    async fn mark_dead(&self, node_id: ClusterNodeId) -> Result<()> {
        ChitchatStyleDiscovery::mark_dead(self, node_id);
        Ok(())
    }

    fn candidates(&self) -> Vec<ClusterCandidate> {
        ChitchatStyleDiscovery::candidates(self)
    }

    fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        ChitchatStyleDiscovery::events(self)
    }
}
