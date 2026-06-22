use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClusterEndpoints {
    /// Control endpoint for future member/client protocol requests.
    pub control: Option<String>,
    /// Invalidation endpoint used by a future external bus.
    pub invalidation: Option<String>,
    /// Diagnostics or actuator endpoint.
    pub diagnostics: Option<String>,
}

impl ClusterEndpoints {
    /// Create an empty endpoint set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the control endpoint.
    pub fn control(mut self, endpoint: impl Into<String>) -> Self {
        self.control = Some(endpoint.into());
        self
    }

    /// Set the invalidation endpoint.
    pub fn invalidation(mut self, endpoint: impl Into<String>) -> Self {
        self.invalidation = Some(endpoint.into());
        self
    }

    /// Set the diagnostics endpoint.
    pub fn diagnostics(mut self, endpoint: impl Into<String>) -> Self {
        self.diagnostics = Some(endpoint.into());
        self
    }
}

/// Candidate discovered before authoritative membership admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterCandidate {
    /// Candidate node id.
    pub node_id: ClusterNodeId,
    /// Candidate process generation.
    pub generation: ClusterGeneration,
    /// Requested runtime role.
    pub role: ClusterRole,
    /// Advertised endpoints.
    pub endpoints: ClusterEndpoints,
    /// Small metadata map for future discovery adapters.
    pub metadata: BTreeMap<String, String>,
}

impl ClusterCandidate {
    /// Create a member candidate.
    pub fn member(node_id: impl Into<ClusterNodeId>) -> Self {
        Self::new(node_id, ClusterRole::Member)
    }

    /// Create a client candidate.
    pub fn client(node_id: impl Into<ClusterNodeId>) -> Self {
        Self::new(node_id, ClusterRole::Client)
    }

    fn new(node_id: impl Into<ClusterNodeId>, role: ClusterRole) -> Self {
        Self {
            node_id: node_id.into(),
            generation: ClusterGeneration::default(),
            role,
            endpoints: ClusterEndpoints::default(),
            metadata: BTreeMap::new(),
        }
    }

    /// Set the candidate generation.
    pub fn generation(mut self, generation: ClusterGeneration) -> Self {
        self.generation = generation;
        self
    }

    /// Set advertised endpoints.
    pub fn endpoints(mut self, endpoints: ClusterEndpoints) -> Self {
        self.endpoints = endpoints;
        self
    }

    /// Add one metadata entry.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Advertise the base URL used by peer-fetch transports.
    ///
    /// The URL should not include the concrete peer-fetch path. For example,
    /// use `http://127.0.0.1:3000`, not
    /// `http://127.0.0.1:3000/cluster/peer-fetch`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{ClusterCandidate, CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY};
    ///
    /// let candidate = ClusterCandidate::member("member-a")
    ///     .peer_fetch_base_url("http://127.0.0.1:3000");
    ///
    /// assert_eq!(
    ///     candidate.peer_fetch_base_url_value(),
    ///     Some("http://127.0.0.1:3000")
    /// );
    /// assert_eq!(
    ///     candidate
    ///         .metadata
    ///         .get(CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY)
    ///         .map(String::as_str),
    ///     Some("http://127.0.0.1:3000")
    /// );
    /// ```
    pub fn peer_fetch_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.metadata.insert(
            CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY.to_owned(),
            base_url.into(),
        );
        self
    }

    /// Return the advertised peer-fetch base URL, when present.
    pub fn peer_fetch_base_url_value(&self) -> Option<&str> {
        self.metadata
            .get(CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY)
            .map(String::as_str)
    }
}

/// Admitted cluster participant snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterMember {
    /// Admitted node id.
    pub node_id: ClusterNodeId,
    /// Admitted process generation.
    pub generation: ClusterGeneration,
    /// Runtime role.
    pub role: ClusterRole,
    /// Cluster epoch observed when this participant was admitted.
    pub epoch: ClusterEpoch,
    /// Advertised endpoints.
    pub endpoints: ClusterEndpoints,
    /// Metadata carried from discovery.
    pub metadata: BTreeMap<String, String>,
}

impl ClusterMember {
    pub(super) fn from_candidate(candidate: ClusterCandidate, epoch: ClusterEpoch) -> Self {
        Self {
            node_id: candidate.node_id,
            generation: candidate.generation,
            role: candidate.role,
            epoch,
            endpoints: candidate.endpoints,
            metadata: candidate.metadata,
        }
    }

