use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use hydracache::ClusterNodeId;
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_cluster_transport_axum::{
    AllowAllAuthorizer, ClusterRoute, ClusterRouteAuth, StaticNodeIdentityProvider,
};
use hydracache_observability::MANAGEMENT_API_SCHEMA_VERSION;
use hydracache_server::{
    AdminHttpSurface, ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime,
    HttpManagementPeerTransport, LocalConsensusStatus, LocalManagementSnapshotProvider,
    ManagementAggregationIssue, ManagementConsensusObservation, ManagementMemberSnapshot,
    ManagementPeerTarget, ManagementPeerTransport, ManagementSnapshotAggregator,
    ManagementSnapshotRequest, ManagementSnapshotRpcService, MemberRole, MemberStatus,
    Reachability, ReshardPhase, ServerConfig, ServerRole, ServerRuntime, StatusSource,
    CLUSTER_MANAGEMENT_SNAPSHOT_PATH, MANAGEMENT_CONSENSUS_PROGRESS_PATH,
    MANAGEMENT_FORMATION_PATH, MANAGEMENT_SNAPSHOT_MAX_REQUEST_BYTES,
    MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES,
};
use tower::ServiceExt;

#[derive(Debug)]
struct StaticStatusProvider(Mutex<ClusterStatus>);

impl ClusterStatusProvider for StaticStatusProvider {
    fn cluster_status(&self, _runtime: ClusterStatusRuntime) -> ClusterStatus {
        self.0.lock().unwrap().clone()
    }
}

#[derive(Debug)]
struct AggregateTransport;

#[async_trait::async_trait]
impl ManagementPeerTransport for AggregateTransport {
    async fn fetch(
        &self,
        target: &ManagementPeerTarget,
        request: ManagementSnapshotRequest,
    ) -> Result<Vec<u8>, ManagementAggregationIssue> {
        if target.node_id == "remote-z-private" {
            return Ok(serde_json::to_vec(&ManagementMemberSnapshot {
                schema_version: MANAGEMENT_API_SCHEMA_VERSION,
                node_id: target.node_id.clone(),
                generation: target.generation,
                authority_epoch: request.authority_epoch,
                observation_seq: 18,
                consensus: Some(ManagementConsensusObservation {
                    commit_index: 19,
                    applied_index: 18,
                    last_snapshot_index: Some(12),
                    catch_up_target: 19,
                    voter: true,
                }),
            })
            .unwrap());
        }
        Err(ManagementAggregationIssue::Transport)
    }
}

#[derive(Debug)]
struct CountingProvider {
    calls: AtomicUsize,
    snapshot: ManagementMemberSnapshot,
}

#[derive(Debug)]
struct ToggleProvider {
    ready: std::sync::atomic::AtomicBool,
    snapshot: ManagementMemberSnapshot,
}

impl LocalManagementSnapshotProvider for ToggleProvider {
    fn local_snapshot(&self) -> Option<ManagementMemberSnapshot> {
        self.ready
            .load(Ordering::SeqCst)
            .then(|| self.snapshot.clone())
    }
}

impl LocalManagementSnapshotProvider for CountingProvider {
    fn local_snapshot(&self) -> Option<ManagementMemberSnapshot> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Some(self.snapshot.clone())
    }
}

fn member() -> ManagementMemberSnapshot {
    ManagementMemberSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node_id: "member-b".to_owned(),
        generation: 4,
        authority_epoch: 9,
        observation_seq: 12,
        consensus: Some(ManagementConsensusObservation {
            commit_index: 12,
            applied_index: 11,
            last_snapshot_index: Some(8),
            catch_up_target: 12,
            voter: true,
        }),
    }
}

fn secure_auth() -> ClusterRouteAuth {
    ClusterRouteAuth::secure(
        Arc::new(StaticNodeIdentityProvider::new(
            ClusterNodeId::from("member-a"),
            "k1",
            "secret",
        )),
        Arc::new(AllowAllAuthorizer),
    )
}

