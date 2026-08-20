use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

const ENTRY_METADATA_BYTES: u64 = 64;
const TAG_MEMBERSHIP_METADATA_BYTES: u64 = 24;
const EXPIRY_METADATA_BYTES: u64 = 16;

/// Runtime cost posture for bounded memory-footprint instrumentation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryInstrumentationMode {
    /// Do not update memory counters. Snapshot shape remains available.
    Off,
    /// Low-cost counters suitable for normal production binaries.
    #[default]
    Production,
    /// Production counters plus phase/provider correlation in profiling builds.
    Profile,
}

/// Whether all subsystem observations belonged to one acknowledged epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySnapshotConsistency {
    Exact,
    ObservedNonAtomic,
}

/// Token returned only while the local mutation counter is quiescent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySnapshotBarrier {
    pub epoch: u64,
}

/// Snapshot request. Exact requests must echo a barrier obtained from the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySnapshotRequest {
    Admin,
    Exact { acknowledged_epoch: u64 },
}

/// A bounded observation associated with one ownership-registry id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryOwnerObservation {
    pub owner_id: String,
    pub records: u64,
    pub logical_bytes: u64,
    pub estimated_retained_bytes: u64,
    pub available: bool,
}

/// Before/after sequence for one bounded subsystem observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySubsystemVersion {
    pub owner_id: String,
    pub before: u64,
    pub after: u64,
    pub stable: bool,
}

/// Allocator observation supplied by an external provider adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllocatorMemoryObservation {
    pub provider: String,
    pub resident_bytes: Option<u64>,
    pub allocated_bytes: Option<u64>,
    pub active_bytes: Option<u64>,
    pub retained_bytes: Option<u64>,
    pub fragmentation_ratio: Option<f64>,
    pub unavailable_reason: Option<String>,
}

impl Default for AllocatorMemoryObservation {
    fn default() -> Self {
        Self {
            provider: "unavailable".to_owned(),
            resident_bytes: None,
            allocated_bytes: None,
            active_bytes: None,
            retained_bytes: None,
            fragmentation_ratio: None,
            unavailable_reason: Some("no allocator provider observation attached".to_owned()),
        }
    }
}

/// Aggregate observations owned by optional client/server/durable surfaces.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalMemoryObservation {
    pub client_surface_entries: Option<u64>,
    pub client_surface_bytes: Option<u64>,
    pub idempotency_records: Option<u64>,
    pub idempotency_bytes: Option<u64>,
    pub idempotency_oldest_age_seconds: Option<u64>,
    pub audit_records: Option<u64>,
    pub audit_bytes: Option<u64>,
    pub audit_sink_failures: Option<u64>,
    pub outbound_queued_bytes: Option<u64>,
    pub subscriptions: Option<u64>,
    pub sessions: Option<u64>,
    pub connections: Option<u64>,
    pub durable_logical_bytes: Option<u64>,
    pub file_backed_bytes: Option<u64>,
    pub unavailable_reason: Option<String>,
}

/// Bounded, secret-free aggregate memory snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryFootprintSnapshot {
    pub schema_version: String,
    pub mode: MemoryInstrumentationMode,
    pub epoch: u64,
    pub consistency: MemorySnapshotConsistency,
    pub workload_epoch_acknowledged: bool,
    pub observed_non_atomic: bool,
    pub subsystem_version_before: u64,
    pub subsystem_version_after: u64,
    pub live_entries: u64,
    pub logical_key_bytes: u64,
    pub logical_value_bytes: u64,
    pub tag_memberships: u64,
    pub tag_generation_records: u64,
    pub key_generation_records: u64,
    pub expiry_records: u64,
    pub estimated_retained_bytes: u64,
    pub pending_loads: u64,
    pub event_ring_occupancy: u64,
    pub event_ring_bytes: Option<u64>,
    pub event_subscribers: u64,
    pub event_dropped: u64,
    pub owners: Vec<MemoryOwnerObservation>,
    pub subsystem_versions: Vec<MemorySubsystemVersion>,
    pub allocator: AllocatorMemoryObservation,
    pub external: ExternalMemoryObservation,
}

/// A discrepancy found by the explicit exact reconciliation path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryReconciliationReport {
    pub matched: bool,
    pub counter_entries: u64,
    pub exact_entries: u64,
    pub counter_key_bytes: u64,
    pub exact_key_bytes: u64,
    pub counter_value_bytes: u64,
    pub exact_value_bytes: u64,
    pub counter_tag_memberships: u64,
    pub exact_tag_memberships: u64,
}

/// Fail-loud instrumentation errors. No counter is silently wrapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryFootprintError {
    CounterFault,
    NotQuiescent,
    EpochMismatch { expected: u64, acknowledged: u64 },
    ReconciliationMismatch,
}