    /// Return whether this member is a client near-cache.
    pub fn is_client(&self) -> bool {
        self.role == ClusterRole::Client
    }

    /// Return whether this member is a cluster member node.
    pub fn is_member(&self) -> bool {
        self.role == ClusterRole::Member
    }

    /// Return the advertised peer-fetch base URL, when present.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{
    ///     ClusterCandidate, ClusterControlPlane, InMemoryCluster,
    /// };
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cluster = InMemoryCluster::new("orders");
    /// let member = ClusterControlPlane::join_member(
    ///     &cluster,
    ///     ClusterCandidate::member("member-a")
    ///         .peer_fetch_base_url("http://127.0.0.1:3000"),
    /// )
    /// .await?;
    ///
    /// assert_eq!(
    ///     member.peer_fetch_base_url(),
    ///     Some("http://127.0.0.1:3000")
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn peer_fetch_base_url(&self) -> Option<&str> {
        self.metadata
            .get(CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY)
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterMembershipEvent {
    /// A member node joined or was updated.
    MemberJoined(ClusterMember),
    /// A client near-cache connected or was updated.
    ClientConnected(ClusterMember),
    /// A node left the in-memory cluster model.
    NodeLeft {
        /// Node id.
        node_id: ClusterNodeId,
        /// Role before leaving.
        role: ClusterRole,
        /// Epoch after the leave operation.
        epoch: ClusterEpoch,
    },
    /// A stale process generation was rejected.
    StaleGenerationRejected {
        /// Rejected node id.
        node_id: ClusterNodeId,
        /// Runtime role associated with the rejected generation.
        role: ClusterRole,
        /// Existing accepted generation.
        existing: ClusterGeneration,
        /// Attempted stale generation.
        attempted: ClusterGeneration,
        /// Machine-friendly rejection reason.
        reason: String,
    },
}

/// Error returned by [`ClusterMembershipSubscriber::recv`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterMembershipRecvError {
    /// The membership event stream has been closed.
    Closed,
    /// The subscriber lagged behind the bounded event stream.
    Lagged(u64),
}

impl fmt::Display for ClusterMembershipRecvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("cluster membership subscription closed"),
            Self::Lagged(skipped) => {
                write!(
                    formatter,
                    "cluster membership subscriber lagged by {skipped} events"
                )
            }
        }
    }
}

impl std::error::Error for ClusterMembershipRecvError {}

/// Receiver for cluster membership events from a control plane.
///
/// The stream is intentionally bounded. Admission and leave operations never
/// wait for slow subscribers; slow consumers receive
/// [`ClusterMembershipRecvError::Lagged`] and can decide whether to rebuild
/// their view from diagnostics/snapshots.
#[derive(Debug)]
pub struct ClusterMembershipSubscriber {
    receiver: broadcast::Receiver<ClusterMembershipEvent>,
}

impl ClusterMembershipSubscriber {
    fn new(receiver: broadcast::Receiver<ClusterMembershipEvent>) -> Self {
        Self { receiver }
    }

    pub(super) fn closed() -> Self {
        let (sender, receiver) = broadcast::channel(1);
        drop(sender);
        Self { receiver }
    }

    /// Receive the next membership event.
    pub async fn recv(
        &mut self,
    ) -> std::result::Result<ClusterMembershipEvent, ClusterMembershipRecvError> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Closed) => Err(ClusterMembershipRecvError::Closed),
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                Err(ClusterMembershipRecvError::Lagged(skipped))
            }
        }
    }

    /// Receive the next event, skipping lag notifications.
    pub async fn next_event(&mut self) -> Option<ClusterMembershipEvent> {
        loop {
            match self.recv().await {
                Ok(event) => return Some(event),
                Err(ClusterMembershipRecvError::Closed) => return None,
                Err(ClusterMembershipRecvError::Lagged(_)) => continue,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ClusterMembershipEventBus {
    sender: broadcast::Sender<ClusterMembershipEvent>,
}

impl ClusterMembershipEventBus {
    pub(super) fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    pub(super) fn publish(&self, event: ClusterMembershipEvent) {
        let _ = self.sender.send(event);
    }

    pub(super) fn subscribe(&self) -> ClusterMembershipSubscriber {
        ClusterMembershipSubscriber::new(self.sender.subscribe())
    }

    pub(super) fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for ClusterMembershipEventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}
