# Redis RESP Compatibility

HydraCache `0.63.0` adds an optional RESP edge surface for the cache subset. It is
**Redis protocol compatible for the cache subset, not Redis feature compatible**.
The listener is off by default, translates RESP commands into HydraCache client-surface
operations, and must preserve tenancy, limits, and consistency by going through
`ClientSurfaceState`.

The facade is a standalone Redis endpoint. Redis Cluster is intentionally not
implemented: there are no hash slots, no cluster topology, and no `MOVED` or
`ASK` redirects. Cluster-aware Redis clients must be configured in ordinary
standalone mode when talking to HydraCache.

For implementation-level boundaries and translation notes, see
[`redis-api-implementation-notes.md`](redis-api-implementation-notes.md).

The executable contract is
[`redis_compat_conformance.json`](redis_compat_conformance.json). That manifest is
the source of truth for the docs matrix, translator tests, real Redis oracle
scenarios, client smoke tests, and release-note command table. Do not add a command
to this page without adding or updating the manifest row first.

## Support Levels

| Status | Meaning |
| --- | --- |
| `supported` | HydraCache intends to match Redis RESP behavior for the documented subset row. Oracle tests compare against pinned real Redis versions. |
| `supported_with_caveat` | The command is accepted for client compatibility, but the manifest documents the caveat and normalization rule. |
| `candidate` | The command is not claimed yet. It must either graduate with tests or return a stable unsupported/configuration error. |
| `admin_disabled` | The command is dangerous or administrative and is disabled by default. |
| `hydracache_extension` | The command is HydraCache-only under `HC.*`; real Redis should report it as unknown. |
| `unsupported` | The command is outside the cache subset and must fail loud. |

## Initial Command Matrix

