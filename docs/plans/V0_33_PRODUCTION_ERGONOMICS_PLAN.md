# HydraCache 0.33.0 Production Ergonomics Plan

## Goal

`0.33.0` turns the now-aligned database adapters into a more production-ready
cache toolkit. The release focuses on two user-facing outcomes:

- make common cache policies easy to choose without hand-building every TTL,
  key, and tag pattern from scratch;
- add safe stale/refresh behavior so applications can trade perfect freshness
  for resilience when a loader or database is slow or temporarily failing.

The release may break the fresh `0.32.x` API where a cleaner public model is
worth it. HydraCache is still pre-`1.0`, and no external users need migration
compatibility yet.

## Product Direction

HydraCache should feel useful at three levels:

- local cache/memoization with explicit keys and tags;
- database query result caching through SQLx, Diesel, SeaORM, or custom
  repositories;
- optional production diagnostics, cluster synchronization, and sandbox demos.

`0.33.0` strengthens the first two levels without hiding control. Users should
be able to start with a preset and later drop down to explicit configuration
when the freshness model becomes more precise.

## Public API Shape

Policy presets should be named after intent, not implementation detail:

```rust
use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::read_mostly()
    .for_entity("user", 42);

let policy = QueryCachePolicy::short_lived()
    .collection("users:active");

let policy = QueryCachePolicy::negative_cache()
    .key("user:not-found:42");
```

Local cache stale behavior should remain explicit at the call site:

```rust
use std::time::Duration;

use hydracache::{CacheOptions, HydraCache, RefreshOptions};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();

let value = cache
    .get_or_load_with_refresh(
        "user:42",
        CacheOptions::new().ttl(Duration::from_secs(60)),
        RefreshOptions::new()
            .stale_while_revalidate(Duration::from_secs(300))
            .serve_stale_on_loader_error(true),
        || async { Ok::<_, std::io::Error>("fresh-user".to_owned()) },
    )
    .await?;
# let _ = value;
# Ok(())
# }
```

Database adapters should expose the same stale behavior through the shared
database-neutral descriptor, so SQLx, Diesel, SeaORM, and custom repositories do
not drift:

```rust
use std::time::Duration;

use hydracache_db::{DbCache, QueryCachePolicy, RefreshPolicy};

# async fn example(queries: DbCache) -> hydracache_db::Result<()> {
let user = queries
    .cached_with::<String>(
        QueryCachePolicy::read_mostly()
            .key("user:42")
            .tag("user:42"),
    )
    .refresh_policy(
        RefreshPolicy::new()
            .stale_while_revalidate(Duration::from_secs(300))
            .serve_stale_on_loader_error(true),
    )
    .load(|| async { Ok::<_, std::io::Error>("fresh-user".to_owned()) })
    .await?;
# let _ = user;
# Ok(())
# }
```

## Implementation Steps

1. Add this plan and a `0.33.0` release note shell.
2. Add database policy presets:
   `short_lived`, `read_mostly`, `per_entity`, `no_ttl_explicit_invalidation`,
   and `negative_cache`.
3. Add local refresh/stale options with tests for:
   fresh hit, normal miss, expired stale return, background refresh,
   stale-on-loader-error, loader-error-without-stale, and stale window expiry.
4. Thread refresh policies through `hydracache-db` descriptors and the SQLx,
   Diesel, and SeaORM adapters. The behavior must be shared through
   `DbQuery`, not reimplemented separately per adapter.
5. Tighten the database adapter ergonomics and docs:
   all examples should show presets first, explicit policies second, and
   engine-specific miss execution last.
6. Add production guidance:
   what to cache, what not to cache, choosing keys/tags, choosing presets,
   stale safety, invalidation race expectations, and operational metrics.
7. Add memory/allocation notes and low-risk cleanup where possible, especially
   around repeated tag cloning and release verification artifacts.
8. Add a soft target cleanup script that removes generated release/check
   directories without deleting the whole `target` directory.
9. Update README, crate READMEs, generated rustdoc examples, sandbox docs,
   testing docs, and release notes.
10. Bump workspace versions to `0.33.0`, run the release gate, package all
    publishable crates, and run the crates.io consumer check after publishing.

## Non-Goals

- Preserve old `0.32.x` naming where a better `0.33.0` API is clearer.
- Add SQL parsing, transparent ORM instrumentation, or automatic query-key
  extraction.
- Add CDC, database triggers, or replication-log based invalidation.
- Add a production distributed storage engine.
- Hide loader execution behind generated code only; explicit loaders remain the
  lowest-level control surface.

## Verification Checklist

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo test -p hydracache --locked`
- `cargo test -p hydracache-db --locked`
- `cargo test -p hydracache-sqlx --locked`
- `cargo test -p hydracache-diesel --locked`
- `cargo test -p hydracache-seaorm --locked`
- `cargo test -p hydracache-sandbox --locked`
- `cargo test --workspace --all-targets --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --doc --workspace --locked`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`
- `.\scripts\package-publishable.ps1 -Set bootstrap -AllowDirty`
- `.\scripts\verify-crates-io-consumer.ps1 -Version 0.33.0`
