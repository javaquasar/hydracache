use hydracache_observability::{
    evaluate_management_health, retain_newer_recovery_observation, AdmissionState, BoundedCount,
    BoundedPlacementConstraints, CatchUpState, ClusterFormationSnapshot, ConsensusProgressSnapshot,
    DiscoveryState, DurableRecoveryStatus, FormationReasonCode, ManagementCompleteness,
    ManagementConsensusRole, ManagementHealthInput, ManagementHealthStatus,
    ManagementHealthThresholds, ManagementObservationSource, OpaqueNodeIdentity,
    OperationalHealthSignals, PlacementCandidatePage, PlacementDecisionTrace, PlacementOutcome,
    RecoveryArtifactKind, RecoveryHealthObservation, RecoveryOutcome, RecoveryPhase, RecoveryScope,
    RepairState, ServingState, TransportState, MANAGEMENT_API_SCHEMA_VERSION,
};

fn formation(node: &str) -> ClusterFormationSnapshot {
    ClusterFormationSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node: OpaqueNodeIdentity::new(node),
        generation: 1,
        candidate_observation_seq: Some(9),
        authority_epoch: Some(7),
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

fn consensus(node: &str, lag: u64) -> ConsensusProgressSnapshot {
    let applied = 2_000 - lag;
    ConsensusProgressSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node: OpaqueNodeIdentity::new(node),
        generation: 1,
        authority_epoch: Some(7),
        commit_index: Some(2_000),
        applied_index: Some(applied),
        last_snapshot_index: Some(applied.min(1_000)),
        catch_up_target: Some(2_000),
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
}

fn placement(outcome: PlacementOutcome, truncated: bool) -> PlacementDecisionTrace {
    let applied = matches!(outcome, PlacementOutcome::Applied);
    PlacementDecisionTrace {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        trace_id: "trace-1".into(),
        topology_epoch: 7,
        request_digest: "request-1".into(),
        requested_replicas: 1,
        constraints: BoundedPlacementConstraints::default(),
        candidates: PlacementCandidatePage {
            items: Vec::new(),
            truncated,
        },
        selected: if applied {
            vec![OpaqueNodeIdentity::new("node-a")]
        } else {
            Vec::new()
        },
        tie_break_seed: Some(3),
        outcome,
        commit_index: applied.then_some(2_000),
        applied_index: applied.then_some(2_000),
        reason: None,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
}

fn recovery(outcome: RecoveryOutcome) -> RecoveryHealthObservation {
    let clean = outcome == RecoveryOutcome::Clean;
    RecoveryHealthObservation {
        authority_epoch: Some(7),
        observation_seq: 9,
        status: DurableRecoveryStatus {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            scope: RecoveryScope::Node,
            outcome,
            phase: RecoveryPhase::Complete,
            artifact: RecoveryArtifactKind::DurableValueStore,
            artifact_format_version: Some(1),
            validated_watermark: None,
            records_checked: BoundedCount::exact(20),
            records_recovered: BoundedCount::exact(if clean { 0 } else { 1 }),
            records_discarded: BoundedCount::exact(0),
            corrupt_records: BoundedCount::exact(0),
            repair: if clean {
                RepairState::NotRequired
            } else {
                RepairState::Complete
            },
            reason: None,
            started_at_unix_ms: Some(1),
            completed_at_unix_ms: Some(2),
            source: ManagementObservationSource::Live,
            completeness: ManagementCompleteness::Complete,
        },
    }
}

fn healthy_input() -> ManagementHealthInput {
    ManagementHealthInput {
        authority_epoch: Some(7),
        observation_seq: 9,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
        formation: vec![formation("node-a")],
        consensus: vec![consensus("node-a", 0)],
        placement: Some(placement(PlacementOutcome::Applied, false)),
        recovery: Some(recovery(RecoveryOutcome::Clean)),
        operational: healthy_operational(),
        thresholds: ManagementHealthThresholds::default(),
    }
}

fn healthy_operational() -> OperationalHealthSignals {
    OperationalHealthSignals {
        quorum_ok: Some(true),
        leader_present: Some(true),
        unreachable_members: Some(0),
        incompatible_members: Some(0),
        config_skew_members: Some(0),
        unassigned_partitions: Some(0),
        under_replicated_partitions: Some(0),
        zone_underspread_partitions: Some(0),
        replication_failures: Some(0),
        replication_backpressure: Some(0),
        repair_debt: Some(0),
        degraded_mode: Some(false),
        reshard_active: Some(false),
        reshard_backfill_lag: Some(0),
        reshard_backfill_lag_limit: Some(10),
        retained_bytes: Some(0),
        retained_bytes_limit: Some(10),
        admission_queue_depth: Some(0),
        admission_queue_limit: Some(10),
        expiry_backlog: Some(0),
        expiry_backlog_limit: Some(10),
        active_clients: Some(0),
        active_clients_limit: Some(10),
        buffered_client_bytes: Some(0),
        buffered_client_bytes_limit: Some(10),
        persistence_enabled: Some(true),
        disk_available: Some(true),
        backup_age_seconds: Some(0),
        backup_age_limit_seconds: Some(10),
        audit_enabled: Some(true),
        audit_sink_healthy: Some(true),
        history_enabled: Some(true),
        history_source_available: Some(true),
    }
}

fn status(input: &ManagementHealthInput, id: &str) -> ManagementHealthStatus {
    evaluate_management_health(input)
        .checks
        .into_iter()
        .find(|check| check.id == id)
        .expect("stable check exists")
        .status
}

#[test]
fn healthy_fixture_passes_the_complete_initial_catalogue() {
    let report = evaluate_management_health(&healthy_input());
    assert!(report.validate().is_ok());
    assert_eq!(report.checks.len(), 18);
    assert_eq!(report.counts.pass, 18);
    assert_eq!(report.counts.unknown, 0);
    assert_eq!(report.aggregate, ManagementHealthStatus::Pass);
}

#[test]
fn operational_limit_rules_cover_boundary_minus_one_equal_and_plus_one() {
    for (id, selector) in [
        ("HC-RESHARD-001", 0_u8),
        ("HC-MEM-001", 1),
        ("HC-EXPIRY-001", 2),
        ("HC-CLIENT-001", 3),
        ("HC-PERSIST-001", 4),
    ] {
        for (value, expected) in [
            (9, ManagementHealthStatus::Pass),
            (10, ManagementHealthStatus::Fail),
            (11, ManagementHealthStatus::Fail),
        ] {
            let mut input = healthy_input();
            match selector {
                0 => {
                    input.operational.reshard_active = Some(true);
                    input.operational.reshard_backfill_lag = Some(value);
                }
                1 => input.operational.retained_bytes = Some(value),
                2 => input.operational.expiry_backlog = Some(value),
                3 => input.operational.active_clients = Some(value),
                _ => input.operational.backup_age_seconds = Some(value),
            }
            assert_eq!(status(&input, id), expected, "{id} value={value}");
        }
    }
}

#[test]
fn explicit_optional_disable_is_distinct_from_missing_source() {
    let mut input = healthy_input();
    input.operational.history_enabled = Some(false);
    assert_eq!(
        status(&input, "HC-HISTORY-001"),
        ManagementHealthStatus::Disabled
    );
    input.operational.history_enabled = None;
    assert_eq!(
        status(&input, "HC-HISTORY-001"),
        ManagementHealthStatus::Unknown
    );
}

#[test]
fn operational_adverse_signal_matrix_has_reviewed_severity() {
    let cases = [
        ("HC-AUTH-001", 0_u8, ManagementHealthStatus::Fail),
        ("HC-MEMBER-001", 1, ManagementHealthStatus::Fail),
        ("HC-PART-001", 2, ManagementHealthStatus::Fail),
        ("HC-REPL-001", 3, ManagementHealthStatus::Fail),
        ("HC-REPAIR-001", 4, ManagementHealthStatus::Fail),
        ("HC-AUDIT-001", 5, ManagementHealthStatus::Fail),
    ];
    for (id, selector, expected) in cases {
        let mut input = healthy_input();
        match selector {
            0 => input.operational.quorum_ok = Some(false),
            1 => input.operational.incompatible_members = Some(1),
            2 => input.operational.unassigned_partitions = Some(1),
            3 => input.operational.replication_failures = Some(1),
            4 => input.operational.degraded_mode = Some(true),
            _ => input.operational.audit_sink_healthy = Some(false),
        }
        assert_eq!(status(&input, id), expected, "{id}");
    }
}

#[test]
fn unavailable_operational_sources_are_all_unknown() {
    let mut input = healthy_input();
    input.operational = OperationalHealthSignals::default();
    let report = evaluate_management_health(&input);
    assert_eq!(report.counts.pass, 6);
    assert_eq!(report.counts.unknown, 12);
    assert_eq!(report.aggregate, ManagementHealthStatus::Pass);
}

#[test]
fn raft_thresholds_cover_boundary_minus_one_equal_and_plus_one() {
    let cases = [
        (99, ManagementHealthStatus::Pass),
        (100, ManagementHealthStatus::Warn),
        (101, ManagementHealthStatus::Warn),
        (999, ManagementHealthStatus::Warn),
        (1_000, ManagementHealthStatus::Fail),
        (1_001, ManagementHealthStatus::Fail),
    ];
    for (lag, expected) in cases {
        let mut input = healthy_input();
        input.consensus = vec![consensus("node-a", lag)];
        assert_eq!(status(&input, "HC-RAFT-001"), expected, "lag={lag}");
    }
}

#[test]
fn missing_modeled_and_partial_inputs_are_unknown_not_pass() {
    for mutate in 0..3 {
        let mut input = healthy_input();
        match mutate {
            0 => input.consensus.clear(),
            1 => input.source = ManagementObservationSource::Modeled,
            _ => input.completeness = ManagementCompleteness::Partial,
        }
        let report = evaluate_management_health(&input);
        assert!(report.counts.unknown > 0);
        if mutate != 0 {
            assert_eq!(report.counts.pass, 0);
        }
    }
}

#[test]
fn discovered_or_reachable_does_not_imply_formation_pass() {
    let mut input = healthy_input();
    let item = &mut input.formation[0];
    item.transport = TransportState::Reachable;
    item.admission = AdmissionState::Pending;
    item.consensus_role = ManagementConsensusRole::None;
    item.catch_up = CatchUpState::Behind;
    item.serving = ServingState::Blocked;
    item.blocker = Some(FormationReasonCode::NotAdmitted);
    assert_eq!(status(&input, "HC-FORM-001"), ManagementHealthStatus::Warn);
}

#[test]
fn rejected_identity_or_authentication_is_formation_fail() {
    for blocker in [
        FormationReasonCode::ClusterIdentityMismatch,
        FormationReasonCode::PeerUnauthenticated,
    ] {
        let mut input = healthy_input();
        let item = &mut input.formation[0];
        item.admission = AdmissionState::Rejected;
        item.consensus_role = ManagementConsensusRole::None;
        item.catch_up = CatchUpState::Behind;
        item.serving = ServingState::Blocked;
        item.blocker = Some(blocker);
        assert_eq!(status(&input, "HC-FORM-001"), ManagementHealthStatus::Fail);
    }
}

#[test]
fn truncated_unknown_placement_cannot_pass() {
    let mut input = healthy_input();
    input.placement = Some(placement(PlacementOutcome::Unknown, true));
    assert_eq!(
        status(&input, "HC-PLACE-001"),
        ManagementHealthStatus::Unknown
    );
}

#[test]
fn repaired_recovery_warns_and_verified_corruption_fails() {
    let mut input = healthy_input();
    input.recovery = Some(recovery(RecoveryOutcome::Repaired));
    assert_eq!(
        status(&input, "HC-RECOVERY-001"),
        ManagementHealthStatus::Warn
    );

    input.recovery.as_mut().unwrap().status.corrupt_records = BoundedCount::exact(2);
    assert_eq!(
        status(&input, "HC-RECOVERY-002"),
        ManagementHealthStatus::Fail
    );
}

#[test]
fn truncated_recovery_counts_are_unknown_not_clean() {
    let mut input = healthy_input();
    input
        .recovery
        .as_mut()
        .unwrap()
        .status
        .records_checked
        .truncated = true;
    assert_eq!(
        status(&input, "HC-RECOVERY-002"),
        ManagementHealthStatus::Unknown
    );
}

#[test]
fn input_order_does_not_change_normalized_json() {
    let mut left = healthy_input();
    left.formation.push(formation("node-b"));
    left.consensus.push(consensus("node-b", 3));
    let mut right = left.clone();
    right.formation.reverse();
    right.consensus.reverse();
    assert_eq!(
        serde_json::to_vec(&evaluate_management_health(&left)).unwrap(),
        serde_json::to_vec(&evaluate_management_health(&right)).unwrap()
    );
}

#[test]
fn unknown_count_survives_a_known_failure() {
    let mut input = healthy_input();
    input.consensus = vec![consensus("node-a", 1_001)];
    input.placement = None;
    let report = evaluate_management_health(&input);
    assert_eq!(report.aggregate, ManagementHealthStatus::Fail);
    assert!(report.counts.fail >= 1);
    assert!(report.counts.unknown >= 1);
}

#[test]
fn every_stable_check_has_documentation_truth_browser_canary_and_evidence_rows() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registry =
        std::fs::read_to_string(root.join("docs/testing/management-center/0.72/healthchecks.toml"))
            .unwrap();
    let operations =
        std::fs::read_to_string(root.join("docs/operations/management-healthchecks.md")).unwrap();
    let browser = std::fs::read_to_string(root.join("console/tests/fixtures.js")).unwrap();
    let canaries =
        std::fs::read_to_string(root.join("docs/testing/canaries/0.72-management-center.toml"))
            .unwrap();
    let manifest =
        std::fs::read_to_string(root.join("docs/testing/release-evidence/0.72/manifest.toml"))
            .unwrap();
    let report = evaluate_management_health(&healthy_input());
    assert_eq!(registry.matches("[[check]]").count(), report.checks.len());
    for check in report.checks {
        assert!(registry.contains(&format!("id = \"{}\"", check.id)));
        assert!(registry.contains(check.remediation_code));
        assert!(operations.contains(check.id));
        assert!(browser.contains(check.id));
    }
    for canary in [
        "MC72-W8-UNKNOWN-AS-PASS",
        "MC72-W8-STALE-RECOVERY-WINS",
        "MC72-W8-TRUNCATED-PLACEMENT-PASS",
    ] {
        assert!(registry.contains(canary));
        assert!(canaries.contains(canary));
    }
    assert!(manifest.contains("W8-health-engine"));
}

#[test]
fn canary_missing_required_input_is_never_promoted_to_pass() {
    let mut input = healthy_input();
    input.placement = None;
    let mut observed = status(&input, "HC-PLACE-001");
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W8-UNKNOWN-AS-PASS") {
        observed = ManagementHealthStatus::Pass;
    }
    assert_eq!(
        observed,
        ManagementHealthStatus::Unknown,
        "HC-CANARY-RED:MC72-W8-UNKNOWN-AS-PASS missing health evidence was painted green"
    );
}

