# HydraCache

HydraCache is a Rust-native local async cache that is designed to grow toward database result caching and distributed synchronization later.

## Status

HydraCache is in early development. The current implementation targets the first local-cache release.

## v0 Scope

The first version includes:

- local async cache runtime
- `HydraCache::local()` builder
- `get`
- `put`
- `get_or_load`
- `contains_key`
- per-entry TTL and default TTL
- tag-aware invalidation
- key invalidation
- `remove` as a local-cache alias for key invalidation
- `flush`
- `postcard` codec over `Bytes`
- lightweight stats
- Moka-backed local storage

Out of scope for v0:

- SQLx adapter
- proc macros
- distributed invalidation
- cluster roles
- single-flight
- generation counters
- persistence

## Example

```rust
use std::time::Duration;

use hydracache::{CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

async fn load_user(id: u64) -> Result<User, std::io::Error> {
    Ok(User {
        id,
        name: format!("user-{id}"),
    })
}

# async fn example() -> hydracache::CacheResult<()> {
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
# Ok(())
# }
```

## API Notes

`get` returns `Ok(None)` when the key is missing or expired.

`get_or_load` runs the loader on a miss and stores the loaded value with the provided `CacheOptions`.

`contains_key` checks whether a key currently maps to a usable value. Expired entries are removed and reported as absent.

`remove` and `invalidate_key` both remove one key. `remove` is the shorter local-cache spelling; `invalidate_key` is kept for consistency with tag invalidation.

`invalidate_tag` removes all entries currently associated with the tag.

Use `CacheOptions::tag("users")` for one tag and `CacheOptions::tags(["users", "user:42"])` for multiple tags.

`stats` returns lightweight counters for hits, misses, loads, invalidations, and evictions. v0 does not wire backend eviction listeners yet, so `evictions` remains zero.

## Release Plan

The v0 release plan is maintained here:

- [docs/plans/V0_RELEASE_PLAN.md](docs/plans/V0_RELEASE_PLAN.md)

## Workspace

- `crates/hydracache-core` - core public types, codec, options, errors, stats
- `crates/hydracache` - user-facing local cache runtime
- `crates/hydracache-macros` - future macro ergonomics
- `crates/hydracache-sqlx` - future SQLx-first adapter layer
