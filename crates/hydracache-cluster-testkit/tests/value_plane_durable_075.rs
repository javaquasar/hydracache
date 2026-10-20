use hydracache_cluster_testkit::value_plane_durable_075::{
    AcknowledgementClass, DurableError, DurableRecoveryModel, RecoveryFault,
};

#[test]
fn durable_acknowledgements_survive_all_admitted_restart_classes() {
    for fault in [
        RecoveryFault::CleanRestart,
        RecoveryFault::OwnerAndBackupLoss,
        RecoveryFault::WholeClusterRestart,
    ] {
        let mut model = DurableRecoveryModel::new(4).unwrap();
        model.append(1, 11).unwrap();
        model.commit(1).unwrap();
        model.flush(1).unwrap();
        model.acknowledge(1, AcknowledgementClass::Durable).unwrap();
        model.snapshot(1).unwrap();
        let report = model.recover(fault).unwrap();
        assert_eq!(report.recovered_through, 1);
        assert!(report.durable_acknowledgement_preserved);
        assert!(!report.memory_acknowledgement_lost);
        assert_eq!(model.snapshot_watermark(), 1);
    }
}

#[test]
fn memory_ack_is_not_mislabeled_as_durable_after_total_loss() {
    let mut model = DurableRecoveryModel::new(4).unwrap();
    model.append(1, 11).unwrap();
    model.commit(1).unwrap();
    model.acknowledge(1, AcknowledgementClass::Memory).unwrap();
    let report = model.recover(RecoveryFault::OwnerAndBackupLoss).unwrap();
    assert_eq!(report.recovered_through, 0);
    assert!(report.memory_acknowledgement_lost);
}

#[test]
fn truncation_corruption_and_illegal_watermarks_fail_loud() {
    let mut model = DurableRecoveryModel::new(2).unwrap();
    model.append(1, 11).unwrap();
    assert_eq!(model.flush(1), Err(DurableError::FlushBeyondCommit));
    assert_eq!(
        model.acknowledge(1, AcknowledgementClass::Durable),
        Err(DurableError::AcknowledgeBeforeDurable)
    );
    model.commit(1).unwrap();
    model.flush(1).unwrap();
    assert_eq!(model.durable_watermark(), 1);
    assert_eq!(
        model.recover(RecoveryFault::TruncateAfter(0)),
        Err(DurableError::DurableDataTruncated)
    );
    assert_eq!(
        model.recover(RecoveryFault::Corrupt(1)),
        Err(DurableError::DurableDataCorrupted)
    );
    assert_eq!(model.snapshot(2), Err(DurableError::SnapshotBeyondDurable));
}

#[test]
fn log_bounds_and_monotonic_versions_are_enforced() {
    assert!(DurableRecoveryModel::new(0).is_err());
    let mut model = DurableRecoveryModel::new(1).unwrap();
    model.append(1, 11).unwrap();
    assert_eq!(model.append(1, 12), Err(DurableError::Capacity));
}
