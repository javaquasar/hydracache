# Cache Semantics

HydraCache treats cache behavior as application semantics, not as a hidden map lookup.

A production cache entry needs a contract:

- **key**: the exact result identity;
- **tags**: invalidation handles for writes that can make the result stale;
- **TTL**: a fallback freshness bound;
- **loader**: the backing operation used on misses;
- **events and stats**: evidence that the cache avoids work and invalidates the right values.

The key mistake HydraCache tries to prevent is answering a different question than the caller asked. A key like `users:active` may be fine for a toy app, but it is unsafe when tenants, principals, filters, policy versions, locale, region, or feature flags change the result.

Use explicit cache semantics first. Add convenience only after the manual shape is clear.

## Why a Plain Map Is Not Enough

A map answers one narrow question:

```text
does this key currently have a value?
```

A production cache has to answer more:

```text
is this value still safe for this caller, tenant, query shape, and write history?
```

That difference is why HydraCache exposes explicit options instead of treating every cache operation as a simple `get` and `insert`.

## Review Shape

When reviewing a cached operation, prefer code that makes the decision visible:

- the key is named in domain terms;
- the tags correspond to write paths;
- the TTL is a fallback, not the only correctness mechanism;
- the loader boundary is clear;
- invalidation happens after successful writes.

The API can become more ergonomic over time, but the semantics should remain visible.
