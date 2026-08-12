# Listener implementation: current state

## Snapshot

- Status: current implementation snapshot, not a forward-looking compatibility claim.
- Snapshot date: 2026-08-12 (0.68 Redis listener addendum).
- Inspected worktree commit: `5388a406b99949e48dc56e45f736826e260053bf`.
- Inspected `origin/main`: `5c1c7f29d3aa36a569b748a1c4996b149ea3d98b`.
- Scope: cache event listeners, distributed invalidations, and the external
  `SubscribeInvalidations` / `SubscribeEntryEvents` surfaces.
- Unreleased addendum branch: `feat/0.68-generated-client-plane-foundation`.

The listener implementation has three distinct layers. Local Rust subscriptions
and callbacks are executable. Distributed invalidations are executable and feed
local cache events on receiving nodes. The external entry-event protocol shape,
admission, and lifecycle accounting exist, but the current HTTP transport does
not yet push event frames to a remote client.

This distinction is normative for this snapshot. A protocol type or an accepted
subscription request does not by itself prove a live remote event stream.

## Status matrix

| Capability | Status | Evidence / boundary |
| --- | --- | --- |
| Local Rust event subscription | Implemented | `HydraCache::subscribe` returns a live `CacheEventSubscriber`. |
| Mutation/access convenience subscriptions | Implemented | `subscribe_mutations` and `subscribe_access`. |
| Exact-key, key-prefix, tag, kind, and origin filters | Implemented | `CacheEventOptions`; filters are evaluated by each subscriber. |
| Callback listener | Implemented | A Tokio background task reads a normal subscription; callbacks do not execute on the cache operation hot path. |
| Bounded local event buffer | Implemented | Tokio broadcast ring, default capacity `1024`, configurable with `event_buffer_capacity`. |
| Slow-listener detection | Implemented | `CacheEventRecvError::Lagged(skipped)` and `event_subscriber_lagged`. |
| Optional latest-event behavior | Implemented | `next_event` skips explicit lag notifications and resumes with available events. |
| Distributed invalidation publication/reception | Implemented | Background invalidation receiver validates generation and applies remote invalidations. |
| Distributed lag/decode/closure diagnostics | Implemented | Dedicated bounded counters in `CacheStats`. |
| Watermark gap-repair decision model | Implemented in protocol/client model | Generation changes clear the partition; sequence gaps cause conservative invalidation. |
| `SubscribeEntryEvents` request/response codec | Implemented | Protocol v2+ request and `Subscribed { from }` response round-trip. |
| Conservative IMap-style event projection | Implemented in protocol model | `Stored -> Upserted`, removal/eviction mappings, and safe invalidation fallback. |
| External identity, tenant quota, and stream-limit admission | Implemented | The external surface admits or rejects the subscription before returning `Subscribed`. |
| External HTTP/2 server-push event stream | **Not implemented** | The transport currently reserves/accounts a modeled subscription and returns an acknowledgement only. |
| Remote stream heartbeat and idle-timeout behavior | **Reserved, not implemented as a live stream** | Limits exist in configuration, but no event-producing streaming loop consumes them. |
| Remote disconnect decrement/re-registration | **Not implemented** | No live connection-owned subscription exists to close or restore. |
| Durable event replay | **Not provided** | Listeners are cache-coherency/diagnostic signals, not a durable log. |
| Exactly-once delivery or global total order | **Not provided** | Consumers must tolerate gaps, duplicates, and concurrent/distributed reordering. |
| Full Hazelcast `IMap.addEntryListener` compatibility | **Not claimed** | Server push, reconnect, re-registration, and wider Hazelcast listener semantics remain missing. |
| Redis keyspace notification stream | Implemented for the 0.68 draft | Off-by-default exact/pattern RESP subscriptions consume a bounded metadata-only shared client-surface mutation bus. See `docs/integrations/redis-keyspace-event-listener.md`. |

## 0.68 Redis keyspace notification addendum

The Redis event listener is deliberately separate from the native cache-event
bus and the HC/2 entry-listener API at the wire boundary, while all three reuse
the same verified mutation state. Redis clients receive only Redis-representable
key metadata on `__keyspace@0__:*` and `__keyevent@0__:*` channels. RESP2 uses
subscription arrays; RESP3 uses push frames. A slow receiver is disconnected
after bounded-bus lag, and no value bytes enter the event signal.

The claim remains node-local and at-most-once. It excludes `PUBLISH`, durable
replay, cross-daemon delivery, values in events, and a general message-broker
surface. The complete operator and compatibility contract is
[`redis-keyspace-event-listener.md`](../integrations/redis-keyspace-event-listener.md).

## Local event bus

