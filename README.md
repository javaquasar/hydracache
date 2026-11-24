<p align="center">
  <img src="docs-site/src/assets/brand/hydracache-emblem-orange-256.png" width="112" alt="HydraCache logo">
</p>

<h1 align="center">HydraCache</h1>

<p align="center">
  <b>The Rust cache that knows your database.</b><br>
  Embedded, query-result-aware, correctness-first — grows from an in-process cache<br>
  into a distributed cache grid on one codebase.
</p>

<p align="center">
  <a href="https://github.com/javaquasar/hydracache/actions/workflows/ci.yml"><img src="https://github.com/javaquasar/hydracache/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/hydracache"><img src="https://img.shields.io/crates/v/hydracache.svg" alt="crates.io"></a>
  <a href="https://docs.rs/hydracache/latest/hydracache/"><img src="https://docs.rs/hydracache/badge.svg" alt="docs.rs"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License"></a>
</p>

<p align="center">
  <a href="https://javaquasar.github.io/hydracache/getting-started.html">Getting Started</a> ·
  <a href="https://javaquasar.github.io/hydracache/index.html">Docs</a> ·
  <a href="https://docs.rs/hydracache/latest/hydracache/">API</a> ·
  <a href="https://javaquasar.github.io/hydracache/reference/crate-map.html">Crate Map</a>
</p>

---

Most caches are opaque key→blob stores: they know nothing about the database behind
them, so freshness is left to TTL guesses and hand-rolled cache busting. HydraCache
lives **inside your Rust service**, next to `sqlx` / `diesel` / `seaorm`, and makes the
cache contract explicit: what a key identifies, which writes invalidate it, and what
staleness is acceptable when invalidation is late.

## Why HydraCache

- **🔗 Built for SQL-backed services.** Cache query results by entity and collection,
  invalidate them after commit with transaction-aware plans and a transactional
  outbox, and lint SQL dependencies of cache policies at build time
  (`hydracache-sql-lint`). Assisted correctness — not a proxy, not magic.
- **✅ Correctness you can audit.** Single-flight loading, versioned tombstones,
  per-operation consistency levels, fail-loud errors instead of silent degradation,
  and a deterministic seeded fault simulator with invariant checkers — enforced as
  boolean release gates in CI.
- **📈 Embedded → cluster → geo on one API.** Start with an in-process cache. Add
  distributed invalidation, a raft-backed control plane, peer fetch, online
  resharding, and multi-region replication when you need them — opt-in crates that
  leave embedded behaviour unchanged.
- **🦀 Rust-native, zero extra infrastructure to start.** No JVM, no sidecar, no
  separate server for the embedded case. When you do want a server, there is a
  standalone daemon, a client protocol, and an optional Redis RESP facade.

## Quick start

```sh
cargo add hydracache
```

### Local cache

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

    // Concurrent misses for the same key run the loader once (single-flight).
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

### Database query results (SQLx)

```sh
cargo add hydracache hydracache-sqlx
```

```rust
use hydracache::HydraCache;
use hydracache_sqlx::{DbCache, HydraCacheEntity, InvalidationPlan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let cache = HydraCache::local().build();
    let queries = DbCache::new(cache.clone(), "app");

    // Read path: cached by entity identity and tagged with `user:42` + `users`.
    let user = queries
        .for_entity::<User>(42)
        .fetch_with(|| async {
            // sqlx::query_as!(User, "SELECT id, name FROM users WHERE id = $1", 42)
            //     .fetch_one(&pool).await
            Ok::<_, hydracache_sqlx::sqlx::Error>(User { id: 42, name: "Ada".into() })
        })
        .await?;
    assert_eq!(user.name, "Ada");

    // Write path: stage invalidations, commit, then apply.
    // A rollback simply drops the plan and leaves the cache untouched.
    let pending = InvalidationPlan::new().cache_entity::<User>(42);
    // tx.execute(...).await?; tx.commit().await?;
    pending.execute(&cache).await?;

    Ok(())
}
```

