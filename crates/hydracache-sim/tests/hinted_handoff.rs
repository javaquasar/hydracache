//! TD-0016 step 3b: in the replicated model a primary keeps a hint for a
//! backup it cannot reach, replays it when the backup returns, never lets a
//! hint overwrite a newer value, loses hints with its own crash, and an
//! anti-entropy pass repairs copies that were lost in flight.

use hydracache::{rank_replicas, ClusterNodeId, ConsistencyLevel, Replicas};
use hydracache_sim::{SimConfig, SimReplicationModel, SimSnapshot, SimWorld};

const NS: &str = "profiles";

/// A formed world in `replicated { rf: 2, ONE }` with an admitted writer, and
/// a key whose primary and backup are both other than the writer's member.
fn setup(seed: u64) -> (SimWorld, String, Replicas) {
    let mut world = SimWorld::new(seed, SimConfig::default());
    world.set_workload_enabled(false);
    world.run(8);
    world
        .set_replication(SimReplicationModel::Replicated {
            replication_factor: 2,
            write_level: ConsistencyLevel::One,
        })
        .expect("model");
    world.add_client("writer", NS).expect("admitted");
    let snapshot = world.snapshot();
    let member = snapshot
        .admitted_clients
        .iter()
        .find(|entry| entry.id == "writer")
        .and_then(|entry| entry.connected_node.clone())
        .expect("connected");
    let ids = snapshot
        .nodes
        .iter()
        .map(|node| ClusterNodeId::from(node.id.clone()))
        .collect::<Vec<_>>();
    let (key, owners) = (0..128)
        .map(|index| format!("k{index}"))
        .find_map(|key| {
            let owners = rank_replicas(&format!("{NS}:{key}"), ids.iter(), 2)?;
            let elsewhere = owners
                .all_nodes()
                .iter()
                .all(|node| node.as_str() != member);
            elsewhere.then_some((key, owners))
        })
        .expect("a key owned away from the writer's member");
    (world, key, owners)
}

fn holds(snapshot: &SimSnapshot, key: &str, node: &str) -> u64 {
    snapshot
        .keys
        .iter()
        .find(|view| view.key == format!("{NS}:{key}"))
        .and_then(|view| view.replicas.iter().find(|replica| replica.node_id == node))
        .map_or(0, |replica| replica.version)
}

#[test]
fn a_down_backup_gets_a_hint_that_is_replayed_when_it_returns() {
    let (mut world, key, owners) = setup(0x5601);
    let backup = owners.backups[0].to_string();
    assert!(world.crash_node(backup.clone()));
    world
        .push_event("writer", NS, &key, "v1")
        .expect("ONE is met");
    world.run(3);
    let held = world.snapshot();
    let write = held.writes.last().expect("write").clone();
    assert_eq!(write.status, "acknowledged", "the hint is not an ack");
    assert_eq!(write.hinted, std::slice::from_ref(&backup));
    assert_eq!(held.hints.len(), 1);
    assert_eq!(held.hints[0].holder, owners.primary.to_string());
    assert_eq!(held.hints[0].target, backup);
    assert_eq!(holds(&held, &key, &backup), 0);

    assert!(world.restart_node(backup.clone()));
    world.run(4);
    let replayed = world.snapshot();
    assert!(replayed.hints.is_empty());
    assert_eq!(replayed.repair.hints_replayed, 1);
    let primary_version = holds(&replayed, &key, &owners.primary.to_string());
    assert!(primary_version > 0);
    assert_eq!(holds(&replayed, &key, &backup), primary_version);
    let write = replayed.writes.last().expect("write");
    assert!(write.hinted.is_empty());
    assert_eq!(write.backups_applied, [backup]);
}

#[test]
fn a_hint_never_overwrites_a_newer_value() {
    let (mut world, key, owners) = setup(0x5602);
    let backup = owners.backups[0].to_string();
    assert!(world.crash_node(backup.clone()));
    world.push_event("writer", NS, &key, "v1").expect("push v1");
    world.run(2);
    assert!(world.restart_node(backup.clone()));
    // The backup is reachable again: v2 goes to it directly, before the hint
    // for v1 is replayed.
    world.push_event("writer", NS, &key, "v2").expect("push v2");
    world.run(5);
    let snapshot = world.snapshot();
    // Whether the hint went out before v2 arrived or not, the backup never
    // applies v1 over v2: versioned apply suppresses it at the target.
    assert_eq!(snapshot.repair.hints_suppressed, 1, "{:?}", snapshot.repair);
    assert_eq!(
        holds(&snapshot, &key, &backup),
        holds(&snapshot, &key, &owners.primary.to_string()),
        "the backup keeps v2"
    );
}

