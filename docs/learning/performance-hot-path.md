# Performance Hot Path

## Concept

The hot path is the common operation that happens most often. For HydraCache, that is local cache lookup and successful hit return.

## Why It Matters For HydraCache

If cache hits are not cheap, the library loses its purpose. Maintenance, metrics, invalidation indexes, and adapter logic must not make the hit path heavy.

## Current Direction

- Use Moka for local cache internals.
- Keep tag and invalidation bookkeeping above the backend layer.
- Avoid cluster and DB logic in the local hit path.
- Keep Phase 0 configuration small.

## Reference Projects

- [Moka reread](../../../moka/MOKA_HYDRACACHE_REREAD.md)
- [Caffeine reread](../../../caffeine/CAFFEINE_HYDRACACHE_REREAD.md)
- [HikariCP reread](../../../hikaricp/HIKARICP_HYDRACACHE_REREAD.md)

## Open Questions

- What is the target latency budget for a local cache hit?
- Which metrics can be recorded without distorting the hot path?
- Should generation counters affect only writes/inserts, never reads?