Diesel and SeaORM have equivalent adapters. See
[Database Query Caching](https://javaquasar.github.io/hydracache/guides/database-query-caching.html).

## How it fits

```text
  your Rust service
 ┌──────────────────────────────────────────────┐
 │  repository code (sqlx / diesel / seaorm)     │
 │        │ read                    │ write+commit│
 │        ▼                         ▼            │
 │  ┌───────────┐  miss   ┌───────────────────┐  │
 │  │ HydraCache│───────▶ │  loader → SQL DB  │  │
 │  │  (local)  │◀─────── └───────────────────┘  │
 │  └─────┬─────┘  invalidation plan / outbox    │
 └────────┼─────────────────────────────────────┘
          │ opt-in: invalidation bus, peer fetch, raft control plane
          ▼
   other HydraCache members  ·  standalone server  ·  remote clients
```

The cluster layer is developed against a deterministic simulator. Its
[interactive browser demo](https://javaquasar.github.io/hydracache/demo/) runs the real `hydracache-sim` engine and invariant
checkers in WASM, so you can partition, crash, and replay a cluster by seed
([what the demo does and does not model](demo/README.md)).

## Why not …?

| Alternative | When to pick HydraCache instead |
| --- | --- |
| **Redis / Valkey / Dragonfly** | They are fast, DB-agnostic remote stores. Pick HydraCache when you want the cache in-process and its freshness tied to your SQL writes. |
| **moka / foyer** | Excellent embedded caches, but DB-agnostic and local-only. HydraCache adds DB-aware invalidation and a path to a cluster. |
| **Hazelcast / Infinispan / Ignite** | Mature JVM data grids. HydraCache is the Rust-native, embeddable option with explicit, auditable consistency. |
| **ReadySet** | A DB proxy doing incremental view maintenance. HydraCache is a library: no extra network hop, opt-in, explicit invalidation. |

More in [`docs/POSITIONING.md`](docs/POSITIONING.md).

## Pick your level

| You need | Crates |
| --- | --- |
| In-process cache, tags, TTL, single-flight, `#[cacheable]` | `hydracache` |
| Cached DB query results + post-commit invalidation | `hydracache-db`, `hydracache-sqlx` / `hydracache-diesel` / `hydracache-seaorm` |
| Build-time SQL dependency checks | `hydracache-sql-lint` |
| Diagnostics, stats, events, read-only actuator | `hydracache-observability`, `hydracache-actuator-axum` |
| Distributed invalidation and cluster membership | `hydracache-cluster`, `hydracache-cluster-chitchat`, `hydracache-cluster-raft`, `hydracache-transport-nats` / `-redis` |
| Standalone daemon, remote clients, Redis RESP facade | `hydracache-server`, `hydracache-client`, `hydracache-redis-compat` |

Full list: [Crate Map](https://javaquasar.github.io/hydracache/reference/crate-map.html) ·
[Decision Guide](https://javaquasar.github.io/hydracache/decision-guide.html).

## Status

HydraCache is **pre-1.0**: the API is still moving and there is no
semver commitment yet. The distributed core and server packaging exist and are
covered by deterministic simulation, soak, and release gates, but they do not yet
have long production operating history. Some pieces are explicitly early — the
Postgres CDC connector is a stub and the Kubernetes operator is scaffolding.

HydraCache is **not** pitched as a Redis throughput replacement and is **not** a
database: distributed transactions are a permanent non-goal. Performance numbers are
published only with their exact host, scenario, and method — see the
[bare-metal measurement report](docs/testing/perf-scenarios/0.67/results/ax42-reference-0.67.1-20260911.md).

## Learn more

- [Getting Started](https://javaquasar.github.io/hydracache/getting-started.html) →
  [Architecture](https://javaquasar.github.io/hydracache/architecture.html) →
  [Local Cache](https://javaquasar.github.io/hydracache/guides/local-cache.html) →
  [Database Query Caching](https://javaquasar.github.io/hydracache/guides/database-query-caching.html)
- [Production Checklist](https://javaquasar.github.io/hydracache/production-checklist.html) ·
  [Anti-patterns](https://javaquasar.github.io/hydracache/anti-patterns.html)
- Project rules and release plans: [`docs/RULES.md`](docs/RULES.md),
  [`docs/plans/INDEX.md`](docs/plans/INDEX.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the local quality gate and workflow.

## License

Apache-2.0. See [LICENSE](LICENSE).
