# Decision Guide

Use this page when you know the shape of the problem but not the HydraCache API to start with.

| Situation | Start with |
| --- | --- |
| Store or load one typed value in-process | [Local Cache](guides/local-cache.md) |
| Several call sites reuse one value type and namespace | [Typed Cache](guides/typed-cache.md) |
| A normal async function should be cached with less boilerplate | [Cacheable Functions](guides/cacheable-functions.md) |
| A DB/repository result needs explicit query-result caching | [Database Query Caching](guides/database-query-caching.md) |
| A stale value may be acceptable during refresh | [Refresh and Stale Reads](guides/refresh-and-stale-reads.md) |
| You need to prove hit/miss/load behavior | [Diagnostics and Events](guides/diagnostics-and-events.md) |
| Multiple local caches should observe invalidations | [Distributed Invalidation](guides/distributed-invalidation.md) |
| You need role, membership, generation, or owner vocabulary | [Client and Member Cluster](guides/client-member-cluster.md) |

## API Choices

Use `put` and `get` for simple explicit storage.

Use `get_or_load` when the loader can fail and you want the full local cache API.

Use `get_or_insert_with` when the loader cannot fail.

Use `get_or_load_with_refresh` when freshness behavior is a product decision and stale fallback must be visible.

Use `cacheable_loader!` only after the key, tags, TTL, and loader boundary are already obvious.

Use `hydracache-db` when cached values are repository or query results, not arbitrary function results.
