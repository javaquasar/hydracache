use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, PartitionId};
use crate::grid::active_active::HybridLogicalClock;
use crate::grid::elasticity::RegionId;
use crate::grid::hardening::ValueVersion;

/// Stable client/application session id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Create a session id from application-provided text.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the session id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SessionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SessionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Session-visible partition/region dependency key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PartitionKey {
    /// Partition containing the observed value.
    pub partition: PartitionId,
    /// Region where the observation/write was produced.
    pub region: RegionId,
}

impl PartitionKey {
    /// Create a partition key.
    pub fn new(partition: PartitionId, region: impl Into<RegionId>) -> Self {
        Self {
            partition,
            region: region.into(),
        }
    }
}

/// Version stamp used by session watermarks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VersionStamp {
    /// Monotonic value version.
    pub version: ValueVersion,
    /// Authority epoch.
    pub epoch: ClusterEpoch,
    /// Hybrid logical timestamp.
    pub hlc: HybridLogicalClock,
}

impl VersionStamp {
    /// Create a stamp.
    pub const fn new(version: ValueVersion, epoch: ClusterEpoch, hlc: HybridLogicalClock) -> Self {
        Self {
            version,
            epoch,
            hlc,
        }
    }

    /// Return whether this stamp covers `required`.
    pub fn covers(self, required: Self) -> bool {
        self >= required
    }

    /// Conservative version-distance used by bounded staleness.
    pub fn version_distance_from(self, candidate: Self) -> u64 {
        self.version.saturating_sub(candidate.version)
    }
}

impl Default for VersionStamp {
    fn default() -> Self {
        Self {
            version: 0,
            epoch: ClusterEpoch::default(),
            hlc: HybridLogicalClock::new(0, 0),
        }
    }
}

/// Bounded session watermark carried by the client token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWatermark {
    cap: usize,
    seen: BTreeMap<PartitionKey, VersionStamp>,
    coarsened_total: u64,
}

impl SessionWatermark {
    /// Create an empty bounded watermark.
    pub fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            seen: BTreeMap::new(),
            coarsened_total: 0,
        }
    }

    /// Return the hard entry cap.
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Return retained exact entries.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Return whether no stamps are retained.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    /// Return number of conservative coarsening events.
    pub fn coarsened_total(&self) -> u64 {
        self.coarsened_total
    }

    /// Observe a stamp from a read.
    pub fn observe(&mut self, key: PartitionKey, stamp: VersionStamp) {
        self.record(key, stamp);
    }

    /// Record a stamp produced by a write.
    pub fn record_write(&mut self, key: PartitionKey, stamp: VersionStamp) {
        self.record(key, stamp);
    }

    /// Return whether the watermark covers a required stamp for this key.
    pub fn covers(&self, key: &PartitionKey, required: VersionStamp) -> bool {
        self.highest_seen(key)
            .map(|seen| seen.covers(required))
            .unwrap_or(false)
    }

    /// Return the highest exact retained stamp for this key.
    pub fn highest_seen(&self, key: &PartitionKey) -> Option<VersionStamp> {
        self.seen.get(key).copied()
    }

    /// Return retained entries for diagnostics/tests.
    pub fn entries(&self) -> &BTreeMap<PartitionKey, VersionStamp> {
        &self.seen
    }

    fn record(&mut self, key: PartitionKey, stamp: VersionStamp) {
        if let Some(existing) = self.seen.get_mut(&key) {
            if stamp > *existing {
                *existing = stamp;
            }
            return;
        }

        if self.seen.len() >= self.cap {
            self.coarsen_one();
        }
        self.seen.insert(key, stamp);
    }

    fn coarsen_one(&mut self) {
        if let Some(key) = self
            .seen
            .iter()
            .min_by_key(|(_, stamp)| **stamp)
            .map(|(key, _)| key.clone())
        {
            self.seen.remove(&key);
            self.coarsened_total = self.coarsened_total.saturating_add(1);
        }
    }
}

/// Request session mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionRequest {
    /// No session token was supplied; 0.46 behavior is preserved.
    Sessionless,
    /// A verified session token was supplied.
    Session(SessionToken),
}

