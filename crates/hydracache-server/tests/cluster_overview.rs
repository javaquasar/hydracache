use std::path::PathBuf;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::ClusterGridCounters;
use hydracache_client_transport_axum::{AxumClientSurface, ClientSurfaceLimits};
use hydracache_server::{
    AdminApiConfig, AdminHttpSurface, BackupConfig, ClientApiConfig, ClusterAuthConfig,
    ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime, MemberRole, MemberStatus,
    Reachability, ReshardPhase, ServerConfig, ServerObservabilityModel, ServerRole, ServerRuntime,
    StatusSource, TlsConfig, ADMIN_CLUSTER_OVERVIEW_PATH,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn member_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:0".parse().unwrap(),
        node_id: None,
        seeds: vec!["127.0.0.1:0".to_owned()],
        storage_dir: Some(PathBuf::from("target/test-hydracache-cluster-overview")),
        drain_timeout_ms: 1_000,
        tls: TlsConfig::default(),
        cluster_auth: ClusterAuthConfig::default(),
        backup: BackupConfig::default(),
        client_api: ClientApiConfig::default(),
        admin_api: AdminApiConfig::default(),
        ..ServerConfig::default()
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

fn overview_request() -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(ADMIN_CLUSTER_OVERVIEW_PATH)
        .body(Body::empty())
        .unwrap()
}

