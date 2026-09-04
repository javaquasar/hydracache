use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_observability::{
    ClientProtocolLifecycle, ManagementClientProtocol, ManagementClientsSnapshot,
    ManagementCompleteness, ManagementObservationSource, MANAGEMENT_API_SCHEMA_VERSION,
};
use hydracache_server::{
    AdminHttpSurface, Hc2ClientPlaneService, ServerConfig, ServerRuntime, MANAGEMENT_CLIENTS_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

fn request() -> Request<Body> {
    Request::builder()
        .uri(MANAGEMENT_CLIENTS_PATH)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Body::empty())
        .unwrap()
}

async fn json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 262_144).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn configured_surface() -> AdminHttpSurface {
    let mut config = ServerConfig::default();
    config.client_api.enabled = true;
    config.redis_api.enabled = true;
    let surface = AdminHttpSurface::new(ServerRuntime::new(config).unwrap().start());
    let dispatch = surface
        .runtime()
        .lock()
        .unwrap()
        .client_dispatch_state()
        .unwrap();
    let hc2 = Hc2ClientPlaneService::new(dispatch, "test-cluster");
    surface.with_hc2_metrics(hc2)
}

#[tokio::test]
async fn client_lifecycle_route_is_admin_only_bounded_and_protocol_complete_without_identity() {
    let surface = configured_surface();
    {
        let runtime = surface.runtime();
        let mut runtime = runtime.lock().unwrap();
        assert!(runtime.begin_redis_connection());
        assert!(runtime.begin_redis_connection());
        runtime.finish_redis_connection();
    }
    let router = surface.routes();
    let anonymous = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(MANAGEMENT_CLIENTS_PATH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let response = router.oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["completeness"], "partial");
    assert_eq!(body["data"]["detail_available"], false);
    assert!(body["data"]["active_connections"].is_null());
    assert_eq!(body["data"]["protocols"].as_array().unwrap().len(), 3);
    assert_eq!(body["data"]["protocols"][0]["protocol"], "hc1");
    assert_eq!(body["data"]["protocols"][1]["protocol"], "hc2");
    assert_eq!(body["data"]["protocols"][2]["protocol"], "resp");
    assert_eq!(body["data"]["protocols"][2]["active_connections"], 1);
    assert_eq!(body["data"]["protocols"][2]["accepted_total"], 2);
    assert_eq!(body["data"]["protocols"][2]["closed_total"], 1);
    let encoded = body.to_string();
    for forbidden in [
        "remote_address",
        "certificate",
        "auth_token",
        "tenant-a",
        "cache_key",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}

#[test]
fn lifecycle_contract_rejects_leaked_closed_owner_and_unstable_protocol_order() {
    let protocol = |kind, active, accepted, closed| ClientProtocolLifecycle {
        protocol: kind,
        version: Some("v1".to_owned()),
        active_connections: Some(active),
        accepted_total: Some(accepted),
        closed_total: Some(closed),
        rejected_total: Some(0),
        pending_invocations: Some(0),
        active_subscriptions: Some(0),
        active_sessions: Some(0),
        buffered_bytes: Some(0),
    };
    let snapshot = |protocols| ManagementClientsSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        authority_epoch: Some(1),
        observation_seq: 1,
        active_connections: Some(0),
        accepted_total: Some(1),
        closed_total: Some(1),
        rejected_total: 0,
        pending_invocations: Some(0),
        active_subscriptions: 0,
        active_sessions: Some(0),
        buffered_bytes: Some(0),
        reconnecting: Some(0),
        slow: Some(0),
        quota_rejected_total: Some(0),
        cleanup_lag: Some(0),
        protocols,
        detail_available: false,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    };
    snapshot(vec![protocol(ManagementClientProtocol::Resp, 0, 1, 1)])
        .validate()
        .unwrap();
    assert!(
        snapshot(vec![protocol(ManagementClientProtocol::Resp, 1, 1, 1)])
            .validate()
            .is_err()
    );
    assert!(snapshot(vec![
        protocol(ManagementClientProtocol::Resp, 0, 1, 1),
        protocol(ManagementClientProtocol::Hc1, 0, 0, 0),
    ])
    .validate()
    .is_err());
}

#[test]
fn canary_closed_client_owner_cannot_survive_cleanup() {
    let defect =
        std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W6-CLOSED-CLIENT-LEAK");
    let lifecycle = ClientProtocolLifecycle {
        protocol: ManagementClientProtocol::Hc2,
        version: Some("hc-2-alpha".to_owned()),
        active_connections: Some(if defect { 1 } else { 0 }),
        accepted_total: Some(1),
        closed_total: Some(1),
        rejected_total: Some(0),
        pending_invocations: Some(0),
        active_subscriptions: Some(0),
        active_sessions: Some(0),
        buffered_bytes: Some(0),
    };
    let result = ManagementClientsSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        authority_epoch: Some(1),
        observation_seq: 1,
        active_connections: Some(if defect { 1 } else { 0 }),
        accepted_total: Some(1),
        closed_total: Some(1),
        rejected_total: 0,
        pending_invocations: Some(0),
        active_subscriptions: 0,
        active_sessions: Some(0),
        buffered_bytes: Some(0),
        reconnecting: Some(0),
        slow: Some(0),
        quota_rejected_total: Some(0),
        cleanup_lag: Some(0),
        protocols: vec![lifecycle],
        detail_available: false,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
    .validate();
    assert!(
        result.is_ok(),
        "HC-CANARY-RED:MC72-W6-CLOSED-CLIENT-LEAK closed client remained in the live owner count"
    );
}
