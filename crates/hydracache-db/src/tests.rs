use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheKeyBuilder, HydraCache, TagSet};
use serde::{Deserialize, Serialize};

use crate::{
    CacheEntity, DbAdapterKind, DbCache, DbCacheError, DbResultShape, HydraCacheEntity,
    PreparedQueryPolicy, QueryCachePolicy, RefreshPolicy,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = u64)]
struct User {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(
    entity = "account:user",
    collection = "users:active",
    id = &'static str
)]
struct AccountUser {
    id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "profile", id = u64)]
struct Profile {
    id: u64,
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
        Err(DbCacheError::MissingKey { operation, .. }) if operation == "db:unnamed"
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
async fn query_cache_policy_exposes_reusable_metadata() {
    let policy = QueryCachePolicy::named("load-user")
        .key_builder(CacheKeyBuilder::new().tenant(7).entity("user", 42))
        .tag("users")
        .tags(["user:42", "tenant:7"])
        .ttl(Duration::from_secs(30));

    assert_eq!(policy.name(), Some("load-user"));
    assert_eq!(policy.key_value(), Some("tenant:7:user:42"));
    assert_eq!(
        policy.tags_value(),
        &[
            "users".to_owned(),
            "user:42".to_owned(),
            "tenant:7".to_owned()
        ]
    );
    assert_eq!(policy.ttl_value(), Some(Duration::from_secs(30)));
}

#[tokio::test]
async fn query_cache_policy_presets_encode_common_ttl_intent() {
    assert_eq!(
        QueryCachePolicy::short_lived().ttl_value(),
        Some(Duration::from_secs(30))
    );
    assert_eq!(
        QueryCachePolicy::read_mostly().ttl_value(),
        Some(Duration::from_secs(300))
    );
    assert_eq!(
        QueryCachePolicy::per_entity().ttl_value(),
        Some(Duration::from_secs(300))
    );
    assert_eq!(
        QueryCachePolicy::no_ttl_explicit_invalidation().ttl_value(),
        None
    );
    assert_eq!(
        QueryCachePolicy::negative_cache().ttl_value(),
        Some(Duration::from_secs(30))
    );
}

#[tokio::test]
async fn query_cache_policy_presets_compose_with_entity_and_collection_metadata() {
    let entity = QueryCachePolicy::per_entity().for_cache_entity::<User>(42);

    assert_eq!(entity.key_value(), Some("user:42"));
    assert_eq!(
        entity.tags_value(),
        &["user:42".to_owned(), "users".to_owned()]
    );
    assert_eq!(entity.ttl_value(), Some(Duration::from_secs(300)));

    let collection = QueryCachePolicy::read_mostly().collection("users:active");

    assert_eq!(collection.key_value(), Some("users%3Aactive"));
    assert_eq!(collection.tags_value(), &["users%3Aactive".to_owned()]);
    assert_eq!(collection.ttl_value(), Some(Duration::from_secs(300)));
}

#[tokio::test]
async fn query_cache_policy_stores_refresh_policy_metadata() {
    let refresh = RefreshPolicy::new()
        .refresh_ahead(Duration::from_secs(10))
        .stale_while_revalidate(Duration::from_secs(60));
    let policy = QueryCachePolicy::read_mostly()
        .for_cache_entity::<User>(42)
        .refresh_policy(refresh);

    assert_eq!(policy.refresh_policy_value(), Some(refresh));
}

#[tokio::test]
async fn prepared_query_policy_preserves_refresh_policy_when_binding_id() {
    let refresh = RefreshPolicy::new().stale_on_loader_error(Duration::from_secs(120));
    let prepared = PreparedQueryPolicy::per_entity()
        .cache_entity::<User>()
        .refresh_policy(refresh);

    assert_eq!(prepared.refresh_policy_value(), Some(refresh));

    let query = adapter().prepare::<User>(prepared).for_id(42);
    assert_eq!(query.refresh_policy_value(), Some(refresh));
}

#[tokio::test]
async fn db_query_refresh_policy_serves_stale_and_refreshes_in_background() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();
    let base_policy = QueryCachePolicy::new()
        .key("user:refresh")
        .tag("users")
        .refresh_policy(RefreshPolicy::new().stale_while_revalidate(Duration::from_millis(200)));
    let initial_policy = base_policy.clone().ttl(Duration::from_millis(20));
    let refresh_policy = base_policy.ttl(Duration::from_millis(500));

