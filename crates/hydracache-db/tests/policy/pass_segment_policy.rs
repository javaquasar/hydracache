use std::time::Duration;

use hydracache_db::{query_cache_policy, CacheKeyBuilder};

fn main() {
    let tenant_id = 7_u64;
    let permission_hash = "perm:read%v2";
    let query = "ada:lovelace";
    let page = 2_u32;
    let sort = "name:asc";
    let locale = "en-US";
    let region = "eu:west";
    let feature_flag = "beta%search";
    let window_start = "2026-06-16T00:00:00Z";
    let window_end = "2026-06-16T01:00:00Z";

    let policy = query_cache_policy!(
        name = "search-users",
        key_segments = [
            "tenant", tenant_id,
            "permission", permission_hash,
            "q", query,
            "page", page,
            "sort", sort,
            "locale", locale,
            "region", region,
            "feature", feature_flag,
            "window", window_start, window_end,
        ],
        tag_segments = [
            ["tenant", tenant_id],
            ["permission", permission_hash],
            ["users"],
            ["region", region],
            ["feature", feature_flag],
        ],
        ttl_secs = 30,
    );

    let expected_key = CacheKeyBuilder::new()
        .segment("tenant")
        .segment(tenant_id)
        .segment("permission")
        .segment(permission_hash)
        .segment("q")
        .segment(query)
        .segment("page")
        .segment(page)
        .segment("sort")
        .segment(sort)
        .segment("locale")
        .segment(locale)
        .segment("region")
        .segment(region)
        .segment("feature")
        .segment(feature_flag)
        .segment("window")
        .segment(window_start)
        .segment(window_end)
        .build_string();

    assert_eq!(policy.name(), Some("search-users"));
    assert_eq!(policy.key_value(), Some(expected_key.as_str()));
    assert_eq!(
        policy.tags_value(),
        &[
            "tenant:7".to_owned(),
            "permission:perm%3Aread%25v2".to_owned(),
            "users".to_owned(),
            "region:eu%3Awest".to_owned(),
            "feature:beta%25search".to_owned(),
        ]
    );
    assert_eq!(policy.ttl_value(), Some(Duration::from_secs(30)));
}
