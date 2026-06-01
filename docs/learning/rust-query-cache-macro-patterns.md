# Rust Query Cache Macro Patterns

Status: reference analysis for future HydraCache macro ergonomics.

## Sources

- [Rust Reference: procedural macros](https://doc.rust-lang.org/stable/reference/procedural-macros.html)
- [SQLx crate documentation](https://docs.rs/crate/sqlx/latest)
- [SQLx FAQ](https://github.com/launchbadge/sqlx/blob/main/FAQ.md)
- [SQLx query_as! macro documentation](https://docs.rs/sqlx/latest/sqlx/macro.query_as.html)
- [cached proc macro documentation](https://docs.rs/cached/latest/cached/proc_macro/index.html)
- [SeaORM derive macro design](https://www.sea-ql.org/SeaORM/docs/internal-design/derive-macro/)
- [Diesel derive macro documentation](https://docs.diesel.rs/2.2.x/diesel_derives/index.html)

For adapter-specific wrapper design, see
[Rust ORM Adapter Patterns](rust-orm-adapter-patterns.md).

## Goal

HydraCache should support two equally valid usage modes:

- full manual control: users build keys, tags, TTLs, and loaders explicitly;
- generated convenience: users opt into macros that derive common keys, tags,
  diagnostics, and cache wrappers.

The macro layer must be sugar over the explicit API. It must not become a
parallel hidden runtime, query language, or database abstraction.

```text
Manual API is the source of truth.
Macros generate manual API calls.
Generated code must be predictable enough to write by hand.
```

## Rust Macro Toolbox

Rust gives HydraCache three useful macro shapes.

### Derive Macros

Derive macros inspect a struct or enum and generate additional items, usually
trait implementations.

Best fit for HydraCache:

- entity metadata;
- default entity kind;
- default collection tag;
- id-field based key generation;
- compile-time validation that a configured id field exists.

Example direction:

```rust
#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", id = "id", collection = "users")]
struct User {
    id: i64,
    name: String,
}
```

The derive should generate metadata traits only. It should not generate SQL or
execute database calls.

### Attribute Macros

Attribute macros wrap an item, commonly a function. This is the closest Rust
equivalent to Java annotations such as Spring's `@Cacheable` and `@CacheEvict`.

Best fit for HydraCache:

- wrapping repository/service functions in cache lookup logic;
- deriving a diagnostic name from the function path;
- applying default TTL and tags;
- generating invalidation around mutation functions.

Example direction:

```rust
#[hydracache::cached_query(
    cache = queries,
    entity(User, id = user_id),
    collection = "users",
    ttl = "60s"
)]
async fn find_user(pool: PgPool, queries: DbCache, user_id: i64) -> Result<User, sqlx::Error> {
    sqlx::query_as!(User, "select id, name from users where id = $1", user_id)
        .fetch_one(&pool)
        .await
}
```

The expansion should be equivalent to:

```rust
async fn find_user(pool: PgPool, queries: DbCache, user_id: i64) -> Result<User, sqlx::Error> {
    queries
        .entity::<User>("user", user_id)
        .collection_tag("users")
        .ttl(Duration::from_secs(60))
        .fetch_with(move || async move {
            sqlx::query_as!(User, "select id, name from users where id = $1", user_id)
                .fetch_one(&pool)
                .await
        })
        .await
        .map_err(Into::into)
}
```

This keeps the function body as the source of database truth. SQLx still owns
SQL validation and row mapping.

### Function-Like Macros

Function-like macros are powerful but are the easiest to overuse. They can
accept custom token syntax, but IDE support and error messages are usually worse
than normal Rust syntax.

Best fit for HydraCache:

- small helpers such as cache key construction;
- compile-time literal validation;
- future optional DSLs, only after the builder and attribute APIs are stable.

Avoid making a `cached_query! { ... }` DSL the main API. That would compete with
SQLx macros, Diesel's typed DSL, and normal Rust functions.

## Similar Rust Solutions

### SQLx

SQLx is the most important reference for database ergonomics. It uses macros to
validate SQL at compile time while deliberately avoiding an ORM or query DSL.
Its docs describe `query!` and `query_as!` as compile-time checked regular SQL.
The macro may connect to a development database during compilation, while
offline mode can cache query metadata in `.sqlx`.

Useful lessons:

- do not parse SQL inside HydraCache;
- do not duplicate SQLx's database validation role;
- keep SQL visible at the call site;
- support SQLx macros through `fetch_with`;
- make generated code inspectable and debuggable;
- be careful with macro-side effects and build-time requirements.

HydraCache should complement SQLx:

```text
SQLx macro validates SQL and maps rows.
HydraCache macro builds cache identity and wraps execution.
```

### cached

The `cached` crate is the closest Rust reference for annotation-like caching. It
offers attribute macros such as `#[cached]` and `#[once]`, with options for size,
TTL, custom key conversion, caching only `Ok` or `Some`, synchronizing duplicate
writes, and exposing a cached flag.

Useful lessons:

- attribute macros can make cache call sites very small;
- options like TTL, key conversion, `result`, `option`, and sync behavior are
  ergonomic;
- users still need escape hatches for custom cache type and key conversion.

What HydraCache should not copy directly:

- hidden static cache objects as the primary model;
- function-argument memoization as the only identity model;
- global cache ownership.

HydraCache's product shape is application-owned cache instances. Macro APIs
should therefore require or clearly discover a cache argument:

```rust
#[hydracache::cached_query(cache = queries, ...)]
```

### Diesel

Diesel uses derive macros such as `Queryable`, `Insertable`, `Identifiable`, and
`Selectable` to bind Rust types to database schemas and statically typed query
construction.

Useful lessons:

- derive macros are excellent for repetitive trait glue;
- explicit attributes make generated behavior reviewable;
- static typing can move many errors to compile time.

What HydraCache should avoid:

- coupling cache metadata to one query engine;
- requiring schema-level DSL adoption;
- generating large implicit type machinery before the manual API is mature.

### SeaORM

SeaORM's derive macro design shows a model-first approach. `DeriveEntityModel`
generates entity, column, primary-key, model, and active-model metadata from a
Rust model.

Useful lessons:

- derive macros can make entity metadata convenient;
- id/primary-key metadata is a natural place to generate cache keys;
- enum/column iteration patterns are useful for metadata-driven features.

What HydraCache should avoid:

- becoming an ORM;
- generating database queries from entity definitions;
- tying invalidation correctness to ORM entity tracking.

## Recommended HydraCache Layers

HydraCache should expose progressive layers. Each layer should be optional and
should desugar to the layer below it.

### Layer 0: Explicit Core

This remains the most important API:

```rust
let user = queries
    .cached::<User>()
    .key(format!("user:{user_id}"))
    .tag(format!("user:{user_id}"))
    .tag("users")
    .ttl(Duration::from_secs(60))
    .fetch_with(move || async move {
        sqlx::query_as!(User, "select id, name from users where id = $1", user_id)
            .fetch_one(&pool)
            .await
    })
    .await?;
```

This is the API for advanced users, unusual keys, complex invalidation, and
cases where macro expansion would be more confusing than useful.

### Layer 1: Manual Ergonomic Helpers

This is the planned `0.9.0` style:

```rust
let user = queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
    .ttl(Duration::from_secs(60))
    .fetch_with(move || async move {
        sqlx::query_as!(User, "select id, name from users where id = $1", user_id)
            .fetch_one(&pool)
            .await
    })
    .await?;
```

This should be implemented before procedural macros. It creates a stable target
for macros to generate.

### Layer 2: Entity Metadata Derive

Introduce a trait such as `CacheEntity`:

```rust
pub trait CacheEntity {
    const ENTITY: &'static str;
    const COLLECTION: Option<&'static str>;

    type Id;

    fn cache_key_for(id: &Self::Id) -> CacheKeyBuilder;
    fn entity_tag_for(id: &Self::Id) -> String;
    fn collection_tag() -> Option<&'static str> {
        Self::COLLECTION
    }
}
```

Then derive it:

```rust
#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", id = "id", collection = "users")]
struct User {
    id: i64,
    name: String,
}
```

Possible use:

```rust
let user = queries
    .for_entity::<User>(user_id)
    .ttl(Duration::from_secs(60))
    .fetch_with(loader)
    .await?;
```

This gives users Java-like domain ergonomics without hiding database execution.

### Layer 3: Cached Query Attribute

After the explicit helper layer and derive metadata are stable, add a repository
function wrapper:

```rust
#[hydracache::cached_query(
    cache = queries,
    entity(User, id = user_id),
    ttl = "60s"
)]
async fn find_user(pool: PgPool, queries: DbCache, user_id: i64) -> Result<User, sqlx::Error> {
    sqlx::query_as!(User, "select id, name from users where id = $1", user_id)
        .fetch_one(&pool)
        .await
}
```

Defaults:

- diagnostic name: module path plus function name;
- cache only successful `Result::Ok` values;
- key: entity metadata plus selected id expression;
- tag: entity tag;
- collection tag: from entity metadata if configured;
- TTL: explicit attribute or cache default.

Escape hatches:

```rust
#[hydracache::cached_query(
    cache = queries,
    key = CacheKeyBuilder::new("user").segment(tenant_id).segment(user_id),
    tags = [
        format!("tenant:{tenant_id}"),
        format!("user:{tenant_id}:{user_id}")
    ],
    ttl = Duration::from_secs(30),
    name = "user.lookup_by_tenant"
)]
```

### Layer 4: Invalidation Attribute

Spring Cache's `@CacheEvict` maps naturally to mutation functions, but this
should come after cached query macros:

```rust
#[hydracache::invalidate(
    cache = queries,
    after = "success",
    tags = [format!("user:{user_id}"), "users"]
)]
async fn update_user(pool: PgPool, queries: DbCache, user_id: i64, name: String) -> Result<(), sqlx::Error> {
    sqlx::query!("update users set name = $1 where id = $2", name, user_id)
        .execute(&pool)
        .await?;
    Ok(())
}
```

This makes invalidation visible, keeps write semantics explicit, and avoids
pretending that HydraCache can infer all freshness rules.

## What Can Be Auto-Generated Safely

Safe to generate:

- diagnostic names from function path;
- key and tag strings from explicit macro arguments;
- entity key/tag builders from derive metadata;
- collection tags from derive metadata;
- `fetch_with` wrapping;
- "cache only Ok" behavior for `Result<T, E>`;
- "cache only Some" behavior when explicitly requested;
- invalidation after successful mutation when tags are explicit.

Risky or not recommended:

- deriving cache keys from SQL text;
- deriving cache keys from all function arguments by default;
- inferring the cache variable by type;
- parsing SQL to discover table names;
- invalidating by guessed table/entity relationships;
- hiding database calls behind generated repository implementations too early;
- creating global static caches by default.

## Important Rust Limitations

Procedural macros operate on tokens, not type-checked Rust. A macro cannot know
that a parameter is a `DbCache` unless the user identifies it by syntax or name.

Therefore this is good:

```rust
#[hydracache::cached_query(cache = queries, entity(User, id = user_id))]
```

This is fragile and should be avoided:

```rust
#[hydracache::cached_query]
```

Attribute macros also make async lifetimes more visible. The current
`fetch_with` API requires a `FnOnce + Send + 'static` loader. Generated wrappers
may need `move` closures and owned or cloned handles. That is acceptable, but it
must be documented because it affects function signatures.

## Suggested Crate Layout

Keep macros optional:

```text
crates/
  hydracache-core/
  hydracache/
  hydracache-db/
  hydracache-sqlx/
  hydracache-macros/      # optional proc-macro crate
```

Possible features:

```toml
[features]
macros = ["dep:hydracache-macros"]
```

The main crates can re-export macros behind a feature, but the proc-macro code
must live in a separate `proc-macro` crate.

## Testing Strategy

Macro code needs stronger testing than normal builder helpers:

- unit tests for generated key/tag helper functions;
- integration tests that use generated macros against the real cache;
- `trybuild` compile-pass tests for valid macro usage;
- `trybuild` compile-fail tests for missing cache argument, missing id field,
  invalid TTL, unsupported return type, and invalid entity metadata;
- docs examples showing both manual and generated equivalents.

Every macro example should include an equivalent explicit expansion. This keeps
the documentation honest and helps users debug issues without magic.

## Recommended Roadmap

### Step 1: Finish Manual Ergonomics

Implement `entity`, `collection`, `for_entity`, and `collection_tag` first.
This gives macros a stable target.

### Step 2: Add CacheEntity Trait

Add the trait manually before adding the derive macro:

```rust
impl CacheEntity for User {
    const ENTITY: &'static str = "user";
    const COLLECTION: Option<&'static str> = Some("users");
    type Id = i64;
}
```

This lets users test the model without macros.

### Step 3: Add HydraCacheEntity Derive

Generate `CacheEntity` impls from struct metadata. Keep the derive small and
strict. Good compile errors matter more than clever inference.

### Step 4: Add cached_query Attribute

Wrap functions only after manual and derive layers are proven. Start with
explicit `cache = ...`, `entity(...)`, `key = ...`, `tags = ...`, and `ttl = ...`.

### Step 5: Add invalidate Attribute

Add mutation-side invalidation as a separate feature. Do not hide it inside the
read query macro.

## Adapter-Aware Macro Guidance

Macros should generate cache wrapping code, not database queries. This
distinction matters most for Diesel and SeaORM.

For Diesel, a macro should wrap the function body or a user-supplied closure. It
should not try to inspect Diesel's query type, infer a table, or borrow `&mut
Connection` across a hidden async boundary.

For SeaORM, a macro can optionally use entity metadata because SeaORM already has
`Entity`, `Model`, and primary-key concepts. Even there, HydraCache should keep
query construction visible and only generate the cache descriptor and wrapper.

Recommended macro expansion target:

```text
derive/entity metadata -> DbCache::entity / DbQuery::for_entity
attribute query wrapper -> DbQuery::fetch_with or adapter extension method
attribute invalidation -> explicit invalidate_tag calls
```

## Recommendation

The best HydraCache macro story is not "Rust Spring annotations." It is:

```text
Start explicit.
Add domain helpers.
Derive cache metadata.
Optionally wrap repository functions.
Always allow dropping back to the manual builder.
```

This gives Java-like convenience without Java-like hidden runtime behavior.
