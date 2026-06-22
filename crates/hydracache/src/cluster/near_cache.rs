use super::*;

/// Action selected by near-cache watermark repair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NearCacheRepairAction {
    /// Apply the frame normally.
    Apply,
    /// Owner generation changed; clear the partition before applying/refreshing.
    ClearPartition,
    /// A sequence gap was observed; invalidate conservatively.
    InvalidateConservatively,
}

/// Per-partition near-cache watermark metadata.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaDataContainer {
    last_uuid: Option<ClusterGeneration>,
    last_seq: u64,
}

impl MetaDataContainer {
    /// Return the last owner generation observed for this partition.
    pub fn last_uuid(&self) -> Option<ClusterGeneration> {
        self.last_uuid
    }

    /// Return the last applied invalidation sequence.
    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// Update the watermark from an invalidation frame.
    pub fn on_watermark(
        &mut self,
        generation: Option<ClusterGeneration>,
        message_id: Option<u64>,
    ) -> NearCacheRepairAction {
        let generation = generation.unwrap_or_default();
        let seq = message_id.unwrap_or_default();

        if self.last_uuid != Some(generation) {
            self.last_uuid = Some(generation);
            self.last_seq = seq;
            return NearCacheRepairAction::ClearPartition;
        }

        if seq > self.last_seq.saturating_add(1) {
            self.last_seq = seq;
            return NearCacheRepairAction::InvalidateConservatively;
        }

        self.last_seq = self.last_seq.max(seq);
        NearCacheRepairAction::Apply
    }
}
