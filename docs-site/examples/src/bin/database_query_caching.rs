use hydracache::HydraCache;
use hydracache_db::{query_cache_policy, CacheEntity, DbCache};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
}

impl CacheEntity for User {
    type Id = i64;

    const ENTITY: &'static str = "user";
    const COLLECTION: Option<&'static str> = Some("users");
}

#[tokio::main]
async fn main() -> hydracache_db::Result<()> {
    // ANCHOR: database-query-caching
    let local = HydraCache::local().build();
    let queries = DbCache::new(local, "db");

    let user = queries
        .entity::<User>("user", 42)
        .collection_tag("users")
        .ttl(Duration::from_secs(60))
        .fetch_with(|| async {
            // Replace this with SQLx, Diesel, SeaORM, or repository code.
            Ok::<_, std::io::Error>(User {
                id: 42,
                name: "Ada".to_owned(),
            })
        })
        .await?;

    assert_eq!(user.name, "Ada");
    // ANCHOR_END: database-query-caching

    // ANCHOR: query-cache-policy
    let user_id = 42_i64;
    let policy = query_cache_policy!(
        preset = read_mostly,
        name = "load-user",
        entity = User,
        id = user_id,
        refresh_ahead_secs = 10,
        stale_while_revalidate_secs = 300,
    );

    assert_eq!(policy.name(), Some("load-user"));
    assert_eq!(policy.key_value(), Some("user:42"));
    assert!(policy.refresh_policy_value().is_some());

    let search = query_cache_policy!(
        name = "search-users",
        key_segments = ["tenant", 7_u64, "q", "ada:lovelace", "page", 1_u32],
        tag_segments = [["tenant", 7_u64], ["users"]],
        ttl_secs = 30,
    );

    assert_eq!(search.key_value(), Some("tenant:7:q:ada%3Alovelace:page:1"));
    assert_eq!(
        search.tags_value(),
        &["tenant:7".to_owned(), "users".to_owned()]
    );
    // ANCHOR_END: query-cache-policy

    Ok(())
}
