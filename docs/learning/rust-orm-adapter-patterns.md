# Rust ORM Adapter Patterns

Status: reference analysis for future HydraCache database adapters.

## Sources

- [Diesel RunQueryDsl](https://docs.diesel.rs/2.0.x/diesel/query_dsl/trait.RunQueryDsl.html)
- [Diesel query_dsl module](https://docs.diesel.rs/2.0.x/diesel/query_dsl/index.html)
- [Diesel getting started guide](https://diesel.rs/guides/getting-started.html)
- [diesel_async RunQueryDsl](https://docs.rs/diesel-async/latest/diesel_async/trait.RunQueryDsl.html)
- [SeaORM select documentation](https://www.sea-ql.org/SeaORM/docs/next/basic-crud/select/)
- [SeaORM derive macro design](https://www.sea-ql.org/SeaORM/docs/0.6.x/internal-design/derive-macro/)
- [SeaORM docs.rs API reference](https://docs.rs/sea-orm/latest/sea_orm/)

## Goal

HydraCache should be easy to use with SQLx, Diesel, SeaORM, and future database
libraries without moving database execution into the core crate.

The adapter design should preserve this boundary:

```text
Database library owns query construction, execution, transactions, and row mapping.
HydraCache owns cache keys, tags, TTL, single-flight, codecs, and invalidation.
```

Users should be able to choose between:

- full manual control through `DbQuery::fetch_with`;
- adapter helper traits for common fetch shapes;
- optional macros that generate the same manual or adapter calls.

## Adapter Crate Shape

Keep database integrations optional and small:

```text
crates/
  hydracache-db/             # database-agnostic query descriptor
  hydracache-sqlx/           # SQLx extension trait
  hydracache-diesel/         # Diesel sync or diesel_async extension traits
  hydracache-seaorm/         # SeaORM extension traits
  hydracache-macros/         # optional derive and attribute macros
```

The core `hydracache-db` crate should not depend on Diesel, SeaORM, or SQLx.
Adapter crates should implement extension traits for `DbQuery<T, C>`.

## Shared Adapter Vocabulary

Every adapter should use the same result-cardinality vocabulary:

```text
fetch_one       exactly one row or database-library error
fetch_optional  zero-or-one row
fetch_all       all rows as Vec<T>
```

This keeps the API familiar across SQLx, Diesel, and SeaORM even though each
library exposes different native method names.

## Manual Baseline

The universal path must keep working for all libraries:

```rust
let user = queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
    .ttl(Duration::from_secs(60))
    .fetch_with(move || async move {
        repository.find_user(user_id).await
    })
    .await?;
```

This is the escape hatch for transactions, custom repositories, complex joins,
streaming queries, or library-specific APIs that adapter crates do not cover.

## Diesel Adapter Analysis

Diesel's native execution model is centered on `RunQueryDsl`. The synchronous
trait exposes methods such as `first`, `get_result`, `get_results`, and `load`
against a mutable connection reference. Optional results are normally represented
by calling Diesel's `optional()` extension on a `NotFound` result.

This creates two important constraints:

- sync Diesel queries need `&mut Conn`;
- blocking database execution should not happen directly on an async runtime
  worker thread.

### Diesel Sync Strategy

For sync Diesel, the safest adapter should not accept a borrowed `&mut Conn`
across an async cache boundary. Instead, users should pass a pool or a closure
that obtains a connection inside the blocking operation.

Recommended low-level pattern:

```rust
let user = queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
    .ttl(Duration::from_secs(60))
    .fetch_with(move || async move {
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            users::table.find(user_id).first::<User>(&mut conn)
        })
        .await
        .expect("diesel worker panicked")
    })
    .await?;
```

This pattern is explicit and safe, but verbose. A future `hydracache-diesel`
adapter can package it.

### Possible Sync Diesel Helper

```rust
let user = queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
    .ttl(Duration::from_secs(60))
    .diesel_first(pool.clone(), move |conn| {
        users::table.find(user_id).first::<User>(conn)
    })
    .await?;
```

Potential trait:

```rust
#[async_trait]
pub trait DieselQueryExt<T, C>
where
    C: CacheCodec,
{
    async fn diesel_first<P, F, E>(self, pool: P, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        P: DieselConnectionPool + Send + Sync + Clone + 'static,
        F: FnOnce(&mut P::Connection) -> std::result::Result<T, E> + Send + 'static,
        E: Error + Send + Sync + 'static;
}
```

The concrete pool abstraction needs care. Diesel users may use `r2d2`,
`deadpool-diesel`, `bb8`, custom pools, or direct connections. A small adapter
should avoid blessing one pool too early.

Better first step:

```rust
query.fetch_blocking_with(move || {
    let mut conn = pool.get()?;
    users::table.find(user_id).first::<User>(&mut conn)
})
```

This could live in `hydracache-db` because it is not Diesel-specific. Then
Diesel examples become simple without adding a Diesel dependency.

### diesel_async Strategy

`diesel_async` is a more natural match because its `RunQueryDsl` methods return
futures. It exposes methods such as `first`, `get_result`, `get_results`,
`load`, and `load_stream`.

Possible adapter shape:

```rust
let user = queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
    .diesel_async_first(conn, users::table.find(user_id))
    .await?;
```

However, this has a lifetime problem: Diesel async methods commonly need
`&mut Conn`, while `DbQuery::fetch_with` currently expects a `'static` loader.
Holding a borrowed mutable connection across a generated cache loader is hard to
make ergonomic.

Safer first adapter shape:

```rust
let user = queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
    .fetch_with(move || async move {
        let mut conn = pool.get().await?;
        users::table.find(user_id).first::<User>(&mut conn).await
    })
    .await?;
```

A future adapter can wrap a pool-like async connection source instead of a
borrowed connection.

### Diesel Macro Implications

Diesel wrappers should not try to inspect Diesel's query type. The macro should
only generate cache identity and wrap the function body:

```rust
#[hydracache::cached_query(
    cache = queries,
    entity(User, id = user_id),
    collection = "users",
    ttl = "60s"
)]
async fn find_user(pool: DbPool, queries: DbCache, user_id: i64) -> Result<User, Error> {
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        users::table.find(user_id).first::<User>(&mut conn)
    })
    .await?
}
```

This keeps Diesel's type system and query construction untouched.

## SeaORM Adapter Analysis

SeaORM is async-first. Its common read shape is:

```rust
let user: Option<user::Model> = User::find_by_id(user_id).one(db).await?;
let users: Vec<user::Model> = User::find().all(db).await?;
```

SeaORM also already has rich entity metadata through derives such as
`DeriveEntityModel`, `DeriveColumn`, `DerivePrimaryKey`, and `DeriveModel`.
That makes it a strong fit for HydraCache entity helpers and derive-based cache
metadata.

### SeaORM Helper Shape

SeaORM's native methods already match the desired result-cardinality model:

```text
one(db) -> Result<Option<Model>, DbErr>
all(db) -> Result<Vec<Model>, DbErr>
```

HydraCache can expose a thin adapter:

```rust
let user = queries
    .entity::<user::Model>("user", user_id)
    .collection_tag("users")
    .ttl(Duration::from_secs(60))
    .sea_one(User::find_by_id(user_id), db.clone())
    .await?;
```

For collections:

```rust
let users = queries
    .collection::<user::Model>("users")
    .ttl(Duration::from_secs(30))
    .sea_all(User::find().order_by_asc(user::Column::Id), db.clone())
    .await?;
```

The adapter can internally call `fetch_value_with`:

```rust
self.fetch_value_with(move || async move {
    selector.one(&db).await
})
.await
```

### SeaORM Entity Metadata

SeaORM already knows the entity type and primary key shape, but HydraCache
should not depend on SeaORM inside `hydracache-db`. A SeaORM adapter can provide
optional helpers:

```rust
let user = queries
    .sea_entity::<user::Entity>(user_id)
    .ttl(Duration::from_secs(60))
    .sea_one(user::Entity::find_by_id(user_id), db.clone())
    .await?;
```

Potential behavior:

- entity name from SeaORM entity metadata or explicit override;
- primary key value becomes key segment;
- collection tag from table/entity name;
- output model type inferred from `EntityTrait::Model`.

This is convenient, but it should be a later layer. The first SeaORM adapter
should prefer explicit `entity::<Model>("user", id)` so behavior is obvious.

### SeaORM Macro Implications

SeaORM is the easiest target for a high-level macro because it is async and has
entity metadata:

```rust
#[hydracache::cached_query(
    cache = queries,
    seaorm(entity = user::Entity, id = user_id),
    ttl = "60s"
)]
async fn find_user(db: DatabaseConnection, queries: DbCache, user_id: i64) -> Result<Option<user::Model>, DbErr> {
    user::Entity::find_by_id(user_id).one(&db).await
}
```

The macro should still wrap the body, not generate the query:

```rust
queries
    .entity::<user::Model>("user", user_id)
    .collection_tag("users")
    .ttl(Duration::from_secs(60))
    .fetch_value_with(move || async move {
        user::Entity::find_by_id(user_id).one(&db).await
    })
    .await
```

This means SeaORM can evolve independently and HydraCache only owns caching.

## Common Wrapper Design

Adapter wrappers should be extension traits, not new cache types:

```rust
pub trait SeaOrmQueryExt<T, C>
where
    C: CacheCodec,
{
    async fn sea_one<S, E>(self, selector: S, db: E) -> Result<Option<T>>;
    async fn sea_all<S, E>(self, selector: S, db: E) -> Result<Vec<T>>;
}
```

```rust
pub trait DieselBlockingQueryExt<T, C>
where
    C: CacheCodec,
{
    async fn blocking_first<F, E>(self, loader: F) -> Result<T>;
    async fn blocking_optional<F, E>(self, loader: F) -> Result<Option<T>>;
    async fn blocking_all<F, E>(self, loader: F) -> Result<Vec<T>>;
}
```

The exact trait bounds should be developed in the adapter crates. Keep the core
design rule stable:

```text
Adapter methods call DbQuery::fetch_value_with.
They do not own key generation, invalidation policy, or database query creation.
```

## Macro Compatibility

Macros should target the common `DbQuery` builder surface:

```text
manual key/tag API
entity/collection helpers
fetch_with/fetch_value_with
adapter extension methods
```

This gives each database library a compatible path:

```text
SQLx macro body -> fetch_with or SqlxQueryExt
Diesel sync body -> fetch_blocking_with or DieselBlockingQueryExt
diesel_async body -> fetch_with with owned pool/connection source
SeaORM body -> fetch_with or SeaOrmQueryExt
```

Avoid generating library-specific queries in HydraCache macros. Generate cache
wrapping code around the user's body.

## Invalidation Support

For both Diesel and SeaORM, writes should be handled through explicit
invalidation, not guessed from update/delete queries.

Good:

```rust
#[hydracache::invalidate(
    cache = queries,
    after = "success",
    tags = [format!("user:{user_id}"), "users"]
)]
async fn update_user(...) -> Result<(), Error> {
    ...
}
```

Risky:

```rust
// Do not infer this automatically from table names in SQL or ORM query types.
#[hydracache::invalidate_automatically]
```

Freshness rules are domain rules. HydraCache should make them concise, not
pretend to infer them perfectly.

## Recommended Roadmap

### Step 1: Add Library-Agnostic Blocking Helper

Consider adding:

```rust
DbQuery::fetch_blocking_with(...)
```

This helps sync Diesel, filesystem loaders, legacy clients, and other blocking
data sources without making `hydracache-db` depend on Diesel.

### Step 2: Add SeaORM Adapter First

SeaORM is async-first and should be the simplest non-SQLx adapter. Start with:

- `sea_one`;
- `sea_all`;
- examples for `Entity::find_by_id(...).one(db)`;
- examples for `Entity::find().all(db)`;
- testcontainers integration if practical.

### Step 3: Add Diesel Examples Before Diesel Traits

Diesel's sync and async connection models require care. Before adding public
Diesel extension traits, document manual patterns with:

- `spawn_blocking`;
- pool-owned connection acquisition;
- `fetch_with`;
- optional result handling.

### Step 4: Add Diesel Adapter Behind Explicit Feature

If examples are stable, add `hydracache-diesel` with separate features:

```toml
[features]
sync = ["diesel"]
async = ["diesel-async"]
```

Do not force async users to depend on sync Diesel or vice versa.

### Step 5: Add Macro Examples For Each Adapter

Macro docs should show equivalent explicit code for:

- SQLx;
- SeaORM;
- Diesel sync;
- diesel_async.

This will keep the generated layer honest and easy to debug.

## Recommendation

SeaORM should be the first ORM adapter because its async execution and entity
metadata fit HydraCache naturally.

Diesel should start with documented manual patterns and a library-agnostic
blocking helper. A Diesel-specific adapter is valuable, but only after the pool
and lifetime story is proven with examples.

Both adapters should make future macro support easier by reusing the same
`entity`, `collection`, `collection_tag`, `fetch_with`, and `fetch_value_with`
surface.
