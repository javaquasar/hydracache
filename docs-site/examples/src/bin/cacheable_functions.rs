use hydracache::{cacheable, cacheable_infallible, cacheable_loader, HydraCache};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Profile {
    id: u64,
    name: String,
}

#[derive(Debug)]
struct LoadError;

impl std::fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("load failed")
    }
}

impl std::error::Error for LoadError {}

// ANCHOR: cacheable-attribute
#[cacheable(
    cache = cache,
    key_segments = ["profile", profile_id],
    tag_segments = [["profile", profile_id], ["profiles"]],
    ttl_secs = 60
)]
async fn load_profile(cache: &HydraCache, profile_id: u64) -> Result<Profile, LoadError> {
    Ok(Profile {
        id: profile_id,
        name: "Ada".to_owned(),
    })
}
// ANCHOR_END: cacheable-attribute

#[tokio::main]
async fn main() -> hydracache::CacheResult<()> {
    let cache = HydraCache::local().build();

    // ANCHOR: cacheable-loader
    let profile_id = 42_u64;
    let profile = cacheable_loader!(
        cache = cache,
        key = "profile:42",
        tags = ["profiles", "profile:42"],
        ttl_secs = 60,
        load = move || async move {
            Ok::<_, LoadError>(Profile {
                id: profile_id,
                name: "Ada".to_owned(),
            })
        },
    )
    .await?;

    assert_eq!(profile.id, 42);
    // ANCHOR_END: cacheable-loader

    // ANCHOR: cacheable-infallible
    let total = cacheable_infallible!(
        cache = cache,
        key = "profiles:count",
        tags = ["profiles"],
        ttl_secs = 60,
        load = || async { 1_u64 },
    )
    .await?;

    assert_eq!(total, 1);
    // ANCHOR_END: cacheable-infallible

    let cached = load_profile(&cache, 42).await?;
    assert_eq!(cached.name, "Ada");

    Ok(())
}
