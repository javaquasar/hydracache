use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{Method, Request, StatusCode};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use hydracache_observability::{
    MAX_MANAGEMENT_CLIENT_PROTOCOLS, MAX_MANAGEMENT_CURSOR_BYTES,
    MAX_MANAGEMENT_DIAGNOSTIC_STRING_BYTES, MAX_MANAGEMENT_HEALTH_CHECKS,
    MAX_MANAGEMENT_HEALTH_EVIDENCE, MAX_MANAGEMENT_MEMBERS, MAX_MANAGEMENT_NODE_ID_BYTES,
    MAX_MANAGEMENT_PAGE_ITEMS, MAX_MANAGEMENT_WARNINGS, MAX_MEMBER_FORMATION_TRANSITIONS,
    MAX_PLACEMENT_CANDIDATES, MAX_PLACEMENT_LABELS, MAX_PLACEMENT_LABEL_BYTES,
    MAX_PLACEMENT_REASONS_PER_CANDIDATE, MAX_PLACEMENT_SELECTED,
};
use hydracache_server::{
    AdminHttpSurface, ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime,
    LocalConsensusStatus, MemberRole, MemberStatus, Reachability, ReshardPhase, ServerConfig,
    ServerRole, ServerRuntime, StatusSource, CLUSTER_MANAGEMENT_SNAPSHOT_PATH,
    MANAGEMENT_CAPABILITIES_PATH, MANAGEMENT_CLIENTS_PATH, MANAGEMENT_CLUSTER_FORMATION_PATH,
    MANAGEMENT_CLUSTER_MEMBERS_PATH, MANAGEMENT_CLUSTER_PARTITIONS_PATH,
    MANAGEMENT_CONSENSUS_PROGRESS_PATH, MANAGEMENT_CURSOR_TTL_MS, MANAGEMENT_DASHBOARD_PATH,
    MANAGEMENT_FORMATION_PATH, MANAGEMENT_HEALTHCHECKS_PATH, MANAGEMENT_HISTORY_DEADLINE,
    MANAGEMENT_HISTORY_MAX_CONCURRENCY, MANAGEMENT_HISTORY_MAX_POINTS,
    MANAGEMENT_HISTORY_MAX_RANGE_MS, MANAGEMENT_HISTORY_MAX_RESOLVED_ADDRESSES,
    MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES, MANAGEMENT_HISTORY_MAX_SERIES,
    MANAGEMENT_HISTORY_MAX_TOKEN_BYTES, MANAGEMENT_HISTORY_MIN_STEP_MS, MANAGEMENT_HISTORY_PATH,
    MANAGEMENT_MAX_RESPONSE_BYTES, MANAGEMENT_MAX_RETAINED_CURSORS, MANAGEMENT_NAMESPACES_PATH,
    MANAGEMENT_NAMESPACE_CACHES_PREFIX, MANAGEMENT_PLACEMENT_TRACE_PREFIX,
    MANAGEMENT_RECOVERY_PATH, MANAGEMENT_SNAPSHOT_CACHE_TTL, MANAGEMENT_SNAPSHOT_MAX_CONCURRENCY,
    MANAGEMENT_SNAPSHOT_MAX_PEERS, MANAGEMENT_SNAPSHOT_MAX_REQUEST_BYTES,
    MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES, MANAGEMENT_SNAPSHOT_PEER_TIMEOUT,
    MANAGEMENT_SNAPSHOT_REFRESH_TIMEOUT, MANAGEMENT_STALE_AFTER_MS,
};
use serde_json::Value;
use tower::ServiceExt;

fn local_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Local,
        seeds: Vec::new(),
        storage_dir: None,
        ..ServerConfig::default()
    }
}

