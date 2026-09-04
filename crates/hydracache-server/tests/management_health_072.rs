use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_observability::{
    BoundedPlacementConstraints, ManagementCompleteness, ManagementObservationSource,
    ManagementPartitionSnapshot, OpaqueNodeIdentity, PartitionMemberCount,
    PlacementCandidateDecision, PlacementCandidatePage, PlacementDecisionTrace, PlacementOutcome,
    MANAGEMENT_API_SCHEMA_VERSION,
};
use hydracache_server::{
    AdminHttpSurface, ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime,
    LocalConsensusStatus, ManagementTopologyModel, MemberRole, MemberStatus, Reachability,
    ReshardPhase, ServerConfig, ServerRuntime, StatusSource, MANAGEMENT_HEALTHCHECKS_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

#[derive(Debug)]
struct FixedStatus(ClusterStatus);

impl ClusterStatusProvider for FixedStatus {
    fn cluster_status(&self, _runtime: ClusterStatusRuntime) -> ClusterStatus {
        self.0.clone()
    }
}

fn status() -> ClusterStatus {
    ClusterStatus {
        source: StatusSource::Live,
        leader: Some("health-node-secret".to_owned()),
        term: 2,
        epoch: 72,
        observation_seq: 44,
        metadata_authoritative: true,
        quorum_ok: true,
        members: vec![MemberStatus {
            node_id: "health-node-secret".to_owned(),
            role: MemberRole::Member,
            reachable: Reachability::Reachable,
            generation: 8,
        }],
        voters: 1,
        voter_ids: vec![1],
        reshard_phase: ReshardPhase::Idle,
        draining: false,
        local_consensus: Some(LocalConsensusStatus {
            node_id: "health-node-secret".to_owned(),
            generation: 8,
            voter: true,
            commit_index: 500,
            applied_index: 500,
            last_snapshot_index: Some(400),
            catch_up_target: 500,
        }),
    }
}

fn topology() -> ManagementTopologyModel {
    let trace = PlacementDecisionTrace {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        trace_id: "health-trace-72".to_owned(),
        topology_epoch: 72,
        request_digest: "health-request-digest".to_owned(),
        requested_replicas: 1,
        constraints: BoundedPlacementConstraints::default(),
        candidates: PlacementCandidatePage {
            items: vec![PlacementCandidateDecision {
                node: OpaqueNodeIdentity::new("opaque-health-node"),
                selected: true,
                reasons: Vec::new(),
            }],
            truncated: false,
        },
        selected: vec![OpaqueNodeIdentity::new("opaque-health-node")],
        tie_break_seed: Some(72),
        outcome: PlacementOutcome::Applied,
        commit_index: Some(500),
        applied_index: Some(500),
        reason: None,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    };
    let partition = ManagementPartitionSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        authority_epoch: Some(72),
        observation_seq: 44,
        total: 1,
        assigned: Some(1),
        unassigned: Some(0),
        distribution: vec![PartitionMemberCount {
            node: OpaqueNodeIdentity::new("opaque-health-node"),
            primary: 1,
            backup: 0,
        }],
        under_replicated: 0,
        zone_underspread: 0,
        repair_debt: 0,
        reshard_phase: "idle".to_owned(),
        reshard_moves_inflight: 0,
        backfill_lag: 0,
        placement_trace_id: Some("health-trace-72".to_owned()),
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    };
    ManagementTopologyModel::new()
        .with_partition_snapshot(partition)
        .unwrap()
        .with_placement_trace(trace)
        .unwrap()
}

fn surface() -> AdminHttpSurface {
    let provider: Arc<dyn ClusterStatusProvider> = Arc::new(FixedStatus(status()));
    AdminHttpSurface::new(
        ServerRuntime::new(ServerConfig::default())
            .unwrap()
            .with_cluster_status_provider(provider)
            .with_management_topology_model(topology())
            .start(),
    )
}

fn request(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
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

#[tokio::test]
async fn server_evaluates_checks_and_retains_unknown_separately() {
    let response = surface()
        .routes()
        .oneshot(request(MANAGEMENT_HEALTHCHECKS_PATH))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["data"]["aggregate"], "PASS");
    assert_eq!(body["data"]["counts"]["pass"], 8);
    assert_eq!(body["data"]["counts"]["unknown"], 10);
    assert_eq!(
        body["data"]["checks"]["items"].as_array().unwrap().len(),
        18
    );
    assert_eq!(
        body["data"]["thresholds"]["raft_apply_lag_warn_entries"],
        100
    );
    let encoded = body.to_string();
    assert!(!encoded.contains("health-node-secret"));
}

#[tokio::test]
async fn category_status_filters_and_cursor_are_server_side_and_stable() {
    let router = surface().routes();
    let unknown = json(
        router
            .clone()
            .oneshot(request(&format!(
                "{MANAGEMENT_HEALTHCHECKS_PATH}?status=UNKNOWN"
            )))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(unknown["data"]["counts"]["unknown"], 10);
    assert!(unknown["data"]["checks"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|check| check["status"] == "UNKNOWN"));

    let recovery = json(
        router
            .clone()
            .oneshot(request(&format!(
                "{MANAGEMENT_HEALTHCHECKS_PATH}?category=recovery&limit=2"
            )))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        recovery["data"]["checks"]["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(recovery["data"]["checks"]["truncated"], true);
    let cursor = recovery["data"]["checks"]["next_cursor"].as_str().unwrap();
    let second = json(
        router
            .oneshot(request(&format!(
                "{MANAGEMENT_HEALTHCHECKS_PATH}?category=recovery&limit=2&cursor={cursor}"
            )))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        second["data"]["checks"]["items"].as_array().unwrap().len(),
        1
    );
    assert_eq!(second["data"]["checks"]["truncated"], false);
}

#[tokio::test]
async fn authentication_and_unknown_query_rejection_precede_health_disclosure() {
    let router = surface().routes();
    let anonymous = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(MANAGEMENT_HEALTHCHECKS_PATH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    let invalid = router
        .oneshot(request(&format!(
            "{MANAGEMENT_HEALTHCHECKS_PATH}?threshold=0"
        )))
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
}
