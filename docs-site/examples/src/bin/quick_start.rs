use hydracache::{CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

#[tokio::main]
async fn main() -> hydracache::CacheResult<()> {
    // ANCHOR: quick-start
    let cache = HydraCache::local()
        .default_ttl(Duration::from_secs(300))
        .build();

    let user_42_options = CacheOptions::new()
        .ttl(Duration::from_secs(60))
        .tag("users")
        .tag("user:42");

    cache
        .put(
            "user:42",
            User {
                id: 42,
                name: "Ada".to_owned(),
            },
            user_42_options,
        )
        .await?;

    let cached: Option<User> = cache.get("user:42").await?;
    assert_eq!(cached.as_ref().map(|user| user.name.as_str()), Some("Ada"));

    let loads = Arc::new(AtomicUsize::new(0));
    let user_43_options = CacheOptions::new()
        .ttl(Duration::from_secs(60))
        .tag("users")
        .tag("user:43");

    let loaded = cache
        .get_or_load("user:43", user_43_options, {
            let loads = Arc::clone(&loads);
            move || async move {
                loads.fetch_add(1, Ordering::Relaxed);
                Ok::<_, std::io::Error>(User {
                    id: 43,
                    name: "Grace".to_owned(),
                })
            }
        })
        .await?;

    assert_eq!(loaded.name, "Grace");
    assert_eq!(loads.load(Ordering::Relaxed), 1);

    cache.invalidate_tag("user:42").await?;
    assert_eq!(cache.get::<User>("user:42").await?, None);
    // ANCHOR_END: quick-start

    Ok(())
}
