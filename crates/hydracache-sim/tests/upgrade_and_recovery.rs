use hydracache_sim::{run_upgrade_and_recovery, DeploymentFault, DeploymentRecoveryScenario};

#[test]
fn upgrade_and_recovery_faults_are_seeded_and_replayable() {
    let scenario = DeploymentRecoveryScenario::all(48);

    let first = run_upgrade_and_recovery(scenario.clone());
    let second = run_upgrade_and_recovery(scenario);

    assert_eq!(first, second);
    assert_eq!(first.seed, 48);
    assert!(!first.trace.is_empty());
}

#[test]
fn upgrade_and_recovery_invariants_hold_across_upgrade_rotation_restore() {
    let report = run_upgrade_and_recovery(DeploymentRecoveryScenario::all(4_848));

    assert!(report.passed(), "{report:?}");
    assert_eq!(
        report.faults_exercised,
        vec![
            DeploymentFault::RollingUpgrade,
            DeploymentFault::CertRotation,
            DeploymentFault::BackupCorruption,
            DeploymentFault::PitrRestore,
        ]
    );
}

#[test]
fn upgrade_and_recovery_backup_corruption_is_detected_not_served() {
    let report = run_upgrade_and_recovery(DeploymentRecoveryScenario::new(
        7,
        vec![DeploymentFault::BackupCorruption],
    ));

    assert!(report.corrupt_backup_rejected);
    assert!(report.passed());
    assert!(report.trace[0].starts_with("backup-corruption:key="));
}

#[test]
#[ignore = "nightly gate: pair deterministic model with kind rolling update and restore drill"]
fn upgrade_and_recovery_kind_drill_replays_the_same_fault_model() {
    let report = run_upgrade_and_recovery(DeploymentRecoveryScenario::all(48));
    assert!(report.passed());
}
