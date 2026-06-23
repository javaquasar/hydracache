use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, ClusterNodeId, PartitionId};
use crate::grid::{EffectiveReplicationMap, ReplicatedSlot, ReplicationConfig};

/// Monotonic version used by replicated value records.
pub type ValueVersion = u64;

/// Bytes after the operator-supplied replication boundary has sealed/redacted
/// them. Durable stores persist these bytes, never the original plaintext.
pub type SealedBytes = Vec<u8>;

/// Durable value-store format version registered in `docs/COMPAT.md`.
pub const REPLICATED_VALUE_RECORD_FORMAT_VERSION: u32 = 1;

/// Checksummed durable envelope version registered in `docs/COMPAT.md`.
pub const REPLICATED_VALUE_RECORD_CHECKSUM_FORMAT_VERSION: u32 = 1;

/// Durable replicated value record keyed externally by cache key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicatedValueRecord {
    /// Partition that owns this value.
    pub partition: PartitionId,
    /// Monotonic value/tombstone version.
    pub version: ValueVersion,
    /// Authority epoch that produced this record.
    pub epoch: ClusterEpoch,
    /// Sealed value bytes or tombstone marker.
    pub state: ReplicatedSlot<SealedBytes>,
}

impl ReplicatedValueRecord {
    /// Create a live value record.
    pub fn value(
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
        sealed: impl Into<SealedBytes>,
    ) -> Self {
        Self {
            partition,
            version,
            epoch,
            state: ReplicatedSlot::Value {
                value: sealed.into(),
                version,
            },
        }
    }

    /// Create a tombstone record.
    pub fn tombstone(
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
        gc_eligible_after: Option<ClusterEpoch>,
    ) -> Self {
        Self {
            partition,
            version,
            epoch,
            state: ReplicatedSlot::Tombstone {
                version,
                gc_eligible_after,
            },
        }
    }

    /// Return whether this record is a tombstone.
    pub fn is_tombstone(&self) -> bool {
        self.state.is_tombstone()
    }

    /// Approximate bytes charged against the durable value-store budget.
    pub fn approx_bytes(&self) -> u64 {
        match &self.state {
            ReplicatedSlot::Value { value, .. } => value.len().max(1) as u64,
            ReplicatedSlot::Tombstone { .. } => 1,
        }
    }

    /// Return a deterministic checksum over the durable record payload.
    pub fn artifact_checksum(&self) -> u64 {
        let mut checksum = ArtifactChecksum::new();
        checksum.u32(REPLICATED_VALUE_RECORD_FORMAT_VERSION);
        checksum.u32(self.partition.value());
        checksum.u64(self.version);
        checksum.u64(self.epoch.value());
        match &self.state {
            ReplicatedSlot::Value { value, version } => {
                checksum.u8(1);
                checksum.u64(*version);
                checksum.bytes(value);
            }
            ReplicatedSlot::Tombstone {
                version,
                gc_eligible_after,
            } => {
                checksum.u8(2);
                checksum.u64(*version);
                checksum.u64(
                    gc_eligible_after
                        .map(ClusterEpoch::value)
                        .unwrap_or(u64::MAX),
                );
            }
        }
        checksum.finish()
    }

