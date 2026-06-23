use hydracache::{ClusterNodeId, ClusterNodeMessage, LogicalDuration, LogicalTime};
use hydracache_sim::{LinkFault, PartitionSymmetry, SimNetwork};

#[test]
fn network_same_seed_same_delivery_order() {
    let mut left = seeded_network();
    let mut right = seeded_network();
    let now = LogicalTime::from_millis(0);

    for network in [&mut left, &mut right] {
        network.inject_recoverable_fault_from_rng("a", "b");
        network.inject_recoverable_fault_from_rng("a", "b");
        network.send(node("a"), node("b"), heartbeat(1), now);
        network.send(node("a"), node("b"), heartbeat(2), now);
    }

    assert_eq!(
        left.deliverable(LogicalTime::from_millis(10)),
        right.deliverable(LogicalTime::from_millis(10))
    );
}

#[test]
fn network_symmetric_and_asymmetric_partition() {
    let mut network = seeded_network();
    let left = [node("a")];
    let right = [node("b")];

    network.partition((&left, &right), PartitionSymmetry::Symmetric);
    assert!(!network.can_deliver(&node("a"), &node("b")));
    assert!(!network.can_deliver(&node("b"), &node("a")));

    network.heal();
    network.partition((&left, &right), PartitionSymmetry::LeftToRight);
    assert!(!network.can_deliver(&node("a"), &node("b")));
    assert!(network.can_deliver(&node("b"), &node("a")));
}

#[test]
fn network_heal_drains_in_flight() {
    let mut network = seeded_network();
    let now = LogicalTime::from_millis(0);

    network.inject_link_fault("a", "b", LinkFault::Delay(LogicalDuration::from_millis(5)));
    network.send(node("a"), node("b"), heartbeat(7), now);
    assert_eq!(network.in_flight_len(), 1);

    network.partition((&[node("a")], &[node("b")]), PartitionSymmetry::Symmetric);
    assert!(network.deliverable(LogicalTime::from_millis(5)).is_empty());
    assert_eq!(network.in_flight_len(), 0);

    network.inject_link_fault("a", "b", LinkFault::Delay(LogicalDuration::from_millis(5)));
    network.heal();
    network.send(node("a"), node("b"), heartbeat(8), now);
    assert_eq!(
        network.deliverable(LogicalTime::from_millis(5)),
        vec![(node("a"), node("b"), heartbeat(8))]
    );
}

#[test]
fn network_duplicate_and_reorder_are_deterministic() {
    let mut network = seeded_network();
    let now = LogicalTime::from_millis(0);

    network.inject_link_fault("a", "b", LinkFault::Reorder);
    network.inject_link_fault("a", "b", LinkFault::Duplicate);
    network.send(node("a"), node("b"), heartbeat(1), now);
    network.send(node("a"), node("b"), heartbeat(2), now);

    assert_eq!(
        network.deliverable(now),
        vec![
            (node("a"), node("b"), heartbeat(2)),
            (node("a"), node("b"), heartbeat(2))
        ]
    );
    assert_eq!(
        network.deliverable(LogicalTime::from_millis(1)),
        vec![(node("a"), node("b"), heartbeat(1))]
    );
}

fn seeded_network() -> SimNetwork {
    SimNetwork::from_seed(44)
}

fn node(id: &str) -> ClusterNodeId {
    ClusterNodeId::from(id)
}

fn heartbeat(sequence: u64) -> ClusterNodeMessage {
    ClusterNodeMessage::Heartbeat {
        at: LogicalTime::from_millis(sequence),
        sequence,
    }
}
