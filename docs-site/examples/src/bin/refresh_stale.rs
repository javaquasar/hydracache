use hydracache::{CacheOptions, HydraCache, RefreshOptions};
use serde::{Deserialize, Serialize};
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
    // ANCHOR: refresh-stale
    let cache = HydraCache::local().build();

    cache
        .put(
            "profile:42",
            Profile {
                id: 42,
                display_name: "Ada".to_owned(),
            },
            CacheOptions::new().ttl(Duration::from_millis(10)),
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(20)).await;

    let value = cache
        .get_or_load_with_refresh(
            "profile:42",
            CacheOptions::new().ttl(Duration::from_secs(60)),
            RefreshOptions::new().stale_while_revalidate(Duration::from_secs(5)),
            || async {
                Ok::<_, LoadError>(Profile {
                    id: 42,
                    display_name: "Grace".to_owned(),
                })
            },
        )
        .await?;

    assert_eq!(value.display_name, "Ada");
    // ANCHOR_END: refresh-stale

    Ok(())
}
