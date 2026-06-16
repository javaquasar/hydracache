use std::time::Duration;

use hydracache_db::{query_cache_policy, RefreshPolicy};

fn main() {
    let policy = query_cache_policy!(
        preset = read_mostly,
        name = "load-catalog",
        key = "catalog:active",
        tag = "catalog",
        refresh_ahead_secs = 10,
        stale_while_revalidate_secs = 300,
        stale_on_loader_error_secs = 600,
    );

    let refresh = RefreshPolicy::new()
        .refresh_ahead(Duration::from_secs(10))
        .stale_while_revalidate(Duration::from_secs(300))
        .stale_on_loader_error(Duration::from_secs(600));

    assert_eq!(policy.name(), Some("load-catalog"));
    assert_eq!(policy.key_value(), Some("catalog:active"));
    assert_eq!(policy.tags_value(), &["catalog".to_owned()]);
    assert_eq!(policy.ttl_value(), Some(Duration::from_secs(300)));
    assert_eq!(policy.refresh_policy_value(), Some(refresh));
}
