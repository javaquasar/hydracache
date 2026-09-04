use hydracache_observability::{
    AdmissionState, BoundedCount, BoundedPage, BoundedPlacementConstraints, CatchUpState,
    ClusterFormationSnapshot, ConsensusProgressSnapshot, DiscoveryState, DurableRecoveryStatus,
    FormationReasonCode, ManagementCapabilities, ManagementCapability,
    ManagementCapabilityAvailability, ManagementCapabilityId, ManagementCompleteness,
    ManagementConsensusRole, ManagementContractError, ManagementEnvelope,
    ManagementObservationSource, ManagementWarning, ManagementWarningCode, OpaqueCursor,
    OpaqueNodeIdentity, PlacementCandidateDecision, PlacementCandidatePage, PlacementDecisionTrace,
    PlacementOutcome, PlacementReasonCode, RecoveryArtifactKind, RecoveryOutcome, RecoveryPhase,
    RecoveryReasonCode, RecoveryScope, RecoveryWatermark, RepairState, ServingState,
    TransportState, MANAGEMENT_API_SCHEMA_VERSION, MAX_MANAGEMENT_NODE_ID_BYTES,
    MAX_MANAGEMENT_PAGE_ITEMS, MAX_MANAGEMENT_WARNINGS, MAX_PLACEMENT_CANDIDATES,
    MAX_PLACEMENT_LABELS, MAX_PLACEMENT_LABEL_BYTES, MAX_PLACEMENT_REASONS_PER_CANDIDATE,
    MAX_PLACEMENT_SELECTED,
};

#[test]
fn envelope_rejects_unavailable_complete_and_missing_source_warnings() {
    let mut envelope = ManagementEnvelope {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        observation_seq: 7,
        authority_epoch: Some(3),
        captured_at_unix_ms: 1_000,
        source: ManagementObservationSource::Unavailable,
        completeness: ManagementCompleteness::Complete,
        stale_after_ms: 2_000,
        warnings: Vec::new(),
        data: (),
    };
    assert_incoherent(envelope.validate(), "completeness");

    envelope.source = ManagementObservationSource::Live;
    envelope.warnings.push(ManagementWarning {
        code: ManagementWarningCode::SourceUnavailable,
        affected_count: Some(1),
    });
    assert_incoherent(envelope.validate(), "warnings");

    envelope.completeness = ManagementCompleteness::Partial;
    assert_eq!(envelope.validate(), Ok(()));
    envelope.source = ManagementObservationSource::Unknown;
    envelope.completeness = ManagementCompleteness::Complete;
    envelope.warnings.clear();
    assert_incoherent(envelope.validate(), "completeness");
    envelope.source = ManagementObservationSource::Live;
    envelope.completeness = ManagementCompleteness::Unknown;
    assert_incoherent(envelope.validate(), "completeness");
    envelope.completeness = ManagementCompleteness::Partial;
    envelope.warnings = vec![
        ManagementWarning {
            code: ManagementWarningCode::PartialObservation,
            affected_count: None,
        };
        MAX_MANAGEMENT_WARNINGS + 1
    ];
    assert_limit(envelope.validate(), "warnings", MAX_MANAGEMENT_WARNINGS);
}

#[test]
fn bounded_page_requires_bounded_items_and_truthful_continuation() {
    let mut page = BoundedPage {
        items: vec![0_u8; MAX_MANAGEMENT_PAGE_ITEMS + 1],
        next_cursor: None,
        truncated: true,
    };
    assert_limit(page.validate(), "page.items", MAX_MANAGEMENT_PAGE_ITEMS);

    page.items.truncate(1);
    page.next_cursor = Some(OpaqueCursor::new("opaque"));
    page.truncated = false;
    assert_incoherent(page.validate(), "next_cursor");
    page.truncated = true;
    assert_eq!(page.validate(), Ok(()));
}

