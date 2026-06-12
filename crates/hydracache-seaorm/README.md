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
    .sea_one(|| async { Ok::<_, hydracache_seaorm::sea_orm::DbErr>("Ada".to_owned()) })
    .await?;

assert_eq!(value, "Ada");
# Ok(())
# }
```

`sea_one`, `sea_optional`, and `sea_all` execute the async loader only on a
cache miss. Use `sea_optional` for ordinary SeaORM
`Entity::find_by_id(id).one(&db).await` calls, and use `sea_all` for
`Entity::find().all(&db).await` collection queries.
