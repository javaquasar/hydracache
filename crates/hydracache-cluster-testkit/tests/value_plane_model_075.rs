use hydracache_cluster_testkit::value_plane_history_075::{
    HistoryCall, HistoryOperation, HistoryOutcome, HistoryResult, ValuePlaneHistory,
    ValuePlaneHistoryOracle,
};
use hydracache_cluster_testkit::value_plane_model_075::{
    authority_invariant_violations, AuthorityModelBounds, AuthorityModelExplorer,
    AuthorityModelSnapshot, CanonicalMapKey, DeterministicFaultSchedule, FaultCheckpoint,
    FaultKind, LogicalTime, MutationDigest, MutationIdentity, MutationOperation, MutationPlan,
    MutationStage, OutcomeCertainty, ReferenceValuePlane, TtlDirective, ValuePlaneBounds,
    ValuePlaneError, AUTHORITY_INVARIANT_IDS,
};

fn key(name: &[u8]) -> CanonicalMapKey {
    CanonicalMapKey::new("tenant-a", "orders", 7, name.to_vec(), 3)
}

fn identity(sequence: u64) -> MutationIdentity {
    MutationIdentity::new("client-a", sequence)
}

fn put_plan(sequence: u64, digest: u64, value: &[u8], epoch: u64) -> MutationPlan {
    MutationPlan::new(
        identity(sequence),
        MutationDigest::new(digest),
        key(b"key"),
        MutationOperation::Put {
            value: value.to_vec(),
            ttl: TtlDirective::Eternal,
        },
        epoch,
    )
}

fn applied_through_replica(model: &mut ReferenceValuePlane, plan: MutationPlan) {
    let id = plan.identity.clone();
    model.receive(plan).unwrap();
    model.advance(&id, MutationStage::Admitted).unwrap();
    model.advance(&id, MutationStage::Routed).unwrap();
    model.advance(&id, MutationStage::OwnerDecided).unwrap();
    model.apply_owner(&id).unwrap();
    model.mark_visible_owner(&id).unwrap();
    model.prove_replica(&id, "backup-b").unwrap();
}

#[test]
fn canonical_mutation_reaches_ack_only_after_replica_proof() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 11).unwrap();
    let plan = put_plan(1, 101, b"value", 11);
    let id = plan.identity.clone();
    applied_through_replica(&mut model, plan);
    model.acknowledge(&id).unwrap();
    let outcome = model.respond(&id).unwrap();

    assert_eq!(outcome.certainty, OutcomeCertainty::Certain);
    assert!(outcome.applied);
    assert_eq!(model.live_value(&key(b"key")), Some(b"value".as_slice()));
    assert_eq!(model.mutation(&id).unwrap().stage, MutationStage::Responded);
}

#[test]
fn illegal_stage_regression_negative_canary() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 11).unwrap();
    let plan = put_plan(1, 101, b"value", 11);
    let id = plan.identity.clone();
    model.receive(plan).unwrap();
    model.advance(&id, MutationStage::Admitted).unwrap();

    assert!(matches!(
        model.advance(&id, MutationStage::Received),
        Err(ValuePlaneError::IllegalStageTransition { .. })
    ));
}

#[test]
fn acknowledgement_without_replica_proof_negative_canary() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 11).unwrap();
    let plan = put_plan(1, 101, b"value", 11);
    let id = plan.identity.clone();
    model.receive(plan).unwrap();

    assert!(matches!(
        model.acknowledge(&id),
        Err(ValuePlaneError::ReplicaProofRequired)
    ));
}

#[test]
fn stale_epoch_is_rejected_before_owner_apply_negative_canary() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 12).unwrap();
    let plan = put_plan(1, 101, b"value", 11);
    let id = plan.identity.clone();
    model.receive(plan).unwrap();
    model.advance(&id, MutationStage::Admitted).unwrap();
    model.advance(&id, MutationStage::Routed).unwrap();
    model.advance(&id, MutationStage::OwnerDecided).unwrap();

    assert!(matches!(
        model.apply_owner(&id),
        Err(ValuePlaneError::StaleEpoch {
            expected: 11,
            actual: 12
        })
    ));
    assert_eq!(model.live_value(&key(b"key")), None);
}

#[test]
fn same_identity_with_different_digest_is_terminal_negative_canary() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 11).unwrap();
    model.receive(put_plan(1, 101, b"one", 11)).unwrap();

    assert!(matches!(
        model.receive(put_plan(1, 102, b"two", 11)),
        Err(ValuePlaneError::MutationIdentityConflict)
    ));
}

