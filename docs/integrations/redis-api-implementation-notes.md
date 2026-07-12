# HydraCache Redis API Implementation Notes

HydraCache exposes an optional Redis RESP edge API for clients that already know
how to speak Redis. This API is a compatibility facade over HydraCache cache
operations; it is not a Redis server implementation and does not expose Redis as
HydraCache's internal storage engine.

The command support table and release claim live in
[`redis-compat.md`](redis-compat.md). The executable source of truth is
[`redis_compat_conformance.json`](redis_compat_conformance.json). This document
explains the implementation choices behind that contract.

## High-Level Shape

The Redis facade is implemented by `hydracache-redis-compat` and wired into the
optional `hydracache-server` RESP listener. The listener is disabled by default.
When enabled, it parses RESP2 or RESP3 frames, authenticates connection-local
Redis credentials when configured, and translates accepted commands into
HydraCache client-surface operations.

The facade deliberately goes through `ClientSurfaceState` instead of bypassing
the public client path. That keeps tenant scope, request limits, deadlines,
state mutation rules, auditability, and protocol-version checks aligned with the
normal HydraCache client API.

## Data Model

The Redis-compatible data model is a cache-subset model:

- keys are Redis bulk-string bytes;
- values are opaque byte strings stored as HydraCache cache entries;
- supported reads and writes preserve Redis nil, integer-count, and array-order
  behavior for the claimed subset;
- TTL support is mapped onto HydraCache client protocol v3 expiry metadata;
- Redis lock acquire/release/extend support is mapped onto protocol v4
  conditional value operations;
- Redis hashes, sorted sets, lists, streams, modules, transactions, pub/sub, and
  general Lua are intentionally outside this facade.

Because HydraCache is a cache library rather than a Redis process, commands that
would require global Redis server state must either be narrowly implemented with
a stated caveat or fail loud before mutation.

## Command Translation Rules

`GET`, bare `SET`, `MGET`, `MSET`, `DEL`, and `EXISTS` translate to ordinary
HydraCache get/put/invalidate paths. `MSET` is handled as an atomic batch at the
client surface: either the validated batch is applied as one mutation group or
the command fails before partial writes.

`SET EX/PX`, `SETEX`, `PSETEX`, `EXPIRE`, `PEXPIRE`, `PERSIST`, `TTL`, and
`PTTL` use protocol v3 expiry operations. TTL results are compared with bounded
tolerance in oracle tests because remaining wall-clock time is inherently
time-sensitive.

`SET NX PX/EX` is the intentionally narrow Redis lock-acquire subset. It uses
protocol v4 `ConditionalPut IfAbsent`, so success returns Redis `OK`,
contention returns Redis nil/null, expired keys are treated as absent, and the
write is atomic inside the client surface. `SET NX` without TTL, `SET XX`,
`SET GET`, and `SET KEEPTTL` remain outside the claimed subset.

`EVAL` and `EVALSHA` do not run arbitrary Lua. Only reviewed lock release and
extension script shapes are accepted. Unknown scripts, changed library scripts,
wrong arity, wrong `KEYS`/`ARGV` mapping, or non-string arguments return stable
errors before mutation. `SCRIPT LOAD` and `SCRIPT EXISTS` are metadata helpers
for the same allowlist, not a general Redis script cache.

## Protocol And Connection Behavior

The facade supports RESP2 and RESP3 for the same command subset. `HELLO 2` and
`HELLO 3` negotiate the connection dialect; unsupported RESP3 aggregate command
forms are rejected before dispatch. The parser enforces frame and bulk limits so
malformed, oversized, or truncated input cannot mutate cache state.

Redis `AUTH` is connection-local and maps to the configured listener token. It
returns Redis-shaped `NOAUTH`, `WRONGPASS`, and `OK` classes and must redact
credential material from replies, logs, metrics, and diagnostics. Redis ACLs are
not implemented.

Native `rediss://` is a transport option for the RESP listener when explicitly
enabled with server TLS material. TLS and Redis `AUTH` are independent gates:
TLS protects the connection, while `AUTH` authorizes Redis cache commands.

## Single Database, Standalone Endpoint

HydraCache exposes one logical Redis database. `SELECT 0` is accepted as a
connection-local no-op for mainstream client URL compatibility. Non-zero
databases fail loud with a stable multiple-database error, and invalid database
indexes return an invalid-index error.

Redis Cluster is intentionally not implemented. The facade does not compute hash
slots, does not maintain Redis cluster topology, does not answer `CLUSTER *`,
and never emits `MOVED` or `ASK`. Cluster-aware Redis clients must be configured
in standalone mode when targeting HydraCache.

## Honest Probe And Admin Posture

Probe commands are limited to facts HydraCache can state honestly. `PING`,
`HELLO`, `COMMAND`, `CLIENT SETNAME`, `CLIENT SETINFO`, minimal `INFO`, and
cache-subset `TYPE` are supported with the caveats documented in
[`redis-compat.md`](redis-compat.md). `INFO` reports facade facts such as
standalone mode, role, HydraCache version, supported RESP dialects, accepted
connections, processed commands, and RESP errors. It does not fabricate Redis
memory, keyspace, persistence, replication, or cluster sections.

`CONFIG`, `FLUSHDB`, and `FLUSHALL` are recognized but disabled by default.
They return stable `NOPERM` errors before dispatch because they would otherwise
pretend to expose Redis server configuration or destructive Redis-global
keyspace behavior. A future destructive operation should be a HydraCache-native
admin API with explicit tenant/namespace scope, authorization, audit, and rollout
gates.

## HydraCache Extension Commands

`HC.NAMESPACE`, `HC.TAG`, `HC.SETTAGS`, and `HC.INVALIDATE_TAG` are
HydraCache-only RESP extension commands. They are not Redis commands and should
not be compared against real Redis as supported behavior.

The tag implementation is listener-local and in-memory. Tags are attached only
to existing live keys through the RESP edge, and `HC.INVALIDATE_TAG` invalidates
those live keys through `ClientSurfaceState`. It does not scan the Redis
keyspace, does not claim cross-listener persistence, and does not expose Redis
Cluster or global HydraCache tag metadata semantics.

## Testing Contract

Redis compatibility is not claimed from translator unit tests alone. The 0.63
contract is anchored by:

- the conformance manifest at
  [`redis_compat_conformance.json`](redis_compat_conformance.json);
- fast translator and server listener tests for supported and unsupported
  command behavior;
- protocol v3/v4 compatibility tests for TTL and lock conditional operations;
- Docker-gated real Redis oracle rows against pinned Redis images;
- mainstream client matrix rows for Rust, Python, Node, Go, and JVM clients;
- explicit documented-divergence rows for unsupported, admin-disabled, and
  HydraCache-only commands.

When the manifest, docs, release notes, and tests disagree, the release is not
ready. New Redis API behavior must first be added to the manifest with support
level, oracle rule, and test anchors, then implemented through the client
surface, and finally documented in the compatibility guide.
