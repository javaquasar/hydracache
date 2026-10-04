//! Provisional, deterministic 0.75 fairness and namespace-lifecycle reference models.

use std::collections::{BTreeMap, VecDeque};

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
    Deleting,
    Reclaimable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceError {
    StaleGeneration { expected: u64, actual: u64 },
    NotActive,
    NotReclaimable,
    InvalidGeneration,
}

/// Generation-fenced delete/reclaim/recreate state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceLifecycle {
    generation: u64,
    phase: NamespacePhase,
    delete_watermark: Option<u64>,
    delete_applied: u64,
    listener_cutover: u64,
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
            delete_applied: 0,
            listener_cutover: 0,
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

    pub fn begin_delete(&mut self, watermark: u64) -> Result<(), NamespaceError> {
        if self.phase != NamespacePhase::Active {
            return Err(NamespaceError::NotActive);
        }
        self.phase = NamespacePhase::Deleting;
        self.delete_watermark = Some(watermark);
        Ok(())
    }

    pub fn advance_reclamation(&mut self, delete_applied: u64, listener_cutover: u64) {
        self.delete_applied = self.delete_applied.max(delete_applied);
        self.listener_cutover = self.listener_cutover.max(listener_cutover);
        if let Some(required) = self.delete_watermark {
            if self.phase == NamespacePhase::Deleting
                && self.delete_applied >= required
                && self.listener_cutover >= required
            {
                self.phase = NamespacePhase::Reclaimable;
            }
        }
    }

    pub fn recreate(&mut self) -> Result<u64, NamespaceError> {
        if self.phase != NamespacePhase::Reclaimable {
            return Err(NamespaceError::NotReclaimable);
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(NamespaceError::InvalidGeneration)?;
        self.phase = NamespacePhase::Active;
        self.delete_watermark = None;
        self.delete_applied = 0;
        self.listener_cutover = 0;
        Ok(self.generation)
    }
}
