use std::time::Duration;

use hydracache::{CacheKeyBuilder, HydraCache};
use hydracache_db::{DbCache, HydraCacheEntity, QueryCachePolicy, RefreshPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User {
    id: i64,
    name: String,
}

#[test]
fn entity_by_id_policy_uses_entity_and_collection_tags() {
    let policy = QueryCachePolicy::per_entity()
        .for_cache_entity::<User>(42)
        .with_name("load-user");

    assert_eq!(policy.name(), Some("load-user"));
    assert_eq!(policy.key_value(), Some("user:42"));
    assert_eq!(
        policy.tags_value(),
        &["user:42".to_owned(), "users".to_owned()]
    );
    assert_eq!(policy.ttl_value(), Some(Duration::from_secs(300)));
}

#[test]
fn read_mostly_catalog_policy_uses_refresh_ahead_and_collection_tags() {
    let refresh = RefreshPolicy::new()
        .refresh_ahead(Duration::from_secs(30))
        .stale_while_revalidate(Duration::from_secs(300));
    let policy = QueryCachePolicy::read_mostly()
        .for_entity("product", 7)
        .collection_tag("products")
        .refresh_policy(refresh);

    assert_eq!(policy.key_value(), Some("product:7"));
    assert_eq!(
        policy.tags_value(),
        &["product:7".to_owned(), "products".to_owned()]
    );
    assert_eq!(policy.refresh_policy_value(), Some(refresh));
}

#[test]
fn short_lived_search_policy_includes_tenant_query_and_collection_tags() {
    let key = CacheKeyBuilder::new()
        .tenant(7)
        .segment("search")
        .segment("users")
        .segment("status=active")
        .segment("page=1")
        .build_string();

    let policy = QueryCachePolicy::short_lived()
        .key(key)
        .collection_tag("users")
        .tag("tenant:7");

    assert_eq!(
        policy.key_value(),
        Some("tenant:7:search:users:status=active:page=1")
    );
    assert_eq!(
        policy.tags_value(),
        &["users".to_owned(), "tenant:7".to_owned()]
    );
    assert_eq!(policy.ttl_value(), Some(Duration::from_secs(30)));
}

#[test]
fn permission_policy_key_contains_security_dimensions() {
    let key = CacheKeyBuilder::new()
        .tenant(7)
        .segment("permission")
        .segment("principal=42")
        .segment("resource=document:99")
        .segment("action=read")
        .build_string();

    let policy = QueryCachePolicy::short_lived()
        .key(key)
        .tag("principal:42")
        .tag("document:99");

    assert_eq!(
        policy.key_value(),
        Some("tenant:7:permission:principal=42:resource=document%3A99:action=read")
    );
    assert_eq!(
        policy.tags_value(),
        &["principal:42".to_owned(), "document:99".to_owned()]
    );
}

#[test]
fn tenant_dimension_changes_physical_key() {
    let queries = DbCache::new(HydraCache::local().build(), "db");
    let tenant_7 = QueryCachePolicy::short_lived()
        .key_builder(CacheKeyBuilder::new().tenant(7).entity("user", 42))
        .tag("tenant:7");
    let tenant_8 = QueryCachePolicy::short_lived()
        .key_builder(CacheKeyBuilder::new().tenant(8).entity("user", 42))
        .tag("tenant:8");

    let key_7 = queries.cached_with::<User>(tenant_7).physical_key();
    let key_8 = queries.cached_with::<User>(tenant_8).physical_key();

    assert_eq!(key_7, Some("db:tenant:7:user:42".to_owned()));
    assert_eq!(key_8, Some("db:tenant:8:user:42".to_owned()));
    assert_ne!(key_7, key_8);
}

#[test]
fn permission_dimension_changes_physical_key() {
    let queries = DbCache::new(HydraCache::local().build(), "db");
    let version_3 = QueryCachePolicy::short_lived().key_builder(
        CacheKeyBuilder::new()
            .tenant(7)
            .segment("permission")
            .segment("principal=42")
            .segment("policy_version=3")
            .segment("resource=document:99")
            .segment("action=read"),
    );
    let version_4 = QueryCachePolicy::short_lived().key_builder(
        CacheKeyBuilder::new()
            .tenant(7)
            .segment("permission")
            .segment("principal=42")
            .segment("policy_version=4")
            .segment("resource=document:99")
            .segment("action=read"),
    );

    let key_3 = queries.cached_with::<bool>(version_3).physical_key();
    let key_4 = queries.cached_with::<bool>(version_4).physical_key();

    assert_eq!(
        key_3,
        Some(
            "db:tenant:7:permission:principal=42:policy_version=3:resource=document%3A99:action=read"
                .to_owned()
        )
    );
    assert_ne!(key_3, key_4);
}

#[test]
fn filters_are_escaped_as_key_segments() {
    let key = CacheKeyBuilder::new()
        .tenant(7)
        .segment("users")
        .segment("status:active")
        .segment("email_like=100%")
        .build_string();

    let policy = QueryCachePolicy::short_lived()
        .key(key)
        .collection_tag("users");

    assert_eq!(
        policy.key_value(),
        Some("tenant:7:users:status%3Aactive:email_like=100%25")
    );
}

#[test]
fn pagination_and_sort_are_part_of_list_key() {
    let key = CacheKeyBuilder::new()
        .tenant(7)
        .segment("users")
        .segment("status=active")
        .segment("page=2")
        .segment("limit=50")
        .segment("sort=name_desc")
        .build_string();

    let policy = QueryCachePolicy::short_lived()
        .key(key)
        .collection_tag("users");

    assert_eq!(
        policy.key_value(),
        Some("tenant:7:users:status=active:page=2:limit=50:sort=name_desc")
    );
}

#[test]
fn collection_tag_does_not_replace_unique_key() {
    let key = CacheKeyBuilder::new()
        .tenant(7)
        .segment("users")
        .segment("status=active")
        .segment("page=1")
        .build_string();
    let policy = QueryCachePolicy::short_lived()
        .key(key)
        .collection_tag("users");

    assert_eq!(
        policy.key_value(),
        Some("tenant:7:users:status=active:page=1")
    );
    assert_eq!(policy.tags_value(), &["users".to_owned()]);
    assert_ne!(policy.key_value(), Some("users"));
}

#[test]
fn unsafe_key_examples_are_documented_not_runtime_enforced() {
    let unsafe_policy = QueryCachePolicy::short_lived()
        .key("users:active")
        .collection_tag("users");
    let safe_policy = QueryCachePolicy::short_lived()
        .key_builder(
            CacheKeyBuilder::new()
                .tenant(7)
                .segment("users")
                .segment("status=active")
                .segment("page=1")
                .segment("sort=name_asc"),
        )
        .collection_tag("users")
        .tag("tenant:7");

    assert_eq!(unsafe_policy.key_value(), Some("users:active"));
    assert_eq!(
        safe_policy.key_value(),
        Some("tenant:7:users:status=active:page=1:sort=name_asc")
    );
    assert_ne!(unsafe_policy.key_value(), safe_policy.key_value());
}

#[test]
fn negative_and_explicit_invalidation_policies_encode_different_freshness_intent() {
    let negative = QueryCachePolicy::negative_cache()
        .for_entity("user", 404)
        .collection_tag("users");
    let explicit = QueryCachePolicy::no_ttl_explicit_invalidation()
        .key("reference:country-codes")
        .tag("reference-data");

    assert_eq!(negative.ttl_value(), Some(Duration::from_secs(30)));
    assert_eq!(
        negative.tags_value(),
        &["user:404".to_owned(), "users".to_owned()]
    );
    assert_eq!(explicit.ttl_value(), None);
    assert_eq!(explicit.tags_value(), &["reference-data".to_owned()]);
}

#[test]
fn fragile_upstream_policy_has_bounded_stale_on_loader_error() {
    let refresh = RefreshPolicy::new().stale_on_loader_error(Duration::from_secs(300));
    let policy = QueryCachePolicy::read_mostly()
        .for_entity("profile", 42)
        .refresh_policy(refresh);

    assert_eq!(policy.key_value(), Some("profile:42"));
    assert_eq!(policy.refresh_policy_value(), Some(refresh));
}
