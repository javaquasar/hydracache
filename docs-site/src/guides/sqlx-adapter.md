# SQLx Adapter

Use `hydracache-sqlx` when SQLx remains the query authority and HydraCache owns only the cache boundary.

The adapter re-exports `DbCache`, policy types, `HydraCacheEntity`, and `query_cache_policy!` for SQLx users. It does not replace SQLx macros, pools, transactions, or row mapping.

```rust,ignore
use hydracache::HydraCache;
use hydracache_sqlx::{DbCache, HydraCacheEntity, SqlxQueryExt};

#[derive(serde::Serialize, serde::Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}

async fn load_user(pool: sqlx::PgPool) -> hydracache_sqlx::Result<User> {
    let queries = DbCache::new(HydraCache::local().build(), "db");

    queries
        .for_entity::<User>(42)
        .fetch_with(move || async move {
            let (id, name): (i64, String) =
                sqlx::query_as("select id, name from users where id = $1")
                    .bind(42_i64)
                    .fetch_one(&pool)
                    .await?;

            Ok::<_, sqlx::Error>(User { id, name })
        })
        .await
}
```

Use `fetch_with` when SQLx macros, transactions, or repository functions need to stay in charge. Use `sqlx_one`, `sqlx_optional`, and `sqlx_all` for small pool-like helper calls.
