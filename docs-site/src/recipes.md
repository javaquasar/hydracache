# Recipes

Short, copy-pasteable solutions to common caching problems. Every snippet is compiled and run from `docs-site/examples/src/bin/recipes.rs`:

```powershell
cargo run --manifest-path docs-site/examples/Cargo.toml --bin recipes
```

## Bound Memory and Set a Default TTL

Give the cache a byte budget, reject oversized values, and make sure nothing lives forever by accident.

```rust
{{#include ../examples/src/bin/recipes.rs:bounded-cache}}
```

`max_capacity` is weighted by encoded byte length, not by entry count. Oversized values are rejected and counted in `oversize_rejections`; evictions are counted in `evictions`. For sizing guidance see [Memory Sizing](guides/memory-sizing.md).

## Multi-tenant, Paginated Search

Search results depend on more than the query string. Put **every dimension that changes the result** into the key — tenant, query, page, and permission scope if results are filtered by it — and tag the result with the writes that can make it stale.

```rust
{{#include ../examples/src/bin/recipes.rs:tenant-search}}
```

Why this shape:

- forgetting `tenant` in the key would leak one tenant's results to another;
- forgetting `page` would make every page return the first page;
- a tenant-scoped tag lets one write invalidate *all* cached searches and pages for that tenant without touching other tenants.

See [Keys and Tags](concepts/keys-and-tags.md) for the full checklist.

## Keep Serving When the Database or Upstream Is Down

When the loader fails, you can serve the last known value for a bounded window instead of failing the request:

```rust
{{#include ../examples/src/bin/recipes.rs:stale-if-error}}
```

This is explicit per call, never a silent fallback. For background refresh while serving a slightly stale value, use `stale_while_revalidate`; for refreshing hot keys before they expire, use `refresh_ahead`. See [Refresh and Stale Reads](guides/refresh-and-stale-reads.md).

## Cache an External HTTP API

Any async call can be cached. `cacheable_loader!` names the key, tags, and TTL next to the call; concurrent callers share one upstream request.

```rust
{{#include ../examples/src/bin/recipes.rs:external-api}}
```

For whole functions, the `#[cacheable]` attribute does the same declaratively. See [Cacheable Functions](guides/cacheable-functions.md).

## Warm the Cache at Startup

Preload hot entries so a fresh instance does not send its first burst of traffic straight to the database:

```rust
{{#include ../examples/src/bin/recipes.rs:warm-up}}
```

Use the same key and tag scheme as the read path, so warmed entries are invalidated exactly like loaded ones.

## Check the Hit Ratio

```rust
{{#include ../examples/src/bin/recipes.rs:hit-ratio}}
```

A low hit ratio usually means the key includes something that changes on every request (a timestamp, a request id), the TTL is too short, or values are evicted. `single_flight_joins` shows how many loader calls were saved during miss storms. See [Diagnostics and Events](guides/diagnostics-and-events.md) and the [FAQ](faq.md).

## More Recipes Elsewhere

- Invalidate after commit across instances → [Tutorial](tutorial.md)
- Cache one entity by id with SQLx, Diesel, or SeaORM → [Database Query Caching](guides/database-query-caching.md)
- Share invalidation through Redis or NATS → [Distributed Invalidation](guides/distributed-invalidation.md)
