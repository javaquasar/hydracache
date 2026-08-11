# Local Cache

Use the local cache when the current process can safely reuse a value without contacting a backing source.

The local cache is the full-control API. You choose:

- the key;
- the TTL and refresh behavior;
- the invalidation tags;
- the loader boundary;
- the typed value stored at that boundary.

```rust
{{#include ../../examples/src/bin/local_cache.rs:local-cache}}
```

This guide intentionally keeps the example small:

- `put` stores a typed value;
- `get` reads the same typed value;
- `get_or_load` avoids repeated loader calls;
- tag invalidation removes related entries after writes.

Production code should give keys and tags names that match the domain model. A key identifies one cached value. A tag identifies a group of values that a write can make stale.

## Typed Namespaces

`typed::<T>("namespace")` creates a typed, namespaced view over the same cache.

Use it when several call sites work with the same value type and domain namespace. The view keeps shared storage, stats, single-flight, tags, and invalidation safety, but it removes repeated type annotations at call sites and prefixes keys with the namespace.

## Refresh Behavior

TTL says when a value expires. Refresh behavior says what the cache may do around expiry.

Use explicit refresh options when a production path can tolerate a recently expired value while a background refresh runs. Keep this choice visible in code review because stale fallback is a product decision, not a storage detail.

## Where To Go Next

- Use [Cacheable Functions](cacheable-functions.md) when ordinary async functions need the same explicit cache boundary with less boilerplate.
- Use [Local Cache API](../reference/local-cache-api.md) as a compact reference for local runtime methods.
