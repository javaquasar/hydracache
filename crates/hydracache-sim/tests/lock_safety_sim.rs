use hydracache_sim::{run_lock_safety, LockSafetyReport, LockSafetyScenario};

fn run_replayed(seed: u64) -> LockSafetyReport {
    let scenario = LockSafetyScenario::all(seed);
    let first = run_lock_safety(scenario.clone());
    let second = run_lock_safety(scenario);
    assert_eq!(
        first, second,
        "lock safety scenario must replay seed {seed}"
    );
    assert_eq!(first.seed, seed);
    first
}

#[test]
fn mutual_exclusion_holds_under_partition_and_leader_change() {
    let report = run_replayed(0x52_07_01);

    assert!(
        report.invariants.is_ok(),
        "seed {} violated invariants: {:?}",
        report.seed,
        report.invariants.violations
    );
    assert!(
        report.partition_blocks > 0,
        "seed {} did not exercise a partition block",
        report.seed
    );
    assert!(
        report.leader_changes > 0,
        "seed {} did not exercise leader change",
        report.seed
    );
    assert_eq!(report.max_live_owners, 1);
}

#[test]
fn session_loss_advances_fence_and_rejects_zombie_writer() {
    let report = run_replayed(0x52_07_02);

    assert!(report.invariants.is_ok(), "seed {}", report.seed);
    assert!(
        report.session_losses > 0,
        "seed {} did not exercise session loss",
        report.seed
    );
    assert!(
        report.zombie_rejections > 0,
        "seed {} did not reject a zombie holder",
        report.seed
    );
    assert!(
        report.acquired_fences.len() >= 2,
        "seed {} did not acquire after session loss",
        report.seed
    );
    assert!(report.acquired_fences[1] > report.acquired_fences[0]);
}

#[test]
fn fence_is_strictly_monotonic_across_ownership_changes() {
    let report = run_replayed(0x52_07_03);

    assert!(report.invariants.is_ok(), "seed {}", report.seed);
    assert!(
        report
            .acquired_fences
            .windows(2)
            .all(|pair| pair[1] > pair[0]),
        "seed {} fences were not strictly monotonic: {:?}",
        report.seed,
        report.acquired_fences
    );
}

#[test]
fn no_lock_acquired_at_weak_consistency_level() {
    let report = run_replayed(0x52_07_04);

    assert!(report.invariants.is_ok(), "seed {}", report.seed);
    assert!(
        report.weak_rejections > 0,
        "seed {} did not exercise weak consistency rejection",
        report.seed
    );
    assert_eq!(report.weak_acquisitions, 0);
}
