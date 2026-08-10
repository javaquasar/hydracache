# SeaORM Adapter

Use `hydracache-seaorm` when SeaORM remains the query authority and HydraCache should wrap only the query-result boundary.

```rust,ignore
use hydracache::HydraCache;
use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};

async fn load_user_name() -> hydracache_seaorm::Result<String> {
    let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

    queries
        .entity::<String>("user", 42)
        .collection_tag("users")
        .sea_one(|| async {
            // Run a SeaORM query here.
            Ok::<_, hydracache_seaorm::sea_orm::DbErr>("Ada".to_owned())
        })
        .await
}
```

Use `fetch_with` when a SeaORM query, transaction, or repository function does not fit the convenience helper shape.
