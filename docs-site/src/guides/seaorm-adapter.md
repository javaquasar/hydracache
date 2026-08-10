# SeaORM Adapter

Use `hydracache-seaorm` when SeaORM remains the query authority and HydraCache should wrap only the query-result boundary.

```rust
{{#include ../../examples/src/bin/seaorm_adapter.rs:seaorm-adapter}}
```

Use `fetch_with` when a SeaORM query, transaction, or repository function does not fit the convenience helper shape.
