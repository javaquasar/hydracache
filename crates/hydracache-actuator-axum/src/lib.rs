//! Axum read-only actuator endpoints for HydraCache.
//!
//! The actuator is intentionally a separate crate so applications that only
//! need embedded caching do not pull in HTTP dependencies.
//!
//! # Example
//!
//! ```rust
//! use axum::Router;
//! use hydracache::HydraCache;
//! use hydracache_actuator_axum::HydraCacheActuator;
//! use hydracache_observability::HydraCacheRegistry;
//!
//! let cache = HydraCache::local().build();
//! let registry = HydraCacheRegistry::new().with_cache("main", cache);
//! let app: Router = Router::new().nest(
//!     "/actuator/hydracache",
//!     HydraCacheActuator::new(registry).routes(),
//! );
//! # let _ = app;
//! ```
//!
//! Exposed routes are read-only:
//!
//! ```text
//! GET /health
//! GET /caches
//! GET /caches/{name}/diagnostics
//! GET /caches/{name}/stats
//! GET /
//! ```

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use hydracache_observability::{
    CacheDiagnosticsSnapshot, CacheStatsSnapshot, HydraCacheOverview, HydraCacheRegistry,
};
use serde::Serialize;

/// Read-only Axum actuator for registered HydraCache instances.
#[derive(Debug, Clone)]
pub struct HydraCacheActuator {
    registry: HydraCacheRegistry,
}

impl HydraCacheActuator {
    /// Create a new actuator from a framework-neutral cache registry.
    pub fn new(registry: HydraCacheRegistry) -> Self {
        Self { registry }
    }

    /// Build routes for nesting under an application-controlled prefix.
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum::Router;
    /// use hydracache::HydraCache;
    /// use hydracache_actuator_axum::HydraCacheActuator;
    /// use hydracache_observability::HydraCacheRegistry;
    ///
    /// let cache = HydraCache::local().build();
    /// let registry = HydraCacheRegistry::new().with_cache("main", cache);
    ///
    /// let app: Router = Router::new()
    ///     .nest("/actuator/hydracache", HydraCacheActuator::new(registry).routes());
    /// # let _ = app;
    /// ```
    pub fn routes(self) -> Router {
        Self::routes_for(self.registry)
    }

    /// Build routes directly from a registry.
    pub fn routes_for(registry: HydraCacheRegistry) -> Router {
        Router::new()
            .route("/", get(overview))
            .route("/health", get(health))
            .route("/caches", get(caches))
            .route("/caches/{name}/diagnostics", get(cache_diagnostics))
            .route("/caches/{name}/stats", get(cache_stats))
            .with_state(registry)
    }
}

/// Health response for the read-only actuator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActuatorHealth {
    /// Simple status field for smoke checks.
    pub status: &'static str,
    /// Number of registered caches.
    pub cache_count: usize,
}

/// Cache-name list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CacheList {
    /// Registered cache names in stable sorted order.
    pub caches: Vec<String>,
}

async fn overview(State(registry): State<HydraCacheRegistry>) -> Json<HydraCacheOverview> {
    Json(registry.overview().await)
}

async fn health(State(registry): State<HydraCacheRegistry>) -> Json<ActuatorHealth> {
    Json(ActuatorHealth {
        status: "UP",
        cache_count: registry.len(),
    })
}

async fn caches(State(registry): State<HydraCacheRegistry>) -> Json<CacheList> {
    Json(CacheList {
        caches: registry.cache_names(),
    })
}

async fn cache_diagnostics(
    State(registry): State<HydraCacheRegistry>,
    Path(name): Path<String>,
) -> Result<Json<CacheDiagnosticsSnapshot>, StatusCode> {
    registry
        .diagnostics(&name)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn cache_stats(
    State(registry): State<HydraCacheRegistry>,
    Path(name): Path<String>,
) -> Result<Json<CacheStatsSnapshot>, StatusCode> {
    let diagnostics = registry
        .diagnostics(&name)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(diagnostics.stats))
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use hydracache::{CacheOptions, HydraCache};
    use hydracache_observability::HydraCacheRegistry;
    use serde_json::Value;
    use tower::ServiceExt;

    use super::HydraCacheActuator;

    #[tokio::test]
    async fn actuator_routes_return_read_only_cache_diagnostics() {
        let cache = HydraCache::local().build();
        cache
            .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
            .await
            .unwrap();
        cache
            .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
            .await
            .unwrap();

        let registry = HydraCacheRegistry::new().with_cache("main", cache);
        let app = HydraCacheActuator::new(registry).routes();

        let response = app
            .oneshot(request("/caches/main/diagnostics"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = json_body(response).await;
        assert_eq!(body["name"], "main");
        assert_eq!(body["stats"]["loads"], 1);
        assert_eq!(body["stats"]["hits"], 1);
        assert_eq!(body["stats"]["total_requests"], 2);
        assert_eq!(body["stats"]["hit_ratio"], 0.5);
        assert_eq!(body["empty"], false);
    }

    #[tokio::test]
    async fn actuator_routes_return_health_cache_list_overview_and_stats() {
        let cache = HydraCache::local().build();
        cache
            .put("answer", 42_u64, CacheOptions::new())
            .await
            .unwrap();

        let registry = HydraCacheRegistry::new().with_cache("main", cache);
        let app = HydraCacheActuator::routes_for(registry);

        let health = app
            .clone()
            .oneshot(request("/health"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(health["status"], "UP");
        assert_eq!(health["cache_count"], 1);

        let caches = app
            .clone()
            .oneshot(request("/caches"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(caches["caches"][0], "main");

        let overview = app
            .clone()
            .oneshot(request("/"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(overview["caches"][0]["name"], "main");

        let stats = app
            .oneshot(request("/caches/main/stats"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(stats["loads"], 0);
        assert_eq!(stats["total_requests"], 0);
    }

    #[tokio::test]
    async fn actuator_routes_return_not_found_for_unknown_cache() {
        let app = HydraCacheActuator::routes_for(HydraCacheRegistry::new());

        let diagnostics = app
            .clone()
            .oneshot(request("/caches/missing/diagnostics"))
            .await
            .unwrap();
        let stats = app.oneshot(request("/caches/missing/stats")).await.unwrap();

        assert_eq!(diagnostics.status(), StatusCode::NOT_FOUND);
        assert_eq!(stats.status(), StatusCode::NOT_FOUND);
    }

    fn request(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    async fn json_body(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
}
