use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use chitchat::transport::Transport;
use chitchat::ChitchatMessage;
use hydracache::{
    ClusterAdmissionBridge, ClusterCandidate, ClusterControlPlane, ClusterDiscovery,
    ClusterGeneration, ClusterNodeId, ClusterRole,
};
use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
use hydracache_cluster_raft::RaftMetadataRuntime;
use hydracache_cluster_testkit::{
    FilteredChitchatTransport, GossipFilterAction, GossipFilterSet, GossipLivenessScript,
    GossipPacketFilter,
};

fn addr(port: u16) -> SocketAddr {
    ([127, 0, 0, 1], port).into()
}

fn discovery_config(port: u16, node: &str) -> ChitchatDiscoveryConfig {
    ChitchatDiscoveryConfig::new(
        "orders",
        node,
        ClusterGeneration::new(port as u64),
        addr(port),
    )
    .gossip_interval(Duration::from_secs(60))
}

async fn drive_gossip(
    left: &ChitchatDiscovery,
    left_addr: SocketAddr,
    right: &ChitchatDiscovery,
    right_addr: SocketAddr,
    rounds: usize,
) {
    for _ in 0..rounds {
        left.gossip_once(right_addr).unwrap();
        right.gossip_once(left_addr).unwrap();
        tokio::task::yield_now().await;
    }
}

async fn gossip_flap_preserves_membership(base_port: u16, inject_bug: bool) -> bool {
    let transport = FilteredChitchatTransport::default();
    let member_a = Arc::new(
        ChitchatDiscovery::spawn_with_transport(
            discovery_config(base_port, "member-a"),
            &transport,
        )
        .await
        .unwrap(),
    );

    member_a
        .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();
    member_a
        .announce(ClusterCandidate::member("member-b").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();

    let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());
    let bridge = ClusterAdmissionBridge::new(member_a.clone(), control_plane.clone());
    assert_eq!(bridge.run_once().await, 2);
    let baseline_members = control_plane.members().len();
    let baseline_commits = control_plane.snapshot().commands_committed;

    let mut script = GossipLivenessScript::live_suspect_dead_live("member-b");
    script.apply_all(&*member_a).await.unwrap();
    assert_eq!(script.trace().len(), 4);

    if inject_bug {
        control_plane
            .leave(&ClusterNodeId::from("member-b"), ClusterGeneration::new(1))
            .await
            .unwrap();
    }
    bridge.run_once().await;

    control_plane.members().len() == baseline_members
        && control_plane.snapshot().commands_committed == baseline_commits
}

#[tokio::test]
async fn gossip_flap_does_not_flap_quorum() {
    assert!(gossip_flap_preserves_membership(49_001, false).await);
    assert!(!gossip_flap_preserves_membership(49_011, true).await);
}

struct LostLeaveOutcome {
    local_marker_named: bool,
    dropped_count: usize,
    dropped_trace_count: usize,
    trace_debug: String,
}

async fn lost_leave_marker_scenario(base_port: u16, inject_bug: bool) -> LostLeaveOutcome {
    let filters = GossipFilterSet::default();
    let transport = FilteredChitchatTransport::channel(filters.clone());
    let member = ChitchatDiscovery::spawn_with_transport(
        discovery_config(base_port, "member-a"),
        &transport,
    )
    .await
    .unwrap();
    let observer = ChitchatDiscovery::spawn_with_transport(
        discovery_config(base_port + 1, "observer").seed_node(addr(base_port).to_string()),
        &transport,
    )
    .await
    .unwrap();

    member
        .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(7)))
        .await
        .unwrap();

    if !inject_bug {
        filters.add_filter(
            GossipPacketFilter::new()
                .from(addr(base_port))
                .to(addr(base_port + 1))
                .key_prefix("hydracache.")
                .action(GossipFilterAction::Drop),
        );
    }

    member
        .mark_leaving("member-a", ClusterGeneration::new(7), ClusterRole::Member)
        .await
        .unwrap();
    drive_gossip(&member, addr(base_port), &observer, addr(base_port + 1), 8).await;

    let local_marker_named = member
        .local_value("hydracache.lifecycle")
        .await
        .is_some_and(|value| value == "leaving")
        && member
            .local_value("hydracache.left.generation")
            .await
            .is_some_and(|value| value == "7");

    LostLeaveOutcome {
        local_marker_named,
        dropped_count: filters.dropped_count(),
        dropped_trace_count: filters
            .trace()
            .iter()
            .filter(|event| event.action == "dropped")
            .count(),
        trace_debug: format!("{:?}", filters.trace()),
    }
}

#[tokio::test]
async fn lost_leave_marker_behavior_is_named() {
    let normal = lost_leave_marker_scenario(49_031, false).await;
    assert!(normal.local_marker_named);
    assert!(normal.dropped_count > 0, "trace: {}", normal.trace_debug);
    assert!(normal.dropped_trace_count > 0);

    let canary = lost_leave_marker_scenario(49_041, true).await;
    assert!(canary.local_marker_named);
    assert_eq!(canary.dropped_count, 0);
}

async fn stale_generation_is_rejected(base_port: u16, inject_bug: bool) -> bool {
    let transport = FilteredChitchatTransport::default();
    let discovery = Arc::new(
        ChitchatDiscovery::spawn_with_transport(
            discovery_config(base_port, "member-a"),
            &transport,
        )
        .await
        .unwrap(),
    );
    let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());
    let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

    discovery
        .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(3)))
        .await
        .unwrap();
    assert_eq!(bridge.run_once().await, 1);
    let baseline_commits = control_plane.snapshot().commands_committed;

    let mut script = GossipLivenessScript::live_suspect_dead_live("member-a");
    script.apply_all(&*discovery).await.unwrap();

    if inject_bug {
        control_plane
            .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(3))
            .await
            .unwrap();
    }

    discovery
        .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)))
        .await
        .unwrap();
    bridge.run_once().await;

    let rejected = bridge.diagnostics().candidates_rejected > 0;
    rejected && control_plane.snapshot().commands_committed == baseline_commits
}

#[tokio::test]
async fn stale_generation_candidate_resurrection_is_rejected_under_flap() {
    assert!(stale_generation_is_rejected(49_061, false).await);
    assert!(!stale_generation_is_rejected(49_071, true).await);
}

async fn normalized_filter_trace(base_port: u16) -> Vec<(&'static str, String)> {
    let filters = GossipFilterSet::default();
    let transport = FilteredChitchatTransport::channel(filters.clone());
    let mut first = transport.open(addr(base_port)).await.unwrap();
    let mut second = transport.open(addr(base_port + 1)).await.unwrap();

    filters.add_filter(
        GossipPacketFilter::new()
            .from(addr(base_port))
            .to(addr(base_port + 1))
            .action(GossipFilterAction::Duplicate(1)),
    );
    first
        .send(addr(base_port + 1), ChitchatMessage::BadCluster)
        .await
        .unwrap();
    let _ = second.recv().await.unwrap();
    let _ = second.recv().await.unwrap();

    filters
        .trace()
        .into_iter()
        .map(|event| (event.action, format!("{:?}", event.message_type)))
        .collect()
}

#[tokio::test]
async fn gossip_filter_replays_identically_for_same_seed() {
    let first = normalized_filter_trace(49_081).await;
    let second = normalized_filter_trace(49_091).await;

    assert_eq!(first, second);
    assert!(first.iter().any(|(action, _)| *action == "duplicated"));
}
