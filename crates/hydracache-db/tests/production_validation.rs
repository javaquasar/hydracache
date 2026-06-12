use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::HydraCache;
use hydracache_db::{DbCache, HydraCacheEntity, QueryCachePolicy, RefreshPolicy};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User {
    id: i64,
    name: String,
}

#[derive(Debug, Clone)]
struct UserRepository {
    rows: Arc<RwLock<HashMap<i64, User>>>,
    calls: Arc<AtomicUsize>,
    fail_loads: Arc<AtomicBool>,
}

impl UserRepository {
    fn seeded() -> Self {
        let mut rows = HashMap::new();
        rows.insert(
            42,
            User {
                id: 42,
                name: "Ada".to_owned(),
            },
        );

        Self {
            rows: Arc::new(RwLock::new(rows)),
            calls: Arc::new(AtomicUsize::new(0)),
            fail_loads: Arc::new(AtomicBool::new(false)),
        }
    }

    async fn upsert(&self, id: i64, name: impl Into<String>) {
        self.rows.write().await.insert(
            id,
            User {
                id,
                name: name.into(),
            },
        );
    }

    fn fail_loads(&self, fail: bool) {
        self.fail_loads.store(fail, Ordering::SeqCst);
    }

    fn load_calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    async fn load_user(&self, id: i64) -> Result<User, RepositoryError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_loads.load(Ordering::SeqCst) {
            return Err(RepositoryError::Unavailable);
        }

        self.rows
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or(RepositoryError::NotFound(id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepositoryError {
    NotFound(i64),
    Unavailable,
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(formatter, "user {id} was not found"),
            Self::Unavailable => formatter.write_str("repository is temporarily unavailable"),
        }
    }
}

impl Error for RepositoryError {}

fn user_policy(id: i64) -> QueryCachePolicy {
    QueryCachePolicy::read_mostly()
        .for_cache_entity::<User>(id)
        .with_name("production-load-user")
        .ttl(Duration::from_millis(120))
        .refresh_policy(
            RefreshPolicy::new()
                .refresh_ahead(Duration::from_millis(70))
                .stale_while_revalidate(Duration::from_millis(250))
                .stale_on_loader_error(Duration::from_millis(250)),
        )
}

fn stale_on_error_policy(id: i64) -> QueryCachePolicy {
    QueryCachePolicy::read_mostly()
        .for_cache_entity::<User>(id)
        .with_name("production-load-user-with-error-fallback")
        .ttl(Duration::from_millis(120))
        .refresh_policy(RefreshPolicy::new().stale_on_loader_error(Duration::from_millis(250)))
}

async fn cached_user_with_policy(
    queries: &DbCache,
    repository: &UserRepository,
    id: i64,
    policy: QueryCachePolicy,
) -> hydracache_db::Result<User> {
    let repository = repository.clone();
    queries
        .cached_with::<User>(policy)
        .load(move || async move { repository.load_user(id).await })
        .await
}

async fn cached_user(
    queries: &DbCache,
    repository: &UserRepository,
    id: i64,
) -> hydracache_db::Result<User> {
    cached_user_with_policy(queries, repository, id, user_policy(id)).await
}

async fn wait_for_cached_name(cache: &HydraCache, key: &str, expected: &str) {
    for _ in 0..30 {
        if let Some(user) = cache.get::<User>(key).await.unwrap() {
            if user.name == expected {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("cached value `{key}` did not become `{expected}`");
}

#[tokio::test]
async fn production_style_query_cache_flow_validates_hits_invalidation_refresh_and_diagnostics() {
    let cache = HydraCache::local().max_capacity(1_000).build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let repository = UserRepository::seeded();

    let first = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(first.name, "Ada");
    assert_eq!(repository.load_calls(), 1);

    repository.upsert(42, "Updated").await;
    let cached = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(cached.name, "Ada");
    assert_eq!(repository.load_calls(), 1);

    let removed = cache.invalidate_tag("user:42").await.unwrap();
    assert_eq!(removed, 1);

    let reloaded = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(reloaded.name, "Updated");
    assert_eq!(repository.load_calls(), 2);

    tokio::time::sleep(Duration::from_millis(80)).await;
    repository.upsert(42, "RefreshAhead").await;

    let refresh_ahead_hit = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(refresh_ahead_hit.name, "Updated");
    wait_for_cached_name(&cache, "production-db:user:42", "RefreshAhead").await;
    assert!(repository.load_calls() >= 3);

    tokio::time::sleep(Duration::from_millis(130)).await;
    repository.fail_loads(true);

    let calls_before_error = repository.load_calls();
    let stale_fallback =
        cached_user_with_policy(&queries, &repository, 42, stale_on_error_policy(42))
            .await
            .unwrap();
    assert_eq!(stale_fallback.name, "RefreshAhead");
    assert_eq!(repository.load_calls(), calls_before_error + 1);

    let diagnostics = cache.diagnostics().await;
    assert!(diagnostics.stats.hits > 0);
    assert!(diagnostics.stats.misses > 0);
    assert!(diagnostics.stats.loads >= 3);
    assert!(diagnostics.total_requests() > 0);
    assert!(diagnostics.hit_ratio().is_some());
}
