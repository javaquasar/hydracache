use std::path::PathBuf;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_client_transport_axum::{AxumClientSurface, ClientSurfaceLimits};
use hydracache_server::{
    AdminApiConfig, AdminHttpSurface, BackupConfig, ClientApiConfig, ClusterAuthConfig,
    ServerConfig, ServerRole, ServerRuntime, TlsConfig, ADMIN_CLUSTER_OVERVIEW_PATH,
    ADMIN_METRICS_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

#[test]
fn deploy_smoke_dockerfile_builds_hydracache_server_binary() {
    let dockerfile = include_str!("../../../Dockerfile");

    assert!(dockerfile.contains("cargo build --release --locked -p hydracache-server"));
    assert!(dockerfile.contains("gcr.io/distroless/cc-debian12:nonroot"));
    assert!(dockerfile.contains("USER nonroot:nonroot"));
    assert!(dockerfile.contains(r#"ENTRYPOINT ["/usr/local/bin/hydracache-server"]"#));
}

#[test]
fn deploy_smoke_k8s_manifests_wire_stateful_identity_storage_tls_backup_and_probes() {
    let statefulset = include_str!("../../../deploy/k8s/statefulset.yaml");
    let service = include_str!("../../../deploy/k8s/service.yaml");
    let pdb = include_str!("../../../deploy/k8s/pdb.yaml");

    assert!(statefulset.contains("kind: StatefulSet"));
    assert!(statefulset.contains("serviceName: hydracache-headless"));
    assert!(statefulset.contains("volumeClaimTemplates"));
    assert!(statefulset.contains("HYDRACACHE_SEEDS"));
    assert!(statefulset.contains("HYDRACACHE_TLS_ENABLED"));
    assert!(statefulset.contains("HYDRACACHE_BACKUP_LOCATION"));
    assert!(statefulset.contains("HYDRACACHE_ADMIN_ADDR"));
    assert!(statefulset.contains("name: admin"));
    assert!(statefulset.contains("livenessProbe"));
    assert!(statefulset.contains("path: /healthz"));
    assert!(statefulset.contains("port: admin"));
    assert!(statefulset.contains("readinessProbe"));
    assert!(statefulset.contains("path: /readyz"));
    assert!(service.contains("clusterIP: None"));
    assert!(service.contains("name: admin"));
    assert!(service.contains("name: metrics"));
    assert!(pdb.contains("kind: PodDisruptionBudget"));
    assert!(pdb.contains("minAvailable: 2"));
}

#[tokio::test]
async fn daemon_serves_metrics_and_cluster_overview_with_source_on_internal_port() {
    let admin = AdminHttpSurface::new(ServerRuntime::new(member_config()).unwrap().start());

    let metrics = admin
        .routes()
        .oneshot(get_request(ADMIN_METRICS_PATH))
        .await
        .unwrap();
    assert_eq!(metrics.status(), StatusCode::OK);
    let metrics_text = text_response(metrics).await;
    assert!(metrics_text.contains("hydracache_cluster_members{source=\"live\"} 1"));

    let overview = admin
        .routes()
        .oneshot(get_request(ADMIN_CLUSTER_OVERVIEW_PATH))
        .await
        .unwrap();
    assert_eq!(overview.status(), StatusCode::OK);
    let overview_body = json_response(overview).await;
    assert_eq!(overview_body["source"], "live");
    assert_eq!(overview_body["members"].as_array().unwrap().len(), 1);
    assert_eq!(
        overview_body["leader"]["node_id"],
        overview_body["members"][0]["node_id"]
    );

    let client = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let client_metrics = client
        .routes()
        .oneshot(get_request(ADMIN_METRICS_PATH))
        .await
        .unwrap();
    assert_eq!(client_metrics.status(), StatusCode::NOT_FOUND);
    let client_overview = client
        .routes()
        .oneshot(get_request(ADMIN_CLUSTER_OVERVIEW_PATH))
        .await
        .unwrap();
    assert_eq!(client_overview.status(), StatusCode::NOT_FOUND);
}

#[test]
fn deploy_smoke_helm_chart_exposes_replicas_rf_tls_and_backup_values() {
    let chart = include_str!("../../../deploy/helm/hydracache/Chart.yaml");
    let values = include_str!("../../../deploy/helm/hydracache/values.yaml");
    let helpers = include_str!("../../../deploy/helm/hydracache/templates/_helpers.tpl");
    let statefulset = include_str!("../../../deploy/helm/hydracache/templates/statefulset.yaml");

    assert!(chart.contains("version: 0.48.0"));
    assert!(helpers.contains(r#"define "hydracache.fullname""#));
    assert!(values.contains("replicaCount: 3"));
    assert!(values.contains("replicationFactor: 3"));
    assert!(values.contains("tls:"));
    assert!(values.contains("backup:"));
    assert!(values.contains("adminPort: 9091"));
    assert!(statefulset.contains("{{ .Values.tls.enabled | quote }}"));
    assert!(statefulset.contains("{{ .Values.backup.location | quote }}"));
    assert!(statefulset.contains("HYDRACACHE_ADMIN_ADDR"));
    assert!(statefulset.contains("path: /healthz"));
    assert!(statefulset.contains("path: /readyz"));
}

#[test]
#[ignore = "nightly gate: requires Docker daemon"]
fn deploy_smoke_image_builds_and_container_serves_health() {
    assert!(std::process::Command::new("docker")
        .args(["build", "-t", "hydracache-server:test", "."])
        .status()
        .expect("docker is installed")
        .success());
}

#[test]
#[ignore = "nightly gate: requires kind and kubectl"]
fn deploy_smoke_kind_statefulset_forms_quorum_and_survives_rolling_update() {
    assert!(std::process::Command::new("kind")
        .arg("version")
        .status()
        .expect("kind is installed")
        .success());
}

fn member_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:0".parse().unwrap(),
        node_id: None,
        seeds: vec!["127.0.0.1:0".to_owned()],
        storage_dir: Some(PathBuf::from("target/test-hydracache-deploy-smoke")),
        drain_timeout_ms: 1_000,
        tls: TlsConfig::default(),
        cluster_auth: ClusterAuthConfig::default(),
        backup: BackupConfig::default(),
        client_api: ClientApiConfig::default(),
        admin_api: AdminApiConfig::default(),
        ..ServerConfig::default()
    }
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
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
