use std::time::Duration;

use hydracache_db::{prepared_query_policy, CacheEntity, PreparedQueryPolicy, RefreshPolicy};

struct User;

impl CacheEntity for User {
    type Id = i64;

    const ENTITY: &'static str = "user";
    const COLLECTION: Option<&'static str> = Some("users");
}

fn main() {
    let prepared = prepared_query_policy!(
        per_entity = User,
        name = "load-user",
        ttl_secs = 300,
        stale_on_loader_error_secs = 120,
    );
    let refresh = RefreshPolicy::new().stale_on_loader_error(Duration::from_secs(120));
    let expected = PreparedQueryPolicy::per_entity()
        .cache_entity::<User>()
        .with_name("load-user")
        .ttl(Duration::from_secs(300))
        .refresh_policy(refresh);

    assert_eq!(prepared, expected);
    assert_eq!(prepared.bind_id(42).key_value(), Some("user:42"));

    let search = prepared_query_policy!(
        key_segments = ["tenant", 7_u64, "q", "ada:lovelace"],
        tag_segments = [["tenant", 7_u64], ["users"]],
        ttl_secs = 30,
    );

    assert_eq!(
        search.to_policy().key_value(),
        Some("tenant:7:q:ada%3Alovelace")
    );
    assert_eq!(
        search.to_policy().tags_value(),
        &["tenant:7".to_owned(), "users".to_owned()]
    );
}
