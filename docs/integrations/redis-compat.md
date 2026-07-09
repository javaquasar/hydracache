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
| `MSET` | `candidate` | candidate | Ships only if atomic; otherwise unsupported-loud. |
| `SET EX/PX`, `EXPIRE`, `PEXPIRE`, `TTL`, `PTTL`, `PERSIST` | `candidate` | TTL tolerance only if shipped | Current client surface needs real TTL application and remaining-TTL metadata before these can be supported. |
| `SELECT` | `candidate` | candidate | Requires explicit database-to-namespace mapping. |
| `INFO`, `ROLE`, `DBSIZE`, `TYPE`, `SCAN` | `candidate` or `unsupported` | candidate or documented divergence | Health probes must be minimal and honest; no fabricated Redis server state. |
| `CONFIG`, `FLUSHDB`, `FLUSHALL` | `admin_disabled` | documented divergence | Disabled by default. |
| `HSET`, `ZADD`, lists, streams, Lua, transactions, modules, `CLUSTER` | `unsupported` | documented divergence | HydraCache is not a Redis clone and does not emit `MOVED` or `ASK`. |
| `HC.STATS`, `HC.DIAGNOSTICS`, `HC.INVALIDATE` | `hydracache_extension` | HydraCache-only | Must be tenant-scoped and go through HydraCache surfaces. |
| `HC.NAMESPACE`, `HC.TAG`, `HC.SETTAGS`, `HC.INVALIDATE_TAG` | `candidate` | candidate | Tag invalidation ships only with a native tag-scoped path; scan-and-loop is forbidden. |

## Real Redis Oracle

Compatibility scenarios run against pinned Docker `redis-server` versions and the
HydraCache RESP facade. Supported Redis-subset commands compare exact RESP shape,
integer counts, nil/bulk behavior, and array order. Error text may be normalized by
class. TTL values may use bounded tolerance only if the TTL metadata gate ships.

Unsupported Redis commands are expected to diverge: real Redis may succeed, while
HydraCache returns the documented loud error. `HC.*` commands are HydraCache-only:
real Redis should return unknown command behavior.

## Adding A Command

1. Update `redis_compat_conformance.json`.
2. Add golden RESP fixture coverage.
3. Add translator tests or unsupported-matrix tests.
4. Add real Redis oracle expectations when the command is part of the Redis subset.
5. Update this page and release notes from the same manifest row.

Commands that cannot satisfy those steps stay `candidate` or `unsupported`.
