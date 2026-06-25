use hydracache_sim::{
    ControlActionV1, ReplayScriptV1, SimMode, MAX_REPLAY_ACTIONS, REPLAY_SCRIPT_VERSION,
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
