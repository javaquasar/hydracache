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