| Command | Status | Oracle rule | Notes |
| --- | --- | --- | --- |
| `PING`, `ECHO`, `QUIT`, `HELLO 2`, `HELLO 3`, `COMMAND` | `supported` | exact or documented normalized metadata | Startup handshake needed by mainstream clients. `HELLO 3` switches the connection to RESP3 for the same cache subset. |
| `AUTH`, `HELLO 2 AUTH` | `supported_with_caveat` | normalized error | Supported for auth-required listeners with Redis-shaped `NOAUTH`/`WRONGPASS`/`OK`, credential redaction, and connection-local authenticated state. Redis ACL categories are not implemented by this row. |
| `rediss://` listener TLS | `supported_with_caveat` | normalized error | Native Redis TLS is supported for the RESP listener when explicitly enabled and backed by server TLS certificate/key material. TLS protects transport; Redis `AUTH` remains the application-layer gate. |
| `CLIENT SETNAME`, `CLIENT SETINFO` | `supported_with_caveat` | normalized error/metadata | Accepted only as bounded, side-effect-free connection metadata. |
| `GET`, bare `SET`, `MGET`, `DEL`, `EXISTS` | `supported` | exact | Counts, nils, and ordering must match real Redis. Bare `SET` means no conditional, return-old-value, retention, or absolute-expiry options. |
| `MSET` | `supported` | exact | Atomic batch write through `ClientSurfaceState`; duplicate keys use Redis last-value-wins ordering. |
| `SET EX/PX`, `SETEX`, `PSETEX`, `EXPIRE`, `PEXPIRE`, `TTL`, `PTTL`, `PERSIST` | `supported` | bounded TTL tolerance | Backed by `hydracache-client-protocol` v3 TTL metadata and client-surface expiry enforcement. `SETEX`/`PSETEX` are normalized to the same `SET EX/PX` path used by mainstream clients such as Jedis. |
| `SET NX PX`, `SET NX EX` | `supported` | bounded TTL tolerance | Narrow single-key Redis lock acquire subset backed by `hydracache-client-protocol` v4 `ConditionalPut IfAbsent`. Success returns `OK`, contention returns nil/null, expired keys are treated as absent, and the mutation is atomic inside `ClientSurfaceState`. `SET NX` without a TTL remains unsupported because the release only claims expiring lock acquire semantics. |
| `EVAL`/`EVALSHA` lock release/extend scripts, `SCRIPT LOAD`/`SCRIPT EXISTS` | `supported_with_caveat` | bounded TTL tolerance / normalized metadata | Only reviewed lock-script fingerprints are accepted: redis-py `Lock` release/extend/reacquire shapes, the simple token-safe release/extend idioms, and `redlock@5.0.0-beta.2` single-resource acquire/extend/release scripts. redis-py default `replace_ttl=False` adds the requested extension to the current remaining TTL; `replace_ttl=True` replaces TTL only when the key is already expiring; persistent/missing keys return `0`. Unknown or changed Lua returns a stable error before mutation. HydraCache does not run general Lua and does not implement Redis script-cache persistence beyond per-listener allowlisted `SCRIPT LOAD` metadata. |
| `SET NX` without TTL, `SET XX`, `SET GET`, `SET KEEPTTL` | `unsupported` | documented divergence | Rejected before dispatch with Redis-shaped errors. HydraCache supports only the expiring `NX` lock-acquire subset; compare-and-return-old-value, retention, and non-expiring conditional writes are outside the 0.63 contract. |
| `SET EXAT`, `SET PXAT` | `unsupported` | documented divergence | Rejected before dispatch with Redis-shaped `ERR syntax error`. These absolute-expiry options are not conditional lock primitives; they are deferred candidates because they need a separate contract for server clock source, past timestamp behavior, overflow, TTL tolerance, and real Redis oracle/client rows. |
| `SELECT 0` | `supported_with_caveat` | normalized error | Accepted as a connection-local no-op for Redis client URL compatibility. HydraCache exposes one logical Redis database only; `SELECT 1` and every non-zero DB index fail loud with `ERR multiple Redis databases are not supported; use SELECT 0`, and invalid indexes return `ERR invalid DB index`. |
| `INFO` | `supported_with_caveat` | normalized metadata | Minimal honest RESP facade facts only: standalone mode, role, HydraCache version, RESP dialects, accepted connection count, processed command count, and RESP error count. No fake Redis memory, keyspace, replication, or cluster sections. |
| `TYPE` | `supported_with_caveat` | exact | Returns `string` for an existing cache value and `none` for a miss. No other Redis data types are claimed. |
| `ROLE`, `DBSIZE`, `SCAN` | `unsupported` | documented divergence | `ROLE` would fabricate Redis replication state, `DBSIZE` would imply an exact tenant/keyspace cardinality contract, and `SCAN` would expose iterable keyspace behavior HydraCache does not provide at this edge. |
| `CONFIG`, `FLUSHDB`, `FLUSHALL` | `admin_disabled` | documented divergence | Recognized but disabled by default. `CONFIG` would imply Redis server configuration read/write support; `FLUSHDB` and `FLUSHALL` are destructive keyspace-wide operations. All return stable `NOPERM` before mutation. |
| `HSET`, `ZADD`, lists, streams, general Lua, transactions, modules | `unsupported` | documented divergence | HydraCache is not a Redis clone; non-subset commands fail loud. The only Lua accepted in 0.63 is the narrow lock-script allowlist above. |
| `CLUSTER SLOTS`, `CLUSTER NODES`, `CLUSTER INFO` | `unsupported` | documented divergence | Standalone-only facade. No hash slots, topology, `MOVED`, or `ASK` are fabricated. |
| `HC.STATS`, `HC.DIAGNOSTICS`, `HC.INVALIDATE` | `hydracache_extension` | HydraCache-only | Must be tenant-scoped and go through HydraCache surfaces. |
| `HC.NAMESPACE`, `HC.TAG`, `HC.SETTAGS`, `HC.INVALIDATE_TAG` | `hydracache_extension` | HydraCache-only | `HC.NAMESPACE` is listener-scoped. Tag metadata is RESP-listener-local, attached only to existing keys, and `HC.INVALIDATE_TAG` invalidates tagged live keys through `ClientSurfaceState`; it does not scan the Redis keyspace or claim Redis Cluster/global tag semantics. |

## Real Redis Oracle

Compatibility scenarios run against pinned Docker `redis-server` versions and the
HydraCache RESP facade. Supported Redis-subset commands compare exact RESP shape,
integer counts, nil/bulk behavior, array order, and atomic `MSET` outcome. Error text may be
normalized by class. TTL values use bounded tolerance because wall-clock remaining time is
time-sensitive. The lock subset compares `SET NX PX/EX` acquire/contention, token-safe release, and
token-safe extension against real Redis with bounded TTL tolerance, including redis-py additive
`extend` semantics and persistent-key `0`/no-mutation behavior. Auth scenarios compare
Redis-shaped `NOAUTH`/`WRONGPASS`/`OK` classes and must never expose credential material in replies,
logs, metrics, or diagnostics.

