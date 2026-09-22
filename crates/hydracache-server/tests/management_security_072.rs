use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{
    AdminHttpSurface, BackupConfig, ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime,
    MemberRole, MemberStatus, Reachability, ReshardPhase, ServerConfig, ServerRole, ServerRuntime,
    StatusSource, ADMIN_BACKUP_PATH, ADMIN_CONSOLE_PATH, HYDRACACHE_MANAGEMENT_READ_HEADER,
    MANAGEMENT_AUDIT_PATH, MANAGEMENT_CAPABILITIES_PATH, MANAGEMENT_CLIENTS_PATH,
    MANAGEMENT_CLUSTER_FORMATION_PATH, MANAGEMENT_CLUSTER_MEMBERS_PATH,
    MANAGEMENT_CLUSTER_PARTITIONS_PATH, MANAGEMENT_CONSENSUS_PROGRESS_PATH,
    MANAGEMENT_DASHBOARD_PATH, MANAGEMENT_FORMATION_PATH, MANAGEMENT_HEALTHCHECKS_PATH,
    MANAGEMENT_NAMESPACES_PATH, MANAGEMENT_OPERATIONS_PATH, MANAGEMENT_PERSISTENCE_PATH,
    MANAGEMENT_READ_MAX_CONCURRENCY, MANAGEMENT_RECOVERY_PATH,
};
use tower::ServiceExt;

#[derive(Clone, Copy)]
enum IdentityCapability {
    None,
    TenantReader,
    ManagementReader,
    WriteAdmin,
}

fn config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Local,
        seeds: Vec::new(),
        storage_dir: None,
        backup: BackupConfig {
            enabled: true,
            location: Some("memory://test-only".to_owned()),
        },
        ..ServerConfig::default()
    }
}

fn request(method: Method, uri: &str, capability: IdentityCapability) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if !matches!(capability, IdentityCapability::None) {
        builder = builder
            .header(HYDRACACHE_CLIENT_ID_HEADER, "reader")
            .header(HYDRACACHE_TENANT_HEADER, "tenant-a");
    }
    if matches!(capability, IdentityCapability::ManagementReader) {
        builder = builder.header(HYDRACACHE_MANAGEMENT_READ_HEADER, "true");
    }
    if matches!(capability, IdentityCapability::WriteAdmin) {
        builder = builder.header(HYDRACACHE_ADMIN_HEADER, "true");
    }
    builder.body(Body::empty()).unwrap()
}

fn management_paths() -> [&'static str; 17] {
    [
        MANAGEMENT_CAPABILITIES_PATH,
        MANAGEMENT_DASHBOARD_PATH,
        MANAGEMENT_FORMATION_PATH,
        MANAGEMENT_CLUSTER_FORMATION_PATH,
        MANAGEMENT_CLUSTER_MEMBERS_PATH,
        MANAGEMENT_CLUSTER_PARTITIONS_PATH,
        MANAGEMENT_CLIENTS_PATH,
        MANAGEMENT_NAMESPACES_PATH,
        MANAGEMENT_HEALTHCHECKS_PATH,
        MANAGEMENT_PERSISTENCE_PATH,
        MANAGEMENT_OPERATIONS_PATH,
        MANAGEMENT_AUDIT_PATH,
        MANAGEMENT_RECOVERY_PATH,
        MANAGEMENT_CONSENSUS_PROGRESS_PATH,
        "/management/v1/history?query_id=cache_entries&start_ms=1000&end_ms=61000&step_ms=10000",
        "/management/v1/namespaces/private/caches",
        "/management/v1/cluster/placement-traces/opaque",
    ]
}

