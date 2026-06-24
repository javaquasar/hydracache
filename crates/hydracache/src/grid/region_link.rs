use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, PartitionId};
use crate::grid::active_active::GeoWrite;
use crate::grid::elasticity::RegionId;
use crate::grid::hardening::{AdaptiveWindow, MergePolicy, ReplicatedValueRecord, ValueVersion};
use crate::grid::residency::{ResidencyLinkSendReport, ResidencyPolicyEnforcer};

/// Idempotency key attached to a cross-region replication write.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Create a key.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for IdempotencyKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for IdempotencyKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// WAN replication batch. Compression is represented by deterministic accounting;
/// adapters can map the payload to their concrete wire codec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoBatch {
    /// Peer region.
    pub peer: RegionId,
    /// Writes carried by this batch.
    pub entries: Vec<GeoWrite>,
    /// Idempotency keys paired with entries.
    pub idem_keys: Vec<IdempotencyKey>,
    /// Estimated uncompressed bytes.
    pub raw_bytes: u64,
    /// Estimated compressed bytes.
    pub compressed_bytes: u64,
}

impl GeoBatch {
    /// Build a deterministic batch and reject mismatched idempotency vectors.
    pub fn new(
        peer: impl Into<RegionId>,
        entries: Vec<GeoWrite>,
        idem_keys: Vec<IdempotencyKey>,
    ) -> Result<Self, RegionLinkError> {
        if entries.len() != idem_keys.len() {
            return Err(RegionLinkError::new(
                "geo batch entries and idempotency keys must have the same length",
            ));
        }
        let raw_bytes = estimate_batch_bytes(&entries, &idem_keys);
        let compressed_bytes = compressed_size(raw_bytes);
        Ok(Self {
            peer: peer.into(),
            entries,
            idem_keys,
            raw_bytes,
            compressed_bytes,
        })
    }

    /// Return whether compression reduced or preserved the encoded size.
    pub fn is_compressed(&self) -> bool {
        self.compressed_bytes <= self.raw_bytes
    }
}

fn estimate_batch_bytes(entries: &[GeoWrite], keys: &[IdempotencyKey]) -> u64 {
    entries
        .iter()
        .zip(keys)
        .map(|(entry, key)| {
            entry.key.len() as u64 + entry.value.len() as u64 + key.as_str().len() as u64 + 48
        })
        .sum::<u64>()
        .max(1)
}

fn compressed_size(raw_bytes: u64) -> u64 {
    raw_bytes.saturating_mul(7).saturating_add(9) / 10
}

/// Error returned by region-link helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionLinkError {
    message: String,
}

impl RegionLinkError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RegionLinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RegionLinkError {}

/// Apply report for one received batch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoBatchApplyReport {
    /// Entries applied with at-most-once effect.
    pub applied: u64,
    /// Entries skipped because the idempotency key was already seen.
    pub deduped: u64,
    /// Compressed bytes accepted.
    pub bytes_total: u64,
}

/// WAN-aware replication link state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionLink {
    peer: RegionId,
    window: AdaptiveWindow,
    seen: BTreeSet<IdempotencyKey>,
    lag: u64,
    bytes_total: u64,
}

impl RegionLink {
    /// Create a link with a WAN-tuned AIMD window.
    pub fn new(peer: impl Into<RegionId>, floor: usize, initial: usize, ceil: usize) -> Self {
        Self {
            peer: peer.into(),
            window: AdaptiveWindow::new(floor, initial, ceil),
            seen: BTreeSet::new(),
            lag: 0,
            bytes_total: 0,
        }
    }

    /// Return the peer region.
    pub fn peer(&self) -> &RegionId {
        &self.peer
    }

    /// Return the current flow-control window.
    pub fn window(&self) -> AdaptiveWindow {
        self.window
    }

    /// Return link lag in queued batches.
    pub fn lag(&self) -> u64 {
        self.lag
    }

    /// Return total compressed bytes accepted by this link.
    pub fn bytes_total(&self) -> u64 {
        self.bytes_total
    }

    /// Admit a batch for sending, or account it as lag when the WAN window is full.
    pub fn try_send(&mut self, batch: &GeoBatch) -> bool {
        if self.window.try_acquire() {
            self.bytes_total = self.bytes_total.saturating_add(batch.compressed_bytes);
            true
        } else {
            self.lag = self.lag.saturating_add(1);
            false
        }
    }

