# FAQ and Troubleshooting

## Getting Started

**Do I need to run a server?**
No. The core `hydracache` crate is a library that lives in your process. A server (`hydracache-server`) exists for teams that want one, but the embedded cache, query-result caching, and distributed invalidation all work without it.

**Does HydraCache parse my SQL or intercept queries?**
No. You name the key and the tags; HydraCache never guesses what a query depends on. `hydracache-sql-lint` can check at build time which tables your cache policies depend on, but it does not change runtime behavior. See [What HydraCache Is Not](concepts/what-hydracache-is-not.md).

**Is it production-ready?**
The local cache and query-result caching are the stable core. The cluster layer is staging-ready: it is covered by deterministic simulation, networked tests, and soak runs, but it has limited production history. HydraCache is pre-1.0, so APIs can still change. See the [Production Checklist](production-checklist.md).

**How does it compare with Redis or moka?**
See [Compared to Alternatives](alternatives.md).

## Stale Data

**I updated a row but still read the old value.**
Check, in order:

1. Does the write path invalidate at all? Every write that changes a cached result needs an `InvalidationPlan`, an outbox entry, or an explicit `invalidate_*` call.
2. Does it invalidate **after** commit? Invalidating before commit lets a concurrent reader re-cache the old row.
3. Do the tags match? The write must invalidate a tag that the cached entry actually carries — `HydraCacheEntity` and `query_cache_policy!` exist so both sides derive the same names.
4. Was the write made by another service or by hand? Writes outside your service need their own invalidation path.
5. With several instances: is the invalidation reaching them? Check `distributed_invalidations_published` on the writer and `distributed_invalidations_applied` on the others.

TTL is the last safety net: it bounds how long a missed invalidation can keep a value alive.

**Can HydraCache return a value that was deleted?**
Not by design. Deletes in the grid leave versioned tombstones that are collected only after repair confirms it is safe, so a slow replica cannot resurrect them. In the embedded cache, a deleted entry is simply gone.

## Hit Ratio and Performance

**My hit ratio is low.**
Typical causes: the key contains something unique per request (timestamp, request id, unordered parameters); the TTL is shorter than the time between reads; the cache is too small and evicts (watch `evictions`); or values are rejected as too large (watch `oversize_rejections`). See [Recipes → Check the Hit Ratio](recipes.md#check-the-hit-ratio).

**How do I know single-flight is working?**
Watch `single_flight_joins`. Under a miss storm for one key, it grows while `loads` stays at one per key and expiry.

**Requests spike every time a hot key expires.**
That is expected without refresh. Use `refresh_ahead` to reload hot keys before they expire, or `stale_while_revalidate` to serve the old value while one background load runs. See [Refresh and Stale Reads](guides/refresh-and-stale-reads.md).

**Is HydraCache faster than Redis?**
For in-process reads, there is no network hop at all. Over the optional RESP facade, Redis is faster today, especially with pipelining. See [Performance](performance.md) for what has been measured and how.

## Memory

**How much memory will the cache use?**
Set `max_capacity` (weighted by encoded bytes) and `max_entry_bytes` on the builder. For estimates of retained bytes per entry and per tag, see [Memory Accounting](guides/memory-accounting.md) and [Memory Sizing](guides/memory-sizing.md).

## Clusters

**Do instances share cached values?**
Not in the embedded and client/member modes: each process keeps its own entries and only invalidation intent travels. Value replication is an explicit opt-in of the grid layer.

**What happens if an invalidation message is lost?**
The affected instance keeps its copy until the next invalidation for that key or tag, or until TTL. Lag and decode errors are counted (`distributed_invalidation_lagged`, `distributed_invalidation_decode_errors`), so a degraded transport is visible.

**Does HydraCache support distributed transactions?**
No, and it never will. The ceiling is single-partition atomic invalidation and single-key linearizable conditional writes. See [Consistency Model](concepts/consistency-model.md).
