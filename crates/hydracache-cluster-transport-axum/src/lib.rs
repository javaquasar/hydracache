//! Axum/HTTP peer-fetch transport for HydraCache cluster members.
//!
//! The base `hydracache` crate exposes the transport-neutral
//! [`ClusterPeerFetch`] seam. This crate provides
//! an opt-in HTTP implementation so local-only applications do not inherit
//! HTTP client/server dependencies.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use hydracache::{
//!     CacheOptions, ClusterGeneration, ClusterPeerFetch, ClusterPeerFetchRequest, HydraCache,
//! };
//! use hydracache_cluster_transport_axum::{AxumPeerFetchService, HttpPeerFetch};
//!
//! # async fn example() -> hydracache::CacheResult<()> {
//! let owner_cache = HydraCache::local().build();
//! owner_cache.put("user:42", 42_u64, CacheOptions::new()).await?;
//!
//! let routes = AxumPeerFetchService::new(
//!     "member-a",
//!     ClusterGeneration::new(1),
//!     Arc::new(owner_cache),
//! )
//! .routes();
//! # let _ = routes;
//!
//! let peer_fetch = HttpPeerFetch::for_base_url("http://127.0.0.1:3000");
//! let response = peer_fetch
//!     .fetch(
//!         ClusterPeerFetchRequest::new("member-a", "user:42")
//!             .generation(ClusterGeneration::new(1)),
//!     )
//!     .await;
//! # let _ = response;
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use bytes::Bytes;
use hydracache::{
    CacheError, CacheResult, ClusterGeneration, ClusterNodeId, ClusterPeerFetch,
    ClusterPeerFetchRequest, ClusterPeerFetchResponse, HydraCache,
};
use hydracache_core::CacheCodec;
use serde::{Deserialize, Serialize};

/// Default HTTP path used by the peer-fetch route and client.
pub const DEFAULT_PEER_FETCH_PATH: &str = "/cluster/peer-fetch";

/// Owner-side store abstraction used by the HTTP route.
///
/// The store returns encoded bytes. It deliberately does not deserialize values
/// because remote peer fetch should be type-agnostic and codec-preserving.
#[async_trait::async_trait]
pub trait PeerFetchStore: Send + Sync + 'static {
    /// Return the encoded value for `key`, or `None` on miss/expiry.
    async fn get_encoded(&self, key: &str) -> CacheResult<Option<Bytes>>;
}

#[async_trait::async_trait]
impl<C> PeerFetchStore for HydraCache<C>
where
    C: CacheCodec + Send + Sync + 'static,
{
    async fn get_encoded(&self, key: &str) -> CacheResult<Option<Bytes>> {
        HydraCache::get_encoded(self, key).await
    }
}

/// Simple owner-side encoded-byte store for transport tests and examples.
#[derive(Debug, Clone, Default)]
pub struct MemoryPeerFetchStore {
    values: Arc<Mutex<BTreeMap<String, Bytes>>>,
}

impl MemoryPeerFetchStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store encoded bytes for one key.
    pub fn put(&self, key: impl Into<String>, value: impl Into<Bytes>) {
        self.values
            .lock()
            .expect("peer-fetch store poisoned")
            .insert(key.into(), value.into());
    }

    /// Remove encoded bytes for one key.
    pub fn remove(&self, key: &str) -> Option<Bytes> {
        self.values
            .lock()
            .expect("peer-fetch store poisoned")
            .remove(key)
    }

    /// Return the number of stored keys.
    pub fn len(&self) -> usize {
        self.values.lock().expect("peer-fetch store poisoned").len()
    }

    /// Return whether the store has no keys.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait::async_trait]
impl PeerFetchStore for MemoryPeerFetchStore {
    async fn get_encoded(&self, key: &str) -> CacheResult<Option<Bytes>> {
        Ok(self
            .values
            .lock()
            .expect("peer-fetch store poisoned")
            .get(key)
            .cloned())
    }
}

#[derive(Clone)]
struct PeerFetchState {
    owner: ClusterNodeId,
    generation: ClusterGeneration,
    store: Arc<dyn PeerFetchStore>,
}

