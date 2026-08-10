# Database Query Caching

Use database query caching when a repository result is expensive enough to reuse and the application can describe its identity and invalidation model.

The database layer remains visible. HydraCache does not parse SQL, infer table dependencies, install triggers, or replace SQLx, Diesel, SeaORM, or a repository layer.

```rust
{{#include ../../examples/src/bin/database_query_caching.rs:database-query-caching}}
```

Before enabling a cached query, answer:

1. What exact result does this key identify?
2. Which write paths invalidate the tags?
3. What TTL or stale behavior is acceptable if invalidation is delayed?
4. How will hits, misses, loader calls, and invalidations be observed?

For SQLx, Diesel, and SeaORM integrations, keep the adapter helper small. Drop to `fetch_with` when a transaction, macro-shaped query, or repository method needs more control.

## Policy Macro

Use `query_cache_policy!` when a query has stable metadata and repeated builder calls would obscure the key/tag contract.

```rust
{{#include ../../examples/src/bin/database_query_caching.rs:query-cache-policy}}
```

The macro is still explicit: it names the key source, tags, TTL or preset, and optional refresh behavior. It does not inspect SQL or infer invalidation.
