//! Provisional, deterministic 0.75 fairness and namespace-lifecycle reference models.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionLimits {
    pub total: usize,
    pub per_tenant: usize,
    pub per_partition: usize,
    pub safety: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataWork {
    pub tenant: String,
    pub partition: u32,
    pub request_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkItem {
    Safety { operation: String },
    Data(DataWork),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    InvalidBound(&'static str),
    TotalQueueFull,
    SafetyQueueFull,
    TenantQueueFull(String),
    PartitionQueueFull { tenant: String, partition: u32 },
}

/// Bounded admission with an isolated safety lane and deterministic tenant round-robin.
#[derive(Debug)]
pub struct FairAdmissionQueue {
    limits: AdmissionLimits,
    safety: VecDeque<WorkItem>,
    tenants: BTreeMap<String, VecDeque<DataWork>>,
    tenant_order: Vec<String>,
    cursor: usize,
    data_len: usize,
}

impl FairAdmissionQueue {
    pub fn new(limits: AdmissionLimits) -> Result<Self, AdmissionError> {
        for (name, value) in [
            ("total", limits.total),
            ("per_tenant", limits.per_tenant),
            ("per_partition", limits.per_partition),
            ("safety", limits.safety),
        ] {
            if value == 0 {
                return Err(AdmissionError::InvalidBound(name));
            }
        }
        Ok(Self {
            limits,
            safety: VecDeque::new(),
            tenants: BTreeMap::new(),
            tenant_order: Vec::new(),
            cursor: 0,
            data_len: 0,
        })
    }

    pub fn admit_safety(&mut self, operation: impl Into<String>) -> Result<(), AdmissionError> {
        if self.safety.len() >= self.limits.safety {
            return Err(AdmissionError::SafetyQueueFull);
        }
        self.safety.push_back(WorkItem::Safety {
            operation: operation.into(),
        });
        Ok(())
    }

    pub fn admit_data(&mut self, work: DataWork) -> Result<(), AdmissionError> {
        if self.data_len >= self.limits.total {
            return Err(AdmissionError::TotalQueueFull);
        }
        let queue = self.tenants.entry(work.tenant.clone()).or_default();
        if queue.len() >= self.limits.per_tenant {
            return Err(AdmissionError::TenantQueueFull(work.tenant));
        }
        let partition_len = queue
            .iter()
            .filter(|queued| queued.partition == work.partition)
            .count();
        if partition_len >= self.limits.per_partition {
            return Err(AdmissionError::PartitionQueueFull {
                tenant: work.tenant,
                partition: work.partition,
            });
        }
        if queue.is_empty() {
            self.tenant_order.push(work.tenant.clone());
            self.tenant_order.sort();
        }
        queue.push_back(work);
        self.data_len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<WorkItem> {
        if let Some(safety) = self.safety.pop_front() {
            return Some(safety);
        }
        if self.tenant_order.is_empty() {
            return None;
        }
        for _ in 0..self.tenant_order.len() {
            self.cursor %= self.tenant_order.len();
            let tenant = self.tenant_order[self.cursor].clone();
            self.cursor = (self.cursor + 1) % self.tenant_order.len();
            if let Some(work) = self.tenants.get_mut(&tenant).and_then(VecDeque::pop_front) {
                self.data_len -= 1;
                return Some(WorkItem::Data(work));
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.data_len + self.safety.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespacePhase {
    Active,
    Draining,
    DeleteCommitted,
    Reclaimable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReclamationWatermarks {
    pub replica_applied: u64,
    pub listener_cutover: u64,
    pub dedup_expired: u64,
    pub transfer_closed: u64,
    pub tombstone_gc: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReclamationOwners {
    pub entries: usize,
    pub ttl_tasks: usize,
    pub tombstones: usize,
    pub dedup_results: usize,
    pub subscriptions: usize,
    pub staging_files: usize,
    pub quota_owners: usize,
}

impl ReclamationOwners {
    pub fn total(self) -> usize {
        self.entries
            .saturating_add(self.ttl_tasks)
            .saturating_add(self.tombstones)
            .saturating_add(self.dedup_results)
            .saturating_add(self.subscriptions)
            .saturating_add(self.staging_files)
            .saturating_add(self.quota_owners)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceError {
    StaleGeneration { expected: u64, actual: u64 },
    NotActive,
    NotDraining,
    NotReclaimable,
    OwnersRemain(usize),
    InvalidBound(&'static str),
    InvalidGeneration,
}

impl fmt::Display for NamespaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for NamespaceError {}

/// Generation-fenced delete/reclaim/recreate state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceLifecycle {
    generation: u64,
    phase: NamespacePhase,
    delete_watermark: Option<u64>,
    watermarks: ReclamationWatermarks,
    owners: ReclamationOwners,
}

impl NamespaceLifecycle {
    pub fn new(generation: u64) -> Result<Self, NamespaceError> {
        if generation == 0 {
            return Err(NamespaceError::InvalidGeneration);
        }
        Ok(Self {
            generation,
            phase: NamespacePhase::Active,
            delete_watermark: None,
            watermarks: ReclamationWatermarks::default(),
            owners: ReclamationOwners::default(),
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn phase(&self) -> NamespacePhase {
        self.phase
    }

    pub fn validate_request(&self, generation: u64) -> Result<(), NamespaceError> {
        if generation != self.generation {
            return Err(NamespaceError::StaleGeneration {
                expected: self.generation,
                actual: generation,
            });
        }
        if self.phase != NamespacePhase::Active {
            return Err(NamespaceError::NotActive);
        }
        Ok(())
    }

    pub fn begin_delete(&mut self) -> Result<(), NamespaceError> {
        if self.phase != NamespacePhase::Active {
            return Err(NamespaceError::NotActive);
        }
        self.phase = NamespacePhase::Draining;
        Ok(())
    }

    pub fn commit_delete(
        &mut self,
        watermark: u64,
        owners: ReclamationOwners,
    ) -> Result<(), NamespaceError> {
        if self.phase != NamespacePhase::Draining {
            return Err(NamespaceError::NotDraining);
        }
        if watermark == 0 {
            return Err(NamespaceError::InvalidGeneration);
        }
        self.phase = NamespacePhase::DeleteCommitted;
        self.delete_watermark = Some(watermark);
        self.owners = owners;
        Ok(())
    }

    pub fn advance_reclamation(&mut self, watermarks: ReclamationWatermarks) {
        self.watermarks.replica_applied = self
            .watermarks
            .replica_applied
            .max(watermarks.replica_applied);
        self.watermarks.listener_cutover = self
            .watermarks
            .listener_cutover
            .max(watermarks.listener_cutover);
        self.watermarks.dedup_expired = self.watermarks.dedup_expired.max(watermarks.dedup_expired);
        self.watermarks.transfer_closed = self
            .watermarks
            .transfer_closed
            .max(watermarks.transfer_closed);
        self.watermarks.tombstone_gc = self.watermarks.tombstone_gc.max(watermarks.tombstone_gc);
        if let Some(required) = self.delete_watermark {
            if self.phase == NamespacePhase::DeleteCommitted
                && self.watermarks.replica_applied >= required
                && self.watermarks.listener_cutover >= required
                && self.watermarks.dedup_expired >= required
                && self.watermarks.transfer_closed >= required
                && self.watermarks.tombstone_gc >= required
            {
                self.phase = NamespacePhase::Reclaimable;
            }
        }
    }

    pub fn reclaim_step(&mut self, max_items: usize) -> Result<usize, NamespaceError> {
        if max_items == 0 {
            return Err(NamespaceError::InvalidBound("reclaim_items"));
        }
        if self.phase != NamespacePhase::Reclaimable {
            return Err(NamespaceError::NotReclaimable);
        }
        let before = self.owners.total();
        let mut remaining = max_items;
        for owner in [
            &mut self.owners.entries,
            &mut self.owners.ttl_tasks,
            &mut self.owners.tombstones,
            &mut self.owners.dedup_results,
            &mut self.owners.subscriptions,
            &mut self.owners.staging_files,
            &mut self.owners.quota_owners,
        ] {
            let removed = (*owner).min(remaining);
            *owner -= removed;
            remaining -= removed;
            if remaining == 0 {
                break;
            }
        }
        Ok(before.saturating_sub(self.owners.total()))
    }

    pub const fn owners(&self) -> ReclamationOwners {
        self.owners
    }

    pub fn recreate(&mut self) -> Result<u64, NamespaceError> {
        if self.phase != NamespacePhase::Reclaimable {
            return Err(NamespaceError::NotReclaimable);
        }
        if self.owners.total() != 0 {
            return Err(NamespaceError::OwnersRemain(self.owners.total()));
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(NamespaceError::InvalidGeneration)?;
        self.phase = NamespacePhase::Active;
        self.delete_watermark = None;
        self.watermarks = ReclamationWatermarks::default();
        self.owners = ReclamationOwners::default();
        Ok(self.generation)
    }
}
