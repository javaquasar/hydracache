use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{HeaderMap, StatusCode};
use hydracache::ClusterNodeId;
use hydracache_cluster_transport_axum::{
    AllowAllAuthorizer, AxumClusterMessageService, ClusterOpaqueMessage, ClusterRoute,
    ClusterRouteAuth, ClusterRouteErrorBody, DenyRouteAuthorizer, MemoryClusterMessageHandler,
    NodeCredential, NodeIdentityProvider, StaticNodeIdentityProvider, DEFAULT_RAFT_APPEND_PATH,
    DEFAULT_REPLICATION_PATH, HYDRACACHE_NODE_KEY_ID_HEADER, HYDRACACHE_NODE_TOKEN_HEADER,
};
use tower::ServiceExt;

fn secure_auth() -> ClusterRouteAuth {
    ClusterRouteAuth::secure(
        Arc::new(StaticNodeIdentityProvider::new(
            ClusterNodeId::from("member-a"),
            "k2",
            "secret-new",
        )),
        Arc::new(AllowAllAuthorizer),
    )
}

fn headers_for(key_id: &str, token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(HYDRACACHE_NODE_KEY_ID_HEADER, key_id.parse().unwrap());
    headers.insert(HYDRACACHE_NODE_TOKEN_HEADER, token.parse().unwrap());
    headers
}

#[test]
fn cluster_auth_unauthenticated_peer_fetch_is_rejected() {
    let auth = secure_auth();

    let error = auth
        .verify(ClusterRoute::PeerFetch, &HeaderMap::new())
        .unwrap_err();

    assert_eq!(error.status, StatusCode::UNAUTHORIZED);
    assert_eq!(error.code, "unauthenticated");
    assert_eq!(auth.rejected_total(), 1);
    assert_eq!(auth.rejected_total_for_route(ClusterRoute::PeerFetch), 1);
    assert_eq!(auth.rejected_total_for_route(ClusterRoute::Replicate), 0);
}

#[test]
fn cluster_auth_unauthorized_route_is_denied() {
    let auth = ClusterRouteAuth::secure(
        Arc::new(StaticNodeIdentityProvider::new(
            ClusterNodeId::from("member-a"),
            "k1",
            "secret",
        )),
        Arc::new(DenyRouteAuthorizer::new(ClusterRoute::Admin)),
    );
    let headers = headers_for("k1", "secret");

    let error = auth.verify(ClusterRoute::Admin, &headers).unwrap_err();

    assert_eq!(error.status, StatusCode::FORBIDDEN);
    assert_eq!(error.code, "unauthorized");
    assert_eq!(auth.rejected_total(), 1);
    assert_eq!(auth.rejected_total_for_route(ClusterRoute::Admin), 1);
}

#[test]
fn cluster_auth_rotation_window_accepts_old_and_new() {
    let provider =
        StaticNodeIdentityProvider::new(ClusterNodeId::from("member-a"), "new", "secret-new")
            .with_previous("old", "secret-old");

    assert_eq!(
        provider
            .verify(&NodeCredential::new("old", "secret-old"))
            .unwrap(),
        ClusterNodeId::from("member-a")
    );
    assert_eq!(
        provider
            .verify(&NodeCredential::new("new", "secret-new"))
            .unwrap(),
        ClusterNodeId::from("member-a")
    );
    assert_eq!(
        provider.accepted(),
        vec!["new".to_owned(), "old".to_owned()]
    );

    let rotated =
        StaticNodeIdentityProvider::new(ClusterNodeId::from("member-a"), "newer", "secret-newer");
    assert!(rotated
        .verify(&NodeCredential::new("old", "secret-old"))
        .is_err());
}

