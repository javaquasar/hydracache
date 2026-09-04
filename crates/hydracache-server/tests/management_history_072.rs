use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{HeaderMap, Request, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{
    parse_prometheus_response, validate_resolved_addresses, AdminHttpSurface,
    ManagementHistoryConfig, ManagementHistoryError, ManagementHistoryQueryId,
    ManagementHistoryRequest, ManagementHistoryService, ManagementHistoryState, ServerConfig,
    ServerRuntime, MANAGEMENT_HISTORY_MAX_POINTS, MANAGEMENT_HISTORY_MAX_RANGE_MS,
    MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES, MANAGEMENT_HISTORY_MAX_SERIES,
    MANAGEMENT_HISTORY_MIN_STEP_MS, MANAGEMENT_HISTORY_PATH,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn request(query_id: ManagementHistoryQueryId) -> ManagementHistoryRequest {
    ManagementHistoryRequest {
        query_id,
        start_ms: 1_000_000,
        end_ms: 1_060_000,
        step_ms: MANAGEMENT_HISTORY_MIN_STEP_MS,
    }
}

fn history_config(origin: String) -> ManagementHistoryConfig {
    ManagementHistoryConfig {
        enabled: true,
        origin: Some(origin),
        bearer_token_file: None,
        allow_http: true,
        allow_private_networks: true,
    }
}

#[test]
fn opt_in_configuration_rejects_missing_or_unsafe_origins() {
    let mut config = ServerConfig {
        management_history: ManagementHistoryConfig {
            enabled: true,
            ..ManagementHistoryConfig::default()
        },
        ..ServerConfig::default()
    };
    assert!(config.validate().is_err());
    config.management_history.origin = Some("http://prometheus.example".to_owned());
    assert!(config.validate().is_err());
    config.management_history.allow_http = true;
    assert!(config.validate().is_ok());
    for origin in [
        "http://user:secret@prometheus.example",
        "http://prometheus.example/custom/path",
        "http://prometheus.example?query=up",
    ] {
        config.management_history.origin = Some(origin.to_owned());
        assert!(config.validate().is_err(), "origin={origin}");
    }
}

#[test]
fn request_bounds_reject_amplification_before_network_work() {
    let mut candidate = request(ManagementHistoryQueryId::CacheEntries);
    candidate.end_ms = candidate
        .start_ms
        .saturating_add(MANAGEMENT_HISTORY_MAX_RANGE_MS);
    candidate.step_ms = MANAGEMENT_HISTORY_MIN_STEP_MS;
    assert_eq!(
        candidate.validate(),
        Err(ManagementHistoryError::InvalidRange)
    );

    candidate.end_ms = candidate.start_ms + 60_000;
    candidate.step_ms = MANAGEMENT_HISTORY_MIN_STEP_MS - 1;
    assert_eq!(
        candidate.validate(),
        Err(ManagementHistoryError::InvalidRange)
    );
    candidate.step_ms = 60_001;
    assert_eq!(
        candidate.validate(),
        Err(ManagementHistoryError::InvalidRange)
    );
    candidate.step_ms = 60_000;
    assert!(candidate.validate().is_ok());
}

#[test]
fn resolver_rejects_private_link_local_metadata_and_mixed_rebinding_sets() {
    let config = ManagementHistoryConfig {
        enabled: true,
        origin: Some("https://prometheus.example".to_owned()),
        bearer_token_file: None,
        allow_http: false,
        allow_private_networks: false,
    };
    let public = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 443);
    assert!(validate_resolved_addresses(&config, &[public]).is_ok());
    for unsafe_address in [
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 443),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 443),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)), 443),
        SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 443),
    ] {
        assert_eq!(
            validate_resolved_addresses(&config, &[unsafe_address]),
            Err(ManagementHistoryError::UnsafeDestination)
        );
        assert_eq!(
            validate_resolved_addresses(&config, &[public, unsafe_address]),
            Err(ManagementHistoryError::UnsafeDestination),
            "one unsafe DNS answer rejects the complete set"
        );
    }
}

