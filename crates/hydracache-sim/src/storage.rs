use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use hydracache::{ClusterStorage, LogicalDuration, StorageOp, StorageOpKind, StorageResult};

use crate::SimRng;

const DEFAULT_ZONE: &str = "default";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Stable simulated storage zone id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StorageZoneId(String);

impl StorageZoneId {
    /// Create a zone id from a stable string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for StorageZoneId {
    fn default() -> Self {
        Self::new(DEFAULT_ZONE)
    }
}

impl From<&str> for StorageZoneId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for StorageZoneId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for StorageZoneId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Storage faults the simulator can inject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageFault {
    /// The next read reports a latent media error.
    LatentReadError,
    /// The next checked read flips stored bytes without updating the checksum.
    Corruption,
    /// The next write persists only a prefix while keeping the intended checksum.
    TornWrite,
    /// The next operation reports deterministic latency to the scheduler.
    Slow(LogicalDuration),
    /// The next write remains volatile and is lost on crash until a later fsync.
    LostOnCrash,
}

/// Stored bytes plus checksum metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredValue {
    bytes: Vec<u8>,
    checksum: u64,
}

impl StoredValue {
    /// Build a stored value and checksum its bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        let checksum = checksum(&bytes);
        Self { bytes, checksum }
    }

    /// Build a stored value with an explicit checksum, used for torn writes.
    pub fn with_checksum(bytes: Vec<u8>, checksum: u64) -> Self {
        Self { bytes, checksum }
    }

    /// Return stored bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return the stored checksum.
    pub fn checksum(&self) -> u64 {
        self.checksum
    }

    /// Return whether the stored checksum matches current bytes.
    pub fn checksum_matches(&self) -> bool {
        checksum(&self.bytes) == self.checksum
    }

    fn corrupt(&mut self) {
        if let Some(first) = self.bytes.first_mut() {
            *first ^= 0xff;
        } else {
            self.bytes.push(0xff);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingState {
    Value(StoredValue),
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaultOutcome {
    Continue,
    Applied,
}

#[derive(Debug, Clone, Default)]
struct StoredEntry {
    stable: Option<StoredValue>,
    volatile: Option<PendingState>,
}

impl StoredEntry {
    fn read(&self) -> Option<&StoredValue> {
        match &self.volatile {
            Some(PendingState::Value(value)) => Some(value),
            Some(PendingState::Deleted) => None,
            None => self.stable.as_ref(),
        }
    }

    fn read_mut(&mut self) -> Option<&mut StoredValue> {
        match &mut self.volatile {
            Some(PendingState::Value(value)) => Some(value),
            Some(PendingState::Deleted) => None,
            None => self.stable.as_mut(),
        }
    }

    fn write(&mut self, value: StoredValue) {
        self.volatile = Some(PendingState::Value(value));
    }

    fn delete(&mut self) {
        self.volatile = Some(PendingState::Deleted);
    }

    fn fsync(&mut self) {
        if let Some(pending) = self.volatile.take() {
            self.stable = match pending {
                PendingState::Value(value) => Some(value),
                PendingState::Deleted => None,
            };
        }
    }

    fn crash(&mut self) {
        self.volatile = None;
    }

    fn is_empty(&self) -> bool {
        self.stable.is_none() && self.volatile.is_none()
    }
}

#[derive(Debug, Clone, Default)]
struct SimStorageZone {
    entries: BTreeMap<String, StoredEntry>,
    faults: VecDeque<StorageFault>,
}

impl SimStorageZone {
    fn pop_fault(&mut self) -> Option<StorageFault> {
        self.faults.pop_front()
    }

    fn read(&self, key: &str) -> Result<Option<Vec<u8>>, SimStorageError> {
        let value = self.entries.get(key).and_then(StoredEntry::read);
        match value {
            Some(value) if value.checksum_matches() => Ok(Some(value.bytes.clone())),
            Some(value) => Err(SimStorageError::ChecksumMismatch {
                expected: value.checksum(),
                actual: checksum(value.bytes()),
            }),
            None => Ok(None),
        }
    }

    fn corrupt_visible_value(&mut self, key: &str) {
        if let Some(value) = self.entries.get_mut(key).and_then(StoredEntry::read_mut) {
            value.corrupt();
        }
    }

    fn fsync(&mut self) {
        for entry in self.entries.values_mut() {
            entry.fsync();
        }
        self.entries.retain(|_, entry| !entry.is_empty());
    }

    fn crash(&mut self) {
        for entry in self.entries.values_mut() {
            entry.crash();
        }
        self.entries.retain(|_, entry| !entry.is_empty());
    }
}

/// Byte and marker footprint tracked by simulated storage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StorageFootprint {
    /// Number of storage zones present in the simulation.
    pub zones: u64,
    /// Number of key entries tracked across all zones.
    pub entries: u64,
    /// Bytes currently visible to reads.
    pub live_bytes: u64,
    /// Bytes durably stored after the latest fsync.
    pub stable_bytes: u64,
    /// Bytes pending fsync or crash.
    pub volatile_bytes: u64,
    /// Pending delete markers tracked by the simulated storage layer.
    pub pending_delete_markers: u64,
}

impl StorageFootprint {
    /// Return all byte payloads currently retained by the simulator.
    pub fn tracked_bytes(&self) -> u64 {
        self.stable_bytes.saturating_add(self.volatile_bytes)
    }
}

/// Result of a storage operation plus deterministic simulated delay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimStorageApply {
    /// Storage result returned to the node.
    pub result: StorageResult,
    /// Delay the scheduler should account for.
    pub delay: LogicalDuration,
}

/// Simulated storage read errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimStorageError {
    /// A latent media error was injected for this zone.
    LatentReadError { zone: StorageZoneId, key: String },
    /// Stored bytes no longer match the checksum.
    ChecksumMismatch { expected: u64, actual: u64 },
}