#[test]
fn cluster_auth_missing_provider_refuses_replication_routes_unless_acked() {
    let missing = ClusterRouteAuth::missing_provider();
    assert!(!missing.route_enabled(ClusterRoute::Replicate));
    assert!(missing
        .verify(ClusterRoute::Replicate, &HeaderMap::new())
        .is_err());
    assert_eq!(missing.rejected_total_for_route(ClusterRoute::Replicate), 1);

    let acknowledged =
        ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true);
    assert!(acknowledged.route_enabled(ClusterRoute::Replicate));
    assert_eq!(
        acknowledged
            .verify(ClusterRoute::Replicate, &HeaderMap::new())
            .unwrap(),
        ClusterNodeId::from("insecure-trust-boundary")
    );
}

#[test]
fn cluster_auth_raft_transport_requires_identity() {
    let auth = secure_auth();

    let error = auth
        .verify(ClusterRoute::RaftAppend, &HeaderMap::new())
        .unwrap_err();

    assert_eq!(error.status, StatusCode::UNAUTHORIZED);
    assert_eq!(auth.rejected_total(), 1);
    assert_eq!(auth.rejected_total_for_route(ClusterRoute::RaftAppend), 1);
}

#[test]
fn cluster_auth_outbound_headers_present_current_credential() {
    let auth = secure_auth();
    let mut headers = HeaderMap::new();

    auth.apply_outbound_headers(&mut headers).unwrap();

    assert_eq!(headers[HYDRACACHE_NODE_KEY_ID_HEADER], "k2");
    assert_eq!(headers[HYDRACACHE_NODE_TOKEN_HEADER], "secret-new");
    assert_eq!(
        auth.verify(ClusterRoute::OwnerLoad, &headers).unwrap(),
        ClusterNodeId::from("member-a")
    );
}

#[tokio::test]
async fn cluster_auth_unauthenticated_raft_append_is_rejected() {
    let auth = secure_auth();
    let handler = Arc::new(MemoryClusterMessageHandler::new("member-b"));
    let app = AxumClusterMessageService::new("member-b", handler, auth.clone()).routes();
    let body = serde_json::to_vec(&ClusterOpaqueMessage::new(
        "member-a", "member-b", 1, b"append",
    ))
    .unwrap();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(DEFAULT_RAFT_APPEND_PATH)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error: ClusterRouteErrorBody = serde_json::from_slice(&body).unwrap();
    assert_eq!(error.code, "unauthenticated");
    assert_eq!(error.route, ClusterRoute::RaftAppend.as_str());
    assert_eq!(auth.rejected_total(), 1);
    assert_eq!(auth.rejected_total_for_route(ClusterRoute::RaftAppend), 1);
}

#[tokio::test]
async fn cluster_auth_unauthorized_replication_is_denied() {
    let auth = ClusterRouteAuth::secure(
        Arc::new(StaticNodeIdentityProvider::new(
            ClusterNodeId::from("member-a"),
            "k1",
            "secret",
        )),
        Arc::new(DenyRouteAuthorizer::new(ClusterRoute::Replicate)),
    );
    let handler = Arc::new(MemoryClusterMessageHandler::new("member-b"));
    let auth_for_assertions = auth.clone();
    let app = AxumClusterMessageService::new("member-b", handler, auth).routes();
    let body = serde_json::to_vec(&ClusterOpaqueMessage::new(
        "member-a",
        "member-b",
        1,
        b"replicate",
    ))
    .unwrap();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(DEFAULT_REPLICATION_PATH)
                .header("content-type", "application/json")
                .header(HYDRACACHE_NODE_KEY_ID_HEADER, "k1")
                .header(HYDRACACHE_NODE_TOKEN_HEADER, "secret")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error: ClusterRouteErrorBody = serde_json::from_slice(&body).unwrap();
    assert_eq!(error.code, "unauthorized");
    assert_eq!(error.route, ClusterRoute::Replicate.as_str());
    assert_eq!(auth_for_assertions.rejected_total(), 1);
    assert_eq!(
        auth_for_assertions.rejected_total_for_route(ClusterRoute::Replicate),
        1
    );
}
