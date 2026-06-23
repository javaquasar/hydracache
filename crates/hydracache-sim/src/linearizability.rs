use hydracache::LogicalTime;

use crate::{History, HistoryEvent, WorkloadOp, WorkloadResult};

/// One linearizability violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearizabilityViolation {
    /// Key whose register semantics were violated.
    pub key: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Linearizability check report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinearizabilityReport {
    /// Completed reads checked.
    pub checked_reads: usize,
    /// Violations found.
    pub violations: Vec<LinearizabilityViolation>,
}

impl LinearizabilityReport {
    /// Return whether the report has no violations.
    pub fn is_ok(&self) -> bool {
        self.violations.is_empty()
    }

    fn violation(&mut self, key: impl Into<String>, message: impl Into<String>) {
        self.violations.push(LinearizabilityViolation {
            key: key.into(),
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletedWrite {
    key: String,
    returned_at: LogicalTime,
    value: Option<Vec<u8>>,
}

/// Register-style linearizability checker for completed history events.
#[derive(Debug, Clone, Default)]
pub struct LinearizabilityChecker;

impl LinearizabilityChecker {
    /// Check non-overlapping register semantics for all keys in history.
    ///
    /// If a write/delete completed before a read was invoked, that read must
    /// observe the latest such state. Reads overlapping writes are allowed to
    /// observe either side and are intentionally not rejected by this fast gate.
    pub fn check(&self, history: &History) -> LinearizabilityReport {
        let completed = history.completed().collect::<Vec<_>>();
        let writes = completed
            .iter()
            .filter_map(|event| completed_write(event))
            .collect::<Vec<_>>();

        let mut report = LinearizabilityReport::default();
        for event in completed {
            let Some((key, observed)) = completed_read(event) else {
                continue;
            };
            report.checked_reads = report.checked_reads.saturating_add(1);
            let expected = writes
                .iter()
                .filter(|write| write.key == key && write.returned_at <= event.invoked_at)
                .max_by_key(|write| write.returned_at)
                .map(|write| write.value.clone())
                .unwrap_or(None);
            if observed != expected {
                report.violation(
                    key,
                    format!(
                        "read at {} observed {:?}, expected {:?}",
                        event.invoked_at.as_millis(),
                        observed,
                        expected
                    ),
                );
            }
        }
        report
    }
}

fn completed_write(event: &HistoryEvent) -> Option<CompletedWrite> {
    let returned_at = event.returned_at?;
    match (&event.op, &event.result) {
        (
            WorkloadOp::Put { key, value } | WorkloadOp::CompareAndSet { key, value, .. },
            Some(WorkloadResult::Accepted { .. }),
        ) => Some(CompletedWrite {
            key: key.clone(),
            returned_at,
            value: Some(value.clone()),
        }),
        (WorkloadOp::Invalidate { key }, Some(WorkloadResult::Accepted { .. })) => {
            Some(CompletedWrite {
                key: key.clone(),
                returned_at,
                value: None,
            })
        }
        _ => None,
    }
}

fn completed_read(event: &HistoryEvent) -> Option<(String, Option<Vec<u8>>)> {
    match (&event.op, &event.result) {
        (
            WorkloadOp::Get { key } | WorkloadOp::SessionRead { key },
            Some(WorkloadResult::Value(value)),
        ) => Some((key.clone(), value.clone())),
        _ => None,
    }
}