#[test]
fn capability_list_is_sorted_and_unavailable_entries_explain_why() {
    let mut capabilities = ManagementCapabilities {
        capabilities: vec![
            ManagementCapability {
                id: ManagementCapabilityId::ClusterFormation,
                availability: ManagementCapabilityAvailability::Available,
                reason: None,
            },
            ManagementCapability {
                id: ManagementCapabilityId::ConsensusProgress,
                availability: ManagementCapabilityAvailability::Unavailable,
                reason: Some(ManagementWarningCode::SourceUnavailable),
            },
        ],
    };
    assert_eq!(capabilities.validate(), Ok(()));
    capabilities.capabilities.reverse();
    assert_unstable(capabilities.validate(), "capabilities");
    capabilities.capabilities.reverse();
    capabilities.capabilities[1].reason = None;
    assert_incoherent(capabilities.validate(), "capability.reason");
}

fn live_formation() -> ClusterFormationSnapshot {
    ClusterFormationSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node: OpaqueNodeIdentity::new("member-a"),
        generation: 7,
        candidate_observation_seq: Some(11),
        authority_epoch: Some(13),
        discovery: DiscoveryState::Discovered,
        transport: TransportState::Authenticated,
        admission: AdmissionState::Admitted,
        consensus_role: ManagementConsensusRole::Voter,
        catch_up: CatchUpState::Current,
        serving: ServingState::Serving,
        blocker: None,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
}

