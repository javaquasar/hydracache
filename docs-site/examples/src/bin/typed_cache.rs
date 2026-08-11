use hydracache::{CacheOptions, HydraCache, TagSet};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Profile {
    id: u64,
    display_name: String,
}

#[tokio::main]
async fn main() -> hydracache::CacheResult<()> {
    // ANCHOR: typed-cache
    let cache = HydraCache::local().build();
    let profiles = cache.typed::<Profile>("profiles");

    profiles
        .put(
            "42",
            Profile {
                id: 42,
                display_name: "Ada".to_owned(),
            },
            CacheOptions::new()
                .ttl(Duration::from_secs(60))
                .tag_set(TagSet::new().entity("profile", 42).tag("profiles")),
        )
        .await?;

    let profile = profiles.get("42").await?;
    assert_eq!(profile.as_ref().map(|profile| profile.id), Some(42));

    cache.invalidate_tag("profile:42").await?;
    assert_eq!(profiles.get("42").await?, None);
    // ANCHOR_END: typed-cache

    Ok(())
}
