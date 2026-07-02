use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, ClusterNodeId, PartitionId};
use crate::grid::durability::DurabilitySnapshotManifest;
use crate::grid::hardening::WriteWatermark;

/// Cluster checkpoint manifest format version registered in `docs/COMPAT.md`.
pub const CLUSTER_CHECKPOINT_MANIFEST_FORMAT_VERSION: u32 = 1;

/// Per-node durable manifests collected for a cluster-wide checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCheckpointManifest {
    /// Node that produced these durable snapshot manifests.
    pub node_id: ClusterNodeId,
    /// Durable snapshot manifests reported by the node.
    pub snapshots: Vec<DurabilitySnapshotManifest>,
}

impl NodeCheckpointManifest {
    /// Create a normalized per-node checkpoint manifest.
    pub fn new(
        node_id: impl Into<ClusterNodeId>,
        mut snapshots: Vec<DurabilitySnapshotManifest>,
    ) -> Self {
        snapshots.sort_by(snapshot_sort);
        Self {
            node_id: node_id.into(),
            snapshots,
        }
    }
}

/// Cluster-wide consistent cut over durable per-node snapshot manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterCheckpointManifest {
    /// Manifest format version.
    pub format_version: u32,
    /// Operator/controller supplied checkpoint id.
    pub checkpoint_id: String,
    /// Authority epoch for the cut.
    pub epoch: ClusterEpoch,
    /// Required barrier watermark for each partition in the cut.
    pub partition_watermarks: BTreeMap<PartitionId, WriteWatermark>,
    /// Per-node durable manifests that prove the cut is covered.
    pub node_manifests: BTreeMap<ClusterNodeId, Vec<DurabilitySnapshotManifest>>,
    /// Controller timestamp in milliseconds after process start.
    pub created_after_ms: u64,
    /// Checksum over stable manifest fields.
    pub checksum: u64,
}

impl ClusterCheckpointManifest {
    /// Build and verify a checksum-protected cluster checkpoint manifest.
    pub fn new(
        checkpoint_id: impl Into<String>,
        epoch: ClusterEpoch,
        watermarks: impl IntoIterator<Item = WriteWatermark>,
        node_manifests: impl IntoIterator<Item = NodeCheckpointManifest>,
        created_after: Duration,
    ) -> Result<Self, ClusterCheckpointError> {
        let checkpoint_id = checkpoint_id.into();
        let partition_watermarks = normalize_watermarks(epoch, watermarks)?;
        let node_manifests = normalize_node_manifests(node_manifests)?;
        let created_after_ms = duration_millis(created_after);
        let checksum = checkpoint_checksum(
            &checkpoint_id,
            epoch,
            &partition_watermarks,
            &node_manifests,
            created_after_ms,
        );
        let manifest = Self {
            format_version: CLUSTER_CHECKPOINT_MANIFEST_FORMAT_VERSION,
            checkpoint_id,
            epoch,
            partition_watermarks,
            node_manifests,
            created_after_ms,
            checksum,
        };
        manifest.verify()?;
        Ok(manifest)
    }

    /// Verify format, checksum, per-node manifests, and partition coverage.
    pub fn verify(&self) -> Result<(), ClusterCheckpointError> {
        if self.format_version != CLUSTER_CHECKPOINT_MANIFEST_FORMAT_VERSION {
            return Err(ClusterCheckpointError::new(
                ClusterCheckpointErrorKind::UnsupportedFormat,
                format!(
                    "unsupported cluster checkpoint format {}; expected {}",
                    self.format_version, CLUSTER_CHECKPOINT_MANIFEST_FORMAT_VERSION
                ),
            ));
        }
        if self.partition_watermarks.is_empty() {
            return Err(ClusterCheckpointError::new(
                ClusterCheckpointErrorKind::EmptyBarrier,
                "cluster checkpoint barrier has no partitions",
            ));
        }
        for (partition, watermark) in &self.partition_watermarks {
            if *partition != watermark.partition {
                return Err(ClusterCheckpointError::new(
                    ClusterCheckpointErrorKind::Manifest,
                    format!(
                        "checkpoint partition key {} does not match watermark partition {}",
                        partition.value(),
                        watermark.partition.value()
                    ),
                ));
            }
            if watermark.epoch != self.epoch {
                return Err(ClusterCheckpointError::new(
                    ClusterCheckpointErrorKind::StaleWatermark,
                    format!(
                        "checkpoint partition {} watermark epoch {} does not match checkpoint epoch {}",
                        partition.value(),
                        watermark.epoch.value(),
                        self.epoch.value()
                    ),
                ));
            }
        }
        if self.node_manifests.is_empty() {
            return Err(ClusterCheckpointError::new(
                ClusterCheckpointErrorKind::PartialCut,
                "cluster checkpoint has no node manifests",
            ));
        }
        for snapshots in self.node_manifests.values() {
            for snapshot in snapshots {
                snapshot.verify().map_err(|error| {
                    ClusterCheckpointError::new(
                        ClusterCheckpointErrorKind::Manifest,
                        error.to_string(),
                    )
                })?;
            }
        }
        validate_partition_coverage(&self.partition_watermarks, &self.node_manifests)?;
        let expected = checkpoint_checksum(
            &self.checkpoint_id,
            self.epoch,
            &self.partition_watermarks,
            &self.node_manifests,
            self.created_after_ms,
        );
        if expected != self.checksum {
            return Err(ClusterCheckpointError::new(
                ClusterCheckpointErrorKind::Checksum,
                format!(
                    "cluster checkpoint checksum mismatch for '{}'",
                    self.checkpoint_id
                ),
            ));
        }
        Ok(())
    }

