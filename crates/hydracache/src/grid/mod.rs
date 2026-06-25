use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub(crate) mod active_active;
pub(crate) mod capacity;
pub(crate) mod causal_consistency;
pub(crate) mod conditional;
pub(crate) mod consistency_level;
pub(crate) mod convergence_staleness;
pub(crate) mod crdt;
pub(crate) mod durability;
#[cfg(feature = "durable-value-store")]
pub(crate) mod durable_store;
pub(crate) mod elasticity;
pub(crate) mod failure_detector;
pub(crate) mod hardening;
pub(crate) mod hinted_handoff;
pub(crate) mod invalidation_ring;
pub(crate) mod merkle_repair;
pub(crate) mod persistence_config;
pub(crate) mod persistence_policy;
pub(crate) mod recovery;
pub(crate) mod region_failover;
pub(crate) mod region_link;
pub(crate) mod residency;
pub(crate) mod session_context;
pub(crate) mod session_lifecycle;
pub(crate) mod session_monotonic;
pub(crate) mod session_ryw;

use crate::cluster::{
    ClusterEpoch, ClusterGeneration, ClusterMember, ClusterNodeId, PartitionId,
    RendezvousClusterOwnership,
};

/// Strategy for selecting primary and backup owners for a cache key.
pub trait ClusterReplicationStrategy: Send + Sync {
    /// Stable strategy name used in diagnostics.
    fn name(&self) -> &'static str;

    /// Return the primary plus up to `replication_factor - 1` backup owners.
    fn replicas_for_key(
        &self,
        key: &str,
        members: &[ClusterMember],
        replication_factor: usize,
    ) -> Option<Replicas>;
}

/// Primary plus backup owners for one replicated key or partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replicas {
    /// Primary owner selected by rendezvous ranking.
    pub primary: ClusterNodeId,
    /// Backup owners in deterministic rendezvous order.
    pub backups: Vec<ClusterNodeId>,
}

impl Replicas {
    /// Build a replica set and remove accidental duplicate backups.
    pub fn new(primary: impl Into<ClusterNodeId>, backups: Vec<ClusterNodeId>) -> Self {
        let primary = primary.into();
        let mut seen = BTreeSet::new();
        let backups = backups
            .into_iter()
            .filter(|backup| backup != &primary)
            .filter(|backup| seen.insert(backup.clone()))
            .collect();
        Self { primary, backups }
    }

    /// Return every readable owner, primary first.
    pub fn all_nodes(&self) -> Vec<ClusterNodeId> {
        let mut nodes = Vec::with_capacity(1 + self.backups.len());
        nodes.push(self.primary.clone());
        nodes.extend(self.backups.iter().cloned());
        nodes
    }

    /// Return the number of physical copies represented by this set.
    pub fn copy_count(&self) -> usize {
        1 + self.backups.len()
    }
}

/// Effective placement used while rebalance is in flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveReplicationMap {
    /// Current committed natural placement.
    pub natural: Replicas,
    /// Readable owners during a move window.
    pub reading: Vec<ClusterNodeId>,
    /// In-flight placement, if a rebalance is moving ownership.
    pub pending: Option<Replicas>,
}

impl EffectiveReplicationMap {
    /// Create a map without pending movement.
    pub fn new(natural: Replicas) -> Self {
        let reading = natural.all_nodes();
        Self {
            natural,
            reading,
            pending: None,
        }
    }

    /// Create a map that reads from both committed and pending owners.
    pub fn with_pending(natural: Replicas, pending: Replicas) -> Self {
        let mut reading = natural.all_nodes();
        for node in pending.all_nodes() {
            if !reading.contains(&node) {
                reading.push(node);
            }
        }
        Self {
            natural,
            reading,
            pending: Some(pending),
        }
    }

    /// Return whether `node_id` belongs to the read set.
    pub fn is_readable_from(&self, node_id: &ClusterNodeId) -> bool {
        self.reading.iter().any(|node| node == node_id)
    }
}

/// Replication/quorum configuration for the 0.41 grid slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Total number of desired copies, including the primary.
    pub replication_factor: usize,
    /// Number of replicas that must answer a read.
    pub read_quorum: usize,
    /// Number of replicas that must acknowledge a write.
    pub write_quorum: usize,
    /// Backup acknowledgements required before returning to the caller.
    pub sync_backups: usize,
    /// Best-effort backups that do not block the caller.
    pub async_backups: usize,
    /// Maximum encoded value size accepted by value replication.
    pub max_replicated_entry_bytes: usize,
    /// Whether value replication is enabled.
    pub replicate_values: bool,
}

