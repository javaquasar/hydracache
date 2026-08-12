# Redis keyspace event listener (0.68 draft)

HydraCache 0.68 adds an off-by-default event-listener subset for applications
that already consume Redis keyspace notifications. The listener is implemented
on the existing RESP endpoint and reuses the same verified
`ClientSurfaceState` as HC/1 and HC/2. It does not create a second cache or a
transport-specific mutation path.

This is a compatibility subset of Redis keyspace notifications, not a general
Redis Pub/Sub broker. Redis documents keyspace notifications as disabled by
default and fire-and-forget: a disconnected or lagging subscriber cannot replay
missed messages. HydraCache intentionally keeps those properties. See the
[Redis keyspace notification reference](https://redis.io/docs/latest/develop/pubsub/keyspace-notifications/),
[Redis Pub/Sub delivery contract](https://redis.io/docs/latest/develop/pubsub/),
and [RESP push-frame specification](https://redis.io/docs/latest/develop/reference/protocol-spec/).

## Supported wire contract

The live RESP connection recognizes:

- `SUBSCRIBE channel [channel ...]`;
- `UNSUBSCRIBE [channel ...]`;
- `PSUBSCRIBE pattern [pattern ...]`; and
- `PUNSUBSCRIBE [pattern ...]`.

RESP2 uses the Redis array shapes for subscription acknowledgements and
messages. While subscribed, only subscription commands, `PING`, and `QUIT` are
accepted. RESP3 uses push frames and continues to allow ordinary commands on
the same connection. Pattern subscriptions use binary-safe Redis glob matching
for `*`, `?`, escaped bytes, and bracket classes/ranges.

Only database zero exists. A key mutation can produce these channels:

| Channel | Payload | Example |
| --- | --- | --- |
| `__keyspace@0__:<key>` | event name | channel `__keyspace@0__:user:42`, payload `set` |
| `__keyevent@0__:<event>` | key bytes | channel `__keyevent@0__:set`, payload `user:42` |

The 0.68 subset emits `set`, `del`, `expire`, and `persist` for successful
client-surface transitions. Rejected, conditional-mismatch, read-only, or
wrong-tenant operations do not publish. Values, credentials, tags, and request
bodies are never placed in the event bus.

## Configuration

Both the RESP listener and its event mode are disabled by default:

```powershell
$env:HYDRACACHE_REDIS_API_ENABLED = "true"
$env:HYDRACACHE_REDIS_ADDR = "127.0.0.1:6379"
$env:HYDRACACHE_REDIS_KEYSPACE_EVENTS_ENABLED = "true"
$env:HYDRACACHE_REDIS_MAX_EVENT_SUBSCRIPTIONS_PER_CONNECTION = "64"
$env:HYDRACACHE_REDIS_MAX_EVENT_SUBSCRIPTION_BYTES_PER_CONNECTION = "65536"
```

The two subscription limits are the combined count and combined byte length of
unique exact channels and patterns owned by one connection. Admission is
atomic: a multi-channel request that would exceed either limit retains none of
its additions. Zero and malformed values fail startup. For shared environments,
also require Redis `AUTH` and `rediss://`; authentication is checked before a
subscription is admitted.

## Shared API behavior

RESP, HC/1, and HC/2 listeners built by one `ServerRuntime` share one verified
dispatch state. Consequently, a Redis subscriber can observe a successful
mutation submitted through another enabled client protocol when all of these
conditions hold:

1. both surfaces use the same verified tenant;
2. the mutation targets the RESP listener's configured namespace; and
3. the structured key uses the reversible Redis binary-key representation.

This is node-local in 0.68, matching the existing `redis_scope:node-local`
claim. It is not a cluster-wide event stream. Mutations on another daemon do
not become Redis keyspace notifications unless a later distributed RESP/event
contract explicitly implements and tests that behavior.

The same runtime also projects a deliberately narrow native-backend contract:
a successful `put` through `runtime.cache().typed::<T>("redis")` produces a
Redis `set` notification for the logical typed key. Only physical keys under
the configured `<redis namespace>:` prefix are eligible; the prefix is removed
from the published Redis key. Native keys outside that prefix, non-key events,
and native mutation kinds without an exact Redis name are not projected. The
bridge is metadata-only and does not copy native values into the separate RESP
client-surface store.

The namespace comparison is an exact, case-sensitive `<namespace>:` prefix.
For example, a `backend` listener accepts `backend:user:42` but rejects
`backend-extra:user:42` and `redis:user:42`. Logical UTF-8 keys may contain
additional colons. A notification is not evidence that Redis `GET` can read the
native value: the event is projected, the value is not.

The client-surface and native event buses are independent sources. HydraCache
preserves each source's own publication order, but does not claim one global
order across simultaneous RESP/HC2 and native mutations. A mutation emitted by
one source produces one matching publication path; the mixed-source regression
asserts that separate native and verified-surface writes are delivered once
each without an accidental duplicate.

## Bounds, lag, and observability

Publication is non-blocking. A process-local broadcast ring retains at most
`1024` mutation signals, and each RESP connection retains at most the configured
number and byte length of exact plus pattern subscriptions. A slow socket cannot
block a cache write. If its receiver falls behind the ring, HydraCache sends a
stable lag error, increments `hydracache_lagged_event_subscribers`, and closes
the connection so the client must reconnect and resubscribe.

Unsubscribing the last exact or pattern subscription drops both internal
receivers. Events produced with zero subscriptions are neither retained nor
replayed after a later subscription. A native receiver lag uses the stable
`ERR native cache event subscriber lagged; reconnect and resubscribe` error,
closes the connection, and leaves cache writers non-blocking. A reconnect and
new subscription observes only later events.

`INFO` exposes only bounded counters:

- `hydracache_active_event_subscribers`;
- `hydracache_event_messages`; and
- `hydracache_lagged_event_subscribers`.

No metric label contains a channel, pattern, tenant, key, or value.

## Explicit non-goals

- no `PUBLISH` and no arbitrary application Pub/Sub channels;
- no durable history, acknowledgement, replay, exactly-once delivery, or
  global total order;
- no Redis `CONFIG SET notify-keyspace-events` emulation;
- no fabricated `expired`/`evicted` event when the current client surface
  cannot prove that transition at publication time;
- no values in notifications; and
- no cross-daemon or Redis Cluster event claim.

Consumers must treat notifications as at-most-once cache-coherency hints. They
must repair state with a normal read after reconnect or any detected gap, and
must not use this listener as an audit log, CDC log, queue, or source of truth.

## Executable evidence

Run the focused contract locally:

```powershell
cargo test -p hydracache-client-transport-axum --test mutation_events --locked
cargo test -p hydracache-redis-compat --test redis_event_listeners --locked
cargo test -p hydracache-redis-compat --locked
cargo test -p hydracache-server --test server_lifecycle redis --locked
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets --locked
cargo test -p xtask --test doc_check redis_compat --locked
```

The integration target uses a real TCP listener and the mainstream `redis-rs`
Pub/Sub client. It proves exact and pattern delivery, RESP-to-RESP mutation,
HC/2-shaped shared-surface mutation, disabled/auth behavior, unique-subscription
bounds by count and retained bytes, unsubscribe lifecycle, RESP2 arrays, and
RESP3 push encoding. The server lifecycle target additionally proves that a
native typed-cache backend write is received by both a real `redis-rs`
subscriber and an authenticated `rediss://` subscriber. Native-specific
characterization covers exact subscription, a custom namespace and
prefix-collision refusal, UTF-8 and colon-bearing keys, RESP3 push framing,
metadata-only value-store separation, unsupported-remove suppression,
unsubscribe/resubscribe without replay, bounded lag and recovery, and
concurrent native/verified-source delivery.
The command rows and test names are pinned by
[`redis_compat_conformance.json`](redis_compat_conformance.json).
