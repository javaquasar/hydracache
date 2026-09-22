use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_protocol::{ClientRequest, ClientRequestEnvelope, Namespace, StructuredKey};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceLimits, ClientSurfaceState, HYDRACACHE_ADMIN_HEADER,
    HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{
    AdminHttpSurface, ServerConfig, ServerRuntime, MANAGEMENT_NAMESPACES_PATH,
    MANAGEMENT_NAMESPACE_CACHES_PREFIX,
};
use serde_json::Value;
use tower::ServiceExt;

fn isolated_state() -> Arc<ClientSurfaceState> {
    let tenant_a = Tenant::new("tenant-a")
        .unwrap()
        .allow_client("client-a")
        .namespace("other", NamespaceQuota::new(200, 10))
        .namespace("shared", NamespaceQuota::new(100, 8));
    let tenant_b = Tenant::new("tenant-b")
        .unwrap()
        .allow_client("client-b")
        .namespace("shared", NamespaceQuota::new(10_000, 80))
        .namespace("tenant-b-secret", NamespaceQuota::new(10_000, 80));
    let roster = TenantRoster::new(vec![tenant_a, tenant_b]).unwrap();
    Arc::new(
        ClientSurfaceState::with_isolation(
            ClientSurfaceLimits::default(),
            ConsumerIsolation::new(roster, ConsumerIsolationConfig::default()),
        )
        .unwrap(),
    )
}

fn put(
    state: &ClientSurfaceState,
    client: &str,
    tenant: &str,
    namespace: &str,
    key: &str,
    value: &[u8],
) {
    let identity = ClientIdentity::new(client, tenant).unwrap();
    let response = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            format!("request-{client}-{key}"),
            ClientRequest::Put {
                ns: Namespace::new(namespace).unwrap(),
                key: StructuredKey::new(vec![key.to_owned()]).unwrap(),
                value: value.to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ),
    );
    assert!(response.result.is_ok());
}

fn surface(state: Arc<ClientSurfaceState>) -> AdminHttpSurface {
    AdminHttpSurface::new(
        ServerRuntime::new(ServerConfig::default())
            .unwrap()
            .with_management_client_state(state)
            .start(),
    )
}

fn request(client: &str, tenant: &str, path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header(HYDRACACHE_CLIENT_ID_HEADER, client)
        .header(HYDRACACHE_TENANT_HEADER, tenant)
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Body::empty())
        .unwrap()
}

async fn json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 262_144).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn namespace_and_cache_views_are_authorized_before_totals_and_keep_unavailable_unknown() {
    let state = isolated_state();
    put(&state, "client-a", "tenant-a", "shared", "a-key", b"alpha");
    put(
        &state,
        "client-b",
        "tenant-b",
        "tenant-b-secret",
        "distinctive-b-key",
        b"distinctive-b-payload",
    );
    let router = surface(state).routes();
    let response = router
        .clone()
        .oneshot(request("client-a", "tenant-a", MANAGEMENT_NAMESPACES_PATH))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 2);
    let shared = body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["namespace"] == "shared")
        .unwrap();
    assert_eq!(shared["entries"], 1);
    assert_eq!(shared["logical_bytes"], 5);
    assert!(shared["retained_bytes"].is_null());
    let encoded = body.to_string();
    assert!(!encoded.contains("tenant-b-secret"));
    assert!(!encoded.contains("distinctive-b"));
    assert!(!encoded.contains("tenant-a"));

    let cache = router
        .clone()
        .oneshot(request(
            "client-a",
            "tenant-a",
            &format!("{MANAGEMENT_NAMESPACE_CACHES_PREFIX}/shared/caches"),
        ))
        .await
        .unwrap();
    assert_eq!(cache.status(), StatusCode::OK);
    let cache = json(cache).await;
    assert_eq!(cache["data"]["items"][0]["cache"], "client-surface");
    assert_eq!(cache["data"]["items"][0]["entries"], 1);
    assert!(cache["data"]["items"][0]["hit_total"].is_null());

    let forbidden = router
        .clone()
        .oneshot(request("client-a", "tenant-b", MANAGEMENT_NAMESPACES_PATH))
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    for hidden in ["tenant-b-secret", "does-not-exist"] {
        let response = router
            .clone()
            .oneshot(request(
                "client-a",
                "tenant-a",
                &format!("{MANAGEMENT_NAMESPACE_CACHES_PREFIX}/{hidden}/caches"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn namespace_pagination_is_stable_and_bound_to_the_observation() {
    let router = surface(isolated_state()).routes();
    let first = json(
        router
            .clone()
            .oneshot(request(
                "client-a",
                "tenant-a",
                &format!("{MANAGEMENT_NAMESPACES_PATH}?limit=1"),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first["data"]["items"][0]["namespace"], "other");
    assert_eq!(first["data"]["truncated"], true);
    let cursor = first["data"]["next_cursor"].as_str().unwrap();
    let second = json(
        router
            .oneshot(request(
                "client-a",
                "tenant-a",
                &format!("{MANAGEMENT_NAMESPACES_PATH}?limit=1&cursor={cursor}"),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(second["data"]["items"][0]["namespace"], "shared");
    assert_eq!(second["data"]["truncated"], false);
}

#[tokio::test]
async fn canary_cross_tenant_source_never_changes_authorized_body() {
    let state = isolated_state();
    put(&state, "client-a", "tenant-a", "shared", "a-key", b"alpha");
    put(
        &state,
        "client-b",
        "tenant-b",
        "tenant-b-secret",
        "cross-tenant-canary-key",
        b"cross-tenant-canary-payload",
    );
    let defect = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W7-CROSS-TENANT");
    let (client, tenant) = if defect {
        ("client-b", "tenant-b")
    } else {
        ("client-a", "tenant-a")
    };
    let response = surface(state)
        .routes()
        .oneshot(request(client, tenant, MANAGEMENT_NAMESPACES_PATH))
        .await
        .unwrap();
    let body = json(response).await.to_string();
    assert!(
        !body.contains("tenant-b-secret") && !body.contains("cross-tenant-canary"),
        "HC-CANARY-RED:MC72-W7-CROSS-TENANT hidden tenant data escaped authorization"
    );
}
