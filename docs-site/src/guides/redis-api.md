# Redis API

HydraCache can expose an optional Redis-compatible RESP edge for clients that already speak Redis.

This surface is a compatibility facade over HydraCache client-surface commands. It is not a promise that HydraCache is a full Redis replacement, and it does not add SQL/query semantics. Use it when Redis client interoperability is useful but the application still wants HydraCache ownership of cache data, TTLs, tags, diagnostics, and invalidation intent.

## Crates

| Crate | Role |
| --- | --- |
| `hydracache-redis-compat` | RESP2/RESP3 parsing, command translation, Redis-style responses, resource limits, and HydraCache extension commands. |
| `hydracache-server` | Optional TCP listener that serves the Redis RESP facade in the standalone daemon. |

Application crates that only use the embedded `HydraCache` runtime do not need these crates.

## Enable The Listener

The server disables the Redis facade by default. Enable it explicitly and keep it on a separate address from HTTP, admin, and cluster listeners.

```powershell
$env:HYDRACACHE_REDIS_API_ENABLED = "true"
$env:HYDRACACHE_REDIS_ADDR = "127.0.0.1:6379"
```

For authenticated local or staging use:

```powershell
$env:HYDRACACHE_REDIS_AUTH_REQUIRED = "true"
$env:HYDRACACHE_REDIS_AUTH_USERNAME = "default"
$env:HYDRACACHE_REDIS_AUTH_TOKEN_FILE = "C:\secrets\hydracache-redis-token.txt"
```

For `rediss://`, enable server TLS and the Redis TLS facade together:

```powershell
$env:HYDRACACHE_TLS_ENABLED = "true"
$env:HYDRACACHE_TLS_CERT_PATH = "C:\certs\server.crt"
$env:HYDRACACHE_TLS_KEY_PATH = "C:\certs\server.key"
$env:HYDRACACHE_TLS_CA_PATH = "C:\certs\ca.crt"
$env:HYDRACACHE_REDIS_REDISS_ENABLED = "true"
```

HydraCache rejects `rediss` startup without complete TLS material.

## Supported Shape

The facade targets Redis string/cache-client interoperability:

- connection and introspection basics such as `PING`, `ECHO`, `HELLO`, `AUTH`, `CLIENT SETNAME`, `COMMAND`, `INFO`, `SELECT`, and `TYPE`;
- string and key operations such as `GET`, `SET`, `MGET`, `MSET`, `DEL`, and `EXISTS`;
- TTL operations such as `EXPIRE`, `PEXPIRE`, `PERSIST`, `TTL`, and `PTTL`;
- lock-oriented `EVAL`, `EVALSHA`, `SCRIPT LOAD`, and `SCRIPT EXISTS` for the supported compare-value lock scripts;
- HydraCache extension commands such as `HC.STATS`, `HC.DIAGNOSTICS`, `HC.INVALIDATE`, `HC.NAMESPACE`, `HC.TAG`, `HC.SETTAGS`, and `HC.INVALIDATE_TAG`.

Unsupported Redis data structures or broad server features should be treated as outside the facade. For example, hash commands are not the goal of this surface.

## Client Examples

Any Redis client can use the facade for the supported command subset.

```text
PING
SET user:42 Ada PX 60000
GET user:42
MSET user:43 Grace user:44 Linus
MGET user:42 user:43 user:44
TTL user:42
DEL user:44
```

HydraCache-specific tags are available through extension commands:

```text
HC.SETTAGS user:42 user:42 users
HC.INVALIDATE_TAG users
HC.STATS
HC.DIAGNOSTICS
```

Use tags when Redis-facing clients should participate in the same invalidation model as native HydraCache code.

## Boundaries

The Redis API is best for interoperability, migration seams, smoke tests, and operational probes. Prefer the native Rust API when application code can call HydraCache directly because native calls preserve typed values, loader boundaries, single-flight behavior, and compile-time policy structure.

Review these boundaries before enabling the facade:

- RESP resource limits protect frame size, array length, bulk string size, read buffer size, and idle connections.
- AUTH is optional but should be required for any shared environment.
- The listener address must not conflict with HTTP, cluster, or admin addresses.
- Redis keys map into the Redis namespace of the HydraCache client surface.
- Redis clients send bytes, not typed Rust values; typed decoding remains a native API concern.

Use Redis compatibility to meet existing clients where they are. Use HydraCache semantics to decide what should be cached and invalidated.