fn request(epoch: u64, headers: Option<HeaderMap>) -> Request<Body> {
    let body = serde_json::to_vec(&ManagementSnapshotRequest {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        authority_epoch: epoch,
    })
    .unwrap();
    let mut request = Request::builder()
        .method("POST")
        .uri(CLUSTER_MANAGEMENT_SNAPSHOT_PATH)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    if let Some(headers) = headers {
        *request.headers_mut() = headers;
        request
            .headers_mut()
            .insert("content-type", "application/json".parse().unwrap());
    }
    request
}

#[tokio::test]
async fn protected_rpc_authenticates_before_reading_local_source() {
    let provider = Arc::new(CountingProvider {
        calls: AtomicUsize::new(0),
        snapshot: member(),
    });
    let auth = secure_auth();
    let app = ManagementSnapshotRpcService::new(provider.clone(), auth.clone()).routes();

    let denied = app.clone().oneshot(request(9, None)).await.unwrap();

    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        auth.rejected_total_for_route(ClusterRoute::ManagementSnapshot),
        1
    );

    let mut headers = HeaderMap::new();
    auth.apply_outbound_headers(&mut headers).unwrap();
    let accepted = app.oneshot(request(9, Some(headers))).await.unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
    let bytes = to_bytes(accepted.into_body(), usize::MAX).await.unwrap();
    let snapshot: ManagementMemberSnapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(snapshot, member());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn protected_rpc_fails_loud_for_epoch_schema_method_and_body_bounds() {
    let provider = Arc::new(CountingProvider {
        calls: AtomicUsize::new(0),
        snapshot: member(),
    });
    let auth = secure_auth();
    let app = ManagementSnapshotRpcService::new(provider.clone(), auth.clone()).routes();
    let mut headers = HeaderMap::new();
    auth.apply_outbound_headers(&mut headers).unwrap();

    let stale = app
        .clone()
        .oneshot(request(8, Some(headers.clone())))
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);

    let incompatible_body = serde_json::to_vec(&ManagementSnapshotRequest {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION + 1,
        authority_epoch: 9,
    })
    .unwrap();
    let mut incompatible = Request::builder()
        .method("POST")
        .uri(CLUSTER_MANAGEMENT_SNAPSHOT_PATH)
        .header("content-type", "application/json")
        .body(Body::from(incompatible_body))
        .unwrap();
    *incompatible.headers_mut() = headers.clone();
    incompatible
        .headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    assert_eq!(
        app.clone().oneshot(incompatible).await.unwrap().status(),
        StatusCode::NOT_ACCEPTABLE
    );

    let get = Request::builder()
        .method("GET")
        .uri(CLUSTER_MANAGEMENT_SNAPSHOT_PATH)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(get).await.unwrap().status(),
        StatusCode::METHOD_NOT_ALLOWED
    );

    let mut oversize = Request::builder()
        .method("POST")
        .uri(CLUSTER_MANAGEMENT_SNAPSHOT_PATH)
        .header("content-type", "application/json")
        .body(Body::from(vec![
            b'x';
            MANAGEMENT_SNAPSHOT_MAX_REQUEST_BYTES + 1
        ]))
        .unwrap();
    *oversize.headers_mut() = headers;
    oversize
        .headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    assert_eq!(
        app.oneshot(oversize).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

fn aggregate_status() -> ClusterStatus {
    ClusterStatus {
        source: StatusSource::Live,
        leader: Some("local-secret-token".to_owned()),
        term: 7,
        epoch: 42,
        observation_seq: 19,
        metadata_authoritative: true,
        quorum_ok: true,
        members: vec![
            MemberStatus {
                node_id: "local-secret-token".to_owned(),
                role: MemberRole::Member,
                reachable: Reachability::Reachable,
                generation: 7,
            },
            MemberStatus {
                node_id: "remote-z-private".to_owned(),
                role: MemberRole::Member,
                reachable: Reachability::Reachable,
                generation: 2,
            },
            MemberStatus {
                node_id: "remote-a-private".to_owned(),
                role: MemberRole::Member,
                reachable: Reachability::Unreachable,
                generation: 3,
            },
        ],
        voters: 3,
        voter_ids: vec![1, 2, 3],
        reshard_phase: ReshardPhase::Idle,
        draining: false,
        local_consensus: Some(LocalConsensusStatus {
            node_id: "local-secret-token".to_owned(),
            generation: 7,
            voter: true,
            commit_index: 19,
            applied_index: 19,
            last_snapshot_index: Some(12),
            catch_up_target: 19,
        }),
    }
}

fn management_get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn public_routes_publish_cluster_aggregate_as_partial_without_raw_identity() {
    let status = aggregate_status();
    let provider: Arc<dyn ClusterStatusProvider> =
        Arc::new(StaticStatusProvider(Mutex::new(status)));
    let local = member_for_aggregate("local-secret-token", 7, 42, 19, 19);
    let runtime = ServerRuntime::new(ServerConfig {
        role: ServerRole::Local,
        storage_dir: None,
        seeds: Vec::new(),
        ..ServerConfig::default()
    })
    .unwrap()
    .with_cluster_status_provider(provider)
    .with_management_snapshot_source(
        local,
        vec![
            ManagementPeerTarget {
                node_id: "remote-z-private".to_owned(),
                generation: 2,
                endpoint: "remote-z:7000".to_owned(),
            },
            ManagementPeerTarget {
                node_id: "remote-a-private".to_owned(),
                generation: 3,
                endpoint: "remote-a:7000".to_owned(),
            },
        ],
        Arc::new(AggregateTransport),
    )
    .start();
    let router = AdminHttpSurface::new(runtime).routes();

    let consensus = router
        .clone()
        .oneshot(management_get(MANAGEMENT_CONSENSUS_PROGRESS_PATH))
        .await
        .unwrap();
    assert_eq!(consensus.status(), StatusCode::OK);
    let bytes = to_bytes(consensus.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["completeness"], "partial");
    assert_eq!(json["data"]["items"].as_array().unwrap().len(), 2);
    assert!(json["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["code"] == "source-unavailable"));
    let serialized = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!serialized.contains("local-secret-token"));
    assert!(!serialized.contains("remote-z-private"));
    assert!(!serialized.contains("remote-a-private"));

    let formation = router
        .oneshot(management_get(MANAGEMENT_FORMATION_PATH))
        .await
        .unwrap();
    assert_eq!(formation.status(), StatusCode::OK);
    let bytes = to_bytes(formation.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["completeness"], "partial");
    assert_eq!(json["data"]["items"].as_array().unwrap().len(), 3);
    assert!(!String::from_utf8(bytes.to_vec())
        .unwrap()
        .contains("remote-z-private"));
}

fn member_for_aggregate(
    node_id: &str,
    generation: u64,
    epoch: u64,
    seq: u64,
    applied: u64,
) -> ManagementMemberSnapshot {
    ManagementMemberSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node_id: node_id.to_owned(),
        generation,
        authority_epoch: epoch,
        observation_seq: seq,
        consensus: Some(ManagementConsensusObservation {
            commit_index: seq,
            applied_index: applied,
            last_snapshot_index: Some(applied.saturating_sub(1)),
            catch_up_target: seq,
            voter: true,
        }),
    }
}

async fn spawn_test_server(router: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (endpoint, task)
}

#[tokio::test]
async fn real_http_transport_rejects_oversize_and_aggregator_rejects_malformed_reply() {
    let oversize_router = Router::new().route(
        CLUSTER_MANAGEMENT_SNAPSHOT_PATH,
        post(|| async {
            (
                StatusCode::OK,
                vec![b'x'; MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES + 1],
            )
                .into_response()
        }),
    );
    let (oversize_endpoint, oversize_server) = spawn_test_server(oversize_router).await;
    let auth = ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true);
    let transport = HttpManagementPeerTransport::new(reqwest::Client::new(), auth.clone(), false);
    let target = ManagementPeerTarget {
        node_id: "peer".to_owned(),
        generation: 1,
        endpoint: oversize_endpoint,
    };
    let result = transport
        .fetch(
            &target,
            ManagementSnapshotRequest {
                schema_version: MANAGEMENT_API_SCHEMA_VERSION,
                authority_epoch: 7,
            },
        )
        .await;
    assert_eq!(result, Err(ManagementAggregationIssue::Oversize));
    oversize_server.abort();

    let malformed_router = Router::new().route(
        CLUSTER_MANAGEMENT_SNAPSHOT_PATH,
        post(|| async { (StatusCode::OK, "not-json").into_response() }),
    );
    let (malformed_endpoint, malformed_server) = spawn_test_server(malformed_router).await;
    let transport: Arc<dyn ManagementPeerTransport> = Arc::new(HttpManagementPeerTransport::new(
        reqwest::Client::new(),
        auth,
        false,
    ));
    let aggregator = ManagementSnapshotAggregator::new(transport);
    let result = aggregator
        .refresh(
            member_for_aggregate("local", 1, 7, 2, 2),
            vec![ManagementPeerTarget {
                node_id: "peer".to_owned(),
                generation: 1,
                endpoint: malformed_endpoint,
            }],
        )
        .await;
    assert_eq!(result.members.len(), 1);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(
        result.failures[0].issue,
        ManagementAggregationIssue::Malformed
    );
    malformed_server.abort();
}

