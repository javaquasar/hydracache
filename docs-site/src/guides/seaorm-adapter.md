# SeaORM Adapter

**API:** [`hydracache-seaorm`](https://docs.rs/hydracache-seaorm/latest/hydracache_seaorm/)

> [!NOTE]
> **At a glance**
>
> - **Use when:** SeaORM is your query layer.
> - **Crate:** `hydracache-seaorm`.
> - **Guarantees:** SeaORM keeps owning entities, connections, and transactions; the cache boundary stays explicit.
> - **Watch:** the same counters as [Database Query Caching](database-query-caching.md).
> - **Pitfalls:** invalidating before `tx.commit()`.

Use `hydracache-seaorm` when SeaORM remains the query authority and HydraCache should wrap only the query-result boundary.

```rust
{{#include ../../examples/src/bin/seaorm_adapter.rs:seaorm-adapter}}
```

Use `fetch_with` when a SeaORM query, transaction, or repository function does not fit the convenience helper shape.

## Next Steps

- [Database Query Caching](database-query-caching.md) — policies and invalidation
- [Tutorial: Two Instances](../tutorial.md) — the same flow with SQLx