#[tokio::test]
async fn authorization_matrix_separates_management_read_tenant_read_and_write_admin() {
    let router = AdminHttpSurface::new(ServerRuntime::new(config()).unwrap().start()).routes();
    for path in management_paths() {
        let unauthenticated = router
            .clone()
            .oneshot(request(Method::GET, path, IdentityCapability::None))
            .await
            .unwrap();
        assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED, "{path}");

        let tenant_only = router
            .clone()
            .oneshot(request(Method::GET, path, IdentityCapability::TenantReader))
            .await
            .unwrap();
        assert_eq!(tenant_only.status(), StatusCode::FORBIDDEN, "{path}");

        for capability in [
            IdentityCapability::ManagementReader,
            IdentityCapability::WriteAdmin,
        ] {
            let response = router
                .clone()
                .oneshot(request(Method::GET, path, capability))
                .await
                .unwrap();
            assert_ne!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
            assert_ne!(response.status(), StatusCode::FORBIDDEN, "{path}");
        }
    }

    let reader_write = router
        .oneshot(request(
            Method::POST,
            ADMIN_BACKUP_PATH,
            IdentityCapability::ManagementReader,
        ))
        .await
        .unwrap();
    assert_eq!(reader_write.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn console_and_json_responses_ship_restrictive_security_headers() {
    let router = AdminHttpSurface::new(ServerRuntime::new(config()).unwrap().start()).routes();
    for path in [
        ADMIN_CONSOLE_PATH,
        "/console/app.js",
        "/console/history.js",
        "/console/style.css",
    ] {
        let response = router
            .clone()
            .oneshot(request(Method::GET, path, IdentityCapability::None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        let csp = headers["content-security-policy"].to_str().unwrap();
        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert_eq!(headers["x-frame-options"], "DENY");
        assert_eq!(headers["referrer-policy"], "no-referrer");
        assert_eq!(headers["cache-control"], "no-store");
    }

    let response = router
        .oneshot(request(
            Method::GET,
            MANAGEMENT_DASHBOARD_PATH,
            IdentityCapability::ManagementReader,
        ))
        .await
        .unwrap();
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(response.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .contains("default-src 'none'"));
}

#[tokio::test]
async fn management_saturation_is_fail_fast_and_does_not_consume_write_admin_capacity() {
    let surface = AdminHttpSurface::new(ServerRuntime::new(config()).unwrap().start());
    let limiter = surface.management_read_limiter();
    let permits = (0..MANAGEMENT_READ_MAX_CONCURRENCY)
        .map(|_| limiter.try_acquire().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(limiter.available_permits(), 0);
    let router = surface.routes();

    let saturated = router
        .clone()
        .oneshot(request(
            Method::GET,
            MANAGEMENT_DASHBOARD_PATH,
            IdentityCapability::ManagementReader,
        ))
        .await
        .unwrap();
    assert_eq!(saturated.status(), StatusCode::TOO_MANY_REQUESTS);

    let write = router
        .clone()
        .oneshot(request(
            Method::POST,
            ADMIN_BACKUP_PATH,
            IdentityCapability::WriteAdmin,
        ))
        .await
        .unwrap();
    assert_eq!(write.status(), StatusCode::ACCEPTED);

    drop(permits);
    assert_eq!(limiter.available_permits(), MANAGEMENT_READ_MAX_CONCURRENCY);
    let recovered = router
        .oneshot(request(
            Method::GET,
            MANAGEMENT_DASHBOARD_PATH,
            IdentityCapability::ManagementReader,
        ))
        .await
        .unwrap();
    assert_eq!(recovered.status(), StatusCode::OK);
}

#[derive(Debug)]
struct HostileStatus;

impl ClusterStatusProvider for HostileStatus {
    fn cluster_status(&self, runtime: ClusterStatusRuntime) -> ClusterStatus {
        ClusterStatus {
            source: StatusSource::Live,
            leader: Some("<script>window.__hc_xss=1</script>".to_owned()),
            term: 1,
            epoch: 1,
            observation_seq: 1,
            metadata_authoritative: true,
            quorum_ok: !runtime.draining,
            members: vec![MemberStatus {
                node_id: "<img src=x onerror=window.__hc_xss=1>".to_owned(),
                role: MemberRole::Member,
                reachable: Reachability::Reachable,
                generation: 1,
            }],
            voters: 1,
            voter_ids: vec![1],
            reshard_phase: ReshardPhase::Idle,
            draining: runtime.draining,
            local_consensus: None,
        }
    }
}

#[tokio::test]
async fn canary_xss_and_management_auth_bypass_are_both_rejected() {
    let provider: Arc<dyn ClusterStatusProvider> = Arc::new(HostileStatus);
    let router = AdminHttpSurface::new(
        ServerRuntime::new(config())
            .unwrap()
            .with_cluster_status_provider(provider)
            .start(),
    )
    .routes();
    let unauthorized = router
        .clone()
        .oneshot(request(
            Method::GET,
            MANAGEMENT_CLUSTER_MEMBERS_PATH,
            IdentityCapability::TenantReader,
        ))
        .await
        .unwrap();
    let payload = router
        .oneshot(request(
            Method::GET,
            MANAGEMENT_CLUSTER_MEMBERS_PATH,
            IdentityCapability::ManagementReader,
        ))
        .await
        .unwrap();
    let body = to_bytes(payload.into_body(), 1024 * 1024).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    let defect =
        std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W11-XSS-AUTH-BYPASS");
    assert!(
        (!defect && unauthorized.status() == StatusCode::FORBIDDEN)
            && !body.contains("<script>")
            && !body.contains("<img"),
        "HC-CANARY-RED:MC72-W11-XSS-AUTH-BYPASS hostile identity escaped pseudonymization or management.read guard was bypassed"
    );
}
