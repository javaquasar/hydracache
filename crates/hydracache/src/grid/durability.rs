use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, PartitionId};
#[cfg(feature = "durable-value-store")]
use crate::grid::durable_store::DurableValueStore;
use crate::grid::elasticity::RegionId;
use crate::grid::hardening::{
    InMemoryReplicatedValueStore, ReplicatedValueRecord, ReplicatedValueStore, ValueStoreError,
    ValueVersion, WriteWatermark,
};
use crate::grid::persistence_policy::{
    PersistenceDurability, PersistencePolicy, PersistencePolicyError, PersistenceRegionPlacement,
};

/// Snapshot manifest format version registered in `docs/COMPAT.md`.
pub const DURABILITY_SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// Adapter hook used by the durability write path to force a durable flush.
pub trait DurableFlush {
    /// Flush pending durable bytes before acknowledging a sync durability write.
    fn flush_durable(&self) -> Result<(), ValueStoreError>;
}

impl DurableFlush for InMemoryReplicatedValueStore {
    fn flush_durable(&self) -> Result<(), ValueStoreError> {
        Ok(())
    }
}

#[cfg(feature = "durable-value-store")]
impl DurableFlush for DurableValueStore {
    fn flush_durable(&self) -> Result<(), ValueStoreError> {
        self.flush()
    }
}

/// Result of routing one write through the durability path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurableWriteOutcome {
    /// Namespace resolved to RAM-only for this local region; no durable write was made.
    SkippedRamOnly,
    /// Sync durability wrote and flushed before returning to the caller.
    SyncAcked {
        /// Watermark covered by the acknowledged write.
        watermark: WriteWatermark,
        /// Whether the flush completed before the acknowledgement.
        fsync_before_ack: bool,
    },
    /// Async-bounded durability admitted the write to the bounded write-behind queue.
    Queued {
        /// Watermark covered after the queued write is drained.
        watermark: WriteWatermark,
        /// Current pending queue length after enqueue.
        lag: usize,
    },
}

/// Bounded aggregate metrics for the durability path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurabilityMetricsSnapshot {
    /// Sync writes flushed before acknowledgement.
    pub sync_write_total: u64,
    /// Async writes admitted to the bounded queue.
    pub async_queued_total: u64,
    /// Async writes drained to the underlying store.
    pub async_drained_total: u64,
    /// Writes skipped because the namespace is RAM-only on this node.
    pub ram_only_skipped_total: u64,
    /// Writes refused by async lag backpressure.
    pub backpressure_total: u64,
    /// Snapshot manifests recorded.
    pub snapshot_total: u64,
    /// Current aggregate async queue lag.
    pub async_lag: usize,
}

/// Durable snapshot manifest with a checksum-protected recovery watermark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurabilitySnapshotManifest {
    /// Manifest format version.
    pub format_version: u32,
    /// Namespace covered by the manifest.
    pub namespace: String,
    /// Watermark covered by the snapshot.
    pub watermark: WriteWatermark,
    /// Scheduler timestamp in milliseconds after node/process start.
    pub created_after_ms: u64,
    /// Policy snapshot interval that triggered the manifest.
    pub interval_ms: u64,
    /// Checksum over stable manifest fields.
    pub checksum: u64,
}

impl DurabilitySnapshotManifest {
    /// Create a checksum-protected manifest.
    pub fn new(
        namespace: impl Into<String>,
        watermark: WriteWatermark,
        created_after: Duration,
        interval: Duration,
    ) -> Self {
        let namespace = namespace.into();
        let created_after_ms = duration_millis(created_after);
        let interval_ms = duration_millis(interval);
        let checksum = snapshot_checksum(&namespace, watermark, created_after_ms, interval_ms);
        Self {
            format_version: DURABILITY_SNAPSHOT_FORMAT_VERSION,
            namespace,
            watermark,
            created_after_ms,
            interval_ms,
            checksum,
        }
    }

    /// Verify the manifest format and checksum before recovery trusts it.
    pub fn verify(&self) -> Result<(), DurabilityError> {
        if self.format_version != DURABILITY_SNAPSHOT_FORMAT_VERSION {
            return Err(DurabilityError::snapshot(format!(
                "unsupported durability snapshot format {}; expected {}",
                self.format_version, DURABILITY_SNAPSHOT_FORMAT_VERSION
            )));
        }
        let expected = snapshot_checksum(
            &self.namespace,
            self.watermark,
            self.created_after_ms,
            self.interval_ms,
        );
        if expected != self.checksum {
            return Err(DurabilityError::snapshot(format!(
                "durability snapshot checksum mismatch for namespace '{}'",
                self.namespace
            )));
        }
        Ok(())
    }
}

