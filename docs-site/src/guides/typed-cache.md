# Typed Cache

**API:** [`TypedCache`](https://docs.rs/hydracache/latest/hydracache/struct.TypedCache.html)

> [!NOTE]
> **At a glance**
>
> - **Use when:** several call sites share one value type and one domain namespace.
> - **Crate:** `hydracache`.
> - **Guarantees:** a view over the same cache: storage, stats, events, single-flight, and invalidation are shared; keys are prefixed with the namespace.
> - **Watch:** the underlying cache's counters.
> - **Pitfalls:** namespaces that mirror modules instead of domains; hiding TTLs and tags behind the view.

Use a typed cache view when several call sites share one value type and one domain namespace.

```rust
{{#include ../../examples/src/bin/typed_cache.rs:typed-cache}}
```

The typed view does not create separate storage. It is a namespaced view over the same `HydraCache` runtime, so stats, events, invalidation safety, and single-flight behavior remain shared.

The namespace prefixes keys. In the example, the typed key `42` is stored as `profiles:42` in the underlying cache. Use namespaces that match domain boundaries rather than module names.

Typed views are most useful when they remove repeated annotations without hiding cache semantics. Keep keys, tags, and TTLs visible at the call site.

## Next Steps

- [Cacheable Functions](cacheable-functions.md) — cache whole async functions
- [Database Query Caching](database-query-caching.md) — typed repository results
