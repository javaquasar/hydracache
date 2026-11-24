use hydracache_sim::{ControlActionV1, SimConfig, SimWorld, MAX_SUBSCRIBER_BUFFER};

const NS: &str = "profiles";
const KEY: &str = "profile-42";
const FULL_KEY: &str = "profiles:profile-42";

#[test]
fn pushed_event_replicates_and_converges() {
    let mut world = manual_world();

    world
        .push_event("client-a", NS, KEY, "fresh")
        .expect("manual push applies");
    world.run(2);

    let snapshot = world.snapshot();
    let versions = versions_for(&snapshot, FULL_KEY);
    assert_eq!(versions.len(), 3);
    assert!(versions.iter().all(|version| *version > 0));
    assert!(versions.iter().all(|version| *version == versions[0]));
}

#[test]
fn subscriber_receives_event_after_push() {
    let mut world = manual_world();
    world.subscribe("client-a", NS);

    world
        .push_event("client-a", NS, KEY, "fresh")
        .expect("manual push applies");
    assert!(world
        .snapshot()
        .subscribers
        .iter()
        .all(|subscriber| subscriber.last_event.is_none()));

    world.run(2);
    let snapshot = world.snapshot();
    let subscriber = snapshot
        .subscribers
        .iter()
        .find(|subscriber| subscriber.id == "client-a@profiles")
        .expect("subscriber is visible");
    let event = subscriber.last_event.as_ref().expect("event delivered");
    assert_eq!(event.kind, "upserted");
    assert_eq!(event.key, FULL_KEY);
    assert!(event.commit_index > 0);
    assert!(event.delivered_at_step >= snapshot.step.saturating_sub(2));
    assert_eq!(subscriber.lag, 0);
}

#[test]
fn divergence_then_convergence_is_observable_in_keys() {
    let mut world = manual_world();

    world
        .push_event("client-a", NS, KEY, "fresh")
        .expect("manual push applies");
    let diverged = versions_for(&world.snapshot(), FULL_KEY);
    assert!(diverged.contains(&0));
    assert!(diverged.iter().any(|version| *version > 0));

    world.run(2);
    let converged = versions_for(&world.snapshot(), FULL_KEY);
    assert!(converged.iter().all(|version| *version > 0));
    assert!(converged.iter().all(|version| *version == converged[0]));
}

#[test]
fn subscriber_only_sees_bus_carried_event_kinds() {
    let mut world = manual_world();
    world.subscribe("client-a", NS);
    world
        .push_event("client-a", NS, KEY, "fresh")
        .expect("manual push applies");
    world.run(2);

    let snapshot = world.snapshot();
    let kinds = snapshot
        .subscribers
        .iter()
        .filter_map(|subscriber| subscriber.last_event.as_ref())
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();

    assert_eq!(kinds, vec!["upserted"]);
}

#[test]
fn slow_subscriber_drops_with_counter() {
    let mut world = manual_world();
    world.subscribe("client-a", NS);

    for index in 0..(MAX_SUBSCRIBER_BUFFER + 3) {
        world
            .push_event("client-a", NS, format!("key-{index}"), format!("v-{index}"))
            .expect("manual push applies");
    }

    let snapshot = world.snapshot();
    let subscriber = snapshot
        .subscribers
        .iter()
        .find(|subscriber| subscriber.id == "client-a@profiles")
        .expect("subscriber is visible");
    assert_eq!(subscriber.lag, MAX_SUBSCRIBER_BUFFER as u64);
    assert_eq!(subscriber.dropped, 3);
}

#[test]
fn manual_push_uses_control_action_surface() {
    let mut world = manual_world();
    let step = world.outcome().steps;
    world
        .apply_control_action(ControlActionV1::Subscribe {
            at_step: step,
            client: "client-a".to_owned(),
            ns: NS.to_owned(),
        })
        .expect("subscribe action applies");
    world
        .apply_control_action(ControlActionV1::PushEvent {
            at_step: step,
            client: "client-a".to_owned(),
            ns: NS.to_owned(),
            key: KEY.to_owned(),
            value: "fresh".to_owned(),
        })
        .expect("push action applies");
    world.run(2);

    let snapshot = world.snapshot();
    assert_eq!(snapshot.clients.len(), 1);
    assert!(snapshot
        .subscribers
        .iter()
        .any(|subscriber| subscriber.last_event.is_some()));
}

