use serde::{Deserialize, Serialize};

use crate::grid::hardening::{ReplicatedValueRecord, ReplicatedValueStore, ValueStoreError};

/// Checksum status reported by the durable value inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurableInspectChecksumStatus {
    /// The record decoded successfully and its durable checksum verified.
    Verified,
}

/// Domain-level view of one durable replicated value record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableInspectRecord {
    /// Cache key stored in the durable value plane.
    pub key: String,
    /// Owning partition id.
    pub partition: u32,
    /// Record version.
    pub version: u64,
    /// Cluster epoch that produced this record.
    pub epoch: u64,
    /// Whether the record is a tombstone.
    pub tombstone: bool,
    /// Approximate bytes charged to the durable value budget.
    pub approx_bytes: u64,
    /// Durable checksum status.
    pub checksum_status: DurableInspectChecksumStatus,
}

impl DurableInspectRecord {
    fn from_record(key: String, record: ReplicatedValueRecord) -> Self {
        Self {
            key,
            partition: record.partition.value(),
            version: record.version,
            epoch: record.epoch.value(),
            tombstone: record.is_tombstone(),
            approx_bytes: record.approx_bytes(),
            checksum_status: DurableInspectChecksumStatus::Verified,
        }
    }
}

/// Inspect every record in a replicated value store.
///
/// Durable stores decode and checksum records inside `scan_all`, so a corrupt
/// record fails this dump loudly instead of being presented as healthy data.
pub fn inspect_replicated_store<S>(store: &S) -> Result<Vec<DurableInspectRecord>, ValueStoreError>
where
    S: ReplicatedValueStore,
{
    let mut records = store.scan_all()?;
    records.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(records
        .into_iter()
        .map(|(key, record)| DurableInspectRecord::from_record(key, record))
        .collect())
}
