//! Bounded, process-local evidence for read-only management operation and audit views.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Maximum terminal and active operation records retained by one process generation.
pub const MANAGEMENT_OPERATION_CAPACITY: usize = 128;
/// Maximum redacted management audit records retained by one process generation.
pub const MANAGEMENT_AUDIT_CAPACITY: usize = 256;

/// Stable management operation state. Ordering is intentionally not used for transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementOperationState {
    Requested,
    Accepted,
    Running,
    Completed,
    Failed,
    Unknown,
}

impl ManagementOperationState {
    fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

/// Stable operation class exposed to the read-only console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementOperationKind {
    Drain,
    Reshard,
    Repair,
    Backup,
    RaftCompaction,
}

impl ManagementOperationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Drain => "drain",
            Self::Reshard => "reshard",
            Self::Repair => "repair",
            Self::Backup => "backup",
            Self::RaftCompaction => "raft_compaction",
        }
    }
}

/// One immutable-sequence operation snapshot. It contains no key, value, path, or credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagementOperationRecord {
    pub operation_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub kind: ManagementOperationKind,
    pub scope: &'static str,
    pub state: ManagementOperationState,
    pub requested_at_unix_ms: u64,
    pub accepted_at_unix_ms: Option<u64>,
    pub started_at_unix_ms: Option<u64>,
    pub terminal_at_unix_ms: Option<u64>,
    pub progress_current: Option<u64>,
    pub progress_total: Option<u64>,
    pub progress_unit: Option<&'static str>,
    pub source: &'static str,
    pub completeness: &'static str,
    pub reason_code: Option<&'static str>,
    pub remediation_code: Option<&'static str>,
}

/// Redacted management/security metadata derived from operation transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagementAuditRecord {
    pub event_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub operation_id: String,
    pub category: &'static str,
    pub action: &'static str,
    pub outcome: ManagementOperationState,
    pub occurred_at_unix_ms: u64,
    pub source: &'static str,
}

/// Bounded point-in-time journal snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagementOperationSnapshot {
    pub generation: u64,
    pub latest_sequence: u64,
    pub evicted_records: u64,
    pub items: Vec<ManagementOperationRecord>,
}

/// Bounded point-in-time audit snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagementAuditSnapshot {
    pub generation: u64,
    pub latest_sequence: u64,
    pub evicted_records: u64,
    pub items: Vec<ManagementAuditRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagementOperationError {
    NotFound,
    IllegalTransition,
    TerminalRecord,
}

#[derive(Debug)]
struct JournalInner {
    generation: u64,
    sequence: u64,
    audit_sequence: u64,
    evicted_operations: u64,
    evicted_audit: u64,
    records: VecDeque<ManagementOperationRecord>,
    audit: VecDeque<ManagementAuditRecord>,
}

/// Cloneable process-local operation owner shared by runtime handles.
#[derive(Debug, Clone)]
pub struct ManagementOperationJournal {
    inner: Arc<Mutex<JournalInner>>,
}

impl Default for ManagementOperationJournal {
    fn default() -> Self {
        Self::new(now_unix_ms())
    }
}

impl ManagementOperationJournal {
    /// Create a journal with an explicit generation, primarily for deterministic tests.
    pub fn new(generation: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(JournalInner {
                generation,
                sequence: 0,
                audit_sequence: 0,
                evicted_operations: 0,
                evicted_audit: 0,
                records: VecDeque::with_capacity(MANAGEMENT_OPERATION_CAPACITY),
                audit: VecDeque::with_capacity(MANAGEMENT_AUDIT_CAPACITY),
            })),
        }
    }

    /// Retain a requested operation and return its opaque identifier.
    pub fn request(&self, kind: ManagementOperationKind, scope: &'static str) -> String {
        let mut inner = self
            .inner
            .lock()
            .expect("management operation journal mutex");
        inner.sequence = inner.sequence.saturating_add(1);
        let sequence = inner.sequence;
        let operation_id = opaque_id(inner.generation, sequence, kind.as_str());
        let record = ManagementOperationRecord {
            operation_id: operation_id.clone(),
            generation: inner.generation,
            sequence,
            kind,
            scope,
            state: ManagementOperationState::Requested,
            requested_at_unix_ms: now_unix_ms(),
            accepted_at_unix_ms: None,
            started_at_unix_ms: None,
            terminal_at_unix_ms: None,
            progress_current: None,
            progress_total: None,
            progress_unit: None,
            source: "runtime_journal",
            completeness: "partial",
            reason_code: None,
            remediation_code: None,
        };
        if inner.records.len() == MANAGEMENT_OPERATION_CAPACITY {
            inner.records.pop_front();
            inner.evicted_operations = inner.evicted_operations.saturating_add(1);
        }
        inner.records.push_back(record);
        operation_id
    }

    /// Apply a legal state transition and append a redacted audit record.
    pub fn transition(
        &self,
        operation_id: &str,
        next: ManagementOperationState,
        reason_code: Option<&'static str>,
        remediation_code: Option<&'static str>,
    ) -> Result<(), ManagementOperationError> {
        let mut inner = self
            .inner
            .lock()
            .expect("management operation journal mutex");
        let index = inner
            .records
            .iter()
            .position(|record| record.operation_id == operation_id)
            .ok_or(ManagementOperationError::NotFound)?;
        let current = inner.records[index].state;
        if current.terminal() {
            return Err(ManagementOperationError::TerminalRecord);
        }
        if !legal_transition(current, next) {
            return Err(ManagementOperationError::IllegalTransition);
        }
        let now = now_unix_ms();
        let record = &mut inner.records[index];
        record.state = next;
        match next {
            ManagementOperationState::Accepted => record.accepted_at_unix_ms = Some(now),
            ManagementOperationState::Running => {
                record.accepted_at_unix_ms.get_or_insert(now);
                record.started_at_unix_ms = Some(now);
            }
            ManagementOperationState::Completed | ManagementOperationState::Failed => {
                record.terminal_at_unix_ms = Some(now);
                record.completeness = "complete";
            }
            ManagementOperationState::Requested | ManagementOperationState::Unknown => {}
        }
        record.reason_code = reason_code;
        record.remediation_code = remediation_code;
        let (generation, operation_id, action) = (
            record.generation,
            record.operation_id.clone(),
            record.kind.as_str(),
        );
        inner.audit_sequence = inner.audit_sequence.saturating_add(1);
        let audit_sequence = inner.audit_sequence;
        let audit = ManagementAuditRecord {
            event_id: opaque_id(generation, audit_sequence, "audit"),
            generation,
            sequence: audit_sequence,
            operation_id,
            category: "management_operation",
            action,
            outcome: next,
            occurred_at_unix_ms: now,
            source: "runtime_journal",
        };
        if inner.audit.len() == MANAGEMENT_AUDIT_CAPACITY {
            inner.audit.pop_front();
            inner.evicted_audit = inner.evicted_audit.saturating_add(1);
        }
        inner.audit.push_back(audit);
        Ok(())
    }

    /// Return newest-first records from the current process generation.
    pub fn snapshot(&self) -> ManagementOperationSnapshot {
        let inner = self
            .inner
            .lock()
            .expect("management operation journal mutex");
        ManagementOperationSnapshot {
            generation: inner.generation,
            latest_sequence: inner.sequence,
            evicted_records: inner.evicted_operations,
            items: inner.records.iter().rev().cloned().collect(),
        }
    }

    /// Return newest-first redacted audit metadata.
    pub fn audit_snapshot(&self) -> ManagementAuditSnapshot {
        let inner = self
            .inner
            .lock()
            .expect("management operation journal mutex");
        ManagementAuditSnapshot {
            generation: inner.generation,
            latest_sequence: inner.audit_sequence,
            evicted_records: inner.evicted_audit,
            items: inner.audit.iter().rev().cloned().collect(),
        }
    }
}

