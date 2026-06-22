use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

mod support;
use support::fault_injector::{Fault, FaultInjector};

use hydracache::{
    ClusterEndpoints, ClusterEpoch, ClusterGeneration, ClusterMember, ClusterNodeId,
    ClusterReplicationStrategy, ClusterRole, NodeTopology, RendezvousClusterOwnership,
    TopologyAuthority, ZoneAwareReplicationStrategy,
};

fn member(id: &str) -> ClusterMember {
    ClusterMember {
        node_id: ClusterNodeId::from(id),
        generation: ClusterGeneration::default(),
        role: ClusterRole::Member,
        epoch: ClusterEpoch::new(1),
        endpoints: ClusterEndpoints::default(),
        metadata: BTreeMap::new(),
    }
}

fn members(ids: &[&str]) -> Vec<ClusterMember> {
    ids.iter().map(|id| member(id)).collect()
}

fn authority(entries: &[(&str, &str, &str)]) -> TopologyAuthority {
    let mut authority = TopologyAuthority::new();
    for (id, region, zone) in entries {
        authority.commit_topology(
            ClusterNodeId::from(*id),
            NodeTopology::new(*region, *zone),
            ClusterEpoch::new(1),
        );
    }
    authority
}

#[test]
fn zone_placement_replicas_spread_across_zones_when_available() {
    let members = members(&["a1", "a2", "b1", "b2", "c1", "c2"]);
    let authority = authority(&[
        ("a1", "eu", "az-a"),
        ("a2", "eu", "az-a"),
        ("b1", "eu", "az-b"),
        ("b2", "eu", "az-b"),
        ("c1", "eu", "az-c"),
        ("c2", "eu", "az-c"),
    ]);
    let strategy = ZoneAwareReplicationStrategy::new(authority.committed_map(), 3, 3);

    let replicas = strategy
        .zone_replicas_for_key("tenant:1:user:42", &members)
        .expect("replicas");

    assert_eq!(replicas.replicas.copy_count(), 3);
    assert_eq!(replicas.zone_count(), 3);
    assert!(!replicas.placement_zone_underspread);
}

#[test]
fn zone_placement_single_zone_loss_keeps_write_quorum() {
    let members = members(&["a1", "b1", "c1"]);
    let authority = authority(&[
        ("a1", "eu", "az-a"),
        ("b1", "eu", "az-b"),
        ("c1", "eu", "az-c"),
    ]);
    let strategy = ZoneAwareReplicationStrategy::new(authority.committed_map(), 3, 3);
    let replicas = strategy
        .zone_replicas_for_key("orders:42", &members)
        .unwrap();

    assert!(replicas.single_zone_loss_keeps_write_quorum(2));
    let readiness = strategy
        .readiness_for_key("orders:42", &members, 2)
        .expect("readiness");
    assert!(readiness.is_ready());
}

#[test]
#[ignore = "chaos gate: run with -- --ignored when exercising live zone-loss placement"]
fn zone_placement_live_single_zone_loss_keeps_quorum() {
    let members = members(&["a1", "b1", "c1"]);
    let authority = authority(&[
        ("a1", "eu", "az-a"),
        ("b1", "eu", "az-b"),
        ("c1", "eu", "az-c"),
    ]);
    let strategy = ZoneAwareReplicationStrategy::new(authority.committed_map(), 3, 3);
    let replicas = strategy
        .zone_replicas_for_key("orders:42", &members)
        .unwrap();
    let mut fault_injector = FaultInjector::new(43);
    let nodes = replicas.all_nodes();
    let partition_fault = fault_injector.next_fault(&nodes).expect("partition fault");
    let Fault::Partition {
        from,
        to,
        symmetric: _,
    } = partition_fault
    else {
        panic!("expected partition fault");
    };
    assert_ne!(from, to);
    fault_injector.partition("a1", "b1", true);
    assert!(!fault_injector.can_deliver(&ClusterNodeId::from("a1"), &ClusterNodeId::from("b1")));
    fault_injector.inject_latency("c1", Duration::from_millis(7));
    assert_eq!(
        fault_injector.observed_latency(&ClusterNodeId::from("c1")),
        Duration::from_millis(7)
    );
    let latency_fault = Fault::Latency {
        node: ClusterNodeId::from("c1"),
        latency: Duration::from_millis(7),
    };
    let Fault::Latency { node, latency } = latency_fault else {
        panic!("expected latency fault");
    };
    assert_eq!(node.as_str(), "c1");
    assert_eq!(latency, Duration::from_millis(7));

    let fault = fault_injector.lose_zone("az-a", vec![ClusterNodeId::from("a1")]);
    let Fault::ZoneLoss { zone, nodes } = fault else {
        panic!("expected zone-loss fault");
    };
    let lost = fault_injector.zone_lost_nodes(&zone);
    let surviving_replicas = replicas
        .all_nodes()
        .into_iter()
        .filter(|node| !lost.contains(node))
        .count();

    assert_eq!(zone, "az-a");
    assert_eq!(nodes, vec![ClusterNodeId::from("a1")]);
    assert_eq!(surviving_replicas, 2);
    assert!(replicas.single_zone_loss_keeps_write_quorum(2));
}

