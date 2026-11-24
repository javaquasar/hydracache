use std::collections::BTreeSet;

use hydracache_sim::{run_scenario, scenario_matches_expectation, scenario_presets};

#[test]
fn each_preset_seed_is_deterministic_and_matches_expected_verdict() {
    let presets = scenario_presets();
    assert!(!presets.is_empty());

    let mut names = BTreeSet::new();
    for preset in presets {
        assert!(
            names.insert(preset.name),
            "duplicate scenario {}",
            preset.name
        );

        let first = run_scenario(preset.name).expect("scenario runs");
        let second = run_scenario(preset.name).expect("scenario reruns");

        assert_eq!(first.snapshot, second.snapshot);
        assert_eq!(first.snapshot.seed, preset.seed);
        assert_eq!(first.snapshot.step, preset.steps);
        assert!(
            scenario_matches_expectation(&preset, &first.snapshot),
            "scenario {} did not match expected {:?}/{:?}: {:?}",
            preset.name,
            preset.expected_verdict,
            preset.expected_progress,
            first.snapshot
        );
    }
}

#[test]
fn presets_on_raft_election_keep_their_source_and_expectations() {
    use hydracache_sim::{run_scenario_with, scenario_matches_expectation, SimConfig, SimWorld};

    for preset in hydracache_sim::scenario_presets() {
        let run = run_scenario_with(preset.name, |seed| {
            SimWorld::with_raft_election(seed, SimConfig::default())
        })
        .expect("preset runs on raft");
        assert_eq!(run.snapshot.election_source, "raft", "{}", preset.name);
        assert_eq!(run.snapshot.active_scenario.as_deref(), Some(preset.name));
        assert!(
            scenario_matches_expectation(&preset, &run.snapshot),
            "{} does not meet its expectation on raft: {:?}",
            preset.name,
            run.snapshot.progress
        );
    }
}
