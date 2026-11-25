# Refresh and Stale Reads

**API:** [`RefreshOptions`](https://docs.rs/hydracache/latest/hydracache/struct.RefreshOptions.html) · [`HydraCache`](https://docs.rs/hydracache/latest/hydracache/struct.HydraCache.html)

> [!NOTE]
> **At a glance**
>
> - **Use when:** a bounded stale value is acceptable for this product path.
> - **Crate:** `hydracache` (`RefreshOptions`).
> - **Guarantees:** stale values are served only inside the configured window and only where the call opts in — never as an implicit fallback.
> - **Watch:** `loads`, `misses`, and the refresh events in diagnostics.
> - **Pitfalls:** stale windows longer than the product tolerates; using stale reads to hide a missing invalidation.

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

## Next Steps

- [Recipes: keep serving when upstream is down](../recipes.md#keep-serving-when-the-database-or-upstream-is-down) — stale-on-loader-error in practice
- [Diagnostics and Events](diagnostics-and-events.md) — confirm which path runs
