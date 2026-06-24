use hydracache::LogicalTime;
use hydracache_sim::{
    History, SimConfig, SimSnapshot, SimWorld, VerdictView, WorkloadOp, WorkloadResult,
    SIM_SNAPSHOT_SCHEMA_VERSION,
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

    let future = serde_json::json!({
        "schema_version": SIM_SNAPSHOT_SCHEMA_VERSION + 1,
        "seed": 1,
        "step": 0,
        "logical_time_millis": 0,
        "nodes": [],
        "links": [],
        "keys": [],
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
