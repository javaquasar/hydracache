use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::{ClusterGeneration, HydraCache, InMemoryCluster};
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_observability::HydraCacheRegistry;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn actuator_cluster_pilot_report_highlights_auth_missing() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let _other = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();
    let member = HydraCache::member()
        .shared_cluster(cluster)
        .node_id("member-b")
        .generation(ClusterGeneration::new(1))
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();
    member.record_cluster_barrier_timeout();

    let registry = HydraCacheRegistry::new().with_cache("member", member);
    let app = HydraCacheActuator::new(registry).routes();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/cluster/pilot-report")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["highlights"][0], "AUTH MISSING");
    assert_eq!(body["caches"][0]["name"], "member");
    assert_eq!(body["caches"][0]["report"]["readiness"]["member_count"], 2);
    assert_eq!(body["caches"][0]["report"]["barrier_timeouts"], 1);
    assert_eq!(
        body["caches"][0]["report"]["transport_posture"]["auth"],
        false
    );
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
