# Diesel Adapter

Use `hydracache-diesel` when Diesel remains the query authority and cached values should still use HydraCache keys, tags, TTL, and diagnostics.

Diesel is synchronous, so Diesel helpers run the supplied loader on `tokio::task::spawn_blocking`. The loader should own or acquire its connection inside the closure.

```rust,ignore
use hydracache::HydraCache;
use hydracache_diesel::{DieselCache, DieselQueryExt};

async fn load_user_name() -> hydracache_diesel::Result<String> {
    let queries = DieselCache::new(HydraCache::local().build(), "diesel");

    queries
        .entity::<String>("user", 42)
        .collection_tag("users")
        .diesel_one(move || {
            // Acquire/use a Diesel connection here.
            Ok::<_, hydracache_diesel::diesel::result::Error>("Ada".to_owned())
        })
        .await
}
```

Use `fetch_with` when custom repository code, transactions, or an async Diesel wrapper needs more control.
