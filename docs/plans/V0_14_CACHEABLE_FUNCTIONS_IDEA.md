# HydraCache 0.14.0 Cacheable Functions Idea

Status: implemented in `0.14.0` as the explicit `cacheable!(...)`
function-like macro.

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

`0.14.0` implements the function-like macro first:

```rust
let value = hydracache::cacheable!(
    cache = cache,
    key = "expensive:42",
    tag = "expensive",
    ttl_secs = 60,
    load = || async { Ok::<_, std::io::Error>(42_u64) },
)
.await?;
```

Supported options:

- `cache = ...` is required and points to the `HydraCache` instance.
- `key = ...` is required and stays application-owned.
- `load = ...` is required and is passed to `HydraCache::get_or_load`.
- `tag = ...` can be repeated.
- `ttl = ...` accepts a `Duration` expression.
- `ttl_secs = ...` is a short `Duration::from_secs(...)` form.
- `ttl` and `ttl_secs` are mutually exclusive.

## Design Boundary

- `hydracache`: local cache runtime, ordinary function caching, typed local
  wrappers, single-flight, TTL, tags.
- `hydracache-db`: database/repository result cache, `QueryCachePolicy`,
  `CacheEntity`, entity tags, collection tags.
- `hydracache-sqlx`: SQLx-specific convenience helpers and re-exports.

`cacheable` for ordinary functions should not depend on `hydracache-db`.

## Open Questions

- Attribute macro design: how should it receive or locate the cache instance?
- Should future versions support generated keys from function arguments?
- Should the key syntax become a format string, segmented key builder, or stay
  an expression?
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
