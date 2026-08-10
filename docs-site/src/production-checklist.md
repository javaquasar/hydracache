# Production Checklist

Use this checklist before a cached path moves beyond experimentation.

## Cache Boundary

- The key names exactly one reusable value.
- Tenant, locale, permission, feature flag, pagination, and sort dimensions are present when they affect the value.
- The cached type is serializable and stable enough for the chosen cache lifetime.
- The loader boundary is small enough to review.

## Invalidation

- Every write path that can make the value stale has an explicit invalidation plan.
- Entity reads have entity tags.
- Collection or search reads have collection/query-family tags.
- Transactional writes invalidate only after commit.
- Tag names are treated as domain API, not throwaway strings.

## Freshness

- TTL is a fallback bound, not the only freshness model for mutable data.
- Refresh/stale behavior is explicitly accepted by the product path.
- Stale fallback windows are bounded.
- Loader errors are observable when stale fallback is allowed.

## Operations

- Hits, misses, loads, invalidations, and stale load discards are observable.
- Diagnostics are checked in local smoke tests.
- High-volume access events are enabled only when needed.
- Distributed invalidation is tested with two cache instances before adding real transport.

## Documentation

- The example lives under `docs-site/examples` when it appears in public docs.
- The Markdown includes the checked snippet instead of copying code by hand.
- Link checks and visual smoke checks pass before publishing.