fn legal_transition(current: ManagementOperationState, next: ManagementOperationState) -> bool {
    matches!(
        (current, next),
        (
            ManagementOperationState::Requested,
            ManagementOperationState::Accepted
        ) | (
            ManagementOperationState::Requested,
            ManagementOperationState::Failed
        ) | (
            ManagementOperationState::Accepted,
            ManagementOperationState::Running
        ) | (
            ManagementOperationState::Accepted,
            ManagementOperationState::Failed
        ) | (
            ManagementOperationState::Running,
            ManagementOperationState::Completed
        ) | (
            ManagementOperationState::Running,
            ManagementOperationState::Failed
        )
    )
}

fn opaque_id(generation: u64, sequence: u64, domain: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"hydracache-management-operation-v1\0");
    hasher.update(generation.to_le_bytes());
    hasher.update(sequence.to_le_bytes());
    hasher.update(domain.as_bytes());
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(24);
    for byte in digest.iter().take(12) {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    format!("op-{encoded}")
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_is_not_completed_and_transitions_are_monotone() {
        let journal = ManagementOperationJournal::new(7);
        let id = journal.request(ManagementOperationKind::Backup, "cluster");
        journal
            .transition(&id, ManagementOperationState::Accepted, None, None)
            .unwrap();
        assert_eq!(
            journal.snapshot().items[0].state,
            ManagementOperationState::Accepted
        );
        assert_eq!(
            journal.transition(&id, ManagementOperationState::Completed, None, None),
            Err(ManagementOperationError::IllegalTransition)
        );
        journal
            .transition(&id, ManagementOperationState::Running, None, None)
            .unwrap();
        journal
            .transition(&id, ManagementOperationState::Completed, None, None)
            .unwrap();
        assert_eq!(
            journal.transition(&id, ManagementOperationState::Failed, None, None),
            Err(ManagementOperationError::TerminalRecord)
        );
    }

    #[test]
    fn restart_generation_and_bounded_eviction_are_explicit() {
        let first = ManagementOperationJournal::new(10);
        let second = ManagementOperationJournal::new(11);
        assert_ne!(first.snapshot().generation, second.snapshot().generation);
        for _ in 0..=MANAGEMENT_OPERATION_CAPACITY {
            first.request(ManagementOperationKind::Repair, "node");
        }
        let snapshot = first.snapshot();
        assert_eq!(snapshot.items.len(), MANAGEMENT_OPERATION_CAPACITY);
        assert_eq!(snapshot.evicted_records, 1);
        assert!(snapshot
            .items
            .windows(2)
            .all(|pair| pair[0].sequence > pair[1].sequence));
    }

    #[test]
    fn audit_is_redacted_and_bounded() {
        let journal = ManagementOperationJournal::new(7);
        for _ in 0..=MANAGEMENT_AUDIT_CAPACITY {
            let id = journal.request(ManagementOperationKind::Drain, "node");
            journal
                .transition(
                    &id,
                    ManagementOperationState::Failed,
                    Some("not_ready"),
                    Some("retry_when_ready"),
                )
                .unwrap();
        }
        let audit = journal.audit_snapshot();
        assert_eq!(audit.items.len(), MANAGEMENT_AUDIT_CAPACITY);
        assert_eq!(audit.evicted_records, 1);
        let encoded = serde_json::to_string(&audit).unwrap();
        assert!(!encoded.contains("path"));
        assert!(!encoded.contains("credential"));
    }
}