/// Axum route factory for serving peer-fetch requests from one member.
#[derive(Clone)]
pub struct AxumPeerFetchService {
    state: PeerFetchState,
}

impl fmt::Debug for AxumPeerFetchService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AxumPeerFetchService")
            .field("owner", &self.state.owner)
            .field("generation", &self.state.generation)
            .finish_non_exhaustive()
    }
}

impl AxumPeerFetchService {
    /// Create a peer-fetch service for one member owner.
    pub fn new(
        owner: impl Into<ClusterNodeId>,
        generation: ClusterGeneration,
        store: Arc<dyn PeerFetchStore>,
    ) -> Self {
        Self {
            state: PeerFetchState {
                owner: owner.into(),
                generation,
                store,
            },
        }
    }

    /// Return the owner node id served by this route.
    pub fn owner(&self) -> &ClusterNodeId {
        &self.state.owner
    }

    /// Return the owner generation served by this route.
    pub fn generation(&self) -> ClusterGeneration {
        self.state.generation
    }

    /// Build the Axum router with `POST /cluster/peer-fetch`.
    pub fn routes(&self) -> Router {
        Router::new()
            .route(DEFAULT_PEER_FETCH_PATH, post(handle_peer_fetch))
            .with_state(self.state.clone())
    }
}

/// JSON request body used by the HTTP transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerFetchHttpRequest {
    /// Expected owner member id.
    pub owner: String,
    /// Logical cache key requested from that owner.
    pub key: String,
    /// Optional owner generation observed by the caller.
    pub generation: Option<u64>,
}

impl PeerFetchHttpRequest {
    /// Build a transport DTO from the transport-neutral request.
    pub fn from_peer_request(request: &ClusterPeerFetchRequest) -> Self {
        Self {
            owner: request.owner.as_str().to_owned(),
            key: request.key.clone(),
            generation: request.generation.map(ClusterGeneration::value),
        }
    }
}

/// JSON response body used by the HTTP transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerFetchHttpResponse {
    /// Owner member that served the request.
    pub owner: String,
    /// Logical cache key requested from the owner.
    pub key: String,
    /// Base64-encoded cache bytes. `None` means cache miss.
    pub value_base64: Option<String>,
}

impl PeerFetchHttpResponse {
    /// Build a transport DTO from the transport-neutral response.
    pub fn from_peer_response(response: &ClusterPeerFetchResponse) -> Self {
        Self {
            owner: response.owner.as_str().to_owned(),
            key: response.key.clone(),
            value_base64: response
                .value
                .as_ref()
                .map(|value| BASE64_STANDARD.encode(value.as_ref())),
        }
    }

    /// Decode the optional base64 payload into bytes.
    pub fn decode_value(&self) -> CacheResult<Option<Bytes>> {
        self.value_base64
            .as_ref()
            .map(|value| {
                BASE64_STANDARD
                    .decode(value)
                    .map(Bytes::from)
                    .map_err(|error| {
                        CacheError::Decode(format!("invalid peer-fetch payload: {error}"))
                    })
            })
            .transpose()
    }

    fn into_peer_response(self) -> CacheResult<ClusterPeerFetchResponse> {
        let value = self.decode_value()?;
        Ok(ClusterPeerFetchResponse {
            owner: self.owner.into(),
            key: self.key,
            value,
        })
    }
}

/// JSON error body returned by the HTTP route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerFetchHttpErrorBody {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable error detail.
    pub message: String,
    /// Generation observed by the caller, if provided.
    pub requested_generation: Option<u64>,
    /// Current owner generation on this server, when relevant.
    pub current_generation: Option<u64>,
}

