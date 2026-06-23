use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{partition_for_key, ClusterEpoch, ClusterNodeId, PartitionId};
use crate::grid::elasticity::RegionId;
use crate::grid::hardening::{
    quorum_read_your_writes, MergePolicy, ReplicatedValueRecord, ValueVersion, WriteWatermark,
};

/// Write-authority mode for a geo-distributed partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteAuthority {
    /// The 0.43 default: only the home region may accept writes.
    HomeRegionOnly { home: RegionId },
    /// 0.45 opt-in: any region may accept a local write and converge via home.
    ActiveActive { home: RegionId },
}

impl WriteAuthority {
    /// Return the home region that still owns authoritative ordering.
    pub fn home(&self) -> &RegionId {
        match self {
            Self::HomeRegionOnly { home } | Self::ActiveActive { home } => home,
        }
    }

    /// Return whether remote regions may acknowledge local writes.
    pub fn accepts_local_writes_in(&self, region: &RegionId) -> bool {
        match self {
            Self::HomeRegionOnly { home } => home == region,
            Self::ActiveActive { .. } => true,
        }
    }
}

/// Explicit acknowledgement that active-active weakens cross-region consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveActiveAcknowledgement {
    /// The caller did not acknowledge bounded cross-region staleness.
    Missing,
    /// The caller accepted the bounded-staleness contract explicitly.
    BoundedStalenessAccepted,
}

/// Error returned when active-active is requested without the loud acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveActiveConfigError {
    message: String,
}

impl ActiveActiveConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ActiveActiveConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ActiveActiveConfigError {}

/// Active-active mode configuration for one cache/grid slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveActiveConfig {
    /// Write authority mode.
    pub authority: WriteAuthority,
    /// Whether the weaker cross-region contract was acknowledged.
    pub acknowledgement: ActiveActiveAcknowledgement,
}

impl ActiveActiveConfig {
    /// Create the 0.43-compatible single-authority mode.
    pub fn home_region_only(home: impl Into<RegionId>) -> Self {
        Self {
            authority: WriteAuthority::HomeRegionOnly { home: home.into() },
            acknowledgement: ActiveActiveAcknowledgement::Missing,
        }
    }

    /// Create active-active mode only when the caller acknowledges bounded staleness.
    pub fn active_active(
        home: impl Into<RegionId>,
        acknowledgement: ActiveActiveAcknowledgement,
    ) -> Result<Self, ActiveActiveConfigError> {
        if acknowledgement != ActiveActiveAcknowledgement::BoundedStalenessAccepted {
            return Err(ActiveActiveConfigError::new(
                "active-active requires explicit bounded-staleness acknowledgement",
            ));
        }
        Ok(Self {
            authority: WriteAuthority::ActiveActive { home: home.into() },
            acknowledgement,
        })
    }

    /// Return whether this config is ready to accept active-active writes.
    pub fn active_active_ready(&self) -> bool {
        matches!(self.authority, WriteAuthority::ActiveActive { .. })
            && self.acknowledgement == ActiveActiveAcknowledgement::BoundedStalenessAccepted
    }
}

/// Hybrid logical clock timestamp used for deterministic cross-region tie-breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HybridLogicalClock {
    wall: u64,
    logical: u32,
}

impl HybridLogicalClock {
    /// Create a timestamp from wall/logical components.
    pub const fn new(wall: u64, logical: u32) -> Self {
        Self { wall, logical }
    }

    /// Return the physical component observed by the node.
    pub const fn wall(self) -> u64 {
        self.wall
    }

    /// Return the logical component.
    pub const fn logical(self) -> u32 {
        self.logical
    }

    /// Advance the clock for a local event.
    pub fn tick(&mut self, observed_wall: u64) -> Self {
        if observed_wall > self.wall {
            self.wall = observed_wall;
            self.logical = 0;
        } else {
            self.logical = self.logical.saturating_add(1);
        }
        *self
    }

    /// Observe a remote timestamp and return the next local timestamp.
    pub fn observe(&mut self, remote: Self, observed_wall: u64) -> Self {
        let max_wall = self.wall.max(remote.wall).max(observed_wall);
        self.logical = if max_wall == self.wall && max_wall == remote.wall {
            self.logical.max(remote.logical).saturating_add(1)
        } else if max_wall == self.wall {
            self.logical.saturating_add(1)
        } else if max_wall == remote.wall {
            remote.logical.saturating_add(1)
        } else {
            0
        };
        self.wall = max_wall;
        *self
    }
}

/// Active-active write stamped at the accepting region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoWrite {
    /// Cache key being written.
    pub key: String,
    /// Owning partition.
    pub partition: PartitionId,
    /// Monotonic value version.
    pub version: ValueVersion,
    /// Authority epoch.
    pub epoch: ClusterEpoch,
    /// HLC used only as a deterministic tie-break.
    pub hlc: HybridLogicalClock,
    /// Region that accepted the write locally.
    pub origin_region: RegionId,
    /// Node that accepted the write.
    pub origin_node: ClusterNodeId,
    /// Sealed value bytes.
    pub value: Vec<u8>,
}

impl GeoWrite {
    /// Convert a write into a replicated value record.
    pub fn to_record(&self) -> ReplicatedValueRecord {
        ReplicatedValueRecord::value(self.partition, self.version, self.epoch, self.value.clone())
    }
}

/// Result of accepting a local write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoWriteAck {
    /// Watermark visible inside the accepting region.
    pub watermark: WriteWatermark,
    /// Whether the acknowledgement path crossed the WAN.
    pub crossed_wan: bool,
    /// Regions that must receive the write asynchronously.
    pub replication_targets: Vec<RegionId>,
}