#[test]
fn parser_handles_matrix_vector_empty_malformed_oversize_and_bounds() {
    let matrix = json!({
        "status":"success",
        "data":{"resultType":"matrix","result":[
            {"metric":{"secret":"discarded"},"values":[[2.0,"2"],[1.0,"1"]]}
        ]}
    });
    let parsed = parse_prometheus_response(
        ManagementHistoryQueryId::CacheEntries,
        &serde_json::to_vec(&matrix).unwrap(),
    )
    .unwrap();
    assert_eq!(parsed.state, ManagementHistoryState::Available);
    assert_eq!(parsed.series[0].points[0].timestamp_unix_ms, 1_000);
    assert!(!serde_json::to_string(&parsed).unwrap().contains("secret"));

    let vector = json!({"status":"success","data":{"resultType":"vector","result":[{"metric":{},"value":[3.0,"4.5"]}]}});
    assert_eq!(
        parse_prometheus_response(
            ManagementHistoryQueryId::AdmissionQueueDepth,
            &serde_json::to_vec(&vector).unwrap()
        )
        .unwrap()
        .series[0]
            .points[0]
            .value,
        4.5
    );
    let empty = json!({"status":"success","data":{"resultType":"matrix","result":[]}});
    assert_eq!(
        parse_prometheus_response(
            ManagementHistoryQueryId::ReplicationFailure,
            &serde_json::to_vec(&empty).unwrap()
        )
        .unwrap()
        .state,
        ManagementHistoryState::NoData
    );
    assert_eq!(
        parse_prometheus_response(ManagementHistoryQueryId::CacheEntries, b"not-json"),
        Err(ManagementHistoryError::Malformed)
    );
    assert_eq!(
        parse_prometheus_response(
            ManagementHistoryQueryId::CacheEntries,
            &vec![b'x'; MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES + 1]
        ),
        Err(ManagementHistoryError::Oversize)
    );

    let values = (0..=MANAGEMENT_HISTORY_MAX_POINTS)
        .map(|index| json!([index, "1"]))
        .collect::<Vec<_>>();
    let result = (0..=MANAGEMENT_HISTORY_MAX_SERIES)
        .map(|_| json!({"metric":{},"values":values}))
        .collect::<Vec<_>>();
    let bounded = json!({"status":"success","data":{"resultType":"matrix","result":result}});
    let bounded = parse_prometheus_response(
        ManagementHistoryQueryId::CacheEntries,
        &serde_json::to_vec(&bounded).unwrap(),
    )
    .unwrap();
    assert_eq!(bounded.state, ManagementHistoryState::Partial);
    assert!(bounded.truncated);
    assert_eq!(bounded.series.len(), MANAGEMENT_HISTORY_MAX_SERIES);
    assert!(bounded
        .series
        .iter()
        .all(|series| series.points.len() <= MANAGEMENT_HISTORY_MAX_POINTS));
}

type CapturedCall = (String, Option<String>);

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<CapturedCall>>>);

async fn prometheus(
    State(captured): State<Captured>,
    uri: Uri,
    headers: HeaderMap,
) -> impl IntoResponse {
    captured.0.lock().unwrap().push((
        uri.to_string(),
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
    ));
    axum::Json(
        json!({"status":"success","data":{"resultType":"matrix","result":[{"metric":{"tenant":"must-not-escape"},"values":[[1000.0,"7"]]}]}}),
    )
}

#[tokio::test]
async fn real_adapter_uses_fixed_path_query_and_server_only_bearer_token() {
    let captured = Captured::default();
    let app = Router::new()
        .route("/api/v1/query_range", get(prometheus))
        .with_state(captured.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let token_path = std::env::temp_dir().join(format!(
        "hydracache-history-token-{}-{}",
        std::process::id(),
        address.port()
    ));
    std::fs::write(&token_path, "history-secret-token\n").unwrap();
    let mut config = history_config(format!("http://127.0.0.1:{}", address.port()));
    config.bearer_token_file = Some(token_path.clone());
    let service = ManagementHistoryService::from_config(&config)
        .unwrap()
        .unwrap();
    let result = service
        .query(request(ManagementHistoryQueryId::ReplicationSuccess))
        .await
        .unwrap();
    assert_eq!(result.state, ManagementHistoryState::Available);
    assert!(!serde_json::to_string(&result).unwrap().contains("tenant"));
    let calls = captured.0.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0]
        .0
        .starts_with("/api/v1/query_range?query=sum%28hydracache_replication_success_total%29"));
    assert_eq!(calls[0].1.as_deref(), Some("Bearer history-secret-token"));
    drop(calls);
    std::fs::remove_file(token_path).unwrap();
    server.abort();
}

async fn service_for(app: Router) -> (ManagementHistoryService, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let service = ManagementHistoryService::from_config(&history_config(format!(
        "http://127.0.0.1:{}",
        address.port()
    )))
    .unwrap()
    .unwrap();
    (service, server)
}

