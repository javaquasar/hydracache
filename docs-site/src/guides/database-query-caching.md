# Database Query Caching

**API:** [`DbCache`](https://docs.rs/hydracache-db/latest/hydracache_db/struct.DbCache.html) · [`DbQuery`](https://docs.rs/hydracache-db/latest/hydracache_db/struct.DbQuery.html) · [`query_cache_policy!`](https://docs.rs/hydracache-db/latest/hydracache_db/macro.query_cache_policy.html) · [`InvalidationPlan`](https://docs.rs/hydracache-db/latest/hydracache_db/struct.InvalidationPlan.html) · [`HydraCacheEntity`](https://docs.rs/hydracache-db/latest/hydracache_db/derive.HydraCacheEntity.html)

> [!NOTE]
> **At a glance**
>
> - **Use when:** a repository or query result is reusable and you know which writes make it stale.
> - **Crate:** `hydracache-db` (or an adapter crate).
> - **Guarantees:** explicit key and tags per query; the database library keeps owning SQL, transactions, and row mapping.
> - **Watch:** `hits`, `loads`, `invalidations`.
> - **Pitfalls:** keys without tenant, permission, filter, or page; invalidating before commit.

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

## Next Steps

- [Invalidation After Commit](invalidation-after-commit.md) — `InvalidationPlan` vs the transactional outbox
- [Tutorial: Two Instances](../tutorial.md) — invalidate after commit, end to end
- [SQLx Adapter](sqlx-adapter.md) — adapter-specific helpers
