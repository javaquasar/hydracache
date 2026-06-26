use hydracache_sim::{
    scripted_lab_catalog, ControlActionV1, ReplayScriptV1, SimConfig, SimMode, SimWorld,
};

#[test]
fn scripted_mode_loops_catalog_deterministically() {
    let catalog = scripted_lab_catalog();
    assert!(catalog.len() >= 3);

    for script in &catalog {
        let first = run_script(script);
        let second = run_script(script);
        assert_eq!(first.snapshot().to_json(), second.snapshot().to_json());
        assert_eq!(first.snapshot().mode, "scripted");
        assert!(first.snapshot().active_scenario.is_some());
    }
}

#[test]
fn user_intervention_merges_into_scripted_run_and_replays() {
    let mut script = scripted_lab_catalog()
        .into_iter()
        .find(|script| script.scenario.as_deref() == Some("manual-push-convergence"))
        .expect("manual push script exists");
    script.mode = SimMode::Mixed;
    script.actions.push(ControlActionV1::Isolate {
        at_step: 10,
        node: "node-0".to_owned(),
    });
    script.actions.push(ControlActionV1::Rejoin {
        at_step: 11,
        node: "node-0".to_owned(),
    });

    let first = run_script(&script);
    let replay = first.replay_script();
    let second = run_script(&replay);

    assert_eq!(first.snapshot().to_json(), second.snapshot().to_json());
    assert_eq!(first.snapshot().mode, "mixed");
    assert!(first.snapshot().intervention_count >= script.actions.len() as u64);
}

#[test]
fn mode_and_scenario_are_in_snapshot() {
    let script = ReplayScriptV1 {
        version: hydracache_sim::REPLAY_SCRIPT_VERSION,
        seed: 0x5355,
        mode: SimMode::Mixed,
        scenario: Some("custom-mixed".to_owned()),
        actions: vec![
            ControlActionV1::ModeChange {
                at_step: 0,
                mode: SimMode::Mixed,
            },
            ControlActionV1::Step { at_step: 0, n: 2 },
        ],
    };

    let world = run_script(&script);
    let snapshot = world.snapshot();

    assert_eq!(snapshot.mode, "mixed");
    assert_eq!(snapshot.active_scenario.as_deref(), Some("custom-mixed"));
    assert_eq!(snapshot.intervention_count, 2);
}

fn run_script(script: &ReplayScriptV1) -> SimWorld {
    let mut world = SimWorld::new(script.seed, SimConfig::default());
    world
        .apply_replay_script(script)
        .expect("scripted mode applies");
    world
}