Each `HydraCache` instance owns an `EventBus` backed by
`tokio::sync::broadcast`. The builder creates it with the configured capacity;
the default is `1024`, and any configured value is clamped to at least one.

The publication path has two deliberate hot-path protections:

1. Access/load events are not published unless `enable_access_events(true)` was
   set. Mutation and invalidation events remain eligible without that option.
2. An eligible event is not sent when the event bus has no receivers.

Sending into the bounded broadcast channel is synchronous and non-waiting from
the cache operation's perspective. A slow subscriber cannot block a put, get,
remove, expiry, eviction, or invalidation operation.

The event groups are:

- access/load: `Hit`, `Miss`, `SingleFlightJoined`, `LoadStarted`,
  `LoadCompleted`, and `LoadFailed`;
- mutation/invalidation: `Stored`, `Removed`, `KeyInvalidated`,
  `TagInvalidated`, `Flushed`, `StaleLoadDiscarded`, `Expired`, and `Evicted`.

Every local `CacheEvent` carries metadata:

- event kind;
- key, tag, or cache-wide scope;
- origin (`LocalApi`, `Loader`, `SingleFlight`, `Backend`, or
  `DistributedBus`);
- associated tags;
- event timestamp;
- affected-key count when known.

The current native event is metadata-only. `CacheEventValueMode::EncodedBytes`
reserves an API shape for future value delivery, but the current `CacheEvent`
does not carry an encoded value.

## Subscription and callback behavior

`HydraCache::subscribe(options)` creates a receiver with immutable
`CacheEventOptions`. Convenience methods select mutation events, access events,
one physical key, or one tag. The full options support:

- included and excluded event kinds;
- one exact physical key;
- a physical-key prefix;
- one tag;
- one event origin;
- a reserved value mode.

Each subscriber receives from the same bounded event sequence and discards
events that do not match its filters. Dropping a subscriber unregisters it.

`HydraCache::add_listener` and its `on_mutation` / `on_access` shorthands are
callback adapters over an ordinary subscription. They spawn a Tokio task that
waits for matching events and invokes the callback there. Dropping the listener
handle or calling `unsubscribe` aborts that task.

Callbacks are synchronous functions executed by their background task. A long
callback therefore makes that listener fall behind, but it still does not block
the cache operation that produced the event.

## Overflow, delivery, and ordering contract

When a subscriber falls behind the bounded ring, `recv` returns
`CacheEventRecvError::Lagged(skipped)`. The skipped count is also added to
`event_subscriber_lagged`.

Two consumption styles are intentional:

- `recv` exposes a gap so correctness-sensitive cache consumers can repair or
  invalidate conservatively;
- `next_event` skips lag errors and continues to the latest available matching
  signal, which is suitable for dashboards and lightweight callbacks.

The local channel preserves its send sequence for events that a receiver does
not lose. This is not a global operation order: concurrent publishers can
interleave, distributed delivery can be delayed or repeated, and buffer
overflow creates explicit gaps. There is no persistence, acknowledgement,
exactly-once guarantee, or replay of events published before a local subscription
was created.

Consequently, listeners are appropriate for:

- near-cache invalidation and conservative repair;
- local derived-state refresh;
- diagnostics, metrics, and management UI;
- best-effort application callbacks.

They must not be used as an audit log, CDC stream, financial ledger, or source
of truth for irreversible business actions.

## Distributed invalidation path

When configured with an invalidation bus, a cache mutation publishes a
`CacheInvalidationMessage`. Cluster-backed publishers attach their current
source generation after validating that generation against the cluster runtime.

Every participating cache starts a background invalidation receiver. It:

1. ignores messages emitted by its own node id;
2. validates a remote source generation when cluster metadata is available;
3. applies the key, tag, or cache-wide invalidation to the local cache;
4. publishes the resulting local `CacheEvent` with origin `DistributedBus`.

The invalidation bus is also bounded. Lag, decode failures, publish failures,
and unexpected receiver closure are counted instead of being hidden.

Protocol invalidation events can carry a `(source_generation, message_id)`
watermark. `SubscriptionWatermarkTracker` chooses a repair action as follows:

| Observation | Repair action |
| --- | --- |
| First event for a tracker | `ClearPartition` |
| Source generation changed | `ClearPartition` |
| Message-id gap | `InvalidateConservatively` |
| Contiguous, duplicate, or stale message id in the same generation | `Apply` without moving the watermark backwards |

This is a cache-correctness repair mechanism, not durable replay. The public
listener contract explicitly permits coalescing and incomplete history.

## IMap-style entry-event projection

Protocol v2 introduced:

```text
SubscribeEntryEvents {
    ns,
    region,
    from,
    include_value,
    projection,
}
```

