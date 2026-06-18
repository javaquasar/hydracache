# HydraCache Compatibility Register

This file tracks durable and wire-visible artifacts whose versions matter during
rolling upgrades. Runtime-only Rust types are intentionally out of scope unless
they are persisted or transmitted across processes.

## Versioned Artifacts

| Artifact | Current Version | Writer | Reader Compatibility | Failure Mode |
| --- | --- | --- | --- | --- |
| `CacheInvalidationFrame` | `1` | `hydracache` invalidation bus publishers | Readers accept version `1` only. Unknown versions are rejected before apply. | Decode error is reported and the receiver continues. |
| `hydracache_invalidation_outbox` schema | `1` | `hydracache-db` outbox writers or application SQL writers | Workers accept schema version `1`. Unknown future versions must fail loud before draining. | Worker refuses to start; intent is left durable and pending. |

## Upgrade Rules

- Writers may not emit a newer durable or wire artifact until readers in the
  deployment explicitly support it.
- Unknown future schema versions fail closed. A worker must not silently drain a
  table it does not understand.
- Unknown wire versions are treated as decode errors, not panics.
- Forward-only migrations must be idempotent: applying the same migration twice
  leaves the artifact at the same version.

## 0.37 Baseline

`0.37.0` starts this register with the existing invalidation frame and the new
database invalidation outbox schema. Later cluster releases should append raft
log format, replicated value record format, and public client protocols here
before claiming rolling-upgrade compatibility.
