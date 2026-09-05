use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use hydracache_client_protocol::{ClientRequest, ClientRequestEnvelope, Namespace, StructuredKey};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientIdentity, ClientSurfaceLimits, HYDRACACHE_ADMIN_HEADER,
    HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{
    AdminApiConfig, AdminHttpSurface, BackupConfig, ClientApiConfig, ClusterAuthConfig,
    ServerConfig, ServerRole, ServerRuntime, TlsConfig, ADMIN_BACKUP_PATH, ADMIN_CONSOLE_PATH,
    ADMIN_DIAGNOSTIC_RESET_PATH, ADMIN_DRAIN_PATH, ADMIN_MEMORY_FOOTPRINT_PATH, ADMIN_METRICS_PATH,
    ADMIN_RAFT_COMPACTION_PATH, ADMIN_READYZ_PATH, ADMIN_RESHARD_PATH, ADMIN_STATUS_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

mod admin_http {
    use super::*;

    static STORAGE_SEQ: AtomicU64 = AtomicU64::new(0);

    fn test_path(label: &str) -> PathBuf {
        let seq = STORAGE_SEQ.fetch_add(1, Ordering::SeqCst);
        PathBuf::from(format!(
            "target/test-hydracache-server-admin/{label}-{}-{seq}",
            std::process::id()
        ))
    }

    fn file_url(path: PathBuf) -> String {
        format!("file://{}", path.to_string_lossy().replace('\\', "/"))
    }

    fn member_config() -> ServerConfig {
        ServerConfig {
            role: ServerRole::Member,
            listen_addr: "127.0.0.1:18080".parse().unwrap(),
            cluster_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: None,
            seeds: vec!["127.0.0.1:0".to_owned()],
            storage_dir: Some(test_path("member")),
            drain_timeout_ms: 1_000,
            tls: TlsConfig::default(),
            cluster_auth: ClusterAuthConfig::default(),
            backup: BackupConfig::default(),
            client_api: ClientApiConfig::default(),
            admin_api: AdminApiConfig::default(),
            ..ServerConfig::default()
        }
    }

    fn member_config_with_backup() -> ServerConfig {
        ServerConfig {
            backup: BackupConfig {
                enabled: true,
                location: Some(file_url(test_path("backups"))),
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
        assert_eq!(body["epoch"], 1);
        assert_eq!(body["quorum_ok"], true);
        assert_eq!(body["members"], 1);
        assert_eq!(body["reshard_phase"], "idle");
        assert_eq!(body["draining"], false);
    }

    #[tokio::test]
    async fn memory_footprint_is_secret_free_and_admin_only() {
        let runtime = ServerRuntime::new(local_config()).unwrap().start();
        runtime
            .cache()
            .put(
                "private-key",
                "private-value",
                hydracache::CacheOptions::new().tag("private-tag"),
            )
            .await
            .unwrap();
        let surface = AdminHttpSurface::new(runtime);

        let rejected = surface
            .routes()
            .oneshot(non_admin_request("GET", ADMIN_MEMORY_FOOTPRINT_PATH))
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::FORBIDDEN);

        let response = surface
            .routes()
            .oneshot(admin_request("GET", ADMIN_MEMORY_FOOTPRINT_PATH))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_response(response).await;
        assert_eq!(
            body["schema_version"],
            "hydracache-admin-memory-footprint-v1"
        );
        assert_eq!(body["embedded_cache"]["live_entries"], 1);
        let serialized = serde_json::to_string(&body).unwrap();
        assert!(!serialized.contains("private-key"));
        assert!(!serialized.contains("private-value"));
        assert!(!serialized.contains("private-tag"));
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
        assert!(html.contains("./assets/"));
        assert!(html.contains("type=\"module\""));

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
        assert!(javascript.contains("/management/v1/dashboard"));
        assert!(javascript.contains("MAX_RENDERED_MEMBERS"));

        let history = surface
            .routes()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/console/history.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(history.status(), StatusCode::OK);
        assert!(text_response(history).await.contains("HISTORY_LIMITS"));
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
        assert_eq!(backup.status(), StatusCode::ACCEPTED);
        let backup_body = json_response(backup).await;
        assert_eq!(backup_body["action"], "backup");
        assert_eq!(backup_body["outcome"], "accepted");
        assert_eq!(backup_body["authority"], "request_only");
        assert_eq!(backup_body["durable_artifact_created"], false);
        assert_eq!(backup_body["restore_point_available"], false);

        let first_drain = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_DRAIN_PATH))
            .await
            .unwrap();
        assert_eq!(first_drain.status(), StatusCode::OK);
        let first_body = json_response(first_drain).await;
        assert_eq!(first_body["action"], "drain");
        assert_eq!(first_body["outcome"], "accepted");

        let status_after_drain = surface
            .routes()
            .oneshot(admin_request("GET", ADMIN_STATUS_PATH))
            .await
            .unwrap();
        assert_eq!(status_after_drain.status(), StatusCode::OK);
        let status_body = json_response(status_after_drain).await;
        assert_eq!(status_body["draining"], true);

        let second_drain = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_DRAIN_PATH))
            .await
            .unwrap();
        assert_eq!(second_drain.status(), StatusCode::OK);
        let second_body = json_response(second_drain).await;
        assert_eq!(second_body["drain"]["remaining"], 0);
        assert_eq!(second_body["outcome"], "accepted");

        let kube_prestop_drain = surface
            .routes()
            .oneshot(admin_request("GET", ADMIN_DRAIN_PATH))
            .await
            .unwrap();
        assert_eq!(kube_prestop_drain.status(), StatusCode::OK);
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

    #[tokio::test]
    async fn raft_compaction_seam_is_observable_but_inert_by_default() {
        let surface = AdminHttpSurface::new(ServerRuntime::new(member_config()).unwrap().start());

        let before = surface
            .routes()
            .oneshot(admin_request("GET", ADMIN_RAFT_COMPACTION_PATH))
            .await
            .unwrap();
        assert_eq!(before.status(), StatusCode::OK);
        let before = json_response(before).await;
        assert_eq!(before["available"], true);
        assert_eq!(before["enabled"], false);
        assert_eq!(before["snapshot_index"], 0);
        assert!(before["applied_index"].as_u64().unwrap() > 0);
        assert_eq!(before["snapshot_sender_tasks_current"], 0);
        assert_eq!(before["snapshot_sender_tasks_high_water"], 0);

        let rejected = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_RAFT_COMPACTION_PATH))
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::CONFLICT);
        let rejected = json_response(rejected).await;
        assert_eq!(rejected["outcome"], "rejected");
        assert!(rejected["detail"]
            .as_str()
            .unwrap()
            .contains("HYDRACACHE_RAFT_COMPACTION=true"));

        let after = surface
            .routes()
            .oneshot(admin_request("GET", ADMIN_RAFT_COMPACTION_PATH))
            .await
            .unwrap();
        let after = json_response(after).await;
        assert_eq!(after["snapshot_index"], before["snapshot_index"]);
        assert_eq!(after["first_log_index"], before["first_log_index"]);
    }

    #[tokio::test]
    async fn explicitly_enabled_raft_compaction_seam_compacts_at_applied_progress() {
        let mut config = member_config();
        config.raft_compaction_enabled = true;
        let surface = AdminHttpSurface::new(ServerRuntime::new(config).unwrap().start());

        let compacted = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_RAFT_COMPACTION_PATH))
            .await
            .unwrap();
        assert_eq!(compacted.status(), StatusCode::OK);
        let compacted = json_response(compacted).await;
        assert_eq!(compacted["available"], true);
        assert_eq!(compacted["enabled"], true);
        assert_eq!(compacted["snapshot_index"], compacted["applied_index"]);
        assert!(compacted["snapshot_index"].as_u64().unwrap() > 0);
        assert_eq!(
            compacted["first_log_index"].as_u64().unwrap(),
            compacted["snapshot_index"].as_u64().unwrap() + 1
        );
    }

    #[tokio::test]
    async fn diagnostic_reset_is_off_by_default_and_returns_verified_zero_counts() {
        let disabled = AdminHttpSurface::new(ServerRuntime::new(local_config()).unwrap().start());
        let response = disabled
            .routes()
            .oneshot(admin_request("POST", ADMIN_DIAGNOSTIC_RESET_PATH))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let mut config = local_config();
        config.client_api.enabled = true;
        config.admin_api.diagnostic_reset_enabled = true;
        let runtime = ServerRuntime::new(config).unwrap().start();
        let state = runtime
            .client_dispatch_state()
            .expect("shared client state");
        let identity = ClientIdentity::new("diagnostic", "tenant").unwrap();
        let put = state.dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new(
                "put-before-reset",
                ClientRequest::Put {
                    ns: Namespace::new("diagnostic").unwrap(),
                    key: StructuredKey::new(vec!["key".to_owned()]).unwrap(),
                    value: b"value".to_vec(),
                    ttl_ms: None,
                    dimensions: Vec::new(),
                },
            )
            .with_idempotency_key("diagnostic-idempotency"),
        );
        assert!(put.result.is_ok());
        let surface = AdminHttpSurface::new(runtime);

        let response = surface
            .routes()
            .oneshot(admin_request("POST", ADMIN_DIAGNOSTIC_RESET_PATH))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = json_response(response).await;
        assert_eq!(body["action"], "diagnostic_reset");
        assert_eq!(body["outcome"], "completed");
        assert_eq!(body["embedded_after"], 0);
        assert_eq!(body["client"]["before"]["store_entries"], 1);
        assert_eq!(body["client"]["before"]["idempotency_outcomes"], 1);
        assert_eq!(body["client"]["after"]["store_entries"], 0);
        assert_eq!(body["client"]["after"]["idempotency_outcomes"], 0);
        assert_eq!(body["client"]["after"]["conditional"]["records"], 0);
        assert_eq!(state.retained_state_for_diagnostics().store_entries, 0);
    }

    #[test]
    fn diagnostic_reset_config_rejects_member_or_exposed_admin_modes() {
        let mut member = member_config();
        member.admin_api.diagnostic_reset_enabled = true;
        assert!(ServerRuntime::new(member).is_err());

        let mut exposed = local_config();
        exposed.admin_api.diagnostic_reset_enabled = true;
        exposed.admin_api.listen_addr = "0.0.0.0:9091".parse().unwrap();
        exposed.tls.acknowledge_insecure = true;
        assert!(ServerRuntime::new(exposed).is_err());
    }
}
