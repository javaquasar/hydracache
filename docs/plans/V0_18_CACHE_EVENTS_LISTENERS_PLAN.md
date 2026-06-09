# HydraCache 0.18.0 Cache Events And Listeners Plan

Status: proposed.

## Goal

Add a first-class cache event/listener model so applications can observe cache
behavior without wrapping every call manually.

The feature should support three use cases:

- Application callbacks: react to invalidation, remove, flush, stale-load
  discard, load completion, and optionally hit/miss activity.
- Observability: expose a structured event stream that can feed actuator,
  sandbox timelines, metrics, and future distributed invalidation.
- Future cluster roles: keep the model compatible with Local, Client, and
  Member modes without forcing distributed runtime dependencies into the local
  cache.

## Hazelcast Inputs

Hazelcast is useful here because it separates public listener ergonomics from
an internal event service. HydraCache should borrow the shape, not the full Java
callback model.

Generated/local docs:

- `../../../hazelcast/HAZELCAST_KNOWLEDGE_BASE.md` notes `EventService` as the
  pub/sub layer for service-internal events.
- `../../../hazelcast/README.md` describes Hazelcast maps as queryable
  key-value stores with event listeners.

Source references:

- `../../../hazelcast/hazelcast/src/main/java/com/hazelcast/core/EntryListener.java`
  groups entry added, updated, removed, evicted, expired, map cleared, and map
  evicted callbacks.
- `../../../hazelcast/hazelcast/src/main/java/com/hazelcast/map/listener/MapListener.java`
  uses a marker interface plus event-specific sub-interfaces.
- `../../../hazelcast/hazelcast/src/main/java/com/hazelcast/map/IMap.java`
  exposes local listeners, key/predicate-filtered listeners, `includeValue`,
  UUID registration IDs, and explicit removal by ID.
- `../../../hazelcast/hazelcast/src/main/java/com/hazelcast/spi/impl/eventservice/EventService.java`
  owns local/global listener registration, deregistration, queue capacity,
  queue size, ordered publish by `orderKey`, and callback execution.
- `../../../hazelcast/hazelcast/src/main/java/com/hazelcast/spi/impl/eventservice/EventRegistration.java`
  represents the internal registration handle.
- `../../../hazelcast/hazelcast/src/main/java/com/hazelcast/core/EntryEvent.java`
  and `../../../hazelcast/hazelcast/src/main/java/com/hazelcast/map/MapEvent.java`
  split single-entry events from map-wide events.

## Ideas To Borrow

- Keep the event bus internal and expose a small public listener/subscription
  API on `HydraCache`.
- Return a registration handle for callback listeners; dropping a stream
  subscription should unsubscribe automatically.
- Separate single-key events from cache-wide events.
- Make values opt-in. Hazelcast has `includeValue`; HydraCache should default
  to metadata-only events to avoid clone, serialization, and data-leak costs.
- Support filters early: event kind, key, key prefix, tag, and origin.
- Keep local-only delivery as the first implementation. Distributed listeners
  can be layered later through a bridge.
- Use bounded queues and expose lag/drop diagnostics. Listener delivery must
  not block cache mutation or loader completion.
- Preserve ordering only where we can state it clearly. Hazelcast orders by
  `orderKey`; HydraCache should initially guarantee per-cache emission order for
  a single process, then add per-key ordering if the distributed bus needs it.
- Treat listener events as observations, not durability. Missed listener events
  must not corrupt cache correctness.

## Non-Goals

- No distributed event delivery in the first release.
- No durable event journal in the core cache.
- No typed value delivery in the first release. Values are encoded bytes inside
  the portable cache model; exposing typed values would require a separate typed
  subscription layer.
- No user callback execution on the hot path.
- No promise that hit/miss events are enabled by default. They are useful for
  demos and diagnostics, but too noisy for some production workloads.

## Proposed Public API

The preferred Rust-first API is a stream-like subscription. Callback listeners
can be implemented as a convenience wrapper over the same bus.

```rust
use hydracache::{CacheEventKind, CacheEventOptions, HydraCache};

let cache = HydraCache::local().build();

let mut events = cache.subscribe(
    CacheEventOptions::new()
        .include_kinds([
            CacheEventKind::Stored,
            CacheEventKind::Removed,
            CacheEventKind::TagInvalidated,
            CacheEventKind::StaleLoadDiscarded,
        ])
        .tag("user:42"),
);

tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        println!("{event:?}");
    }
});
```

