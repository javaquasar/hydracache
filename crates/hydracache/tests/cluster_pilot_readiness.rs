use std::sync::Arc;

use hydracache::{ClusterGeneration, HydraCache, InMemoryCluster, TransportPosture};

async fn configured_member_cluster(member_count: usize) -> (Arc<InMemoryCluster>, HydraCache) {
    let cluster = Arc::new(InMemoryCluster::new("pilot-readiness"));
    for index in 0..member_count.saturating_sub(1) {
        let node_id = format!("member-{index}");
        let _member = HydraCache::member()
            .shared_cluster(cluster.clone())
            .node_id(node_id)
            .generation(ClusterGeneration::new(1))
            .transport_auth_configured(true)
            .strict_wire_compatibility(true)
            .start()
            .await
            .unwrap();
    }

    let observed = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-observed")
        .generation(ClusterGeneration::new(1))
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();

    (cluster, observed)
}

#[tokio::test]
async fn pilot_ready_for_configured_topology() {
    let (_cluster, cache) = configured_member_cluster(3).await;

    let readiness = cache.cluster_pilot_readiness();

    assert!(readiness.is_pilot_ready());
    assert_eq!(readiness.member_count, 3);
    assert!(readiness.transport_posture.is_safe());
    assert!(readiness.highlights().is_empty());
}

#[tokio::test]
async fn not_ready_when_auth_missing() {
    let cluster = Arc::new(InMemoryCluster::new("pilot-readiness"));
    let _other = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();
    let cache = HydraCache::member()
        .shared_cluster(cluster)
        .node_id("member-b")
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();

    let readiness = cache.cluster_pilot_readiness();

    assert!(!readiness.is_pilot_ready());
    assert!(!readiness.transport_posture.auth);
    assert_eq!(
        readiness.transport_posture.highlight(),
        Some("AUTH MISSING")
    );
}

#[tokio::test]
async fn not_ready_when_wire_not_strict() {
    let cluster = Arc::new(InMemoryCluster::new("pilot-readiness"));
    let _other = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();
    let cache = HydraCache::member()
        .shared_cluster(cluster)
        .node_id("member-b")
        .transport_auth_configured(true)
        .start()
        .await
        .unwrap();

    let readiness = cache.cluster_pilot_readiness();

    assert!(!readiness.is_pilot_ready());
    assert!(!readiness.strict_wire_compatibility);
}

#[test]
fn not_ready_when_no_members() {
    let cache = HydraCache::local()
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .build();

    let readiness = cache.cluster_pilot_readiness();

    assert!(!readiness.is_pilot_ready());
    assert!(!readiness.has_members);
    assert_eq!(readiness.member_count, 0);
}

#[tokio::test]
async fn not_ready_when_outside_supported_size() {
    let (_cluster, cache) = configured_member_cluster(6).await;

    let readiness = cache.cluster_pilot_readiness();

    assert!(!readiness.is_pilot_ready());
    assert_eq!(readiness.member_count, 6);
    assert!(!readiness.within_supported_size);
}

#[tokio::test]
async fn not_ready_when_lifecycle_stopped() {
    let cluster = Arc::new(InMemoryCluster::new("pilot-readiness"));
    let _member_a = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();
    let _member_b = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-b")
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .connect()
        .await
        .unwrap();

    client.leave_cluster().await.unwrap();

    let readiness = client.cluster_pilot_readiness();
    assert!(!readiness.is_pilot_ready());
    assert!(!readiness.lifecycle_operational);
    assert!(readiness.within_supported_size);
}

#[test]
fn posture_safe_with_auth_and_wire_or_declared_mesh() {
    assert!(TransportPosture::new(true, true, false).is_safe());
    assert!(TransportPosture::new(false, false, true).is_safe());
    assert!(!TransportPosture::new(true, false, false).is_safe());
}

#[tokio::test]
async fn pilot_report_contains_highlight_and_stable_shape() {
    let cluster = Arc::new(InMemoryCluster::new("pilot-readiness"));
    let _other = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();
    let cache = HydraCache::member()
        .shared_cluster(cluster)
        .node_id("member-b")
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();

    let report = serde_json::to_value(cache.cluster_pilot_report()).unwrap();

    assert_eq!(report["readiness"]["member_count"], 2);
    assert_eq!(report["transport_posture"]["auth"], false);
    assert_eq!(report["highlights"][0], "AUTH MISSING");
    assert!(report.get("stamp").is_some());
    assert!(report.get("barrier_timeouts").is_some());
}