Unsupported Redis commands are expected to diverge: real Redis may succeed, while
HydraCache returns the documented loud error. `HC.*` commands are HydraCache-only:
real Redis should return unknown command behavior. The heavy client matrix also
exercises the HydraCache-only tag extension path through mainstream clients, but
those commands are not compared for exact behavior against real Redis because
real Redis does not implement them.

Targeted Rust tests are not the final compatibility claim. Before release, the
Docker/client matrix must prove the same supported subset through mainstream
Python, Node, Go, JVM, and Rust Redis clients, and the pinned Redis oracle must
compare the subset against the documented Redis image tags. The 0.63 lock-library claim is limited
to redis-py `Lock` and Node `redlock@5.0.0-beta.2` single-resource rows; Go/JVM lock libraries and
Redisson full locks require their own reviewed script traces before support can be claimed. If those
heavy gates
are not green, release notes must say the implementation has targeted coverage
but ecosystem/oracle proof is still pending.

Redis Cluster is a documented non-goal rather than a partial implementation.
`CLUSTER *` commands return a stable unsupported error, and the facade never
returns topology, hash slot metadata, `MOVED`, or `ASK`.

## Health And Probe Commands

HydraCache keeps Redis health/probe support deliberately small. `PING`,
`HELLO`, `COMMAND`, `CLIENT SETNAME`, and `CLIENT SETINFO` cover mainstream
client startup and liveness. `INFO` is supported only as a minimal honest facade
snapshot, not as a Redis server inventory. Its response may include standalone
mode, `role:master`, HydraCache package version, supported RESP dialects,
accepted connection count, processed command count, and RESP error count. It
does not include memory, database cardinality, replication offsets, Redis
Cluster state, or per-DB keyspace sections.

`TYPE key` is supported for the cache subset and returns only `string` or
`none`. `ROLE`, `DBSIZE`, and `SCAN` remain unsupported because returning
Redis-like replication roles, exact key counts, or iterable keyspace state would
be misleading or unsafe for a tenant-scoped HydraCache facade.

## Admin Commands

`CONFIG`, `FLUSHDB`, and `FLUSHALL` are intentionally `admin_disabled`, not
partially implemented Redis features. `CONFIG` is a Redis server administration
interface for reading and mutating runtime settings such as memory policy,
persistence, TLS, and ACL behavior. HydraCache does not expose those Redis
server internals through the RESP cache facade, so returning a fake config map or
accepting `CONFIG SET` would be wrong-but-green.

`FLUSHDB` and `FLUSHALL` are destructive commands. In Redis, `FLUSHDB` removes
all keys in the selected database and `FLUSHALL` removes all keys in all
databases. HydraCache exposes only `SELECT 0` as a compatibility no-op and does
not expose Redis multi-db or Redis-global server keyspace semantics. A wipe
operation, if added later, must be a HydraCache-native admin API with explicit
tenant/namespace scope, authorization, audit, and rollout gates.

The default RESP facade returns stable `NOPERM ... is disabled by the HydraCache
Redis facade` errors for these commands before dispatch. Tests assert that
`CONFIG GET *` does not fabricate configuration, and that `FLUSHDB`/`FLUSHALL`
leave existing keys intact.

## HydraCache Extension Tag Scope

`HC.NAMESPACE`, `HC.TAG`, `HC.SETTAGS`, and `HC.INVALIDATE_TAG` are
HydraCache-only extension commands rather than Redis commands. `HC.NAMESPACE`
without arguments returns the namespace configured on the RESP listener.
`HC.NAMESPACE <same-name>` returns `OK`; any other namespace fails loud. It is
not Redis multi-db support and does not change the `SELECT 0` contract.

`HC.TAG key tag [tag ...]` attaches tags to an existing live cache key in the
RESP listener's local tag index and returns the number of newly attached tags.
`HC.SETTAGS key tag [tag ...]` replaces that key's local tag set and returns the
number of unique tags now attached. Missing or expired keys return `0` and do
not create metadata. Tags are non-empty UTF-8 strings; keys remain binary-safe
Redis bulk strings.

