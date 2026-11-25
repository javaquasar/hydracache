# Distributed Invalidation

**API:** [`CacheInvalidationBus`](https://docs.rs/hydracache/latest/hydracache/trait.CacheInvalidationBus.html) · [`InMemoryInvalidationBus`](https://docs.rs/hydracache/latest/hydracache/struct.InMemoryInvalidationBus.html) · [`hydracache-transport-redis`](https://docs.rs/hydracache-transport-redis/latest/hydracache_transport_redis/) · [`hydracache-transport-nats`](https://docs.rs/hydracache-transport-nats/latest/hydracache_transport_nats/)

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

`InMemoryFramedInvalidationBus` serializes each message into a `CacheInvalidationFrame` before delivery. It is still in-process, but it exercises the same binary frame boundary that the external transports use.

```rust
{{#include ../../examples/src/bin/distributed_invalidation.rs:framed-bus}}
```

Use this for transport experiments and compatibility tests. For invalidation across processes, use one of the external transports below.

## External Transports

Two opt-in crates carry the same invalidation frames between processes:

| Crate | Transport | Verification |
| --- | --- | --- |
| `hydracache-transport-redis` | Redis via the async Redis client | Redis testcontainers smoke test |
| `hydracache-transport-nats` | NATS via the async NATS client | NATS testcontainers smoke test |

Both implement the `InvalidationTransport` seam and keep the same semantics as the in-memory bus: values are not replicated, and malformed frames are reported as decode errors instead of being silently dropped.

> [!TIP]
> Delivery guarantees are those of the underlying broker. Keep a TTL on entries as a freshness bound for writes whose invalidation can be lost.

## Custom Transports

External transports implement `CacheInvalidationBus` and return a receiver.

```rust
{{#include ../../examples/src/bin/distributed_invalidation.rs:custom-bus}}
```

Return `Message(...)` for normal delivery, `Lagged(n)` when the transport reports skipped messages, and `Closed` when the stream is no longer usable.

## Next Steps

- [Client and Member Cluster](client-member-cluster.md) — roles, membership, and the grid
- [Consistency Model](../concepts/consistency-model.md) — what is and is not guaranteed
