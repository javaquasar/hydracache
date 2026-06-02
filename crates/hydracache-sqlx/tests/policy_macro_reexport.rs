use std::time::Duration;

use hydracache_sqlx::{query_cache_policy, CacheEntity};

struct User;

impl CacheEntity for User {
    type Id = i64;

    const ENTITY: &'static str = "user";
    const COLLECTION: Option<&'static str> = Some("users");
}

#[test]
fn query_cache_policy_macro_is_reexported_for_sqlx_users() {
    let user_id = 42_i64;
    let policy = query_cache_policy!(
        name = "load-user",
        entity = User,
        id = user_id,
        ttl_secs = 30,
    );

    assert_eq!(policy.name(), Some("load-user"));
    assert_eq!(policy.key_value(), Some("user:42"));
    assert_eq!(
        policy.tags_value(),
        &["user:42".to_owned(), "users".to_owned()]
    );
    assert_eq!(policy.ttl_value(), Some(Duration::from_secs(30)));
}
