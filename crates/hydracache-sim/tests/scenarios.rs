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
