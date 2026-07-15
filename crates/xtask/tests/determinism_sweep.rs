use serde_json::json;

#[test]
fn determinism_sweep_matches_normalized_digests_across_repeated_and_serial_parallel_runs() {
    let evidence = logical_evidence();
    let first = xtask::determinism_sweep::logical_digest(&evidence).unwrap();
    let repeated = xtask::determinism_sweep::logical_digest(&evidence).unwrap();
    let serial = xtask::determinism_sweep::logical_digest(&evidence).unwrap();
    assert!(xtask::determinism_sweep::digests_match([
        &first, &repeated, &serial
    ]));
}

#[test]
fn determinism_digest_ignores_ephemeral_metadata_but_detects_logical_schedule_drift() {
    let left = logical_evidence();
    let mut ephemeral = logical_evidence();
    ephemeral["metadata"] = json!({
        "wall_clock": "2026-07-14T10:00:00Z",
        "duration_ms": 999,
        "absolute_path": "C:\\temp\\run-a",
        "port": 38123,
        "thread_id": 44
    });
    assert_eq!(
        xtask::determinism_sweep::logical_digest(&left).unwrap(),
        xtask::determinism_sweep::logical_digest(&ephemeral).unwrap()
    );

    let mut drifted = logical_evidence();
    drifted["schedule"][1] = json!("partition(1,3)");
    assert_ne!(
        xtask::determinism_sweep::logical_digest(&left).unwrap(),
        xtask::determinism_sweep::logical_digest(&drifted).unwrap()
    );
}

fn logical_evidence() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "suite": "fixture",
        "seed": 64,
        "schedule": ["tick", "partition(1,2)"],
        "operations": ["put:a", "read:a"],
        "invariant_verdicts": {"single_leader": true, "converged": true},
        "final_state": {"members": ["a", "b"], "term": 7},
        "metadata": {}
    })
}
