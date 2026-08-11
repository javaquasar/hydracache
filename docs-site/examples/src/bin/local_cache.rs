use hydracache::{CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Profile {
    id: u64,
    display_name: String,
}

#[derive(Debug)]
struct LoadError;

impl std::fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("load failed")
    }
}

impl std::error::Error for LoadError {}

#[tokio::main]
async fn main() -> hydracache::CacheResult<()> {
    // ANCHOR: local-cache
    let cache = HydraCache::local().build();
    let loads = Arc::new(AtomicUsize::new(0));

    let first = cache
        .get_or_load(
            "profile:42",
            CacheOptions::new()
                .ttl(Duration::from_secs(60))
                .tag("profiles")
                .tag("profile:42"),
            {
                let loads = Arc::clone(&loads);
                move || async move {
                    loads.fetch_add(1, Ordering::Relaxed);
                    Ok::<_, LoadError>(Profile {
                        id: 42,
                        display_name: "Ada".to_owned(),
                    })
                }
            },
        )
        .await?;

    let second: Option<Profile> = cache.get("profile:42").await?;
    assert_eq!(second, Some(first));
    assert_eq!(loads.load(Ordering::Relaxed), 1);

    cache.invalidate_tag("profile:42").await?;
    assert_eq!(cache.get::<Profile>("profile:42").await?, None);
    // ANCHOR_END: local-cache

    Ok(())
}
