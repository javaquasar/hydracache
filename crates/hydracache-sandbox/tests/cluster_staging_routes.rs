use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_sandbox::{build_sandbox, SandboxConfig};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn sandbox_staging_gate_route_returns_healthy_report() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;

    let body = json_post(
        app,
        "/sandbox/cluster/staging-gate",
        r#"{"cluster":"route-staging-gate","invalidations":3,"flow_id":"route-staging-gate"}"#,
    )
    .await;

    assert_eq!(body["flow_id"], "route-staging-gate");
    assert_eq!(body["scenario"], "staging-gate");
    assert_eq!(body["passed"], true);
    assert_eq!(body["health"]["state"], "healthy");
    assert_eq!(body["report"]["published"], body["report"]["received"]);
    assert_eq!(body["report"]["received"], body["report"]["applied"]);
    assert_eq!(body["report"]["lagged"], 0);
    assert_eq!(body["report"]["decode_errors"], 0);
    assert_eq!(body["report"]["publish_failures"], 0);
    assert_eq!(body["report"]["receiver_closed"], 0);
    assert_eq!(body["report"]["owner_load_success"], 1);
    assert_eq!(body["report"]["remote_fetch_success"], 1);
    assert_eq!(body["report"]["hot_cache_hits"], 1);
    assert!(body["runbook"]
        .as_str()
        .unwrap()
        .contains("PRODUCTION_CLUSTER_READINESS"));
}

#[tokio::test]
async fn sandbox_leave_rejoin_route_reports_generation_fencing() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;

    let body = json_post(
        app,
        "/sandbox/cluster/leave-rejoin",
        r#"{"cluster":"route-leave-rejoin","flow_id":"route-leave-rejoin"}"#,
    )
    .await;

    assert_eq!(body["scenario"], "leave-rejoin");
    assert_eq!(body["passed"], true);
    assert_eq!(body["health"]["state"], "healthy");
    assert_eq!(body["report"]["stale_generation_rejected"], 1);
}

#[tokio::test]
async fn sandbox_stale_generation_route_reports_fencing() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;

    let body = json_post(
        app,
        "/sandbox/cluster/stale-generation",
        r#"{"cluster":"route-stale-generation","flow_id":"route-stale-generation"}"#,
    )
    .await;

    assert_eq!(body["scenario"], "stale-generation");
    assert_eq!(body["passed"], true);
    assert_eq!(body["health"]["state"], "healthy");
    assert_eq!(body["report"]["stale_generation_rejected"], 1);
}

#[tokio::test]
async fn sandbox_peer_fetch_auth_wire_route_reports_rejections() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;

    let body = json_post(
        app,
        "/sandbox/cluster/peer-fetch-auth-wire",
        r#"{"cluster":"route-peer-fetch-auth-wire","flow_id":"route-peer-fetch-auth-wire"}"#,
    )
    .await;

    assert_eq!(body["scenario"], "peer-fetch-auth-wire");
    assert_eq!(body["passed"], true);
    assert_eq!(body["health"]["state"], "healthy");
    assert_eq!(body["report"]["peer_fetch_auth_failures"], 1);
    assert_eq!(body["report"]["wire_version_rejections"], 1);
}

async fn json_post(app: axum::Router, uri: &str, body: &'static str) -> Value {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