    let first = cache
        .cached_with::<User>(initial_policy)
        .load({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(1))
            }
        })
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(45)).await;

    let stale = cache
        .cached_with::<User>(refresh_policy.clone())
        .load({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(2))
            }
        })
        .await
        .unwrap();

    assert_eq!(first, user(1));
    assert_eq!(stale, user(1));

    tokio::time::sleep(Duration::from_millis(80)).await;

    let refreshed = cache
        .cached_with::<User>(refresh_policy)
        .load({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(3))
            }
        })
        .await
        .unwrap();

    assert_eq!(refreshed, user(2));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn db_query_stale_on_loader_error_uses_bounded_fallback() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();
    let policy = QueryCachePolicy::new()
        .key("user:stale-if-error")
        .ttl(Duration::from_millis(20))
        .refresh_policy(RefreshPolicy::new().stale_on_loader_error(Duration::from_millis(200)));

    let first = cache
        .cached_with::<User>(policy.clone())
        .load({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(1))
            }
        })
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(45)).await;

    let fallback = cache
        .cached_with::<User>(policy)
        .load({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err::<User, _>(LoadError)
            }
        })
        .await
        .unwrap();

    assert_eq!(first, user(1));
    assert_eq!(fallback, user(1));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn negative_cache_preset_caches_absence_briefly() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();
    let policy = QueryCachePolicy::negative_cache()
        .key("user:not-found:404")
        .tag("users");

    let first: Option<User> = cache
        .cached_with::<User>(policy.clone())
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(None)
            }
        })
        .await
        .unwrap();

    let cached: Option<User> = cache
        .cached_with::<User>(policy)
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(Some(user(404)))
            }
        })
        .await
        .unwrap();

    assert_eq!(first, None);
    assert_eq!(cached, None);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cached_with_applies_reusable_query_cache_policy() {
    let policy = QueryCachePolicy::named("load-user")
        .for_cache_entity::<User>(42)
        .ttl(Duration::from_secs(30));

    let first = adapter().cached_with::<User>(policy.clone());
    let second = adapter().cached_with::<User>(policy);

    assert_eq!(first.name(), Some("load-user"));
    assert_eq!(first.key_value(), Some("user:42"));
    assert_eq!(
        first.tags_value(),
        &["user:42".to_owned(), "users".to_owned()]
    );
    assert_eq!(first.ttl_value(), Some(Duration::from_secs(30)));
    assert_eq!(second.physical_key(), Some("db:user:42".to_owned()));
}

#[tokio::test]
async fn prepared_query_policy_descriptor_binds_entity_ids() {
    let prepared = adapter().prepare::<User>(
        PreparedQueryPolicy::for_cache_entity::<User>()
            .with_name("load-user")
            .ttl(Duration::from_secs(30)),
    );

    assert_eq!(prepared.namespace(), "db");
    assert_eq!(prepared.name(), Some("load-user"));
    assert!(prepared.requires_id());
    assert_eq!(prepared.entity_key_prefix(), Some("user"));
    assert_eq!(prepared.tags_value(), &["users".to_owned()]);
    assert_eq!(prepared.ttl_value(), Some(Duration::from_secs(30)));

    let query = prepared.for_id(42);
    assert_eq!(query.name(), Some("load-user"));
    assert_eq!(query.key_value(), Some("user:42"));
    assert_eq!(query.physical_key(), Some("db:user:42".to_owned()));
    assert_eq!(
        query.tags_value(),
        &["users".to_owned(), "user:42".to_owned()]
    );
    assert_eq!(query.ttl_value(), Some(Duration::from_secs(30)));
}

#[tokio::test]
async fn prepared_query_load_id_caches_and_invalidates_entity_result() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();
    let prepared = cache
        .prepare::<User>(PreparedQueryPolicy::for_cache_entity::<User>().with_name("load-user"));

    let first = prepared
        .load_id(1, {
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(user(1))
            }
        })
        .await
        .unwrap();

    let cached = prepared
        .load_id(1, {
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

    let reloaded = prepared
        .load_id(1, || async { Ok::<_, LoadError>(user(2)) })
        .await
        .unwrap();

    assert_eq!(reloaded, user(2));
}

#[tokio::test]
async fn prepared_static_query_loads_collection_values() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();
    let prepared = cache.prepare::<User>(
        PreparedQueryPolicy::named("list-users")
            .collection("users:active")
            .ttl(Duration::from_secs(30)),
    );

    assert!(!prepared.requires_id());
    assert_eq!(prepared.static_key_value(), Some("users%3Aactive"));

    let first: Vec<User> = prepared
        .fetch_value_with({
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(vec![user(1)])
            }
        })
        .await
        .unwrap();

    let cached: Vec<User> = prepared
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
    assert_eq!(
        prepared
            .cache()
            .invalidate_tag("users%3Aactive")
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn prepared_entity_helper_uses_cache_entity_metadata() {
    let prepared = adapter().prepare_entity::<User>();

    assert!(prepared.requires_id());
    assert_eq!(prepared.entity_key_prefix(), Some("user"));
    assert_eq!(prepared.tags_value(), &["users".to_owned()]);

    let query = prepared.for_id(7);
    assert_eq!(query.physical_key(), Some("db:user:7".to_owned()));
    assert_eq!(
        query.tags_value(),
        &["users".to_owned(), "user:7".to_owned()]
    );
}

#[tokio::test]
async fn prepared_entity_without_bound_id_reports_missing_key() {
    let result = adapter()
        .prepare::<User>(PreparedQueryPolicy::for_cache_entity::<User>().with_name("load-user"))
        .load(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation, .. }) if operation == "load-user"
    ));
}

