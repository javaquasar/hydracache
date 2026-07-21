use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::{write_full_backup, BackupDataset, InMemoryObjectStore};
use hydracache_client_protocol::{ClientRequest, ClientRequestEnvelope, Namespace, StructuredKey};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceLimits, ClientSurfaceState, HYDRACACHE_ADMIN_HEADER,
    HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{
    AdminApiConfig, AdminHttpSurface, BackupConfig, ClientApiConfig, ClusterAuthConfig,
    ServerConfig, ServerRole, ServerRuntime, TlsConfig, ADMIN_BACKUP_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn test_path(label: &str) -> PathBuf {
    let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    PathBuf::from(format!(
        "target/test-hydracache-server-backup-boundary/{label}-{}-{sequence}",
        std::process::id()
    ))
}

fn file_url(path: &std::path::Path) -> String {
    format!("file://{}", path.to_string_lossy().replace('\\', "/"))
}

fn member_config(backup_path: &std::path::Path) -> ServerConfig {
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
        backup: BackupConfig {
            enabled: true,
            location: Some(file_url(backup_path)),
        },
        client_api: ClientApiConfig::default(),
        admin_api: AdminApiConfig::default(),
        ..ServerConfig::default()
    }
}

fn admin_backup_request() -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(ADMIN_BACKUP_PATH)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Body::empty())
        .unwrap()
}

async fn accepted_backup_response() -> (StatusCode, Value, PathBuf) {
    let backup_path = test_path("unwritten-backup-target");
    let surface = AdminHttpSurface::new(
        ServerRuntime::new(member_config(&backup_path))
            .unwrap()
            .start(),
    );
    let response = surface
        .routes()
        .oneshot(admin_backup_request())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap();
    (status, body, backup_path)
}

#[tokio::test]
async fn accepted_backup_request_is_not_a_durable_artifact_or_restore_point() {
    let (status, body, backup_path) = accepted_backup_response().await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["action"], "backup");
    assert_eq!(body["outcome"], "accepted");
    assert_eq!(body["authority"], "request_only");
    assert_eq!(body["durable_artifact_created"], false);
    assert_eq!(body["restore_point_available"], false);
    assert!(body.get("manifest_key").is_none());
    assert!(body.get("restore_completed").is_none());
    assert!(
        !backup_path.exists(),
        "request-only acceptance must not create a backup target behind the API boundary"
    );
}

#[test]
fn backup_dataset_values_are_only_caller_supplied() {
    let live_state = ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap();
    let identity = ClientIdentity::new("client-a", "tenant-a").unwrap();
    let response = live_state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "put-live-value",
            ClientRequest::Put {
                ns: Namespace::new("users").unwrap(),
                key: StructuredKey::new(vec!["live-key".to_owned()]).unwrap(),
                value: b"live-value".to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ),
    );
    assert!(response.result.is_ok());
    assert_eq!(live_state.state_mutations(), 1);

    let dataset = BackupDataset::new(b"caller-control-plane".to_vec());
    let mut store = InMemoryObjectStore::default();
    let manifest = write_full_backup(&mut store, "caller-dataset", 7, &dataset).unwrap();

    assert!(dataset.values.is_empty());
    assert!(
        manifest.values.is_empty(),
        "backup helper must persist only values explicitly supplied by its caller"
    );
}

#[tokio::test]
async fn canary_backup_request_acceptance_is_treated_as_completed_backup() {
    let (_, body, _) = accepted_backup_response().await;
    let defect_enabled = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W4");
    let interpreted_as_durable = if defect_enabled {
        body["outcome"] == "accepted"
    } else {
        body["durable_artifact_created"] == true
    };

    assert!(
        !interpreted_as_durable,
        "HC-CANARY-RED:W4 backup request acceptance was treated as a completed backup"
    );
}