impl ReplicationConfig {
    /// Create a local-first config with value replication disabled.
    pub const fn local_first() -> Self {
        Self {
            replication_factor: 1,
            read_quorum: 1,
            write_quorum: 1,
            sync_backups: 0,
            async_backups: 0,
            max_replicated_entry_bytes: 0,
            replicate_values: false,
        }
    }

    /// Validate the olric-style replica/quorum invariants.
    pub fn validate(self) -> Result<(), ReplicationConfigError> {
        if self.replication_factor == 0 {
            return Err(ReplicationConfigError::ReplicationFactorZero);
        }
        if self.read_quorum == 0 || self.write_quorum == 0 {
            return Err(ReplicationConfigError::QuorumZero);
        }
        if self.read_quorum > self.replication_factor || self.write_quorum > self.replication_factor
        {
            return Err(ReplicationConfigError::QuorumExceedsReplicationFactor);
        }
        let requested_backups = self.sync_backups.saturating_add(self.async_backups);
        if requested_backups > self.replication_factor.saturating_sub(1) {
            return Err(ReplicationConfigError::BackupCountExceedsReplicationFactor);
        }
        if self.replicate_values && self.max_replicated_entry_bytes == 0 {
            return Err(ReplicationConfigError::MissingReplicatedEntryByteCap);
        }
        Ok(())
    }
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self::local_first()
    }
}

/// Errors returned by [`ReplicationConfig::validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicationConfigError {
    /// `replication_factor` must be at least one.
    ReplicationFactorZero,
    /// Read and write quorums must be at least one.
    QuorumZero,
    /// A quorum cannot exceed `replication_factor`.
    QuorumExceedsReplicationFactor,
    /// `sync_backups + async_backups` cannot exceed RF-1.
    BackupCountExceedsReplicationFactor,
    /// Value replication requires a byte cap before startup.
    MissingReplicatedEntryByteCap,
}

impl fmt::Display for ReplicationConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ReplicationFactorZero => "replication_factor must be at least 1",
            Self::QuorumZero => "read_quorum and write_quorum must be at least 1",
            Self::QuorumExceedsReplicationFactor => {
                "read_quorum/write_quorum cannot exceed replication_factor"
            }
            Self::BackupCountExceedsReplicationFactor => {
                "sync_backups + async_backups cannot exceed replication_factor - 1"
            }
            Self::MissingReplicatedEntryByteCap => {
                "replicate_values(true) requires max_replicated_entry_bytes"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ReplicationConfigError {}

impl ClusterReplicationStrategy for RendezvousClusterOwnership {
    fn name(&self) -> &'static str {
        "rendezvous"
    }

    fn replicas_for_key(
        &self,
        key: &str,
        members: &[ClusterMember],
        replication_factor: usize,
    ) -> Option<Replicas> {
        let mut ranked = members
            .iter()
            .filter(|member| member.is_member())
            .map(|member| {
                (
                    grid_rendezvous_score(key, &member.node_id),
                    member.node_id.clone(),
                )
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|(left_score, left_node), (right_score, right_node)| {
            right_score
                .cmp(left_score)
                .then_with(|| right_node.cmp(left_node))
        });
        ranked.dedup_by(|(_, left_node), (_, right_node)| left_node == right_node);

        let mut nodes = ranked
            .into_iter()
            .map(|(_, node)| node)
            .take(replication_factor.max(1))
            .collect::<Vec<_>>();
        if nodes.is_empty() {
            return None;
        }
        let primary = nodes.remove(0);
        Some(Replicas::new(primary, nodes))
    }
}

fn grid_rendezvous_score(key: &str, node_id: &ClusterNodeId) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in key.bytes().chain([0xff]).chain(node_id.as_str().bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Rebalance work materialized as committed data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebalanceTask {
    /// Move a partition primary from one owner to another.
    MovePartition {
        /// Partition being moved.
        partition: PartitionId,
        /// Previous primary.
        from: ClusterNodeId,
        /// New primary.
        to: ClusterNodeId,
    },
    /// Create or refresh a backup copy for a partition.
    ReReplicate {
        /// Partition being re-replicated.
        partition: PartitionId,
        /// Backup target.
        target: ClusterNodeId,
    },
}

/// Rebalance plan committed by the single coordinator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalancePlan {
    /// Topology epoch that owns this plan.
    pub epoch: ClusterEpoch,
    /// Deterministic list of movement tasks.
    pub tasks: Vec<RebalanceTask>,
}

impl RebalancePlan {
    /// Create a plan.
    pub fn new(epoch: ClusterEpoch, mut tasks: Vec<RebalanceTask>) -> Self {
        tasks.sort_by_key(rebalance_task_sort_key);
        tasks.dedup();
        Self { epoch, tasks }
    }

    /// Return whether every task is acknowledged.
    pub fn is_complete(&self, acks: &[RebalanceTaskAck]) -> bool {
        self.tasks.iter().all(|task| {
            acks.iter()
                .any(|ack| ack.epoch == self.epoch && ack.task == *task)
        })
    }

    /// Return the number of unacknowledged tasks.
    pub fn pending_task_count(&self, acks: &[RebalanceTaskAck]) -> usize {
        self.tasks
            .iter()
            .filter(|task| {
                !acks
                    .iter()
                    .any(|ack| ack.epoch == self.epoch && ack.task == **task)
            })
            .count()
    }
}

/// Acknowledgement for one rebalance task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceTaskAck {
    /// Epoch of the acknowledged plan.
    pub epoch: ClusterEpoch,
    /// Completed task.
    pub task: RebalanceTask,
}

