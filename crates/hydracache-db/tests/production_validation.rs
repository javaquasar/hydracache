use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::HydraCache;
use hydracache_db::{DbCache, HydraCacheEntity, QueryCachePolicy, RefreshPolicy};
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Notify, RwLock};

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

    async fn begin(&self) -> UserTransaction {
        UserTransaction {
            repository: self.clone(),
            staged_rows: self.rows.read().await.clone(),
        }
    }

    async fn failed_write(
        &self,
        _id: i64,
        _name: impl Into<String>,
    ) -> Result<(), RepositoryError> {
        Err(RepositoryError::Unavailable)
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

    async fn list_users(&self) -> Result<Vec<User>, RepositoryError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_loads.load(Ordering::SeqCst) {
            return Err(RepositoryError::Unavailable);
        }

        let mut users = self.rows.read().await.values().cloned().collect::<Vec<_>>();
        users.sort_by_key(|user| user.id);
        Ok(users)
    }
}

struct UserTransaction {
    repository: UserRepository,
    staged_rows: HashMap<i64, User>,
}

impl UserTransaction {
    fn upsert(&mut self, id: i64, name: impl Into<String>) {
        self.staged_rows.insert(
            id,
            User {
                id,
                name: name.into(),
            },
        );
    }

    fn delete(&mut self, id: i64) {
        self.staged_rows.remove(&id);
    }

    async fn commit(self) {
        *self.repository.rows.write().await = self.staged_rows;
    }

    async fn rollback(self) {}
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

fn users_policy() -> QueryCachePolicy {
    QueryCachePolicy::short_lived()
        .collection("users")
        .with_name("production-list-users")
        .ttl(Duration::from_millis(120))
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

async fn cached_users(
    queries: &DbCache,
    repository: &UserRepository,
) -> hydracache_db::Result<Vec<User>> {
    let repository = repository.clone();
    queries
        .cached_with::<Vec<User>>(users_policy())
        .load(move || async move { repository.list_users().await })
        .await
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
async fn failed_write_does_not_invalidate_cached_entity() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache, "production-db");
    let repository = UserRepository::seeded();

    let first = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(first.name, "Ada");
    assert_eq!(repository.load_calls(), 1);

    let failed = repository.failed_write(42, "Grace").await;
    assert!(matches!(failed, Err(RepositoryError::Unavailable)));

    let still_cached = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(still_cached.name, "Ada");
    assert_eq!(repository.load_calls(), 1);
}

#[tokio::test]
async fn committed_update_invalidates_entity_and_reloads() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let repository = UserRepository::seeded();

    let first = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(first.name, "Ada");

    let mut tx = repository.begin().await;
    tx.upsert(42, "Grace");
    tx.commit().await;

    assert_eq!(cache.invalidate_tag("user:42").await.unwrap(), 1);

    let reloaded = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(reloaded.name, "Grace");
    assert_eq!(repository.load_calls(), 2);
}

#[tokio::test]
async fn committed_insert_invalidates_collection() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let repository = UserRepository::seeded();

    let first = cached_users(&queries, &repository).await.unwrap();
    assert_eq!(
        first.iter().map(|user| user.id).collect::<Vec<_>>(),
        vec![42]
    );

    let mut tx = repository.begin().await;
    tx.upsert(43, "Linus");
    tx.commit().await;

    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 1);

    let reloaded = cached_users(&queries, &repository).await.unwrap();
    assert_eq!(
        reloaded
            .iter()
            .map(|user| (user.id, user.name.as_str()))
            .collect::<Vec<_>>(),
        vec![(42, "Ada"), (43, "Linus")]
    );
    assert_eq!(repository.load_calls(), 2);
}

#[tokio::test]
async fn committed_delete_invalidates_entity_and_collection() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let repository = UserRepository::seeded();

    let cached = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(cached.name, "Ada");
    let users = cached_users(&queries, &repository).await.unwrap();
    assert_eq!(users.len(), 1);

    let mut tx = repository.begin().await;
    tx.delete(42);
    tx.commit().await;

    assert_eq!(cache.invalidate_tag("user:42").await.unwrap(), 1);
    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 1);

    let reloaded = cached_users(&queries, &repository).await.unwrap();
    assert!(reloaded.is_empty());
    assert!(cached_user(&queries, &repository, 42).await.is_err());
}

#[tokio::test]
async fn rollback_keeps_existing_cached_value() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache, "production-db");
    let repository = UserRepository::seeded();

    let first = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(first.name, "Ada");

    let mut tx = repository.begin().await;
    tx.upsert(42, "RolledBack");
    tx.rollback().await;

    let still_cached = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(still_cached.name, "Ada");
    assert_eq!(repository.load_calls(), 1);
}

#[tokio::test]
async fn double_invalidation_after_commit_is_idempotent() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let repository = UserRepository::seeded();

    let first = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(first.name, "Ada");

    let mut tx = repository.begin().await;
    tx.upsert(42, "Grace");
    tx.commit().await;

    assert_eq!(cache.invalidate_tag("user:42").await.unwrap(), 1);
    assert_eq!(cache.invalidate_tag("user:42").await.unwrap(), 0);

    let reloaded = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(reloaded.name, "Grace");
    assert_eq!(repository.load_calls(), 2);
}

