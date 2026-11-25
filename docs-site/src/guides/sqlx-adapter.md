# SQLx Adapter

**API:** [`hydracache-sqlx`](https://docs.rs/hydracache-sqlx/latest/hydracache_sqlx/)

> [!NOTE]
> **At a glance**
>
> - **Use when:** SQLx is your query layer.
> - **Crate:** `hydracache-sqlx`.
> - **Guarantees:** SQLx keeps owning macros, pools, and transactions; HydraCache owns only the cache boundary.
> - **Watch:** the same counters as [Database Query Caching](database-query-caching.md).
> - **Pitfalls:** mixing the re-exported `sqlx` with a different `sqlx` version in the same crate.

Use `hydracache-sqlx` when SQLx remains the query authority and HydraCache owns only the cache boundary.

The adapter re-exports `DbCache`, policy types, `HydraCacheEntity`, and `query_cache_policy!` for SQLx users. It does not replace SQLx macros, pools, transactions, or row mapping.

```rust
{{#include ../../examples/src/bin/sqlx_adapter.rs:sqlx-adapter}}
```

Use `fetch_with` when SQLx macros, transactions, or repository functions need to stay in charge. Use `sqlx_one`, `sqlx_optional`, and `sqlx_all` for small pool-like helper calls.

## Next Steps

- [Tutorial: Two Instances](../tutorial.md) — a complete SQLx read and write path
- [Database Query Caching](database-query-caching.md) — policies and invalidation
