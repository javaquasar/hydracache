use hydracache_sim::run_checkpoint_rescale;

#[test]
fn rescale_with_checkpoint_loses_no_committed_write() {
    for seed in [1, 55, 1_337, 5_555, 65_535] {
        let report = run_checkpoint_rescale(seed);

        assert!(report.passed(), "{report:?}");
        assert!(report.committed_after >= report.committed_before);
    }
}

#[test]
fn checkpoint_sim_is_seeded_and_replayable() {
    let first = run_checkpoint_rescale(55);
    let second = run_checkpoint_rescale(55);

    assert_eq!(first, second);
    assert_eq!(first.seed, 55);
}
