use std::convert::TryInto;
use std::path::Path;

use crate::cluster::{ClusterEpoch, PartitionId};
use crate::grid::hardening::{
    ChecksummedReplicatedValueRecord, ReplicatedValueRecord, ReplicatedValueStore, ValueStoreError,
    ValueVersion,
};
use crate::grid::{EffectiveReplicationMap, ReplicatedSlot, TombstoneTracker};

/// On-disk value-store format version registered in `docs/COMPAT.md`.
pub const DURABLE_VALUE_FORMAT_VERSION: u32 = 1;

const FORMAT_KEY: &[u8] = b"hydracache:durable-value-store:format";
const RECORD_PREFIX: &[u8] = b"record:";
const MAGIC: &[u8; 4] = b"HCDV";
const STATE_VALUE: u8 = 1;
const STATE_TOMBSTONE: u8 = 2;
const NONE_EPOCH: u64 = u64::MAX;

/// Sled-backed durable value store used as the cold tier for persisted namespaces.
///
/// This store persists sealed replicated value records and tombstones. It refuses
/// unknown future store/record formats and checksum mismatches before serving data.
#[derive(Debug)]
pub struct DurableValueStore {
    db: sled::Db,
    max_total_bytes: u64,
    rejected_total: u64,
}

/// Report from one bounded durable value-store GC cycle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DurableGcReport {
    /// Records scanned this cycle.
    pub scanned: usize,
    /// Live records skipped.
    pub skipped_live: usize,
    /// Tombstones skipped because repair has not been confirmed.
    pub skipped_repair_pending: usize,
    /// Records removed this cycle.
    pub removed: usize,
    /// Approximate durable budget bytes reclaimed this cycle.
    pub reclaimed_bytes: u64,
    /// Counter: `durable_gc_reclaimed_total`.
    pub durable_gc_reclaimed_total: u64,
    /// Counter: `durable_gc_skipped_repair_pending_total`.
    pub durable_gc_skipped_repair_pending_total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableRawRecord {
    pub key: String,
    pub bytes: Vec<u8>,
}

