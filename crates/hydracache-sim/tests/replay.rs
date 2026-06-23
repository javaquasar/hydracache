use hydracache::LogicalDuration;
use hydracache_sim::{FaultSchedule, ReplayRunner, ScheduledFault, ScheduledFaultKind};

#[test]
fn replay_seed_reproduces_identical_violation() {
    let runner = ReplayRunner;
    let schedule = FaultSchedule::from_faults(vec![
        ScheduledFault::new(
            2,
            ScheduledFaultKind::NetworkDelay {
                from: "a".to_owned(),
                to: "b".to_owned(),
                duration: LogicalDuration::from_millis(5),
            },
        ),
        ScheduledFault::new(
            3,
            ScheduledFaultKind::SyntheticViolation {
                invariant: "consensus-prefix".to_owned(),
            },
        ),
    ]);

    let left = runner.run(44, 10, schedule.clone());
    let right = runner.run(44, 10, schedule);

    assert_eq!(left.failure, right.failure);
    assert_eq!(left.failure.expect("failure").step, 3);
}

#[test]
fn replay_shrinker_preserves_the_violation() {
    let runner = ReplayRunner;
    let schedule = FaultSchedule::from_faults(vec![
        ScheduledFault::new(
            1,
            ScheduledFaultKind::NetworkDrop {
                from: "a".to_owned(),
                to: "b".to_owned(),
            },
        ),
        ScheduledFault::new(
            3,
            ScheduledFaultKind::SyntheticViolation {
                invariant: "read-your-writes".to_owned(),
            },
        ),
        ScheduledFault::new(
            4,
            ScheduledFaultKind::Crash {
                node: "c".to_owned(),
            },
        ),
    ]);

    let shrunk = runner.shrink_failure(45, 10, schedule.clone());

    assert!(shrunk.faults().len() < schedule.faults().len());
    assert!(runner.run(45, 10, shrunk.clone()).failure.is_some());
    assert_eq!(
        shrunk.faults(),
        &[ScheduledFault::new(
            3,
            ScheduledFaultKind::SyntheticViolation {
                invariant: "read-your-writes".to_owned()
            }
        )]
    );
}
