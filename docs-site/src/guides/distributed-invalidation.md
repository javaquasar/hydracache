# Distributed Invalidation

Use a shared invalidation bus when several cache instances should share invalidation intent.

The bus propagates invalidations, not values. Each cache keeps its own local entries. When one cache invalidates a key, tag, or all entries, peers receive the same intent and remove their local copies.

```rust
{{#include ../../examples/src/bin/distributed_invalidation.rs:invalidation-bus}}
```

Important semantics:

- cached values are never replicated;
- the in-memory bus is best-effort and does not replay messages after restart;
- self-originated messages are ignored to avoid echo loops;
- remote invalidations emit normal events with `CacheEventOrigin::DistributedBus`;
- diagnostics expose published, received, applied, lagged, decode-error, and closed-receiver counters.

## Framed Boundary

`InMemoryFramedInvalidationBus` serializes each message into a `CacheInvalidationFrame` before delivery. It is still in-process, but it exercises the binary boundary that external transports can use later.

```rust
{{#include ../../examples/src/bin/distributed_invalidation.rs:framed-bus}}
```

Use this for transport experiments and compatibility tests. It is not a production Redis, NATS, Postgres, or TCP adapter.

## Custom Transports

External transports implement `CacheInvalidationBus` and return a receiver.

```rust
{{#include ../../examples/src/bin/distributed_invalidation.rs:custom-bus}}
```

Return `Message(...)` for normal delivery, `Lagged(n)` when the transport reports skipped messages, and `Closed` when the stream is no longer usable.
