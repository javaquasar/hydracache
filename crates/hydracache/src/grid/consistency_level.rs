use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::ClusterNodeId;
use crate::grid::elasticity::RegionId;
use crate::grid::{EffectiveReplicationMap, ReplicationConfig};

/// Per-operation consistency level for grid reads, writes, and invalidations.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ConsistencyLevel {
    /// One live replica is enough.
    #[default]
    One,
    /// A quorum of replicas in the caller's local region is required.
    LocalQuorum,
    /// A grid-wide quorum is required.
    Quorum,
    /// A quorum in every represented region is required.
    EachQuorum,
    /// Every effective replica must acknowledge.
    All,
}

impl ConsistencyLevel {
    /// Return the default read level implied by the legacy replication config.
    pub fn default_read(config: ReplicationConfig) -> Self {
        Self::from_legacy_quorum(config.read_quorum, config.replication_factor)
    }

    /// Return the default write level implied by the legacy replication config.
    pub fn default_write(config: ReplicationConfig) -> Self {
        Self::from_legacy_quorum(config.write_quorum, config.replication_factor)
    }

    /// Return whether this level can back a single-key linearizable decision.
    pub fn allows_single_key_linearizable(self) -> bool {
        matches!(self, Self::Quorum | Self::EachQuorum | Self::All)
    }

    /// Compute the acknowledgement requirement for this level.
    pub fn required_acks(
        self,
        map: &EffectiveReplicationMap,
        topology: &BTreeMap<ClusterNodeId, RegionId>,
        local_region: &RegionId,
    ) -> AckRequirement {
        let replicas = effective_replicas(map);
        let total_replicas = replicas.len();
        let mut replicas_by_region = BTreeMap::<RegionId, Vec<ClusterNodeId>>::new();
        for node in &replicas {
            let region = topology
                .get(node)
                .cloned()
                .unwrap_or_else(|| local_region.clone());
            replicas_by_region
                .entry(region)
                .or_default()
                .push(node.clone());
        }

        let (required_total, required_per_region) = match self {
            Self::One => (1, BTreeMap::new()),
            Self::LocalQuorum => {
                let local_count = replicas_by_region
                    .get(local_region)
                    .map(Vec::len)
                    .unwrap_or_default();
                (
                    quorum_for(local_count),
                    BTreeMap::from([(local_region.clone(), quorum_for(local_count))]),
                )
            }
            Self::Quorum => (quorum_for(total_replicas), BTreeMap::new()),
            Self::EachQuorum => {
                let per_region = replicas_by_region
                    .iter()
                    .map(|(region, nodes)| (region.clone(), quorum_for(nodes.len())))
                    .collect::<BTreeMap<_, _>>();
                (per_region.values().sum(), per_region)
            }
            Self::All => (total_replicas.max(1), BTreeMap::new()),
        };

        AckRequirement {
            level: self,
            replicas,
            total_replicas,
            required_total,
            required_per_region,
            replicas_by_region,
        }
    }

    /// Return the requirement or a loud unsatisfiable error.
    pub fn validate(
        self,
        map: &EffectiveReplicationMap,
        topology: &BTreeMap<ClusterNodeId, RegionId>,
        local_region: &RegionId,
        live_nodes: &BTreeSet<ClusterNodeId>,
    ) -> Result<AckRequirement, ConsistencyUnsatisfiable> {
        let requirement = self.required_acks(map, topology, local_region);
        if requirement.is_satisfiable(live_nodes) {
            Ok(requirement)
        } else {
            Err(ConsistencyUnsatisfiable::new(
                self,
                requirement.unsatisfied_reason(live_nodes),
            ))
        }
    }

    fn from_legacy_quorum(required: usize, replication_factor: usize) -> Self {
        let required = required.max(1);
        let replication_factor = replication_factor.max(1);
        if required >= replication_factor {
            Self::All
        } else if required > replication_factor / 2 {
            Self::Quorum
        } else {
            Self::One
        }
    }
}

/// Read options carrying the per-operation consistency override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadOptions {
    /// Requested consistency level for this read.
    pub level: ConsistencyLevel,
}

impl ReadOptions {
    /// Build read options with an explicit level.
    pub const fn new(level: ConsistencyLevel) -> Self {
        Self { level }
    }

    /// Build read options from the deployment default.
    pub fn from_config(config: ReplicationConfig) -> Self {
        Self {
            level: ConsistencyLevel::default_read(config),
        }
    }
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            level: ConsistencyLevel::One,
        }
    }
}

/// Write options carrying the per-operation consistency override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteOptions {
    /// Requested consistency level for this write/invalidation.
    pub level: ConsistencyLevel,
}

impl WriteOptions {
    /// Build write options with an explicit level.
    pub const fn new(level: ConsistencyLevel) -> Self {
        Self { level }
    }

    /// Build write options from the deployment default.
    pub fn from_config(config: ReplicationConfig) -> Self {
        Self {
            level: ConsistencyLevel::default_write(config),
        }
    }
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            level: ConsistencyLevel::One,
        }
    }
}