`HC.INVALIDATE_TAG tag` looks up only keys that were explicitly tagged through
this listener and invalidates live matches through `ClientSurfaceState` using
per-key `ClientRequest::Invalidate`. It returns the number of live keys
invalidated, prunes stale expired/deleted tag entries, and leaves untagged keys
untouched. It is deliberately not implemented as `SCAN pattern -> DEL`, and it
does not claim cross-listener, persisted, Redis Cluster, or HydraCache-core-wide
tag metadata semantics. A future native global tag index can replace this
edge-local path only with its own compatibility entry and tests.

## Executable Examples

Every example below is covered by the `redis_clients` gated target. They use the
supported RESP2/RESP3 cache subset, including atomic `MSET`, TTL commands, the
HydraCache-only tag extension path, the auth-required startup path, and the native `rediss://` startup path. Auth-enabled examples use
`redis://default:<password>@host:port/`; TLS examples use `rediss://default:<password>@host:port/`
with the configured CA. URLs may include `/0`; non-zero database paths must be rejected by clients or
surface the same loud `SELECT` error. The `HC.*` examples are HydraCache-only and
should not be sent to a real Redis server except in divergence/oracle tests.
Cluster clients should use their normal standalone Redis connection path, not a
cluster topology client, because HydraCache does not expose Redis Cluster slots
or redirects.

### redis-cli

Gate: `redis_clients`

```sh
redis-cli -u redis://127.0.0.1:6379/0 SELECT 0
redis-cli -u redis://127.0.0.1:6379/0 INFO
redis-cli -u redis://127.0.0.1:6379/0 SET demo:k v
redis-cli -u redis://127.0.0.1:6379/0 GET demo:k
redis-cli -u redis://127.0.0.1:6379/0 TYPE demo:k
redis-cli -u redis://127.0.0.1:6379/0 MSET demo:a 1 demo:b 2
redis-cli -u redis://127.0.0.1:6379/0 MGET demo:k demo:missing
redis-cli -u redis://127.0.0.1:6379/0 SET demo:ttl v EX 30
redis-cli -u redis://127.0.0.1:6379/0 TTL demo:ttl
redis-cli -u redis://127.0.0.1:6379/0 HC.NAMESPACE
redis-cli -u redis://127.0.0.1:6379/0 HC.TAG demo:k model
redis-cli -u redis://127.0.0.1:6379/0 HC.INVALIDATE_TAG model
redis-cli -u redis://127.0.0.1:6379/0 DEL demo:k demo:missing
redis-cli -u redis://default:secret@127.0.0.1:6379/0 GET demo:k
redis-cli --tls --cacert ca.pem -u rediss://default:secret@127.0.0.1:6379/0 PING
```

### Rust (redis-rs)

Gate: `redis_clients`

```rust
let client = redis::Client::open("redis://127.0.0.1:6379/0")?;
let mut connection = client.get_multiplexed_async_connection().await?;
redis::cmd("SET").arg("demo:k").arg("v").query_async::<()>(&mut connection).await?;
redis::cmd("MSET").arg("demo:a").arg("1").arg("demo:b").arg("2").query_async::<()>(&mut connection).await?;
redis::cmd("SET").arg("demo:ttl").arg("v").arg("EX").arg(30).query_async::<()>(&mut connection).await?;
let ttl: i64 = redis::cmd("TTL").arg("demo:ttl").query_async(&mut connection).await?;
assert!(ttl > 0);
let value: String = redis::cmd("GET").arg("demo:k").query_async(&mut connection).await?;
assert_eq!(value, "v");
let added: i64 = redis::cmd("HC.TAG").arg("demo:k").arg("model").query_async(&mut connection).await?;
assert_eq!(added, 1);
let invalidated: i64 = redis::cmd("HC.INVALIDATE_TAG").arg("model").query_async(&mut connection).await?;
assert_eq!(invalidated, 1);
```

### Python (redis-py)

Gate: `redis_clients`

```python
import redis

r = redis.Redis.from_url("redis://127.0.0.1:6379/0", decode_responses=True)
assert r.set("demo:k", "v") is True
assert r.mset({"demo:a": "1", "demo:b": "2"}) is True
assert r.set("demo:ttl", "v", ex=30) is True
assert r.ttl("demo:ttl") > 0
assert r.get("demo:k") == "v"
assert r.mget(["demo:k", "demo:missing"]) == ["v", None]
```

### Node (node-redis)

