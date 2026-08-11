use hydracache::{cacheable, cacheable_infallible, cacheable_loader, HydraCache};
use hydracache_db::{
    prepared_query_policy, query_cache_policy, CacheEntity, DbCache, HydraCacheEntity,
    InvalidationPlan,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct LoadError;

impl std::fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("load failed")
    }
}

impl std::error::Error for LoadError {}

// ANCHOR: entity-derive
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}

fn assert_user_entity_contract() {
    assert_eq!(User::cache_key_for(&42), "user:42");
    assert_eq!(User::entity_tag_for(&42), "user:42");
    assert_eq!(User::collection_tag(), Some("users".to_owned()));
}
// ANCHOR_END: entity-derive

// ANCHOR: cacheable-attribute
#[cacheable(
    cache = cache,
    key_segments = ["profile", user_id],
    tag_segments = [["user", user_id], ["users"]],
    ttl_secs = 60
)]
async fn load_profile(cache: &HydraCache, user_id: i64) -> Result<User, LoadError> {
    Ok(User {
        id: user_id,
        name: "Ada".to_owned(),
    })
}
// ANCHOR_END: cacheable-attribute

#[tokio::main]
async fn main() -> hydracache_db::Result<()> {
    assert_user_entity_contract();

    let cache = HydraCache::local().build();

    // ANCHOR: function-wrapper
    let user_id = 42_i64;
    let user = cacheable_loader!(
        cache = cache,
        key = "user:42",
        tags = ["user:42", "users"],
        ttl_secs = 60,
        load = move || async move {
            Ok::<_, LoadError>(User {
                id: user_id,
                name: "Ada".to_owned(),
            })
        },
    )
    .await?;

    assert_eq!(user.name, "Ada");

    let count = cacheable_infallible!(
        cache = cache,
        key = "users:count",
        tags = ["users"],
        ttl_secs = 30,
        load = || async { 1_u64 },
    )
    .await?;

    assert_eq!(count, 1);
    // ANCHOR_END: function-wrapper

    let profile = load_profile(&cache, 42).await?;
    assert_eq!(profile.id, 42);

    // ANCHOR: query-policy
    let policy = query_cache_policy!(
        preset = read_mostly,
        name = "load-user",
        entity = User,
        id = user_id,
        refresh_ahead_secs = 10,
        stale_while_revalidate_secs = 300,
    );

    assert_eq!(policy.key_value(), Some("user:42"));
    assert_eq!(
        policy.tags_value(),
        &["user:42".to_owned(), "users".to_owned()]
    );
    // ANCHOR_END: query-policy

    // ANCHOR: prepared-policy
    let queries = DbCache::new(cache.clone(), "db");
    let load_user = queries.prepare::<User>(prepared_query_policy!(
        per_entity = User,
        name = "load-user",
        ttl_secs = 300,
    ));

    let cached = load_user
        .load_id(42, || async {
            Ok::<_, std::io::Error>(User {
                id: 42,
                name: "Ada".to_owned(),
            })
        })
        .await?;

    assert_eq!(cached.id, 42);
    // ANCHOR_END: prepared-policy

    // ANCHOR: invalidation-plan
    let pending = InvalidationPlan::new().cache_entity::<User>(42);
    let report = pending.execute(&cache).await?;

    assert_eq!(report.tag_count, 2);
    // ANCHOR_END: invalidation-plan

    Ok(())
}
