//! Bounded snapshot transport for replacing an inactive synchronous backup.

use std::time::Duration;

use hydracache::{CacheError, PartitionReplicaRepair};
use hydracache_cluster_raft::RaftPartitionSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const CHUNK_BYTES: usize = 64 * 1024;
pub(crate) const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_IMAGE_ROWS: usize = 4096;
pub(crate) const REPAIR_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RepairManifest {
    pub plan: PartitionReplicaRepair,
    pub scope: [u8; 32],
    pub image_len: usize,
}

impl RepairManifest {
    pub fn chunks(&self) -> u32 {
        self.image_len.div_ceil(CHUNK_BYTES) as u32
    }

    pub fn chunk_len(&self, index: u32) -> Option<usize> {
        (index < self.chunks())
            .then(|| (self.image_len - index as usize * CHUNK_BYTES).min(CHUNK_BYTES))
    }

    pub fn is_bounded(&self) -> bool {
        (1..=MAX_IMAGE_BYTES).contains(&self.image_len)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RepairRequest {
    pub manifest: RepairManifest,
    pub step: RepairStep,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub(crate) enum RepairStep {
    Status,
    Chunk {
        index: u32,
        digest: [u8; 32],
        bytes: Vec<u8>,
    },
    Install,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RepairReceipt {
    pub manifest: RepairManifest,
    pub next: u32,
    pub installed: bool,
}

/// Result of a staged durable snapshot copy and Raft backup replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaRepairOutcome {
    pub plan: PartitionReplicaRepair,
    pub resumed_from: u32,
    pub chunks: u32,
}

pub(crate) fn admits_repair(
    snapshot: &RaftPartitionSnapshot,
    plan: &PartitionReplicaRepair,
) -> bool {
    snapshot.applied_index == snapshot.commit_index
        && snapshot
            .value_promotions
            .get(&plan.partition)
            .is_some_and(|active| {
                plan.replacement_slot(
                    active,
                    snapshot.epoch,
                    snapshot.value_prefixes.get(&plan.partition).copied(),
                    &snapshot.members,
                )
                .is_some()
            })
}

pub(crate) fn bytes_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub(crate) fn repair_error(reason: &str) -> CacheError {
    CacheError::Backend(format!("W10f replica repair: {reason}"))
}
