# hydracache-diesel

Diesel-facing helpers for HydraCache database result caching.

The database-neutral query cache API lives in `hydracache-db`. This crate keeps
Diesel users on a convenient import path while preserving Diesel's ownership of
queries, connections, transactions, and row mapping.

```rust
use hydracache::HydraCache;
use hydracache_diesel::{DieselCache, DieselQueryExt};

# async fn example() -> hydracache_diesel::Result<()> {
let queries = DieselCache::new(HydraCache::local().build(), "diesel");

let value = queries
    .entity::<String>("user", 42)
    .collection_tag("users")
    .diesel_one(move || Ok::<_, hydracache_diesel::diesel::result::Error>("Ada".to_owned()))
    .await?;

assert_eq!(value, "Ada");
# Ok(())
# }
```

`diesel_one`, `diesel_optional`, and `diesel_all` run the supplied Diesel
loader with `tokio::task::spawn_blocking`. Pass an owned pool handle or another
owned connection source into the closure; do not hold a borrowed Diesel
connection across an async cache boundary.

Keep Diesel transactions in application code. Invalidate entity and collection
tags only after a write transaction commits successfully. A rollback path should
leave existing cached values alone because the committed database state did not
change.

Stage invalidations while building the write path, then execute them only after
the Diesel transaction succeeds:

```rust
use hydracache_diesel::InvalidationPlan;

# fn example() -> Result<InvalidationPlan, diesel::result::Error> {
let pending = InvalidationPlan::new()
    .tag("diesel-user:42")
    .tag("diesel-users");

// connection.transaction::<_, diesel::result::Error, _>(|connection| {
//     diesel::sql_query("update users set name = 'Grace' where id = 42")
//         .execute(connection)?;
//     Ok(())
// })?;

Ok(pending)
# }
```

After the blocking transaction returns to async service code, call
`pending.execute(queries.cache()).await`. If Diesel rolls back or returns an
error, drop `pending` and keep the previous cached value.