    /// Merge two records: higher `(version, epoch)` wins, and tombstones win
    /// ties so deletes cannot be undone by stale values.
    pub fn merge(self, other: Self) -> Self {
        match (self.version, self.epoch).cmp(&(other.version, other.epoch)) {
            std::cmp::Ordering::Greater => self,
            std::cmp::Ordering::Less => other,
            std::cmp::Ordering::Equal if self.is_tombstone() => self,
            std::cmp::Ordering::Equal => other,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ArtifactChecksum(u64);

impl ArtifactChecksum {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn u8(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(Self::PRIME);
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.u8(*byte);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}

/// Durable replicated value record plus checksum metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecksummedReplicatedValueRecord {
    /// Envelope format version.
    pub checksum_format: u32,
    /// Durable replicated value record.
    pub record: ReplicatedValueRecord,
    /// Checksum over the durable record payload.
    pub checksum: u64,
}

impl ChecksummedReplicatedValueRecord {
    /// Create a checksummed envelope.
    pub fn seal(record: ReplicatedValueRecord) -> Self {
        let checksum = record.artifact_checksum();
        Self {
            checksum_format: REPLICATED_VALUE_RECORD_CHECKSUM_FORMAT_VERSION,
            record,
            checksum,
        }
    }

    /// Recreate an envelope from durable parts.
    pub fn from_parts(checksum_format: u32, record: ReplicatedValueRecord, checksum: u64) -> Self {
        Self {
            checksum_format,
            record,
            checksum,
        }
    }

    /// Verify envelope version and checksum.
    pub fn verify(&self) -> Result<(), ScrubErrorKind> {
        if self.checksum_format != REPLICATED_VALUE_RECORD_CHECKSUM_FORMAT_VERSION {
            return Err(ScrubErrorKind::UnsupportedChecksumFormat {
                found: self.checksum_format,
            });
        }
        let actual = self.record.artifact_checksum();
        if actual != self.checksum {
            return Err(ScrubErrorKind::ChecksumMismatch {
                expected: self.checksum,
                actual,
            });
        }
        Ok(())
    }

    /// Return the verified record or a scrub error.
    pub fn verified_record(
        &self,
        key: impl Into<String>,
    ) -> Result<&ReplicatedValueRecord, ScrubError> {
        self.verify().map_err(|kind| ScrubError {
            key: key.into(),
            kind,
        })?;
        Ok(&self.record)
    }
}

/// Scrubber error kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrubErrorKind {
    /// Durable envelope uses an unsupported future format.
    UnsupportedChecksumFormat { found: u32 },
    /// Record checksum does not match payload.
    ChecksumMismatch { expected: u64, actual: u64 },
}

/// Scrubber error with key context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrubError {
    /// Corrupt key.
    pub key: String,
    /// Error kind.
    pub kind: ScrubErrorKind,
}

impl fmt::Display for ScrubError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "scrub error for key '{}': {:?}",
            self.key, self.kind
        )
    }
}

impl std::error::Error for ScrubError {}

/// Scrubber run report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScrubReport {
    /// Records checked.
    pub checked: usize,
    /// Corrupt records found.
    pub corrupt: usize,
    /// Records repaired from peers.
    pub repaired: usize,
    /// Corrupt records that could not be repaired.
    pub unrepairable: usize,
    /// Detailed errors.
    pub errors: Vec<ScrubError>,
}

impl ScrubReport {
    /// Return whether no unrepairable corruption remains.
    pub fn is_ok(&self) -> bool {
        self.unrepairable == 0
    }
}

/// Rate-limited replicated-record scrubber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scrubber {
    max_records_per_run: usize,
}

impl Scrubber {
    /// Create a scrubber with a normalized per-run budget.
    pub fn new(max_records_per_run: usize) -> Self {
        Self {
            max_records_per_run: max_records_per_run.max(1),
        }
    }

    /// Verify primary records and repair corrupt ones from valid peer copies.
    pub fn scrub_replicated_values(
        &self,
        primary: &mut BTreeMap<String, ChecksummedReplicatedValueRecord>,
        peers: &[BTreeMap<String, ChecksummedReplicatedValueRecord>],
    ) -> ScrubReport {
        let keys = primary
            .keys()
            .take(self.max_records_per_run)
            .cloned()
            .collect::<Vec<_>>();
        let mut report = ScrubReport::default();
        for key in keys {
            report.checked = report.checked.saturating_add(1);
            let Some(record) = primary.get(&key) else {
                continue;
            };
            if record.verify().is_ok() {
                continue;
            }
            report.corrupt = report.corrupt.saturating_add(1);
            if let Some(repair) = peers.iter().find_map(|peer| {
                peer.get(&key)
                    .filter(|candidate| candidate.verify().is_ok())
                    .cloned()
            }) {
                primary.insert(key, repair);
                report.repaired = report.repaired.saturating_add(1);
            } else {
                report.unrepairable = report.unrepairable.saturating_add(1);
                let kind = record.verify().expect_err("record is known corrupt");
                report.errors.push(ScrubError { key, kind });
            }
        }
        report
    }

