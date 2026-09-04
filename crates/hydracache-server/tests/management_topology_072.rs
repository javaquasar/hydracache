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
    PlacementReasonCode, MANAGEMENT_API_SCHEMA_VERSION,
};
use hydracache_server::{
    normalize_placement_trace, AdminHttpSurface, ClusterStatus, ClusterStatusProvider,
    ClusterStatusRuntime, LocalConsensusStatus, ManagementTopologyModel, MemberRole, MemberStatus,
    Reachability, ReshardPhase, ServerConfig, ServerRuntime, StatusSource,
    MANAGEMENT_CLUSTER_FORMATION_PATH, MANAGEMENT_CLUSTER_MEMBERS_PATH,
    MANAGEMENT_CLUSTER_PARTITIONS_PATH, MANAGEMENT_PLACEMENT_TRACE_PREFIX,
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
        leader: Some("raw-local-secret".to_owned()),
        term: 3,
        epoch: 71,
        observation_seq: 101,
        metadata_authoritative: true,
        quorum_ok: true,
        members: vec![
            MemberStatus {
                node_id: "raw-local-secret".to_owned(),
                role: MemberRole::Member,
                reachable: Reachability::Reachable,
                generation: 4,
            },
            MemberStatus {
                node_id: "raw-remote-secret".to_owned(),
                role: MemberRole::Member,
                reachable: Reachability::Unreachable,
                generation: 9,
            },
        ],
        voters: 2,
        voter_ids: vec![1, 2],
        reshard_phase: ReshardPhase::Moving,
        draining: false,
        local_consensus: Some(LocalConsensusStatus {
            node_id: "raw-local-secret".to_owned(),
            generation: 4,
            voter: true,
            commit_index: 50,
            applied_index: 49,
            last_snapshot_index: Some(40),
            catch_up_target: 50,
        }),
    }
}

fn partition(epoch: u64, sequence: u64) -> ManagementPartitionSnapshot {
    ManagementPartitionSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        authority_epoch: Some(epoch),
        observation_seq: sequence,
        total: 16,
        assigned: Some(16),
        unassigned: Some(0),
        distribution: vec![
            PartitionMemberCount {
                node: OpaqueNodeIdentity::new("member-a"),
                primary: 8,
                backup: 8,
            },
            PartitionMemberCount {
                node: OpaqueNodeIdentity::new("member-b"),
                primary: 8,
                backup: 8,
            },
        ],
        under_replicated: 1,
        zone_underspread: 2,
        repair_debt: 3,
        reshard_phase: "moving".to_owned(),
        reshard_moves_inflight: 4,
        backfill_lag: 5,
        placement_trace_id: Some("trace_Opaque-71".to_owned()),
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
}

fn trace(candidates: Vec<PlacementCandidateDecision>) -> PlacementDecisionTrace {
    PlacementDecisionTrace {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        trace_id: "trace_Opaque-71".to_owned(),
        topology_epoch: 71,
        request_digest: "sha256-v1:request".to_owned(),
        requested_replicas: 1,
        constraints: BoundedPlacementConstraints {
            required_labels: vec!["ssd".to_owned()],
            excluded_labels: Vec::new(),
            required_zones: 1,
            unique_hosts: true,
            truncated: false,
        },
        candidates: PlacementCandidatePage {
            items: candidates,
            truncated: false,
        },
        selected: vec![OpaqueNodeIdentity::new("member-b")],
        tie_break_seed: Some(9),
        outcome: PlacementOutcome::Committed,
        commit_index: Some(50),
        applied_index: Some(49),
        reason: None,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Complete,
    }
}

fn candidates() -> Vec<PlacementCandidateDecision> {
    vec![
        PlacementCandidateDecision {
            node: OpaqueNodeIdentity::new("member-a"),
            selected: false,
            reasons: vec![PlacementReasonCode::ZoneConflict],
        },
        PlacementCandidateDecision {
            node: OpaqueNodeIdentity::new("member-b"),
            selected: true,
            reasons: Vec::new(),
        },
    ]
}

fn surface(model: ManagementTopologyModel) -> AdminHttpSurface {
    let provider: Arc<dyn ClusterStatusProvider> = Arc::new(FixedStatus(status()));
    AdminHttpSurface::new(
        ServerRuntime::new(ServerConfig::default())
            .unwrap()
            .with_cluster_status_provider(provider)
            .with_management_topology_model(model)
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
async fn members_are_committed_epoch_fenced_pseudonymous_and_platform_unknown_is_null() {
    let response = surface(ManagementTopologyModel::new())
        .routes()
        .oneshot(request(MANAGEMENT_CLUSTER_MEMBERS_PATH))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| item["authority_epoch"] == 71));
    assert!(items.iter().all(|item| item["cpu_percent"].is_null()));
    assert!(items.iter().all(|item| item["rss_bytes"].is_null()));
    let local = items.iter().find(|item| item["generation"] == 4).unwrap();
    assert!(local["product_version"].is_string());
    assert!(local["config_digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256-v1:"));
    let remote = items.iter().find(|item| item["generation"] == 9).unwrap();
    assert_eq!(remote["formation"]["serving"], "blocked");
    assert_eq!(remote["reachability_reason"], "transport-unreachable");
    assert!(remote["uptime_seconds"].is_null());
    let encoded = body.to_string();
    assert!(!encoded.contains("raw-local-secret"));
    assert!(!encoded.contains("raw-remote-secret"));
}

