# hydracache-seaorm

SeaORM-facing helpers for HydraCache database result caching.

The database-neutral query cache API lives in `hydracache-db`. This crate keeps
SeaORM users on a convenient import path while preserving SeaORM's ownership of
entities, selectors, connections, transactions, and row mapping.

```rust
use hydracache::HydraCache;
use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};

# async fn example() -> hydracache_seaorm::Result<()> {
let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

let value = queries
    .entity::<String>("user", 42)
    .collection_tag("users")
    .sea_one(|| async { Ok::<_, hydracache_seaorm::sea_orm::DbErr>(Some("Ada".to_owned())) })
    .await?;

assert_eq!(value, Some("Ada".to_owned()));
# Ok(())
# }
```

`sea_one`, `sea_value`, and `sea_all` execute the async loader only on a cache
miss. The loader should contain the ordinary SeaORM call, such as
`Entity::find_by_id(id).one(&db).await` or `Entity::find().all(&db).await`.