    /// Read one record only if its checksum verifies.
    pub fn verified_get(
        records: &BTreeMap<String, ChecksummedReplicatedValueRecord>,
        key: &str,
    ) -> Result<Option<ReplicatedValueRecord>, ScrubError> {
        records
            .get(key)
            .map(|record| record.verified_record(key.to_owned()).cloned())
            .transpose()
    }
}

impl Default for Scrubber {
    fn default() -> Self {
        Self::new(128)
    }
}

/// Value-store admission error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueStoreError {
    message: String,
}

impl ValueStoreError {
    /// Create a value-store error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ValueStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ValueStoreError {}

/// Durable replicated value-store seam.
pub trait ReplicatedValueStore: Send + Sync {
    /// Upsert a live value or tombstone.
    fn upsert(
        &mut self,
        key: impl Into<String>,
        rec: ReplicatedValueRecord,
    ) -> Result<(), ValueStoreError>;

    /// Return a stored record.
    fn get(&self, key: &str) -> Result<Option<ReplicatedValueRecord>, ValueStoreError>;

    /// Persist a tombstone.
    fn tombstone(
        &mut self,
        key: impl Into<String>,
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
    ) -> Result<(), ValueStoreError>;

    /// Return records whose partition is readable under the supplied effective map.
    fn scan_owned(
        &self,
        map: &EffectiveReplicationMap,
    ) -> Result<Vec<(String, ReplicatedValueRecord)>, ValueStoreError>;
}

/// Deterministic in-memory implementation used by the fast 0.42 tests and as
/// the model for durable engine semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InMemoryReplicatedValueStore {
    records: BTreeMap<String, ReplicatedValueRecord>,
    max_total_bytes: u64,
    rejected_total: u64,
}

impl InMemoryReplicatedValueStore {
    /// Create a store with a normalized total-byte budget.
    pub fn with_budget(max_total_bytes: u64) -> Self {
        Self {
            records: BTreeMap::new(),
            max_total_bytes: max_total_bytes.max(1),
            rejected_total: 0,
        }
    }

    /// Return the total budgeted bytes currently retained.
    pub fn total_bytes(&self) -> u64 {
        self.records
            .values()
            .map(ReplicatedValueRecord::approx_bytes)
            .sum()
    }

    /// Return rejected-upsert count.
    pub fn rejected_total(&self) -> u64 {
        self.rejected_total
    }

    /// Return a clone of all records, modeling a restart/reopen.
    pub fn snapshot(&self) -> BTreeMap<String, ReplicatedValueRecord> {
        self.records.clone()
    }

    /// Reopen a store from a previous snapshot.
    pub fn reopen_from_snapshot(
        max_total_bytes: u64,
        records: BTreeMap<String, ReplicatedValueRecord>,
    ) -> Self {
        Self {
            records,
            max_total_bytes: max_total_bytes.max(1),
            rejected_total: 0,
        }
    }

    fn would_fit(&self, key: &str, rec: &ReplicatedValueRecord) -> bool {
        let existing = self
            .records
            .get(key)
            .map(ReplicatedValueRecord::approx_bytes)
            .unwrap_or_default();
        self.total_bytes()
            .saturating_sub(existing)
            .saturating_add(rec.approx_bytes())
            <= self.max_total_bytes
    }
}

impl Default for InMemoryReplicatedValueStore {
    fn default() -> Self {
        Self::with_budget(u64::MAX)
    }
}

impl ReplicatedValueStore for InMemoryReplicatedValueStore {
    fn upsert(
        &mut self,
        key: impl Into<String>,
        rec: ReplicatedValueRecord,
    ) -> Result<(), ValueStoreError> {
        let key = key.into();
        if !self.would_fit(&key, &rec) {
            self.rejected_total = self.rejected_total.saturating_add(1);
            return Err(ValueStoreError::new(
                "replicated value store total byte budget exceeded",
            ));
        }
        let merged = self
            .records
            .remove(&key)
            .map(|current| current.merge(rec.clone()))
            .unwrap_or(rec);
        self.records.insert(key, merged);
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<ReplicatedValueRecord>, ValueStoreError> {
        Ok(self.records.get(key).cloned())
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
        Ok(self
            .records
            .iter()
            .map(|(key, record)| (key.clone(), record.clone()))
            .collect())
    }
}

/// AIMD flow-control window for one backup replication stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptiveWindow {
    in_flight: usize,
    max_in_flight: usize,
    floor: usize,
    ceil: usize,
}