#[tokio::test]
async fn cluster_formation_alias_and_all_new_routes_authenticate_before_source_use() {
    let router = surface(ManagementTopologyModel::new()).routes();
    let formation = router
        .clone()
        .oneshot(request(MANAGEMENT_CLUSTER_FORMATION_PATH))
        .await
        .unwrap();
    assert_eq!(formation.status(), StatusCode::OK);
    for path in [
        MANAGEMENT_CLUSTER_MEMBERS_PATH,
        MANAGEMENT_CLUSTER_PARTITIONS_PATH,
        &format!("{MANAGEMENT_PLACEMENT_TRACE_PREFIX}/trace_Opaque-71"),
    ] {
        let anonymous = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn partitions_and_trace_are_returned_only_for_the_exact_authority_observation() {
    let model = ManagementTopologyModel::new()
        .with_partition_snapshot(partition(71, 101))
        .unwrap()
        .with_placement_trace(trace(candidates()))
        .unwrap();
    let router = surface(model).routes();
    let partitions = json(
        router
            .clone()
            .oneshot(request(MANAGEMENT_CLUSTER_PARTITIONS_PATH))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(partitions["completeness"], "complete");
    assert_eq!(partitions["data"]["assigned"], 16);
    assert_eq!(partitions["data"]["distribution"][0]["primary"], 8);

    let trace = json(
        router
            .clone()
            .oneshot(request(&format!(
                "{MANAGEMENT_PLACEMENT_TRACE_PREFIX}/trace_Opaque-71"
            )))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(trace["data"]["outcome"], "committed");
    assert_eq!(trace["data"]["commit_index"], 50);
    assert_eq!(trace["data"]["applied_index"], 49);
    assert_eq!(trace["data"]["candidates"]["items"][0]["selected"], true);

    for id in ["unknown", "%3Cscript%3E", "..%2Fsecret"] {
        let response = router
            .clone()
            .oneshot(request(&format!(
                "{MANAGEMENT_PLACEMENT_TRACE_PREFIX}/{id}"
            )))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn canary_mixed_epoch_partition_evidence_is_never_published_as_current() {
    let stale = partition(70, 100);
    let model = ManagementTopologyModel::new()
        .with_partition_snapshot(stale.clone())
        .unwrap();
    let body = if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W5-MIXED-EPOCH")
    {
        serde_json::to_value(stale).unwrap()
    } else {
        json(
            surface(model)
                .routes()
                .oneshot(request(MANAGEMENT_CLUSTER_PARTITIONS_PATH))
                .await
                .unwrap(),
        )
        .await["data"]
            .clone()
    };
    assert_eq!(
        body["authority_epoch"], 71,
        "HC-CANARY-RED:MC72-W5-MIXED-EPOCH stale partition evidence escaped the epoch fence"
    );
    assert!(body["assigned"].is_null());
}

#[tokio::test]
async fn canary_discovered_or_reachable_member_is_not_inferred_ready() {
    let mut body = json(
        surface(ManagementTopologyModel::new())
            .routes()
            .oneshot(request(MANAGEMENT_CLUSTER_MEMBERS_PATH))
            .await
            .unwrap(),
    )
    .await;
    let remote = body["data"]["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["generation"] == 9)
        .unwrap();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W5-DISCOVERED-IS-READY") {
        remote["formation"]["serving"] = Value::String("serving".to_owned());
    }
    assert_ne!(
        remote["formation"]["serving"], "serving",
        "HC-CANARY-RED:MC72-W5-DISCOVERED-IS-READY discovery was promoted to readiness"
    );
}

#[test]
fn canary_placement_normalization_is_permutation_invariant() {
    let first = normalize_placement_trace(trace(candidates())).unwrap();
    let mut reversed = candidates();
    reversed.reverse();
    let mut second = normalize_placement_trace(trace(reversed)).unwrap();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref()
        == Ok("MC72-W5-NONDETERMINISTIC-PLACEMENT")
    {
        second.candidates.items.reverse();
    }
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap(),
        "HC-CANARY-RED:MC72-W5-NONDETERMINISTIC-PLACEMENT candidate input order changed the trace"
    );
}

#[test]
fn canary_committed_placement_is_not_reported_as_applied() {
    let mut evidence = normalize_placement_trace(trace(candidates())).unwrap();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W5-COMMIT-IS-APPLY") {
        evidence.outcome = PlacementOutcome::Applied;
        evidence.applied_index = evidence.commit_index;
    }
    assert_eq!(
        evidence.outcome,
        PlacementOutcome::Committed,
        "HC-CANARY-RED:MC72-W5-COMMIT-IS-APPLY commit acknowledgement was presented as apply"
    );
    assert!(evidence.applied_index < evidence.commit_index);
}
