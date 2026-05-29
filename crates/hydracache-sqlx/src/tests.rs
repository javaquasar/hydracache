use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheKeyBuilder, HydraCache, TagSet};
use serde::{Deserialize, Serialize};

use crate::{SqlxCache, SqlxCacheError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

#[derive(Debug)]
struct LoadError;

impl std::fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("load failed")
    }
}

impl std::error::Error for LoadError {}

fn adapter() -> SqlxCache {
    SqlxCache::new(HydraCache::local().build(), "sql")
}

#[tokio::test]
async fn fetch_with_requires_explicit_key() {
    let result = adapter()
        .query_as::<User>("select id, name from users")
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(result, Err(SqlxCacheError::MissingKey { .. })));
}

#[tokio::test]
async fn query_builder_exposes_metadata() {
    let query = adapter()
        .query_as::<User>("select id, name from users where id = $1")
        .key_builder(CacheKeyBuilder::new().tenant(7).entity("user", 42))
        .tag("users")
        .tags(["user:42", "tenant:7"])
        .ttl(Duration::from_secs(30));

    assert_eq!(query.namespace(), "sql");
    assert_eq!(query.sql(), "select id, name from users where id = $1");
    assert_eq!(query.key_value(), Some("tenant:7:user:42"));
    assert_eq!(
        query.physical_key(),
        Some("sql:tenant:7:user:42".to_owned())
    );
    assert_eq!(
        query.tags_value(),
        &[
            "users".to_owned(),
            "user:42".to_owned(),
            "tenant:7".to_owned()
        ]
    );
    assert_eq!(query.ttl_value(), Some(Duration::from_secs(30)));
}

#[tokio::test]
async fn fetch_with_caches_loaded_value() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();

    let first = cache
        .query_as::<User>("select id, name from users where id = $1")
        .key("user:1")
        .fetch_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(1))
            }
        })
        .await
        .unwrap();

    let second = cache
        .query_as::<User>("select id, name from users where id = $1")
        .key("user:1")
        .fetch_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(2))
            }
        })
        .await
        .unwrap();

    assert_eq!(first, user(1));
    assert_eq!(second, user(1));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn tag_invalidation_removes_cached_query_result() {
    let cache = adapter();

    cache
        .query_as::<User>("select id, name from users where id = $1")
        .key("user:1")
        .tag_set(TagSet::new().tag("users").entity("user", 1))
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await
        .unwrap();

    assert_eq!(cache.cache().invalidate_tag("user:1").await.unwrap(), 1);

    let reloaded = cache
        .query_as::<User>("select id, name from users where id = $1")
        .key("user:1")
        .fetch_with(|| async { Ok::<_, LoadError>(user(2)) })
        .await
        .unwrap();

    assert_eq!(reloaded, user(2));
}

#[tokio::test]
async fn empty_namespace_uses_logical_key_as_physical_key() {
    let query = SqlxCache::new(HydraCache::local().build(), "")
        .query_as::<User>("select 1")
        .key("one");

    assert_eq!(query.physical_key(), Some("one".to_owned()));
}

fn user(id: u64) -> User {
    User {
        id,
        name: format!("user-{id}"),
    }
}
