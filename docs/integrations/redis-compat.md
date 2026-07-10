# Redis RESP Compatibility

HydraCache `0.63.0` adds an optional RESP edge surface for the cache subset. It is
**Redis protocol compatible for the cache subset, not Redis feature compatible**.
The listener is off by default, translates RESP commands into HydraCache client-surface
operations, and must preserve tenancy, limits, and consistency by going through
`ClientSurfaceState`.

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
| `PING`, `ECHO`, `QUIT`, `HELLO 2`, `COMMAND` | `supported` | exact or documented normalized metadata | Startup handshake needed by mainstream clients. |
| `AUTH` | `candidate` | candidate | Identity mapping is release-blocking before auth-required listeners are claimed. |
| `CLIENT SETNAME`, `CLIENT SETINFO` | `supported_with_caveat` | normalized error/metadata | Accepted only as bounded, side-effect-free connection metadata. |
| `GET`, `SET`, `MGET`, `DEL`, `EXISTS` | `supported` | exact | Counts, nils, and ordering must match real Redis. |
| `MSET` | `supported` | exact | Atomic batch write through `ClientSurfaceState`; duplicate keys use Redis last-value-wins ordering. |
| `SET EX/PX`, `EXPIRE`, `PEXPIRE`, `TTL`, `PTTL`, `PERSIST` | `supported` | bounded TTL tolerance | Backed by `hydracache-client-protocol` v3 TTL metadata and client-surface expiry enforcement. |
| `SELECT` | `candidate` | candidate | Requires explicit database-to-namespace mapping. |
| `INFO`, `ROLE`, `DBSIZE`, `TYPE`, `SCAN` | `candidate` or `unsupported` | candidate or documented divergence | Health probes must be minimal and honest; no fabricated Redis server state. |
| `CONFIG`, `FLUSHDB`, `FLUSHALL` | `admin_disabled` | documented divergence | Disabled by default. |
| `HSET`, `ZADD`, lists, streams, Lua, transactions, modules, `CLUSTER` | `unsupported` | documented divergence | HydraCache is not a Redis clone and does not emit `MOVED` or `ASK`. |
| `HC.STATS`, `HC.DIAGNOSTICS`, `HC.INVALIDATE` | `hydracache_extension` | HydraCache-only | Must be tenant-scoped and go through HydraCache surfaces. |
| `HC.NAMESPACE`, `HC.TAG`, `HC.SETTAGS`, `HC.INVALIDATE_TAG` | `candidate` | candidate | Tag invalidation ships only with a native tag-scoped path; scan-and-loop is forbidden. |

## Real Redis Oracle

Compatibility scenarios run against pinned Docker `redis-server` versions and the
HydraCache RESP facade. Supported Redis-subset commands compare exact RESP shape,
integer counts, nil/bulk behavior, array order, and atomic `MSET` outcome. Error text may be
normalized by class. TTL values use bounded tolerance because wall-clock remaining time is
time-sensitive.

Unsupported Redis commands are expected to diverge: real Redis may succeed, while
HydraCache returns the documented loud error. `HC.*` commands are HydraCache-only:
real Redis should return unknown command behavior.

## Executable Examples

Every example below is covered by the `redis_clients` gated target. They use only
the supported RESP2 cache subset, including atomic `MSET` and TTL commands. `SELECT`, RESP3,
`rediss://`, and `HC.*` examples stay out of user-facing docs until their matching gates ship.

### redis-cli

Gate: `redis_clients`

```sh
redis-cli -u redis://127.0.0.1:6379 SET demo:k v
redis-cli -u redis://127.0.0.1:6379 GET demo:k
redis-cli -u redis://127.0.0.1:6379 MSET demo:a 1 demo:b 2
redis-cli -u redis://127.0.0.1:6379 MGET demo:k demo:missing
redis-cli -u redis://127.0.0.1:6379 SET demo:ttl v EX 30
redis-cli -u redis://127.0.0.1:6379 TTL demo:ttl
redis-cli -u redis://127.0.0.1:6379 DEL demo:k demo:missing
```

### Rust (redis-rs)

Gate: `redis_clients`

```rust
let client = redis::Client::open("redis://127.0.0.1:6379/")?;
let mut connection = client.get_multiplexed_async_connection().await?;
redis::cmd("SET").arg("demo:k").arg("v").query_async::<()>(&mut connection).await?;
redis::cmd("MSET").arg("demo:a").arg("1").arg("demo:b").arg("2").query_async::<()>(&mut connection).await?;
redis::cmd("SET").arg("demo:ttl").arg("v").arg("EX").arg(30).query_async::<()>(&mut connection).await?;
let ttl: i64 = redis::cmd("TTL").arg("demo:ttl").query_async(&mut connection).await?;
assert!(ttl > 0);
let value: String = redis::cmd("GET").arg("demo:k").query_async(&mut connection).await?;
assert_eq!(value, "v");
```

### Python (redis-py)

Gate: `redis_clients`

```python
import redis

r = redis.Redis.from_url("redis://127.0.0.1:6379", decode_responses=True)
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

const client = createClient({ url: "redis://127.0.0.1:6379" });
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
client := redis.NewClient(&redis.Options{Addr: "127.0.0.1:6379"})
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
try (Jedis jedis = new Jedis(URI.create("redis://127.0.0.1:6379"))) {
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
default. Use private networking, NetworkPolicy, and the same auth/TLS posture as
other externally reachable client surfaces before allowing non-loopback access.

The server rejects Redis listener addresses that overlap the public daemon
listener, cluster listener, or enabled admin listener. Disabling the listener is
the rollback default; existing RESP connections are drained or closed through
the daemon drain path, and new RESP connections are refused once the drain gate
closes.

## Rollout And Rollback

Canary enablement starts with one edge/daemon, then runs the fast RESP gate,
the pinned real Redis oracle, and the mainstream-client matrix before expanding.
Watch command status labels, unsupported-command rate, auth/admin-disabled
events, memory and file descriptor plateau, p99 command latency, response-order
checks, and any cross-tenant access/audit anomaly.

Rollback triggers are: auth failures spike unexpectedly, unsupported command
rate exceeds the migration baseline, memory or fd usage does not plateau,
response-order/pipeline checks fail, p99 latency violates the edge SLO, or audit
events indicate wrong tenant scope. Disable the listener, drain/close existing
RESP connections, preserve logs and fixtures, and keep the conformance manifest
row at `candidate` or `unsupported` until the failed scenario is fixed.

## Adding A Command

1. Update `redis_compat_conformance.json`.
2. Add golden RESP fixture coverage.
3. Add translator tests or unsupported-matrix tests.
4. Add real Redis oracle expectations when the command is part of the Redis subset.
5. Update this page and release notes from the same manifest row.

Commands that cannot satisfy those steps stay `candidate` or `unsupported`.