#[test]
fn hints_are_lost_with_their_holder() {
    let (mut world, key, owners) = setup(0x5603);
    let backup = owners.backups[0].to_string();
    assert!(world.crash_node(backup.clone()));
    world.push_event("writer", NS, &key, "v1").expect("push");
    world.run(2);
    assert_eq!(world.snapshot().hints.len(), 1);
    assert!(world.crash_node(owners.primary.to_string()));
    world.run(1);
    let snapshot = world.snapshot();
    assert!(snapshot.hints.is_empty());
    assert_eq!(snapshot.repair.hints_dropped, 1);
}

#[test]
fn hints_over_budget_are_dropped() {
    let (mut world, key, owners) = setup(0x5604);
    assert!(world.crash_node(owners.backups[0].to_string()));
    for index in 0..40 {
        world
            .push_event("writer", NS, &key, format!("v{index}"))
            .expect("push");
        // The forward reaches the primary, which then stores the hint.
        world.run(2);
    }
    let snapshot = world.snapshot();
    assert_eq!(snapshot.hints.len(), 32);
    assert_eq!(snapshot.repair.hints_dropped, 8);
}

#[test]
fn anti_entropy_repairs_a_copy_lost_in_flight() {
    let (mut world, key, owners) = setup(0x5605);
    let primary = owners.primary.to_string();
    let backup = owners.backups[0].to_string();
    world.push_event("writer", NS, &key, "v1").expect("push");
    // Let the forward reach the primary, then cut primary -> backup while its
    // copy is in flight: the primary sent it, so no hint exists.
    let mut cut = false;
    for _ in 0..4 {
        world.step();
        let snapshot = world.snapshot();
        if snapshot.in_flight.iter().any(|message| {
            message.from == primary && message.to == backup && message.kind == "replicate_put"
        }) {
            assert!(world.partition_link(primary.clone(), backup.clone()));
            cut = true;
            break;
        }
    }
    assert!(cut, "the copy to the backup was observed in flight");
    world.run(2);
    let lost = world.snapshot();
    assert!(lost.hints.is_empty());
    assert_eq!(holds(&lost, &key, &backup), 0, "the copy was lost");

    assert!(world.heal_link(primary.clone(), backup.clone()));
    world.run(8);
    let repaired = world.snapshot();
    assert!(repaired.repair.repair_copies >= 1, "{:?}", repaired.repair);
    assert!(repaired.repair.last_run_step.is_some());
    assert_eq!(
        holds(&repaired, &key, &backup),
        holds(&repaired, &key, &primary)
    );
}

#[test]
fn an_older_invalidation_never_removes_a_newer_local_first_value() {
    let mut world = SimWorld::new(0x5606, SimConfig::default());
    world.set_workload_enabled(false);
    world.run(8);
    world
        .set_replication(SimReplicationModel::LocalFirst)
        .expect("model");
    let member = |world: &SimWorld, client: &str| {
        world
            .snapshot()
            .admitted_clients
            .iter()
            .find(|entry| entry.id == client)
            .and_then(|entry| entry.connected_node.clone())
            .expect("connected")
    };
    world.add_client("a", NS).expect("a");
    let a_member = member(&world, "a");
    let b = (0..64)
        .map(|index| format!("b{index}"))
        .find(|name| {
            world.add_client(name, NS).expect("admitted");
            member(&world, name) != a_member
        })
        .expect("a writer on another member");
    let b_member = member(&world, &b);

    // Two writes of one key from two members, before either invalidation lands.
    world.push_event("a", NS, "p", "old").expect("a writes");
    world.push_event(&b, NS, "p", "new").expect("b writes");
    world.run(3);
    let snapshot = world.snapshot();
    let holders = snapshot
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .filter(|node| holds(&snapshot, "p", node) > 0)
        .collect::<Vec<_>>();
    assert_eq!(
        holders,
        [b_member],
        "the newer write survives where it was made"
    );
}
