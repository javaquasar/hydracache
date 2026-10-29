use hydracache_long_run_supervisor_074::watchdog::{
    ProgressHealth, ProgressWatchdog, WatchdogError, PROCESS_CPU_TIME_NS,
};
use hydracache_long_run_supervisor_074::{CheckpointPayload, Phase, ProcessIdentity, Role};
use std::collections::BTreeMap;

fn process(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 100,
        process_group: 10,
        cgroup_path: "/hc/a".to_owned(),
        cgroup_inode: 30,
        unit_name: "hc-a.service".to_owned(),
    }
}

fn sample(phase: Phase) -> CheckpointPayload {
    CheckpointPayload {
        campaign_id: "a".repeat(64),
        role: Role::I74,
        phase,
        phase_epoch: 1,
        monotonic_elapsed_ns: 1_000,
        wall_clock_utc: "2026-10-04T00:00:00Z".to_owned(),
        observed_unix_seconds: 1_000,
        useful_progress_unix_seconds: 1_000,
        completed: 10,
        failed: 0,
        rejected: 0,
        timed_out: 0,
        outstanding: 10,
        telemetry_sequence: 1,
        milestone: "steady".to_owned(),
        surface_counters: BTreeMap::from([("resp".to_owned(), 10)]),
        resource_counters: BTreeMap::from([
            (PROCESS_CPU_TIME_NS.to_owned(), 100),
            ("rss_bytes".to_owned(), 1_000),
        ]),
        owner_counters: BTreeMap::from([("owned".to_owned(), 1)]),
        harness: process(10),
        daemon: process(11),
    }
}

#[test]
fn measured_heartbeat_does_not_reset_the_useful_progress_deadline() {
    let initial = sample(Phase::Measured);
    let mut watchdog = ProgressWatchdog::new(1, initial.clone(), 1_000, 90, 180).unwrap();
    let observation = watchdog.observe(2, initial, 1_091).unwrap();
    assert!(!observation.useful_progress);
    assert_eq!(observation.health, ProgressHealth::Warning);
    assert_eq!(watchdog.health_at(1_181), ProgressHealth::Rejected);
    assert_eq!(watchdog.last_useful_progress_unix_seconds(), 1_000);
}

#[test]
fn measured_work_requires_operations_surface_accounting_and_cpu_to_advance() {
    let initial = sample(Phase::Measured);
    let mut current = initial.clone();
    current.completed += 1;
    *current.surface_counters.get_mut("resp").unwrap() += 1;
    *current
        .resource_counters
        .get_mut(PROCESS_CPU_TIME_NS)
        .unwrap() += 10;
    *current.resource_counters.get_mut("rss_bytes").unwrap() -= 100;
    let mut watchdog = ProgressWatchdog::new(1, initial, 1_000, 90, 180).unwrap();
    let observation = watchdog.observe(2, current, 1_170).unwrap();
    assert!(observation.useful_progress);
    assert_eq!(observation.health, ProgressHealth::Healthy);
    assert_eq!(observation.last_useful_progress_unix_seconds, 1_170);
}

#[test]
fn reconciliation_accepts_owner_convergence_in_either_direction() {
    let initial = sample(Phase::Reconciliation);
    let mut current = initial.clone();
    *current.owner_counters.get_mut("owned").unwrap() = 0;
    let mut watchdog = ProgressWatchdog::new(1, initial, 1_000, 90, 180).unwrap();
    assert!(watchdog.observe(2, current, 1_030).unwrap().useful_progress);
}

#[test]
fn drain_and_intentional_idle_use_distinct_progress_signals() {
    let drain = sample(Phase::Drain);
    let mut drained = drain.clone();
    drained.outstanding -= 1;
    let mut watchdog = ProgressWatchdog::new(1, drain, 1_000, 90, 180).unwrap();
    assert!(watchdog.observe(2, drained, 1_030).unwrap().useful_progress);

    let idle = sample(Phase::PostWorkIdle);
    let mut telemetry = idle.clone();
    telemetry.telemetry_sequence += 1;
    let mut watchdog = ProgressWatchdog::new(1, idle.clone(), 1_000, 90, 180).unwrap();
    assert!(
        watchdog
            .observe(2, telemetry, 1_030)
            .unwrap()
            .useful_progress
    );

    let mut invalid = idle.clone();
    invalid.completed += 1;
    let mut watchdog = ProgressWatchdog::new(1, idle, 1_000, 90, 180).unwrap();
    assert_eq!(
        watchdog.observe(2, invalid, 1_030),
        Err(WatchdogError::IdleOperationDrift)
    );
}

#[test]
fn identity_phase_sequence_and_counter_drift_fail_closed() {
    let initial = sample(Phase::Warmup);
    let mut watchdog = ProgressWatchdog::new(1, initial.clone(), 1_000, 90, 180).unwrap();
    assert_eq!(
        watchdog.observe(3, initial.clone(), 1_030),
        Err(WatchdogError::Sequence)
    );

    let mut identity = initial.clone();
    identity.daemon.start_ticks += 1;
    assert_eq!(
        watchdog.observe(2, identity, 1_030),
        Err(WatchdogError::IdentityDrift)
    );

    let mut skipped = initial.clone();
    skipped.phase = Phase::Drain;
    assert_eq!(
        watchdog.observe(2, skipped, 1_030),
        Err(WatchdogError::PhaseTransition)
    );

    let mut regressed = initial;
    regressed.completed -= 1;
    assert_eq!(
        watchdog.observe(2, regressed, 1_030),
        Err(WatchdogError::CounterRegression)
    );
}
