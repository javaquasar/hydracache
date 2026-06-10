# HydraCache 0.19.0 Distributed Invalidation Bus Plan

Status: implemented.

## Goal

Add the first synchronization layer for multiple local HydraCache instances
without introducing a daemon, proxy, cluster membership, or external service.

The release should let applications wire a shared invalidation bus so local
cache instances can keep freshness aligned when one instance calls:

- `invalidate_key`
- `remove`
- `invalidate_tag`
- `flush`

The bus propagates invalidation intent only. It does not replicate values.

## Design

The runtime exposes a small transport abstraction:

- `CacheInvalidation`
- `CacheInvalidationMessage`
- `CacheInvalidationBus`
- `CacheInvalidationReceiver`
- `InMemoryInvalidationBus`

`HydraCacheBuilder::shared_invalidation_bus(...)` attaches a shared bus to a
cache instance. `HydraCacheBuilder::invalidation_node_id(...)` gives the cache a
stable id for diagnostics and echo suppression.

When a cache publishes a local invalidation, the message includes its node id.
Every receiving cache ignores messages from its own node id. Remote messages are
applied locally with `CacheEventOrigin::DistributedBus` and are not republished
to the bus.

## Guarantees

- Local-only caches pay no bus cost.
- Bus delivery invalidates keys/tags/flushes, but never sends cached values.
- Self-originated invalidation messages are ignored to prevent echo loops.
- Remote invalidations emit normal cache events with `DistributedBus` origin.
- Diagnostics expose published, received, and applied bus invalidation counters.

## Non-Goals

- No external transport in this release.
- No durability or replay after process restart.
- No cluster membership, discovery, partitioning, or ownership model.
- No distributed value replication.
- No read/write actuator admin endpoints.

## Sandbox

`POST /demo/distributed/invalidation/run` creates two temporary cache nodes on
one `InMemoryInvalidationBus`, then verifies:

- tag invalidation propagates from source to target
- key invalidation propagates even when the source did not hold the key
- flush propagates to the target
- target events use `origin = distributed-bus`
- source/target diagnostics expose bus counters

## Tests

Runtime tests cover:

- tag invalidation propagation
- key invalidation propagation from a source miss
- flush propagation
- echo-loop suppression
- typed cache physical-key invalidation over the shared bus

Sandbox tests cover:

- OpenAPI path and schema registration
- the distributed invalidation endpoint response
- remote event labels and bus diagnostics

## Future Work

Likely follow-up releases can add transport crates without changing the runtime
API shape:

- Postgres LISTEN/NOTIFY invalidation bus
- Redis Pub/Sub invalidation bus
- NATS or Kafka-backed invalidation bus
- explicit `Local`, `Client`, and `Member` runtime roles
