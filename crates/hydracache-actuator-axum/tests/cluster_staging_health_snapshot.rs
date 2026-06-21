use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::{ClusterGeneration, HydraCache, InMemoryCluster};
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_observability::HydraCacheRegistry;
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn actuator_cluster_staging_health_json_shape_is_stable() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let member = HydraCache::member()
        .shared_cluster(cluster)
        .node_id("member-a")
        .generation(ClusterGeneration::new(7))
        .start()
        .await
        .unwrap();
    member.record_cluster_owner_load_success();
    member.record_cluster_remote_fetch_success();
    member.record_cluster_hot_cache_hit();
    member.record_cluster_gossip_reset(25);

    let registry = HydraCacheRegistry::new()
        .with_cache("local", HydraCache::local().build())
        .with_cache("member", member);
    let app = HydraCacheActuator::new(registry).routes();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/cluster/staging-health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["caches"].as_array().unwrap().len(), 1);
    assert_eq!(body["caches"][0]["name"], "member");
    assert_eq!(body["caches"][0]["health"]["node_id"], "member-a");
    assert_eq!(body["caches"][0]["health"]["generation"], 7);
    assert_eq!(body["caches"][0]["health"]["owner_load_success"], 1);
    assert_eq!(body["caches"][0]["health"]["remote_fetch_success"], 1);
    assert_eq!(body["caches"][0]["health"]["hot_cache_hits"], 1);
    assert_eq!(
        body["caches"][0]["health"]["state"],
        json!({
            "state": "degraded",
            "reasons": [
                {
                    "gossip_reset_recent": {
                        "tombstone_age_ms": 25,
                        "reset_count": 1
                    }
                }
            ]
        })
    );
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