#[tokio::test]
async fn adapter_refuses_redirect_timeout_and_oversize_without_exposing_upstream_body() {
    let redirect_app = Router::new().route(
        "/api/v1/query_range",
        get(|| async {
            (
                StatusCode::FOUND,
                [("location", "http://169.254.169.254/latest/meta-data")],
                "credential=must-not-follow",
            )
        }),
    );
    let (service, task) = service_for(redirect_app).await;
    assert_eq!(
        service
            .query(request(ManagementHistoryQueryId::CacheEntries))
            .await,
        Err(ManagementHistoryError::Upstream)
    );
    task.abort();

    let timeout_app = Router::new().route(
        "/api/v1/query_range",
        get(|| async {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            axum::Json(json!({"status":"success","data":{"resultType":"matrix","result":[]}}))
        }),
    );
    let (service, task) = service_for(timeout_app).await;
    assert_eq!(
        service
            .query(request(ManagementHistoryQueryId::CacheEntries))
            .await,
        Err(ManagementHistoryError::Timeout)
    );
    task.abort();

    let oversize_app = Router::new().route(
        "/api/v1/query_range",
        get(|| async { vec![b'x'; MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES + 1] }),
    );
    let (service, task) = service_for(oversize_app).await;
    assert_eq!(
        service
            .query(request(ManagementHistoryQueryId::CacheEntries))
            .await,
        Err(ManagementHistoryError::Oversize)
    );
    task.abort();
}

fn management_request(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Body::empty())
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 262_144).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn disabled_adapter_falls_back_without_removing_browser_local_history() {
    let surface = AdminHttpSurface::new(ServerRuntime::new(ServerConfig::default()).unwrap());
    let path = format!(
        "{MANAGEMENT_HISTORY_PATH}?query_id=cache_entries&start_ms=1000000&end_ms=1060000&step_ms=10000"
    );
    let response = surface
        .routes()
        .oneshot(management_request(&path))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body(response).await;
    assert_eq!(body["data"]["state"], "no_adapter");
    assert_eq!(body["source"], "unavailable");

    let invalid = surface
        .routes()
        .oneshot(management_request(&format!(
            "{MANAGEMENT_HISTORY_PATH}?query_id=cache_entries&start_ms=1&end_ms=2&step_ms=1"
        )))
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let arbitrary = surface
        .routes()
        .oneshot(management_request(&format!(
            "{MANAGEMENT_HISTORY_PATH}?query_id=cache_entries&start_ms=1000000&end_ms=1060000&step_ms=10000&query=up&url=http://169.254.169.254"
        )))
        .await
        .unwrap();
    assert_eq!(arbitrary.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn enabled_admin_route_returns_only_bounded_typed_history() {
    let app = Router::new().route(
        "/api/v1/query_range",
        get(|| async {
            axum::Json(json!({"status":"success","data":{"resultType":"matrix","result":[{"metric":{"instance":"private-upstream-label"},"values":[[1000.0,"11"]]}]}}))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let upstream = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let config = ServerConfig {
        management_history: history_config(format!("http://127.0.0.1:{}", address.port())),
        ..ServerConfig::default()
    };
    let surface = AdminHttpSurface::new(ServerRuntime::new(config).unwrap());
    let response = surface
        .routes()
        .oneshot(management_request(&format!(
            "{MANAGEMENT_HISTORY_PATH}?query_id=cache_entries&start_ms=1000000&end_ms=1060000&step_ms=10000"
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body(response).await;
    assert_eq!(body["data"]["state"], "available");
    assert_eq!(body["data"]["series"][0]["points"][0]["value"], 11.0);
    assert!(!body.to_string().contains("private-upstream-label"));
    upstream.abort();
}

#[test]
fn canary_arbitrary_proxy_and_amplification_are_rejected_before_socket_open() {
    let config = ManagementHistoryConfig {
        enabled: true,
        origin: Some("https://prometheus.example".to_owned()),
        bearer_token_file: None,
        allow_http: false,
        allow_private_networks: false,
    };
    let mut amplified = request(ManagementHistoryQueryId::CacheEntries);
    amplified.end_ms = amplified
        .start_ms
        .saturating_add(MANAGEMENT_HISTORY_MAX_RANGE_MS + 1);
    let metadata = [SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
        443,
    )];
    let mut rejected =
        amplified.validate().is_err() && validate_resolved_addresses(&config, &metadata).is_err();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W9-ARBITRARY-PROXY") {
        rejected = false;
    }
    assert!(
        rejected,
        "HC-CANARY-RED:MC72-W9-ARBITRARY-PROXY arbitrary metadata destination or amplified range passed pre-socket validation"
    );
}
