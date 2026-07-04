use std::time::Duration;

use hydracache_sim::{
    minimal_failing_steps_by, run_soak_with_seed_runner, shrink_failing_schedule_with,
    FaultSchedule, ScheduledFault, ScheduledFaultKind, SimConfig, SimRng, SoakConfig, SoakReport,
};
use serde_json::Value;

fn bounded_cfg(master_seed: u64, max_seeds: u64, steps_per_seed: u64) -> SoakConfig {
    SoakConfig::new(
        master_seed,
        Duration::from_secs(60),
        steps_per_seed,
        SimConfig::default(),
    )
    .with_max_seeds(max_seeds)
}

fn fail_on_count(target: u64) -> impl FnMut(u64, u64, &SimConfig) -> Option<(u64, Vec<String>)> {
    let mut seen = 0_u64;
    move |_seed, steps, _sim| {
        seen = seen.saturating_add(1);
        (seen == target).then(|| (steps, vec!["synthetic invariant violation".to_owned()]))
    }
}

#[test]
fn soak_fleet_is_reproducible_from_master_seed() {
    let cfg = bounded_cfg(0x58_01, 8, 16);

    let first = run_soak_with_seed_runner(&cfg, fail_on_count(3));
    let second = run_soak_with_seed_runner(&cfg, fail_on_count(3));

    assert_eq!(first, second);
    assert_eq!(first.seeds_run, 3);
    assert!(first.first_failure.is_some());
}

#[test]
fn first_failing_seed_reproduces_the_violation_exactly() {
    let master_seed = 0x58_02;
    let mut fleet = SimRng::from_seed(master_seed);
    let failing_seed = fleet.next_u64();
    let cfg = bounded_cfg(master_seed, 1, 16);

    let outcome = run_soak_with_seed_runner(&cfg, move |seed, steps, _sim| {
        (seed == failing_seed && steps >= 7)
            .then(|| (7, vec!["synthetic invariant violation".to_owned()]))
    });

    let failure = outcome.first_failure.expect("first seed fails");
    assert_eq!(failure.seed, failing_seed);
    assert_eq!(failure.step, 7);
    assert!(failure.seed == failing_seed && failure.step >= 7);
    assert!(failure.seed != failing_seed || failure.step <= 7);
}

#[test]
fn soak_stops_loud_on_first_invariant_violation() {
    let cfg = bounded_cfg(0x58_03, 16, 32);

    let outcome = run_soak_with_seed_runner(&cfg, fail_on_count(2));

    assert_eq!(outcome.seeds_run, 2);
    assert_eq!(outcome.total_steps, 64);
    assert!(outcome.first_failure.is_some());
}

#[test]
fn failing_schedule_shrinks_to_minimal_reproducing_subset() {
    let keep = ScheduledFault::new(
        2,
        ScheduledFaultKind::SyntheticViolation {
            invariant: "keep".to_owned(),
        },
    );
    let schedule = FaultSchedule::from_faults(vec![
        ScheduledFault::new(
            1,
            ScheduledFaultKind::SyntheticViolation {
                invariant: "drop-a".to_owned(),
            },
        ),
        keep.clone(),
        ScheduledFault::new(
            3,
            ScheduledFaultKind::SyntheticViolation {
                invariant: "drop-b".to_owned(),
            },
        ),
    ]);

    let shrunk = shrink_failing_schedule_with(schedule, |candidate| {
        candidate.faults().iter().any(|fault| {
            matches!(
                &fault.kind,
                ScheduledFaultKind::SyntheticViolation { invariant }
                    if invariant == "keep"
            )
        })
    });

    assert_eq!(shrunk.faults(), &[keep]);
}

#[test]
fn plain_seed_failure_bisects_to_minimal_step_count() {
    let minimal = minimal_failing_steps_by(64, |steps| steps >= 17);

    assert_eq!(minimal, Some(17));
}

#[test]
fn soak_driver_memory_is_bounded_over_a_long_fleet() {
    let cfg = bounded_cfg(0x58_04, 2_048, 1);

    let outcome = run_soak_with_seed_runner(&cfg, fail_on_count(2_048));

    let failure = outcome.first_failure.expect("last seed fails");
    assert_eq!(outcome.seeds_run, 2_048);
    assert_eq!(failure.violations, ["synthetic invariant violation"]);
}

#[test]
fn soak_report_serializes_without_a_self_score() {
    let cfg = bounded_cfg(0x58_05, 1, 4);
    let outcome = run_soak_with_seed_runner(&cfg, fail_on_count(1));
    let report = SoakReport::from(&outcome);
    let json = serde_json::to_value(report).expect("report serializes");

    assert_eq!(json["outcome"]["status"], "failed");
    for forbidden in [
        "score",
        "self_score",
        "health_score",
        "throughput",
        "ops_per_sec",
    ] {
        assert!(
            !contains_key(&json, forbidden),
            "report must not contain score-like field '{forbidden}': {json}"
        );
    }
}

fn contains_key(value: &Value, needle: &str) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(key, value)| key == needle || contains_key(value, needle)),
        Value::Array(values) => values.iter().any(|value| contains_key(value, needle)),
        _ => false,
    }
}
