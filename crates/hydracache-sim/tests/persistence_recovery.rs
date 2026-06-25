use hydracache_sim::{
    run_persistence_recovery, PersistenceRecoveryFault, PersistenceRecoveryScenario,
};

#[test]
fn persistence_recovery_faults_are_seeded_and_replayable() {
    let scenario = PersistenceRecoveryScenario::all(51);

    let first = run_persistence_recovery(scenario.clone());
    let second = run_persistence_recovery(scenario);

    assert_eq!(first, second);
    assert_eq!(first.seed, 51);
    assert_ne!(first.deterministic_digest, 0);
    assert!(!first.trace.is_empty());
}

#[test]
fn persistence_recovery_invariants_hold_across_restart_and_faults() {
    let report = run_persistence_recovery(PersistenceRecoveryScenario::all(5_151));

    assert!(report.passed(), "{report:?}");
    assert_eq!(
        report.faults_exercised,
        vec![
            PersistenceRecoveryFault::WholeClusterCrashRestart,
            PersistenceRecoveryFault::CrashMidSnapshot,
            PersistenceRecoveryFault::TornDurableWrite,
            PersistenceRecoveryFault::StorageCorruption,
            PersistenceRecoveryFault::StaleEpochOnDisk,
        ]
    );
}

#[test]
fn persistence_recovery_selected_faults_are_exercised() {
    let report = run_persistence_recovery(PersistenceRecoveryScenario::new(
        8,
        vec![
            PersistenceRecoveryFault::TornDurableWrite,
            PersistenceRecoveryFault::StorageCorruption,
            PersistenceRecoveryFault::StaleEpochOnDisk,
        ],
    ));

    assert!(report.passed(), "{report:?}");
    assert!(report.torn_write_refused);
    assert!(report.corrupt_storage_refused);
    assert!(report.stale_records_fenced);
    assert_eq!(
        report.faults_exercised,
        vec![
            PersistenceRecoveryFault::TornDurableWrite,
            PersistenceRecoveryFault::StorageCorruption,
            PersistenceRecoveryFault::StaleEpochOnDisk,
        ]
    );
}

#[test]
#[ignore = "nightly gate: pair persistence recovery model with broader storage fault drill"]
fn persistence_recovery_kind_drill_covers_storage_faults() {
    let report = run_persistence_recovery(PersistenceRecoveryScenario::all(51));
    assert!(report.passed(), "{report:?}");
}