impl DurableValueStore {
    /// Open or create a durable value store under `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ValueStoreError> {
        Self::open_with_budget(path, u64::MAX)
    }

    /// Open or create a durable value store under `path` with a total byte budget.
    pub fn open_with_budget(
        path: impl AsRef<Path>,
        max_total_bytes: u64,
    ) -> Result<Self, ValueStoreError> {
        let db = sled::open(path).map_err(sled_error)?;
        validate_or_initialize_format(&db)?;
        Ok(Self {
            db,
            max_total_bytes: max_total_bytes.max(1),
            rejected_total: 0,
        })
    }

    /// Flush outstanding writes to disk.
    pub fn flush(&self) -> Result<(), ValueStoreError> {
        self.db.flush().map(|_| ()).map_err(sled_error)
    }

    /// Return total retained value bytes.
    pub fn total_bytes(&self) -> Result<u64, ValueStoreError> {
        let mut total = 0_u64;
        for item in self.db.scan_prefix(RECORD_PREFIX) {
            let (key, value) = item.map_err(sled_error)?;
            let key = stored_key_to_cache_key(key.as_ref())?;
            let record = decode_record(&key, value.as_ref())?;
            total = total.saturating_add(record.approx_bytes());
        }
        Ok(total)
    }

    /// Return how many writes were rejected by the byte budget.
    pub fn rejected_total(&self) -> u64 {
        self.rejected_total
    }

    /// Write an explicit store format marker, used by compatibility tests.
    #[doc(hidden)]
    pub fn write_format_marker_for_test(
        path: impl AsRef<Path>,
        version: u32,
    ) -> Result<(), ValueStoreError> {
        let db = sled::open(path).map_err(sled_error)?;
        db.insert(FORMAT_KEY, version.to_le_bytes().as_slice())
            .map_err(sled_error)?;
        db.flush().map(|_| ()).map_err(sled_error)
    }

    /// Insert raw bytes for one key, used by corruption tests.
    #[doc(hidden)]
    pub fn put_raw_record_for_test(
        &self,
        key: &str,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), ValueStoreError> {
        self.db
            .insert(record_key(key), bytes.as_ref())
            .map_err(sled_error)?;
        self.flush()
    }

    /// Return raw bytes for one key, used by checksum fault-injection tests.
    #[doc(hidden)]
    pub fn raw_record_for_test(&self, key: &str) -> Result<Option<Vec<u8>>, ValueStoreError> {
        self.db
            .get(record_key(key))
            .map(|bytes| bytes.map(|bytes| bytes.to_vec()))
            .map_err(sled_error)
    }

    pub(crate) fn raw_record_batch_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<DurableRawRecord>, ValueStoreError> {
        let limit = limit.max(1);
        let mut records = Vec::with_capacity(limit);
        match after {
            Some(after) => {
                for item in self.db.range(record_key(after)..) {
                    let (stored_key, bytes) = item.map_err(sled_error)?;
                    let stored_key = stored_key.as_ref();
                    if !stored_key.starts_with(RECORD_PREFIX) {
                        break;
                    }
                    let key = stored_key_to_cache_key(stored_key)?;
                    if key.as_str() <= after {
                        continue;
                    }
                    records.push(DurableRawRecord {
                        key,
                        bytes: bytes.to_vec(),
                    });
                    if records.len() == limit {
                        break;
                    }
                }
            }
            None => {
                for item in self.db.scan_prefix(RECORD_PREFIX).take(limit) {
                    let (stored_key, bytes) = item.map_err(sled_error)?;
                    records.push(DurableRawRecord {
                        key: stored_key_to_cache_key(stored_key.as_ref())?,
                        bytes: bytes.to_vec(),
                    });
                }
            }
        }
        Ok(records)
    }

    pub(crate) fn decode_raw_record(
        key: &str,
        bytes: &[u8],
    ) -> Result<ReplicatedValueRecord, ValueStoreError> {
        decode_record(key, bytes)
    }

    /// Reclaim repair-confirmed tombstones in a bounded maintenance cycle.
    pub fn collect_tombstone_garbage(
        &mut self,
        tracker: &mut TombstoneTracker,
        now_epoch: ClusterEpoch,
        max_records: usize,
    ) -> Result<DurableGcReport, ValueStoreError> {
        let max_records = max_records.max(1);
        let mut report = DurableGcReport::default();
        let mut records = self.scan_all()?;
        records.sort_by(|left, right| left.0.cmp(&right.0));

        for (key, record) in records.into_iter().take(max_records) {
            report.scanned = report.scanned.saturating_add(1);
            if !record.is_tombstone() {
                report.skipped_live = report.skipped_live.saturating_add(1);
                continue;
            }
            let Some(eligible_after) = tracker.gc_eligible_after(&key) else {
                report.skipped_repair_pending = report.skipped_repair_pending.saturating_add(1);
                continue;
            };
            if eligible_after > now_epoch {
                report.skipped_repair_pending = report.skipped_repair_pending.saturating_add(1);
                continue;
            }
            let reclaimed = record.approx_bytes();
            self.remove(&key)?;
            tracker.forget(&key);
            report.removed = report.removed.saturating_add(1);
            report.reclaimed_bytes = report.reclaimed_bytes.saturating_add(reclaimed);
        }
        report.durable_gc_reclaimed_total = report.reclaimed_bytes;
        report.durable_gc_skipped_repair_pending_total = report.skipped_repair_pending as u64;
        Ok(report)
    }

    fn would_fit(
        &self,
        key: &str,
        record: &ReplicatedValueRecord,
    ) -> Result<bool, ValueStoreError> {
        let existing = self
            .get(key)?
            .map(|existing| existing.approx_bytes())
            .unwrap_or_default();
        Ok(self
            .total_bytes()?
            .saturating_sub(existing)
            .saturating_add(record.approx_bytes())
            <= self.max_total_bytes)
    }

    fn scan_records(&self) -> Result<Vec<(String, ReplicatedValueRecord)>, ValueStoreError> {
        let mut records = Vec::new();
        for item in self.db.scan_prefix(RECORD_PREFIX) {
            let (key, value) = item.map_err(sled_error)?;
            let key = stored_key_to_cache_key(key.as_ref())?;
            let record = decode_record(&key, value.as_ref())?;
            records.push((key, record));
        }
        Ok(records)
    }
}

impl ReplicatedValueStore for DurableValueStore {
    fn upsert(
        &mut self,
        key: impl Into<String>,
        record: ReplicatedValueRecord,
    ) -> Result<(), ValueStoreError> {
        let key = key.into();
        if !self.would_fit(&key, &record)? {
            self.rejected_total = self.rejected_total.saturating_add(1);
            return Err(ValueStoreError::new(
                "durable value store total byte budget exceeded",
            ));
        }
        let merged = self
            .get(&key)?
            .map(|current| current.merge(record.clone()))
            .unwrap_or(record);
        self.db
            .insert(record_key(&key), encode_record(&key, &merged)?)
            .map_err(sled_error)?;
        self.flush()
    }