impl fmt::Display for MemoryFootprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CounterFault => formatter.write_str("memory counter overflow or underflow"),
            Self::NotQuiescent => formatter.write_str("exact memory snapshot requires quiescence"),
            Self::EpochMismatch {
                expected,
                acknowledged,
            } => write!(
                formatter,
                "memory snapshot epoch mismatch: expected {expected}, acknowledged {acknowledged}"
            ),
            Self::ReconciliationMismatch => {
                formatter.write_str("incremental memory counters failed exact reconciliation")
            }
        }
    }
}

impl std::error::Error for MemoryFootprintError {}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EntryMemoryDelta {
    pub(crate) entries: u64,
    pub(crate) key_bytes: u64,
    pub(crate) value_bytes: u64,
    pub(crate) tag_memberships: u64,
    pub(crate) expiry_records: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MemoryCaptureInput {
    pub(crate) pending_loads: u64,
    pub(crate) event_ring_occupancy: u64,
    pub(crate) event_subscribers: u64,
    pub(crate) event_dropped: u64,
    pub(crate) tag_version_before: u64,
    pub(crate) tag_version_after: u64,
    pub(crate) tag_generation_records: u64,
    pub(crate) key_generation_records: u64,
}

impl EntryMemoryDelta {
    pub(crate) fn new(key: &str, value_bytes: usize, tags: usize, expires: bool) -> Self {
        Self {
            entries: 1,
            key_bytes: key.len() as u64,
            value_bytes: value_bytes as u64,
            tag_memberships: tags as u64,
            expiry_records: u64::from(expires),
        }
    }

    fn estimated_retained_bytes(self) -> u64 {
        self.key_bytes
            .saturating_add(self.value_bytes)
            .saturating_add(self.entries.saturating_mul(ENTRY_METADATA_BYTES))
            .saturating_add(
                self.tag_memberships
                    .saturating_mul(TAG_MEMBERSHIP_METADATA_BYTES),
            )
            .saturating_add(self.expiry_records.saturating_mul(EXPIRY_METADATA_BYTES))
    }
}

#[derive(Debug)]
pub(crate) struct MemoryFootprintCounters {
    mode: MemoryInstrumentationMode,
    epoch: AtomicU64,
    version: AtomicU64,
    active_mutations: AtomicU64,
    faulted: AtomicBool,
    entries: AtomicU64,
    key_bytes: AtomicU64,
    value_bytes: AtomicU64,
    tag_memberships: AtomicU64,
    expiry_records: AtomicU64,
}

impl MemoryFootprintCounters {
    pub(crate) fn new(mode: MemoryInstrumentationMode) -> Self {
        Self {
            mode,
            epoch: AtomicU64::new(0),
            version: AtomicU64::new(0),
            active_mutations: AtomicU64::new(0),
            faulted: AtomicBool::new(false),
            entries: AtomicU64::new(0),
            key_bytes: AtomicU64::new(0),
            value_bytes: AtomicU64::new(0),
            tag_memberships: AtomicU64::new(0),
            expiry_records: AtomicU64::new(0),
        }
    }

