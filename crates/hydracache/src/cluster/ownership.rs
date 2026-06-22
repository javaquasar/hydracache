use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterOwnershipDecision {
    /// Logical cache key used for the lookup.
    pub key: String,
    /// Owner selected by the resolver, if at least one member is eligible.
    pub owner: Option<ClusterMember>,
    /// Number of eligible member nodes considered by the resolver.
    pub member_count: usize,
    /// Stable resolver name for diagnostics and sandbox reports.
    pub resolver: &'static str,
}

impl ClusterOwnershipDecision {
    /// Return whether an owner was selected.
    pub fn has_owner(&self) -> bool {
        self.owner.is_some()
    }

    /// Return the selected owner node id.
    pub fn owner_node_id(&self) -> Option<&ClusterNodeId> {
        self.owner.as_ref().map(|owner| &owner.node_id)
    }

    /// Return the selected owner generation.
    pub fn owner_generation(&self) -> Option<ClusterGeneration> {
        self.owner.as_ref().map(|owner| owner.generation)
    }

    /// Build a peer-fetch request for this decision, if it has an owner.
    pub fn peer_fetch_request(&self) -> Option<ClusterPeerFetchRequest> {
        self.owner.as_ref().map(|owner| {
            ClusterPeerFetchRequest::new(owner.node_id.clone(), self.key.clone())
                .generation(owner.generation)
        })
    }
}

/// Strategy for mapping cache keys to admitted cluster members.
///
/// This trait is intentionally value-agnostic. It decides ownership only; a
/// later peer-fetch layer can use the decision to contact the owner.
pub trait ClusterOwnershipResolver: Send + Sync {
    /// Stable resolver name for diagnostics.
    fn name(&self) -> &'static str;

    /// Resolve the owner for `key` among the provided participants.
    fn resolve_owner(&self, key: &str, participants: &[ClusterMember]) -> ClusterOwnershipDecision;
}

/// Deterministic rendezvous-style ownership resolver.
///
/// The resolver scores each admitted member by hashing `key` with the member
/// node id and picks the highest score. It ignores clients and local roles.
#[derive(Debug, Clone, Copy, Default)]
pub struct RendezvousClusterOwnership;

impl ClusterOwnershipResolver for RendezvousClusterOwnership {
    fn name(&self) -> &'static str {
        "rendezvous"
    }

    fn resolve_owner(&self, key: &str, participants: &[ClusterMember]) -> ClusterOwnershipDecision {
        let mut member_count = 0_usize;
        let mut best: Option<(u64, ClusterMember)> = None;

        for participant in participants
            .iter()
            .filter(|candidate| candidate.is_member())
        {
            member_count = member_count.saturating_add(1);
            let score = rendezvous_score(key, &participant.node_id);
            let replace = best
                .as_ref()
                .map(|(best_score, best_member)| {
                    score > *best_score
                        || (score == *best_score && participant.node_id > best_member.node_id)
                })
                .unwrap_or(true);
            if replace {
                best = Some((score, participant.clone()));
            }
        }

        ClusterOwnershipDecision {
            key: key.to_owned(),
            owner: best.map(|(_, member)| member),
            member_count,
            resolver: self.name(),
        }
    }
}

fn rendezvous_score(key: &str, node_id: &ClusterNodeId) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in key.bytes().chain([0xff]).chain(node_id.as_str().bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Stable partition id used as a thin indirection over rendezvous ownership.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct PartitionId(u32);

impl PartitionId {
    /// Create a partition id from its numeric value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the numeric partition id.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Return the deterministic partition for a key.
///
/// A zero `partition_count` is normalized to one partition so callers cannot
/// accidentally divide by zero while validating a partially built config.
pub fn partition_for_key(key: impl AsRef<str>, partition_count: u32) -> PartitionId {
    let partition_count = partition_count.max(1);
    PartitionId((stable_key_hash(key.as_ref()) % u64::from(partition_count)) as u32)
}

fn stable_key_hash(key: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in key.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Replica/quorum pilot configuration error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterReplicaConfigError {
    /// `min_replica` must be at least one.
    MinReplicaZero,
    /// `quorum` must be at least one.
    QuorumZero,
    /// `quorum` cannot exceed `replication_factor`.
    QuorumExceedsReplication,
}

impl fmt::Display for ClusterReplicaConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MinReplicaZero => "min_replica must be at least 1",
            Self::QuorumZero => "quorum must be at least 1",
            Self::QuorumExceedsReplication => "quorum cannot exceed replication_factor",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ClusterReplicaConfigError {}

/// Validate the narrow 0.40 pilot replica/quorum shape.
///
/// Value replication still lands in a later release, but validating these
/// values now makes invalid future topology config fail early.
pub fn validate_replica_config(
    min_replica: usize,
    replication_factor: usize,
    quorum: usize,
) -> std::result::Result<(), ClusterReplicaConfigError> {
    if min_replica < 1 {
        return Err(ClusterReplicaConfigError::MinReplicaZero);
    }
    if quorum == 0 {
        return Err(ClusterReplicaConfigError::QuorumZero);
    }
    if quorum > replication_factor {
        return Err(ClusterReplicaConfigError::QuorumExceedsReplication);
    }
    Ok(())
}