    /// Return the barrier watermark for `partition`.
    pub fn watermark_for_partition(&self, partition: PartitionId) -> Option<WriteWatermark> {
        self.partition_watermarks.get(&partition).copied()
    }

    /// Return whether this checkpoint covers the supplied write watermark.
    pub fn covers(&self, watermark: WriteWatermark) -> bool {
        self.watermark_for_partition(watermark.partition)
            .map(|barrier| watermark_is_at_or_before_barrier(watermark, barrier))
            .unwrap_or(false)
    }
}

/// In-memory coordinator for cluster-wide checkpoint collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckpointCoordinator {
    latest_valid: Option<ClusterCheckpointManifest>,
    rejected_total: u64,
}

impl CheckpointCoordinator {
    /// Create an empty checkpoint coordinator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Collect per-node manifests and store the checkpoint only if the cut is complete.
    pub fn coordinate(
        &mut self,
        checkpoint_id: impl Into<String>,
        epoch: ClusterEpoch,
        watermarks: impl IntoIterator<Item = WriteWatermark>,
        node_manifests: impl IntoIterator<Item = NodeCheckpointManifest>,
        created_after: Duration,
    ) -> Result<ClusterCheckpointManifest, ClusterCheckpointError> {
        match ClusterCheckpointManifest::new(
            checkpoint_id,
            epoch,
            watermarks,
            node_manifests,
            created_after,
        ) {
            Ok(manifest) => {
                self.latest_valid = Some(manifest.clone());
                Ok(manifest)
            }
            Err(error) => {
                self.rejected_total = self.rejected_total.saturating_add(1);
                Err(error)
            }
        }
    }

    /// Return the latest complete, verified checkpoint, if one exists.
    pub fn latest_valid(&self) -> Option<&ClusterCheckpointManifest> {
        self.latest_valid.as_ref()
    }

    /// Return rejected partial/stale/invalid checkpoint attempts.
    pub fn rejected_total(&self) -> u64 {
        self.rejected_total
    }
}

/// Stable checkpoint error kind used by recovery and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterCheckpointErrorKind {
    /// The barrier did not name any partitions.
    EmptyBarrier,
    /// The manifest format is newer or otherwise unsupported.
    UnsupportedFormat,
    /// The manifest checksum does not match stable fields.
    Checksum,
    /// A node crashed or failed to provide coverage for a required partition.
    PartialCut,
    /// A reported snapshot does not cover the required partition watermark.
    StaleWatermark,
    /// A nested durable manifest is malformed or failed verification.
    Manifest,
    /// Restore/rescale was fenced by the current authority epoch.
    AuthorityFence,
}

/// Error returned by checkpoint collection, verification, and restore fencing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterCheckpointError {
    kind: ClusterCheckpointErrorKind,
    message: String,
}