#[tokio::test]
async fn db_flow_updates_hit_miss_load_counters() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let repository = UserRepository::seeded();

    let first = cached_user(&queries, &repository, 42).await.unwrap();
    let second = cached_user(&queries, &repository, 42).await.unwrap();

    assert_eq!(first, second);
    assert_eq!(repository.load_calls(), 1);

    let diagnostics = cache.diagnostics().await;
    assert_eq!(diagnostics.stats.misses, 1);
    assert_eq!(diagnostics.stats.hits, 1);
    assert_eq!(diagnostics.stats.loads, 1);
    assert_eq!(diagnostics.total_requests(), 2);
    assert_eq!(diagnostics.hit_ratio(), Some(0.5));
}

#[tokio::test]
async fn db_flow_records_single_flight_joins() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let calls = Arc::new(AtomicUsize::new(0));
    let loader_started = Arc::new(Notify::new());
    let release_loader = Arc::new(Notify::new());

    let first_queries = queries.clone();
    let first_calls = calls.clone();
    let first_started = loader_started.clone();
    let first_release = release_loader.clone();
    let first = tokio::spawn(async move {
        first_queries
            .entity::<User>("user", 42)
            .collection_tag("users")
            .load(move || async move {
                first_calls.fetch_add(1, Ordering::SeqCst);
                first_started.notify_one();
                first_release.notified().await;
                Ok::<_, RepositoryError>(User {
                    id: 42,
                    name: "single-flight".to_owned(),
                })
            })
            .await
    });

    tokio::time::timeout(Duration::from_secs(1), loader_started.notified())
        .await
        .expect("first loader should start before joining request");

    let second_queries = queries.clone();
    let second_calls = calls.clone();
    let second = tokio::spawn(async move {
        second_queries
            .entity::<User>("user", 42)
            .collection_tag("users")
            .load(move || async move {
                second_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, RepositoryError>(User {
                    id: 42,
                    name: "duplicate".to_owned(),
                })
            })
            .await
    });

    tokio::time::timeout(Duration::from_secs(1), async {
        while cache.diagnostics().await.stats.single_flight_joins == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("second request should join the in-flight load");

    release_loader.notify_one();

    let first = first.await.unwrap().unwrap();
    let second = second.await.unwrap().unwrap();

    assert_eq!(first, second);
    assert_eq!(first.name, "single-flight");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache.diagnostics().await.stats.single_flight_joins, 1);
}

#[tokio::test]
async fn db_flow_records_invalidation_count() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let repository = UserRepository::seeded();

    let first = cached_user(&queries, &repository, 42).await.unwrap();
    assert_eq!(first.name, "Ada");

    assert_eq!(cache.invalidate_tag("user:42").await.unwrap(), 1);
    assert_eq!(cache.diagnostics().await.stats.invalidations, 1);
}

#[tokio::test]
async fn db_flow_records_stale_load_discard_when_invalidated_during_load() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();

    let task = tokio::spawn({
        let queries = queries.clone();
        async move {
            queries
                .cached_with::<User>(
                    QueryCachePolicy::read_mostly()
                        .key("race:user:42")
                        .tag("users")
                        .with_name("race-load-user"),
                )
                .load(move || async move {
                    let _ = started_tx.send(());
                    let _ = release_rx.await;
                    Ok::<_, RepositoryError>(User {
                        id: 42,
                        name: "stale".to_owned(),
                    })
                })
                .await
        }
    });

    started_rx.await.unwrap();
    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 0);
    let _ = release_tx.send(());
    let loaded = task.await.unwrap().unwrap();
    assert_eq!(loaded.name, "stale");

    assert!(cache
        .get::<User>("production-db:race:user:42")
        .await
        .unwrap()
        .is_none());
    assert_eq!(cache.diagnostics().await.stats.stale_load_discards, 1);
}

#[tokio::test]
async fn db_flow_records_stale_fallback_when_loader_fails() {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "production-db");
    let repository = UserRepository::seeded();
    let fallback_policy = QueryCachePolicy::read_mostly()
        .for_cache_entity::<User>(42)
        .with_name("short-stale-fallback")
        .ttl(Duration::from_millis(20))
        .refresh_policy(RefreshPolicy::new().stale_on_loader_error(Duration::from_millis(250)));

    let first = cached_user_with_policy(&queries, &repository, 42, fallback_policy.clone())
        .await
        .unwrap();
    assert_eq!(first.name, "Ada");
    let loads_before_failure = cache.diagnostics().await.stats.loads;

    tokio::time::sleep(Duration::from_millis(40)).await;
    repository.fail_loads(true);

    let stale = cached_user_with_policy(&queries, &repository, 42, fallback_policy)
        .await
        .unwrap();
    assert_eq!(stale.name, "Ada");
    assert_eq!(repository.load_calls(), 2);
    assert_eq!(
        cache.diagnostics().await.stats.loads,
        loads_before_failure + 1
    );
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
