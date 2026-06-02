use std::time::Duration;

use hydracache_db::query_cache_policy;

fn main() {
    let ttl = Duration::from_secs(15);
    let policy = query_cache_policy!(
        key = "users:active",
        tag = "users",
        collection_tag = "tenants:7",
        ttl = ttl,
    );

    assert_eq!(policy.key_value(), Some("users:active"));
    assert_eq!(
        policy.tags_value(),
        &["users".to_owned(), "tenants%3A7".to_owned()]
    );
    assert_eq!(policy.ttl_value(), Some(Duration::from_secs(15)));
}