/// Deterministically diff two maps for one partition.
pub fn diff_effective_maps(
    partition: PartitionId,
    old: &EffectiveReplicationMap,
    new: &EffectiveReplicationMap,
) -> Vec<RebalanceTask> {
    let mut tasks = Vec::new();
    if old.natural.primary != new.natural.primary {
        tasks.push(RebalanceTask::MovePartition {
            partition,
            from: old.natural.primary.clone(),
            to: new.natural.primary.clone(),
        });
    }

    let old_backups = old.natural.backups.iter().collect::<BTreeSet<_>>();
    for target in &new.natural.backups {
        if !old_backups.contains(target) {
            tasks.push(RebalanceTask::ReReplicate {
                partition,
                target: target.clone(),
            });
        }
    }
    tasks.sort_by_key(rebalance_task_sort_key);
    tasks
}

fn rebalance_task_sort_key(task: &RebalanceTask) -> (u32, u8, String, String) {
    match task {
        RebalanceTask::MovePartition {
            partition,
            from,
            to,
        } => (
            partition.value(),
            0,
            from.as_str().to_owned(),
            to.as_str().to_owned(),
        ),
        RebalanceTask::ReReplicate { partition, target } => (
            partition.value(),
            1,
            target.as_str().to_owned(),
            String::new(),
        ),
    }
}

/// Replicated value slot with tombstones participating in version ordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicatedSlot<V> {
    /// Live value bytes or decoded value.
    Value {
        /// Stored value.
        value: V,
        /// Monotonic version.
        version: u64,
    },
    /// Deletion marker that beats values at the same version.
    Tombstone {
        /// Monotonic tombstone version.
        version: u64,
        /// Epoch after which GC is allowed, once repair confirmed every backup.
        gc_eligible_after: Option<ClusterEpoch>,
    },
}

impl<V> ReplicatedSlot<V> {
    /// Return the slot version.
    pub fn version(&self) -> u64 {
        match self {
            Self::Value { version, .. } | Self::Tombstone { version, .. } => *version,
        }
    }

    /// Return whether this slot is a tombstone.
    pub fn is_tombstone(&self) -> bool {
        matches!(self, Self::Tombstone { .. })
    }

    /// Merge two slots: higher version wins; on ties tombstone wins.
    pub fn merge(self, other: Self) -> Self {
        match self.version().cmp(&other.version()) {
            std::cmp::Ordering::Greater => self,
            std::cmp::Ordering::Less => other,
            std::cmp::Ordering::Equal if self.is_tombstone() => self,
            std::cmp::Ordering::Equal => other,
        }
    }
}

/// Compose the version used for replicated values/tombstones.
pub fn replicated_slot_version(generation: ClusterGeneration, message_id: u64) -> u64 {
    generation.value().min(0xffff_ffff) << 32 | (message_id & 0xffff_ffff)
}