#[tokio::test]
async fn with_policy_replaces_existing_descriptor_policy() {
    let policy = QueryCachePolicy::new().key("new").tag("new-tag");
    let query = adapter()
        .cached::<User>()
        .key("old")
        .tag("old-tag")
        .with_policy(policy);

    assert_eq!(query.key_value(), Some("new"));
    assert_eq!(query.tags_value(), &["new-tag".to_owned()]);
    assert_eq!(query.cache_policy().key_value(), Some("new"));
}

#[tokio::test]
async fn query_cache_policy_collection_sets_key_and_tag() {
    let policy = QueryCachePolicy::new().collection("users:active");
    let query = adapter().cached_with::<Vec<User>>(policy);

    assert_eq!(query.key_value(), Some("users%3Aactive"));
    assert_eq!(query.physical_key(), Some("db:users%3Aactive".to_owned()));
    assert_eq!(query.tags_value(), &["users%3Aactive".to_owned()]);
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
async fn descriptor_collection_method_sets_escaped_key_and_collection_tag() {
    let query = adapter().cached::<User>().collection("users:active");

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
async fn load_alias_caches_repository_result() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cache = adapter();

    let first = cache
        .for_entity::<User>(1)
        .load({
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
        .load({
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
        Err(DbCacheError::MissingKey { operation, .. }) if operation == "load-profile"
    ));
}

#[tokio::test]
async fn missing_key_error_includes_operation_adapter_and_shape_context() {
    let result: crate::Result<Option<User>> = adapter()
        .named::<User>("load-profile")
        .adapter_context(DbAdapterKind::Sqlx, DbResultShape::Optional)
        .fetch_value_with(|| async { Ok::<_, LoadError>(None) })
        .await;

    match result.expect_err("query without key should fail") {
        DbCacheError::MissingKey {
            operation,
            adapter,
            namespace,
            result_shape,
        } => {
            assert_eq!(operation, "load-profile");
            assert_eq!(adapter, DbAdapterKind::Sqlx);
            assert_eq!(namespace, "db");
            assert_eq!(result_shape, DbResultShape::Optional);
        }
        other => panic!("expected missing-key error, got {other:?}"),
    }
}

#[tokio::test]
async fn adapter_error_display_contains_operation_context() {
    let result = adapter()
        .named::<User>("load-user")
        .key("user:1")
        .adapter_context(DbAdapterKind::Generic, DbResultShape::One)
        .fetch_with(|| async { Err::<User, _>(LoadError) })
        .await;

    match result.expect_err("loader error should include database cache context") {
        DbCacheError::Operation {
            operation,
            context,
            source,
        } => {
            assert_eq!(operation, "load-user");
            assert_eq!(context.adapter, DbAdapterKind::Generic);
            assert_eq!(context.namespace, "db");
            assert_eq!(context.physical_key.as_deref(), Some("db:user:1"));
            assert_eq!(context.result_shape, DbResultShape::One);
            assert!(matches!(*source, hydracache::CacheError::Loader(_)));
        }
        other => panic!("expected contextual operation error, got {other:?}"),
    }

    let error = adapter()
        .named::<User>("load-user")
        .key("user:1")
        .adapter_context(DbAdapterKind::Generic, DbResultShape::One)
        .fetch_with(|| async { Err::<User, _>(LoadError) })
        .await
        .expect_err("loader error should include database cache context");
    let message = error.to_string();

    assert!(message.contains("database cached operation `load-user` failed"));
    assert!(message.contains("adapter=generic"));
    assert!(message.contains("namespace=db"));
    assert!(message.contains("key=db:user:1"));
    assert!(message.contains("result_shape=one"));
    assert!(message.contains("cache loader error: load failed"));
}

#[test]
fn db_cache_error_stays_small_enough_for_result_returns() {
    assert!(
        std::mem::size_of::<DbCacheError>() <= 80,
        "DbCacheError should box large context/source fields"
    );
}

#[tokio::test]
async fn missing_key_error_uses_key_context_for_unnamed_queries() {
    let result = DbCache::new(HydraCache::local().build(), "")
        .cached::<User>()
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation, .. }) if operation == "unnamed"
    ));

    let result = adapter()
        .cached::<User>()
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation, .. }) if operation == "db:unnamed"
    ));

    let result = DbCache::new(HydraCache::local().build(), "db")
        .cached::<User>()
        .with_name("")
        .fetch_with(|| async { Ok::<_, LoadError>(user(1)) })
        .await;

    assert!(matches!(
        result,
        Err(DbCacheError::MissingKey { operation, .. }) if operation.is_empty()
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
        Err(DbCacheError::MissingKey { operation, .. }) if operation == "db:unnamed"
    ));
}

fn user(id: u64) -> User {
    User {
        id,
        name: format!("user-{id}"),
    }
}
