use hydracache_sim::{SimConfig, SimWorld};

#[test]
fn dst_fast_budget_runs_bounded_seed_matrix() {
    for seed in 44..49 {
        let mut world = SimWorld::new(seed, SimConfig::default());
        let outcome = world.run(32);

        assert_eq!(outcome.seed, seed);
        assert_eq!(outcome.steps, 32);
        assert_eq!(outcome.accepted_ops, 32);
        assert_eq!(outcome.invariant_violations, 0);
    }
}
