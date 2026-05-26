# HydraCache v0 Release Preparation Plan

> Status: implemented in the local workspace.
> Goal: ship a small but genuinely useful local async cache before adding SQLx, macros, or distributed features.

---

## 1. Release Goal

HydraCache v0 should be a pleasant local async cache library.

The user should be able to:

```rust
let cache = HydraCache::local()
    .default_ttl(Duration::from_secs(300))
    .max_capacity(10_000)
    .build();

let user = cache
    .get_or_load(
        "user:42",
        CacheOptions::new()
            .ttl(Duration::from_secs(60))
            .tags(["user:42", "users"]),
        || async { load_user(42).await },
    )
    .await?;

cache.invalidate_tag("user:42").await?;
```

The first release must stand on its own. It is not a SQLx demo and not a distributed-cache prototype.

---

## 2. Included Scope

### 2.1 Local Cache API

Implement:

- `HydraCache::local()`
- `get`
- `put`
- `get_or_load`
- `invalidate_key`
- `invalidate_tag`
- `flush`

### 2.2 TTL

Implement:

- default TTL on the builder
- per-entry TTL through `CacheOptions`
- expiration via Moka

Do not implement refresh or stale-while-revalidate in v0.

### 2.3 Tags

Implement:

- multiple tags per entry
- in-memory tag index
- `invalidate_tag(tag)` removing all tagged entries
- tag cleanup on explicit invalidation

The tag index is not persisted. On restart, the cache starts empty.

### 2.4 Moka Backend

Use:

- `moka::future::Cache`

Do not implement custom eviction policy.

### 2.5 Serialization

Use:

- `Bytes` as the storage value
- `serde` for value traits
- `postcard` as the default codec

v0 accepts the serialization cost because it keeps the future query-adapter and distributed paths simple.

### 2.6 Basic Stats

Expose lightweight stats:

- hits
- misses
- loads
- invalidations
- evictions if available without complexity

Use an internal `CacheStats` snapshot. Do not add Prometheus or tracing integrations in v0.

### 2.7 Errors

Implement `CacheError` for:

- encode failures
- decode failures
- loader failures through a documented generic path
- backend/internal failures if needed

---

## 3. Explicitly Out Of Scope

Do not implement in v0:

- SQLx adapter
- proc macros
- distributed invalidation
- cluster roles
- single-flight
- generation counters
- stale-while-revalidate
- custom eviction
- persistence
- actor-like control plane

---

## 4. Suggested Crate Shape

Prefer a single crate for v0 unless the current repository already has a crate split.

Suggested simple module shape:

```text
src/
  lib.rs
  cache.rs
  options.rs
  error.rs
  codec.rs
  tag_index.rs
  stats.rs
```

Crate splitting can wait until the first working API is stable.

---

## 5. Public API Target

```rust
pub struct HydraCache { ... }

impl HydraCache {
    pub fn local() -> HydraCacheBuilder;

    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned;

    pub async fn put<T>(&self, key: &str, value: T, options: CacheOptions) -> Result<()>
    where
        T: serde::Serialize;

    pub async fn get_or_load<T, E, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
        E: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>;
}
```

---

## 6. Test Requirements

Cover:

- `put_then_get`
- `get_missing_returns_none`
- `get_or_load_loads_on_miss`
- `get_or_load_uses_cached_value_on_hit`
- `ttl_expires_entry`
- `invalidate_key_removes_one`
- `invalidate_tag_removes_all_tagged`
- `flush_clears_all`
- `stats_track_hits_misses_loads_invalidations`
- `loader_error_is_returned`
- `decode_error_invalidates_bad_entry`

---

## 7. Definition Of Done

v0 implementation is done when:

- all included API exists
- public items have Rustdoc
- examples in docs compile or are marked intentionally
- tests cover the API and error behavior
- `cargo test` passes, if the environment can run it
- no SQLx/distributed/single-flight code is introduced

Current implementation note:

- `cargo test` passes as of the first v0 implementation pass.
- Backend eviction listeners are intentionally not wired in v0, so `CacheStats.evictions` remains zero.