fn live_status() -> ClusterStatus {
    ClusterStatus {
        source: StatusSource::Live,
        leader: Some("local-secret-token".to_owned()),
        term: 7,
        epoch: 42,
        observation_seq: 19,
        metadata_authoritative: true,
        quorum_ok: true,
        members: vec![
            member("remote-z-private", 2, Reachability::Reachable),
            member("local-secret-token", 7, Reachability::Reachable),
            member("remote-a-private", 3, Reachability::Unreachable),
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

fn member(node_id: &str, generation: u64, reachable: Reachability) -> MemberStatus {
    MemberStatus {
        node_id: node_id.to_owned(),
        role: MemberRole::Member,
        reachable,
        generation,
    }
}

#[derive(Debug)]
struct CountingProvider {
    status: Mutex<ClusterStatus>,
    calls: AtomicU64,
}

impl CountingProvider {
    fn new(status: ClusterStatus) -> Self {
        Self {
            status: Mutex::new(status),
            calls: AtomicU64::new(0),
        }
    }

    fn calls(&self) -> u64 {
        self.calls.load(Ordering::SeqCst)
    }

    fn advance_observation(&self) {
        let mut status = self.status.lock().unwrap();
        status.observation_seq = status.observation_seq.saturating_add(1);
        if let Some(progress) = status.local_consensus.as_mut() {
            progress.commit_index = progress.commit_index.saturating_add(1);
            progress.applied_index = progress.commit_index;
            progress.catch_up_target = progress.commit_index;
        }
    }
}

impl ClusterStatusProvider for CountingProvider {
    fn cluster_status(&self, runtime: ClusterStatusRuntime) -> ClusterStatus {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut status = self.status.lock().unwrap().clone();
        status.draining |= runtime.draining;
        status
    }
}

fn surface(provider: Arc<CountingProvider>) -> AdminHttpSurface {
    let provider: Arc<dyn ClusterStatusProvider> = provider;
    AdminHttpSurface::new(
        ServerRuntime::new(local_config())
            .unwrap()
            .with_cluster_status_provider(provider)
            .start(),
    )
}

fn management_request(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Body::empty())
        .unwrap()
}

fn identity_only_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .body(Body::empty())
        .unwrap()
}

async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap()
        .to_vec()
}

async fn json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&body_bytes(response).await).unwrap()
}

#[tokio::test]
async fn management_routes_require_verified_privileged_identity_before_reading_sources() {
    let provider = Arc::new(CountingProvider::new(live_status()));
    let router = surface(Arc::clone(&provider)).routes();
    let baseline = provider.calls();

    for path in [
        MANAGEMENT_CAPABILITIES_PATH,
        MANAGEMENT_DASHBOARD_PATH,
        MANAGEMENT_FORMATION_PATH,
        MANAGEMENT_CLUSTER_FORMATION_PATH,
        MANAGEMENT_CLUSTER_MEMBERS_PATH,
        MANAGEMENT_CLUSTER_PARTITIONS_PATH,
        MANAGEMENT_CLIENTS_PATH,
        MANAGEMENT_NAMESPACES_PATH,
        MANAGEMENT_HEALTHCHECKS_PATH,
        MANAGEMENT_HISTORY_PATH,
        MANAGEMENT_CONSENSUS_PROGRESS_PATH,
        MANAGEMENT_RECOVERY_PATH,
    ] {
        let anonymous = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

        let forbidden = router
            .clone()
            .oneshot(identity_only_request(path))
            .await
            .unwrap();
        assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    }
    assert_eq!(
        provider.calls(),
        baseline,
        "auth rejection must precede source IO"
    );
}