#[test]
fn zone_placement_underspread_zones_are_flagged_not_silently_colocated() {
    let members = members(&["a1", "a2", "b1"]);
    let authority = authority(&[
        ("a1", "eu", "az-a"),
        ("a2", "eu", "az-a"),
        ("b1", "eu", "az-b"),
    ]);
    let strategy = ZoneAwareReplicationStrategy::new(authority.committed_map(), 3, 3);

    let readiness = strategy
        .readiness_for_key("orders:42", &members, 2)
        .expect("readiness");

    assert_eq!(readiness.zone_count, 2);
    assert!(readiness.placement_zone_underspread);
    assert!(!readiness.is_ready());
}

#[test]
fn zone_placement_one_zone_deployment_matches_042_placement() {
    let members = members(&["node-a", "node-b", "node-c", "node-d"]);
    let authority = authority(&[
        ("node-a", "eu", "az-a"),
        ("node-b", "eu", "az-a"),
        ("node-c", "eu", "az-a"),
        ("node-d", "eu", "az-a"),
    ]);
    let zone_aware = ZoneAwareReplicationStrategy::new(authority.committed_map(), 3, 1);

    let flat = RendezvousClusterOwnership
        .replicas_for_key("invoice:7", &members, 3)
        .expect("flat");
    let zoned = zone_aware
        .replicas_for_key("invoice:7", &members, 3)
        .expect("zoned");

    assert_eq!(zoned, flat);
}

#[test]
fn zone_placement_zone_topology_is_authoritative_not_gossip() {
    let mut authority = authority(&[
        ("node-a", "eu", "az-a"),
        ("node-b", "eu", "az-a"),
        ("node-c", "eu", "az-a"),
    ]);
    let members = members(&["node-a", "node-b", "node-c"]);
    let before = ZoneAwareReplicationStrategy::new(authority.committed_map(), 3, 3)
        .zone_replicas_for_key("profile:1", &members)
        .unwrap();

    authority.observe_gossip("node-b", NodeTopology::new("eu", "az-b"));
    authority.observe_gossip("node-c", NodeTopology::new("eu", "az-c"));
    let gossip_only = ZoneAwareReplicationStrategy::new(authority.committed_map(), 3, 3)
        .zone_replicas_for_key("profile:1", &members)
        .unwrap();
    assert_eq!(gossip_only.zone_count(), before.zone_count());

    authority.commit_topology(
        "node-b",
        NodeTopology::new("eu", "az-b"),
        ClusterEpoch::new(2),
    );
    authority.commit_topology(
        "node-c",
        NodeTopology::new("eu", "az-c"),
        ClusterEpoch::new(3),
    );
    let committed = ZoneAwareReplicationStrategy::new(authority.committed_map(), 3, 3)
        .zone_replicas_for_key("profile:1", &members)
        .unwrap();
    let zones = committed
        .topology
        .values()
        .map(|topology| topology.zone.as_str().to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(zones.len(), 3);
}
