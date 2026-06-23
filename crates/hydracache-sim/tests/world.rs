use hydracache::LogicalDuration;
use hydracache_sim::{SimConfig, SimWorld};

#[test]
fn world_run_is_reproducible_from_seed() {
    let cfg = SimConfig {
        node_count: 4,
        heartbeat_interval: LogicalDuration::from_millis(1),
        step_duration: LogicalDuration::from_millis(1),
        key_count: 8,
    };
    let mut left = SimWorld::new(44, cfg.clone());
    let mut right = SimWorld::new(44, cfg);

    let left_outcome = left.run(12);
    let right_outcome = right.run(12);

    assert_eq!(left_outcome, right_outcome);
    assert_ne!(left_outcome.history_hash, 0);
}

#[test]
fn world_healthy_cluster_makes_progress() {
    let mut world = SimWorld::new(45, SimConfig::default());

    let outcome = world.run(8);

    assert_eq!(outcome.steps, 8);
    assert_eq!(outcome.accepted_ops, 8);
    assert!(outcome.delivered_messages > 0);
    assert_ne!(outcome.history_hash, 0);
    assert_eq!(outcome.invariant_violations, 0);
    assert!(world.invariant_report().is_ok());
}