/// Tombstone retention budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TombstoneBudget {
    /// Maximum retained tombstone count.
    pub max_tombstones: usize,
    /// Maximum approximate tombstone bytes.
    pub max_tombstone_bytes: u64,
}

impl TombstoneBudget {
    /// Create a tombstone budget with normalized non-zero limits.
    pub fn new(max_tombstones: usize, max_tombstone_bytes: u64) -> Self {
        Self {
            max_tombstones: max_tombstones.max(1),
            max_tombstone_bytes: max_tombstone_bytes.max(1),
        }
    }
}

/// Result of admitting a tombstone under budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TombstoneAdmission {
    /// Tombstone was stored without needing eviction.
    Stored,
    /// Eligible tombstones were evicted oldest-first.
    EvictedEligible {
        /// Number of eligible tombstones evicted.
        freed: usize,
    },
    /// Budget is exceeded by blocking tombstones, so the node is degraded.
    RepairDebt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TombstoneRecord {
    key: String,
    version: u64,
    approx_bytes: u64,
    gc_eligible_after: Option<ClusterEpoch>,
}

/// Small deterministic tombstone budget tracker used by the 0.41 gate tests.
#[derive(Debug, Clone)]
pub struct TombstoneTracker {
    budget: TombstoneBudget,
    records: Vec<TombstoneRecord>,
    repair_debt: bool,
}

impl TombstoneTracker {
    /// Create an empty tracker.
    pub fn new(budget: TombstoneBudget) -> Self {
        Self {
            budget,
            records: Vec::new(),
            repair_debt: false,
        }
    }

    /// Admit or replace a tombstone and enforce the budget.
    pub fn admit(
        &mut self,
        key: impl Into<String>,
        version: u64,
        approx_bytes: u64,
        gc_eligible_after: Option<ClusterEpoch>,
    ) -> TombstoneAdmission {
        let key = key.into();
        self.records.retain(|record| record.key != key);
        self.records.push(TombstoneRecord {
            key,
            version,
            approx_bytes: approx_bytes.max(1),
            gc_eligible_after,
        });
        self.records.sort_by_key(|record| record.version);
        self.enforce_budget()
    }

    /// Mark a tombstone as repair-confirmed and eligible after `epoch`.
    pub fn confirm_repair(&mut self, key: &str, epoch: ClusterEpoch) {
        if let Some(record) = self.records.iter_mut().find(|record| record.key == key) {
            record.gc_eligible_after = Some(epoch);
        }
    }

    /// Return whether the tracker is in repair debt.
    pub fn repair_debt(&self) -> bool {
        self.repair_debt
    }

    /// Return retained tombstone count.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Return whether no tombstones are retained.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Return whether the key is retained.
    pub fn contains_key(&self, key: &str) -> bool {
        self.records.iter().any(|record| record.key == key)
    }

    fn total_bytes(&self) -> u64 {
        self.records
            .iter()
            .map(|record| record.approx_bytes)
            .sum::<u64>()
    }

    fn over_budget(&self) -> bool {
        self.records.len() > self.budget.max_tombstones
            || self.total_bytes() > self.budget.max_tombstone_bytes
    }

    fn enforce_budget(&mut self) -> TombstoneAdmission {
        let mut freed = 0;
        while self.over_budget() {
            let Some(index) = self
                .records
                .iter()
                .position(|record| record.gc_eligible_after.is_some())
            else {
                self.repair_debt = true;
                return TombstoneAdmission::RepairDebt;
            };
            self.records.remove(index);
            freed += 1;
        }
        self.repair_debt = false;
        if freed == 0 {
            TombstoneAdmission::Stored
        } else {
            TombstoneAdmission::EvictedEligible { freed }
        }
    }
}

/// Value-level replication eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Replication {
    /// Value may be replicated to backups.
    Eligible,
    /// Value must remain only on the local node.
    LocalOnly,
}

/// Replicated payload confidentiality posture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicatedValueSecurityPosture {
    /// Value replication is disabled.
    #[default]
    Disabled,
    /// Payloads are sealed by an operator-supplied provider.
    Encrypted,
    /// Operator explicitly accepted plaintext on the trust boundary.
    PlaintextAcknowledged,
    /// Unsafe posture: replication is on but plaintext was not acknowledged.
    PlaintextUnacknowledged,
}

