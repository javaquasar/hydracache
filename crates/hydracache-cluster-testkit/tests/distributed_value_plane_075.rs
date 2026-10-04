use hydracache_cluster_testkit::distributed_value_plane_075::{
    certainty_label, DistributedValuePlaneSimulator, ExecutionFault, SimEventKind, SimulatorBounds,
    SimulatorError,
};
use hydracache_cluster_testkit::value_plane_model_075::{
    CanonicalMapKey, MutationDigest, MutationIdentity, MutationOperation, MutationPlan,
    OutcomeCertainty, TtlDirective, ValuePlaneBounds,
};

fn simulator(listener_events: usize) -> DistributedValuePlaneSimulator {
    DistributedValuePlaneSimulator::new(
        ["node-a", "node-b", "node-c"],
        SimulatorBounds {
            partitions: 3,
            max_proxy_hops: 1,
            max_bulk_items: 4,
            max_listener_events: listener_events,
        },
        ValuePlaneBounds::default(),
    )
    .unwrap()
}

fn key(partition: u32, suffix: u8) -> CanonicalMapKey {
    CanonicalMapKey::new("tenant-a", "orders", 1, vec![b'k', suffix], partition)
}

fn put(sequence: u64, key: CanonicalMapKey, value: &[u8], epoch: u64) -> MutationPlan {
    MutationPlan::new(
        MutationIdentity::new("client-a", sequence),
        MutationDigest::new(sequence * 17 + u64::from(value[0])),
        key,
        MutationOperation::Put {
            value: value.to_vec(),
            ttl: TtlDirective::Eternal,
        },
        epoch,
    )
}

#[test]
fn non_owner_proxies_once_and_ack_requires_equal_backup_state() {
    let mut sim = simulator(16);
    let key = key(0, 1);
    let assignment = sim.assignment(0).unwrap().clone();
    let entry = ["node-a", "node-b", "node-c"]
        .into_iter()
        .find(|node| *node != assignment.owner)
        .unwrap();
    let result = sim.execute(entry, put(1, key.clone(), b"v1", 1)).unwrap();
    assert!(result.proxied);
    assert_eq!(result.owner, assignment.owner);
    assert_eq!(result.outcome.certainty, OutcomeCertainty::Certain);
    assert_eq!(
        sim.node_value(&assignment.owner, &key).unwrap(),
        Some(b"v1".to_vec())
    );
    assert_eq!(
        sim.node_value(&assignment.backup, &key).unwrap(),
        Some(b"v1".to_vec())
    );
}

#[test]
fn missing_required_backup_rejects_before_owner_apply() {
    let mut sim = simulator(16);
    let key = key(0, 1);
    let assignment = sim.assignment(0).unwrap().clone();
    sim.fail_node(&assignment.backup).unwrap();
    assert_eq!(
        sim.execute(&assignment.owner, put(1, key.clone(), b"v1", 1)),
        Err(SimulatorError::RequiredBackupUnavailable(
            assignment.backup.clone()
        ))
    );
    assert_eq!(sim.node_value(&assignment.owner, &key).unwrap(), None);
}

#[test]
fn response_loss_retry_returns_retained_outcome_without_duplicate_event() {
    let mut sim = simulator(16);
    let key = key(1, 1);
    let plan = put(7, key, b"v1", 1);
    let first = sim
        .execute_with_fault(
            "node-c",
            plan.clone(),
            0,
            ExecutionFault::LoseResponseAfterAcknowledgement,
        )
        .unwrap();
    assert_eq!(certainty_label(first.outcome.certainty), "outcome_unknown");
    let replay = sim.execute("node-c", plan).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.outcome, first.outcome);
    assert_eq!(
        sim.drain_events()
            .into_iter()
            .filter(|event| event.kind == SimEventKind::Mutation)
            .count(),
        1
    );
}

#[test]
fn promotion_blocks_writes_until_new_backup_is_repaired() {
    let mut sim = simulator(16);
    let key = key(0, 1);
    let initial = sim.assignment(0).unwrap().clone();
    sim.execute(&initial.owner, put(1, key.clone(), b"v1", 1))
        .unwrap();
    sim.fail_node(&initial.owner).unwrap();
    let promoted = sim.promote_backup(0, 2).unwrap();
    assert_eq!(promoted.owner, initial.backup);
    assert_eq!(sim.read("node-c", &key, 0).unwrap(), Some(b"v1".to_vec()));
    assert!(matches!(
        sim.execute(&promoted.owner, put(2, key.clone(), b"v2", 2)),
        Err(SimulatorError::RequiredBackupUnavailable(_))
    ));
    sim.repair_backup(0, &promoted.backup).unwrap();
    sim.execute(&promoted.owner, put(2, key.clone(), b"v2", 2))
        .unwrap();
    assert_eq!(sim.read("node-c", &key, 0).unwrap(), Some(b"v2".to_vec()));
}

#[test]
fn rebalance_copies_state_before_epoch_cutover() {
    let mut sim = simulator(16);
    let key = key(0, 1);
    let initial = sim.assignment(0).unwrap().clone();
    sim.execute(&initial.owner, put(1, key.clone(), b"v1", 1))
        .unwrap();
    let target = ["node-a", "node-b", "node-c"]
        .into_iter()
        .find(|node| *node != initial.owner && *node != initial.backup)
        .unwrap();
    let moved = sim.rebalance(0, target, 2).unwrap();
    assert_eq!(moved.owner, target);
    assert_eq!(
        sim.read(&initial.owner, &key, 0).unwrap(),
        Some(b"v1".to_vec())
    );
    assert!(matches!(
        sim.execute(target, put(2, key, b"v2", 1)),
        Err(SimulatorError::StaleEpoch { actual: 2, .. })
    ));
}

#[test]
fn bounded_bulk_retains_input_order_and_explicit_partial_failures() {
    let mut sim = simulator(16);
    let results = sim
        .execute_bulk(
            "node-c",
            vec![put(1, key(0, 1), b"a", 1), put(2, key(99, 2), b"b", 1)],
        )
        .unwrap();
    assert_eq!(results[0].input_index, 0);
    assert!(results[0].result.is_ok());
    assert_eq!(results[1].input_index, 1);
    assert_eq!(results[1].result, Err(SimulatorError::UnknownPartition(99)));
}

#[test]
fn listener_overflow_emits_one_gap_instead_of_unbounded_events() {
    let mut sim = simulator(2);
    for sequence in 1..=3 {
        sim.execute(
            "node-c",
            put(sequence, key(0, sequence as u8), &[sequence as u8], 1),
        )
        .unwrap();
    }
    let events = sim.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, SimEventKind::Gap);
    assert_eq!(sim.dropped_event_count(), 2);
}

#[test]
fn proxy_loops_and_unbounded_shapes_fail_closed() {
    let mut sim = simulator(16);
    let assignment = sim.assignment(0).unwrap().clone();
    let proxy = ["node-a", "node-b", "node-c"]
        .into_iter()
        .find(|node| *node != assignment.owner)
        .unwrap();
    assert_eq!(
        sim.execute_with_fault(proxy, put(1, key(0, 1), b"v", 1), 1, ExecutionFault::None),
        Err(SimulatorError::ProxyHopLimit {
            limit: 1,
            actual: 1
        })
    );
    let oversized = (0..5)
        .map(|index| put(index + 1, key(0, index as u8), b"v", 1))
        .collect();
    assert_eq!(
        sim.execute_bulk("node-a", oversized),
        Err(SimulatorError::BulkLimit {
            limit: 4,
            actual: 5
        })
    );
}
