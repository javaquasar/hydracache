//! Epoch-fenced read-side topology evidence for Management Center member/partition views.

use std::collections::{BTreeMap, BTreeSet};

use hydracache_observability::{
    ManagementContractError, ManagementPartitionSnapshot, PlacementCandidateDecision,
    PlacementDecisionTrace, MAX_PLACEMENT_CANDIDATES,
};

/// Bounded immutable partition and placement evidence supplied by the hosting runtime.
///
/// The default model is empty: an absent ownership source stays unavailable instead of
/// reconstructing placement from discovery or a browser-visible member list.
#[derive(Debug, Clone, Default)]
pub struct ManagementTopologyModel {
    partition: Option<ManagementPartitionSnapshot>,
    traces: BTreeMap<String, PlacementDecisionTrace>,
}

impl ManagementTopologyModel {
    /// Create an empty, explicitly unavailable topology model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach one authoritative partition snapshot after validating its arithmetic.
    pub fn with_partition_snapshot(
        mut self,
        snapshot: ManagementPartitionSnapshot,
    ) -> Result<Self, ManagementContractError> {
        snapshot.validate()?;
        self.partition = Some(snapshot);
        self.validate_alignment()?;
        Ok(self)
    }

    /// Attach a placement trace normalized to deterministic selected/rejected order.
    pub fn with_placement_trace(
        mut self,
        trace: PlacementDecisionTrace,
    ) -> Result<Self, ManagementContractError> {
        let trace = normalize_placement_trace(trace)?;
        self.traces.insert(trace.trace_id.clone(), trace);
        self.validate_alignment()?;
        Ok(self)
    }

    /// Return the snapshot only when it matches the requested authority observation.
    pub fn partition_for(
        &self,
        authority_epoch: u64,
        observation_seq: u64,
    ) -> Option<ManagementPartitionSnapshot> {
        self.partition
            .as_ref()
            .filter(|snapshot| {
                snapshot.authority_epoch == Some(authority_epoch)
                    && snapshot.observation_seq == observation_seq
            })
            .cloned()
    }

    /// Return the retained observation so callers can distinguish absent from stale evidence.
    pub fn partition_observation(&self) -> Option<&ManagementPartitionSnapshot> {
        self.partition.as_ref()
    }

    /// Resolve one non-enumerable trace id inside the current authority epoch.
    pub fn placement_trace_for(
        &self,
        trace_id: &str,
        authority_epoch: u64,
    ) -> Option<PlacementDecisionTrace> {
        self.traces
            .get(trace_id)
            .filter(|trace| trace.topology_epoch == authority_epoch)
            .cloned()
    }

    fn validate_alignment(&self) -> Result<(), ManagementContractError> {
        if let Some(snapshot) = &self.partition {
            for trace in self.traces.values() {
                if snapshot.authority_epoch != Some(trace.topology_epoch) {
                    return Err(ManagementContractError::Incoherent {
                        field: "topology.placement_epoch",
                        reason:
                            "partition snapshot and placement trace must share one authority epoch",
                    });
                }
            }
        }
        Ok(())
    }
}

/// Normalize one trace independently of input map/reason order and reject duplicates.
pub fn normalize_placement_trace(
    mut trace: PlacementDecisionTrace,
) -> Result<PlacementDecisionTrace, ManagementContractError> {
    if trace.candidates.items.len() > MAX_PLACEMENT_CANDIDATES {
        return Err(ManagementContractError::LimitExceeded {
            field: "candidates",
            limit: MAX_PLACEMENT_CANDIDATES,
            actual: trace.candidates.items.len(),
        });
    }
    for candidate in &mut trace.candidates.items {
        candidate.reasons.sort_unstable();
        candidate.reasons.dedup();
    }
    trace.candidates.items.sort_by(candidate_order);
    trace.selected.sort();
    trace.selected.dedup();
    let mut seen = BTreeSet::new();
    if trace
        .candidates
        .items
        .iter()
        .any(|candidate| !seen.insert(candidate.node.clone()))
    {
        return Err(ManagementContractError::UnstableOrder {
            field: "candidates",
        });
    }
    trace.validate()?;
    Ok(trace)
}

fn candidate_order(
    left: &PlacementCandidateDecision,
    right: &PlacementCandidateDecision,
) -> std::cmp::Ordering {
    right
        .selected
        .cmp(&left.selected)
        .then_with(|| left.node.cmp(&right.node))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache_observability::{
        BoundedPlacementConstraints, ManagementCompleteness, ManagementObservationSource,
        OpaqueNodeIdentity, PlacementCandidatePage, PlacementOutcome, PlacementReasonCode,
        MANAGEMENT_API_SCHEMA_VERSION,
    };

    fn trace(candidates: Vec<PlacementCandidateDecision>) -> PlacementDecisionTrace {
        PlacementDecisionTrace {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            trace_id: "trace-opaque-1".to_owned(),
            topology_epoch: 9,
            request_digest: "sha256-v1:abc".to_owned(),
            requested_replicas: 1,
            constraints: BoundedPlacementConstraints::default(),
            candidates: PlacementCandidatePage {
                items: candidates,
                truncated: false,
            },
            selected: vec![OpaqueNodeIdentity::new("selected-z")],
            tie_break_seed: Some(17),
            outcome: PlacementOutcome::Proposed,
            commit_index: None,
            applied_index: None,
            reason: None,
            source: ManagementObservationSource::Live,
            completeness: ManagementCompleteness::Complete,
        }
    }

    #[test]
    fn reducer_is_permutation_invariant_and_lists_selected_first() {
        let selected = PlacementCandidateDecision {
            node: OpaqueNodeIdentity::new("selected-z"),
            selected: true,
            reasons: vec![],
        };
        let rejected = PlacementCandidateDecision {
            node: OpaqueNodeIdentity::new("rejected-a"),
            selected: false,
            reasons: vec![
                PlacementReasonCode::ZoneConflict,
                PlacementReasonCode::CapacityInsufficient,
            ],
        };
        let first =
            normalize_placement_trace(trace(vec![selected.clone(), rejected.clone()])).unwrap();
        let second = normalize_placement_trace(trace(vec![rejected, selected])).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert!(first.candidates.items[0].selected);
    }

    #[test]
    fn reducer_rejects_duplicate_candidate_identity() {
        let candidate = PlacementCandidateDecision {
            node: OpaqueNodeIdentity::new("selected-z"),
            selected: true,
            reasons: vec![],
        };
        assert!(normalize_placement_trace(trace(vec![candidate.clone(), candidate])).is_err());
    }
}
