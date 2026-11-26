//! Bounded snapshot transport for replacing an inactive synchronous backup.

use std::time::Duration;

use hydracache::{CacheError, PartitionReplicaRepair, PartitionValuePrefix};
use hydracache_cluster_raft::RaftPartitionSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::imap_value_journal::Mutation;

pub(crate) const CHUNK_BYTES: usize = 64 * 1024;
pub(crate) const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_IMAGE_ROWS: usize = 4096;
pub(crate) const REPAIR_WAIT: Duration = Duration::from_secs(30);
pub(crate) const MAX_DELTA_ROWS: u64 = 128;
pub(crate) const MAX_DELTA_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const FINAL_DELTA_ROWS: u64 = 8;
pub(crate) const FINAL_DELTA_BYTES: usize = 512 * 1024;
pub(crate) const FINAL_CUT_WAIT: Duration = Duration::from_secs(2);

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub(crate) enum RepairStep {
    Status,
    Chunk {
        index: u32,
        digest: [u8; 32],
        bytes: Vec<u8>,
    },
    Install,
    Delta {
        mutation: Box<Mutation>,
    },
    Seal {
        prefix: Option<PartitionValuePrefix>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RepairReceipt {
    pub manifest: RepairManifest,
    pub next: u32,
    pub installed: bool,
    #[serde(default)]
    pub applied: Option<PartitionValuePrefix>,
}

/// Result of a staged durable snapshot copy and Raft backup replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaRepairOutcome {
    pub plan: PartitionReplicaRepair,
    pub resumed_from: u32,
    pub chunks: u32,
    pub deltas: u64,
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

/// The installed snapshot may trail the latest certificate until final seal.
pub(crate) fn admits_transfer(
    snapshot: &RaftPartitionSnapshot,
    plan: &PartitionReplicaRepair,
) -> bool {
    let latest = snapshot.value_prefixes.get(&plan.partition).copied();
    snapshot.applied_index == snapshot.commit_index
        && is_prefix_at_most(plan.required, latest)
        && snapshot
            .value_promotions
            .get(&plan.partition)
            .is_some_and(|active| {
                plan.replacement_slot(active, snapshot.epoch, plan.required, &snapshot.members)
                    .is_some()
            })
}

pub(crate) fn version(prefix: Option<PartitionValuePrefix>) -> u64 {
    prefix.map_or(0, |prefix| prefix.version)
}

pub(crate) fn is_prefix_at_most(
    prefix: Option<PartitionValuePrefix>,
    limit: Option<PartitionValuePrefix>,
) -> bool {
    prefix == limit
        || match (prefix, limit) {
            (None, Some(_)) => true,
            (Some(earlier), Some(later)) => {
                earlier.partition == later.partition
                    && earlier.version < later.version
                    && earlier.epoch <= later.epoch
            }
            _ => false,
        }
}

pub(crate) fn bytes_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub(crate) fn repair_error(reason: &str) -> CacheError {
    CacheError::Backend(format!("W10f replica repair: {reason}"))
}