#[tokio::test]
async fn dashboard_is_typed_bounded_pseudonymous_and_does_not_zero_fill_missing_sources() {
    let provider = Arc::new(CountingProvider::new(live_status()));
    let response = surface(provider)
        .routes()
        .oneshot(management_request(Method::GET, MANAGEMENT_DASHBOARD_PATH))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = body_bytes(response).await;
    assert!(bytes.len() <= MANAGEMENT_MAX_RESPONSE_BYTES);
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["source"], "live");
    assert_eq!(body["completeness"], "partial");
    assert_eq!(body["data"]["cluster"]["quorum_ok"], true);
    assert_eq!(body["data"]["cluster"]["authority_epoch"], 42);
    assert_eq!(body["data"]["consensus"]["apply_lag"], 0);
    assert_eq!(body["data"]["partitions"]["total"], 0);
    assert!(body["data"]["partitions"]["distribution"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(body["data"]["cache"]["retained_bytes"].is_null());
    assert!(body["data"]["cache"]["ttl_backlog"].is_null());
    assert!(body["data"]["members"][0]["cpu_percent"].is_null());
    let encoded = String::from_utf8(bytes).unwrap();
    assert!(!encoded.contains("local-secret-token"));
    assert!(!encoded.contains("remote-a-private"));
    assert!(encoded.contains("partial-observation"));
}

#[tokio::test]
async fn capabilities_are_sorted_and_explain_partial_or_unavailable_sources() {
    let provider = Arc::new(CountingProvider::new(live_status()));
    let response = surface(provider)
        .routes()
        .oneshot(management_request(
            Method::GET,
            MANAGEMENT_CAPABILITIES_PATH,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    let body = json(response).await;
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["source"], "live");
    assert_eq!(body["completeness"], "complete");
    assert_eq!(body["data"]["capabilities"][0]["id"], "cluster_formation");
    assert_eq!(
        body["data"]["capabilities"][0]["reason"],
        "partial-observation"
    );
    assert_eq!(body["data"]["capabilities"][1]["id"], "consensus_progress");
    assert_eq!(
        body["data"]["capabilities"][2]["availability"],
        "unavailable"
    );
    assert_eq!(
        body["data"]["capabilities"][2]["reason"],
        "status-not-retained"
    );
}

#[tokio::test]
async fn formation_is_stable_paginated_and_never_serializes_raw_node_identity() {
    let provider = Arc::new(CountingProvider::new(live_status()));
    let surface = surface(provider);
    let first_response = surface
        .routes()
        .oneshot(management_request(
            Method::GET,
            &format!("{MANAGEMENT_FORMATION_PATH}?limit=2"),
        ))
        .await
        .unwrap();
    let first_bytes = body_bytes(first_response).await;
    let serialized = String::from_utf8(first_bytes.clone()).unwrap();
    assert!(!serialized.contains("local-secret-token"));
    assert!(!serialized.contains("remote-a-private"));
    assert!(!serialized.contains("remote-z-private"));
    let first: Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(first["data"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(first["data"]["truncated"], true);
    assert_eq!(first["completeness"], "partial");
    let cursor = first["data"]["next_cursor"].as_str().unwrap();
    assert_ne!(cursor, "2");
    assert_eq!(cursor.len(), 64);

    let second = surface
        .routes()
        .oneshot(management_request(
            Method::GET,
            &format!("{MANAGEMENT_FORMATION_PATH}?limit=2&cursor={cursor}"),
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second = json(second).await;
    assert_eq!(second["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(second["data"]["truncated"], false);
    assert!(second["data"]["next_cursor"].is_null());
}

#[tokio::test]
async fn formation_and_consensus_use_one_authority_epoch_without_collapsing_progress() {
    let provider = Arc::new(CountingProvider::new(live_status()));
    let surface = surface(provider);
    let formation = surface
        .routes()
        .oneshot(management_request(Method::GET, MANAGEMENT_FORMATION_PATH))
        .await
        .unwrap();
    let formation = json(formation).await;
    let local = formation["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["generation"] == 7)
        .unwrap();
    assert_eq!(local["authority_epoch"], 42);
    assert_eq!(local["admission"], "admitted");
    assert_eq!(local["consensus_role"], "voter");
    assert_eq!(local["catch_up"], "current");
    assert_eq!(local["serving"], "serving");

    let progress = surface
        .routes()
        .oneshot(management_request(
            Method::GET,
            MANAGEMENT_CONSENSUS_PROGRESS_PATH,
        ))
        .await
        .unwrap();
    let progress = json(progress).await;
    assert_eq!(progress["authority_epoch"], 42);
    assert_eq!(progress["data"]["items"][0]["commit_index"], 19);
    assert_eq!(progress["data"]["items"][0]["applied_index"], 19);
    assert_eq!(progress["data"]["items"][0]["last_snapshot_index"], 12);
    assert_eq!(progress["data"]["items"][0]["catch_up_target"], 19);
}

#[tokio::test]
async fn missing_recovery_record_is_unknown_not_clean() {
    let provider = Arc::new(CountingProvider::new(live_status()));
    let response = surface(provider)
        .routes()
        .oneshot(management_request(Method::GET, MANAGEMENT_RECOVERY_PATH))
        .await
        .unwrap();

    let body = json(response).await;
    assert_eq!(body["source"], "unavailable");
    assert_eq!(body["completeness"], "partial");
    assert_eq!(body["data"]["items"][0]["outcome"], "unknown");
    assert_eq!(body["data"]["items"][0]["reason"], "status-not-retained");
    assert_ne!(body["data"]["items"][0]["outcome"], "clean");
}

#[tokio::test]
async fn query_and_method_policy_fail_loud_without_invoking_source_adapter() {
    let provider = Arc::new(CountingProvider::new(live_status()));
    let router = surface(Arc::clone(&provider)).routes();

    for uri in [
        format!("{MANAGEMENT_FORMATION_PATH}?limit=0"),
        format!("{MANAGEMENT_FORMATION_PATH}?unknown=true"),
        format!("{MANAGEMENT_FORMATION_PATH}?cursor=forged"),
        format!("{MANAGEMENT_FORMATION_PATH}?cursor=%FF"),
    ] {
        let response = router
            .clone()
            .oneshot(management_request(Method::GET, &uri))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = json(response).await;
        assert!(error["code"].as_str().unwrap().starts_with("invalid-"));
    }

    let baseline = provider.calls();
    for method in [Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS] {
        let response = router
            .clone()
            .oneshot(management_request(method, MANAGEMENT_FORMATION_PATH))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
    assert_eq!(
        provider.calls(),
        baseline,
        "unsupported methods must not invoke management sources"
    );

    let head = router
        .oneshot(management_request(
            Method::HEAD,
            MANAGEMENT_CAPABILITIES_PATH,
        ))
        .await
        .unwrap();
    assert_eq!(head.status(), StatusCode::OK);
    assert!(body_bytes(head).await.is_empty());
}

#[tokio::test]
async fn cursor_rejects_snapshot_regression_or_advance() {
    let provider = Arc::new(CountingProvider::new(live_status()));
    let surface = surface(Arc::clone(&provider));
    let first = surface
        .routes()
        .oneshot(management_request(
            Method::GET,
            &format!("{MANAGEMENT_FORMATION_PATH}?limit=1"),
        ))
        .await
        .unwrap();
    let first = json(first).await;
    let cursor = first["data"]["next_cursor"].as_str().unwrap();
    provider.advance_observation();

    let response = surface
        .routes()
        .oneshot(management_request(
            Method::GET,
            &format!("{MANAGEMENT_FORMATION_PATH}?limit=1&cursor={cursor}"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(json(response).await["code"], "snapshot-changed");
}

#[tokio::test]
async fn member_pages_enforce_item_and_response_byte_bounds() {
    let mut status = live_status();
    status.local_consensus = None;
    status.members = (0..250)
        .map(|index| {
            member(
                &format!("raw-private-member-{index}-{}", "x".repeat(1_024)),
                index as u64,
                Reachability::Reachable,
            )
        })
        .collect();
    let provider = Arc::new(CountingProvider::new(status));
    let response = surface(provider)
        .routes()
        .oneshot(management_request(Method::GET, MANAGEMENT_FORMATION_PATH))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = body_bytes(response).await;
    assert!(bytes.len() < 256 * 1024);
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 100);
    assert_eq!(body["data"]["truncated"], true);
}

#[tokio::test]
async fn canary_management_api_detects_internal_identity_leak_or_oversize_payload() {
    let payload = if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W2") {
        serde_json::to_vec(&live_status()).unwrap()
    } else {
        let provider = Arc::new(CountingProvider::new(live_status()));
        let response = surface(provider)
            .routes()
            .oneshot(management_request(Method::GET, MANAGEMENT_FORMATION_PATH))
            .await
            .unwrap();
        body_bytes(response).await
    };
    let payload_text = String::from_utf8(payload.clone()).unwrap();
    assert!(
        payload.len() <= MANAGEMENT_MAX_RESPONSE_BYTES,
        "HC-CANARY-RED:MC72-W2 management response exceeded its byte budget"
    );
    assert!(
        !payload_text.contains("local-secret-token")
            && !payload_text.contains("remote-a-private")
            && !payload_text.contains("remote-z-private"),
        "HC-CANARY-RED:MC72-W2 internal node identity escaped the DTO allowlist"
    );
}

#[tokio::test]
async fn management_routes_are_absent_from_public_client_surface() {
    let response = AxumClientSurface::new(ClientSurfaceLimits::default())
        .unwrap()
        .routes()
        .oneshot(management_request(
            Method::GET,
            MANAGEMENT_CAPABILITIES_PATH,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn source_and_bound_registries_match_executable_contract() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source_map_text =
        std::fs::read_to_string(root.join("docs/testing/management-center/0.72/source-map.toml"))
            .unwrap();
    let source_map: toml::Value = toml::from_str(&source_map_text).unwrap();
    let routes = source_map["source"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["route"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        routes,
        [
            MANAGEMENT_CAPABILITIES_PATH,
            MANAGEMENT_FORMATION_PATH,
            MANAGEMENT_DASHBOARD_PATH,
            MANAGEMENT_CLUSTER_FORMATION_PATH,
            MANAGEMENT_CLUSTER_MEMBERS_PATH,
            MANAGEMENT_CLUSTER_PARTITIONS_PATH,
            "/management/v1/cluster/placement-traces/{opaque_id}",
            MANAGEMENT_CONSENSUS_PROGRESS_PATH,
            MANAGEMENT_CLIENTS_PATH,
            CLUSTER_MANAGEMENT_SNAPSHOT_PATH,
            MANAGEMENT_NAMESPACES_PATH,
            "/management/v1/namespaces/{namespace}/caches",
            MANAGEMENT_HEALTHCHECKS_PATH,
            MANAGEMENT_HISTORY_PATH,
            MANAGEMENT_RECOVERY_PATH,
        ]
    );

    let bounds_text =
        std::fs::read_to_string(root.join("docs/testing/management-center/0.72/bounds.toml"))
            .unwrap();
    let bounds: toml::Value = toml::from_str(&bounds_text).unwrap();
    let value = |section: &str, key: &str| bounds[section][key].as_integer().unwrap() as usize;
    assert_eq!(value("http", "max_page_items"), MAX_MANAGEMENT_PAGE_ITEMS);
    assert_eq!(
        value("http", "max_response_bytes"),
        MANAGEMENT_MAX_RESPONSE_BYTES
    );
    assert_eq!(
        value("http", "stale_after_ms"),
        MANAGEMENT_STALE_AFTER_MS as usize
    );
    assert_eq!(
        value("cursor", "max_encoded_bytes"),
        MAX_MANAGEMENT_CURSOR_BYTES
    );
    assert_eq!(value("cursor", "ttl_ms"), MANAGEMENT_CURSOR_TTL_MS as usize);
    assert_eq!(
        value("cursor", "max_retained_records"),
        MANAGEMENT_MAX_RETAINED_CURSORS
    );
    assert_eq!(
        value("identity", "max_source_node_id_bytes"),
        MAX_MANAGEMENT_NODE_ID_BYTES
    );
    assert_eq!(value("envelope", "max_warnings"), MAX_MANAGEMENT_WARNINGS);
    assert_eq!(
        value("cluster_aggregation", "max_committed_peers"),
        MANAGEMENT_SNAPSHOT_MAX_PEERS
    );
    assert_eq!(
        value("cluster_aggregation", "max_concurrency"),
        MANAGEMENT_SNAPSHOT_MAX_CONCURRENCY
    );
    assert_eq!(
        value("cluster_aggregation", "per_peer_timeout_ms"),
        MANAGEMENT_SNAPSHOT_PEER_TIMEOUT.as_millis() as usize
    );
    assert_eq!(
        value("cluster_aggregation", "whole_refresh_timeout_ms"),
        MANAGEMENT_SNAPSHOT_REFRESH_TIMEOUT.as_millis() as usize
    );
    assert_eq!(
        value("cluster_aggregation", "cache_ttl_ms"),
        MANAGEMENT_SNAPSHOT_CACHE_TTL.as_millis() as usize
    );
    assert_eq!(
        value("cluster_aggregation", "max_rpc_request_bytes"),
        MANAGEMENT_SNAPSHOT_MAX_REQUEST_BYTES
    );
    assert_eq!(
        value("cluster_aggregation", "max_rpc_response_bytes"),
        MANAGEMENT_SNAPSHOT_MAX_RESPONSE_BYTES
    );
    assert_eq!(
        value("placement", "max_candidates"),
        MAX_PLACEMENT_CANDIDATES
    );
    assert_eq!(value("placement", "max_selected"), MAX_PLACEMENT_SELECTED);
    assert_eq!(
        value("placement", "max_reasons_per_candidate"),
        MAX_PLACEMENT_REASONS_PER_CANDIDATE
    );
    assert_eq!(
        value("placement", "max_labels_per_constraint_set"),
        MAX_PLACEMENT_LABELS
    );
    assert_eq!(
        value("placement", "max_label_bytes"),
        MAX_PLACEMENT_LABEL_BYTES
    );
    assert_eq!(value("topology", "max_members"), MAX_MANAGEMENT_MEMBERS);
    assert_eq!(
        value("topology", "max_current_generation_transitions_per_member"),
        MAX_MEMBER_FORMATION_TRANSITIONS
    );
    assert_eq!(
        value("topology", "max_diagnostic_string_bytes"),
        MAX_MANAGEMENT_DIAGNOSTIC_STRING_BYTES
    );
    assert_eq!(
        bounds["topology"]["unknown_trace_status"].as_integer(),
        Some(404)
    );
    assert!(MANAGEMENT_PLACEMENT_TRACE_PREFIX.ends_with("placement-traces"));
    assert_eq!(
        value("clients", "max_protocol_rows"),
        MAX_MANAGEMENT_CLIENT_PROTOCOLS
    );
    assert_eq!(bounds["clients"]["detail_available"].as_bool(), Some(false));
    assert_eq!(
        value("namespaces", "max_page_items"),
        MAX_MANAGEMENT_PAGE_ITEMS
    );
    assert_eq!(value("namespaces", "max_caches_per_client_namespace"), 1);
    assert!(MANAGEMENT_NAMESPACE_CACHES_PREFIX.ends_with("namespaces"));
    assert_eq!(
        value("healthchecks", "max_checks"),
        MAX_MANAGEMENT_HEALTH_CHECKS
    );
    assert_eq!(
        value("healthchecks", "max_evidence_per_check"),
        MAX_MANAGEMENT_HEALTH_EVIDENCE
    );
    assert_eq!(
        value("prometheus_history", "max_range_ms"),
        MANAGEMENT_HISTORY_MAX_RANGE_MS as usize
    );
    assert_eq!(
        value("prometheus_history", "min_step_ms"),
        MANAGEMENT_HISTORY_MIN_STEP_MS as usize
    );
    assert_eq!(
        value("prometheus_history", "max_points"),
        MANAGEMENT_HISTORY_MAX_POINTS
    );
    assert_eq!(
        value("prometheus_history", "max_series"),
        MANAGEMENT_HISTORY_MAX_SERIES
    );
    assert_eq!(
        value("prometheus_history", "max_response_bytes"),
        MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES
    );
    assert_eq!(
        value("prometheus_history", "max_resolved_addresses"),
        MANAGEMENT_HISTORY_MAX_RESOLVED_ADDRESSES
    );
    assert_eq!(
        value("prometheus_history", "max_token_bytes"),
        MANAGEMENT_HISTORY_MAX_TOKEN_BYTES as usize
    );
    assert_eq!(
        value("prometheus_history", "max_concurrency"),
        MANAGEMENT_HISTORY_MAX_CONCURRENCY
    );
    assert_eq!(
        value("prometheus_history", "deadline_ms"),
        MANAGEMENT_HISTORY_DEADLINE.as_millis() as usize
    );

    let canaries_text =
        std::fs::read_to_string(root.join("docs/testing/canaries/0.72-management-center.toml"))
            .unwrap();
    let canaries: toml::Value = toml::from_str(&canaries_text).unwrap();
    let active = canaries["canary"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"].as_str() == Some("MC72-W2-LEAK-OR-OVERSIZE"))
        .unwrap();
    assert_eq!(active["status"].as_str(), Some("active"));
    assert_eq!(
        active["test"].as_str(),
        Some("canary_management_api_detects_internal_identity_leak_or_oversize_payload")
    );
}