Gate: `redis_clients`

```javascript
import { createClient } from "redis";

const client = createClient({ url: "redis://127.0.0.1:6379/0" });
await client.connect();
await client.set("demo:k", "v");
await client.mSet({ "demo:a": "1", "demo:b": "2" });
await client.set("demo:ttl", "v", { EX: 30 });
const ttl = await client.ttl("demo:ttl");
if (ttl <= 0) throw new Error("expected positive TTL");
const value = await client.get("demo:k");
await client.quit();
```

### Go (go-redis)

Gate: `redis_clients`

```go
client := redis.NewClient(&redis.Options{Addr: "127.0.0.1:6379", DB: 0})
if err := client.Set(ctx, "demo:k", "v", 0).Err(); err != nil {
    panic(err)
}
if err := client.MSet(ctx, "demo:a", "1", "demo:b", "2").Err(); err != nil {
    panic(err)
}
if err := client.Set(ctx, "demo:ttl", "v", 30*time.Second).Err(); err != nil {
    panic(err)
}
if ttl, err := client.TTL(ctx, "demo:ttl").Result(); err != nil || ttl <= 0 {
    panic("expected positive TTL")
}
value, err := client.Get(ctx, "demo:k").Result()
```

### JVM (Jedis)

Gate: `redis_clients`

```java
try (Jedis jedis = new Jedis(URI.create("redis://127.0.0.1:6379/0"))) {
  jedis.set("demo:k", "v");
  jedis.mset("demo:a", "1", "demo:b", "2");
  jedis.setex("demo:ttl", 30, "v");
  long ttl = jedis.ttl("demo:ttl");
  if (ttl <= 0) {
    throw new IllegalStateException("expected positive TTL");
  }
  String value = jedis.get("demo:k");
}
```

## Operator Defaults

The RESP listener is disabled by default. Local development may bind it to
`127.0.0.1:6379`, but production examples must require explicit enablement and
explicit port exposure. Do not expose port `6379` on a public load balancer by
default. Auth-required listeners are configured with Redis `AUTH` token material
from a file and may optionally require a username. Native `rediss://` is opt-in and
reuses the server TLS certificate/key material; clients must trust the configured
CA and still authenticate with Redis `AUTH` before cache/data commands. Production
deployments must enable both Redis `AUTH` and TLS before allowing non-loopback access.

The server rejects Redis listener addresses that overlap the public daemon
listener, cluster listener, or enabled admin listener. Disabling the listener is
the rollback default; existing RESP connections are drained or closed through
the daemon drain path, and new RESP connections are refused once the drain gate
closes.

## Logical Database Contract

HydraCache `0.63.0` does not implement Redis multi-db isolation. The RESP edge
uses the configured HydraCache namespace as a single logical Redis database.
`SELECT 0` is accepted as a no-op so mainstream Redis clients and connection
strings such as `redis://host:6379/0` can bootstrap. `SELECT 1`, `SELECT 2`, and
all other non-zero DB indexes fail loud before mutation; a failed `SELECT` never
changes the connection keyspace. Invalid or negative DB indexes return
`ERR invalid DB index`.

## Rollout And Rollback

Canary enablement starts with one edge/daemon, then runs the fast RESP gate,
the pinned real Redis oracle, and the mainstream-client matrix before expanding.
Watch command status labels, unsupported-command rate, TLS handshake failures,
auth/admin-disabled events, memory and file descriptor plateau, p99 command
latency, response-order checks, and any cross-tenant access/audit anomaly.

Rollback triggers are: TLS handshake failures spike unexpectedly, auth failures
spike unexpectedly, unsupported command rate exceeds the migration baseline,
memory or fd usage does not plateau, response-order/pipeline checks fail, p99
latency violates the edge SLO, or audit events indicate wrong tenant scope.
Disable the listener or Redis TLS flag, drain/close existing RESP connections,
preserve logs and fixtures, and keep the conformance manifest row at `candidate`
or `unsupported` until the failed scenario is fixed.

## Adding A Command

1. Update `redis_compat_conformance.json`.
2. Add golden RESP fixture coverage.
3. Add translator tests or unsupported-matrix tests.
4. Add real Redis oracle expectations when the command is part of the Redis subset.
5. Update this page and release notes from the same manifest row.

Commands that cannot satisfy those steps stay `candidate` or `unsupported`.