#[test]
fn canary_stale_clean_recovery_cannot_replace_newer_degraded() {
    let mut current = recovery(RecoveryOutcome::Degraded);
    current.observation_seq = 10;
    current.status.phase = RecoveryPhase::Complete;
    current.status.repair = RepairState::Failed;
    let stale_clean = recovery(RecoveryOutcome::Clean);
    let mut selected = retain_newer_recovery_observation(Some(&current), stale_clean, Some(7))
        .expect("current recovery retained");
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W8-STALE-RECOVERY-WINS") {
        selected = recovery(RecoveryOutcome::Clean);
    }
    assert_eq!(
        selected.status.outcome,
        RecoveryOutcome::Degraded,
        "HC-CANARY-RED:MC72-W8-STALE-RECOVERY-WINS stale clean recovery replaced a newer failure"
    );
}

#[test]
fn canary_truncated_unknown_placement_cannot_pass() {
    let mut input = healthy_input();
    input.placement = Some(placement(PlacementOutcome::Unknown, true));
    let mut observed = status(&input, "HC-PLACE-001");
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref()
        == Ok("MC72-W8-TRUNCATED-PLACEMENT-PASS")
    {
        observed = ManagementHealthStatus::Pass;
    }
    assert_eq!(
        observed,
        ManagementHealthStatus::Unknown,
        "HC-CANARY-RED:MC72-W8-TRUNCATED-PLACEMENT-PASS truncated unknown placement was painted green"
    );
}
