# Diesel Adapter

**API:** [`hydracache-diesel`](https://docs.rs/hydracache-diesel/latest/hydracache_diesel/)

Use `hydracache-diesel` when Diesel remains the query authority and cached values should still use HydraCache keys, tags, TTL, and diagnostics.

Diesel is synchronous, so Diesel helpers run the supplied loader on `tokio::task::spawn_blocking`. The loader should own or acquire its connection inside the closure.

```rust
{{#include ../../examples/src/bin/diesel_adapter.rs:diesel-adapter}}
```

Use `fetch_with` when custom repository code, transactions, or an async Diesel wrapper needs more control.

## Next Steps

- [Database Query Caching](database-query-caching.md) — policies and invalidation
- [Tutorial: Two Instances](../tutorial.md) — the same flow with SQLx
