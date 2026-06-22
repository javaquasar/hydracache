use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{HeaderValue, Request, StatusCode};
use bytes::Bytes;
use hydracache::ClusterGeneration;
use hydracache_cluster_transport_axum::{
    AxumPeerFetchService, HttpWireCompatibility, MemoryPeerFetchStore, PeerFetchHttpErrorBody,
    PeerFetchHttpRequest, PeerFetchHttpResponse, DEFAULT_PEER_FETCH_PATH,
    HYDRACACHE_HTTP_WIRE_VERSION, HYDRACACHE_WIRE_VERSION_HEADER,
};
use serde::de::DeserializeOwned;
use tower::ServiceExt;

#[tokio::test]
async fn replication_wire_version_mismatch_rejects_replication_safely() {
    let store = MemoryPeerFetchStore::new();
    store.put("user:42", Bytes::from_static(b"encoded-user"));
    let app = AxumPeerFetchService::new("member-a", ClusterGeneration::new(7), Arc::new(store))
        .with_wire_compatibility(HttpWireCompatibility::strict_current())
        .routes();

    let mut rejected = peer_fetch_request("user:42", HYDRACACHE_HTTP_WIRE_VERSION + 1);
    rejected.headers_mut().insert(
        HYDRACACHE_WIRE_VERSION_HEADER,
        HeaderValue::from_static("999"),
    );
    let rejected = app.clone().oneshot(rejected).await.unwrap();

    assert_eq!(rejected.status(), StatusCode::UPGRADE_REQUIRED);
    let rejected: PeerFetchHttpErrorBody = response_json(rejected).await;
    assert_eq!(rejected.code, "wire-version-mismatch");
}

#[tokio::test]
async fn replication_current_wire_version_round_trips_encoded_bytes() {
    let store = MemoryPeerFetchStore::new();
    store.put("user:42", Bytes::from_static(b"encoded-user"));
    let app = AxumPeerFetchService::new("member-a", ClusterGeneration::new(7), Arc::new(store))
        .with_wire_compatibility(HttpWireCompatibility::strict_current())
        .routes();

    let response = app
        .oneshot(peer_fetch_request("user:42", HYDRACACHE_HTTP_WIRE_VERSION))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let response: PeerFetchHttpResponse = response_json(response).await;
    assert_eq!(response.decode_value().unwrap(), Some(Bytes::from_static(b"encoded-user")));
}

fn peer_fetch_request(key: &str, wire_version: u16) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(DEFAULT_PEER_FETCH_PATH)
        .header("content-type", "application/json")
        .header(HYDRACACHE_WIRE_VERSION_HEADER, wire_version.to_string())
        .body(Body::from(
            serde_json::to_vec(&PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: key.to_owned(),
                generation: Some(7),
            })
            .unwrap(),
        ))
        .unwrap()
}

async fn response_json<T>(response: axum::response::Response) -> T
where
    T: DeserializeOwned,
{
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