fn placement_trace() -> PlacementDecisionTrace {
    PlacementDecisionTrace {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        trace_id: "placement-17".to_owned(),
        topology_epoch: 13,
        request_digest: "sha256:bounded-redacted-digest".to_owned(),
        requested_replicas: 2,
        constraints: BoundedPlacementConstraints {
            required_labels: vec!["region=eu".to_owned()],
            excluded_labels: vec!["maintenance=true".to_owned()],
            required_zones: 2,
            unique_hosts: true,
            truncated: false,
        },
        candidates: PlacementCandidatePage {
            items: vec![
                PlacementCandidateDecision {
                    node: OpaqueNodeIdentity::new("member-a"),
                    selected: true,
                    reasons: Vec::new(),
                },
                PlacementCandidateDecision {
                    node: OpaqueNodeIdentity::new("member-b"),
                    selected: true,
                    reasons: Vec::new(),
                },
                PlacementCandidateDecision {
                    node: OpaqueNodeIdentity::new("member-c"),
                    selected: false,
                    reasons: vec![PlacementReasonCode::ZoneConflict],
                },
            ],
            truncated: false,
        },
        selected: vec![
            OpaqueNodeIdentity::new("member-a"),
            OpaqueNodeIdentity::new("member-b"),
        ],
        tie_break_seed: Some(29),
        outcome: PlacementOutcome::Applied,
        commit_index: Some(31),
        applied_index: Some(31),
        reason: None,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
}

fn clean_recovery() -> DurableRecoveryStatus {
    DurableRecoveryStatus {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        scope: RecoveryScope::Namespace,
        outcome: RecoveryOutcome::Clean,
        phase: RecoveryPhase::Complete,
        artifact: RecoveryArtifactKind::DurableValueStore,
        artifact_format_version: Some(1),
        validated_watermark: Some(RecoveryWatermark {
            authority_epoch: 13,
            version: 21,
        }),
        records_checked: BoundedCount::exact(100),
        records_recovered: BoundedCount::exact(100),
        records_discarded: BoundedCount::exact(0),
        corrupt_records: BoundedCount::exact(0),
        repair: RepairState::NotRequired,
        reason: None,
        started_at_unix_ms: Some(1_000),
        completed_at_unix_ms: Some(1_100),
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
}

fn progress() -> ConsensusProgressSnapshot {
    ConsensusProgressSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node: OpaqueNodeIdentity::new("member-a"),
        generation: 7,
        authority_epoch: Some(13),
        commit_index: Some(31),
        applied_index: Some(29),
        last_snapshot_index: Some(20),
        catch_up_target: Some(31),
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
}

#[test]
fn formation_requires_authority_authentication_and_currentness_before_serving() {
    let valid = live_formation();
    valid.validate().unwrap();

    let mut admitted_without_epoch = valid.clone();
    admitted_without_epoch.authority_epoch = None;
    assert_incoherent(admitted_without_epoch.validate(), "authority_epoch");

    let mut role_without_admission = valid.clone();
    role_without_admission.admission = AdmissionState::Pending;
    role_without_admission.serving = ServingState::Blocked;
    assert_incoherent(role_without_admission.validate(), "consensus_role");

    let mutations: [fn(&mut ClusterFormationSnapshot); 5] = [
        |value| value.transport = TransportState::Reachable,
        |value| {
            value.admission = AdmissionState::Pending;
            value.consensus_role = ManagementConsensusRole::None;
        },
        |value| value.consensus_role = ManagementConsensusRole::None,
        |value| value.catch_up = CatchUpState::Behind,
        |value| value.source = ManagementObservationSource::Modeled,
    ];
    for mutation in mutations {
        let mut invalid = valid.clone();
        mutation(&mut invalid);
        assert_incoherent(invalid.validate(), "serving");
    }
}

#[test]
fn formation_unknowns_remain_unknown_and_cannot_be_complete() {
    let mut json = serde_json::to_value(live_formation()).unwrap();
    json["transport"] = serde_json::json!("future_transport_state");
    json["serving"] = serde_json::json!("blocked");
    let decoded: ClusterFormationSnapshot = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.transport, TransportState::Unknown);
    assert_incoherent(decoded.validate(), "completeness");

    let partial = ClusterFormationSnapshot {
        completeness: ManagementCompleteness::Partial,
        serving: ServingState::Blocked,
        transport: TransportState::Unknown,
        ..live_formation()
    };
    partial.validate().unwrap();
}

#[test]
fn formation_rejects_unknown_schema_and_invalid_identity_bounds() {
    let mut invalid_schema = live_formation();
    invalid_schema.schema_version += 1;
    assert!(matches!(
        invalid_schema.validate(),
        Err(ManagementContractError::UnsupportedSchema { .. })
    ));

    for node in [String::new(), "x".repeat(MAX_MANAGEMENT_NODE_ID_BYTES + 1)] {
        let mut invalid = live_formation();
        invalid.node = OpaqueNodeIdentity::new(node);
        assert!(matches!(
            invalid.validate(),
            Err(ManagementContractError::InvalidOpaqueId { field: "node", .. })
        ));
    }
}

#[test]
fn formation_and_observation_vocabularies_are_stable_and_unknown_safe() {
    let reasons = [
        FormationReasonCode::ClusterIdentityMismatch,
        FormationReasonCode::DuplicateNodeIdentity,
        FormationReasonCode::GenerationRegression,
        FormationReasonCode::TransportUnreachable,
        FormationReasonCode::PeerUnauthenticated,
        FormationReasonCode::ProtocolIncompatible,
        FormationReasonCode::NotAdmitted,
        FormationReasonCode::LearnerBehind,
        FormationReasonCode::QuorumUnavailable,
        FormationReasonCode::AuthorityUnknown,
        FormationReasonCode::Draining,
        FormationReasonCode::SourceUnavailable,
        FormationReasonCode::Unknown,
    ];
    let encoded = serde_json::to_value(reasons).unwrap();
    assert_eq!(encoded[0], "cluster-identity-mismatch");
    assert_eq!(encoded[11], "source-unavailable");
    assert_eq!(encoded[12], "unknown");

    let source: ManagementObservationSource =
        serde_json::from_str("\"future-live-source\"").unwrap();
    let completeness: ManagementCompleteness =
        serde_json::from_str("\"future-completeness\"").unwrap();
    assert_eq!(source, ManagementObservationSource::Unknown);
    assert_eq!(completeness, ManagementCompleteness::Unknown);
}

#[test]
fn placement_trace_is_stable_bounded_and_selection_consistent() {
    let valid = placement_trace();
    valid.validate().unwrap();
    let first = serde_json::to_vec(&valid).unwrap();
    let second = serde_json::to_vec(&valid).unwrap();
    assert_eq!(first, second);

    let mut unsorted_candidates = valid.clone();
    unsorted_candidates.candidates.items.swap(0, 1);
    assert_unstable(unsorted_candidates.validate(), "candidates");

    let mut duplicate_selected = valid.clone();
    duplicate_selected.selected[1] = duplicate_selected.selected[0].clone();
    assert_unstable(duplicate_selected.validate(), "selected");

    let mut mismatched_flag = valid.clone();
    mismatched_flag.candidates.items[0].selected = false;
    assert_incoherent(mismatched_flag.validate(), "candidate.selected");

    let mut missing_reason = valid;
    missing_reason.candidates.items[2].reasons.clear();
    assert_incoherent(missing_reason.validate(), "candidate.reasons");
}

#[test]
fn placement_trace_enforces_every_collection_and_identifier_bound() {
    let mut too_many_candidates = placement_trace();
    too_many_candidates.requested_replicas = 1;
    too_many_candidates.selected = vec![OpaqueNodeIdentity::new("selected")];
    too_many_candidates.candidates.items = (0..=MAX_PLACEMENT_CANDIDATES)
        .map(|index| PlacementCandidateDecision {
            node: OpaqueNodeIdentity::new(format!("candidate-{index:04}")),
            selected: false,
            reasons: vec![PlacementReasonCode::CapacityInsufficient],
        })
        .collect();
    assert_limit(
        too_many_candidates.validate(),
        "candidates",
        MAX_PLACEMENT_CANDIDATES,
    );

    let mut too_many_selected = placement_trace();
    too_many_selected.requested_replicas = (MAX_PLACEMENT_SELECTED + 1) as u16;
    too_many_selected.selected = (0..=MAX_PLACEMENT_SELECTED)
        .map(|index| OpaqueNodeIdentity::new(format!("member-{index:04}")))
        .collect();
    assert_limit(
        too_many_selected.validate(),
        "selected",
        MAX_PLACEMENT_SELECTED,
    );

    let mut too_many_reasons = placement_trace();
    too_many_reasons.candidates.items[2].reasons =
        vec![PlacementReasonCode::CapacityInsufficient; MAX_PLACEMENT_REASONS_PER_CANDIDATE + 1];
    assert_limit(
        too_many_reasons.validate(),
        "candidate.reasons",
        MAX_PLACEMENT_REASONS_PER_CANDIDATE,
    );

    let mut too_many_labels = placement_trace();
    too_many_labels.constraints.required_labels = (0..=MAX_PLACEMENT_LABELS)
        .map(|index| format!("label-{index:04}"))
        .collect();
    assert_limit(
        too_many_labels.validate(),
        "required_labels",
        MAX_PLACEMENT_LABELS,
    );

    for label in [String::new(), "x".repeat(MAX_PLACEMENT_LABEL_BYTES + 1)] {
        let mut invalid = placement_trace();
        invalid.constraints.required_labels = vec![label];
        assert!(matches!(
            invalid.validate(),
            Err(ManagementContractError::InvalidOpaqueId {
                field: "required_labels",
                ..
            })
        ));
    }

    for (field, value) in [
        ("trace_id", String::new()),
        ("request_digest", String::new()),
    ] {
        let mut invalid = placement_trace();
        if field == "trace_id" {
            invalid.trace_id = value;
        } else {
            invalid.request_digest = value;
        }
        assert!(matches!(
            invalid.validate(),
            Err(ManagementContractError::InvalidOpaqueId { field: found, .. }) if found == field
        ));
    }
}

#[test]
fn placement_progress_never_collapses_proposed_committed_and_applied() {
    let mut zero_replicas = placement_trace();
    zero_replicas.requested_replicas = 0;
    assert_incoherent(zero_replicas.validate(), "requested_replicas");

    let mut too_many_selected = placement_trace();
    too_many_selected.requested_replicas = 1;
    assert_incoherent(too_many_selected.validate(), "selected");

    let mut missing_commit = placement_trace();
    missing_commit.commit_index = None;
    assert_incoherent(missing_commit.validate(), "commit_index");

    let mut missing_apply = placement_trace();
    missing_apply.applied_index = None;
    assert_incoherent(missing_apply.validate(), "outcome");

    let mut apply_ahead = placement_trace();
    apply_ahead.applied_index = Some(32);
    assert_incoherent(apply_ahead.validate(), "applied_index");

    let mut incomplete_applied = placement_trace();
    incomplete_applied.selected.pop();
    incomplete_applied.candidates.items[1].selected = false;
    incomplete_applied.candidates.items[1].reasons = vec![PlacementReasonCode::Unreachable];
    assert_incoherent(incomplete_applied.validate(), "outcome");
}

#[test]
fn placement_reason_vocabulary_is_complete_and_unknown_safe() {
    let reasons = [
        PlacementReasonCode::WrongCluster,
        PlacementReasonCode::WrongRole,
        PlacementReasonCode::NotAdmitted,
        PlacementReasonCode::Unreachable,
        PlacementReasonCode::StaleGeneration,
        PlacementReasonCode::ProtocolIncompatible,
        PlacementReasonCode::CapabilityMissing,
        PlacementReasonCode::CapacityInsufficient,
        PlacementReasonCode::ZoneConflict,
        PlacementReasonCode::HostConflict,
        PlacementReasonCode::RequiredLabelMissing,
        PlacementReasonCode::ExcludedLabel,
        PlacementReasonCode::AlreadySelected,
        PlacementReasonCode::IgnoredByPlan,
        PlacementReasonCode::QuorumWouldBeUnsafe,
        PlacementReasonCode::Unknown,
    ];
    let encoded = serde_json::to_value(reasons).unwrap();
    assert_eq!(encoded[0], "wrong-cluster");
    assert_eq!(encoded[14], "quorum-would-be-unsafe");

    let decoded: PlacementReasonCode = serde_json::from_str("\"future-placement-reason\"").unwrap();
    assert_eq!(decoded, PlacementReasonCode::Unknown);
}

#[test]
fn recovery_outcomes_require_their_declared_evidence() {
    clean_recovery().validate().unwrap();

    let repaired = DurableRecoveryStatus {
        outcome: RecoveryOutcome::Repaired,
        records_recovered: BoundedCount::exact(1),
        repair: RepairState::Complete,
        ..clean_recovery()
    };
    repaired.validate().unwrap();

    let refused = DurableRecoveryStatus {
        outcome: RecoveryOutcome::Refused,
        phase: RecoveryPhase::Validate,
        reason: Some(RecoveryReasonCode::ChecksumMismatch),
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
        ..clean_recovery()
    };
    refused.validate().unwrap();

    for mutation in [
        |value: &mut DurableRecoveryStatus| value.phase = RecoveryPhase::Validate,
        |value: &mut DurableRecoveryStatus| value.repair = RepairState::Failed,
        |value: &mut DurableRecoveryStatus| value.records_discarded = BoundedCount::exact(1),
        |value: &mut DurableRecoveryStatus| value.corrupt_records = BoundedCount::exact(1),
        |value: &mut DurableRecoveryStatus| value.records_checked.truncated = true,
        |value: &mut DurableRecoveryStatus| value.completeness = ManagementCompleteness::Partial,
        |value: &mut DurableRecoveryStatus| value.source = ManagementObservationSource::Modeled,
    ] {
        let mut invalid = clean_recovery();
        mutation(&mut invalid);
        assert_incoherent(invalid.validate(), "outcome");
    }

    let mut repaired_without_defect = clean_recovery();
    repaired_without_defect.outcome = RecoveryOutcome::Repaired;
    repaired_without_defect.repair = RepairState::Complete;
    repaired_without_defect.records_recovered = BoundedCount::exact(0);
    assert_incoherent(repaired_without_defect.validate(), "outcome");

    let mut refused_without_reason = clean_recovery();
    refused_without_reason.outcome = RecoveryOutcome::Refused;
    assert_incoherent(refused_without_reason.validate(), "reason");
}

#[test]
fn recovery_reason_vocabulary_and_unknown_versions_fail_safe() {
    let reasons = [
        RecoveryReasonCode::ArtifactMissing,
        RecoveryReasonCode::UnsupportedFormat,
        RecoveryReasonCode::ChecksumMismatch,
        RecoveryReasonCode::TruncatedArtifact,
        RecoveryReasonCode::ForeignIdentity,
        RecoveryReasonCode::StaleWatermark,
        RecoveryReasonCode::Timeout,
        RecoveryReasonCode::IoError,
        RecoveryReasonCode::DiskFull,
        RecoveryReasonCode::RepairSourceUnavailable,
        RecoveryReasonCode::RepairFailed,
        RecoveryReasonCode::ReconciliationFailed,
        RecoveryReasonCode::StatusNotRetained,
        RecoveryReasonCode::Unknown,
    ];
    let encoded = serde_json::to_value(reasons).unwrap();
    assert_eq!(encoded[0], "artifact-missing");
    assert_eq!(encoded[12], "status-not-retained");

    let reason: RecoveryReasonCode = serde_json::from_str("\"future-recovery-reason\"").unwrap();
    assert_eq!(reason, RecoveryReasonCode::Unknown);

    let mut future_schema = clean_recovery();
    future_schema.schema_version += 1;
    assert!(matches!(
        future_schema.validate(),
        Err(ManagementContractError::UnsupportedSchema { .. })
    ));
}

#[test]
fn consensus_progress_reports_lag_only_for_complete_coherent_positions() {
    let valid = progress();
    valid.validate().unwrap();
    assert_eq!(valid.apply_lag(), Some(2));

    let mut partial = valid.clone();
    partial.completeness = ManagementCompleteness::Partial;
    assert_eq!(partial.apply_lag(), None);

    let mut missing = valid.clone();
    missing.applied_index = None;
    assert_eq!(missing.apply_lag(), None);

    let mut apply_ahead = valid.clone();
    apply_ahead.applied_index = Some(32);
    assert_incoherent(apply_ahead.validate(), "applied_index");
    assert_eq!(apply_ahead.apply_lag(), None);

    let mut snapshot_ahead = valid.clone();
    snapshot_ahead.last_snapshot_index = Some(30);
    snapshot_ahead.applied_index = Some(29);
    assert_incoherent(snapshot_ahead.validate(), "last_snapshot_index");

    let mut target_behind = valid;
    target_behind.catch_up_target = Some(28);
    assert_incoherent(target_behind.validate(), "catch_up_target");
}

#[test]
fn contract_errors_are_stable_and_actionable() {
    let schema = ManagementContractError::UnsupportedSchema {
        found: 2,
        expected: 1,
    };
    let limit = ManagementContractError::LimitExceeded {
        field: "candidates",
        limit: 1,
        actual: 2,
    };
    let order = ManagementContractError::UnstableOrder { field: "selected" };

    assert_eq!(
        schema.to_string(),
        "unsupported management schema version 2; expected 1"
    );
    assert!(limit.to_string().contains("maximum 1, got 2"));
    assert!(order.to_string().contains("strictly ordered"));
}

fn assert_incoherent(result: Result<(), ManagementContractError>, field: &'static str) {
    assert!(matches!(
        result,
        Err(ManagementContractError::Incoherent { field: found, .. }) if found == field
    ));
}

fn assert_unstable(result: Result<(), ManagementContractError>, field: &'static str) {
    assert!(matches!(
        result,
        Err(ManagementContractError::UnstableOrder { field: found }) if found == field
    ));
}

fn assert_limit(result: Result<(), ManagementContractError>, field: &'static str, limit: usize) {
    assert!(matches!(
        result,
        Err(ManagementContractError::LimitExceeded {
            field: found,
            limit: found_limit,
            ..
        }) if found == field && found_limit == limit
    ));
}
