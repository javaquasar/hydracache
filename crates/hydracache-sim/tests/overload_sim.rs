use hydracache_sim::{
    FaultSchedule, LinkStateView, ResourceBudget, ScheduledFault, ScheduledFaultKind, SimConfig,
    SimWorld, MAX_SUBSCRIBER_BUFFER,
};

mod overload_sim {
    use super::*;

    #[test]
    fn high_rate_workload_with_partition_does_not_deadlock() {
        let mut world = SimWorld::new(
            0x58_30,
            SimConfig {
                key_count: 32,
                ..SimConfig::default()
            },
        );
        world.set_workload_enabled(false);
        world.set_resource_budget(ResourceBudget {
            max_storage_bytes: 1 << 20,
            max_network_in_flight: 50_000,
            max_client_in_flight: 64,
            max_subscriber_pending: MAX_SUBSCRIBER_BUFFER as u64,
            sample_window: 4,
        });
        world.subscribe("client-a", "ns");

        let schedule = FaultSchedule::from_faults(vec![
            ScheduledFault::new(
                8,
                ScheduledFaultKind::NetworkPartition {
                    from: "node-0".to_owned(),
                    to: "node-1".to_owned(),
                },
            ),
            ScheduledFault::new(
                64,
                ScheduledFaultKind::NetworkHeal {
                    from: "node-0".to_owned(),
                    to: "node-1".to_owned(),
                },
            ),
        ]);

        let mut manual_pushes = 0_u64;
        for step in 1..=96 {
            apply_scheduled_faults(&mut world, &schedule, step);
            for burst in 0..8 {
                world
                    .push_event(
                        "client-a",
                        "ns",
                        format!("key-{step}-{burst}"),
                        format!("value-{step}-{burst}"),
                    )
                    .expect("manual high-rate push succeeds");
                manual_pushes = manual_pushes.saturating_add(1);
            }
            world.step();
            assert!(
                world.invariant_report().is_ok(),
                "{:?}",
                world.invariant_report().violations
            );
        }

        let outcome = world.outcome();
        assert_eq!(outcome.steps, 96);
        assert_eq!(outcome.accepted_ops, manual_pushes);
        assert!(outcome.delivered_messages > 0, "{outcome:?}");

        let snapshot = world.snapshot();
        let healed = snapshot
            .links
            .iter()
            .find(|link| link.from == "node-0" && link.to == "node-1")
            .expect("scheduled link exists");
        assert_eq!(healed.state, LinkStateView::Up);
        assert!(snapshot
            .subscribers
            .iter()
            .all(|subscriber| subscriber.lag <= MAX_SUBSCRIBER_BUFFER as u64));
    }
}

fn apply_scheduled_faults(world: &mut SimWorld, schedule: &FaultSchedule, step: u64) {
    for fault in schedule.faults_at(step) {
        match &fault.kind {
            ScheduledFaultKind::NetworkPartition { from, to } => {
                assert!(world.partition_link(from.as_str(), to.as_str()));
            }
            ScheduledFaultKind::NetworkHeal { from, to } => {
                assert!(world.heal_link(from.as_str(), to.as_str()));
            }
            other => panic!("unexpected overload_sim fault: {other:?}"),
        }
    }
}
