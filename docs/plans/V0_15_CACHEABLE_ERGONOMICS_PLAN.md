# HydraCache 0.15.0 Cacheable Ergonomics Plan

Status: implemented in `0.15.0`.

## Goal

Make local function caching more pleasant without introducing an attribute macro
or hiding the explicit cache boundary.

`0.14.0` introduced `cacheable!(...)` as the first explicit macro over
`HydraCache::get_or_load`. `0.15.0` keeps the same design but removes two
small sources of boilerplate:

- attaching several tags one by one;
- wrapping infallible loader values in `Ok::<_, Error>(...)`.

## Implemented Scope

- Added `tags = ...` to `cacheable!(...)`.
- Kept repeated `tag = ...` support for compatibility.
- Allowed `tags = ...` to accept any `IntoIterator` accepted by
  `CacheOptions::tags`, including arrays and `TagSet`.
- Added `cacheable_infallible!(...)` over `HydraCache::get_or_insert_with`.
- Re-exported `cacheable_infallible!` from `hydracache`.
- Added runtime tests, macro unit tests, trybuild compile-pass tests, and a
  live example.

## Examples

Fallible loader:

```rust
let value = hydracache::cacheable!(
    cache = cache,
    key = "profile:42",
    tags = ["profiles", "profile:42"],
    ttl_secs = 60,
    load = move || async move {
        Ok::<_, std::io::Error>(load_profile(42).await)
    },
)
.await?;
```

Infallible loader:

```rust
let value = hydracache::cacheable_infallible!(
    cache = cache,
    key = "profile-count",
    tags = ["profiles"],
    ttl_secs = 60,
    load = || async { 1_u64 },
)
.await?;
```

`TagSet` remains useful when keys and invalidation tags are built from the same
domain metadata:

```rust
let value = hydracache::cacheable!(
    cache = cache,
    key = key.as_str(),
    tags = hydracache::TagSet::new().tag("profiles").entity("profile", 42),
    load = move || async move {
        Ok::<_, std::io::Error>(profile)
    },
)
.await?;
```

## Deferred

- Attribute macro syntax such as `#[cacheable(...)]`.
- Automatic key generation from function arguments.
- Sync-function support.
- Hidden global cache lookup.

Those features need a separate design pass because they change ownership,
lifetimes, and user expectations much more than a function-like macro does.