impl AdaptiveWindow {
    /// Create a normalized AIMD window.
    pub fn new(floor: usize, initial: usize, ceil: usize) -> Self {
        let floor = floor.max(1);
        let ceil = ceil.max(floor);
        let max_in_flight = initial.max(floor).min(ceil);
        Self {
            in_flight: 0,
            max_in_flight,
            floor,
            ceil,
        }
    }

    /// Return whether another send can be admitted.
    pub fn admit(&self) -> bool {
        self.in_flight < self.max_in_flight
    }

    /// Record a send if the window admits it.
    pub fn try_acquire(&mut self) -> bool {
        if !self.admit() {
            return false;
        }
        self.in_flight = self.in_flight.saturating_add(1);
        true
    }

    /// Record an acknowledgement and adjust the AIMD limit.
    pub fn on_ack(&mut self, rtt_ok: bool) {
        self.in_flight = self.in_flight.saturating_sub(1);
        if rtt_ok {
            self.max_in_flight = self.max_in_flight.saturating_add(1).min(self.ceil);
        } else {
            self.max_in_flight = (self.max_in_flight / 2).max(self.floor);
        }
    }

    /// Current in-flight sends.
    pub fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// Current AIMD limit.
    pub fn max_in_flight(&self) -> usize {
        self.max_in_flight
    }
}

/// Quorum posture reported by readiness and status surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuorumPosture {
    /// `read_quorum + write_quorum > replication_factor`, so reads overlap writes.
    Strong,
    /// Session-scoped RYOW only; global quorum overlap is not guaranteed.
    DegradedSessionRyow,
}

impl ReplicationConfig {
    /// Return whether the configured read/write quorums overlap.
    pub fn is_strong_ryow(&self) -> bool {
        self.read_quorum.saturating_add(self.write_quorum) > self.replication_factor
    }

    /// Return the RYOW posture for readiness/status reporting.
    pub fn quorum_posture(&self) -> QuorumPosture {
        if self.is_strong_ryow() {
            QuorumPosture::Strong
        } else {
            QuorumPosture::DegradedSessionRyow
        }
    }
}

/// Client-carried write watermark for read-your-writes reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteWatermark {
    /// Partition that owns the write.
    pub partition: PartitionId,
    /// Version acknowledged by write quorum.
    pub version: ValueVersion,
    /// Authority epoch acknowledged by write quorum.
    pub epoch: ClusterEpoch,
}

impl WriteWatermark {
    /// Create a watermark.
    pub fn new(partition: PartitionId, version: ValueVersion, epoch: ClusterEpoch) -> Self {
        Self {
            partition,
            version,
            epoch,
        }
    }
}

/// Read consistency mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadConsistency {
    /// Serve only a quorum value at or above the caller's watermark.
    QuorumReadYourWrites,
    /// Existing eventual-consistency behavior.
    Eventual,
}

/// Decision returned by a quorum read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuorumReadDecision {
    /// Record safe to serve, if any.
    pub record: Option<ReplicatedValueRecord>,
    /// Whether the caller should fall back to the primary or trigger repair.
    pub requires_primary_fallback: bool,
}

/// Select a value for a RYOW quorum read.
pub fn quorum_read_your_writes(
    watermark: WriteWatermark,
    replicas: impl IntoIterator<Item = ReplicatedValueRecord>,
    read_quorum: usize,
) -> QuorumReadDecision {
    let mut candidates = replicas
        .into_iter()
        .filter(|record| record.partition == watermark.partition)
        .filter(|record| (record.version, record.epoch) >= (watermark.version, watermark.epoch))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|record| (record.version, record.epoch));

    if candidates.len() < read_quorum.max(1) {
        return QuorumReadDecision {
            record: None,
            requires_primary_fallback: true,
        };
    }

    QuorumReadDecision {
        record: candidates.pop(),
        requires_primary_fallback: false,
    }
}