Callback adapter:

```rust
use hydracache::{CacheEventOptions, HydraCache};

let cache = HydraCache::local().build();

let listener = cache.add_listener(CacheEventOptions::mutations(), |event| {
    tracing::info!(?event, "cache event");
});

cache.remove_listener(listener.id());
```

The exact callback API can wait until the stream API is stable. The important
part is that both routes use one internal bus.

## Event Model

Add a new public model in `crates/hydracache-core/src/events.rs` and re-export
it from `hydracache`.

```rust
pub enum CacheEventKind {
    Hit,
    Miss,
    SingleFlightJoined,
    LoadStarted,
    LoadCompleted,
    LoadFailed,
    Stored,
    Removed,
    KeyInvalidated,
    TagInvalidated,
    Flushed,
    StaleLoadDiscarded,
    Expired,
    Evicted,
}

pub enum CacheEventScope {
    Key { key: CacheKey },
    Tag { tag: String, affected_keys: usize },
    Cache { affected_keys: Option<usize> },
}

pub enum CacheEventOrigin {
    LocalApi,
    Loader,
    SingleFlight,
    BackendEviction,
    DistributedBus,
}

pub struct CacheEvent {
    pub kind: CacheEventKind,
    pub scope: CacheEventScope,
    pub origin: CacheEventOrigin,
    pub tags: TagSet,
    pub timestamp: SystemTime,
}
```

Value delivery should be designed but not enabled by default:

```rust
pub enum CacheEventValueMode {
    MetadataOnly,
    EncodedBytes,
}
```

For `0.18.0`, prefer `MetadataOnly`. `EncodedBytes` can be added later without
breaking the metadata path.

## Internal Design

Integration points:

- `../../crates/hydracache/src/cache.rs`: emit events after public cache
  operations update cache state and counters.
- `../../crates/hydracache/src/builder.rs`: configure event buffer capacity,
  whether access events are enabled, and default value mode.
- `../../crates/hydracache/src/typed.rs`: delegate subscriptions to the shared
  cache while preserving typed key namespace behavior.
- `../../crates/hydracache/src/stats.rs` and
  `../../crates/hydracache-core/src/stats.rs`: add event counters only if they
  remain cheap and useful.
- `../../crates/hydracache-observability/src/lib.rs`: expose event bus
  diagnostics through probes, not full event payloads.
- `../../crates/hydracache-sandbox/src/lib.rs`: eventually replace some manual
  demo event recording with real cache subscriptions.

Internal bus shape:

```rust
pub(crate) struct EventBus {
    sender: tokio::sync::broadcast::Sender<CacheEvent>,
    access_events: bool,
}
```

Why `broadcast` first:

- Multiple consumers can subscribe independently.
- Dropped receivers unsubscribe by ownership.
- Lag is explicit through `RecvError::Lagged`.
- Publishing can be non-blocking and cheap when there are no receivers.
- It matches the library-first model better than a background dispatcher task.

Open design choice:

- If callback listeners are added in `0.18.0`, implement them as small spawned
  tasks that read from a subscription. Do not execute user closures inline.

## Event Semantics

Mutation and invalidation events:

- `Stored`: after a value is inserted and tag index registration succeeds.
- `Removed`: after explicit key removal completes.
- `KeyInvalidated`: after explicit key invalidation completes.
- `TagInvalidated`: after tag generation advances and affected keys are
  removed.
- `Flushed`: after backend and tag index are cleared.
- `StaleLoadDiscarded`: when generation checks prevent an older load result
  from overwriting a fresher invalidation.

Access and load events:

- `Hit`: optional, emitted on successful local read.
- `Miss`: optional, emitted when no valid entry exists.
- `SingleFlightJoined`: optional, emitted when a caller joins an in-flight load.
- `LoadStarted`: optional, emitted only by the caller that owns the load.
- `LoadCompleted`: optional, emitted after a successful loader result is
  accepted.
- `LoadFailed`: optional, emitted when a loader future returns an error.

Backend events:

- `Expired` and `Evicted` should remain planned until the Moka eviction listener
  is wired into the cache event bus. This is the same area currently called out
  in `../../README.md` for eviction counters.

## Filtering

`CacheEventOptions` should support:

- `include_kinds([...])`
- `exclude_kinds([...])`
- `key(...)`
- `key_prefix(...)`
- `tag(...)`
- `origin(...)`
- `include_access_events(true)`
- `value_mode(CacheEventValueMode::MetadataOnly)`

Filters should be cheap. Start with filtering in the receiver wrapper, not in
the sender. If event volume becomes a problem, move simple kind/tag filters into
the bus.

## Ordering And Backpressure

Initial guarantee:

- Events are emitted in the order the local `HydraCache` instance publishes
  them.
- There is no global ordering across cache instances.
- There is no distributed ordering before the distributed bridge exists.

Backpressure:

- The bus uses a bounded ring buffer.
- Slow subscribers may observe lag.
- Lag should be visible to the subscriber result and optionally counted in
  diagnostics.
- Cache operations must not await subscribers.

Possible future addition:

- `order_key = hash(cache_key)` can preserve per-key ordering if we later move
  from `broadcast` to a striped dispatcher or distributed event bridge.

## Release Scope

Recommended `0.18.0` scope:

- Add `CacheEvent`, `CacheEventKind`, `CacheEventScope`, `CacheEventOrigin`,
  `CacheEventOptions`, and `CacheEventSubscriber`.
- Add `HydraCache::subscribe(options)` and `TypedCache::subscribe(options)`.
- Emit metadata-only events for local mutation, invalidation, stale discard,
  and optional access/load activity.
- Add builder methods:
  - `event_buffer_capacity(usize)`
  - `enable_access_events(bool)`
- Add diagnostics counters for `events_published` and subscriber lag if they are
  cheap to maintain.
- Add documentation examples and README section.
- Add sandbox integration only if it is small; otherwise keep it as the first
  follow-up after the core API is stable.

Defer:

- Distributed listener delivery.
- Durable event journal.
- Typed value events.
- Predicate-like filters over decoded values.
- Callback listener API if it complicates the first release.
- Moka eviction listener bridge if it requires backend restructuring.

## Test Plan

Core/unit tests:

- Subscribing before `put` receives `Stored`.
- `remove` emits `Removed` only when the key existed.
- `invalidate_key` emits `KeyInvalidated`.
- `invalidate_tag` emits `TagInvalidated` with affected key count.
- `flush` emits `Flushed`.
- Stale load race emits `StaleLoadDiscarded`.
- Optional access events emit `Miss`, `LoadStarted`, `LoadCompleted`, and `Hit`
  in the expected order.
- Single-flight emits one owner load sequence and join events for waiters when
  access events are enabled.
- Filters by kind, key, key prefix, tag, and origin suppress unrelated events.
- Dropping a subscriber does not prevent later cache operations.
- Slow subscriber lag is surfaced without failing cache operations.
- `TypedCache::subscribe` observes events from typed operations with the same
  shared cache semantics as `stats` and `diagnostics`.

Integration/documentation tests:

- README event subscription example compiles.
- `cargo test --doc -p hydracache` covers the public event API.
- Sandbox tests should be added only after sandbox starts consuming real cache
  events.

Concurrency tests:

- Concurrent `get_or_insert_with` for the same key publishes one accepted load.
- Concurrent invalidation during load publishes stale discard when the load is
  rejected.
- Event publication must not hold `TagIndex` write locks while notifying
  subscribers.

## Documentation Work

- Add a README section: "Observe cache behavior with events".
- Add crate-level examples in `crates/hydracache/src/lib.rs`.
- Add rustdoc to every public event type.
- Add a short note in `docs/TESTING.md` about event listener tests and lag
  behavior.
- Add a release note `docs/releases/0.18.0.md` once implementation starts.

## Risks

- Event volume can become high if hit/miss events are enabled by default. Keep
  them opt-in.
- Including values can leak sensitive query results and add clone costs. Keep
  metadata-only as the default.
- User callbacks can block cache work if executed inline. Use stream
  subscriptions or spawned callback adapters.
- Event ordering can be overpromised. State local-only ordering clearly.
- Adding too much API at once can freeze poor names. Prefer stream API first,
  callback adapter second.

## Success Criteria

- Applications can subscribe to local cache events with a small, obvious API.
- Existing cache behavior and public exports remain compatible.
- All new public types have rustdoc examples where practical.
- New code is covered by unit, concurrency, and doc tests.
- The design keeps a clean path to future Client/Member distributed listener
  delivery.
