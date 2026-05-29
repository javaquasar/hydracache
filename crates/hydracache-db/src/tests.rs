use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheKeyBuilder, HydraCache, TagSet};
use serde::{Deserialize, Serialize};

use crate::{DbCache, DbCacheError};

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

fn adapter() -> DbCache {
    DbCache::new(HydraCache::local().build(), "db")
}

#[tokio::test]
async fn fetch_with_requires_explicit_key() {
    let result = adapter()
        .cached::<User>()
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation }) if operation == "unnamed"
    ));
}

#[tokio::test]
async fn query_builder_exposes_metadata() {
    let query = adapter()
        .named::<User>("load-user")
        .key_builder(CacheKeyBuilder::new().tenant(7).entity("user", 42))
        .tag("users")
        .tags(["user:42", "tenant:7"])
        .ttl(Duration::from_secs(30));

    assert_eq!(query.namespace(), "db");
    assert_eq!(query.name(), Some("load-user"));
    assert_eq!(query.key_value(), Some("tenant:7:user:42"));
    assert_eq!(query.physical_key(), Some("db:tenant:7:user:42".to_owned()));
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
        .cached::<User>()
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
        .cached::<User>()
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
        .cached::<User>()
        .key("user:1")
        .tag_set(TagSet::new().tag("users").entity("user", 1))
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await
        .unwrap();

    assert_eq!(cache.cache().invalidate_tag("user:1").await.unwrap(), 1);

    let reloaded = cache
        .cached::<User>()
        .key("user:1")
        .fetch_with(|| async { Ok::<_, LoadError>(user(2)) })
        .await
        .unwrap();

    assert_eq!(reloaded, user(2));
}

#[tokio::test]
async fn empty_namespace_uses_logical_key_as_physical_key() {
    let query = DbCache::new(HydraCache::local().build(), "")
        .cached::<User>()
        .key("one");

    assert_eq!(query.physical_key(), Some("one".to_owned()));
}

#[tokio::test]
async fn query_as_keeps_sql_text_as_diagnostic_name() {
    let query = adapter()
        .query_as::<User>("select id from users")
        .key("users");

    assert_eq!(query.name(), Some("select id from users"));
}

#[tokio::test]
async fn missing_key_error_uses_available_context() {
    let result = adapter()
        .named::<User>("load-profile")
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation }) if operation == "load-profile"
    ));
}

fn user(id: u64) -> User {
    User {
        id,
        name: format!("user-{id}"),
    }
}