/// Deterministic active-active state machine used by release gates and adapters.
#[derive(Debug, Clone)]
pub struct ActiveActiveState {
    config: ActiveActiveConfig,
    local_region: RegionId,
    local_node: ClusterNodeId,
    partition_count: u32,
    epoch: ClusterEpoch,
    next_version: ValueVersion,
    clock: HybridLogicalClock,
    peers: Vec<RegionId>,
    records: BTreeMap<String, ReplicatedValueRecord>,
    writes: BTreeMap<String, GeoWrite>,
    pending: Vec<GeoWrite>,
}

impl ActiveActiveState {
    /// Create a deterministic active-active state machine.
    pub fn new(
        config: ActiveActiveConfig,
        local_region: impl Into<RegionId>,
        local_node: impl Into<ClusterNodeId>,
        partition_count: u32,
        epoch: ClusterEpoch,
        peers: Vec<RegionId>,
    ) -> Self {
        Self {
            config,
            local_region: local_region.into(),
            local_node: local_node.into(),
            partition_count: partition_count.max(1),
            epoch,
            next_version: 1,
            clock: HybridLogicalClock::new(0, 0),
            peers,
            records: BTreeMap::new(),
            writes: BTreeMap::new(),
            pending: Vec::new(),
        }
    }

    /// Accept a local write, apply it locally, and enqueue cross-region propagation.
    pub fn accept_local_write(
        &mut self,
        key: impl Into<String>,
        value: impl Into<Vec<u8>>,
        observed_wall: u64,
    ) -> Result<GeoWriteAck, ActiveActiveConfigError> {
        if !self
            .config
            .authority
            .accepts_local_writes_in(&self.local_region)
        {
            return Err(ActiveActiveConfigError::new(
                "local region is not allowed to accept writes for this authority mode",
            ));
        }
        if matches!(self.config.authority, WriteAuthority::ActiveActive { .. })
            && !self.config.active_active_ready()
        {
            return Err(ActiveActiveConfigError::new(
                "active-active write refused without bounded-staleness acknowledgement",
            ));
        }

        let key = key.into();
        let partition = partition_for_key(&key, self.partition_count);
        let version = self.next_version;
        self.next_version = self.next_version.saturating_add(1);
        let hlc = self.clock.tick(observed_wall);
        let write = GeoWrite {
            key: key.clone(),
            partition,
            version,
            epoch: self.epoch,
            hlc,
            origin_region: self.local_region.clone(),
            origin_node: self.local_node.clone(),
            value: value.into(),
        };
        self.records.insert(key, write.to_record());
        self.writes.insert(write.key.clone(), write.clone());
        let targets = self.replication_targets_for(&write.origin_region);
        if matches!(self.config.authority, WriteAuthority::ActiveActive { .. }) {
            self.pending.push(write);
        }
        Ok(GeoWriteAck {
            watermark: WriteWatermark::new(partition, version, self.epoch),
            crossed_wan: false,
            replication_targets: targets,
        })
    }

    /// Reconcile a remote write with local authoritative state.
    pub fn reconcile_remote(&mut self, write: GeoWrite, policy: &dyn MergePolicy) {
        self.clock.observe(write.hlc, write.hlc.wall());
        let key = write.key.clone();
        if let Some(existing) = self.writes.get(&key) {
            let winner = choose_hlc_tiebreak(existing, &write).clone();
            self.records.insert(key.clone(), winner.to_record());
            self.writes.insert(key, winner);
            return;
        }

        let incoming = write.to_record();
        let merged = policy
            .merge(self.records.get(&key), &incoming)
            .unwrap_or(incoming);
        self.records.insert(key, merged);
        self.writes.insert(write.key.clone(), write);
    }

    /// Return a record by key.
    pub fn record(&self, key: &str) -> Option<&ReplicatedValueRecord> {
        self.records.get(key)
    }

    /// Drain pending cross-region writes in deterministic FIFO order.
    pub fn drain_pending(&mut self) -> Vec<GeoWrite> {
        std::mem::take(&mut self.pending)
    }

    /// Check the existing 0.42 in-region read-your-writes contract.
    pub fn intra_region_read_your_writes_holds(
        &self,
        watermark: WriteWatermark,
        read_quorum: usize,
    ) -> bool {
        let replicas = self
            .records
            .values()
            .filter(|record| record.partition == watermark.partition)
            .cloned()
            .collect::<Vec<_>>();
        !quorum_read_your_writes(watermark, replicas, read_quorum).requires_primary_fallback
    }

    fn replication_targets_for(&self, origin: &RegionId) -> Vec<RegionId> {
        let mut targets = Vec::new();
        if self.config.authority.home() != origin {
            targets.push(self.config.authority.home().clone());
        }
        for peer in &self.peers {
            if peer != origin && !targets.contains(peer) {
                targets.push(peer.clone());
            }
        }
        targets
    }
}

/// Deterministically choose between equal `(version, epoch)` writes using HLC.
pub fn choose_hlc_tiebreak<'a>(left: &'a GeoWrite, right: &'a GeoWrite) -> &'a GeoWrite {
    match (left.version, left.epoch).cmp(&(right.version, right.epoch)) {
        std::cmp::Ordering::Greater => left,
        std::cmp::Ordering::Less => right,
        std::cmp::Ordering::Equal => {
            if (left.hlc, &left.origin_region, &left.origin_node)
                >= (right.hlc, &right.origin_region, &right.origin_node)
            {
                left
            } else {
                right
            }
        }
    }
}
