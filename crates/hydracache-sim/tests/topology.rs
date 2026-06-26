use hydracache_sim::{ControlActionV1, ReplayScriptV1, SimConfig, SimMode, SimSnapshot, SimWorld};

const NS: &str = "profiles";
const KEY: &str = "profile-42";
const FULL_KEY: &str = "profiles:profile-42";

#[test]
fn isolating_leader_triggers_reelection() {
    let mut world = topology_world();
    let leader = leader_id(&world.snapshot());

    assert!(world.isolate_node(leader.clone()));
    let mut snapshot = world.snapshot();
    let mut new_leader = None;
    for _ in 0..8 {
        world.step();
        snapshot = world.snapshot();
        new_leader = snapshot
            .nodes
            .iter()
            .find(|node| node.vote_state == "leader")
            .map(|node| node.id.clone());
        if new_leader.is_some() {
            break;
        }
    }
    let new_leader = new_leader.expect("new leader visible after deterministic reelection");

    assert_ne!(new_leader, leader);
    assert_eq!(snapshot.verdict, hydracache_sim::VerdictView::Holding);
}

#[test]
fn rejoin_node_catches_up_to_leader_commit() {
    let mut world = topology_world();
    assert!(world.isolate_node("node-2"));

    world
        .push_event("client-a", NS, KEY, "fresh")
        .expect("push applies");
    world.run(2);
    assert_eq!(version_for(&world.snapshot(), FULL_KEY, "node-2"), Some(0));

    assert!(world.rejoin_node("node-2"));
    let snapshot = world.snapshot();
    let leader_version = snapshot
        .keys
        .iter()
        .find(|key| key.key == FULL_KEY)
        .expect("key visible")
        .replicas
        .iter()
        .map(|replica| replica.version)
        .max()
        .unwrap_or_default();

    assert_eq!(
        version_for(&snapshot, FULL_KEY, "node-2"),
        Some(leader_version)
    );
    assert!(snapshot.rebalance.is_some());
}

#[test]
fn add_node_grows_membership_deterministically() {
    let mut first = topology_world();
    let mut second = topology_world();

    let first_id = first.add_node();
    let second_id = second.add_node();

    assert_eq!(first_id, second_id);
    assert_eq!(first.snapshot().nodes.len(), 4);
    assert!(first
        .snapshot()
        .nodes
        .iter()
        .any(|node| node.id == "node-3"));
}

#[test]
fn reshard_moves_partitions_to_new_node() {
    let mut world = topology_world();
    world
        .push_event("client-a", NS, KEY, "fresh")
        .expect("push applies");
    world.run(2);

    let new_node = world.add_node();
    let snapshot = world.snapshot();

    assert_eq!(
        version_for(&snapshot, FULL_KEY, new_node.as_str()),
        snapshot
            .keys
            .iter()
            .find(|key| key.key == FULL_KEY)
            .and_then(|key| key.replicas.iter().map(|replica| replica.version).max())
    );
    let rebalance = snapshot.rebalance.as_ref().expect("rebalance visible");
    assert_eq!(rebalance.phase, "complete");
    assert!(rebalance.moved_partitions > 0);
}

#[test]
fn invariants_hold_across_isolate_rejoin_and_scale_out() {
    let mut world = topology_world();
    let leader = leader_id(&world.snapshot());
    assert!(world.isolate_node(leader));
    world.step();
    assert!(world.rejoin_node("node-0"));
    world.add_node();
    world.run(2);

    assert!(world.invariant_report().is_ok());
}

#[test]
fn topology_actions_replay_from_seed() {
    let script = ReplayScriptV1::new(
        0x5341,
        SimMode::Mixed,
        vec![
            ControlActionV1::Step { at_step: 0, n: 8 },
            ControlActionV1::PushEvent {
                at_step: 8,
                client: "client-a".to_owned(),
                ns: NS.to_owned(),
                key: KEY.to_owned(),
                value: "fresh".to_owned(),
            },
            ControlActionV1::Step { at_step: 8, n: 2 },
            ControlActionV1::Isolate {
                at_step: 10,
                node: "node-0".to_owned(),
            },
            ControlActionV1::Step { at_step: 10, n: 1 },
            ControlActionV1::Rejoin {
                at_step: 11,
                node: "node-0".to_owned(),
            },
            ControlActionV1::AddNode { at_step: 11 },
        ],
    );
    let mut first = SimWorld::new(script.seed, SimConfig::default());
    let mut second = SimWorld::new(script.seed, SimConfig::default());

    first
        .apply_replay_script(&script)
        .expect("first replay applies");
    second
        .apply_replay_script(&script)
        .expect("second replay applies");

    assert_eq!(first.snapshot().to_json(), second.snapshot().to_json());
}

#[test]
fn disable_and_enable_node_updates_snapshot() {
    let mut world = topology_world();

    assert!(world.disable_node("node-1"));
    let disabled = world.snapshot();
    assert!(disabled
        .nodes
        .iter()
        .any(|node| node.id == "node-1" && node.disabled && !node.up));

    assert!(world.enable_node("node-1"));
    let enabled = world.snapshot();
    assert!(enabled
        .nodes
        .iter()
        .any(|node| node.id == "node-1" && !node.disabled && node.up));
}

fn topology_world() -> SimWorld {
    let mut world = SimWorld::new(0x5342, SimConfig::default());
    world.set_workload_enabled(false);
    world.run(8);
    world
}

fn leader_id(snapshot: &SimSnapshot) -> String {
    snapshot
        .nodes
        .iter()
        .find(|node| node.vote_state == "leader")
        .expect("leader visible")
        .id
        .clone()
}

fn version_for(snapshot: &SimSnapshot, key: &str, node_id: &str) -> Option<u64> {
    snapshot
        .keys
        .iter()
        .find(|entry| entry.key == key)?
        .replicas
        .iter()
        .find(|replica| replica.node_id == node_id)
        .map(|replica| replica.version)
}
