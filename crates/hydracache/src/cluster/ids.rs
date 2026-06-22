use super::*;

/// Stable logical id for a HydraCache cluster participant.
///
/// The id is separate from transport-level identities. A future libp2p adapter
/// can map this value to a `PeerId`, while a server deployment can map it to a
/// configured node name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClusterNodeId(String);

impl ClusterNodeId {
    /// Create a node id from an application-defined string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the node id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ClusterNodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for ClusterNodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ClusterNodeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Monotonic process generation for a cluster node id.
///
/// A restarted process should use a larger generation than the previous
/// process. This lets the cluster reject stale clients or members that still
/// emit invalidation messages after a restart.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ClusterGeneration(u64);

impl ClusterGeneration {
    /// Create a generation from a numeric value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw generation value.
    pub fn value(self) -> u64 {
        self.0
    }

    /// Return the next generation value.
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Committed cluster metadata epoch.
///
/// In v0.20 this is simulated by [`InMemoryCluster`]. A future Raft-backed
/// adapter should advance this value only after committed membership changes.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ClusterEpoch(u64);

impl ClusterEpoch {
    /// Create an epoch from a numeric value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw epoch value.
    pub fn value(self) -> u64 {
        self.0
    }

    pub(super) fn advance(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

/// Runtime role of a HydraCache instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterRole {
    /// No distributed behavior.
    Local,
    /// Application-side near-cache connected to a cluster.
    Client,
    /// Cluster participant that routes invalidations and later owns metadata.
    Member,
}

impl ClusterRole {
    /// Return whether this role is allowed to vote in future Raft metadata.
    pub fn can_vote(self) -> bool {
        matches!(self, Self::Member)
    }
}
