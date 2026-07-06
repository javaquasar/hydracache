use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hydracache_client_protocol::ClientFrame;
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientRouteBoundary, ClientSurfaceLimits, CLIENT_DATA_PATH,
    HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{
    AdminApiConfig, BackupConfig, ClientApiConfig, ServerConfig, ServerRole, ServerRuntime,
    TlsConfig,
};
use tower::ServiceExt;

fn member_config_with_client_surface() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:0".parse().unwrap(),
        seeds: vec!["127.0.0.1:0".to_owned()],
        storage_dir: Some(PathBuf::from(
            "target/test-hydracache-server-client-surface",
        )),
        drain_timeout_ms: 1_000,
        tls: TlsConfig::default(),
        backup: BackupConfig::default(),
        client_api: ClientApiConfig {
            enabled: true,
            limits: ClientSurfaceLimits::default(),
        },
        admin_api: AdminApiConfig::default(),
    }
}

#[test]
fn client_surface_lifecycle_server_keeps_client_surface_running_until_shutdown() {
    let mut runtime = ServerRuntime::new(member_config_with_client_surface())
        .unwrap()
        .start();

    assert!(runtime.ready().ready);
    assert!(runtime.ready().client_surface_ready);
    assert!(runtime.client_surface_ready());
    assert!(runtime.begin_client_subscription());

    runtime.shutdown();

    assert!(!runtime.client_surface_ready());
    assert_eq!(runtime.client_active_subscriptions(), 0);
    assert_eq!(runtime.client_surface_drain().unwrap().started_with, 1);
    assert_eq!(runtime.client_surface_drain().unwrap().remaining, 0);
}

#[test]
fn client_surface_lifecycle_client_routes_are_separate_from_internal_member_routes() {
    assert!(ClientRouteBoundary::is_client_route("/client/v1/data"));
    assert!(ClientRouteBoundary::is_client_route(
        "/client/v1/subscriptions"
    ));
    assert!(!ClientRouteBoundary::is_client_route("/cluster/peer-fetch"));
    assert!(!ClientRouteBoundary::is_client_route("/cluster/replicate"));

    assert!(ClientRouteBoundary::is_internal_member_route(
        "/cluster/peer-fetch"
    ));
    assert!(!ClientRouteBoundary::is_internal_member_route(
        "/client/v1/data"
    ));
}

#[tokio::test]
async fn client_surface_lifecycle_anonymous_client_data_route_is_refused_before_dispatch() {
    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let state = surface.state();
    let payload = ClientFrame::new(Vec::from("hello".as_bytes()))
        .encode()
        .unwrap();

    let response = surface
        .routes()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_DATA_PATH)
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(state.dispatch_attempts(), 0);
    assert_eq!(state.state_mutations(), 0);
    assert_eq!(state.rejected_anonymous(), 1);
}

#[tokio::test]
async fn client_surface_lifecycle_oversized_frame_is_rejected_without_state_mutation() {
    let limits = ClientSurfaceLimits {
        max_frame_bytes: 8,
        ..ClientSurfaceLimits::default()
    };
    let surface = AxumClientSurface::new(limits).unwrap();
    let state = surface.state();

    let response = surface
        .routes()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_DATA_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "client-a")
                .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
                .body(Body::from(vec![0_u8; 32]))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(state.dispatch_attempts(), 0);
    assert_eq!(state.state_mutations(), 0);
    assert_eq!(state.rejected_oversized(), 1);
}

#[test]
fn client_surface_lifecycle_subscription_stream_drains_on_shutdown() {
    let mut runtime = ServerRuntime::new(member_config_with_client_surface())
        .unwrap()
        .start();

    assert!(runtime.begin_client_subscription());
    assert!(runtime.begin_client_subscription());
    assert_eq!(runtime.client_active_subscriptions(), 2);

    runtime.shutdown();

    let drain = runtime.client_surface_drain().unwrap();
    assert_eq!(drain.started_with, 2);
    assert_eq!(drain.remaining, 0);
    assert_eq!(runtime.client_active_subscriptions(), 0);
}
