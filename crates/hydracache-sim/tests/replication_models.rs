//! TD-0016 step 3: manual writes follow the product placement and
//! acknowledgement rules. `local_first` writes on the client's member and
//! invalidates the rest; `replicated` forwards to the rendezvous primary, which
//! replicates to its backups, and is acknowledged per the product
//! `ConsistencyLevel` or refused loud.

use hydracache::{rank_replicas, ClusterNodeId, ConsistencyLevel};
use hydracache_sim::{ControlActionV1, SimConfig, SimReplicationModel, SimSnapshot, SimWorld};

const NS: &str = "profiles";

fn world_with(model: SimReplicationModel) -> SimWorld {
    let mut world = SimWorld::new(0x5503, SimConfig::default());
    world.set_workload_enabled(false);
    world.run(8);
    world.set_replication(model).expect("model applies");
    world
}

fn replicated(rf: u64, level: ConsistencyLevel) -> SimReplicationModel {
    SimReplicationModel::Replicated {
        replication_factor: rf,
        write_level: level,
    }
}

fn version_on(snapshot: &SimSnapshot, key: &str, node: &str) -> u64 {
    snapshot
        .keys
        .iter()
        .find(|view| view.key == key)
        .and_then(|view| view.replicas.iter().find(|replica| replica.node_id == node))
        .map_or(0, |replica| replica.version)
}

fn nodes(snapshot: &SimSnapshot) -> Vec<String> {
    snapshot.nodes.iter().map(|node| node.id.clone()).collect()
}

fn connected(snapshot: &SimSnapshot, client: &str) -> String {
    snapshot
        .admitted_clients
        .iter()
        .find(|entry| entry.id == client)
        .and_then(|entry| entry.connected_node.clone())
        .expect("connected")
}

/// A key whose primary is not `coordinator`, so the write has to be forwarded.
fn key_not_owned_by(coordinator: &str, members: &[String], rf: usize) -> String {
    let ids = members
        .iter()
        .cloned()
        .map(ClusterNodeId::from)
        .collect::<Vec<_>>();
    (0..64)
        .map(|index| format!("k{index}"))
        .find(|key| {
            let replicas = rank_replicas(&format!("{NS}:{key}"), ids.iter(), rf).expect("owners");
            !replicas
                .all_nodes()
                .iter()
                .any(|node| node.as_str() == coordinator)
        })
        .expect("some key is owned elsewhere")
}

#[test]
fn local_first_writes_on_the_clients_member_and_invalidates_the_rest() {
    let mut world = world_with(SimReplicationModel::LocalFirst);
    world.add_client("writer", NS).expect("admitted");
    let member = connected(&world.snapshot(), "writer");
    world.push_event("writer", NS, "p", "v1").expect("push");
    world.run(3);
    let first = world.snapshot();
    let holders = nodes(&first)
        .into_iter()
        .filter(|node| version_on(&first, "profiles:p", node) > 0)
        .collect::<Vec<_>>();
    assert_eq!(
        holders,
        std::slice::from_ref(&member),
        "only the writer's member holds it"
    );

    let write = first.writes.last().expect("write recorded");
    assert_eq!(write.status, "acknowledged");
    assert_eq!(write.coordinator, member);
    assert_eq!(write.acknowledged_at_step, Some(write.started_at_step));
    assert_eq!(first.replication.model, "local_first");
    assert!(first.keys.iter().all(|key| key.owners.is_none()));
}

#[test]
fn replicated_writes_reach_only_the_rendezvous_owners_through_the_primary() {
    let mut world = world_with(replicated(2, ConsistencyLevel::All));
    world.add_client("writer", NS).expect("admitted");
    let snapshot = world.snapshot();
    let member = connected(&snapshot, "writer");
    let key = key_not_owned_by(&member, &nodes(&snapshot), 2);
    world.push_event("writer", NS, &key, "v1").expect("push");

    let started = world.snapshot();
    let write = started.writes.last().expect("write recorded").clone();
    assert_eq!(write.coordinator, member);
    assert_ne!(write.primary, member, "forwarded to an owner");
    assert_eq!(write.status, "pending", "acks travel over the network");
    assert_eq!(write.required_acks, 2);
    assert!(started
        .in_flight
        .iter()
        .any(|message| message.from == member
            && message.to == write.primary
            && message.kind == "replicate_put"));

    world.run(6);
    let settled = world.snapshot();
    let full_key = format!("{NS}:{key}");
    let owners = settled
        .keys
        .iter()
        .find(|view| view.key == full_key)
        .and_then(|view| view.owners.clone())
        .expect("owners reported");
    assert_eq!(owners.primary, write.primary);
    assert_eq!(owners.backups, write.backups);
    for node in nodes(&settled) {
        let owner = node == owners.primary || owners.backups.contains(&node);
        assert_eq!(version_on(&settled, &full_key, &node) > 0, owner, "{node}");
    }
    let done = settled.writes.last().expect("write");
    assert_eq!(done.status, "acknowledged");
    assert_eq!(done.acks, 2);
    assert_eq!(done.backups_applied, done.backups);
}