impl fmt::Display for SimStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LatentReadError { zone, key } => {
                write!(
                    formatter,
                    "latent read error in zone '{zone}' for key '{key}'"
                )
            }
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "checksum mismatch: expected {expected:#x}, actual {actual:#x}"
            ),
        }
    }
}

impl std::error::Error for SimStorageError {}

/// Fault-injecting deterministic in-memory storage.
#[derive(Debug, Clone, Default)]
pub struct SimStorage {
    zones: BTreeMap<StorageZoneId, SimStorageZone>,
}

impl SimStorage {
    /// Create empty storage with the default zone.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a fault for a zone. Faults are consumed in FIFO order.
    pub fn inject_fault(&mut self, zone: impl Into<StorageZoneId>, fault: StorageFault) {
        self.zone_mut(zone.into()).faults.push_back(fault);
    }

    /// Deterministically inject one bounded, recoverable fault class.
    pub fn inject_recoverable_fault_from_rng(
        &mut self,
        zone: impl Into<StorageZoneId>,
        rng: &mut SimRng,
    ) -> StorageFault {
        let fault = match rng.next_index(5) {
            0 => StorageFault::LatentReadError,
            1 => StorageFault::Corruption,
            2 => StorageFault::TornWrite,
            3 => StorageFault::Slow(LogicalDuration::from_millis(10)),
            _ => StorageFault::LostOnCrash,
        };
        self.inject_fault(zone, fault.clone());
        fault
    }

    /// Return the storage resources currently modeled by the simulator.
    pub fn footprint(&self) -> StorageFootprint {
        let mut footprint = StorageFootprint {
            zones: self.zones.len() as u64,
            ..StorageFootprint::default()
        };

        for zone in self.zones.values() {
            footprint.entries = footprint.entries.saturating_add(zone.entries.len() as u64);
            for entry in zone.entries.values() {
                if let Some(value) = entry.read() {
                    footprint.live_bytes = footprint
                        .live_bytes
                        .saturating_add(value.bytes().len() as u64);
                }
                if let Some(value) = &entry.stable {
                    footprint.stable_bytes = footprint
                        .stable_bytes
                        .saturating_add(value.bytes().len() as u64);
                }
                match &entry.volatile {
                    Some(PendingState::Value(value)) => {
                        footprint.volatile_bytes = footprint
                            .volatile_bytes
                            .saturating_add(value.bytes().len() as u64);
                    }
                    Some(PendingState::Deleted) => {
                        footprint.pending_delete_markers =
                            footprint.pending_delete_markers.saturating_add(1);
                    }
                    None => {}
                }
            }
        }

        footprint
    }

    /// Apply an operation in the default zone.
    pub fn apply_checked(&mut self, op: StorageOp) -> Result<SimStorageApply, SimStorageError> {
        self.apply_checked_in_zone(StorageZoneId::default(), op)
    }

    /// Apply an operation in a specific zone.
    pub fn apply_checked_in_zone(
        &mut self,
        zone: StorageZoneId,
        op: StorageOp,
    ) -> Result<SimStorageApply, SimStorageError> {
        let request_id = op.request_id;
        let mut delay = LogicalDuration::from_millis(0);
        let fault = self.zone_mut(zone.clone()).pop_fault();
        if let Some(StorageFault::Slow(duration)) = fault {
            delay = duration;
        } else if let Some(fault) = fault {
            if self.apply_fault(zone.clone(), &op.kind, fault)? == FaultOutcome::Applied {
                return Ok(SimStorageApply {
                    result: StorageResult {
                        request_id,
                        value: None,
                    },
                    delay,
                });
            }
        }

        let value = match op.kind {
            StorageOpKind::Read { key } => self.zone_mut(zone.clone()).read(&key)?,
            StorageOpKind::Write { key, value } => {
                self.write_in_zone(zone, key, StoredValue::new(value));
                None
            }
            StorageOpKind::Delete { key } => {
                self.delete_in_zone(zone, key);
                None
            }
            StorageOpKind::Flush => {
                self.flush_in_zone(zone);
                None
            }
        };

        Ok(SimStorageApply {
            result: StorageResult { request_id, value },
            delay,
        })
    }

