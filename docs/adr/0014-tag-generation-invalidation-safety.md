# ADR-0014: Tag Generation Invalidation Safety

> Status: accepted for 0.4.0.

## Context

HydraCache uses explicit tags for application-controlled invalidation. This is
simple and fits the future database-result cache direction, but it creates a
race when a tagged loader is running at the same time as `invalidate_tag`.

Without an additional guard, an old loader can finish after invalidation and put
stale data back into the cache.

## Decision

HydraCache tracks invalidation generations for tags.

Tagged loads snapshot the current generation of every tag in their
`CacheOptions`. The snapshot is attached to the in-flight load entry.

HydraCache uses this snapshot in two places:

- A caller can join an existing in-flight load only when the generation snapshot
  still matches the caller's current tag generations.
- A completed loader stores its result only when its original generation
  snapshot is still current.

If the snapshot is stale, HydraCache returns the loaded value to the original
caller but does not store it.

`flush` advances a global generation so active loads cannot repopulate the cache
after a full clear.

## Consequences

This prevents write-after-invalidate for tagged local loads.

It also creates a cleaner future boundary for distributed invalidation: remote
invalidation can later become generation advancement plus propagation.

HydraCache does not automatically retry a stale loader. That policy remains
application-controlled for now.

Untagged loads are not protected by tag generations. Applications that need
freshness boundaries should attach tags to cache entries and loader options.
