# HydraCache 0.14.0 Cacheable Functions Idea

Status: idea captured for `0.14.0`.

## Goal

Add a cacheable function API for ordinary expensive functions, not only
database/repository result caching.

HydraCache should support two related but distinct use cases:

- Local function caching: expensive calculations, HTTP calls, filesystem reads,
  feature lookups, or other non-database work.
- Database result caching: query/repository results with entity and collection
  invalidation semantics.

`0.14.0` should keep those concepts separate so the public API does not make the
library look SQLx- or database-only.

## Current Baseline

The runtime already supports ordinary async loaders:

```rust
let value = cache
    .get_or_load("expensive:42", CacheOptions::new(), || async {
        expensive_calculation(42).await
    })
    .await?;
```

This is powerful but verbose when repeated across many functions.

## Proposed Direction

Add a cacheable API near the base `hydracache` crate, not in `hydracache-db`.

The future API can be an attribute macro:

```rust
#[hydracache::cacheable(
    key = "expensive:{user_id}",
    ttl_secs = 60,
    tag = "user:{user_id}"
)]
async fn expensive(user_id: i64) -> Result<Value> {
    expensive_calculation(user_id).await
}
```

Or a function-like macro as a lower-risk first step:

```rust
let value = cacheable!(
    cache = cache,
    key = "expensive:42",
    ttl_secs = 60,
    load = || async {
        expensive_calculation(42).await
    }
).await?;
```

## Design Boundary

- `hydracache`: local cache runtime, ordinary function caching, typed local
  wrappers, single-flight, TTL, tags.
- `hydracache-db`: database/repository result cache, `QueryCachePolicy`,
  `CacheEntity`, entity tags, collection tags.
- `hydracache-sqlx`: SQLx-specific convenience helpers and re-exports.

`cacheable` for ordinary functions should not depend on `hydracache-db`.

## Open Questions

- How does the macro receive or locate the cache instance?
- Should the first version support only explicit `cache = ...`?
- Should the key syntax be a format string, segmented key builder, or expression?
- Should sync functions be supported, or async-only first?
- How should errors be represented for loader failures?
- Should tags support interpolation from function arguments?
- Should the macro support typed cache codecs or use the default codec first?

## Recommended First Step

Start with a function-like macro or explicit helper that wraps
`HydraCache::get_or_load`:

- Lower risk than attribute rewriting.
- Easier to test with normal unit tests and compile tests.
- Keeps cache ownership explicit.
- Establishes key/tag/TTL syntax before adding function attributes.

After that, add an attribute macro only if the explicit macro proves useful.