#[test]
fn request_history_dedup_and_bulk_bounds_fail_closed_negative_canary() {
    let bounds = ValuePlaneBounds {
        max_key_bytes: 3,
        max_value_bytes: 3,
        max_bulk_items: 1,
        max_history_events: 2,
        max_dedup_entries: 1,
        max_fault_steps: 2,
        max_linearizability_operations: 8,
        max_linearizability_search_states: 128,
    };
    let mut model = ReferenceValuePlane::new(bounds, 1).unwrap();

    assert!(matches!(
        model.receive(put_plan(1, 1, b"four", 1)),
        Err(ValuePlaneError::BoundExceeded {
            bound: "value_bytes",
            ..
        })
    ));
    assert!(matches!(
        model.validate_bulk(&[put_plan(1, 1, b"a", 1), put_plan(2, 2, b"b", 1)]),
        Err(ValuePlaneError::BoundExceeded {
            bound: "bulk_items",
            ..
        })
    ));
    model.receive(put_plan(1, 1, b"a", 1)).unwrap();
    assert!(matches!(
        model.receive(put_plan(2, 2, b"b", 1)),
        Err(ValuePlaneError::BoundExceeded {
            bound: "dedup_entries",
            ..
        })
    ));
    let id = identity(1);
    model.advance(&id, MutationStage::Admitted).unwrap();
    assert!(matches!(
        model.advance(&id, MutationStage::Routed),
        Err(ValuePlaneError::BoundExceeded {
            bound: "history_events",
            ..
        })
    ));
}

#[test]
fn response_loss_after_ack_is_explicitly_outcome_unknown() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 11).unwrap();
    let plan = put_plan(1, 101, b"value", 11);
    let id = plan.identity.clone();
    applied_through_replica(&mut model, plan);
    model.acknowledge(&id).unwrap();

    let outcome = model.lose_response(&id).unwrap();
    assert_eq!(outcome.certainty, OutcomeCertainty::OutcomeUnknown);
    assert_eq!(
        model.mutation(&id).unwrap().stage,
        MutationStage::OutcomeUnknown
    );
    assert_eq!(model.live_value(&key(b"key")), Some(b"value".as_slice()));
}

#[test]
fn deterministic_fault_schedules_cover_required_failure_classes() {
    let schedule = DeterministicFaultSchedule::new(
        0x75,
        vec![
            FaultCheckpoint::new(MutationStage::OwnerDecided, FaultKind::OwnerLoss),
            FaultCheckpoint::new(MutationStage::VisibleOwner, FaultKind::BackupLoss),
            FaultCheckpoint::new(
                MutationStage::ReplicaProved,
                FaultKind::Promotion { epoch: 12 },
            ),
            FaultCheckpoint::new(MutationStage::Acknowledged, FaultKind::ResponseLoss),
            FaultCheckpoint::new(MutationStage::OutcomeUnknown, FaultKind::Reconnect),
            FaultCheckpoint::new(
                MutationStage::OutcomeUnknown,
                FaultKind::Rebalance { epoch: 13 },
            ),
        ],
    )
    .unwrap();

    assert_eq!(schedule.seed(), 0x75);
    assert_eq!(schedule.replay_fingerprint(), schedule.replay_fingerprint());
    assert!(schedule.includes(FaultKind::OWNER_LOSS_CLASS));
    assert!(schedule.includes(FaultKind::BACKUP_LOSS_CLASS));
    assert!(schedule.includes(FaultKind::PROMOTION_CLASS));
    assert!(schedule.includes(FaultKind::REBALANCE_CLASS));
    assert!(schedule.includes(FaultKind::RECONNECT_CLASS));
    assert!(schedule.includes(FaultKind::RESPONSE_LOSS_CLASS));
}

#[test]
fn promotion_rebalance_and_reconnect_are_epoch_fenced_and_replay_safe() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 11).unwrap();
    let plan = put_plan(1, 101, b"value", 11);
    let id = plan.identity.clone();
    applied_through_replica(&mut model, plan.clone());
    model.acknowledge(&id).unwrap();
    let original = model.respond(&id).unwrap();

    model.promote(12, "backup-b").unwrap();
    model.rebalance(13, "owner-c").unwrap();
    model.reconnect();
    let replay = model.receive(plan).unwrap();

    assert_eq!(replay.unwrap().outcome.unwrap(), original);
    assert_eq!(model.epoch(), 13);
    assert_eq!(model.connection_generation(), 2);
}

#[test]
fn logical_ttl_never_depends_on_wall_clock() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 1).unwrap();
    model.set_logical_time(LogicalTime::new(10));
    let plan = MutationPlan::new(
        identity(1),
        MutationDigest::new(1),
        key(b"key"),
        MutationOperation::Put {
            value: b"value".to_vec(),
            ttl: TtlDirective::ExpireAfter(5),
        },
        1,
    );
    applied_through_replica(&mut model, plan);
    assert_eq!(model.live_value(&key(b"key")), Some(b"value".as_slice()));
    model.advance_logical_time(5);
    assert_eq!(model.live_value(&key(b"key")), None);
}