fn manual_world() -> SimWorld {
    let mut world = SimWorld::new(0x5330, SimConfig::default());
    world.set_workload_enabled(false);
    world.run(8);
    world
}

fn versions_for(snapshot: &hydracache_sim::SimSnapshot, key: &str) -> Vec<u64> {
    snapshot
        .keys
        .iter()
        .find(|entry| entry.key == key)
        .expect("key is visible")
        .replicas
        .iter()
        .map(|replica| replica.version)
        .collect()
}

// Since 0.74 writers and subscribers are separate: subscribing creates only a
// reader, a writer can be registered before it writes, and a subscriber can
// leave.
#[test]
fn subscribe_creates_only_a_subscriber() {
    let mut world = manual_world();
    world.subscribe("reader-1", NS);
    let snapshot = world.snapshot();
    assert!(snapshot.clients.is_empty(), "no writer twin");
    assert_eq!(snapshot.subscribers.len(), 1);
    assert_eq!(snapshot.subscribers[0].id, "reader-1@profiles");
}

#[test]
fn add_client_registers_a_writer_without_writing() {
    let mut world = manual_world();
    let committed = world.snapshot().progress.committed_entries;
    world
        .add_client("client-b", NS)
        .expect("a new writer registers");
    let snapshot = world.snapshot();
    let writer = snapshot
        .clients
        .iter()
        .find(|client| client.id == "client-b")
        .expect("writer is visible");
    assert_eq!(writer.namespace, NS);
    assert_eq!(writer.last_op, None);
    assert!(writer.connected_node.is_some());
    assert!(snapshot.subscribers.is_empty());
    assert_eq!(
        snapshot.progress.committed_entries, committed,
        "nothing written"
    );

    let again = world.add_client("client-b", NS).unwrap_err();
    assert!(again.to_string().contains("already exists"), "{again}");
    let empty = world.add_client(" ", NS).unwrap_err();
    assert!(empty.to_string().contains("needs a client name"), "{empty}");
}

#[test]
fn unsubscribed_reader_stops_receiving_and_fails_loud_twice() {
    let mut world = manual_world();
    world.subscribe("reader-1", NS);
    world
        .unsubscribe("reader-1", NS)
        .expect("a subscriber can leave");
    world
        .push_event("client-a", NS, KEY, "fresh")
        .expect("manual push applies");
    world.run(2);
    assert!(world.snapshot().subscribers.is_empty());

    let twice = world.unsubscribe("reader-1", NS).unwrap_err();
    assert!(twice.to_string().contains("is not subscribed"), "{twice}");
}

#[test]
fn writer_and_subscriber_actions_replay_identically() {
    let mut world = SimWorld::new(0x5331, SimConfig::default());
    for action in [
        ControlActionV1::Workload {
            at_step: 0,
            enabled: false,
        },
        ControlActionV1::AddClient {
            at_step: 0,
            client: "client-b".to_owned(),
            ns: NS.to_owned(),
        },
        ControlActionV1::Subscribe {
            at_step: 0,
            client: "reader-1".to_owned(),
            ns: NS.to_owned(),
        },
        ControlActionV1::Step { at_step: 0, n: 8 },
        ControlActionV1::PushEvent {
            at_step: 8,
            client: "client-b".to_owned(),
            ns: NS.to_owned(),
            key: KEY.to_owned(),
            value: "fresh".to_owned(),
        },
        ControlActionV1::Step { at_step: 8, n: 2 },
        ControlActionV1::Unsubscribe {
            at_step: 10,
            client: "reader-1".to_owned(),
            ns: NS.to_owned(),
        },
    ] {
        world.apply_control_action(action).expect("action applies");
    }
    let script = world.replay_script();
    let json = script.to_json();
    assert!(json.contains("\"type\":\"add_client\""), "{json}");
    assert!(json.contains("\"type\":\"unsubscribe\""), "{json}");
    let replayed = hydracache_sim::replay_world(
        &hydracache_sim::ReplayScriptV1::from_json(&json).expect("script parses"),
        |seed| SimWorld::new(seed, SimConfig::default()),
    )
    .expect("script replays");
    assert_eq!(replayed.snapshot(), world.snapshot());
}
