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

Keep SeaORM transactions in application code. Invalidate entity and collection
tags only after `commit()` succeeds. A rollback path should not invalidate
because existing cached values still describe the last committed database state.

Stage invalidations next to the write, but execute them only after the SeaORM
transaction commits:

```rust
use hydracache_seaorm::InvalidationPlan;

# async fn example(
#     db: sea_orm::DatabaseConnection,
#     queries: hydracache_seaorm::SeaOrmCache,
# ) -> hydracache::CacheResult<()> {
let tx = db.begin().await.expect("begin transaction");
let pending = InvalidationPlan::new()
    .tag("seaorm-user:42")
    .tag("seaorm-users");

// user::Entity::update(...).exec(&tx).await?;

tx.commit().await.expect("commit transaction");
pending.execute(queries.cache()).await?;
# Ok(())
# }
```

If `rollback()` is called or the transaction returns an error, drop the pending
plan and keep the cached value that still matches the last committed database
state.
