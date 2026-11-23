use hydracache_sim::{
    replay_world, run_scenario, ControlActionV1, ReplayScriptV1, SimConfig, SimMode, SimWorld,
    MAX_REPLAY_ACTIONS, REPLAY_SCRIPT_VERSION,
};

#[test]
fn replay_script_roundtrips_json() {
    let script = ReplayScriptV1::new(
        0x5331,
        SimMode::Manual,
        vec![
            ControlActionV1::Subscribe {
                at_step: 8,
                client: "client-a".to_owned(),
                ns: "profiles".to_owned(),
            },
            ControlActionV1::PushEvent {
                at_step: 8,
                client: "client-a".to_owned(),
                ns: "profiles".to_owned(),
                key: "profile-42".to_owned(),
                value: "fresh".to_owned(),
            },
            ControlActionV1::Step { at_step: 8, n: 2 },
        ],
    );

    let decoded = ReplayScriptV1::from_json(&script.to_json()).expect("current script decodes");

    assert_eq!(decoded, script);
    assert_eq!(decoded.version, REPLAY_SCRIPT_VERSION);
}

#[test]
fn unknown_future_replay_version_rejected() {
    let future = serde_json::json!({
        "version": REPLAY_SCRIPT_VERSION + 1
    });

    let error =
        ReplayScriptV1::from_json(&future.to_string()).expect_err("future script fails loud");

    assert!(error
        .to_string()
        .contains("unsupported replay script version"));
}

#[test]
fn replay_script_over_max_actions_refuses_loud() {
    let script = ReplayScriptV1::new(
        0x5332,
        SimMode::Manual,
        (0..=MAX_REPLAY_ACTIONS)
            .map(|index| ControlActionV1::Step {
                at_step: index as u64,
                n: 1,
            })
            .collect(),
    );

    let error = ReplayScriptV1::from_json(&script.to_json())
        .expect_err("oversized replay script fails loud");

    assert!(error.to_string().contains("max supported"));
}

#[test]
fn fault_actions_roundtrip_with_stable_tags() {
    let mut script = ReplayScriptV1::new(
        0x5402,
        SimMode::Manual,
        vec![
            ControlActionV1::Workload {
                at_step: 0,
                enabled: false,
            },
            ControlActionV1::Crash {
                at_step: 2,
                node: "node-1".to_owned(),
            },
            ControlActionV1::Restart {
                at_step: 4,
                node: "node-1".to_owned(),
            },
            ControlActionV1::Partition {
                at_step: 5,
                from: "node-0".to_owned(),
                to: "node-2".to_owned(),
            },
            ControlActionV1::Heal {
                at_step: 6,
                from: "node-0".to_owned(),
                to: "node-2".to_owned(),
            },
            ControlActionV1::DropNext {
                at_step: 7,
                from: "node-2".to_owned(),
                to: "node-0".to_owned(),
            },
            ControlActionV1::DelayNext {
                at_step: 7,
                from: "node-2".to_owned(),
                to: "node-1".to_owned(),
                millis: 250,
            },
        ],
    );
    script.target_step = Some(9);

    let json = script.to_json();
    for tag in [
        "\"workload\"",
        "\"crash\"",
        "\"restart\"",
        "\"partition\"",
        "\"heal\"",
        "\"drop_next\"",
        "\"delay_next\"",
        "\"target_step\":9",
    ] {
        assert!(json.contains(tag), "missing {tag} in {json}");
    }
    assert_eq!(ReplayScriptV1::from_json(&json).expect("decodes"), script);
}

#[test]
fn legacy_script_without_target_step_decodes() {
    let legacy = r#"{"version":1,"seed":7,"mode":"manual","scenario":null,"actions":[]}"#;

    let script = ReplayScriptV1::from_json(legacy).expect("legacy script decodes");

    assert_eq!(script.target_step, None);
    assert!(!script.to_json().contains("target_step"));
}

#[test]
fn recorded_faults_replay_to_identical_snapshot() {
    let mut world = SimWorld::new(0x5403, SimConfig::default());
    world.run(5);
    for action in [
        ControlActionV1::Workload {
            at_step: 5,
            enabled: false,
        },
        ControlActionV1::Crash {
            at_step: 5,
            node: "node-2".to_owned(),
        },
        ControlActionV1::Partition {
            at_step: 5,
            from: "node-0".to_owned(),
            to: "node-1".to_owned(),
        },
    ] {
        world.apply_control_action(action).expect("action applies");
    }
    world.run(4);
    world
        .apply_control_action(ControlActionV1::Restart {
            at_step: 9,
            node: "node-2".to_owned(),
        })
        .expect("restart applies");
    world.run(3);

    let script = world.replay_script();
    assert_eq!(script.actions.len(), 4);
    assert_eq!(script.target_step, Some(12));

    let replayed = replay_world(&script, |seed| SimWorld::new(seed, SimConfig::default()))
        .expect("replay applies");
    assert_eq!(replayed.snapshot_json(), world.snapshot_json());
}

#[test]
fn unchanged_workload_toggle_is_not_an_intervention() {
    let mut world = SimWorld::new(0x5404, SimConfig::default());

    world
        .apply_control_action(ControlActionV1::Workload {
            at_step: 0,
            enabled: true,
        })
        .expect("no-op toggle applies");

    assert!(world.action_log().is_empty());
}

#[test]
fn scenario_world_carries_its_name_into_replay() {
    let run = run_scenario("symmetric_partition_heal_converges").expect("scenario runs");
    let script = run.world.replay_script();

    assert_eq!(
        script.scenario.as_deref(),
        Some("symmetric_partition_heal_converges")
    );
    let replayed = replay_world(&script, |seed| SimWorld::new(seed, SimConfig::default()))
        .expect("scenario replay applies");
    assert_eq!(replayed.snapshot_json(), run.world.snapshot_json());
}

#[test]
fn scenario_replay_with_foreign_seed_fails_loud() {
    let mut script = run_scenario("minority_partition_cannot_commit")
        .expect("scenario runs")
        .world
        .replay_script();
    script.seed += 1;

    let error = replay_world(&script, |seed| SimWorld::new(seed, SimConfig::default()))
        .expect_err("seed mismatch fails loud");

    assert!(error.to_string().contains("does not match scenario"));
}

#[test]
fn target_step_before_actions_fails_loud() {
    let mut script = ReplayScriptV1::new(
        0x5405,
        SimMode::Manual,
        vec![ControlActionV1::Crash {
            at_step: 6,
            node: "node-0".to_owned(),
        }],
    );
    script.target_step = Some(3);

    let error = replay_world(&script, |seed| SimWorld::new(seed, SimConfig::default()))
        .expect_err("target before actions fails loud");

    assert!(error.to_string().contains("target_step"));
}

#[test]
fn invalid_fault_action_is_rejected_and_not_logged() {
    let mut world = SimWorld::new(0x5406, SimConfig::default());

    let error = world
        .apply_control_action(ControlActionV1::Restart {
            at_step: 0,
            node: "node-0".to_owned(),
        })
        .expect_err("restart of a running node fails loud");

    assert!(error.to_string().contains("not crashed"));
    assert!(world.action_log().is_empty());
}