    fn get(&self, key: &str) -> Result<Option<ReplicatedValueRecord>, ValueStoreError> {
        self.db
            .get(record_key(key))
            .map_err(sled_error)?
            .map(|bytes| decode_record(key, bytes.as_ref()))
            .transpose()
    }

    fn tombstone(
        &mut self,
        key: impl Into<String>,
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
    ) -> Result<(), ValueStoreError> {
        self.upsert(
            key,
            ReplicatedValueRecord::tombstone(partition, version, epoch, None),
        )
    }

    fn scan_owned(
        &self,
        map: &EffectiveReplicationMap,
    ) -> Result<Vec<(String, ReplicatedValueRecord)>, ValueStoreError> {
        if map.reading.is_empty() {
            return Ok(Vec::new());
        }
        self.scan_all()
    }

    fn scan_all(&self) -> Result<Vec<(String, ReplicatedValueRecord)>, ValueStoreError> {
        self.scan_records()
    }

    fn remove(&mut self, key: &str) -> Result<(), ValueStoreError> {
        self.db.remove(record_key(key)).map_err(sled_error)?;
        self.flush()
    }

    fn compact(&mut self) -> Result<u64, ValueStoreError> {
        self.flush()?;
        Ok(0)
    }

    fn total_bytes(&self) -> Result<u64, ValueStoreError> {
        DurableValueStore::total_bytes(self)
    }

    fn rejected_total(&self) -> u64 {
        DurableValueStore::rejected_total(self)
    }
}

fn validate_or_initialize_format(db: &sled::Db) -> Result<(), ValueStoreError> {
    match db.get(FORMAT_KEY).map_err(sled_error)? {
        Some(bytes) => {
            let found = read_u32_exact(bytes.as_ref(), "durable value-store format marker")?;
            if found != DURABLE_VALUE_FORMAT_VERSION {
                return Err(ValueStoreError::new(format!(
                    "unsupported durable value-store format {found}; expected {DURABLE_VALUE_FORMAT_VERSION}"
                )));
            }
            Ok(())
        }
        None => {
            db.insert(
                FORMAT_KEY,
                DURABLE_VALUE_FORMAT_VERSION.to_le_bytes().as_slice(),
            )
            .map_err(sled_error)?;
            db.flush().map(|_| ()).map_err(sled_error)
        }
    }
}

fn encode_record(key: &str, record: &ReplicatedValueRecord) -> Result<Vec<u8>, ValueStoreError> {
    let envelope = ChecksummedReplicatedValueRecord::seal(record.clone());
    let mut payload = Vec::new();
    payload.extend_from_slice(MAGIC);
    payload.extend_from_slice(&DURABLE_VALUE_FORMAT_VERSION.to_le_bytes());
    write_bytes(&mut payload, key.as_bytes())?;
    payload.extend_from_slice(&record.partition.value().to_le_bytes());
    payload.extend_from_slice(&record.version.to_le_bytes());
    payload.extend_from_slice(&record.epoch.value().to_le_bytes());
    match &record.state {
        ReplicatedSlot::Value { value, version } => {
            payload.push(STATE_VALUE);
            payload.extend_from_slice(&version.to_le_bytes());
            write_bytes(&mut payload, value)?;
        }
        ReplicatedSlot::Tombstone {
            version,
            gc_eligible_after,
        } => {
            payload.push(STATE_TOMBSTONE);
            payload.extend_from_slice(&version.to_le_bytes());
            payload.extend_from_slice(
                &gc_eligible_after
                    .map(ClusterEpoch::value)
                    .unwrap_or(NONE_EPOCH)
                    .to_le_bytes(),
            );
        }
    }
    payload.extend_from_slice(&envelope.checksum_format.to_le_bytes());
    payload.extend_from_slice(&envelope.checksum.to_le_bytes());

    let payload_len = u32::try_from(payload.len())
        .map_err(|_| ValueStoreError::new("durable value record payload too large"))?;
    let mut encoded = Vec::with_capacity(4 + payload.len());
    encoded.extend_from_slice(&payload_len.to_le_bytes());
    encoded.extend_from_slice(&payload);
    Ok(encoded)
}

