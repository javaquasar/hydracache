use hydracache::LogicalTime;
use hydracache_sim::{
    History, SimConfig, SimSnapshot, SimWorld, VerdictView, WorkloadOp, WorkloadResult,
    MAX_IN_FLIGHT_RENDERED, SIM_SNAPSHOT_SCHEMA_VERSION,
};

#[test]
fn snapshot_roundtrips_and_is_versioned() {
    let mut world = SimWorld::new(0x50_02, SimConfig::default());
    world.run(12);

    let snapshot = world.snapshot();
    let encoded = snapshot.to_json();
    let decoded = SimSnapshot::from_json(&encoded).expect("current snapshot version decodes");

    assert_eq!(decoded, snapshot);
    assert_eq!(decoded.schema_version, SIM_SNAPSHOT_SCHEMA_VERSION);
    assert_eq!(decoded.step, 12);
    assert_eq!(decoded.nodes.len(), 3);
    assert_eq!(decoded.links.len(), 6);
    assert_eq!(decoded.schema_version, 6);
    assert_eq!(decoded.formation_phase, "formed");
    assert_eq!(decoded.election_source, "sim-model");
    assert!(decoded.over_budget.in_flight_summarized <= decoded.in_flight.len() as u64);
    assert!(decoded
        .election_disclosure
        .contains("not a product consensus claim"));
    assert!(decoded.nodes.iter().any(|node| {
        node.vote_state == "leader" && node.votes_received >= 2 && node.voted_for.is_some()
    }));

    let future = serde_json::json!({
            "schema_version": SIM_SNAPSHOT_SCHEMA_VERSION + 1,
            "seed": 1,
            "step": 0,
            "logical_time_millis": 0,
            "formation_phase": "formed",
            "election_source": "sim-model",
            "election_disclosure": "deterministic simulator election model",
            "nodes": [],
        "links": [],
        "in_flight": [],
        "over_budget": { "in_flight_summarized": 0 },
        "keys": [],
        "clients": [],
        "subscribers": [],
        "sync_progress": [],
        "rebalance": null,
        "mode": "manual",
        "active_scenario": null,
        "intervention_count": 0,
        "verdict": { "status": "holding" },
        "progress": {
            "committed_entries": 0,
            "last_leader_change": null,
            "convergence": "converged"
        }
    });
    let error = SimSnapshot::from_json(&future.to_string()).expect_err("future schema fails loud");
    assert!(error
        .to_string()
        .contains("unsupported simulator snapshot schema version"));
}

#[test]
fn schema_version_matches_contract_for_each_field_set() {
    let mut world = SimWorld::new(0x53_04, SimConfig::default());
    world.set_workload_enabled(false);
    let before = world.snapshot();
    assert_eq!(before.schema_version, 6);
    assert_eq!(before.formation_phase, "unformed");
    assert_eq!(before.election_source, "sim-model");
    assert!(before.in_flight.is_empty());
    assert_eq!(before.over_budget.in_flight_summarized, 0);
    assert!(before.clients.is_empty());
    assert!(before.subscribers.is_empty());
    assert!(before.rebalance.is_none());
    assert_eq!(before.mode, "manual");
    assert_eq!(before.active_scenario, None);
    assert_eq!(before.intervention_count, 0);
    assert!(before
        .sync_progress
        .iter()
        .all(|sync| sync.applied_index == 0));
    assert!(before.nodes.iter().all(|node| {
        node.vote_state == "disconnected"
            && node.voted_for.is_none()
            && node.votes_received == 0
            && !node.disabled
    }));

    world.run(8);
    let formed = world.snapshot();
    assert_eq!(formed.schema_version, 6);
    assert_eq!(formed.formation_phase, "formed");
    assert!(formed.nodes.iter().any(|node| {
        node.vote_state == "leader" && node.voted_for.as_deref() == Some(node.id.as_str())
    }));
}