    /// Read and verify checksum metadata in the default zone.
    pub fn read_checked(&mut self, key: &str) -> Result<Option<Vec<u8>>, SimStorageError> {
        self.read_checked_in_zone(&StorageZoneId::default(), key)
    }

    /// Read and verify checksum metadata in one zone.
    pub fn read_checked_in_zone(
        &mut self,
        zone: &StorageZoneId,
        key: &str,
    ) -> Result<Option<Vec<u8>>, SimStorageError> {
        let fault = self.zone_mut(zone.clone()).pop_fault();
        match fault {
            Some(StorageFault::LatentReadError) => {
                return Err(SimStorageError::LatentReadError {
                    zone: zone.clone(),
                    key: key.to_owned(),
                });
            }
            Some(StorageFault::Corruption) => {
                self.zone_mut(zone.clone()).corrupt_visible_value(key)
            }
            Some(StorageFault::Slow(_))
            | Some(StorageFault::TornWrite)
            | Some(StorageFault::LostOnCrash)
            | None => {}
        }
        self.zone_mut(zone.clone()).read(key)
    }

    /// Commit all volatile writes in every zone.
    pub fn fsync(&mut self) {
        for zone in self.zones.values_mut() {
            zone.fsync();
        }
    }

    /// Commit all volatile writes in one zone.
    pub fn fsync_zone(&mut self, zone: impl Into<StorageZoneId>) {
        self.zone_mut(zone.into()).fsync();
    }

    /// Drop all un-fsynced writes in every zone.
    pub fn crash(&mut self) {
        for zone in self.zones.values_mut() {
            zone.crash();
        }
    }

    /// Drop all un-fsynced writes in one zone.
    pub fn crash_zone(&mut self, zone: impl Into<StorageZoneId>) {
        self.zone_mut(zone.into()).crash();
    }

    pub(crate) fn visible_checksums(&self) -> BTreeMap<String, u64> {
        self.zones
            .get(&StorageZoneId::default())
            .map(|zone| {
                zone.entries
                    .iter()
                    .filter_map(|(key, entry)| {
                        entry.read().map(|value| (key.clone(), value.checksum()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn replace_visible_default_zone_from(&mut self, source: &SimStorage) {
        match source.zones.get(&StorageZoneId::default()).cloned() {
            Some(zone) => {
                self.zones.insert(StorageZoneId::default(), zone);
            }
            None => {
                self.zones.remove(&StorageZoneId::default());
            }
        }
    }

    fn apply_fault(
        &mut self,
        zone: StorageZoneId,
        kind: &StorageOpKind,
        fault: StorageFault,
    ) -> Result<FaultOutcome, SimStorageError> {
        match (fault, kind) {
            (StorageFault::LatentReadError, StorageOpKind::Read { key }) => {
                Err(SimStorageError::LatentReadError {
                    zone,
                    key: key.clone(),
                })
            }
            (StorageFault::Corruption, StorageOpKind::Read { key }) => {
                self.zone_mut(zone).corrupt_visible_value(key);
                Ok(FaultOutcome::Continue)
            }
            (StorageFault::TornWrite, StorageOpKind::Write { key, value }) => {
                let split = value.len().saturating_sub(value.len() / 2);
                let torn = value[..split].to_vec();
                let intended_checksum = checksum(value);
                self.write_in_zone(
                    zone,
                    key.clone(),
                    StoredValue::with_checksum(torn, intended_checksum),
                );
                Ok(FaultOutcome::Applied)
            }
            (StorageFault::LostOnCrash, _)
            | (StorageFault::Slow(_), _)
            | (StorageFault::Corruption, _)
            | (StorageFault::TornWrite, _)
            | (StorageFault::LatentReadError, _) => Ok(FaultOutcome::Continue),
        }
    }

    fn write_in_zone(&mut self, zone: StorageZoneId, key: String, value: StoredValue) {
        self.zone_mut(zone)
            .entries
            .entry(key)
            .or_default()
            .write(value);
    }

    fn delete_in_zone(&mut self, zone: StorageZoneId, key: String) {
        self.zone_mut(zone).entries.entry(key).or_default().delete();
    }

    fn flush_in_zone(&mut self, zone: StorageZoneId) {
        for entry in self.zone_mut(zone).entries.values_mut() {
            entry.delete();
        }
    }

    fn zone_mut(&mut self, zone: StorageZoneId) -> &mut SimStorageZone {
        self.zones.entry(zone).or_default()
    }
}

impl ClusterStorage for SimStorage {
    fn apply(&mut self, op: StorageOp) -> StorageResult {
        let request_id = op.request_id;
        self.apply_checked(op)
            .map(|applied| applied.result)
            .unwrap_or_else(|_| StorageResult {
                request_id,
                value: None,
            })
    }
}

/// Deterministic checksum used by simulated storage artifacts.
pub fn checksum(bytes: &[u8]) -> u64 {
    let mut state = FNV_OFFSET;
    for byte in bytes {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(FNV_PRIME);
    }
    state
}