async fn json_response(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get_overview(surface: &AdminHttpSurface) -> Value {
    let response = surface.routes().oneshot(overview_request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_response(response).await
}

fn surface_with(
    status: ClusterStatus,
    observability: ServerObservabilityModel,
) -> AdminHttpSurface {
    AdminHttpSurface::new(
        ServerRuntime::new(member_config())
            .unwrap()
            .with_cluster_status_provider(Arc::new(FakeStatusProvider { status }))
            .with_observability_model(observability)
            .start(),
    )
}

fn live_status() -> ClusterStatus {
    ClusterStatus {
        source: StatusSource::Live,
        leader: Some("node-2".to_owned()),
        term: 7,
        epoch: 42,
        quorum_ok: true,
        members: vec![
            member("node-1", Reachability::Reachable, 1),
            member("node-2", Reachability::Reachable, 2),
            member("node-3", Reachability::Reachable, 3),
        ],
        voters: 3,
        reshard_phase: ReshardPhase::Moving,
        draining: false,
    }
}

fn member(node_id: &str, reachable: Reachability, generation: u64) -> MemberStatus {
    MemberStatus {
        node_id: node_id.to_owned(),
        role: MemberRole::Member,
        reachable,
        generation,
    }
}

#[derive(Debug, Clone)]
struct FakeStatusProvider {
    status: ClusterStatus,
}

impl ClusterStatusProvider for FakeStatusProvider {
    fn cluster_status(&self, runtime: ClusterStatusRuntime) -> ClusterStatus {
        let mut status = self.status.clone();
        status.draining = status.draining || runtime.draining;
        status.quorum_ok = runtime.ready && status.quorum_ok && !status.draining;
        status
    }
}

mod cluster_overview {
    use super::*;

    #[tokio::test]
    async fn cluster_overview_aggregates_members_leader_partitions_consistency_backup_and_lifecycle(
    ) {
        let mut counters = ClusterGridCounters::default();
        counters.under_replicated_keys = 2;
        counters.consistency_level_operations_total = 9;
        let observability = ServerObservabilityModel::default()
            .with_cluster_grid_counters(counters)
            .with_partition_count(64)
            .with_configured_default_consistency("quorum")
            .with_backup_age_seconds(123)
            .with_upgrade_phase("prepared");
        let surface = surface_with(live_status(), observability);

        let body = get_overview(&surface).await;

        assert_eq!(body["source"], "live");
        assert_eq!(body["members"].as_array().unwrap().len(), 3);
        assert_eq!(body["members"][0]["node_id"], "node-1");
        assert_eq!(body["members"][0]["role"], "member");
        assert_eq!(body["members"][0]["reachable"], true);
        assert_eq!(body["members"][0]["generation"], 1);
        assert_eq!(body["leader"]["node_id"], "node-2");
        assert_eq!(body["leader"]["term"], 7);
        assert_eq!(body["leader"]["epoch"], 42);
        assert_eq!(body["partitions"]["under_replicated"], 2);
        assert_eq!(body["partitions"]["count"], 64);
        assert_eq!(body["consistency"]["configured_default"], "quorum");
        assert_eq!(
            body["consistency"]["op_counts_by_level"],
            json!([{ "level": "aggregate", "count": 9 }])
        );
        assert_eq!(body["backup_age_seconds"], 123);
        assert_eq!(body["lifecycle"]["reshard_phase"], "moving");
        assert_eq!(body["lifecycle"]["upgrade_phase"], "prepared");
    }

    #[tokio::test]
    async fn unreachable_member_is_shown_not_omitted() {
        let mut status = live_status();
        status.members[2].reachable = Reachability::Unreachable;
        let surface = surface_with(status, ServerObservabilityModel::default());

        let body = get_overview(&surface).await;

        let members = body["members"].as_array().unwrap();
        let node = members
            .iter()
            .find(|member| member["node_id"] == "node-3")
            .expect("unreachable member remains visible");
        assert_eq!(node["reachable"], false);
        assert_eq!(node["reachability"], "unreachable");
        assert_eq!(members.len(), 3);
    }

    #[tokio::test]
    async fn no_leader_during_election_is_null() {
        let mut status = live_status();
        status.leader = None;
        let surface = surface_with(status, ServerObservabilityModel::default());

        let body = get_overview(&surface).await;

        assert!(body["leader"].is_null());
        assert_eq!(body["source"], "live");
    }

    #[tokio::test]
    async fn consistency_is_distribution_not_single_current() {
        let mut counters = ClusterGridCounters::default();
        counters.consistency_level_operations_total = 11;
        let surface = surface_with(
            live_status(),
            ServerObservabilityModel::default()
                .with_configured_default_consistency("one")
                .with_cluster_grid_counters(counters),
        );

        let body = get_overview(&surface).await;

        assert!(body["consistency"].get("current").is_none());
        assert_eq!(body["consistency"]["configured_default"], "one");
        assert_eq!(
            body["consistency"]["op_counts_by_level"],
            json!([{ "level": "aggregate", "count": 11 }])
        );
    }

    #[tokio::test]
    async fn backup_age_is_oldest_namespace_or_null() {
        let empty = surface_with(live_status(), ServerObservabilityModel::default());
        let empty_body = get_overview(&empty).await;
        assert!(empty_body["backup_age_seconds"].is_null());

        let with_ages = surface_with(
            live_status(),
            ServerObservabilityModel::default()
                .with_backup_age_seconds_from_namespaces([45, 180, 90]),
        );
        let body = get_overview(&with_ages).await;
        assert_eq!(body["backup_age_seconds"], 180);
    }

    #[tokio::test]
    async fn modeled_source_is_carried_through_to_overview() {
        let surface = AdminHttpSurface::new(ServerRuntime::new(local_config()).unwrap().start());

        let body = get_overview(&surface).await;

        assert_eq!(body["source"], "modeled");
        assert!(body["leader"].is_null());
        assert_eq!(body["members"], json!([]));
        assert_eq!(
            body["partitions"],
            json!({ "under_replicated": 0, "count": 0 })
        );
        assert!(body["backup_age_seconds"].is_null());
    }

    #[tokio::test]
    async fn cluster_overview_is_on_admin_port_not_client_port() {
        let admin = AdminHttpSurface::new(ServerRuntime::new(member_config()).unwrap().start());
        let admin_response = admin.routes().oneshot(overview_request()).await.unwrap();
        assert_eq!(admin_response.status(), StatusCode::OK);

        let client = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
        let client_response = client.routes().oneshot(overview_request()).await.unwrap();
        assert_eq!(client_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn cluster_overview_json_shape_is_stable() {
        let mut counters = ClusterGridCounters::default();
        counters.under_replicated_keys = 1;
        counters.consistency_level_operations_total = 3;
        let mut status = live_status();
        status.members = vec![member("node-1", Reachability::Suspect, 10)];
        status.leader = Some("node-1".to_owned());
        status.term = 8;
        status.epoch = 43;
        status.reshard_phase = ReshardPhase::Finalizing;
        let observability = ServerObservabilityModel::default()
            .with_cluster_grid_counters(counters)
            .with_partition_count(16)
            .with_configured_default_consistency("all")
            .with_backup_age_seconds_from_namespaces([20, 5])
            .with_upgrade_phase("old_draining");
        let surface = surface_with(status, observability);

        let body = get_overview(&surface).await;

        assert_eq!(
            body,
            json!({
                "source": "live",
                "members": [{
                    "node_id": "node-1",
                    "role": "member",
                    "reachable": false,
                    "reachability": "suspect",
                    "generation": 10
                }],
                "leader": {
                    "node_id": "node-1",
                    "term": 8,
                    "epoch": 43
                },
                "partitions": {
                    "under_replicated": 1,
                    "count": 16
                },
                "consistency": {
                    "configured_default": "all",
                    "op_counts_by_level": [{
                        "level": "aggregate",
                        "count": 3
                    }]
                },
                "backup_age_seconds": 20,
                "lifecycle": {
                    "reshard_phase": "finalizing",
                    "upgrade_phase": "old_draining"
                }
            })
        );
    }
}