/// Durability write-path coordinator for one local node.
#[derive(Debug, Clone)]
pub struct DurabilityWritePath<S> {
    store: S,
    policy: PersistencePolicy,
    local_region: RegionId,
    placement: PersistenceRegionPlacement,
    pending: VecDeque<PendingDurableWrite>,
    last_snapshot_at: BTreeMap<String, Duration>,
    snapshots: Vec<DurabilitySnapshotManifest>,
    metrics: DurabilityMetricsSnapshot,
}

impl<S> DurabilityWritePath<S>
where
    S: ReplicatedValueStore + DurableFlush,
{
    /// Create a local durability coordinator.
    pub fn new(
        store: S,
        policy: PersistencePolicy,
        local_region: impl Into<RegionId>,
        placement: PersistenceRegionPlacement,
    ) -> Self {
        Self {
            store,
            policy,
            local_region: local_region.into(),
            placement,
            pending: VecDeque::new(),
            last_snapshot_at: BTreeMap::new(),
            snapshots: Vec::new(),
            metrics: DurabilityMetricsSnapshot::default(),
        }
    }

    /// Return the underlying store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Return the mutable underlying store.
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// Consume the path and return the underlying store.
    pub fn into_store(self) -> S {
        self.store
    }

    /// Return aggregate durability metrics.
    pub fn metrics(&self) -> DurabilityMetricsSnapshot {
        self.metrics.clone()
    }

    /// Return the current async write-behind lag.
    pub fn pending_lag(&self) -> usize {
        self.pending.len()
    }

    /// Return recorded snapshot manifests.
    pub fn snapshots(&self) -> &[DurabilitySnapshotManifest] {
        &self.snapshots
    }

    /// Return snapshot age for a namespace at `now`, if a snapshot has been recorded.
    pub fn snapshot_age_ms(&self, namespace: &str, now: Duration) -> Option<u64> {
        self.last_snapshot_at
            .get(namespace)
            .map(|last| duration_millis(now.saturating_sub(*last)))
    }

    /// Route a value/tombstone write according to the resolved persistence policy.
    pub fn write(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        record: ReplicatedValueRecord,
    ) -> Result<DurableWriteOutcome, DurabilityError> {
        let namespace = namespace.into();
        let key = key.into();
        let resolved = self
            .policy
            .resolve_for_region(&namespace, &self.local_region, &self.placement)
            .map_err(DurabilityError::policy)?;
        if !resolved.persists() {
            self.metrics.ram_only_skipped_total =
                self.metrics.ram_only_skipped_total.saturating_add(1);
            return Ok(DurableWriteOutcome::SkippedRamOnly);
        }

        let watermark = watermark_for(&record);
        match resolved.settings.durability {
            PersistenceDurability::Sync => {
                self.store
                    .upsert(key, record)
                    .map_err(DurabilityError::store)?;
                self.store.flush_durable().map_err(DurabilityError::store)?;
                self.metrics.sync_write_total = self.metrics.sync_write_total.saturating_add(1);
                Ok(DurableWriteOutcome::SyncAcked {
                    watermark,
                    fsync_before_ack: true,
                })
            }
            PersistenceDurability::AsyncBounded { max_lag } => {
                if self.pending.len() >= max_lag {
                    self.metrics.backpressure_total =
                        self.metrics.backpressure_total.saturating_add(1);
                    return Err(DurabilityError::backpressure(format!(
                        "durability async lag bound exceeded for namespace '{namespace}': pending={}, max_lag={max_lag}",
                        self.pending.len()
                    )));
                }
                self.pending.push_back(PendingDurableWrite {
                    namespace,
                    key,
                    record,
                });
                self.metrics.async_queued_total = self.metrics.async_queued_total.saturating_add(1);
                self.metrics.async_lag = self.pending.len();
                Ok(DurableWriteOutcome::Queued {
                    watermark,
                    lag: self.pending.len(),
                })
            }
        }
    }

    /// Persist a tombstone through the same durability routing path as values.
    pub fn tombstone(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
    ) -> Result<DurableWriteOutcome, DurabilityError> {
        self.write(
            namespace,
            key,
            ReplicatedValueRecord::tombstone(partition, version, epoch, None),
        )
    }

    /// Drain all admitted async writes to the underlying store and flush once.
    pub fn drain_async(&mut self) -> Result<usize, DurabilityError> {
        let mut drained = 0_usize;
        while let Some(write) = self.pending.pop_front() {
            let PendingDurableWrite {
                namespace,
                key,
                record,
            } = write;
            if let Err(error) = self.store.upsert(key.clone(), record.clone()) {
                self.pending.push_front(PendingDurableWrite {
                    namespace,
                    key,
                    record,
                });
                self.metrics.async_lag = self.pending.len();
                return Err(DurabilityError::store(error));
            }
            drained = drained.saturating_add(1);
        }
        if drained > 0 {
            self.store.flush_durable().map_err(DurabilityError::store)?;
            self.metrics.async_drained_total = self
                .metrics
                .async_drained_total
                .saturating_add(drained as u64);
        }
        self.metrics.async_lag = self.pending.len();
        Ok(drained)
    }

    /// Flush and record a snapshot manifest if the namespace interval is due.
    pub fn maybe_snapshot(
        &mut self,
        namespace: impl Into<String>,
        now: Duration,
        watermark: WriteWatermark,
    ) -> Result<Option<DurabilitySnapshotManifest>, DurabilityError> {
        let namespace = namespace.into();
        let resolved = self
            .policy
            .resolve_for_region(&namespace, &self.local_region, &self.placement)
            .map_err(DurabilityError::policy)?;
        if !resolved.persists() {
            return Ok(None);
        }
        let Some(interval) = resolved.settings.snapshot_interval else {
            return Ok(None);
        };
        let due = self
            .last_snapshot_at
            .get(&namespace)
            .map(|last| now.saturating_sub(*last) >= interval)
            .unwrap_or(true);
        if !due {
            return Ok(None);
        }

        self.drain_async()?;
        self.store.flush_durable().map_err(DurabilityError::store)?;
        let manifest = DurabilitySnapshotManifest::new(&namespace, watermark, now, interval);
        manifest.verify()?;
        self.last_snapshot_at.insert(namespace, now);
        self.snapshots.push(manifest.clone());
        self.metrics.snapshot_total = self.metrics.snapshot_total.saturating_add(1);
        Ok(Some(manifest))
    }
}

