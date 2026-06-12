use std::time::Duration;

use hydracache::CacheKeyBuilder;
use hydracache_db::{HydraCacheEntity, QueryCachePolicy, RefreshPolicy};
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
