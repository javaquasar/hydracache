use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode};
use hydracache::ClusterNodeId;
use hydracache_cluster_transport_axum::{
    AllowAllAuthorizer, ClusterRoute, ClusterRouteAuth, DenyRouteAuthorizer, NodeCredential,
    NodeIdentityProvider, StaticNodeIdentityProvider, HYDRACACHE_NODE_KEY_ID_HEADER,
    HYDRACACHE_NODE_TOKEN_HEADER,
};

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
