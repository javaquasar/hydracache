//! Logical deadline, cancellation and retry control for partition-grouped bulk work.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledItemOutcome {
    Pending,
    Completed,
    Cancelled,
    TimedOut,
    RetryExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledReceiptItem {
    pub input_index: usize,
    pub partition: u32,
    pub attempts: usize,
    pub outcome: ControlledItemOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkControlError {
    InvalidBound,
    InvalidInput,
    UnknownInput,
    AlreadyTerminal,
    TimeRegression,
}

impl fmt::Display for BulkControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BulkControlError {}

#[derive(Debug, Clone)]
struct ControlledItem {
    partition: u32,
    attempts: usize,
    outcome: ControlledItemOutcome,
}

#[derive(Debug)]
pub struct BulkControl {
    deadline: u64,
    logical_time: u64,
    max_attempts: usize,
    items: Vec<ControlledItem>,
}

impl BulkControl {
    pub fn new(
        partitions: Vec<u32>,
        deadline: u64,
        max_attempts: usize,
    ) -> Result<Self, BulkControlError> {
        if partitions.is_empty() {
            return Err(BulkControlError::InvalidInput);
        }
        if deadline == 0 || max_attempts == 0 {
            return Err(BulkControlError::InvalidBound);
        }
        Ok(Self {
            deadline,
            logical_time: 0,
            max_attempts,
            items: partitions
                .into_iter()
                .map(|partition| ControlledItem {
                    partition,
                    attempts: 1,
                    outcome: ControlledItemOutcome::Pending,
                })
                .collect(),
        })
    }

    pub fn pending_groups(&self) -> BTreeMap<u32, Vec<usize>> {
        let mut groups = BTreeMap::<u32, Vec<usize>>::new();
        for (index, item) in self.items.iter().enumerate() {
            if item.outcome == ControlledItemOutcome::Pending {
                groups.entry(item.partition).or_default().push(index);
            }
        }
        groups
    }

    pub fn complete(&mut self, input_index: usize) -> Result<(), BulkControlError> {
        let item = self
            .items
            .get_mut(input_index)
            .ok_or(BulkControlError::UnknownInput)?;
        if item.outcome != ControlledItemOutcome::Pending {
            return Err(BulkControlError::AlreadyTerminal);
        }
        item.outcome = ControlledItemOutcome::Completed;
        Ok(())
    }

    pub fn owner_changed(&mut self, partition: u32) -> usize {
        let mut retried = 0;
        for item in self.items.iter_mut().filter(|item| {
            item.partition == partition && item.outcome == ControlledItemOutcome::Pending
        }) {
            if item.attempts >= self.max_attempts {
                item.outcome = ControlledItemOutcome::RetryExhausted;
            } else {
                item.attempts += 1;
                retried += 1;
            }
        }
        retried
    }

    pub fn cancel(&mut self) -> usize {
        self.finish_pending(ControlledItemOutcome::Cancelled)
    }

    pub fn advance_to(&mut self, logical_time: u64) -> Result<usize, BulkControlError> {
        if logical_time < self.logical_time {
            return Err(BulkControlError::TimeRegression);
        }
        self.logical_time = logical_time;
        if logical_time < self.deadline {
            return Ok(0);
        }
        Ok(self.finish_pending(ControlledItemOutcome::TimedOut))
    }

    pub fn receipt(&self) -> Vec<ControlledReceiptItem> {
        self.items
            .iter()
            .enumerate()
            .map(|(input_index, item)| ControlledReceiptItem {
                input_index,
                partition: item.partition,
                attempts: item.attempts,
                outcome: item.outcome,
            })
            .collect()
    }

    fn finish_pending(&mut self, outcome: ControlledItemOutcome) -> usize {
        let mut changed = 0;
        for item in &mut self.items {
            if item.outcome == ControlledItemOutcome::Pending {
                item.outcome = outcome;
                changed += 1;
            }
        }
        changed
    }
}
