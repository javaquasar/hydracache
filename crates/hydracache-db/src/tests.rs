use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheKeyBuilder, HydraCache, TagSet};
use serde::{Deserialize, Serialize};

use crate::{CacheEntity, DbCache, DbCacheError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

impl CacheEntity for User {
    type Id = u64;

    const ENTITY: &'static str = "user";
    const COLLECTION: Option<&'static str> = Some("users");
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AccountUser {
    id: String,
}

impl CacheEntity for AccountUser {
    type Id = &'static str;

    const ENTITY: &'static str = "account:user";
    const COLLECTION: Option<&'static str> = Some("users:active");
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Profile {
    id: u64,
}

impl CacheEntity for Profile {
    type Id = u64;

    const ENTITY: &'static str = "profile";
    const COLLECTION: Option<&'static str> = None;
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
        Err(DbCacheError::MissingKey { operation }) if operation == "db:unnamed"
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
async fn entity_helper_sets_escaped_key_and_entity_tag() {
    let query = adapter().entity::<User>("user:type", "42%beta");

    assert_eq!(query.key_value(), Some("user%3Atype:42%25beta"));
    assert_eq!(
        query.physical_key(),
        Some("db:user%3Atype:42%25beta".to_owned())
    );
    assert_eq!(query.tags_value(), &["user%3Atype:42%25beta".to_owned()]);
}

#[tokio::test]
async fn collection_helper_sets_escaped_key_and_collection_tag() {
    let query = adapter().collection::<User>("users:active");

    assert_eq!(query.key_value(), Some("users%3Aactive"));
    assert_eq!(query.physical_key(), Some("db:users%3Aactive".to_owned()));
    assert_eq!(query.tags_value(), &["users%3Aactive".to_owned()]);
}

#[tokio::test]
async fn for_entity_replaces_key_and_preserves_existing_tags() {
    let query = adapter()
        .cached::<User>()
        .key("old")
        .tag("existing")
        .for_entity("user", 42)
        .collection_tag("users");

    assert_eq!(query.key_value(), Some("user:42"));
    assert_eq!(
        query.tags_value(),
        &[
            "existing".to_owned(),
            "user:42".to_owned(),
            "users".to_owned()
        ]
    );
}

#[tokio::test]
async fn collection_tag_escapes_collection_segment() {
    let query = adapter()
        .entity::<User>("user", 42)
        .collection_tag("users:active");

    assert_eq!(
        query.tags_value(),
        &["user:42".to_owned(), "users%3Aactive".to_owned()]
    );
}

#[tokio::test]
async fn cache_entity_trait_generates_default_metadata() {
    assert_eq!(User::cache_key_for(&42), "user:42");
    assert_eq!(User::entity_tag_for(&42), "user:42");
    assert_eq!(User::collection_tag(), Some("users".to_owned()));
}

#[tokio::test]
async fn cache_entity_helper_sets_key_entity_tag_and_collection_tag() {
    let query = adapter().for_entity::<User>(42);

    assert_eq!(query.key_value(), Some("user:42"));
    assert_eq!(query.physical_key(), Some("db:user:42".to_owned()));
    assert_eq!(
        query.tags_value(),
        &["user:42".to_owned(), "users".to_owned()]
    );
}

#[tokio::test]
async fn cache_entity_helper_escapes_entity_id_and_collection_segments() {
    let query = adapter().for_entity::<AccountUser>("42%beta");

    assert_eq!(query.key_value(), Some("account%3Auser:42%25beta"));
    assert_eq!(
        query.tags_value(),
        &[
            "account%3Auser:42%25beta".to_owned(),
            "users%3Aactive".to_owned()
        ]
    );
}

#[tokio::test]
async fn cache_entity_without_collection_only_adds_entity_tag() {
    let query = adapter().for_entity::<Profile>(7);

    assert_eq!(query.key_value(), Some("profile:7"));
    assert_eq!(query.tags_value(), &["profile:7".to_owned()]);
}

#[tokio::test]
async fn query_for_cache_entity_preserves_existing_tags() {
    let query = adapter()
        .cached::<User>()
        .tag("tenant:7")
        .for_cache_entity(42);

    assert_eq!(query.key_value(), Some("user:42"));
    assert_eq!(
        query.tags_value(),
        &[
            "tenant:7".to_owned(),
            "user:42".to_owned(),
            "users".to_owned()
        ]
    );
}

#[tokio::test]
async fn explicit_key_can_override_generated_entity_key() {
    let query = adapter().entity::<User>("user", 42).key("custom:user:42");

    assert_eq!(query.key_value(), Some("custom:user:42"));
    assert_eq!(query.physical_key(), Some("db:custom:user:42".to_owned()));
    assert_eq!(query.tags_value(), &["user:42".to_owned()]);
}

#[tokio::test]
async fn entity_helper_caches_loaded_value_and_uses_generated_tag() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();

    let first = cache
        .entity::<User>("user", 1)
        .collection_tag("users")
        .fetch_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(1))
            }
        })
        .await
        .unwrap();

    let cached = cache
        .entity::<User>("user", 1)
        .collection_tag("users")
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
    assert_eq!(cached, user(1));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    assert_eq!(cache.cache().invalidate_tag("user:1").await.unwrap(), 1);

    let reloaded = cache
        .entity::<User>("user", 1)
        .fetch_with(|| async { Ok::<_, LoadError>(user(2)) })
        .await
        .unwrap();

    assert_eq!(reloaded, user(2));
}

