//! Abstract, test-only durability state machine. It assigns no persisted format or wire identity.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcknowledgementClass {
    Memory,
    Durable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryFault {
    CleanRestart,
    OwnerAndBackupLoss,
    WholeClusterRestart,
    TruncateAfter(u64),
    Corrupt(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    pub recovered_through: u64,
    pub memory_acknowledgement_lost: bool,
    pub durable_acknowledgement_preserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableError {
    InvalidBound,
    Capacity,
    VersionMustAdvance,
    UnknownVersion,
    CommitRegression,
    FlushBeyondCommit,
    FlushRegression,
    AcknowledgeBeforeCommit,
    AcknowledgeBeforeDurable,
    SnapshotBeyondDurable,
    SnapshotRegression,
    DurableDataTruncated,
    DurableDataCorrupted,
}

impl fmt::Display for DurableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DurableError {}

#[derive(Debug, Clone)]
pub struct DurableRecoveryModel {
    max_records: usize,
    records: BTreeMap<u64, u64>,
    committed: u64,
    durable: u64,
    snapshot: u64,
    memory_acknowledged: u64,
    durable_acknowledged: u64,
}

impl DurableRecoveryModel {
    pub fn new(max_records: usize) -> Result<Self, DurableError> {
        if max_records == 0 {
            return Err(DurableError::InvalidBound);
        }
        Ok(Self {
            max_records,
            records: BTreeMap::new(),
            committed: 0,
            durable: 0,
            snapshot: 0,
            memory_acknowledged: 0,
            durable_acknowledged: 0,
        })
    }

    pub fn append(&mut self, version: u64, checksum: u64) -> Result<(), DurableError> {
        if self.records.len() >= self.max_records {
            return Err(DurableError::Capacity);
        }
        if version == 0
            || self
                .records
                .last_key_value()
                .is_some_and(|(last, _)| version <= *last)
        {
            return Err(DurableError::VersionMustAdvance);
        }
        self.records.insert(version, checksum);
        Ok(())
    }

    pub fn commit(&mut self, version: u64) -> Result<(), DurableError> {
        if version < self.committed {
            return Err(DurableError::CommitRegression);
        }
        if !self.records.contains_key(&version) {
            return Err(DurableError::UnknownVersion);
        }
        self.committed = version;
        Ok(())
    }

    pub fn flush(&mut self, version: u64) -> Result<(), DurableError> {
        if version < self.durable {
            return Err(DurableError::FlushRegression);
        }
        if version > self.committed {
            return Err(DurableError::FlushBeyondCommit);
        }
        self.durable = version;
        Ok(())
    }

    pub fn acknowledge(
        &mut self,
        version: u64,
        class: AcknowledgementClass,
    ) -> Result<(), DurableError> {
        match class {
            AcknowledgementClass::Memory if version <= self.committed => {
                self.memory_acknowledged = self.memory_acknowledged.max(version);
            }
            AcknowledgementClass::Memory => return Err(DurableError::AcknowledgeBeforeCommit),
            AcknowledgementClass::Durable if version <= self.durable => {
                self.durable_acknowledged = self.durable_acknowledged.max(version);
            }
            AcknowledgementClass::Durable => return Err(DurableError::AcknowledgeBeforeDurable),
        }
        Ok(())
    }

    pub fn snapshot(&mut self, version: u64) -> Result<(), DurableError> {
        if version < self.snapshot {
            return Err(DurableError::SnapshotRegression);
        }
        if version > self.durable {
            return Err(DurableError::SnapshotBeyondDurable);
        }
        self.snapshot = version;
        Ok(())
    }

    pub fn recover(&mut self, fault: RecoveryFault) -> Result<RecoveryReport, DurableError> {
        match fault {
            RecoveryFault::Corrupt(version) if version <= self.durable => {
                return Err(DurableError::DurableDataCorrupted);
            }
            RecoveryFault::TruncateAfter(version) if version < self.durable => {
                return Err(DurableError::DurableDataTruncated);
            }
            RecoveryFault::TruncateAfter(version) => self.records.retain(|key, _| *key <= version),
            RecoveryFault::Corrupt(_) | RecoveryFault::CleanRestart => {}
            RecoveryFault::OwnerAndBackupLoss | RecoveryFault::WholeClusterRestart => {
                self.records.retain(|version, _| *version <= self.durable);
            }
        }
        let recovered_through = self
            .records
            .last_key_value()
            .map_or(0, |(version, _)| *version);
        self.committed = self.committed.min(recovered_through);
        Ok(RecoveryReport {
            recovered_through,
            memory_acknowledgement_lost: self.memory_acknowledged > recovered_through,
            durable_acknowledgement_preserved: self.durable_acknowledged <= recovered_through,
        })
    }

    pub const fn durable_watermark(&self) -> u64 {
        self.durable
    }

    pub const fn snapshot_watermark(&self) -> u64 {
        self.snapshot
    }
}