/// Split-brain merge report retained in diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitBrainReport {
    /// Winning authority epoch.
    pub winner_epoch: ClusterEpoch,
    /// Losing authority epoch.
    pub loser_epoch: ClusterEpoch,
    /// Entries merged into the winner side.
    pub merged_entries: u64,
    /// Entries discarded from the loser side.
    pub discarded_entries: u64,
    /// Entries that could not be resolved deterministically.
    pub unresolved_conflicts: u64,
}

/// Merge policy for loser-side entries after split-brain detection.
pub trait MergePolicy: Send + Sync {
    /// Return the value to keep, or `None` to discard the loser-side entry.
    fn merge(
        &self,
        winner: Option<&ReplicatedValueRecord>,
        loser: &ReplicatedValueRecord,
    ) -> Option<ReplicatedValueRecord>;
}

/// Default policy: keep the highest `(version, epoch)` and let tombstones win
/// ties through [`ReplicatedValueRecord::merge`].
#[derive(Debug, Clone, Copy, Default)]
pub struct HigherVersionWins;

impl MergePolicy for HigherVersionWins {
    fn merge(
        &self,
        winner: Option<&ReplicatedValueRecord>,
        loser: &ReplicatedValueRecord,
    ) -> Option<ReplicatedValueRecord> {
        Some(
            winner
                .cloned()
                .map(|winner| winner.merge(loser.clone()))
                .unwrap_or_else(|| loser.clone()),
        )
    }
}

/// Keep loser entries only when the winner side has no value.
#[derive(Debug, Clone, Copy, Default)]
pub struct PutIfAbsent;

impl MergePolicy for PutIfAbsent {
    fn merge(
        &self,
        winner: Option<&ReplicatedValueRecord>,
        loser: &ReplicatedValueRecord,
    ) -> Option<ReplicatedValueRecord> {
        winner.cloned().or_else(|| Some(loser.clone()))
    }
}

/// Always discard loser-side entries.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiscardLoser;

impl MergePolicy for DiscardLoser {
    fn merge(
        &self,
        _winner: Option<&ReplicatedValueRecord>,
        _loser: &ReplicatedValueRecord,
    ) -> Option<ReplicatedValueRecord> {
        None
    }
}

/// Result of applying a merge policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterMergeOutcome {
    /// Winner-side records after merge.
    pub records: BTreeMap<String, ReplicatedValueRecord>,
    /// Merge report.
    pub report: SplitBrainReport,
}

/// Merge loser-side records into winner-side records using a deterministic policy.
pub fn merge_split_brain_records(
    winner_epoch: ClusterEpoch,
    loser_epoch: ClusterEpoch,
    mut winner: BTreeMap<String, ReplicatedValueRecord>,
    loser: BTreeMap<String, ReplicatedValueRecord>,
    policy: &dyn MergePolicy,
) -> ClusterMergeOutcome {
    let mut report = SplitBrainReport {
        winner_epoch,
        loser_epoch,
        ..SplitBrainReport::default()
    };

    for (key, loser_record) in loser {
        match policy.merge(winner.get(&key), &loser_record) {
            Some(record) => {
                if winner.get(&key) != Some(&record) {
                    report.merged_entries = report.merged_entries.saturating_add(1);
                }
                winner.insert(key, record);
            }
            None => {
                report.discarded_entries = report.discarded_entries.saturating_add(1);
            }
        }
    }

    ClusterMergeOutcome {
        records: winner,
        report,
    }
}

/// Return `(winner, loser)` epochs according to the authority rule.
pub fn split_brain_winner(left: ClusterEpoch, right: ClusterEpoch) -> (ClusterEpoch, ClusterEpoch) {
    if left >= right {
        (left, right)
    } else {
        (right, left)
    }
}

/// Measured write-freeze window for backup promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionFreezeWindow {
    /// Observed freeze duration in milliseconds.
    pub observed_ms: u64,
    /// Documented upper bound in milliseconds.
    pub bound_ms: u64,
}

impl PromotionFreezeWindow {
    /// Return whether the observed freeze stayed within the documented bound.
    pub fn is_bounded(self) -> bool {
        self.observed_ms <= self.bound_ms
    }
}