impl ReplicatedValueSecurityPosture {
    /// Return the loud readiness highlight for unsafe plaintext replication.
    pub fn highlight(self) -> Option<&'static str> {
        match self {
            Self::PlaintextUnacknowledged => Some("REPLICATED VALUES PLAINTEXT"),
            Self::Disabled | Self::Encrypted | Self::PlaintextAcknowledged => None,
        }
    }
}

/// Error returned by replication encryption/decryption providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationCryptoError {
    message: String,
}

impl ReplicationCryptoError {
    /// Create a crypto error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ReplicationCryptoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ReplicationCryptoError {}

/// Operator-supplied sealing boundary for replicated value bytes.
pub trait ReplicationKeyProvider: Send + Sync {
    /// Seal bytes before they leave the primary.
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, ReplicationCryptoError>;

    /// Open bytes on the backup.
    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, ReplicationCryptoError>;
}

/// Optional redaction hook before replicated bytes are sealed/sent.
pub trait RedactReplicatedValue: Send + Sync {
    /// Return the bytes that are allowed to cross the replication boundary.
    fn redact(&self, plaintext: &[u8]) -> Vec<u8>;
}

/// Prepared replicated payload plus posture metadata.
#[derive(Clone)]
pub struct ReplicationPayload {
    /// Bytes to put on the wire.
    pub bytes: Vec<u8>,
    /// Security posture used while preparing the payload.
    pub posture: ReplicatedValueSecurityPosture,
}

/// Prepare bytes for replication, honoring LocalOnly/redaction/encryption.
pub fn prepare_replicated_payload(
    value: &[u8],
    eligibility: Replication,
    plaintext_acknowledged: bool,
    key_provider: Option<&dyn ReplicationKeyProvider>,
    redactor: Option<&dyn RedactReplicatedValue>,
) -> Result<Option<ReplicationPayload>, ReplicationCryptoError> {
    if eligibility == Replication::LocalOnly {
        return Ok(None);
    }

    let redacted = redactor
        .map(|redactor| redactor.redact(value))
        .unwrap_or_else(|| value.to_vec());
    if let Some(provider) = key_provider {
        return Ok(Some(ReplicationPayload {
            bytes: provider.seal(&redacted)?,
            posture: ReplicatedValueSecurityPosture::Encrypted,
        }));
    }

    let posture = if plaintext_acknowledged {
        ReplicatedValueSecurityPosture::PlaintextAcknowledged
    } else {
        ReplicatedValueSecurityPosture::PlaintextUnacknowledged
    };
    Ok(Some(ReplicationPayload {
        bytes: redacted,
        posture,
    }))
}

/// Per-partition near-cache repair task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairingTask {
    /// Polling interval for owner watermarks.
    pub interval: Duration,
}

impl RepairingTask {
    /// Create a task with a normalized interval.
    pub fn new(interval: Duration) -> Self {
        Self {
            interval: if interval.is_zero() {
                Duration::from_secs(1)
            } else {
                interval
            },
        }
    }
}

/// Promotion phase for deterministic backup failover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionPhase {
    /// Freeze writes before changing the effective map.
    Before,
    /// Commit backup-to-primary promotion.
    Commit,
    /// Unfreeze and restore replication factor.
    Finalize,
}

/// Backup promotion operation for one partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPromotion {
    /// Partition being promoted.
    pub partition: PartitionId,
    /// Departing primary.
    pub departing_primary: ClusterNodeId,
    /// Backup selected as the new primary.
    pub new_primary: ClusterNodeId,
    /// Current promotion phase.
    pub phase: PromotionPhase,
}

/// Select the first backup as the deterministic promotion candidate.
pub fn select_backup_promotion(
    partition: PartitionId,
    replicas: &Replicas,
) -> Option<BackupPromotion> {
    replicas
        .backups
        .first()
        .cloned()
        .map(|new_primary| BackupPromotion {
            partition,
            departing_primary: replicas.primary.clone(),
            new_primary,
            phase: PromotionPhase::Before,
        })
}

/// Per-replica version table used by anti-entropy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionReplicaVersions {
    versions: BTreeMap<(PartitionId, ClusterNodeId), u64>,
}

impl PartitionReplicaVersions {
    /// Set a replica version.
    pub fn set_version(
        &mut self,
        partition: PartitionId,
        node: impl Into<ClusterNodeId>,
        version: u64,
    ) {
        self.versions.insert((partition, node.into()), version);
    }