#[tokio::test]
async fn collection_helper_caches_adapter_chosen_output_type() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();

    let first: Vec<User> = cache
        .collection::<User>("users")
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(vec![user(1)])
            }
        })
        .await
        .unwrap();

    let cached: Vec<User> = cache
        .collection::<User>("users")
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(vec![user(2)])
            }
        })
        .await
        .unwrap();

    assert_eq!(first, vec![user(1)]);
    assert_eq!(cached, vec![user(1)]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    assert_eq!(cache.cache().invalidate_tag("users").await.unwrap(), 1);
}

#[tokio::test]
async fn cache_entity_helper_caches_and_invalidates_by_collection_tag() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();

    let first = cache
        .for_entity::<User>(1)
        .fetch_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(1))
            }
        })
        .await
        .unwrap();

    let cached = cache
        .for_entity::<User>(1)
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
    assert_eq!(cached, user(1));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    assert_eq!(cache.cache().invalidate_tag("users").await.unwrap(), 1);

    let reloaded = cache
        .for_entity::<User>(1)
        .fetch_with(|| async { Ok::<_, LoadError>(user(2)) })
        .await
        .unwrap();

    assert_eq!(reloaded, user(2));
}

#[tokio::test]
async fn query_builder_with_name_replaces_diagnostic_label() {
    let query = adapter()
        .cached::<User>()
        .with_name("load-user")
        .key("user:1");

    assert_eq!(adapter().namespace(), "db");
    assert_eq!(query.name(), Some("load-user"));
}

#[tokio::test]
async fn adapter_and_query_derived_impls_are_usable() {
    let cache = adapter();
    let cache_clone = cache.clone();
    let query = cache.cached::<User>().key("user:1").clone();

    assert_eq!(cache_clone.namespace(), "db");
    assert!(format!("{cache:?}").contains("DbCache"));
    assert!(format!("{query:?}").contains("DbQuery"));
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
async fn per_query_ttl_expires_cached_query_result() {
    let cache = adapter();

    cache
        .cached::<User>()
        .key("user:ttl")
        .ttl(Duration::from_millis(20))
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(40)).await;

    let reloaded = cache
        .cached::<User>()
        .key("user:ttl")
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

#[tokio::test]
async fn missing_key_error_uses_key_context_for_unnamed_queries() {
    let result = DbCache::new(HydraCache::local().build(), "")
        .cached::<User>()
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation }) if operation == "unnamed"
    ));

    let result = adapter()
        .cached::<User>()
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation }) if operation == "db:unnamed"
    ));

    let result = DbCache::new(HydraCache::local().build(), "db")
        .cached::<User>()
        .with_name("")
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation }) if operation.is_empty()
    ));
}

#[tokio::test]
async fn fetch_value_with_caches_adapter_chosen_output_type() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();

    let first: Option<User> = cache
        .cached::<User>()
        .key("maybe-user:1")
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(Some(user(1)))
            }
        })
        .await
        .unwrap();

    let second: Option<User> = cache
        .cached::<User>()
        .key("maybe-user:1")
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(Some(user(2)))
            }
        })
        .await
        .unwrap();

    assert_eq!(first, Some(user(1)));
    assert_eq!(second, Some(user(1)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fetch_value_with_caches_empty_vectors() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();

    let first: Vec<User> = cache
        .cached::<User>()
        .key("users:none")
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(Vec::new())
            }
        })
        .await
        .unwrap();

    let second: Vec<User> = cache
        .cached::<User>()
        .key("users:none")
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(vec![user(2)])
            }
        })
        .await
        .unwrap();

    assert!(first.is_empty());
    assert!(second.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fetch_value_with_requires_explicit_key() {
    let result: crate::Result<Option<User>> = adapter()
        .cached::<User>()
        .fetch_value_with(|| async { Ok::<_, LoadError>(None) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation }) if operation == "db:unnamed"
    ));
}

fn user(id: u64) -> User {
    User {
        id,
        name: format!("user-{id}"),
    }
}
