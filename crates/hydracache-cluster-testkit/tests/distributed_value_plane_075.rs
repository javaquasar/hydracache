use hydracache_cluster_testkit::distributed_value_plane_075::{
    certainty_label, run_seeded_chaos_campaign, ChaosCampaignBounds, ClusterTimeObservation,
    DistributedValuePlaneSimulator, ExecutionFault, SimEventKind, SimulatorBounds, SimulatorError,
};
use hydracache_cluster_testkit::value_plane_model_075::{
    CanonicalMapKey, LogicalTime, MutationDigest, MutationIdentity, MutationOperation,
    MutationPlan, OutcomeCertainty, TtlDirective, ValuePlaneBounds,
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
fn lost_replica_ack_is_unknown_and_repair_allows_same_identity_to_finish() {
    let mut sim = simulator(16);
    let key = key(0, 1);
    let plan = put(1, key.clone(), b"v1", 1);
    sim.fail_node("node-b").unwrap();
    sim.record_write_availability(3);
    assert_eq!(
        sim.execute("node-a", plan.clone()),
        Err(SimulatorError::RequiredBackupUnavailable("node-b".into()))
    );
    assert_eq!(sim.node_value("node-a", &key).unwrap(), None);
    sim.restore_node("node-b").unwrap();
    let lost = sim
        .execute_with_fault(
            "node-a",
            plan.clone(),
            0,
            ExecutionFault::LoseReplicaAcknowledgement,
        )
        .unwrap();
    assert_eq!(lost.outcome.certainty, OutcomeCertainty::OutcomeUnknown);
    assert_eq!(
        sim.node_value("node-b", &key).unwrap(),
        Some(b"v1".to_vec())
    );
    assert_eq!(
        sim.execute("node-a", put(2, key.clone(), b"v2", 1)),
        Err(SimulatorError::ReplicaProofPending(0))
    );
    let again = sim
        .execute_with_fault(
            "node-a",
            plan.clone(),
            0,
            ExecutionFault::LoseReplicaAcknowledgement,
        )
        .unwrap();
    assert!(again.replayed);
    assert_eq!(again.outcome, lost.outcome);
    sim.record_write_availability(4);
    sim.fail_node("node-a").unwrap();
    sim.fail_node("node-b").unwrap();
    sim.record_write_availability(2);
    sim.restore_node("node-b").unwrap();
    let promoted = sim.promote_backup(0, 2).unwrap();
    sim.repair_backup(0, &promoted.backup).unwrap();
    let resumed = sim
        .execute(
            &promoted.owner,
            MutationPlan {
                expected_epoch: 2,
                ..plan
            },
        )
        .unwrap();
    assert!(resumed.replayed);
    assert_eq!(resumed.outcome.certainty, OutcomeCertainty::Certain);
    assert_eq!(resumed.outcome.version, lost.outcome.version);
    assert_eq!(
        sim.drain_events()
            .into_iter()
            .filter(|event| event.kind == SimEventKind::Mutation)
            .count(),
        1
    );
    assert_eq!(sim.unavailable_partition_seconds(), 16);
}

#[test]
fn only_current_owner_replicates_expiry_and_repair_preserves_tombstone() {
    let mut sim = simulator(16).with_expiry_tombstones();
    let key = key(0, 1);
    assert_eq!(
        simulator(16).expire("node-a", &key, MutationIdentity::new("expiry", 1), 5),
        Err(SimulatorError::ExpiryRequiresOwnerCheck)
    );
    sim.observe_node_time(
        "node-b",
        ClusterTimeObservation {
            now: 5,
            uncertainty: 1,
        },
    )
    .unwrap();
    let mut plan = put(1, key.clone(), b"v1", 1);
    plan.operation = MutationOperation::Put {
        value: b"v1".to_vec(),
        ttl: TtlDirective::ExpireAfter(10),
    };
    sim.execute("node-a", plan).unwrap();
    sim.advance_logical_time(25);
    let expiry = MutationIdentity::new("expiry", 1);
    sim.observe_node_time(
        "node-a",
        ClusterTimeObservation {
            now: 14,
            uncertainty: 5,
        },
    )
    .unwrap();
    assert_eq!(sim.expire("node-a", &key, expiry.clone(), 5).unwrap(), None);
    assert_eq!(sim.read("node-a", &key, 0).unwrap(), Some(b"v1".to_vec()));
    assert!(matches!(
        sim.expire("node-b", &key, expiry.clone(), 5),
        Err(SimulatorError::StaleOwner { .. })
    ));
    let unchecked = MutationPlan::new(
        expiry.clone(),
        MutationDigest::new(0),
        key.clone(),
        MutationOperation::ExpireVersion {
            version: 1,
            deadline: LogicalTime::new(10),
        },
        1,
    );
    assert_eq!(
        sim.execute("node-a", unchecked),
        Err(SimulatorError::ExpiryRequiresOwnerCheck)
    );

    sim.isolate_node("node-a").unwrap();
    let promoted = sim.promote_backup(0, 2).unwrap();
    sim.observe_node_time(
        "node-b",
        ClusterTimeObservation {
            now: 11,
            uncertainty: 1,
        },
    )
    .unwrap();
    sim.repair_backup(0, &promoted.backup).unwrap();
    let outcome = sim
        .expire("node-b", &key, expiry.clone(), 5)
        .unwrap()
        .unwrap();
    assert_eq!(outcome.outcome.version, Some(2));
    assert_eq!(sim.expire("node-b", &key, expiry, 5).unwrap(), None);
    assert_eq!(sim.node_value(&promoted.backup, &key).unwrap(), None);
    assert_eq!(
        sim.node_value("node-a", &key).unwrap(),
        Some(b"v1".to_vec())
    );
    sim.heal_node("node-a").unwrap();
    sim.catch_up_node("node-a").unwrap();
    sim.repair_backup(0, "node-a").unwrap();
    assert_eq!(sim.node_value("node-a", &key).unwrap(), None);
    assert_eq!(sim.read("node-a", &key, 0).unwrap(), None);
    assert_eq!(
        sim.drain_events()
            .into_iter()
            .filter(|event| event.kind == SimEventKind::Mutation)
            .count(),
        2
    );
}

#[test]
fn isolated_old_owner_cannot_serve_reads() {
    let mut sim = simulator(16);
    let key = key(0, 1);
    let old = sim.assignment(0).unwrap().owner.clone();
    sim.execute(&old, put(1, key.clone(), b"v1", 1)).unwrap();
    let owner_read = sim.begin_read(&old, &key, 0).unwrap();
    let forwarded_before_promotion = sim.begin_read("node-c", &key, 0).unwrap();
    let forwarded_read = sim.begin_read("node-c", &key, 0).unwrap();

    sim.isolate_node(&old).unwrap();
    assert_eq!(sim.node_value(&old, &key).unwrap(), Some(b"v1".to_vec()));
    assert_eq!(
        sim.finish_read(owner_read),
        Err(SimulatorError::NodeIsolated(old.clone()))
    );
    assert_eq!(
        sim.read("node-c", &key, 0),
        Err(SimulatorError::NodeIsolated(old.clone()))
    );
    assert_eq!(
        sim.finish_read(forwarded_before_promotion),
        Err(SimulatorError::NodeIsolated(old.clone()))
    );
    assert_eq!(
        sim.execute(&old, put(2, key.clone(), b"v2", 1)),
        Err(SimulatorError::NodeIsolated(old.clone()))
    );

    let promoted = sim.promote_backup(0, 2).unwrap();
    assert_eq!(
        sim.read(&promoted.owner, &key, 0).unwrap(),
        Some(b"v1".to_vec())
    );
    assert_eq!(
        sim.finish_read(forwarded_read),
        Err(SimulatorError::StaleEpoch {
            expected: 1,
            actual: 2
        })
    );
    sim.repair_backup(0, &promoted.backup).unwrap();
    sim.execute(&promoted.owner, put(2, key.clone(), b"v2", 2))
        .unwrap();
    assert_eq!(sim.node_value(&old, &key).unwrap(), Some(b"v1".to_vec()));
    sim.fail_node(&old).unwrap();
    sim.restore_node(&old).unwrap();
    assert_eq!(
        sim.catch_up_node(&old),
        Err(SimulatorError::NodeIsolated(old.clone()))
    );

    sim.heal_node(&old).unwrap();
    assert_eq!(
        sim.read(&old, &key, 0),
        Err(SimulatorError::StaleEpoch {
            expected: 1,
            actual: 2
        })
    );
    sim.catch_up_node(&old).unwrap();
    sim.catch_up_node(&old).unwrap();
    assert_eq!(sim.read(&old, &key, 0).unwrap(), Some(b"v2".to_vec()));
    assert_eq!(
        sim.read(&old, &key, 1),
        Err(SimulatorError::ProxyHopLimit {
            limit: 1,
            actual: 1
        })
    );
}

#[test]
fn quorum_loss_refuses_reads_and_unproved_backup_cannot_be_promoted() {
    let mut sim = simulator(16);
    let key = key(0, 1);
    let initial = sim.assignment(0).unwrap().clone();
    let pending = sim.begin_read(&initial.owner, &key, 0).unwrap();
    sim.isolate_node("node-c").unwrap();
    sim.isolate_node(&initial.backup).unwrap();
    let quorum_error = SimulatorError::AuthorityQuorumUnavailable {
        reachable: 1,
        required: 2,
    };
    assert_eq!(sim.read(&initial.owner, &key, 0), Err(quorum_error.clone()));
    assert_eq!(sim.finish_read(pending), Err(quorum_error.clone()));
    assert_eq!(sim.promote_backup(0, 2), Err(quorum_error));
    assert_eq!(sim.epoch(), 1);

    sim.heal_node("node-c").unwrap();
    assert_eq!(sim.read(&initial.owner, &key, 0).unwrap(), None);
    assert_eq!(
        sim.promote_backup(0, 2),
        Err(SimulatorError::NoPromotionCandidate)
    );
    assert_eq!(
        sim.execute(&initial.owner, put(1, key.clone(), b"v1", 1)),
        Err(SimulatorError::RequiredBackupUnavailable(
            initial.backup.clone()
        ))
    );
    assert_eq!(sim.node_value(&initial.owner, &key).unwrap(), None);

    sim.heal_node(&initial.backup).unwrap();
    sim.isolate_node(&initial.owner).unwrap();
    let promoted = sim.promote_backup(0, 2).unwrap();
    sim.heal_node(&initial.owner).unwrap();
    sim.catch_up_node(&initial.owner).unwrap();
    sim.fail_node(&promoted.owner).unwrap();
    assert_eq!(
        sim.heal_node(&promoted.owner),
        Err(SimulatorError::NodeUnavailable(promoted.owner))
    );
    assert_eq!(
        sim.promote_backup(0, 3),
        Err(SimulatorError::NoPromotionCandidate)
    );
    assert_eq!(sim.epoch(), 2);
    assert_eq!(
        sim.isolate_node("unknown"),
        Err(SimulatorError::UnknownNode("unknown".into()))
    );
}

#[test]
fn expiry_eligibility_uses_current_owner_clock_and_its_conservative_bound() {
    let mut sim = simulator(16);
    sim.observe_node_time(
        "node-a",
        ClusterTimeObservation {
            now: 9,
            uncertainty: 10,
        },
    )
    .unwrap();
    sim.observe_node_time(
        "node-b",
        ClusterTimeObservation {
            now: 50,
            uncertainty: 1,
        },
    )
    .unwrap();
    assert!(!sim.expiry_due("node-a", 0, 1, 10).unwrap());
    assert_eq!(
        sim.expiry_due("node-a", 0, 1, 9),
        Err(SimulatorError::ClockUncertaintyExceeded {
            limit: 9,
            actual: 10
        })
    );
    assert_eq!(
        sim.expiry_due("node-b", 0, 20, 10),
        Err(SimulatorError::StaleOwner {
            expected: "node-a".into(),
            actual: "node-b".into(),
        })
    );
    assert_eq!(
        sim.expiry_due("node-a", 3, 20, 10),
        Err(SimulatorError::UnknownPartition(3))
    );

    sim.advance_logical_time(21);
    assert!(sim.expiry_due("node-a", 0, 20, 10).unwrap());
    assert!(!sim.expiry_due("node-a", 0, 21, 10).unwrap());
    sim.observe_node_time(
        "node-b",
        ClusterTimeObservation {
            now: 20,
            uncertainty: 1,
        },
    )
    .unwrap();
    sim.isolate_node("node-a").unwrap();
    assert_eq!(
        sim.expiry_due("node-a", 0, 20, 10),
        Err(SimulatorError::NodeIsolated("node-a".into()))
    );
    let promoted = sim.promote_backup(0, 2).unwrap();
    assert!(!sim.expiry_due(&promoted.owner, 0, 20, 10).unwrap());
    sim.advance_logical_time(1);
    assert!(sim.expiry_due(&promoted.owner, 0, 20, 10).unwrap());
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

#[test]
fn partition_transfer_preserves_unrelated_partition_state() {
    let mut sim = simulator(16);
    let partition_zero = key(0, 1);
    let partition_one = key(1, 1);
    let p0 = sim.assignment(0).unwrap().clone();
    let p1 = sim.assignment(1).unwrap().clone();
    sim.execute(&p0.owner, put(1, partition_zero.clone(), b"p0", 1))
        .unwrap();
    sim.execute(&p1.owner, put(2, partition_one.clone(), b"p1", 1))
        .unwrap();

    sim.rebalance(0, &p0.backup, 2).unwrap();

    assert_eq!(
        sim.read(&p1.owner, &partition_one, 0).unwrap(),
        Some(b"p1".to_vec())
    );
    assert_eq!(
        sim.node_value(&p1.backup, &partition_one).unwrap(),
        Some(b"p1".to_vec())
    );
    assert!(sim
        .invariant_violations(0, &[partition_zero.clone(), partition_one.clone()])
        .is_empty());
    assert!(sim
        .invariant_violations(1, &[partition_zero, partition_one])
        .is_empty());
}

#[test]
fn seeded_chaos_is_deterministic_and_safe() {
    let bounds = ChaosCampaignBounds {
        steps: 128,
        max_trace_events: 128,
    };
    let first = run_seeded_chaos_campaign(0x75, bounds).unwrap();
    let replay = run_seeded_chaos_campaign(0x75, bounds).unwrap();
    assert!(first.passed(), "violations: {:?}", first.violations);
    assert_eq!(first.trace, replay.trace);
    assert_eq!(first.trace_fingerprint, replay.trace_fingerprint);
    assert_eq!(first.steps_completed, 128);
}

#[test]
fn seeded_chaos_sweep_preserves_invariants() {
    let mut covered = std::collections::BTreeSet::new();
    for seed in 0..128 {
        let report = run_seeded_chaos_campaign(
            seed,
            ChaosCampaignBounds {
                steps: 64,
                max_trace_events: 64,
            },
        )
        .unwrap();
        assert!(
            report.passed(),
            "seed {seed} violations: {:?}; trace: {:?}",
            report.violations,
            report.trace
        );
        for event in report.trace {
            covered.insert(event.split(':').next().unwrap().to_owned());
        }
    }
    for required in [
        "put",
        "put-if-absent",
        "replace-if-present",
        "replace-if-value",
        "get-and-put",
        "remove-if-value",
        "get-and-remove",
        "advance-time-expiry",
    ] {
        assert!(
            covered.contains(required),
            "missing chaos action {required}"
        );
    }
}

#[test]
fn chaos_campaign_bounds_fail_closed() {
    assert_eq!(
        run_seeded_chaos_campaign(
            1,
            ChaosCampaignBounds {
                steps: 0,
                max_trace_events: 1,
            }
        ),
        Err(SimulatorError::InvalidBound("chaos_steps"))
    );
    assert_eq!(
        run_seeded_chaos_campaign(
            1,
            ChaosCampaignBounds {
                steps: 2,
                max_trace_events: 1,
            }
        ),
        Err(SimulatorError::InvalidBound("chaos_trace_events"))
    );
}
