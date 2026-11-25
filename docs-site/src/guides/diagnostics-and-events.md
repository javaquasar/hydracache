# Diagnostics and Events

**API:** [`CacheStats`](https://docs.rs/hydracache/latest/hydracache/struct.CacheStats.html) · [`CacheEvent`](https://docs.rs/hydracache/latest/hydracache/struct.CacheEvent.html) · [`hydracache-observability`](https://docs.rs/hydracache-observability/latest/hydracache_observability/)

> [!NOTE]
> **At a glance**
>
> - **Use when:** you need to prove what the cache is doing.
> - **Crate:** `hydracache`, `hydracache-observability`, `hydracache-actuator-axum`.
> - **Guarantees:** counters are always on; access events are opt-in; listeners never slow cache operations.
> - **Watch:** everything in `CacheStats`.
> - **Pitfalls:** enabling access events at high volume without a consumer; slow subscribers that lag.

Use diagnostics to prove cache behavior locally before adding more infrastructure. The simplest useful check is: call the same cached operation twice, then inspect hit, load, and request counters.

```rust
{{#include ../../examples/src/bin/diagnostics_events.rs:diagnostics}}
```

The first call misses and runs the loader. The second call hits the cache and avoids the fallback loader value.

## Event Streams

Use event streams when an application needs to observe cache behavior without wrapping every cache call manually.

```rust
{{#include ../../examples/src/bin/diagnostics_events.rs:tag-events}}
```

Mutation and invalidation events are published when subscribers exist. Hit, miss, and load events are higher volume, so they are opt-in with `enable_access_events(true)`.

```rust
{{#include ../../examples/src/bin/diagnostics_events.rs:access-events}}
```

Subscribers use a bounded buffer. Slow subscribers can lag, but cache operations do not wait for listeners.

## Callback Listeners

Callback listeners are useful for lightweight integration points. Keep the returned handle alive for as long as the listener should be active.

```rust
{{#include ../../examples/src/bin/diagnostics_events.rs:callbacks}}
```

HydraCache builds owned event payloads only when an event kind is enabled and at least one active subscriber can receive it.

## Next Steps

- [FAQ and Troubleshooting](../faq.md) — read the counters to diagnose problems
- [Memory Accounting](memory-accounting.md) — retained-byte estimates