    /// Admit a batch only if residency policy allows every value to cross this link.
    pub fn try_send_with_residency<F>(
        &mut self,
        batch: &GeoBatch,
        enforcer: &mut ResidencyPolicyEnforcer,
        namespace_for_key: F,
    ) -> ResidencyLinkSendReport
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut report = ResidencyLinkSendReport::default();
        for write in &batch.entries {
            let Some(namespace) = namespace_for_key(&write.key) else {
                continue;
            };
            report.checked = report.checked.saturating_add(1);
            if enforcer
                .guard_cross_boundary(&namespace, &write.key, &write.origin_region, &self.peer)
                .is_err()
            {
                report.refused = report.refused.saturating_add(1);
            }
        }
        if report.refused > 0 {
            return report;
        }
        report.sent = self.try_send(batch);
        report
    }

    /// Record a link acknowledgement and update backpressure.
    pub fn on_ack(&mut self, rtt_ok: bool) {
        self.window.on_ack(rtt_ok);
        if rtt_ok {
            self.lag = self.lag.saturating_sub(1);
        } else {
            self.lag = self.lag.saturating_add(1);
        }
    }

    /// Apply a received batch with idempotency dedupe.
    pub fn apply_batch(
        &mut self,
        batch: &GeoBatch,
        records: &mut BTreeMap<String, ReplicatedValueRecord>,
        policy: &dyn MergePolicy,
    ) -> GeoBatchApplyReport {
        let mut report = GeoBatchApplyReport {
            bytes_total: batch.compressed_bytes,
            ..GeoBatchApplyReport::default()
        };
        for (write, key) in batch.entries.iter().zip(&batch.idem_keys) {
            if !self.seen.insert(key.clone()) {
                report.deduped = report.deduped.saturating_add(1);
                continue;
            }
            let incoming = write.to_record();
            let merged = policy
                .merge(records.get(&write.key), &incoming)
                .unwrap_or(incoming);
            records.insert(write.key.clone(), merged);
            report.applied = report.applied.saturating_add(1);
        }
        self.bytes_total = self.bytes_total.saturating_add(batch.compressed_bytes);
        report
    }
}

/// Per-partition version summary for cross-region anti-entropy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionSummary {
    entries: BTreeMap<String, (ValueVersion, ClusterEpoch)>,
}

impl VersionSummary {
    /// Create an empty summary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one key version.
    pub fn insert(&mut self, key: impl Into<String>, version: ValueVersion, epoch: ClusterEpoch) {
        self.entries.insert(key.into(), (version, epoch));
    }

    /// Return one key version.
    pub fn get(&self, key: &str) -> Option<(ValueVersion, ClusterEpoch)> {
        self.entries.get(key).copied()
    }
}

/// Digest exchanged by regions during anti-entropy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionDigest {
    /// Partition summarized by this digest.
    pub partition: PartitionId,
    /// Per-key version summary.
    pub summary: VersionSummary,
}

impl PartitionDigest {
    /// Create a digest.
    pub fn new(partition: PartitionId, summary: VersionSummary) -> Self {
        Self { partition, summary }
    }
}

/// Return keys where `local` is newer or missing from `remote`.
pub fn anti_entropy_diff(local: &PartitionDigest, remote: &PartitionDigest) -> Vec<String> {
    if local.partition != remote.partition {
        return local.summary.entries.keys().cloned().collect();
    }
    local
        .summary
        .entries
        .iter()
        .filter(|entry| {
            let (key, local_stamp) = *entry;
            remote
                .summary
                .get(key)
                .map(|remote_stamp| *local_stamp > remote_stamp)
                .unwrap_or(true)
        })
        .map(|(key, _)| key.clone())
        .collect()
}

/// Confirmation gate for CRDT metadata GC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtMetadataGcGate {
    required: BTreeSet<RegionId>,
    confirmed: BTreeSet<RegionId>,
}

impl CrdtMetadataGcGate {
    /// Create a gate requiring confirmation from every region.
    pub fn new(required: impl IntoIterator<Item = RegionId>) -> Self {
        Self {
            required: required.into_iter().collect(),
            confirmed: BTreeSet::new(),
        }
    }

    /// Mark a region confirmed.
    pub fn confirm(&mut self, region: RegionId) {
        if self.required.contains(&region) {
            self.confirmed.insert(region);
        }
    }

    /// Return whether GC is allowed.
    pub fn can_collect(&self) -> bool {
        self.required.is_subset(&self.confirmed)
    }
}