async fn handle_peer_fetch(
    State(state): State<PeerFetchState>,
    Json(request): Json<PeerFetchHttpRequest>,
) -> Response {
    if request.owner != state.owner.as_str() {
        return error_response(
            StatusCode::NOT_FOUND,
            "owner-mismatch",
            format!(
                "peer-fetch route serves owner '{}', not '{}'",
                state.owner, request.owner
            ),
            request.generation,
            Some(state.generation.value()),
        );
    }

    if let Some(requested_generation) = request.generation {
        let requested = ClusterGeneration::new(requested_generation);
        if requested != state.generation {
            return error_response(
                StatusCode::CONFLICT,
                "generation-mismatch",
                format!(
                    "requested owner generation {} does not match current generation {}",
                    requested_generation,
                    state.generation.value()
                ),
                Some(requested_generation),
                Some(state.generation.value()),
            );
        }
    }

    match state.store.get_encoded(&request.key).await {
        Ok(value) => {
            let response = PeerFetchHttpResponse {
                owner: state.owner.as_str().to_owned(),
                key: request.key,
                value_base64: value.map(|value| BASE64_STANDARD.encode(value.as_ref())),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "store-error",
            error.to_string(),
            request.generation,
            Some(state.generation.value()),
        ),
    }
}

fn error_response(
    status: StatusCode,
    code: impl Into<String>,
    message: impl Into<String>,
    requested_generation: Option<u64>,
    current_generation: Option<u64>,
) -> Response {
    (
        status,
        Json(PeerFetchHttpErrorBody {
            code: code.into(),
            message: message.into(),
            requested_generation,
            current_generation,
        }),
    )
        .into_response()
}

/// HTTP client implementation of [`ClusterPeerFetch`].
#[derive(Debug, Clone)]
pub struct HttpPeerFetch {
    endpoint: String,
    client: reqwest::Client,
}

impl HttpPeerFetch {
    /// Create a peer-fetch client for a full endpoint URL.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Create a peer-fetch client from a member base URL.
    ///
    /// `DEFAULT_PEER_FETCH_PATH` is appended after trimming a trailing slash.
    pub fn for_base_url(base_url: impl AsRef<str>) -> Self {
        let base_url = base_url.as_ref().trim_end_matches('/');
        Self::new(format!("{base_url}{DEFAULT_PEER_FETCH_PATH}"))
    }

    /// Return the endpoint URL used by this client.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

#[async_trait::async_trait]
impl ClusterPeerFetch for HttpPeerFetch {
    async fn fetch(
        &self,
        request: ClusterPeerFetchRequest,
    ) -> CacheResult<ClusterPeerFetchResponse> {
        let expected_owner = request.owner.clone();
        let expected_key = request.key.clone();
        let http_request = PeerFetchHttpRequest::from_peer_request(&request);
        let response = self
            .client
            .post(&self.endpoint)
            .json(&http_request)
            .send()
            .await
            .map_err(|error| CacheError::Backend(format!("peer-fetch request failed: {error}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|error| format!("failed to read error body: {error}"));
            return Err(CacheError::Backend(format!(
                "peer-fetch request failed with {status}: {body}"
            )));
        }

        let response = response
            .json::<PeerFetchHttpResponse>()
            .await
            .map_err(|error| CacheError::Decode(format!("invalid peer-fetch response: {error}")))?;

        if response.owner != expected_owner.as_str() || response.key != expected_key {
            return Err(CacheError::Backend(format!(
                "peer-fetch response identity mismatch: expected owner/key '{expected_owner}/{expected_key}', got '{}/{}'",
                response.owner, response.key
            )));
        }

        response.into_peer_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use serde::de::DeserializeOwned;
    use tokio::sync::oneshot;
    use tower::ServiceExt;

    #[tokio::test]
    async fn memory_store_reports_hits_and_misses() {
        let store = MemoryPeerFetchStore::new();
        assert!(store.is_empty());

        store.put("user:42", Bytes::from_static(b"encoded-user"));

        assert_eq!(store.len(), 1);
        assert_eq!(
            store.get_encoded("user:42").await.unwrap(),
            Some(Bytes::from_static(b"encoded-user"))
        );
        assert_eq!(store.get_encoded("missing").await.unwrap(), None);
        assert_eq!(
            store.remove("user:42"),
            Some(Bytes::from_static(b"encoded-user"))
        );
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn route_returns_base64_hit() {
        let store = MemoryPeerFetchStore::new();
        store.put("user:42", Bytes::from_static(b"encoded-user"));
        let app = service_with_store(store).routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "user:42".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeerFetchHttpResponse = response_json(response).await;
        assert_eq!(body.owner, "member-a");
        assert_eq!(body.key, "user:42");
        assert_eq!(
            body.decode_value().unwrap(),
            Some(Bytes::from_static(b"encoded-user"))
        );
    }

    #[tokio::test]
    async fn route_returns_miss_for_missing_key() {
        let app = service_with_store(MemoryPeerFetchStore::new()).routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "missing".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeerFetchHttpResponse = response_json(response).await;
        assert_eq!(body.value_base64, None);
        assert_eq!(body.decode_value().unwrap(), None);
    }

    #[tokio::test]
    async fn route_rejects_wrong_owner() {
        let app = service_with_store(MemoryPeerFetchStore::new()).routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-b".to_owned(),
                key: "user:42".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body: PeerFetchHttpErrorBody = response_json(response).await;
        assert_eq!(body.code, "owner-mismatch");
        assert_eq!(body.current_generation, Some(7));
    }

    #[tokio::test]
    async fn route_rejects_stale_generation() {
        let app = service_with_store(MemoryPeerFetchStore::new()).routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "user:42".to_owned(),
                generation: Some(6),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body: PeerFetchHttpErrorBody = response_json(response).await;
        assert_eq!(body.code, "generation-mismatch");
        assert_eq!(body.requested_generation, Some(6));
        assert_eq!(body.current_generation, Some(7));
    }

    #[tokio::test]
    async fn http_peer_fetch_round_trips_against_axum_server() {
        let store = MemoryPeerFetchStore::new();
        store.put("user:42", Bytes::from_static(b"encoded-user"));
        let app = service_with_store(store).routes();
        let (base_url, shutdown, server) = spawn_server(app).await;
        let peer_fetch = HttpPeerFetch::for_base_url(&base_url);

        let response = peer_fetch
            .fetch(
                ClusterPeerFetchRequest::new("member-a", "user:42")
                    .generation(ClusterGeneration::new(7)),
            )
            .await
            .unwrap();

        assert_eq!(
            peer_fetch.endpoint(),
            format!("{base_url}{DEFAULT_PEER_FETCH_PATH}")
        );
        assert!(response.is_hit());
        assert_eq!(response.owner.as_str(), "member-a");
        assert_eq!(response.key, "user:42");
        assert_eq!(response.value.unwrap().as_ref(), b"encoded-user");

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn http_peer_fetch_maps_miss_and_generation_error() {
        let app = service_with_store(MemoryPeerFetchStore::new()).routes();
        let (base_url, shutdown, server) = spawn_server(app).await;
        let peer_fetch = HttpPeerFetch::for_base_url(&base_url);

        let missing = peer_fetch
            .fetch(
                ClusterPeerFetchRequest::new("member-a", "missing")
                    .generation(ClusterGeneration::new(7)),
            )
            .await
            .unwrap();
        assert!(missing.is_miss());

        let error = peer_fetch
            .fetch(
                ClusterPeerFetchRequest::new("member-a", "missing")
                    .generation(ClusterGeneration::new(6)),
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("generation-mismatch"));

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn http_response_rejects_invalid_base64() {
        let response = PeerFetchHttpResponse {
            owner: "member-a".to_owned(),
            key: "user:42".to_owned(),
            value_base64: Some("not base64".to_owned()),
        };

        let error = response.decode_value().unwrap_err();
        assert!(matches!(error, CacheError::Decode(_)));
    }

    #[tokio::test]
    async fn hydracache_implements_peer_fetch_store() {
        let cache = HydraCache::local().build();
        cache
            .put("answer", 42_u64, hydracache::CacheOptions::new())
            .await
            .unwrap();

        let encoded = PeerFetchStore::get_encoded(&cache, "answer")
            .await
            .unwrap()
            .expect("stored bytes");

        assert!(!encoded.is_empty());
    }

    fn service_with_store(store: MemoryPeerFetchStore) -> AxumPeerFetchService {
        AxumPeerFetchService::new("member-a", ClusterGeneration::new(7), Arc::new(store))
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

    async fn response_json<T>(response: Response) -> T
    where
        T: DeserializeOwned,
    {
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn spawn_server(
        app: Router,
    ) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        (format!("http://{addr}"), shutdown_tx, server)
    }
}
