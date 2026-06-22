mod support;

use std::time::Duration;

use hydracache::ClusterNodeId;
use support::fault_injector::{Fault, FaultInjector};

#[test]
fn seed_replays_identical_fault_schedule() {
    let nodes = vec![
        ClusterNodeId::from("member-a"),
        ClusterNodeId::from("member-b"),
        ClusterNodeId::from("member-c"),
    ];
    let mut first = FaultInjector::new(42);
    let mut second = FaultInjector::new(42);

    let first_faults = (0..8).map(|_| first.next_fault(&nodes)).collect::<Vec<_>>();
    let second_faults = (0..8)
        .map(|_| second.next_fault(&nodes))
        .collect::<Vec<_>>();

    assert_eq!(first_faults, second_faults);
}

#[test]
fn partition_is_symmetric_and_asymmetric() {
    let a = ClusterNodeId::from("member-a");
    let b = ClusterNodeId::from("member-b");
    let mut injector = FaultInjector::new(1);

    injector.partition(a.clone(), b.clone(), false);
    assert!(!injector.can_deliver(&a, &b));
    assert!(injector.can_deliver(&b, &a));

    let mut injector = FaultInjector::new(1);
    injector.partition(a.clone(), b.clone(), true);
    assert!(!injector.can_deliver(&a, &b));
    assert!(!injector.can_deliver(&b, &a));
}

#[test]
fn injected_latency_is_observed_by_target_only() {
    let a = ClusterNodeId::from("member-a");
    let b = ClusterNodeId::from("member-b");
    let mut injector = FaultInjector::new(1);

    injector.inject_latency(a.clone(), Duration::from_millis(25));
    let _fault = Fault::Latency {
        node: a.clone(),
        latency: Duration::from_millis(25),
    };

    assert_eq!(injector.observed_latency(&a), Duration::from_millis(25));
    assert_eq!(injector.observed_latency(&b), Duration::ZERO);
}