/// Acknowledgement requirement computed from a consistency level and effective placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AckRequirement {
    /// Requested level.
    pub level: ConsistencyLevel,
    /// Effective replica nodes considered by the operation.
    pub replicas: Vec<ClusterNodeId>,
    /// Total number of effective replicas.
    pub total_replicas: usize,
    /// Required acknowledgements across the grid.
    pub required_total: usize,
    /// Required acknowledgements per region for local/each quorum levels.
    pub required_per_region: BTreeMap<RegionId, usize>,
    /// Effective replicas grouped by region.
    pub replicas_by_region: BTreeMap<RegionId, Vec<ClusterNodeId>>,
}

impl AckRequirement {
    /// Return whether the live node set can satisfy this requirement.
    pub fn is_satisfiable(&self, live_nodes: &BTreeSet<ClusterNodeId>) -> bool {
        if self.replicas.is_empty() {
            return false;
        }
        if self.required_per_region.is_empty() {
            return self
                .replicas
                .iter()
                .filter(|node| live_nodes.contains(*node))
                .count()
                >= self.required_total;
        }

        self.required_per_region.iter().all(|(region, required)| {
            self.replicas_by_region
                .get(region)
                .map(|nodes| {
                    nodes
                        .iter()
                        .filter(|node| live_nodes.contains(*node))
                        .count()
                })
                .unwrap_or_default()
                >= *required
        })
    }

    /// Return whether this read/write pair has grid-wide quorum overlap.
    pub fn overlaps_with(&self, other: &Self) -> bool {
        self.required_per_region.is_empty()
            && other.required_per_region.is_empty()
            && self.required_total.saturating_add(other.required_total) > self.total_replicas
    }

    fn unsatisfied_reason(&self, live_nodes: &BTreeSet<ClusterNodeId>) -> String {
        if self.replicas.is_empty() {
            return "no effective replicas are available".to_owned();
        }
        if self.required_per_region.is_empty() {
            let live = self
                .replicas
                .iter()
                .filter(|node| live_nodes.contains(*node))
                .count();
            return format!(
                "requires {} live acknowledgements from {} replicas, only {} are live",
                self.required_total, self.total_replicas, live
            );
        }

        let missing = self
            .required_per_region
            .iter()
            .filter_map(|(region, required)| {
                let live = self
                    .replicas_by_region
                    .get(region)
                    .map(|nodes| {
                        nodes
                            .iter()
                            .filter(|node| live_nodes.contains(*node))
                            .count()
                    })
                    .unwrap_or_default();
                (live < *required)
                    .then(|| format!("{} needs {}, has {}", region.as_str(), required, live))
            })
            .collect::<Vec<_>>();
        format!("regional quorum unsatisfied: {}", missing.join("; "))
    }
}

/// Loud error returned when a requested consistency level cannot be satisfied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyUnsatisfiable {
    /// Requested level.
    pub level: ConsistencyLevel,
    /// Human-readable reason for diagnostics and release gates.
    pub reason: String,
}

impl ConsistencyUnsatisfiable {
    fn new(level: ConsistencyLevel, reason: impl Into<String>) -> Self {
        Self {
            level,
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ConsistencyUnsatisfiable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "consistency level {:?} is unsatisfiable: {}",
            self.level, self.reason
        )
    }
}

impl std::error::Error for ConsistencyUnsatisfiable {}

/// Readiness summary for read-after-write overlap at chosen levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyReadiness {
    /// Computed write requirement.
    pub write: AckRequirement,
    /// Computed read requirement.
    pub read: AckRequirement,
    /// Whether read/write quorums overlap strongly enough for grid RYOW.
    pub read_your_writes_overlap: bool,
}

impl ConsistencyReadiness {
    /// Compute read-after-write overlap for a read/write level pair.
    pub fn evaluate(
        write_level: ConsistencyLevel,
        read_level: ConsistencyLevel,
        map: &EffectiveReplicationMap,
        topology: &BTreeMap<ClusterNodeId, RegionId>,
        local_region: &RegionId,
    ) -> Self {
        let write = write_level.required_acks(map, topology, local_region);
        let read = read_level.required_acks(map, topology, local_region);
        let read_your_writes_overlap = write.overlaps_with(&read);
        Self {
            write,
            read,
            read_your_writes_overlap,
        }
    }
}

fn effective_replicas(map: &EffectiveReplicationMap) -> Vec<ClusterNodeId> {
    let mut seen = BTreeSet::new();
    let mut replicas = Vec::new();
    for node in map
        .natural
        .all_nodes()
        .into_iter()
        .chain(map.reading.iter().cloned())
        .chain(
            map.pending
                .iter()
                .flat_map(|pending| pending.all_nodes())
                .collect::<Vec<_>>(),
        )
    {
        if seen.insert(node.clone()) {
            replicas.push(node);
        }
    }
    replicas
}

fn quorum_for(replica_count: usize) -> usize {
    replica_count / 2 + 1
}