fn decode_record(
    expected_key: &str,
    bytes: &[u8],
) -> Result<ReplicatedValueRecord, ValueStoreError> {
    let mut cursor = Cursor::new(bytes);
    let payload_len = cursor.u32()? as usize;
    if cursor.remaining_len() != payload_len {
        return Err(ValueStoreError::new(format!(
            "durable value record length mismatch for key '{expected_key}'"
        )));
    }
    let magic = cursor.fixed::<4>()?;
    if &magic != MAGIC {
        return Err(ValueStoreError::new(format!(
            "invalid durable value record magic for key '{expected_key}'"
        )));
    }
    let format = cursor.u32()?;
    if format != DURABLE_VALUE_FORMAT_VERSION {
        return Err(ValueStoreError::new(format!(
            "unsupported durable value record format {format} for key '{expected_key}'"
        )));
    }
    let encoded_key = cursor.string()?;
    if encoded_key != expected_key {
        return Err(ValueStoreError::new(format!(
            "durable value record key mismatch: expected '{expected_key}', found '{encoded_key}'"
        )));
    }
    let partition = PartitionId::new(cursor.u32()?);
    let version = cursor.u64()?;
    let epoch = ClusterEpoch::new(cursor.u64()?);
    let state = match cursor.u8()? {
        STATE_VALUE => {
            let state_version = cursor.u64()?;
            let value = cursor.bytes()?.to_vec();
            ReplicatedSlot::Value {
                value,
                version: state_version,
            }
        }
        STATE_TOMBSTONE => {
            let state_version = cursor.u64()?;
            let gc_epoch = cursor.u64()?;
            ReplicatedSlot::Tombstone {
                version: state_version,
                gc_eligible_after: (gc_epoch != NONE_EPOCH).then(|| ClusterEpoch::new(gc_epoch)),
            }
        }
        found => {
            return Err(ValueStoreError::new(format!(
                "invalid durable value state kind {found} for key '{expected_key}'"
            )))
        }
    };
    let checksum_format = cursor.u32()?;
    let checksum = cursor.u64()?;
    if !cursor.is_empty() {
        return Err(ValueStoreError::new(format!(
            "trailing durable value record bytes for key '{expected_key}'"
        )));
    }
    let record = ReplicatedValueRecord {
        partition,
        version,
        epoch,
        state,
    };
    let envelope = ChecksummedReplicatedValueRecord::from_parts(checksum_format, record, checksum);
    envelope.verify().map_err(|error| {
        ValueStoreError::new(format!("durable value checksum error: {error:?}"))
    })?;
    Ok(envelope.record)
}

fn write_bytes(target: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ValueStoreError> {
    let len = u32::try_from(bytes.len())
        .map_err(|_| ValueStoreError::new("durable value field too large"))?;
    target.extend_from_slice(&len.to_le_bytes());
    target.extend_from_slice(bytes);
    Ok(())
}

fn read_u32_exact(bytes: &[u8], context: &str) -> Result<u32, ValueStoreError> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| ValueStoreError::new(format!("invalid {context}")))?;
    Ok(u32::from_le_bytes(array))
}

fn record_key(key: &str) -> Vec<u8> {
    let mut stored = Vec::with_capacity(RECORD_PREFIX.len() + key.len());
    stored.extend_from_slice(RECORD_PREFIX);
    stored.extend_from_slice(key.as_bytes());
    stored
}

fn stored_key_to_cache_key(key: &[u8]) -> Result<String, ValueStoreError> {
    let raw = key
        .strip_prefix(RECORD_PREFIX)
        .ok_or_else(|| ValueStoreError::new("invalid durable value record key prefix"))?;
    String::from_utf8(raw.to_vec())
        .map_err(|_| ValueStoreError::new("durable value record key is not utf-8"))
}

fn sled_error(error: sled::Error) -> ValueStoreError {
    ValueStoreError::new(format!("sled durable value store error: {error}"))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn is_empty(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], ValueStoreError> {
        let bytes = self.take(N)?;
        bytes
            .try_into()
            .map_err(|_| ValueStoreError::new("invalid durable value fixed-width field"))
    }

    fn u8(&mut self) -> Result<u8, ValueStoreError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, ValueStoreError> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }

    fn u64(&mut self) -> Result<u64, ValueStoreError> {
        Ok(u64::from_le_bytes(self.fixed()?))
    }

    fn bytes(&mut self) -> Result<&'a [u8], ValueStoreError> {
        let len = self.u32()? as usize;
        self.take(len)
    }

    fn string(&mut self) -> Result<String, ValueStoreError> {
        String::from_utf8(self.bytes()?.to_vec())
            .map_err(|_| ValueStoreError::new("durable value string field is not utf-8"))
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], ValueStoreError> {
        let end = self
            .position
            .checked_add(len)
            .ok_or_else(|| ValueStoreError::new("durable value record cursor overflow"))?;
        if end > self.bytes.len() {
            return Err(ValueStoreError::new("truncated durable value record"));
        }
        let bytes = &self.bytes[self.position..end];
        self.position = end;
        Ok(bytes)
    }
}