    /// Return a replica version.
    pub fn version(&self, partition: PartitionId, node: &ClusterNodeId) -> Option<u64> {
        self.versions.get(&(partition, node.clone())).copied()
    }

    /// Return backups whose version is lower than the primary version.
    pub fn lagging_replicas(
        &self,
        partition: PartitionId,
        primary: &ClusterNodeId,
        backups: &[ClusterNodeId],
    ) -> Vec<ClusterNodeId> {
        let primary_version = self.version(partition, primary).unwrap_or_default();
        backups
            .iter()
            .filter(|backup| self.version(partition, backup).unwrap_or_default() < primary_version)
            .cloned()
            .collect()
    }
}

/// Throttled anti-entropy executor config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntiEntropyTask {
    /// Repair interval.
    pub interval: Duration,
}

impl AntiEntropyTask {
    /// Create a task with normalized interval.
    pub fn new(interval: Duration) -> Self {
        Self {
            interval: if interval.is_zero() {
                Duration::from_secs(1)
            } else {
                interval
            },
        }
    }
}

/// Tracks which nodes hold hot copies for authoritative invalidation fan-out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotCacheDirectory {
    holders: BTreeMap<String, BTreeSet<ClusterNodeId>>,
}

impl HotCacheDirectory {
    /// Record that `holder` has a hot copy for `key`.
    pub fn record_holder(&mut self, key: impl Into<String>, holder: impl Into<ClusterNodeId>) {
        self.holders
            .entry(key.into())
            .or_default()
            .insert(holder.into());
    }

    /// Remove and return every holder that must receive an invalidation.
    pub fn invalidate(&mut self, key: &str) -> Vec<ClusterNodeId> {
        self.holders
            .remove(key)
            .map(|holders| holders.into_iter().collect())
            .unwrap_or_default()
    }