#[derive(Debug, Clone)]
struct PendingDurableWrite {
    namespace: String,
    key: String,
    record: ReplicatedValueRecord,
}

/// Durability write-path error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurabilityError {
    kind: DurabilityErrorKind,
    message: String,
}

impl DurabilityError {
    fn policy(error: PersistencePolicyError) -> Self {
        Self {
            kind: DurabilityErrorKind::Policy,
            message: error.to_string(),
        }
    }

    fn store(error: ValueStoreError) -> Self {
        Self {
            kind: DurabilityErrorKind::Store,
            message: error.to_string(),
        }
    }

    fn backpressure(message: impl Into<String>) -> Self {
        Self {
            kind: DurabilityErrorKind::Backpressure,
            message: message.into(),
        }
    }

    fn snapshot(message: impl Into<String>) -> Self {
        Self {
            kind: DurabilityErrorKind::Snapshot,
            message: message.into(),
        }
    }

    /// Return the stable error kind.
    pub fn kind(&self) -> DurabilityErrorKind {
        self.kind
    }
}

impl fmt::Display for DurabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DurabilityError {}

/// Stable durability error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurabilityErrorKind {
    /// Persistence policy validation failed.
    Policy,
    /// Underlying value store failed.
    Store,
    /// Async bounded queue is full.
    Backpressure,
    /// Snapshot manifest validation failed.
    Snapshot,
}

fn watermark_for(record: &ReplicatedValueRecord) -> WriteWatermark {
    WriteWatermark::new(record.partition, record.version, record.epoch)
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn snapshot_checksum(
    namespace: &str,
    watermark: WriteWatermark,
    created_after_ms: u64,
    interval_ms: u64,
) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut checksum = OFFSET;
    for byte in DURABILITY_SNAPSHOT_FORMAT_VERSION
        .to_le_bytes()
        .iter()
        .chain(namespace.as_bytes())
        .chain(watermark.partition.value().to_le_bytes().iter())
        .chain(watermark.version.to_le_bytes().iter())
        .chain(watermark.epoch.value().to_le_bytes().iter())
        .chain(created_after_ms.to_le_bytes().iter())
        .chain(interval_ms.to_le_bytes().iter())
    {
        checksum ^= u64::from(*byte);
        checksum = checksum.wrapping_mul(PRIME);
    }
    checksum
}