impl SessionRequest {
    /// Return whether this request uses the sessionless path.
    pub fn is_sessionless(&self) -> bool {
        matches!(self, Self::Sessionless)
    }
}

/// Tamper-evident client-carried session token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionToken {
    /// Session id.
    pub session_id: SessionId,
    /// Bounded watermark carried by the session.
    pub watermark: SessionWatermark,
    /// Monotonic token nonce used for replay rejection.
    pub nonce: u64,
    /// Token issue time in logical milliseconds.
    pub issued_at_millis: u64,
    mac: u64,
}

impl SessionToken {
    /// Issue a token and compute its MAC.
    pub fn issue(
        session_id: impl Into<SessionId>,
        watermark: SessionWatermark,
        nonce: u64,
        issued_at_millis: u64,
        secret: &[u8],
    ) -> Self {
        let session_id = session_id.into();
        let mac = token_mac(&session_id, &watermark, nonce, issued_at_millis, secret);
        Self {
            session_id,
            watermark,
            nonce,
            issued_at_millis,
            mac,
        }
    }

    /// Return the token MAC.
    pub fn mac(&self) -> u64 {
        self.mac
    }

    /// Recompute and verify the token MAC/session/nonce.
    pub fn verify(
        &self,
        expected_session: &SessionId,
        secret: &[u8],
        min_nonce: u64,
    ) -> Result<(), SessionTokenError> {
        if &self.session_id != expected_session {
            return Err(SessionTokenError::WrongSession);
        }
        if self.nonce < min_nonce {
            return Err(SessionTokenError::Replayed);
        }
        let expected = token_mac(
            &self.session_id,
            &self.watermark,
            self.nonce,
            self.issued_at_millis,
            secret,
        );
        if expected != self.mac {
            return Err(SessionTokenError::Forged);
        }
        Ok(())
    }

    /// Return a copy with a deliberately corrupted MAC for tests.
    pub fn forged(mut self) -> Self {
        self.mac ^= 0xa5a5_a5a5_a5a5_a5a5;
        self
    }
}

/// Token verification error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTokenError {
    /// MAC does not match token contents.
    Forged,
    /// Token belongs to a different session id.
    WrongSession,
    /// Token nonce is older than the accepted floor.
    Replayed,
    /// Token expired.
    Expired,
}

impl fmt::Display for SessionTokenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Forged => "session token MAC verification failed",
            Self::WrongSession => "session token belongs to a different session",
            Self::Replayed => "session token replay rejected",
            Self::Expired => "session token expired",
        })
    }
}

impl std::error::Error for SessionTokenError {}

/// Aggregate session-context metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionContextMetrics {
    /// Current watermark entry count.
    pub session_watermark_entries: u64,
    /// Watermark coarsening events.
    pub session_watermark_coarsened_total: u64,
    /// Token rejection events.
    pub session_token_rejected_total: u64,
}

impl From<&SessionWatermark> for SessionContextMetrics {
    fn from(watermark: &SessionWatermark) -> Self {
        Self {
            session_watermark_entries: watermark.len() as u64,
            session_watermark_coarsened_total: watermark.coarsened_total(),
            session_token_rejected_total: 0,
        }
    }
}

fn token_mac(
    session_id: &SessionId,
    watermark: &SessionWatermark,
    nonce: u64,
    issued_at_millis: u64,
    secret: &[u8],
) -> u64 {
    let mut hash = Fnv64::new();
    hash.bytes(secret);
    hash.bytes(session_id.as_str().as_bytes());
    hash.u64(nonce);
    hash.u64(issued_at_millis);
    hash.u64(watermark.cap() as u64);
    hash.u64(watermark.coarsened_total());
    for (key, stamp) in watermark.entries() {
        hash.u32(key.partition.value());
        hash.bytes(key.region.as_str().as_bytes());
        hash.u64(stamp.version);
        hash.u64(stamp.epoch.value());
        hash.u64(stamp.hlc.wall());
        hash.u64(u64::from(stamp.hlc.logical()));
    }
    hash.finish()
}

#[derive(Debug, Clone, Copy)]
struct Fnv64(u64);

impl Fnv64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}
