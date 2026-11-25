use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{cacheable_loader, CacheOptions, HydraCache, RefreshOptions};
use hydracache_db::{query_cache_policy, DbCache, HydraCacheEntity, InvalidationPlan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Rate {
    currency: String,
    micros: u64,
}

#[derive(Debug)]
struct UpstreamError;

impl std::fmt::Display for UpstreamError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("upstream unavailable")
    }
}

impl std::error::Error for UpstreamError {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: bounded-cache
    let cache = HydraCache::local()
        // Weighted by encoded bytes: about 256 MiB of cached values in total.
        .max_capacity(256 * 1024 * 1024)
        // Reject (and count) any single value larger than 256 KiB.
        .max_entry_bytes(256 * 1024)
        .default_ttl(Duration::from_secs(300))
        .build();
    // ANCHOR_END: bounded-cache

    // ANCHOR: tenant-search
    let queries = DbCache::new(cache.clone(), "app");
    let (tenant_id, query, page) = (7_u64, "ada", 1_u32);

    // Every dimension that changes the result is part of the key. Tags name
    // the writes that can make it stale: anything in this tenant's users.
    let policy = query_cache_policy!(
        name = "search-users",
        key_segments = ["tenant", tenant_id, "users", "q", query, "page", page],
        tag_segments = [["tenant", tenant_id, "users"]],
        ttl_secs = 30,
    );

    assert_eq!(policy.tags_value(), &["tenant:7:users".to_owned()]);

    let results = queries
        .named::<Vec<User>>("search-users")
        .with_policy(policy)
        .fetch_with(|| async {
            // SELECT ... WHERE tenant_id = $1 AND name ILIKE $2 LIMIT 20 OFFSET 0
            Ok::<_, std::io::Error>(vec![User {
                id: 42,
                name: "Ada".to_owned(),
            }])
        })
        .await?;
    assert_eq!(results.len(), 1);

    // A write to any user in tenant 7 drops every cached page and query for
    // that tenant, and leaves other tenants untouched.
    InvalidationPlan::new()
        .tag("tenant:7:users")
        .execute(&cache)
        .await?;
    // ANCHOR_END: tenant-search

    // ANCHOR: stale-if-error
    cache
        .put(
            "rate:EUR",
            Rate {
                currency: "EUR".to_owned(),
                micros: 1_080_000,
            },
            CacheOptions::new().ttl(Duration::from_millis(10)),
        )
        .await?;
    tokio::time::sleep(Duration::from_millis(20)).await;

    // The upstream is down. Instead of failing the request, serve the last
    // known value for up to 10 minutes past its TTL.
    let rate = cache
        .get_or_load_with_refresh(
            "rate:EUR",
            CacheOptions::new().ttl(Duration::from_secs(60)),
            RefreshOptions::new().stale_on_loader_error(Duration::from_secs(600)),
            || async { Err::<Rate, _>(UpstreamError) },
        )
        .await?;
    assert_eq!(rate.micros, 1_080_000);
    // ANCHOR_END: stale-if-error

    // ANCHOR: external-api
    let upstream_calls = Arc::new(AtomicUsize::new(0));
    for _ in 0..3 {
        let calls = upstream_calls.clone();
        let rate: Rate = cacheable_loader!(
            cache = cache,
            key = "fx:USD",
            tags = ["fx"],
            ttl_secs = 60,
            load = move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                // reqwest::get("https://fx.example/usd").await?.json().await
                Ok::<_, UpstreamError>(Rate {
                    currency: "USD".to_owned(),
                    micros: 1_000_000,
                })
            },
        )
        .await?;
        assert_eq!(rate.currency, "USD");
    }
    assert_eq!(upstream_calls.load(Ordering::SeqCst), 1);
    // ANCHOR_END: external-api

    // ANCHOR: warm-up
    // At startup, preload hot entries so the first requests do not all miss.
    let hot_users = vec![
        User {
            id: 1,
            name: "Ada".to_owned(),
        },
        User {
            id: 2,
            name: "Grace".to_owned(),
        },
    ];
    for user in hot_users {
        let key = format!("user:{}", user.id);
        let options = CacheOptions::new()
            .ttl(Duration::from_secs(600))
            .tag(key.clone())
            .tag("users");
        cache.put(&key, user, options).await?;
    }
    assert!(cache.contains_key("user:2").await);
    // ANCHOR_END: warm-up

    // ANCHOR: hit-ratio
    let stats = cache.stats();
    let lookups = stats.hits + stats.misses;
    let hit_ratio = if lookups == 0 {
        0.0
    } else {
        stats.hits as f64 / lookups as f64
    };
    println!(
        "hit ratio {:.2}, loads {}, single-flight joins {}, evictions {}",
        hit_ratio, stats.loads, stats.single_flight_joins, stats.evictions
    );
    // ANCHOR_END: hit-ratio

    Ok(())
}