    pub(crate) fn mutation(&self) -> MemoryMutationGuard<'_> {
        if self.mode != MemoryInstrumentationMode::Off
            && self
                .active_mutations
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                    value.checked_add(1)
                })
                .is_err()
        {
            self.faulted.store(true, Ordering::Release);
        }
        MemoryMutationGuard { counters: self }
    }

    pub(crate) fn insert(&self, new: EntryMemoryDelta) {
        if self.mode == MemoryInstrumentationMode::Off {
            return;
        }
        self.add(new);
    }

    pub(crate) fn remove(&self, old: EntryMemoryDelta) {
        if self.mode != MemoryInstrumentationMode::Off {
            self.subtract(old);
        }
    }

    fn add(&self, delta: EntryMemoryDelta) {
        self.checked_add(&self.entries, delta.entries);
        self.checked_add(&self.key_bytes, delta.key_bytes);
        self.checked_add(&self.value_bytes, delta.value_bytes);
        self.checked_add(&self.tag_memberships, delta.tag_memberships);
        self.checked_add(&self.expiry_records, delta.expiry_records);
    }

    fn subtract(&self, delta: EntryMemoryDelta) {
        self.checked_sub(&self.entries, delta.entries);
        self.checked_sub(&self.key_bytes, delta.key_bytes);
        self.checked_sub(&self.value_bytes, delta.value_bytes);
        self.checked_sub(&self.tag_memberships, delta.tag_memberships);
        self.checked_sub(&self.expiry_records, delta.expiry_records);
    }

    fn checked_add(&self, counter: &AtomicU64, delta: u64) {
        if counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(delta)
            })
            .is_err()
        {
            self.faulted.store(true, Ordering::Release);
        }
    }

    fn checked_sub(&self, counter: &AtomicU64, delta: u64) {
        if counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_sub(delta)
            })
            .is_err()
        {
            self.faulted.store(true, Ordering::Release);
        }
    }

    pub(crate) fn barrier(&self) -> Result<MemorySnapshotBarrier, MemoryFootprintError> {
        self.ensure_healthy()?;
        if self.active_mutations.load(Ordering::Acquire) != 0 {
            return Err(MemoryFootprintError::NotQuiescent);
        }
        let epoch = self
            .epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| MemoryFootprintError::CounterFault)?
            + 1;
        Ok(MemorySnapshotBarrier { epoch })
    }

    pub(crate) fn capture(
        &self,
        request: MemorySnapshotRequest,
        input: MemoryCaptureInput,
    ) -> Result<MemoryFootprintSnapshot, MemoryFootprintError> {
        let MemoryCaptureInput {
            pending_loads,
            event_ring_occupancy,
            event_subscribers,
            event_dropped,
            tag_version_before,
            tag_version_after,
            tag_generation_records,
            key_generation_records,
        } = input;
        self.ensure_healthy()?;
        let before = self.version.load(Ordering::Acquire);
        let active_before = self.active_mutations.load(Ordering::Acquire);
        let delta = EntryMemoryDelta {
            entries: self.entries.load(Ordering::Acquire),
            key_bytes: self.key_bytes.load(Ordering::Acquire),
            value_bytes: self.value_bytes.load(Ordering::Acquire),
            tag_memberships: self.tag_memberships.load(Ordering::Acquire),
            expiry_records: self.expiry_records.load(Ordering::Acquire),
        };
        let active_after = self.active_mutations.load(Ordering::Acquire);
        let after = self.version.load(Ordering::Acquire);
        let stable = active_before == 0
            && active_after == 0
            && before == after
            && tag_version_before == tag_version_after;
        let epoch = self.epoch.load(Ordering::Acquire);
        let workload_epoch_acknowledged = matches!(request, MemorySnapshotRequest::Exact { .. });
        let consistency = match request {
            MemorySnapshotRequest::Admin if stable => MemorySnapshotConsistency::Exact,
            MemorySnapshotRequest::Admin => MemorySnapshotConsistency::ObservedNonAtomic,
            MemorySnapshotRequest::Exact { acknowledged_epoch } => {
                if acknowledged_epoch != epoch {
                    return Err(MemoryFootprintError::EpochMismatch {
                        expected: epoch,
                        acknowledged: acknowledged_epoch,
                    });
                }
                if !stable {
                    return Err(MemoryFootprintError::NotQuiescent);
                }
                MemorySnapshotConsistency::Exact
            }
        };
        let estimated_retained_bytes = delta.estimated_retained_bytes();
        let owners = vec![
            MemoryOwnerObservation {
                owner_id: "owner-85bde59bce494e82".to_owned(),
                records: delta.entries,
                logical_bytes: delta.key_bytes.saturating_add(delta.value_bytes),
                estimated_retained_bytes,
                available: self.mode != MemoryInstrumentationMode::Off,
            },
            MemoryOwnerObservation {
                owner_id: "owner-f184fb44837b0512".to_owned(),
                records: delta
                    .tag_memberships
                    .saturating_add(tag_generation_records)
                    .saturating_add(key_generation_records),
                logical_bytes: 0,
                estimated_retained_bytes: delta
                    .tag_memberships
                    .saturating_mul(TAG_MEMBERSHIP_METADATA_BYTES),
                available: self.mode != MemoryInstrumentationMode::Off,
            },
            MemoryOwnerObservation {
                owner_id: "owner-4c66c374f1166c86".to_owned(),
                records: pending_loads,
                logical_bytes: 0,
                estimated_retained_bytes: 0,
                available: true,
            },
            MemoryOwnerObservation {
                owner_id: "owner-85bde59bce494e82".to_owned(),
                records: event_ring_occupancy,
                logical_bytes: 0,
                estimated_retained_bytes: 0,
                available: true,
            },
        ];
        let subsystem_versions = vec![
            MemorySubsystemVersion {
                owner_id: "owner-85bde59bce494e82".to_owned(),
                before,
                after,
                stable: before == after && active_before == 0 && active_after == 0,
            },
            MemorySubsystemVersion {
                owner_id: "owner-f184fb44837b0512".to_owned(),
                before: tag_version_before,
                after: tag_version_after,
                stable: tag_version_before == tag_version_after,
            },
        ];
        Ok(MemoryFootprintSnapshot {
            schema_version: "hydracache-memory-footprint-v1".to_owned(),
            mode: self.mode,
            epoch,
            consistency,
            workload_epoch_acknowledged,
            observed_non_atomic: consistency == MemorySnapshotConsistency::ObservedNonAtomic,
            subsystem_version_before: before,
            subsystem_version_after: after,
            live_entries: delta.entries,
            logical_key_bytes: delta.key_bytes,
            logical_value_bytes: delta.value_bytes,
            tag_memberships: delta.tag_memberships,
            tag_generation_records,
            key_generation_records,
            expiry_records: delta.expiry_records,
            estimated_retained_bytes,
            pending_loads,
            event_ring_occupancy,
            event_ring_bytes: None,
            event_subscribers,
            event_dropped,
            owners,
            subsystem_versions,
            allocator: AllocatorMemoryObservation::default(),
            external: ExternalMemoryObservation {
                unavailable_reason: Some(
                    "optional client/server/durable surfaces are not attached to this cache"
                        .to_owned(),
                ),
                ..ExternalMemoryObservation::default()
            },
        })
    }

    pub(crate) fn reconcile(
        &self,
        exact: EntryMemoryDelta,
    ) -> Result<MemoryReconciliationReport, MemoryFootprintError> {
        self.ensure_healthy()?;
        let report = MemoryReconciliationReport {
            matched: self.entries.load(Ordering::Acquire) == exact.entries
                && self.key_bytes.load(Ordering::Acquire) == exact.key_bytes
                && self.value_bytes.load(Ordering::Acquire) == exact.value_bytes
                && self.tag_memberships.load(Ordering::Acquire) == exact.tag_memberships,
            counter_entries: self.entries.load(Ordering::Acquire),
            exact_entries: exact.entries,
            counter_key_bytes: self.key_bytes.load(Ordering::Acquire),
            exact_key_bytes: exact.key_bytes,
            counter_value_bytes: self.value_bytes.load(Ordering::Acquire),
            exact_value_bytes: exact.value_bytes,
            counter_tag_memberships: self.tag_memberships.load(Ordering::Acquire),
            exact_tag_memberships: exact.tag_memberships,
        };
        if report.matched {
            Ok(report)
        } else {
            Err(MemoryFootprintError::ReconciliationMismatch)
        }
    }

    fn ensure_healthy(&self) -> Result<(), MemoryFootprintError> {
        if self.faulted.load(Ordering::Acquire) {
            Err(MemoryFootprintError::CounterFault)
        } else {
            Ok(())
        }
    }
}