/// Result of resolving a healed split-brain against live side snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSplitBrainResolution {
    /// Winning authority epoch.
    pub winner_epoch: ClusterEpoch,
    /// Losing authority epoch.
    pub loser_epoch: ClusterEpoch,
    /// Merge outcome that should be committed as the topology operation.
    pub outcome: ClusterMergeOutcome,
}

/// Resolve a healed split-brain from two live side snapshots.
///
/// The higher committed epoch wins. Value reconciliation still delegates to the
/// supplied [`MergePolicy`], so tombstone dominance and custom policy behavior
/// stay identical to the isolated merge primitive.
pub fn resolve_live_split_brain(
    left_epoch: ClusterEpoch,
    left_records: BTreeMap<String, ReplicatedValueRecord>,
    right_epoch: ClusterEpoch,
    right_records: BTreeMap<String, ReplicatedValueRecord>,
    policy: &dyn MergePolicy,
) -> LiveSplitBrainResolution {
    let left_wins = left_epoch >= right_epoch;
    let (winner_epoch, loser_epoch) = split_brain_winner(left_epoch, right_epoch);
    let (winner_records, loser_records) = if left_wins {
        (left_records, right_records)
    } else {
        (right_records, left_records)
    };
    let outcome = merge_split_brain_records(
        winner_epoch,
        loser_epoch,
        winner_records,
        loser_records,
        policy,
    );
    LiveSplitBrainResolution {
        winner_epoch,
        loser_epoch,
        outcome,
    }
}

/// Live read-path decision, including readiness posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveReadYourWritesDecision {
    /// Quorum posture advertised by the current replication config.
    pub posture: QuorumPosture,
    /// Selected quorum-read decision.
    pub decision: QuorumReadDecision,
}

/// Apply the read-your-writes quorum rule using the live replication config.
pub fn live_read_your_writes(
    config: ReplicationConfig,
    watermark: WriteWatermark,
    replicas: impl IntoIterator<Item = ReplicatedValueRecord>,
) -> LiveReadYourWritesDecision {
    LiveReadYourWritesDecision {
        posture: config.quorum_posture(),
        decision: quorum_read_your_writes(watermark, replicas, config.read_quorum),
    }
}

/// Outcome of one live replication send attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveReplicationSend {
    /// Whether the peer window admitted the send.
    pub admitted: bool,
    /// Window limit after the send acknowledgement, if admitted.
    pub max_in_flight: usize,
}

/// Peer-local replication stream state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveReplicationPeer {
    node: ClusterNodeId,
    window: AdaptiveWindow,
}

impl LiveReplicationPeer {
    /// Create a peer replication stream with an AIMD window.
    pub fn new(node: impl Into<ClusterNodeId>, window: AdaptiveWindow) -> Self {
        Self {
            node: node.into(),
            window,
        }
    }

    /// Return peer node id.
    pub fn node(&self) -> &ClusterNodeId {
        &self.node
    }

    /// Return current peer window.
    pub fn window(&self) -> AdaptiveWindow {
        self.window
    }

    /// Send one record to the backup store if the peer window admits it.
    pub fn send_record<S>(
        &mut self,
        backup: &mut S,
        key: impl Into<String>,
        record: ReplicatedValueRecord,
        rtt_ok: bool,
    ) -> Result<LiveReplicationSend, ValueStoreError>
    where
        S: ReplicatedValueStore,
    {
        if !self.window.try_acquire() {
            return Ok(LiveReplicationSend {
                admitted: false,
                max_in_flight: self.window.max_in_flight(),
            });
        }
        backup.upsert(key, record)?;
        self.window.on_ack(rtt_ok);
        Ok(LiveReplicationSend {
            admitted: true,
            max_in_flight: self.window.max_in_flight(),
        })
    }
}

/// Repair a backup from a primary snapshot after anti-entropy detects lag.
pub fn anti_entropy_repair<S>(
    backup: &mut S,
    records: impl IntoIterator<Item = (String, ReplicatedValueRecord)>,
) -> Result<u64, ValueStoreError>
where
    S: ReplicatedValueStore,
{
    let mut repaired = 0_u64;
    for (key, record) in records {
        backup.upsert(key, record)?;
        repaired = repaired.saturating_add(1);
    }
    Ok(repaired)
}
