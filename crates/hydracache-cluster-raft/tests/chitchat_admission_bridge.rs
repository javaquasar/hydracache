use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use chitchat::transport::ChannelTransport;
use hydracache::{
    ClusterAdmissionBridge, ClusterAdmissionBridgeEvent, ClusterAdmissionRejectReason,
    ClusterCandidate, ClusterDiscovery, ClusterGeneration, ClusterRole, RaftMetadataCommand,
};
use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
use hydracache_cluster_raft::{RaftMetadataRuntime, RaftRuntimeRole};
use tokio::time::{sleep, timeout};

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
    .gossip_interval(Duration::from_millis(20))
}

#[tokio::test]
async fn chitchat_candidates_are_admitted_into_raft_metadata_by_bridge() {
    let transport = ChannelTransport::default();
    let member_discovery = Arc::new(
        ChitchatDiscovery::spawn_with_transport(discovery_config(48_001, "member-a"), &transport)
            .await
            .unwrap(),
    );
    let client_discovery = Arc::new(
        ChitchatDiscovery::spawn_with_transport(
            discovery_config(48_002, "client-a").seed_node("127.0.0.1:48001"),
            &transport,
        )
        .await
        .unwrap(),
    );
    let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());
    let bridge = ClusterAdmissionBridge::new(member_discovery.clone(), control_plane.clone());

    member_discovery
        .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();
    client_discovery
        .announce(ClusterCandidate::client("client-a").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();
    client_discovery.gossip_once(addr(48_001)).unwrap();

    wait_until(Duration::from_secs(2), || {
        member_discovery
            .candidates()
            .iter()
            .any(|candidate| candidate.node_id.as_str() == "client-a")
    })
    .await;

    assert_eq!(bridge.run_once().await, 2);

    let snapshot = control_plane.snapshot();
    assert_eq!(snapshot.role, RaftRuntimeRole::Leader);
    assert_eq!(snapshot.commands_committed, 2);
    assert_eq!(bridge.diagnostics().candidates_admitted, 2);
    assert!(control_plane.commands().iter().any(|command| matches!(
        command,
        RaftMetadataCommand::MemberUpsert { node_id, .. } if node_id.as_str() == "member-a"
    )));
    assert!(control_plane.commands().iter().any(|command| matches!(
        command,
        RaftMetadataCommand::ClientUpsert { node_id, .. } if node_id.as_str() == "client-a"
    )));

    assert_eq!(bridge.run_once().await, 2);
    assert_eq!(control_plane.snapshot().commands_committed, 2);
    assert_eq!(bridge.diagnostics().candidates_ignored, 2);
}

#[tokio::test]
async fn bridge_rejects_stale_chitchat_generation_before_raft_proposal() {
    let transport = ChannelTransport::default();
    let discovery = Arc::new(
        ChitchatDiscovery::spawn_with_transport(discovery_config(48_011, "member-a"), &transport)
            .await
            .unwrap(),
    );
    let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1).unwrap());
    let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

    discovery
        .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)))
        .await
        .unwrap();
    assert_eq!(bridge.run_once().await, 1);

    discovery
        .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();
    assert_eq!(bridge.run_once().await, 1);

    assert_eq!(control_plane.snapshot().commands_committed, 1);
    assert_eq!(bridge.diagnostics().candidates_rejected, 1);
    assert!(matches!(
        bridge.events().last(),
        Some(ClusterAdmissionBridgeEvent::CandidateRejected {
            candidate,
            reason: ClusterAdmissionRejectReason::StaleGeneration { existing, attempted },
        }) if candidate.role == ClusterRole::Member
            && existing.value() == 2
            && attempted.value() == 1
    ));
}

async fn wait_until(timeout_after: Duration, mut condition: impl FnMut() -> bool) {
    timeout(timeout_after, async {
        loop {
            if condition() {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("condition should become true");
}