impl ClusterCheckpointError {
    /// Create a checkpoint error.
    pub fn new(kind: ClusterCheckpointErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Return the stable error kind.
    pub fn kind(&self) -> ClusterCheckpointErrorKind {
        self.kind
    }
}

impl fmt::Display for ClusterCheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ClusterCheckpointError {}

fn normalize_watermarks(
    epoch: ClusterEpoch,
    watermarks: impl IntoIterator<Item = WriteWatermark>,
) -> Result<BTreeMap<PartitionId, WriteWatermark>, ClusterCheckpointError> {
    let mut normalized: BTreeMap<PartitionId, WriteWatermark> = BTreeMap::new();
    for watermark in watermarks {
        if watermark.epoch != epoch {
            return Err(ClusterCheckpointError::new(
                ClusterCheckpointErrorKind::StaleWatermark,
                format!(
                    "checkpoint watermark for partition {} has epoch {}, expected {}",
                    watermark.partition.value(),
                    watermark.epoch.value(),
                    epoch.value()
                ),
            ));
        }
        match normalized.get(&watermark.partition) {
            Some(existing) if existing.version != watermark.version => {
                return Err(ClusterCheckpointError::new(
                    ClusterCheckpointErrorKind::StaleWatermark,
                    format!(
                        "conflicting checkpoint watermarks for partition {}",
                        watermark.partition.value()
                    ),
                ));
            }
            Some(_) => {}
            None => {
                normalized.insert(watermark.partition, watermark);
            }
        }
    }
    if normalized.is_empty() {
        return Err(ClusterCheckpointError::new(
            ClusterCheckpointErrorKind::EmptyBarrier,
            "cluster checkpoint barrier has no partitions",
        ));
    }
    Ok(normalized)
}

fn normalize_node_manifests(
    node_manifests: impl IntoIterator<Item = NodeCheckpointManifest>,
) -> Result<BTreeMap<ClusterNodeId, Vec<DurabilitySnapshotManifest>>, ClusterCheckpointError> {
    let mut normalized = BTreeMap::new();
    for mut node_manifest in node_manifests {
        if normalized.contains_key(&node_manifest.node_id) {
            return Err(ClusterCheckpointError::new(
                ClusterCheckpointErrorKind::Manifest,
                format!(
                    "duplicate checkpoint manifest for node {}",
                    node_manifest.node_id
                ),
            ));
        }
        node_manifest.snapshots.sort_by(snapshot_sort);
        normalized.insert(node_manifest.node_id, node_manifest.snapshots);
    }
    if normalized.is_empty() {
        return Err(ClusterCheckpointError::new(
            ClusterCheckpointErrorKind::PartialCut,
            "cluster checkpoint has no node manifests",
        ));
    }
    Ok(normalized)
}

fn validate_partition_coverage(
    required: &BTreeMap<PartitionId, WriteWatermark>,
    node_manifests: &BTreeMap<ClusterNodeId, Vec<DurabilitySnapshotManifest>>,
) -> Result<(), ClusterCheckpointError> {
    for (partition, barrier) in required {
        let candidates = node_manifests
            .values()
            .flat_map(|snapshots| snapshots.iter())
            .filter(|snapshot| snapshot.watermark.partition == *partition)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Err(ClusterCheckpointError::new(
                ClusterCheckpointErrorKind::PartialCut,
                format!(
                    "cluster checkpoint missing snapshot coverage for partition {}",
                    partition.value()
                ),
            ));
        }
        if candidates
            .iter()
            .all(|snapshot| !snapshot_covers_required(snapshot.watermark, *barrier))
        {
            return Err(ClusterCheckpointError::new(
                ClusterCheckpointErrorKind::StaleWatermark,
                format!(
                    "cluster checkpoint has no snapshot at or beyond partition {} watermark {}",
                    partition.value(),
                    barrier.version
                ),
            ));
        }
    }
    Ok(())
}

fn snapshot_covers_required(snapshot: WriteWatermark, required: WriteWatermark) -> bool {
    snapshot.partition == required.partition
        && snapshot.epoch == required.epoch
        && snapshot.version >= required.version
}

fn watermark_is_at_or_before_barrier(watermark: WriteWatermark, barrier: WriteWatermark) -> bool {
    watermark.partition == barrier.partition
        && (watermark.epoch < barrier.epoch
            || (watermark.epoch == barrier.epoch && watermark.version <= barrier.version))
}

fn checkpoint_checksum(
    checkpoint_id: &str,
    epoch: ClusterEpoch,
    watermarks: &BTreeMap<PartitionId, WriteWatermark>,
    node_manifests: &BTreeMap<ClusterNodeId, Vec<DurabilitySnapshotManifest>>,
    created_after_ms: u64,
) -> u64 {
    let mut checksum = ArtifactChecksum::new();
    checksum.u32(CLUSTER_CHECKPOINT_MANIFEST_FORMAT_VERSION);
    checksum.bytes(checkpoint_id.as_bytes());
    checksum.u64(epoch.value());
    checksum.u64(created_after_ms);
    checksum.usize(watermarks.len());
    for (partition, watermark) in watermarks {
        checksum.u32(partition.value());
        checksum.u32(watermark.partition.value());
        checksum.u64(watermark.version);
        checksum.u64(watermark.epoch.value());
    }
    checksum.usize(node_manifests.len());
    for (node, snapshots) in node_manifests {
        checksum.bytes(node.as_str().as_bytes());
        checksum.usize(snapshots.len());
        for snapshot in snapshots {
            checksum.u32(snapshot.format_version);
            checksum.bytes(snapshot.namespace.as_bytes());
            checksum.u32(snapshot.watermark.partition.value());
            checksum.u64(snapshot.watermark.version);
            checksum.u64(snapshot.watermark.epoch.value());
            checksum.u64(snapshot.created_after_ms);
            checksum.u64(snapshot.interval_ms);
            checksum.u64(snapshot.checksum);
        }
    }
    checksum.finish()
}

fn snapshot_sort(
    left: &DurabilitySnapshotManifest,
    right: &DurabilitySnapshotManifest,
) -> std::cmp::Ordering {
    (
        left.watermark.partition.value(),
        left.watermark.epoch.value(),
        left.watermark.version,
        left.namespace.as_str(),
        left.created_after_ms,
    )
        .cmp(&(
            right.watermark.partition.value(),
            right.watermark.epoch.value(),
            right.watermark.version,
            right.namespace.as_str(),
            right.created_after_ms,
        ))
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[derive(Debug, Clone, Copy)]
struct ArtifactChecksum(u64);

impl ArtifactChecksum {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn u8(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(Self::PRIME);
    }

    fn u32(&mut self, value: u32) {
        self.raw_bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.raw_bytes(&value.to_le_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.usize(bytes.len());
        self.raw_bytes(bytes);
    }

    fn raw_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.u8(*byte);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}
