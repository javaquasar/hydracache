use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{HeaderValue, Request, StatusCode};
use bytes::Bytes;
use hydracache::ClusterGeneration;
use hydracache_cluster_transport_axum::{
    AxumPeerFetchService, HttpTransportAuth, HttpWireCompatibility, MemoryPeerFetchStore,
    PeerFetchHttpErrorBody, PeerFetchHttpRequest, PeerFetchHttpResponse, DEFAULT_PEER_FETCH_PATH,
    HYDRACACHE_HTTP_WIRE_VERSION, HYDRACACHE_TOKEN_HEADER, HYDRACACHE_WIRE_VERSION_HEADER,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

#[tokio::test]
async fn staging_gate_peer_fetch_auth_accept_and_deny() {
    let store = MemoryPeerFetchStore::new();
    store.put("user:42", Bytes::from_static(b"encoded-user"));
    let app = AxumPeerFetchService::new("member-a", ClusterGeneration::new(7), Arc::new(store))
        .with_auth(HttpTransportAuth::token("secret"))
        .with_wire_compatibility(HttpWireCompatibility::strict_current())
        .routes();

    let denied = app
        .clone()
        .oneshot(peer_fetch_request("user:42", true, false))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let denied: PeerFetchHttpErrorBody = response_json(denied).await;
    assert_eq!(denied.code, "unauthorized");

    let accepted = app
        .oneshot(peer_fetch_request("user:42", true, true))
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
    let accepted: PeerFetchHttpResponse = response_json(accepted).await;
    assert_eq!(accepted.owner, "member-a");
    assert_eq!(accepted.key, "user:42");
    assert_eq!(
        accepted.decode_value().unwrap(),
        Some(Bytes::from_static(b"encoded-user"))
    );
}

#[tokio::test]
async fn staging_gate_wire_version_accept_and_reject() {
    let store = MemoryPeerFetchStore::new();
    store.put("user:42", Bytes::from_static(b"encoded-user"));
    let app = AxumPeerFetchService::new("member-a", ClusterGeneration::new(7), Arc::new(store))
        .with_wire_compatibility(HttpWireCompatibility::strict_current())
        .routes();

    let mut rejected = peer_fetch_request("user:42", false, false);
    rejected.headers_mut().insert(
        HYDRACACHE_WIRE_VERSION_HEADER,
        HeaderValue::from_static("999"),
    );
    let rejected = app.clone().oneshot(rejected).await.unwrap();
    assert_eq!(rejected.status(), StatusCode::UPGRADE_REQUIRED);
    let rejected: PeerFetchHttpErrorBody = response_json(rejected).await;
    assert_eq!(rejected.code, "wire-version-mismatch");

    let accepted = app
        .oneshot(peer_fetch_request("user:42", true, false))
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
    let accepted: PeerFetchHttpResponse = response_json(accepted).await;
    assert_eq!(accepted.value_base64.is_some(), true);
}

fn peer_fetch_request(key: &str, include_wire: bool, include_token: bool) -> Request<Body> {
    let mut request = json_request(PeerFetchHttpRequest {
        owner: "member-a".to_owned(),
        key: key.to_owned(),
        generation: Some(7),
    });
    if include_wire {
        request.headers_mut().insert(
            HYDRACACHE_WIRE_VERSION_HEADER,
            HeaderValue::from_str(&HYDRACACHE_HTTP_WIRE_VERSION.to_string()).unwrap(),
        );
    }
    if include_token {
        request
            .headers_mut()
            .insert(HYDRACACHE_TOKEN_HEADER, HeaderValue::from_static("secret"));
    }
    request
}

fn json_request<T>(body: T) -> Request<Body>
where
    T: Serialize,
{
    Request::builder()
        .method("POST")
        .uri(DEFAULT_PEER_FETCH_PATH)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

async fn response_json<T>(response: axum::response::Response) -> T
where
    T: DeserializeOwned,
{
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
