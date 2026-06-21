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
//! GET /cluster/staging-health
//! GET /correctness
//! GET /
//! ```

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use hydracache::ClusterStagingHealth;
use hydracache_observability::{
    CacheDiagnosticsSnapshot, CacheStatsSnapshot, HydraCacheOverview, HydraCacheRegistry,
};
use serde::Serialize;

/// Read-only Axum actuator for registered HydraCache instances.
#[derive(Debug, Clone)]
pub struct HydraCacheActuator {
    registry: HydraCacheRegistry,
    correctness: ActuatorCorrectnessSnapshot,
}

impl HydraCacheActuator {
    /// Create a new actuator from a framework-neutral cache registry.
    pub fn new(registry: HydraCacheRegistry) -> Self {
        Self {
            registry,
            correctness: ActuatorCorrectnessSnapshot::default(),
        }
    }

    /// Override the correctness snapshot exposed by `/correctness`.
    pub fn with_correctness_snapshot(mut self, correctness: ActuatorCorrectnessSnapshot) -> Self {
        self.correctness = correctness;
        self
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
        Self::routes_for_state(ActuatorState {
            registry: self.registry,
            correctness: self.correctness,
        })
    }

    /// Build routes directly from a registry.
    pub fn routes_for(registry: HydraCacheRegistry) -> Router {
        Self::new(registry).routes()
    }

    fn routes_for_state(state: ActuatorState) -> Router {
        Router::new()
            .route("/", get(overview))
            .route("/health", get(health))
            .route("/caches", get(caches))
            .route("/caches/{name}/diagnostics", get(cache_diagnostics))
            .route("/caches/{name}/stats", get(cache_stats))
            .route("/cluster/staging-health", get(cluster_staging_health))
            .route("/correctness", get(correctness))
            .with_state(state)
    }
}

#[derive(Debug, Clone)]
struct ActuatorState {
    registry: HydraCacheRegistry,
    correctness: ActuatorCorrectnessSnapshot,
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

/// Named cluster staging health snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NamedClusterStagingHealth {
    /// Registered cache name.
    pub name: String,
    /// Staging health summary for that cache.
    pub health: ClusterStagingHealth,
}

/// Cluster staging health response for actuator endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClusterStagingHealthResponse {
    /// Cluster caches that can expose staging health.
    pub caches: Vec<NamedClusterStagingHealth>,
}

/// Correctness-oriented snapshot for staging/release gates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ActuatorCorrectnessSnapshot {
    /// SQL dependency-lint status.
    pub dependency_lint: DependencyLintSnapshot,
    /// Generated database hook status.
    pub generated_hooks: GeneratedHooksSnapshot,
    /// Durable invalidation outbox status.
    pub outbox: OutboxCorrectnessSnapshot,
    /// Named consistency-mode status.
    pub consistency: ConsistencyCorrectnessSnapshot,
    /// Required dimension-profile status.
    pub dimensions: DimensionProfileSnapshot,
    /// SQLx transaction companion status.
    pub transaction_companion: TransactionCompanionSnapshot,
    /// Reconciliation/drift status.
    pub reconciliation: ReconciliationSnapshot,
}

/// Dependency-lint counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct DependencyLintSnapshot {
    pub warnings: u64,
    pub errors: u64,
    pub inconclusive: u64,
}

/// Generated hook counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct GeneratedHooksSnapshot {
    pub generated_plans: u64,
    pub runtime_rows: u64,
    pub schema_mismatches: u64,
}

/// Outbox correctness counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct OutboxCorrectnessSnapshot {
    pub pending: u64,
    pub dead_lettered: u64,
    pub oldest_pending_age_ms: u64,
}

/// Consistency-mode counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ConsistencyCorrectnessSnapshot {
    pub successes: u64,
    pub timeouts: u64,
    pub degraded: u64,
    pub fail_closed: u64,
}

/// Required dimension-profile counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct DimensionProfileSnapshot {
    pub warnings: u64,
    pub denied: u64,
    pub allowed: u64,
}

/// SQLx transaction companion counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct TransactionCompanionSnapshot {
    pub commits: u64,
    pub rollbacks: u64,
    pub enqueue_failures: u64,
    pub commit_failures: u64,
}

/// Reconciliation counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ReconciliationSnapshot {
    pub clean: u64,
    pub drift: u64,
}

async fn overview(State(state): State<ActuatorState>) -> Json<HydraCacheOverview> {
    Json(state.registry.overview().await)
}

async fn health(State(state): State<ActuatorState>) -> Json<ActuatorHealth> {
    Json(ActuatorHealth {
        status: "UP",
        cache_count: state.registry.len(),
    })
}

async fn caches(State(state): State<ActuatorState>) -> Json<CacheList> {
    Json(CacheList {
        caches: state.registry.cache_names(),
    })
}

async fn cache_diagnostics(
    State(state): State<ActuatorState>,
    Path(name): Path<String>,
) -> Result<Json<CacheDiagnosticsSnapshot>, StatusCode> {
    state
        .registry
        .diagnostics(&name)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn cache_stats(
    State(state): State<ActuatorState>,
    Path(name): Path<String>,
) -> Result<Json<CacheStatsSnapshot>, StatusCode> {
    let diagnostics = state
        .registry
        .diagnostics(&name)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(diagnostics.stats))
}

async fn cluster_staging_health(
    State(state): State<ActuatorState>,
) -> Json<ClusterStagingHealthResponse> {
    Json(ClusterStagingHealthResponse {
        caches: state
            .registry
            .cluster_staging_healths()
            .into_iter()
            .map(|(name, health)| NamedClusterStagingHealth { name, health })
            .collect(),
    })
}

async fn correctness(State(state): State<ActuatorState>) -> Json<ActuatorCorrectnessSnapshot> {
    Json(state.correctness)
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use hydracache::{CacheOptions, HydraCache};
    use hydracache_observability::HydraCacheRegistry;
    use serde_json::Value;
    use tower::ServiceExt;

    use super::{
        ActuatorCorrectnessSnapshot, HydraCacheActuator, ReconciliationSnapshot,
        TransactionCompanionSnapshot,
    };

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
    async fn actuator_exposes_correctness_counters() {
        let registry = HydraCacheRegistry::new();
        let correctness = ActuatorCorrectnessSnapshot {
            transaction_companion: TransactionCompanionSnapshot {
                commits: 2,
                rollbacks: 1,
                enqueue_failures: 0,
                commit_failures: 0,
            },
            reconciliation: ReconciliationSnapshot { clean: 1, drift: 0 },
            ..ActuatorCorrectnessSnapshot::default()
        };
        let app = HydraCacheActuator::new(registry)
            .with_correctness_snapshot(correctness)
            .routes();

        let response = app.oneshot(request("/correctness")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;

        assert_eq!(body["transaction_companion"]["commits"], 2);
        assert_eq!(body["transaction_companion"]["rollbacks"], 1);
        assert_eq!(body["reconciliation"]["clean"], 1);
        assert_eq!(body["dependency_lint"]["warnings"], 0);
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