#[test]
fn quorum_is_refused_loud_when_too_few_owners_are_live_but_one_succeeds() {
    let mut world = world_with(replicated(3, ConsistencyLevel::Quorum));
    world.add_client("writer", NS).expect("admitted");
    let member = connected(&world.snapshot(), "writer");
    let others = nodes(&world.snapshot())
        .into_iter()
        .filter(|node| *node != member)
        .collect::<Vec<_>>();
    for node in &others {
        assert!(world.crash_node(node.clone()));
    }
    let error = world
        .push_event("writer", NS, "p", "v1")
        .unwrap_err()
        .to_string();
    assert!(error.starts_with("write refused at quorum:"), "{error}");
    assert!(
        error.contains("requires 2 live acknowledgements from 3 replicas, only 1 are live"),
        "{error}"
    );
    assert!(
        world.snapshot().writes.is_empty(),
        "a refused write never happened"
    );

    world
        .apply_control_action(ControlActionV1::SetReplication {
            at_step: 8,
            model: replicated(3, ConsistencyLevel::One),
        })
        .expect("level changes");
    world
        .push_event("writer", NS, "p", "v2")
        .expect("ONE is met");
    world.run(2);
    assert_eq!(
        world.snapshot().writes.last().expect("write").status,
        "acknowledged"
    );
}

#[test]
fn a_pending_write_fails_loud_when_its_owners_go_down() {
    let mut world = world_with(replicated(3, ConsistencyLevel::All));
    world.add_client("writer", NS).expect("admitted");
    let member = connected(&world.snapshot(), "writer");
    world.push_event("writer", NS, "p", "v1").expect("push");
    let other = nodes(&world.snapshot())
        .into_iter()
        .find(|node| *node != member)
        .expect("another node");
    assert!(world.crash_node(other));
    world.run(3);
    let write = world.snapshot().writes.last().expect("write").clone();
    assert_eq!(write.status, "failed");
    assert!(write
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("requires 3 live acknowledgements")));
}

#[test]
fn subscribers_hear_from_their_own_member() {
    // TD-0016 step 4: an event comes from the subscriber's member when that
    // member applies the write; a member that does not own the key never
    // stores it, so its subscribers get nothing for it.
    let mut world = world_with(replicated(2, ConsistencyLevel::All));
    world.add_client("writer", NS).expect("admitted");
    let snapshot = world.snapshot();
    let ids = nodes(&snapshot)
        .into_iter()
        .map(ClusterNodeId::from)
        .collect::<Vec<_>>();
    let owners = rank_replicas("profiles:p", ids.iter(), 2).expect("owners");
    let mut on_owner = None;
    let mut off_owner = None;
    for index in 0..64 {
        let name = format!("reader-{index}");
        world.subscribe(&name, NS).expect("reader");
        let member = connected(&world.snapshot(), &name);
        if owners
            .all_nodes()
            .iter()
            .any(|node| node.as_str() == member)
        {
            on_owner.get_or_insert(name);
        } else {
            off_owner.get_or_insert(name);
        }
        if on_owner.is_some() && off_owner.is_some() {
            break;
        }
    }
    let (on_owner, off_owner) = (on_owner.expect("on owner"), off_owner.expect("off owner"));
    world.push_event("writer", NS, "p", "v1").expect("push");
    world.run(6);
    let settled = world.snapshot();
    let event_of = |client: &str| {
        settled
            .subscribers
            .iter()
            .find(|subscriber| subscriber.client_id == client)
            .expect("subscriber")
            .clone()
    };
    let heard = event_of(&on_owner);
    let event = heard.last_event.expect("an owner's subscriber hears it");
    assert_eq!(
        (event.kind.as_str(), event.key.as_str()),
        ("upserted", "profiles:p")
    );
    let silent = event_of(&off_owner);
    assert!(silent.last_event.is_none());
    assert_eq!(silent.lag, 0, "the event left its queue");
}

