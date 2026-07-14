use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_client_transport_axum::{AxumClientSurface, ClientSurfaceLimits};
use hydracache_server::{
    AdminApiConfig, AdminHttpSurface, BackupConfig, ClientApiConfig, ClusterAuthConfig,
    ServerConfig, ServerRole, ServerRuntime, TlsConfig, ADMIN_ACTUATOR_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

fn member_config(test_name: &str) -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:0".parse().unwrap(),
        node_id: None,
        seeds: vec!["127.0.0.1:0".to_owned()],
        storage_dir: Some(PathBuf::from("target/test-hydracache-actuator-mount").join(test_name)),
        drain_timeout_ms: 1_000,
        tls: TlsConfig::default(),
        cluster_auth: ClusterAuthConfig::default(),
        backup: BackupConfig::default(),
        client_api: ClientApiConfig::default(),
        admin_api: AdminApiConfig::default(),
        ..ServerConfig::default()
    }
}

fn get_request(uri: impl AsRef<str>) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri.as_ref())
        .body(Body::empty())
        .unwrap()
}

async fn json_response(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn actuator_json_is_served_on_the_admin_port() {
    let surface = AdminHttpSurface::new(
        ServerRuntime::new(member_config("admin-port"))
            .unwrap()
            .start(),
    );

    let health = surface
        .routes()
        .oneshot(get_request(format!("{ADMIN_ACTUATOR_PATH}/health")))
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);
    let health = json_response(health).await;
    assert_eq!(health["status"], "UP");
    assert_eq!(health["cache_count"], 1);

    let caches = surface
        .routes()
        .oneshot(get_request(format!("{ADMIN_ACTUATOR_PATH}/caches")))
        .await
        .unwrap();
    assert_eq!(caches.status(), StatusCode::OK);
    let caches = json_response(caches).await;
    assert_eq!(caches["caches"], serde_json::json!(["server"]));

    let stats = surface
        .routes()
        .oneshot(get_request(format!(
            "{ADMIN_ACTUATOR_PATH}/caches/server/stats"
        )))
        .await
        .unwrap();
    assert_eq!(stats.status(), StatusCode::OK);
    let stats = json_response(stats).await;
    assert_eq!(stats["hits"], 0);
    assert_eq!(stats["total_requests"], 0);

    let missing = surface
        .routes()
        .oneshot(get_request(format!(
            "{ADMIN_ACTUATOR_PATH}/caches/missing/stats"
        )))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn actuator_is_absent_on_the_client_port() {
    let client = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();

    let response = client
        .routes()
        .oneshot(get_request(format!("{ADMIN_ACTUATOR_PATH}/health")))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn actuator_is_served_during_drain() {
    let runtime = Arc::new(Mutex::new(
        ServerRuntime::new(member_config("during-drain"))
            .unwrap()
            .start(),
    ));
    let surface = AdminHttpSurface::from_shared(Arc::clone(&runtime));
    runtime.lock().unwrap().begin_drain();

    let response = surface
        .routes()
        .oneshot(get_request(format!("{ADMIN_ACTUATOR_PATH}/health")))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_response(response).await;
    assert_eq!(body["status"], "UP");
    assert_eq!(body["cache_count"], 1);
}
