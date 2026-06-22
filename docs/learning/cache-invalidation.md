# Cache Invalidation

## Concept

Invalidation is the act of removing cached values when the source of truth changes.

## Why It Matters For HydraCache

HydraCache intentionally avoids pretending that cache freshness is automatic. Phase 0 uses explicit tags and TTL. Later phases add distributed invalidation and optional generation counters.

## Current Direction

Phase 0:

- local `invalidate_tag`
- local `invalidate_key`
- TTL as safety net
- no cross-process guarantees

Phase 1:

- optional generation counters for DB result caching
- single-flight to reduce duplicate loads

Phase 2:

- distributed invalidation bus
- `ClusterMode::Client`

## Important Failure Mode

Invalidation/load race:

1. A cache miss starts loading a value.
2. Another task invalidates the relevant tag.
3. The first load finishes and writes the old value into the cache.

Generation counters can detect this and skip storing the stale value.

## Reference Projects

- [ReadySet reread](../../../readyset/READYSET_HYDRACACHE_REREAD.md)
- [Groupcache reread](../../../groupcache/GROUPCACHE_HYDRACACHE_REREAD.md)
- [Olric reread](../../../olric/OLRIC_HYDRACACHE_REREAD.md)

## Open Questions

- Should generation counters be enabled by default for query adapters?
- Should tag invalidation be synchronous only, or support async acknowledgement modes in cluster mode?
