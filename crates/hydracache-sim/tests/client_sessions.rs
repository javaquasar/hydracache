//! TD-0016 step 2: a client is admitted through the (simulated) Raft metadata
//! path, stays connected to one member, and writes through that member. The
//! Raft leader is not on the write path.

use std::collections::{BTreeMap, BTreeSet};

use hydracache_sim::{SimConfig, SimSnapshot, SimWorld};

const NS: &str = "profiles";

fn formed_world(seed: u64) -> SimWorld {
    let mut world = SimWorld::new(seed, SimConfig::default());
    world.set_workload_enabled(false);
    world.run(8);
    assert!(leader(&world.snapshot()).is_some(), "cluster formed");
    world
}

fn leader(snapshot: &SimSnapshot) -> Option<String> {
    snapshot
        .nodes
        .iter()
        .find(|node| node.vote_state == "leader")
        .map(|node| node.id.clone())
}

fn connected(snapshot: &SimSnapshot, client: &str) -> String {
    snapshot
        .admitted_clients
        .iter()
        .find(|entry| entry.id == client)
        .and_then(|entry| entry.connected_node.clone())
        .unwrap_or_else(|| panic!("{client} is admitted and connected"))
}

/// The node that applied `key` at the current step: the write's entry node.
fn entry_node(snapshot: &SimSnapshot, key: &str) -> Vec<String> {
    snapshot
        .keys
        .iter()
        .find(|view| view.key == key)
        .map(|view| {
            view.replicas
                .iter()
                .filter(|replica| replica.applied_at_step == Some(snapshot.step))
                .map(|replica| replica.node_id.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// A copy of `world` rebuilt from its replay script, to try things on.
fn probe(world: &SimWorld) -> SimWorld {
    let mut copy = hydracache_sim::replay_world(&world.replay_script(), |seed| {
        SimWorld::new(seed, SimConfig::default())
    })
    .expect("world replays");
    copy.set_workload_enabled(false);
    copy
}

/// A client name the routing hash attaches to a node other than the leader.
fn follower_client(world: &SimWorld) -> String {
    let leader = leader(&world.snapshot()).expect("leader");
    (0..64)
        .map(|index| format!("writer-{index}"))
        .find(|name| {
            let mut copy = probe(world);
            copy.add_client(name, NS).expect("probe admits");
            connected(&copy.snapshot(), name) != leader
        })
        .expect("some client name routes to a follower")
}

#[test]
fn a_write_enters_on_the_clients_member_not_the_leader() {
    let mut world = formed_world(0x5402);
    let client = follower_client(&world);
    world.add_client(&client, NS).expect("client admitted");
    let snapshot = world.snapshot();
    let member = connected(&snapshot, &client);
    assert_ne!(Some(member.clone()), leader(&snapshot));

    world
        .push_event(&client, NS, "p", "fresh")
        .expect("push applies");
    assert_eq!(entry_node(&world.snapshot(), "profiles:p"), [member]);
}

#[test]
fn a_new_client_cannot_join_without_a_leader_but_an_admitted_one_keeps_writing() {
    let mut world = formed_world(0x5403);
    world
        .add_client("writer", NS)
        .expect("admitted with a leader");
    let leader = leader(&world.snapshot()).expect("leader");
    assert!(world.crash_node(leader.clone()));

    // No live leader until the next election: admission is a metadata
    // proposal and fails loud with the product's error.
    let error = world.add_client("late", NS).unwrap_err().to_string();
    assert!(
        error.contains("no raft leader; retry metadata proposal after election"),
        "{error}"
    );
    let error = world.subscribe("late-reader", NS).unwrap_err().to_string();
    assert!(error.contains("no raft leader"), "{error}");
    let error = world
        .push_event("late-writer", NS, "p", "x")
        .unwrap_err()
        .to_string();
    assert!(error.contains("no raft leader"), "{error}");
    assert!(world
        .snapshot()
        .admitted_clients
        .iter()
        .all(|entry| entry.id == "writer"));

    // The admitted client needs no leader to write.
    world
        .push_event("writer", NS, "p", "fresh")
        .expect("an admitted client writes without a leader");
    let entry = entry_node(&world.snapshot(), "profiles:p");
    assert_eq!(entry.len(), 1);
    assert_ne!(entry[0], leader, "a crashed member never takes the write");
}

#[test]
fn connection_is_sticky_and_moves_only_when_its_member_goes_down() {
    let mut world = formed_world(0x5404);
    world.add_client("writer", NS).expect("admitted");
    let member = connected(&world.snapshot(), "writer");
    let other = world
        .snapshot()
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .find(|node| *node != member)
        .expect("another node");

    assert!(world.crash_node(other.clone()));
    world.run(3);
    assert_eq!(connected(&world.snapshot(), "writer"), member, "unaffected");
    assert!(world.restart_node(other));
    world.run(3);
    assert_eq!(connected(&world.snapshot(), "writer"), member, "no re-pin");

    assert!(world.crash_node(member.clone()));
    world.run(1);
    let moved = connected(&world.snapshot(), "writer");
    assert_ne!(moved, member, "reconnected away from the crashed member");
    assert!(world
        .trace()
        .iter()
        .any(|line| line == &format!("client_connect:writer:{member}->{moved}")));
    assert!(world.restart_node(member.clone()));
    world.run(3);
    assert_eq!(connected(&world.snapshot(), "writer"), moved, "stays put");
}

#[test]
fn readers_and_writers_share_one_admitted_session_that_outlives_its_roles() {
    let mut world = formed_world(0x5405);
    world.add_client("app", NS).expect("writer admitted");
    world.subscribe("app", NS).expect("same process subscribes");
    world.subscribe("reader", NS).expect("reader admitted");
    let snapshot = world.snapshot();
    let admitted = snapshot
        .admitted_clients
        .iter()
        .map(|entry| (entry.id.clone(), entry.admitted_at_step))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        admitted,
        BTreeMap::from([("app".to_owned(), 8), ("reader".to_owned(), 8)])
    );
    let app_member = connected(&snapshot, "app");
    let app_writer = snapshot
        .clients
        .iter()
        .find(|client| client.id == "app")
        .expect("writer");
    let app_reader = snapshot
        .subscribers
        .iter()
        .find(|subscriber| subscriber.client_id == "app")
        .expect("subscriber");
    assert_eq!(
        app_writer.connected_node.as_deref(),
        Some(app_member.as_str())
    );
    assert_eq!(
        app_reader.connected_node.as_deref(),
        Some(app_member.as_str())
    );

    world.remove_client("app").expect("writer removed");
    world.unsubscribe("app", NS).expect("subscriber removed");
    assert!(world
        .snapshot()
        .admitted_clients
        .iter()
        .any(|entry| entry.id == "app"));
}

#[test]
fn workload_clients_each_write_through_one_member() {
    let mut world = SimWorld::new(0x5406, SimConfig::default());
    world.run(40);
    let mut members = BTreeMap::<String, BTreeSet<String>>::new();
    for line in world.trace() {
        let mut parts = line.splitn(4, ':');
        if parts.next() != Some("client") {
            continue;
        }
        let (Some(client), Some(node)) = (parts.next(), parts.next()) else {
            continue;
        };
        members
            .entry(client.to_owned())
            .or_default()
            .insert(node.to_owned());
    }
    assert!(!members.is_empty(), "the workload ran");
    for (client, nodes) in members {
        assert_eq!(nodes.len(), 1, "workload client {client} used {nodes:?}");
    }
}
