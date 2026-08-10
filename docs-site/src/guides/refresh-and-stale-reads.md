# Refresh and Stale Reads

TTL says when a value expires. Refresh options say what the cache may do around expiry.

Use refresh behavior only when stale fallback is acceptable for the product path. It should be visible in code review.

```rust
{{#include ../../examples/src/bin/refresh_stale.rs:refresh-stale}}
```

`stale_while_revalidate` lets the cache return a recently expired value while a background refresh runs. Once the stale window expires, callers return to foreground loading.

Related choices:

- `refresh_ahead` refreshes a value before expiry while still serving the fresh cached value;
- `stale_while_revalidate` serves an expired value inside a bounded window and refreshes in the background;
- `stale_on_loader_error` can return an expired value when the refresh loader fails.

Use diagnostics to confirm which path is happening in practice.
