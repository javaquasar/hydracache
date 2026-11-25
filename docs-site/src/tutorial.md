# Tutorial: Cache a SQL Service Across Two Instances

This tutorial builds the core HydraCache flow end to end, in one runnable program:

1. a repository that caches query results by entity;
2. a write path that invalidates **after commit**;
3. two service instances that share invalidation, so a write on one is seen by the other.

It uses SQLx with in-memory SQLite and an in-memory invalidation bus so it runs anywhere without Docker. The same code works with Postgres and with the Redis or NATS transports; the last section shows what to swap.

The complete program is checked in at `docs-site/examples/src/bin/tutorial_two_instances.rs`. Run it from the repository root:

```powershell
cargo run --manifest-path docs-site/examples/Cargo.toml --bin tutorial_two_instances
```

## 1. Dependencies

```powershell
cargo add hydracache hydracache-sqlx serde --features serde/derive
cargo add tokio --features macros,rt-multi-thread
```

`hydracache-sqlx` re-exports the matching `sqlx` crate, so you do not have to align versions by hand.

## 2. Describe the Cached Entity

`HydraCacheEntity` gives the type a stable cache identity: entity `user`, collection `users`, and an id field. HydraCache derives keys and tags such as `user:42` and `users` from it, so the read path and the write path cannot drift apart.

```rust
{{#include ../examples/src/bin/tutorial_two_instances.rs:entity}}
```

## 3. The Repository

Each service instance owns a database pool and its own in-process cache. `DbCache` wraps that cache with query-result helpers.

```rust
{{#include ../examples/src/bin/tutorial_two_instances.rs:repository}}
```

Two things to notice:

- **Read path.** `for_entity::<User>(id).fetch_with(...)` returns the cached value on a hit. On a miss it runs the loader once, even if many requests miss at the same time (single-flight), and stores the result tagged with `user:42` and `users`.
- **Write path.** The `InvalidationPlan` is built *before* the transaction and executed *after* `commit()`. If the update fails or the transaction rolls back, `?` returns early, the plan is dropped, and the cache is untouched. Invalidating before commit would let a concurrent reader re-cache the old row.

## 4. Database and Two Instances

```rust
{{#include ../examples/src/bin/tutorial_two_instances.rs:database}}
```

Both instances point at the same database and join the same invalidation bus with distinct node ids:

```rust
{{#include ../examples/src/bin/tutorial_two_instances.rs:instances}}
```

## 5. Reads Hit the Cache

```rust
{{#include ../examples/src/bin/tutorial_two_instances.rs:read}}
```

Each instance loads the row once. The next 20 reads cost zero database queries.

## 6. A Write on One Instance Reaches the Other

```rust
{{#include ../examples/src/bin/tutorial_two_instances.rs:write}}
```

Instance A commits the update and executes its plan. The invalidation removes `user:42` locally and is published on the bus; instance B receives it (the event origin is `DistributedBus`) and drops its own copy. Values themselves never travel over the bus — each instance reloads from the database on its next read.

## 7. Going to Production

| In the tutorial | In production |
| --- | --- |
| `sqlite::memory:` | your Postgres or MySQL pool; the repository code is unchanged |
| `InMemoryInvalidationBus` | `hydracache-transport-redis` or `hydracache-transport-nats`, one connection per process |
| `InvalidationPlan` after commit | the same, or a transactional outbox when the invalidation must survive a crash between commit and publish |
| no TTL | add a TTL as an upper bound on staleness if an invalidation is ever lost |

Then add observability: hits, misses, loader calls, and distributed invalidations published and applied are all exposed as counters. See [Diagnostics and Events](guides/diagnostics-and-events.md).

## Where Next

- [Consistency Model](concepts/consistency-model.md) — exactly what this setup guarantees and what it does not.
- [Database Query Caching](guides/database-query-caching.md) — query policies, collections, and the policy macro.
- [Distributed Invalidation](guides/distributed-invalidation.md) — transports and their semantics.
- [Production Checklist](production-checklist.md) — before you ship.