The response acknowledges the requested starting watermark with
`Subscribed { from }`. `EntryEvent` has namespace, optional structured key,
conservative kind, optional bytes, residency-degradation marker, and optional
watermark.

The projection intentionally avoids inventing state transitions that the source
signal cannot prove:

| Source signal | Projected kind |
| --- | --- |
| `Stored` | `Upserted` |
| `Removed` | `Removed` |
| `Expired` or `Evicted` | `Evicted` |
| Key/tag/cache invalidation | `Invalidated` |
| Stale load discard or unknown source | `Invalidated` |

An ordinary write is `Upserted`, not fabricated as separate Hazelcast `Added`
or `Updated`. Requested value inclusion is subject to residency and transport
support; if bytes cannot legally or technically be included, the event must
degrade to an invalidation signal and mark that degradation.

## External transport limitation

The current Axum external client surface recognizes both
`SubscribeInvalidations` and `SubscribeEntryEvents`. Before acknowledgement it
performs the normal identity/tenant admission and subscription quota check. It
then increments subscription accounting and returns `Subscribed { from }`.

The `/client/v1/subscriptions` route similarly validates admission, increments
the active count, and returns HTTP `202` with `subscription_reserved`.

Neither path currently owns a live response body or bidirectional connection
that reads the event/invalidation bus and emits framed events. The
`heartbeat_interval_ms`, `idle_timeout_ms`, and stream-drain structures are a
deterministic lifecycle model/reserved contract, not proof of live delivery.
The request's `include_value`, region, and projection fields are decoded but are
not consumed by an event-producing transport loop.

Therefore a remote SDK must not yet expose a successful acknowledgement as a
working `IMap.addEntryListener`. Completing that feature requires at least:

1. a connection-owned bounded outbound queue and framed server-push transport;
2. subscription identity and explicit close/decrement behavior;
3. heartbeat, idle timeout, disconnect, and graceful-drain execution;
4. region and projection filtering on real events;
5. watermark resume plus conservative gap repair;
6. reconnect and listener re-registration in each supported SDK;
7. tests that prove delivery, overflow, disconnect, restart, and repair using a
   real server process and socket.

## Observability

The native cache statistics expose the following relevant counters:

- `events_published`;
- `event_subscriber_lagged`;
- `distributed_invalidations_published`;
- `distributed_invalidations_received`;
- `distributed_invalidations_applied`;
- `distributed_invalidation_lagged`;
- `distributed_invalidation_decode_errors`;
- `distributed_invalidation_publish_failures`;
- `distributed_invalidation_receiver_closed`.

The external surface separately tracks active/reserved subscriptions and tenant
quota admission. In the current modeled transport, that count must not be used
as evidence that event bytes reached a client.

## Executable evidence

The current behavior is covered by these focused tests:

- `crates/hydracache/src/tests/events.rs`
  - access events disabled by default and explicitly enabled;
  - slow subscriber lag without blocking cache operations;
  - latest-event continuation after lag;
  - callback delivery and unsubscribe;
  - key, prefix, tag, kind, and origin filtering.
- `crates/hydracache/tests/native_event_listeners.rs`
  - black-box compilation and execution through exported `HydraCache` and
    `TypedCache` APIs only;
  - filtered local mutation delivery and typed namespace isolation;
  - callback unsubscribe, access-event opt-in, explicit lag, and resume.
- `crates/hydracache/src/invalidation_bus.rs`
  - bounded in-memory receiver lag reporting.
- `crates/hydracache-client-protocol/tests/protocol.rs`
  - region delivery, residency degradation, watermark resume, and gap repair.
- `crates/hydracache-client-protocol/tests/imap_entry_listener.rs`
  - event-kind projection, bounded listener contract, codec round-trip, and
    explicit rejection of business-log semantics.
- `crates/hydracache-client-transport-axum/tests/client_surface.rs`
  - `SubscribeEntryEvents` admission and bounded subscription-family
    acknowledgement. This test does not prove event push.

Recommended focused commands:

```bash
cargo test -p hydracache --locked events
cargo test -p hydracache --test native_event_listeners --locked
cargo test -p hydracache-client-protocol --test protocol --locked
cargo test -p hydracache-client-protocol --test imap_entry_listener --locked
cargo test -p hydracache-client-transport-axum --test client_surface --locked
```

## Maintenance rule

Update this document whenever any of the following changes:

- event buffer or lag policy;
- value-bearing event support;
- distributed invalidation delivery or watermark semantics;
- external subscription acknowledgement behavior;
- live server-push, heartbeat, reconnect, or re-registration;
- the supported Hazelcast listener compatibility claim.

When live remote delivery is implemented, replace the corresponding
`Not implemented` rows only after real-process/socket tests prove the behavior.
Do not infer implementation status from protocol structs or documentation alone.