#[test]
fn conditional_remove_matches_bytes_and_expiry_never_resurrects() {
    let mut model = ReferenceValuePlane::new(ValuePlaneBounds::default(), 1).unwrap();
    model.set_logical_time(LogicalTime::new(10));
    applied_through_replica(
        &mut model,
        MutationPlan::new(
            identity(1),
            MutationDigest::new(1),
            key(b"key"),
            MutationOperation::Put {
                value: b"value".to_vec(),
                ttl: TtlDirective::ExpireAfter(2),
            },
            1,
        ),
    );
    let mismatch = MutationPlan::new(
        identity(2),
        MutationDigest::new(2),
        key(b"key"),
        MutationOperation::RemoveIfValue {
            expected: b"other".to_vec(),
        },
        1,
    );
    let mismatch_id = mismatch.identity.clone();
    applied_through_replica(&mut model, mismatch);
    assert!(
        !model
            .mutation(&mismatch_id)
            .unwrap()
            .outcome
            .as_ref()
            .unwrap()
            .applied
    );
    assert_eq!(model.live_value(&key(b"key")), Some(b"value".as_slice()));

    model.advance_logical_time(2);
    assert_eq!(model.live_value(&key(b"key")), None);
    let expired_remove = MutationPlan::new(
        identity(3),
        MutationDigest::new(3),
        key(b"key"),
        MutationOperation::RemoveIfValue {
            expected: b"value".to_vec(),
        },
        1,
    );
    let expired_id = expired_remove.identity.clone();
    applied_through_replica(&mut model, expired_remove);
    assert!(
        !model
            .mutation(&expired_id)
            .unwrap()
            .outcome
            .as_ref()
            .unwrap()
            .applied
    );
    assert_eq!(model.live_value(&key(b"key")), None);
}

#[test]
fn history_oracle_accepts_atomic_conditionals_and_ambiguous_response_loss() {
    let mut history = ValuePlaneHistory::new(8).unwrap();
    history
        .push(HistoryCall::completed(
            1,
            key(b"key"),
            HistoryOperation::PutIfAbsent(b"one".to_vec()),
            1,
            4,
            HistoryResult::Mutation(HistoryOutcome::applied(None, Some(b"one".to_vec()))),
            OutcomeCertainty::Certain,
        ))
        .unwrap();
    history
        .push(HistoryCall::completed(
            2,
            key(b"key"),
            HistoryOperation::GetAndPut(b"two".to_vec()),
            2,
            5,
            HistoryResult::Unknown,
            OutcomeCertainty::OutcomeUnknown,
        ))
        .unwrap();
    history
        .push(HistoryCall::completed(
            3,
            key(b"key"),
            HistoryOperation::Read,
            6,
            7,
            HistoryResult::Value(Some(b"two".to_vec())),
            OutcomeCertainty::Certain,
        ))
        .unwrap();

    let report = ValuePlaneHistoryOracle::new(1_000).check(&history);
    assert!(report.is_linearizable(), "{:?}", report.violation);
}

#[test]
fn unlinearizable_history_is_rejected_negative_canary() {
    let mut history = ValuePlaneHistory::new(4).unwrap();
    history
        .push(HistoryCall::completed(
            1,
            key(b"key"),
            HistoryOperation::Put(b"one".to_vec()),
            1,
            2,
            HistoryResult::Mutation(HistoryOutcome::applied(None, Some(b"one".to_vec()))),
            OutcomeCertainty::Certain,
        ))
        .unwrap();
    history
        .push(HistoryCall::completed(
            2,
            key(b"key"),
            HistoryOperation::Read,
            3,
            4,
            HistoryResult::Value(None),
            OutcomeCertainty::Certain,
        ))
        .unwrap();

    let report = ValuePlaneHistoryOracle::new(128).check(&history);
    assert!(!report.is_linearizable());
    assert!(report.violation.unwrap().contains("no legal linearization"));
}

#[test]
fn bounded_authority_model_explores_without_invariant_failure() {
    let report = AuthorityModelExplorer::new(AuthorityModelBounds {
        max_depth: 8,
        max_states: 20_000,
    })
    .unwrap()
    .explore();

    assert!(report.passed(), "{report:#?}");
    assert!(report.explored_states > 100);
    assert!(report.explored_transitions >= report.explored_states);
    assert_eq!(report.max_depth_reached, 8);
}

#[test]
fn authority_model_canaries_detect_false_ack_and_early_promotion() {
    let false_ack = AuthorityModelSnapshot {
        applied_version: 4,
        replica_proved_version: 3,
        acknowledged_version: 4,
        served_version: 4,
        live_version: 4,
        ..AuthorityModelSnapshot::default()
    };
    let violations = authority_invariant_violations(&false_ack);
    assert!(violations
        .iter()
        .any(|(id, _)| *id == AUTHORITY_INVARIANT_IDS[0]));

    let early_promotion = AuthorityModelSnapshot {
        epoch: 2,
        applied_version: 7,
        replica_proved_version: 6,
        acknowledged_version: 7,
        served_version: 6,
        live_version: 6,
        owner_alive: true,
        serving_owner: true,
        ..AuthorityModelSnapshot::default()
    };
    let violations = authority_invariant_violations(&early_promotion);
    assert!(violations
        .iter()
        .any(|(id, _)| *id == AUTHORITY_INVARIANT_IDS[3]));
}

#[test]
fn authority_model_rejects_unbounded_exploration() {
    assert!(matches!(
        AuthorityModelExplorer::new(AuthorityModelBounds {
            max_depth: 0,
            max_states: 1,
        }),
        Err(ValuePlaneError::InvalidBound("authority_model_depth"))
    ));
}
