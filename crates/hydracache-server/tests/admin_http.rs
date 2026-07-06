use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{
    AdminApiConfig, AdminHttpSurface, BackupConfig, ClientApiConfig, ClusterAuthConfig,
    ServerConfig, ServerRole, ServerRuntime, TlsConfig, ADMIN_BACKUP_PATH, ADMIN_CONSOLE_PATH,
    ADMIN_DRAIN_PATH, ADMIN_METRICS_PATH, ADMIN_READYZ_PATH, ADMIN_RESHARD_PATH, ADMIN_STATUS_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

mod admin_http {
    use super::*;

    fn member_config() -> ServerConfig {
        ServerConfig {
            role: ServerRole::Member,
            listen_addr: "127.0.0.1:18080".parse().unwrap(),
            cluster_addr: "127.0.0.1:0".parse().unwrap(),
            seeds: vec!["127.0.0.1:0".to_owned()],
            storage_dir: Some(PathBuf::from("target/test-hydracache-server-admin")),
            drain_timeout_ms: 1_000,
            tls: TlsConfig::default(),
            cluster_auth: ClusterAuthConfig::default(),
            backup: BackupConfig::default(),
            client_api: ClientApiConfig::default(),
            admin_api: AdminApiConfig::default(),
        }
    }

    fn member_config_with_backup() -> ServerConfig {
        ServerConfig {
            backup: BackupConfig {
                enabled: true,
                location: Some("file://target/test-hydracache-backups".to_owned()),
            },
            ..member_config()
        }
    }

    fn local_config() -> ServerConfig {
        ServerConfig {
            role: ServerRole::Local,
            seeds: Vec::new(),
            storage_dir: None,
            ..member_config()
        }
    }

    fn admin_request(method: &str, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
            .header(HYDRACACHE_TENANT_HEADER, "system")
            .header(HYDRACACHE_ADMIN_HEADER, "true")
            .body(Body::empty())
            .unwrap()
    }

    fn non_admin_request(method: &str, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
            .header(HYDRACACHE_TENANT_HEADER, "system")
            .body(Body::empty())
            .unwrap()
    }

    async fn json_response(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn text_response(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn readyz_reflects_can_serve_and_flips_503_while_draining() {
        let runtime = Arc::new(Mutex::new(
            ServerRuntime::new(member_config()).unwrap().start(),
        ));
        let surface = AdminHttpSurface::from_shared(Arc::clone(&runtime));

        let ready = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(ADMIN_READYZ_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ready.status(), StatusCode::OK);
        let ready_body = json_response(ready).await;
        assert_eq!(ready_body["ready"], true);

        runtime.lock().unwrap().begin_drain();

        let draining = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(ADMIN_READYZ_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(draining.status(), StatusCode::SERVICE_UNAVAILABLE);
        let draining_body = json_response(draining).await;
        assert_eq!(draining_body["ready"], false);
        assert_eq!(draining_body["accepting"], false);
    }

    #[tokio::test]
    async fn admin_status_reports_leader_quorum_reshard_phase() {
        let surface = AdminHttpSurface::new(ServerRuntime::new(member_config()).unwrap().start());

        let response = surface
            .routes()
            .oneshot(admin_request("GET", ADMIN_STATUS_PATH))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = json_response(response).await;
        assert_eq!(body["source"], "live");
        assert!(body["leader"].as_str().unwrap().starts_with("member-"));
        assert_eq!(body["term"], 1);
        assert_eq!(body["quorum_ok"], true);
        assert_eq!(body["members"], 1);
        assert_eq!(body["reshard_phase"], "idle");
        assert_eq!(body["draining"], false);
    }

    #[tokio::test]
    async fn metrics_endpoint_serves_prometheus_text_with_stable_content_type() {
        let surface = AdminHttpSurface::new(ServerRuntime::new(member_config()).unwrap().start());

        let response = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(ADMIN_METRICS_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/plain; version=0.0.4"
        );
        let text = text_response(response).await;
        assert!(text.contains("# TYPE hydracache_cache_hits_total counter"));
        assert!(text.contains("# TYPE hydracache_cluster_members gauge"));
        assert!(text.contains("hydracache_cluster_members{source=\"live\"} 1"));
    }

    #[tokio::test]
    async fn metrics_endpoint_is_served_during_drain() {
        let runtime = Arc::new(Mutex::new(
            ServerRuntime::new(member_config()).unwrap().start(),
        ));
        let surface = AdminHttpSurface::from_shared(Arc::clone(&runtime));
        runtime.lock().unwrap().begin_drain();

        let ready = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(ADMIN_READYZ_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ready.status(), StatusCode::SERVICE_UNAVAILABLE);

        let metrics = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(ADMIN_METRICS_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(metrics.status(), StatusCode::OK);
        let text = text_response(metrics).await;
        assert!(text.contains("hydracache_cluster_members{source=\"live\"} 1"));
    }

    #[tokio::test]
    async fn metrics_endpoint_is_not_on_the_client_port() {
        let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();

        let response = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(ADMIN_METRICS_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn console_assets_are_served_from_admin_surface() {
        let surface = AdminHttpSurface::new(ServerRuntime::new(member_config()).unwrap().start());

        let index = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(ADMIN_CONSOLE_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(index.status(), StatusCode::OK);
        assert_eq!(
            index.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        let html = text_response(index).await;
        assert!(html.contains("HydraCache Management Center"));
        assert!(html.contains("./app.js"));

        let app = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/console/app.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(app.status(), StatusCode::OK);
        let javascript = text_response(app).await;
        assert!(javascript.contains("/cluster/overview"));
        assert!(javascript.contains("MAX_RENDERED_MEMBERS"));
    }

    #[tokio::test]
    async fn admin_actions_are_authz_gated_and_idempotent() {
        let surface = AdminHttpSurface::new(
            ServerRuntime::new(member_config_with_backup())
                .unwrap()
                .start(),
        );

        let anonymous = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(ADMIN_DRAIN_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

        let forbidden = surface
            .routes()
            .oneshot(non_admin_request("POST", ADMIN_DRAIN_PATH))
            .await
            .unwrap();
        assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

        let reshard = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_RESHARD_PATH))
            .await
            .unwrap();
        assert_eq!(reshard.status(), StatusCode::OK);
        let reshard_body = json_response(reshard).await;
        assert_eq!(reshard_body["action"], "reshard");
        assert_eq!(reshard_body["outcome"], "accepted");

        let backup = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_BACKUP_PATH))
            .await
            .unwrap();
        assert_eq!(backup.status(), StatusCode::OK);
        let backup_body = json_response(backup).await;
        assert_eq!(backup_body["action"], "backup");
        assert_eq!(backup_body["outcome"], "accepted");

        let first_drain = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_DRAIN_PATH))
            .await
            .unwrap();
        assert_eq!(first_drain.status(), StatusCode::OK);
        let first_body = json_response(first_drain).await;
        assert_eq!(first_body["action"], "drain");
        assert_eq!(first_body["outcome"], "accepted");

        let second_drain = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_DRAIN_PATH))
            .await
            .unwrap();
        assert_eq!(second_drain.status(), StatusCode::OK);
        let second_body = json_response(second_drain).await;
        assert_eq!(second_body["drain"]["remaining"], 0);
        assert_eq!(second_body["outcome"], "accepted");
    }

    #[tokio::test]
    async fn admin_action_failure_is_loud_not_silent() {
        let local_surface =
            AdminHttpSurface::new(ServerRuntime::new(local_config()).unwrap().start());
        let reshard = local_surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_RESHARD_PATH))
            .await
            .unwrap();
        assert_eq!(reshard.status(), StatusCode::CONFLICT);
        let reshard_body = json_response(reshard).await;
        assert_eq!(reshard_body["outcome"], "rejected");
        assert!(reshard_body["detail"]
            .as_str()
            .unwrap()
            .contains("requires member mode"));

        let backup_surface =
            AdminHttpSurface::new(ServerRuntime::new(member_config()).unwrap().start());
        let backup = backup_surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_BACKUP_PATH))
            .await
            .unwrap();
        assert_eq!(backup.status(), StatusCode::CONFLICT);
        let backup_body = json_response(backup).await;
        assert_eq!(backup_body["outcome"], "rejected");
        assert!(backup_body["detail"]
            .as_str()
            .unwrap()
            .contains("backup.enabled"));
    }
}