    /// Return current holders for diagnostics.
    pub fn holders(&self, key: &str) -> Vec<ClusterNodeId> {
        self.holders
            .get(key)
            .map(|holders| holders.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Aggregate grid counters; high-cardinality detail stays in diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterGridCounters {
    /// Successful value/tombstone replications.
    pub replication_success_total: u64,
    /// Failed value/tombstone replications.
    pub replication_failure_total: u64,
    /// Total replicated bytes.
    pub bytes_replicated_total: u64,
    /// Replication queue backpressure events.
    pub replication_backpressure_total: u64,
    /// Values rejected before send because they exceeded the byte cap.
    pub replication_oversized_rejected_total: u64,
    /// Payload decrypt/open failures.
    pub replication_decrypt_failure_total: u64,
    /// Aggregate under-replicated key count.
    pub under_replicated_keys: u64,
    /// Failover promotions.
    pub failover_total: u64,
    /// Repair task executions.
    pub repair_task_total: u64,
    /// Repair task failures.
    pub repair_failure_total: u64,
    /// Rebalance plans committed.
    pub rebalance_plan_total: u64,
    /// Rebalance task acknowledgements.
    pub rebalance_task_ack_total: u64,
    /// Topology-fence rejections.
    pub topology_fence_rejected_total: u64,
    /// Tombstones blocked by repair debt.
    pub tombstone_repair_debt: u64,
    /// Durable replicated values rejected by total-byte budget.
    pub replicated_value_rejected_total: u64,
    /// Split-brain detections.
    pub split_brain_detected_total: u64,
    /// Loser-side entries discarded during merge.
    pub merge_discarded_entries_total: u64,
    /// Merge conflicts that could not be resolved deterministically.
    pub merge_unresolved_conflicts_total: u64,
    /// Cluster-route authentication or authorization rejections.
    pub cluster_auth_rejected_total: u64,
    /// Whether repair-debt degraded mode is active.
    pub repair_debt_degraded_mode: u64,
    /// Placements that could not span the requested number of zones.
    pub placement_zone_underspread: u64,
    /// Online reshard moves currently in flight.
    pub reshard_moves_inflight: u64,
    /// Aggregate reshard backfill lag.
    pub reshard_backfill_lag: u64,
    /// Local-zone read hits.
    pub read_local_zone_total: u64,
    /// Hedged read requests.
    pub read_hedged_total: u64,
    /// Hedged reads where a hedge response won.
    pub read_hedge_win_total: u64,
    /// Tiered-value promotions from cold to hot.
    pub value_tier_promotions_total: u64,
    /// Tiered-value demotions from hot to cold.
    pub value_tier_demotions_total: u64,
    /// Single-partition invalidation batches.
    pub invalidate_batch_total: u64,
    /// Pending invalidation saga targets.
    pub invalidation_saga_pending: u64,
    /// Auto-repair actions scheduled in active mode.
    pub auto_repair_active_total: u64,
    /// Auto-repair recommendations emitted in advisory mode.
    pub auto_repair_advisory_total: u64,
    /// Operations recorded with an explicit per-operation consistency level.
    pub consistency_level_operations_total: u64,
    /// Operations rejected because the requested consistency level was unsatisfiable.
    pub consistency_unsatisfiable_total: u64,
    /// Hinted handoff writes retained for replay.
    pub hints_stored_total: u64,
    /// Hinted handoff writes replayed successfully.
    pub hints_replayed_total: u64,
    /// Hints dropped because of age or budget.
    pub hints_dropped_total: u64,
    /// Approximate retained hint bytes.
    pub hint_store_bytes: u64,
    /// Merkle repair ranges exchanged.
    pub repair_ranges_exchanged_total: u64,
    /// Foreground read-repair executions.
    pub read_repair_total: u64,
    /// Last aggregate repair progress ratio, scaled 0..=10000.
    pub repair_progress_ratio: u64,
    /// Last aggregate phi suspicion value, scaled by 1000.
    pub peer_phi_scaled: u64,
    /// Suspicions later observed to recover without a real outage.
    pub false_suspect_total: u64,
    /// Applied single-key compare-and-set operations.
    pub cas_applied_total: u64,
    /// Single-key compare-and-set mismatches.
    pub cas_mismatch_total: u64,
    /// Fenced lock acquisitions.
    pub lock_acquired_total: u64,
    /// Stale fenced lock tokens rejected.
    pub lock_stale_token_rejected_total: u64,
    /// Retained invalidation ring events.
    pub invalidation_ring_depth: u64,
    /// Exact invalidation events replayed.
    pub invalidation_replayed_total: u64,
    /// Subscribers that fell behind the retained invalidation window.
    pub invalidation_fell_behind_total: u64,
    /// Invalidation events overwritten by a full ring.
    pub invalidation_ring_overrun_total: u64,
    /// Current retained session watermark entries.
    pub session_watermark_entries: u64,
    /// Current active session count.
    pub session_active_sessions: u64,
    /// P99 retained watermark entries across active sessions.
    pub session_watermark_entries_p99: u64,
    /// Worst observed session staleness in versions.
    pub session_worst_staleness_versions: u64,
    /// Session watermark coarsening events.
    pub session_watermark_coarsened_total: u64,
    /// Rejected session tokens.
    pub session_token_rejected_total: u64,
    /// Session read-your-writes reads that had to escalate.
    pub session_ryw_escalations_total: u64,
    /// Session reads that failed rather than serving below the watermark.
    pub session_guarantee_unmet_total: u64,
    /// Monotonic reads prevented from going backwards.
    pub monotonic_read_violations_prevented_total: u64,
    /// Monotonic writes prevented from reordering or lowering their stamp.
    pub monotonic_write_reorders_prevented_total: u64,
    /// Causal writes deferred until dependencies become visible locally.
    pub causal_writes_deferred_total: u64,
    /// Causal summary coarsening events.
    pub causal_summary_coarsened_total: u64,
    /// Approximate causal dependency metadata bytes.
    pub causal_dependency_bytes: u64,
    /// Reads served locally by explicit bounded staleness.
    pub bounded_staleness_fast_serves_total: u64,
    /// Bounded-staleness reads that had to escalate.
    pub bounded_staleness_escalations_total: u64,
}

/// Bounded metric descriptor used by cardinality tests and exporters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClusterMetricDescriptor {
    /// Metric name.
    pub name: &'static str,
    /// Low-cardinality labels only.
    pub labels: &'static [&'static str],
}

/// Aggregate metric descriptors exported by the grid slice.
pub fn cluster_grid_metric_descriptors() -> &'static [ClusterMetricDescriptor] {
    const DESCRIPTORS: &[ClusterMetricDescriptor] = &[
        ClusterMetricDescriptor {
            name: "hydracache_replication_success_total",
            labels: &["role", "outcome"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_replication_failure_total",
            labels: &["role", "outcome"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_bytes_replicated_total",
            labels: &["role"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_replication_backpressure_total",
            labels: &["role"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_replication_oversized_rejected_total",
            labels: &["role"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_under_replicated_keys",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_topology_fence_rejected_total",
            labels: &["role"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_tombstone_repair_debt",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_replicated_value_rejected_total",
            labels: &["reason"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_replication_window_size",
            labels: &["role"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_promotion_freeze_window_ms",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_replication_lag",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_split_brain_detected_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_merge_discarded_entries_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_merge_unresolved_conflicts_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_cluster_auth_rejected_total",
            labels: &["route"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_repair_debt_degraded_mode",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_placement_zone_underspread",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_reshard_moves_inflight",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_reshard_backfill_lag",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_read_local_zone_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_read_hedged_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_read_hedge_win_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_value_tier_promotions_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_value_tier_demotions_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_invalidate_batch_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_invalidation_saga_pending",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_auto_repair_active_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_auto_repair_advisory_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_region_staleness_window_ms",
            labels: &["region"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_region_link_lag",
            labels: &["link"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_region_link_bytes_total",
            labels: &["link"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_region_link_window",
            labels: &["link"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_region_state",
            labels: &["region", "state"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_region_promotion_total",
            labels: &["region"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_region_restore_duration_ms",
            labels: &["region"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_tenant_bytes",
            labels: &["tenant"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_tenant_entries",
            labels: &["tenant"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_tenant_admission_rejected_total",
            labels: &["tenant"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_client_auth_rejected_total",
            labels: &["route"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_residency_rejected_placement_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_residency_refused_crossing_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_audit_sink_failures_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_audit_mandatory_fail_closed_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_crdt_metadata_bytes",
            labels: &["region"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_capacity_recommendation",
            labels: &["region", "recommendation"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_scale_actions_total",
            labels: &["region", "action"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_op_consistency_level_total",
            labels: &["operation", "level"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_consistency_unsatisfiable_total",
            labels: &["operation", "level"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_hints_stored_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_hints_replayed_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_hints_dropped_total",
            labels: &["reason"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_hint_store_bytes",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_repair_sessions_total",
            labels: &["kind"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_repair_ranges_exchanged_total",
            labels: &["kind"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_read_repair_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_repair_progress_ratio",
            labels: &["partition"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_peer_phi",
            labels: &["peer"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_false_suspect_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_cas_applied_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_cas_mismatch_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_lock_acquired_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_lock_stale_token_rejected_total",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_invalidation_ring_depth",
            labels: &["partition"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_invalidation_replayed_total",
            labels: &["subscriber"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_invalidation_fell_behind_total",
            labels: &["subscriber"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_invalidation_ring_overrun_total",
            labels: &["partition"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_session_watermark_entries",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_session_active_sessions",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_session_watermark_entries_p99",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_session_worst_staleness_versions",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_session_watermark_coarsened_total",
            labels: &["reason"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_session_token_rejected_total",
            labels: &["reason"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_session_ryw_escalations_total",
            labels: &["path"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_session_guarantee_unmet_total",
            labels: &["guarantee"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_monotonic_read_violations_prevented_total",
            labels: &["guarantee"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_monotonic_write_reorders_prevented_total",
            labels: &["guarantee"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_causal_writes_deferred_total",
            labels: &["reason"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_causal_summary_coarsened_total",
            labels: &["reason"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_causal_dependency_bytes",
            labels: &[],
        },
        ClusterMetricDescriptor {
            name: "hydracache_bounded_staleness_fast_serves_total",
            labels: &["mode"],
        },
        ClusterMetricDescriptor {
            name: "hydracache_bounded_staleness_escalations_total",
            labels: &["reason"],
        },
    ];
    DESCRIPTORS
}

/// High-cardinality grid detail kept out of metrics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterGridDiagnostics {
    /// Per-partition detail for on-demand diagnostics only.
    pub partition_replica_versions: BTreeMap<PartitionId, Vec<(ClusterNodeId, u64)>>,
    /// Current aggregate counters.
    pub counters: ClusterGridCounters,
    /// Replicated value confidentiality posture.
    pub replicated_value_security: ReplicatedValueSecurityPosture,
    /// Last split-brain report retained for operator diagnostics.
    pub last_split_brain: Option<hardening::SplitBrainReport>,
}

/// Shared pointer alias for operator-provided key providers.
pub type SharedReplicationKeyProvider = Arc<dyn ReplicationKeyProvider>;
