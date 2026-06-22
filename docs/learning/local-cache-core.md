# Local Cache Core

## Concept

A local cache stores values inside the application process. It should make the common path fast, predictable, and easy to use.

## Why It Matters For HydraCache

HydraCache must be useful before database adapters or distributed features are enabled. Phase 0 succeeds only if the local cache API is pleasant on its own.

## Current Direction

HydraCache Phase 0 should provide:

- `HydraCache::local()`
- `get`
- `put`
- `get_or_load`
- `invalidate_tag`
- `invalidate_key`
- `flush`
- TTL and tags
- small configuration surface

## Reference Projects

- [Moka reread](../../../moka/MOKA_HYDRACACHE_REREAD.md)
- [Caffeine reread](../../../caffeine/CAFFEINE_HYDRACACHE_REREAD.md)
- [HikariCP reread](../../../hikaricp/HIKARICP_HYDRACACHE_REREAD.md)

## Open Questions

- Should Phase 0 expose a `LocalTypedCache<T>` wrapper?
- Is `Bytes + CacheCodec` ergonomic enough for local-only users?
- Which settings should remain hidden until benchmarks justify them?
