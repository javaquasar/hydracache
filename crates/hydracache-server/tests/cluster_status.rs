use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use hydracache::{
    ClusterEndpoints, ClusterEpoch, ClusterGeneration, ClusterMember, ClusterNodeId, ClusterRole,
    RaftMetadataSnapshot,
};
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{
    AdminApiConfig, AdminHttpSurface, BackupConfig, ClientApiConfig, ClusterAuthConfig,
    ClusterStatusProvider, ClusterStatusRuntime, GridControlPlaneHandle, LiveClusterStatus,
    Reachability, ReshardPhase, ServerConfig, ServerRole, ServerRuntime, StatusSource, TlsConfig,
    ADMIN_STATUS_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

fn member_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:0".parse().unwrap(),
        node_id: None,
        seeds: vec!["127.0.0.1:0".to_owned()],
        storage_dir: Some(PathBuf::from("target/test-hydracache-server-status")),
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

fn admin_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Body::empty())
        .unwrap()
}

async fn json_response(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

mod cluster_status {
    use super::*;

    #[test]
    fn modeled_status_is_tagged_modeled_and_never_live() {
        let runtime = ServerRuntime::new(local_config()).unwrap().start();

        let status = runtime.admin_status();

        assert_eq!(status.source, StatusSource::Modeled);
        assert_eq!(status.leader.as_deref(), Some("local"));
        assert_eq!(status.members, 0);
        assert_eq!(status.term, 1);
        assert!(status.quorum_ok);
    }

    #[test]
    fn live_status_reports_real_members_term_and_epoch() {
        let provider = live_provider(FakeGrid::three_members());
        let runtime = ServerRuntime::new(member_config())
            .unwrap()
            .with_cluster_status_provider(provider)
            .start();

        let status = runtime.admin_status();

        assert_eq!(status.source, StatusSource::Live);
        assert_eq!(status.leader.as_deref(), Some("node-2"));
        assert_eq!(status.term, 7);
        assert_eq!(status.members, 3);
        assert_eq!(status.reshard_phase, "moving");
        assert!(status.quorum_ok);
    }

    #[test]
    fn no_leader_during_election_is_none_not_stale() {
        let provider = live_provider(FakeGrid {
            leader: None,
            ..FakeGrid::three_members()
        });
        let runtime = ServerRuntime::new(member_config())
            .unwrap()
            .with_cluster_status_provider(provider)
            .start();

        let status = runtime.admin_status();

        assert_eq!(status.source, StatusSource::Live);
        assert_eq!(status.leader, None);
    }

    #[test]
    fn unreachable_member_is_present_with_unreachable_flag() {
        let provider = live_provider(FakeGrid {
            unreachable: BTreeSet::from([ClusterNodeId::from("node-3")]),
            ..FakeGrid::three_members()
        });

        let status = provider.cluster_status(ClusterStatusRuntime::new(true, false));

        let node = status
            .members
            .iter()
            .find(|member| member.node_id == "node-3")
            .expect("unreachable member remains visible");
        assert_eq!(node.reachable, Reachability::Unreachable);
        assert_eq!(status.members.len(), 3);
    }

    #[test]
    fn draining_sets_draining_and_quorum_false() {
        let provider = live_provider(FakeGrid::three_members());

        let status = provider.cluster_status(ClusterStatusRuntime::new(true, true));

        assert!(status.draining);
        assert!(!status.quorum_ok);
    }

    #[tokio::test]
    async fn admin_status_json_includes_source_field() {
        let surface = AdminHttpSurface::new(ServerRuntime::new(local_config()).unwrap().start());

        let response = surface
            .routes()
            .oneshot(admin_request(ADMIN_STATUS_PATH))
            .await
            .unwrap();

        let body = json_response(response).await;
        assert_eq!(body["source"], "modeled");
        assert_eq!(body["members"], 0);
    }
}

fn live_provider(grid: FakeGrid) -> Arc<dyn ClusterStatusProvider> {
    Arc::new(LiveClusterStatus::new(Arc::new(grid)))
}

#[derive(Debug, Clone)]
struct FakeGrid {
    snapshot: RaftMetadataSnapshot,
    members: Vec<ClusterMember>,
    leader: Option<String>,
    unreachable: BTreeSet<ClusterNodeId>,
    quorum: bool,
    phase: ReshardPhase,
    draining: bool,
}

impl FakeGrid {
    fn three_members() -> Self {
        Self {
            snapshot: RaftMetadataSnapshot {
                term: 7,
                commit_index: 9,
                epoch: ClusterEpoch::new(42),
                member_count: 3,
                client_count: 0,
                last_command: None,
            },
            members: vec![
                member("node-1", 1),
                member("node-2", 2),
                member("node-3", 3),
            ],
            leader: Some("node-2".to_owned()),
            unreachable: BTreeSet::new(),
            quorum: true,
            phase: ReshardPhase::Moving,
            draining: false,
        }
    }
}

impl GridControlPlaneHandle for FakeGrid {
    fn begin_drain(&self) {}

    fn snapshot(&self) -> RaftMetadataSnapshot {
        self.snapshot.clone()
    }

    fn members(&self) -> Vec<ClusterMember> {
        self.members.clone()
    }

    fn raft_leader_id(&self) -> Option<String> {
        self.leader.clone()
    }

    fn has_quorum(&self) -> bool {
        self.quorum
    }

    fn reachability(&self, node: &ClusterNodeId) -> Reachability {
        if self.unreachable.contains(node) {
            Reachability::Unreachable
        } else {
            Reachability::Reachable
        }
    }

    fn reshard_phase(&self) -> ReshardPhase {
        self.phase
    }

    fn is_draining(&self) -> bool {
        self.draining
    }
}

fn member(node_id: &str, generation: u64) -> ClusterMember {
    ClusterMember {
        node_id: ClusterNodeId::from(node_id),
        generation: ClusterGeneration::new(generation),
        role: ClusterRole::Member,
        epoch: ClusterEpoch::new(42),
        endpoints: ClusterEndpoints::default(),
        metadata: BTreeMap::new(),
    }
}