// v6: manual clients and subscribers expose the cluster node they are routed to,
// so the demo can draw the client/subscriber connection.
#[test]
fn manual_client_and_subscriber_expose_connected_node() {
    let mut world = SimWorld::new(0x53_06, SimConfig::default());
    world.run(8);
    world.subscribe("client-a", "profiles");
    world
        .push_event("client-a", "profiles", "profile-1", "fresh")
        .expect("manual push applies");

    let snapshot = world.snapshot();
    let node_ids = snapshot
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();

    let client = snapshot
        .clients
        .iter()
        .find(|client| client.id == "client-a")
        .expect("client-a present");
    let connected = client
        .connected_node
        .as_deref()
        .expect("client routes to a live node");
    assert!(node_ids.iter().any(|id| id == connected));
    assert!(snapshot.subscribers.iter().all(|subscriber| subscriber
        .connected_node
        .as_deref()
        .is_some_and(|node| node_ids.iter().any(|id| id == node))));
}

#[test]
fn snapshot_exposes_typed_in_flight_messages() {
    let mut world = SimWorld::new(0x53_20, SimConfig::default());
    world.set_workload_enabled(false);
    assert!(world.delay_next_on_link_millis("node-0", "node-1", 250));

    world.step();
    let snapshot = world.snapshot();

    assert_eq!(snapshot.schema_version, 6);
    assert!(snapshot.in_flight.iter().any(|message| {
        message.kind == "heartbeat"
            && message.from == "node-0"
            && message.to == "node-1"
            && message.sequence.is_some()
            && message.remaining_millis > 0
    }));
}

#[test]
fn vote_messages_are_visible_during_election() {
    let mut world = SimWorld::new(0x53_21, SimConfig::default());
    world.set_workload_enabled(false);

    for _ in 0..8 {
        world.step();
        let snapshot = world.snapshot();
        if snapshot.formation_phase == "electing" {
            assert!(snapshot
                .in_flight
                .iter()
                .any(|message| message.kind == "vote_request"));
            assert!(snapshot
                .in_flight
                .iter()
                .any(|message| message.kind == "vote_response"));
            return;
        }
    }

    panic!("expected deterministic run to expose an electing snapshot");
}

#[test]
fn in_flight_is_bounded_and_over_budget_is_counted() {
    let cfg = SimConfig {
        node_count: 10,
        ..SimConfig::default()
    };
    let mut world = SimWorld::new(0x53_22, cfg);
    world.set_workload_enabled(false);
    for from in 0..10 {
        for to in 0..10 {
            if from != to {
                assert!(world.delay_next_on_link_millis(
                    format!("node-{from}"),
                    format!("node-{to}"),
                    1_000,
                ));
            }
        }
    }

    world.step();
    let snapshot = world.snapshot();

    assert_eq!(snapshot.in_flight.len(), MAX_IN_FLIGHT_RENDERED);
    assert!(snapshot.over_budget.in_flight_summarized > 0);
}

#[test]
fn cold_start_drives_disconnected_to_connected_via_fsm() {
    let mut world = SimWorld::new(0x53_05, SimConfig::default());
    world.set_workload_enabled(false);

    let mut phases = vec![world.snapshot().formation_phase];
    for _ in 0..8 {
        world.step();
        phases.push(world.snapshot().formation_phase);
    }

    assert_eq!(phases.first().map(String::as_str), Some("unformed"));
    assert!(phases.iter().any(|phase| phase == "bootstrapping"));
    assert!(phases.iter().any(|phase| phase == "electing"));
    assert_eq!(phases.last().map(String::as_str), Some("formed"));
}

#[test]
fn verdict_reflects_real_checker() {
    let mut history = History::new();
    let put = history.record_invocation(
        1,
        WorkloadOp::Put {
            key: "profile:42".to_owned(),
            value: b"fresh".to_vec(),
        },
        LogicalTime::from_millis(1),
    );
    history.record_response(
        put,
        LogicalTime::from_millis(2),
        WorkloadResult::Accepted { sequence: 1 },
    );
    let read = history.record_invocation(
        1,
        WorkloadOp::Get {
            key: "profile:42".to_owned(),
        },
        LogicalTime::from_millis(3),
    );
    history.record_response(
        read,
        LogicalTime::from_millis(4),
        WorkloadResult::Value(Some(b"stale".to_vec())),
    );

    let snapshot = SimSnapshot::from_history(99, 2, &history);

    assert!(matches!(
        snapshot.verdict,
        VerdictView::Violated { ref invariant, .. } if invariant == "read-your-writes"
    ));
    assert_eq!(
        snapshot.progress.convergence,
        hydracache_sim::ConvergenceView::Diverged
    );
}
