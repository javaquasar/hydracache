use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hydracache::{cacheable, cacheable_infallible, CacheKeyBuilder, HydraCache, TagSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Profile {
    id: u64,
    name: String,
}

#[tokio::main]
async fn main() -> hydracache::CacheResult<()> {
    let cache = HydraCache::local().build();
    let load_count = Arc::new(AtomicUsize::new(0));
    let profile_id = 42_u64;
    let key = CacheKeyBuilder::new()
        .entity("profile", profile_id)
        .build_string();

    let first: Profile = cacheable!(
        cache = cache,
        key = key.as_str(),
        tags = TagSet::new().tag("profiles").entity("profile", profile_id),
        ttl_secs = 60,
        load = {
            let load_count = Arc::clone(&load_count);
            move || async move {
                load_count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, std::io::Error>(Profile {
                    id: profile_id,
                    name: "Ada".to_owned(),
                })
            }
        },
    )
    .await?;

    let second: Profile = cacheable!(
        cache = cache,
        key = key.as_str(),
        tags = TagSet::new().tag("profiles").entity("profile", profile_id),
        ttl_secs = 60,
        load = {
            let load_count = Arc::clone(&load_count);
            move || async move {
                load_count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, std::io::Error>(Profile {
                    id: profile_id,
                    name: "Grace".to_owned(),
                })
            }
        },
    )
    .await?;

    assert_eq!(first, second);
    assert_eq!(load_count.load(Ordering::SeqCst), 1);

    cache.invalidate_tag("profile:42").await?;
    assert!(!cache.contains_key(key.as_str()).await);

    let count: u64 = cacheable_infallible!(
        cache = cache,
        key = "profiles:count",
        tags = ["profiles"],
        ttl_secs = 60,
        load = || async { 1_u64 },
    )
    .await?;

    assert_eq!(count, 1);
    Ok(())
}
