# SQLx Adapter

Use `hydracache-sqlx` when SQLx remains the query authority and HydraCache owns only the cache boundary.

The adapter re-exports `DbCache`, policy types, `HydraCacheEntity`, and `query_cache_policy!` for SQLx users. It does not replace SQLx macros, pools, transactions, or row mapping.

```rust
{{#include ../../examples/src/bin/sqlx_adapter.rs:sqlx-adapter}}
```

Use `fetch_with` when SQLx macros, transactions, or repository functions need to stay in charge. Use `sqlx_one`, `sqlx_optional`, and `sqlx_all` for small pool-like helper calls.