pub(crate) struct MemoryMutationGuard<'a> {
    counters: &'a MemoryFootprintCounters,
}

impl Drop for MemoryMutationGuard<'_> {
    fn drop(&mut self) {
        if self.counters.mode == MemoryInstrumentationMode::Off {
            return;
        }
        if self
            .counters
            .active_mutations
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_sub(1)
            })
            .is_err()
        {
            self.counters.faulted.store(true, Ordering::Release);
        }
        if self
            .counters
            .version
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .is_err()
        {
            self.counters.faulted.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_admin_read_is_marked_non_atomic() {
        let counters = MemoryFootprintCounters::new(MemoryInstrumentationMode::Production);
        let mutation = counters.mutation();
        let snapshot = counters
            .capture(MemorySnapshotRequest::Admin, MemoryCaptureInput::default())
            .expect("admin snapshot");
        assert_eq!(
            snapshot.consistency,
            MemorySnapshotConsistency::ObservedNonAtomic
        );
        drop(mutation);
    }

    #[test]
    fn exact_snapshot_rejects_epoch_mismatch() {
        let counters = MemoryFootprintCounters::new(MemoryInstrumentationMode::Production);
        let barrier = counters.barrier().expect("barrier");
        let error = counters
            .capture(
                MemorySnapshotRequest::Exact {
                    acknowledged_epoch: barrier.epoch + 1,
                },
                MemoryCaptureInput::default(),
            )
            .expect_err("mismatched epoch must fail");
        assert!(matches!(error, MemoryFootprintError::EpochMismatch { .. }));
    }

    #[test]
    fn hidden_secondary_index_mismatch_fails_loud() {
        let counters = MemoryFootprintCounters::new(MemoryInstrumentationMode::Production);
        {
            let _mutation = counters.mutation();
            counters.insert(EntryMemoryDelta::new("key", 8, 1, true));
        }
        let hidden_membership = EntryMemoryDelta {
            entries: 1,
            key_bytes: 3,
            value_bytes: 8,
            tag_memberships: 2,
            expiry_records: 1,
        };
        assert_eq!(
            counters.reconcile(hidden_membership),
            Err(MemoryFootprintError::ReconciliationMismatch)
        );
    }
}