#[tokio::test]
async fn three_member_http_aggregate_recovers_from_partial_to_complete() {
    let insecure = ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true);
    let peer_b = Arc::new(ToggleProvider {
        ready: std::sync::atomic::AtomicBool::new(true),
        snapshot: member_for_aggregate("peer-b", 2, 7, 8, 8),
    });
    let peer_c = Arc::new(ToggleProvider {
        ready: std::sync::atomic::AtomicBool::new(false),
        snapshot: member_for_aggregate("peer-c", 3, 7, 8, 8),
    });
    let (endpoint_b, server_b) =
        spawn_test_server(ManagementSnapshotRpcService::new(peer_b, insecure.clone()).routes())
            .await;
    let (endpoint_c, server_c) = spawn_test_server(
        ManagementSnapshotRpcService::new(peer_c.clone(), insecure.clone()).routes(),
    )
    .await;
    let transport: Arc<dyn ManagementPeerTransport> = Arc::new(HttpManagementPeerTransport::new(
        reqwest::Client::new(),
        insecure,
        false,
    ));
    let aggregator = ManagementSnapshotAggregator::new(transport).with_limits(
        std::time::Duration::ZERO,
        std::time::Duration::from_millis(100),
        std::time::Duration::from_millis(250),
        2,
    );
    let targets = vec![
        ManagementPeerTarget {
            node_id: "peer-b".to_owned(),
            generation: 2,
            endpoint: endpoint_b,
        },
        ManagementPeerTarget {
            node_id: "peer-c".to_owned(),
            generation: 3,
            endpoint: endpoint_c,
        },
    ];

    let partial = aggregator
        .refresh(member_for_aggregate("peer-a", 1, 7, 9, 9), targets.clone())
        .await;
    assert_eq!(partial.members.len(), 2);
    assert!(!partial.complete());
    assert_eq!(partial.failures[0].node_id, "peer-c");

    peer_c.ready.store(true, Ordering::SeqCst);
    let complete = aggregator
        .refresh(member_for_aggregate("peer-a", 1, 7, 10, 10), targets)
        .await;
    assert_eq!(complete.members.len(), 3);
    assert!(complete.complete());

    server_b.abort();
    server_c.abort();
}