#[test]
fn local_first_subscribers_on_other_members_hear_an_invalidation() {
    let mut world = world_with(SimReplicationModel::LocalFirst);
    world.add_client("writer", NS).expect("admitted");
    let writer_member = connected(&world.snapshot(), "writer");
    let reader = (0..64)
        .map(|index| format!("reader-{index}"))
        .find(|name| {
            world.subscribe(name, NS).expect("reader");
            connected(&world.snapshot(), name) != writer_member
        })
        .expect("a reader elsewhere");
    world.push_event("writer", NS, "p", "v1").expect("push");
    world.run(3);
    let snapshot = world.snapshot();
    let event = snapshot
        .subscribers
        .iter()
        .find(|subscriber| subscriber.client_id == reader)
        .and_then(|subscriber| subscriber.last_event.clone())
        .expect("heard");
    assert_eq!(event.kind, "invalidated");
}

#[test]
fn the_workload_and_routed_models_exclude_each_other() {
    let mut world = SimWorld::new(0x5506, SimConfig::default());
    let error = world
        .set_replication(SimReplicationModel::LocalFirst)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("turn the built-in workload off first"),
        "{error}"
    );
    world.set_workload_enabled(false);
    world
        .set_replication(SimReplicationModel::LocalFirst)
        .expect("ok");
    let error = world
        .apply_control_action(ControlActionV1::Workload {
            at_step: 0,
            enabled: true,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("full_broadcast"), "{error}");
    let error = world
        .set_replication(replicated(0, ConsistencyLevel::One))
        .unwrap_err()
        .to_string();
    assert!(error.contains("at least 1"), "{error}");
}

#[test]
fn routed_runs_replay_identically() {
    let mut world = SimWorld::new(0x5507, SimConfig::default());
    for action in [
        ControlActionV1::Workload {
            at_step: 0,
            enabled: false,
        },
        ControlActionV1::Step { at_step: 0, n: 8 },
        ControlActionV1::SetReplication {
            at_step: 8,
            model: replicated(2, ConsistencyLevel::Quorum),
        },
        ControlActionV1::PushEvent {
            at_step: 8,
            client: "writer".to_owned(),
            ns: NS.to_owned(),
            key: "p".to_owned(),
            value: "v1".to_owned(),
        },
        ControlActionV1::Step { at_step: 8, n: 4 },
    ] {
        world.apply_control_action(action).expect("action applies");
    }
    let json = world.replay_script().to_json();
    assert!(json.contains("\"type\":\"set_replication\""), "{json}");
    assert!(json.contains("\"kind\":\"replicated\""), "{json}");
    let replayed = hydracache_sim::replay_world(
        &hydracache_sim::ReplayScriptV1::from_json(&json).expect("script parses"),
        |seed| SimWorld::new(seed, SimConfig::default()),
    )
    .expect("script replays");
    assert_eq!(replayed.snapshot(), world.snapshot());
}

#[test]
fn a_down_primary_is_replaced_by_the_next_live_owner() {
    let mut world = world_with(replicated(2, ConsistencyLevel::One));
    world.add_client("writer", NS).expect("admitted");
    let snapshot = world.snapshot();
    let member = connected(&snapshot, "writer");
    let ids = nodes(&snapshot)
        .into_iter()
        .map(ClusterNodeId::from)
        .collect::<Vec<_>>();
    // A key whose natural primary is neither the writer's member nor a node
    // we need, so crashing it leaves the backup to take over.
    let (key, natural) = (0..64)
        .map(|index| format!("k{index}"))
        .find_map(|key| {
            let replicas = rank_replicas(&format!("{NS}:{key}"), ids.iter(), 2)?;
            (replicas.primary.as_str() != member).then_some((key, replicas))
        })
        .expect("a key owned elsewhere");
    assert!(world.crash_node(natural.primary.to_string()));
    world.push_event("writer", NS, &key, "v1").expect("push");
    world.run(4);
    let settled = world.snapshot();
    let write = settled.writes.last().expect("write");
    assert_eq!(
        write.primary,
        natural.backups[0].to_string(),
        "backup acts as primary"
    );
    assert_eq!(write.status, "acknowledged");
    let owners = settled
        .keys
        .iter()
        .find(|view| view.key == format!("{NS}:{key}"))
        .and_then(|view| view.owners.clone())
        .expect("owners");
    assert_eq!(
        owners.primary,
        natural.primary.to_string(),
        "placement is unchanged"
    );
}
