# HydraCache

[![CI](https://github.com/javaquasar/hydracache/actions/workflows/ci.yml/badge.svg)](https://github.com/javaquasar/hydracache/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/hydracache.svg)](https://crates.io/crates/hydracache)
[![docs.rs](https://docs.rs/hydracache/badge.svg)](https://docs.rs/hydracache/latest/hydracache/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

<img src="docs-site/src/assets/brand/hydracache-emblem-orange-256.png" width="96" alt="HydraCache logo">

HydraCache is an embedded Rust cache toolkit for explicit cache semantics: local caching, typed cached values, single-flight loading, tag invalidation, cacheable async functions, database result caching, and local-first distributed invalidation.

The library is intentionally application-facing. It does not try to parse SQL, infer table dependencies, replace Redis, or hide freshness policy behind a framework. Application code names the key, tags, TTL, refresh behavior, and loader boundary.

## Start Here

- Public documentation: <https://javaquasar.github.io/hydracache/index.html>
- Public docs source: [`docs-site`](docs-site/)
- Local preview: `mdbook serve docs-site --hostname 127.0.0.1 --port 3000`
- GitHub repository: <https://github.com/javaquasar/hydracache>
- crates.io: <https://crates.io/crates/hydracache>
- API docs: <https://docs.rs/hydracache/latest/hydracache/>

Recommended reading path:

1. [`Getting Started`](https://javaquasar.github.io/hydracache/getting-started.html)
2. [`Architecture`](https://javaquasar.github.io/hydracache/architecture.html)
3. [`Decision Guide`](https://javaquasar.github.io/hydracache/decision-guide.html)
4. [`Local Cache`](https://javaquasar.github.io/hydracache/guides/local-cache.html)
5. [`Database Query Caching`](https://javaquasar.github.io/hydracache/guides/database-query-caching.html)

## Quick Example

```rust
use std::time::Duration;

use hydracache::{CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

async fn example() -> hydracache::CacheResult<()> {
    let cache = HydraCache::local().build();

    let user = cache
        .get_or_load(
            "user:42",
            CacheOptions::new()
                .ttl(Duration::from_secs(60))
                .tag("users")
                .tag("user:42"),
            || async {
                Ok::<_, std::io::Error>(User {
                    id: 42,
                    name: "Ada".to_owned(),
                })
            },
        )
        .await?;

    assert_eq!(user.name, "Ada");

    cache.invalidate_tag("user:42").await?;
    Ok(())
}
```

## What Is Included

- Local async cache runtime with typed serialization boundaries.
- Per-entry TTL, default TTL, tags, key invalidation, tag invalidation, and flush.
- Single-flight miss deduplication for same-key loaders.
- `TypedCache<T>` namespaced views.
- `cacheable_loader!`, `cacheable_infallible!`, and `#[cacheable]` function caching helpers.
- Database-neutral query result caching through `hydracache-db`.
- SQLx, Diesel, and SeaORM adapter crates.
- Diagnostics, stats, event subscriptions, and read-only Axum actuator support.
- In-process invalidation bus and embedded client/member cluster vocabulary.
- Optional chitchat, raft, and Axum peer-fetch cluster adapter crates.

For crate selection, see [`Crate Map`](https://javaquasar.github.io/hydracache/reference/crate-map.html).

## Quality Gate

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
```

For the documentation site:

```powershell
cargo fmt --manifest-path docs-site/examples/Cargo.toml --check
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets --locked
mdbook build docs-site
```

See [`Quality Gate`](https://javaquasar.github.io/hydracache/reference/quality-gate.html) and [`Publishing Docs`](https://javaquasar.github.io/hydracache/reference/publishing-docs.html).

## Project Notes

HydraCache is in early development. The current public documentation lives in `docs-site`; older long-form plans and release notes remain under `docs/` as project history and operational detail.

The Medium article series is linked from the docs home page as background, but the docs site is intended to be self-contained.
