use std::fs;
use std::path::PathBuf;

use hydracache_sim::{
    ControlActionV1, ReplayScriptV1, SimConfig, SimMode, SimSnapshot, SimWorld,
    SIM_SNAPSHOT_SCHEMA_VERSION,
};

#[test]
fn full_mixed_run_replays_identically_from_script() {
    let script = mixed_lab_script();
    let encoded = script.to_json();
    let decoded = ReplayScriptV1::from_json(&encoded).expect("current replay script decodes");

    let first = replay_snapshot_history_hash(&script);
    let second = replay_snapshot_history_hash(&decoded);

    assert_eq!(
        first, second,
        "same script must produce byte-identical history hash"
    );

    let world = run_script(&script);
    let snapshot = world.snapshot();
    assert_eq!(snapshot.mode, "mixed");
    assert_eq!(snapshot.active_scenario.as_deref(), Some("mixed-lab"));
    assert_eq!(snapshot.nodes.len(), 4);
    assert!(snapshot
        .subscribers
        .iter()
        .any(|subscriber| subscriber.last_event.is_some()));
}

#[test]
fn every_new_snapshot_field_bumped_schema() {
    assert_eq!(SIM_SNAPSHOT_SCHEMA_VERSION, 6);

    let mut world = SimWorld::new(0x5371, SimConfig::default());
    world
        .apply_replay_script(&mixed_lab_script())
        .expect("mixed script applies");
    let snapshot = world.snapshot();
    let encoded = snapshot.to_json();
    let decoded = SimSnapshot::from_json(&encoded).expect("current snapshot decodes");

    assert_eq!(decoded.schema_version, 6);
    assert_eq!(decoded.formation_phase, "formed");
    assert!(decoded
        .nodes
        .iter()
        .any(|node| node.vote_state == "leader" && node.voted_for.is_some()));
    assert!(decoded.in_flight.len() <= hydracache_sim::MAX_IN_FLIGHT_RENDERED);
    assert!(!decoded.clients.is_empty());
    assert!(!decoded.subscribers.is_empty());
    assert!(decoded.rebalance.is_some());
    assert_eq!(decoded.mode, "mixed");
    assert_eq!(decoded.active_scenario.as_deref(), Some("mixed-lab"));
    assert!(decoded.intervention_count >= 1);

    let compat = fs::read_to_string(repo_root().join("docs/COMPAT.md")).expect("COMPAT exists");
    for marker in [
        "Version `2` adds the 0.53 W1 election/formation fields",
        "Version `3` adds typed `in_flight`",
        "Version `4` adds manual-mode `clients`, `subscribers`, and `sync_progress`",
        "Version `5` adds topology/mode state",
        "`ReplayScriptV1` simulator control artifact",
    ] {
        assert!(compat.contains(marker), "missing COMPAT marker: {marker}");
    }
}

#[test]
fn reproducer_roundtrips_seed_mode_and_actions() {
    let script = mixed_lab_script();
    let decoded = ReplayScriptV1::from_json(&script.to_json()).expect("script round-trips");

    assert_eq!(decoded.seed, script.seed);
    assert_eq!(decoded.mode, SimMode::Mixed);
    assert_eq!(decoded.scenario.as_deref(), Some("mixed-lab"));
    assert_eq!(decoded.actions, script.actions);

    let world = run_script(&decoded);
    let reproducer = world.replay_script();

    assert_eq!(reproducer.seed, script.seed);
    assert_eq!(reproducer.mode, SimMode::Mixed);
    assert_eq!(reproducer.scenario.as_deref(), Some("mixed-lab"));
    assert_eq!(reproducer.actions, script.actions);
}

fn mixed_lab_script() -> ReplayScriptV1 {
    ReplayScriptV1 {
        version: hydracache_sim::REPLAY_SCRIPT_VERSION,
        seed: 0x5370,
        mode: SimMode::Mixed,
        scenario: Some("mixed-lab".to_owned()),
        actions: vec![
            ControlActionV1::Step { at_step: 0, n: 8 },
            ControlActionV1::ModeChange {
                at_step: 8,
                mode: SimMode::Mixed,
            },
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
            ControlActionV1::Isolate {
                at_step: 10,
                node: "node-0".to_owned(),
            },
            ControlActionV1::Step { at_step: 10, n: 2 },
            ControlActionV1::Rejoin {
                at_step: 12,
                node: "node-0".to_owned(),
            },
            ControlActionV1::AddNode { at_step: 12 },
            ControlActionV1::PushEvent {
                at_step: 12,
                client: "client-a".to_owned(),
                ns: "profiles".to_owned(),
                key: "profile-42".to_owned(),
                value: "newer".to_owned(),
            },
            ControlActionV1::Step { at_step: 12, n: 3 },
        ],
    }
}

fn run_script(script: &ReplayScriptV1) -> SimWorld {
    let mut world = SimWorld::new(script.seed, SimConfig::default());
    world
        .apply_replay_script(script)
        .expect("mixed replay script applies");
    world
}

fn replay_snapshot_history_hash(script: &ReplayScriptV1) -> u64 {
    let world = run_script(script);
    let mut hash = FNV_OFFSET;
    hash_bytes(&mut hash, world.snapshot().to_json().as_bytes());
    hash_bytes(&mut hash, world.replay_script().to_json().as_bytes());
    hash
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("hydracache-sim crate lives under crates/")
        .to_path_buf()
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
