# HydraCache Compatibility Register

This file tracks durable and wire-visible artifacts whose versions matter during
rolling upgrades. Runtime-only Rust types are intentionally out of scope unless
they are persisted or transmitted across processes.

## Versioned Artifacts

| Artifact | Current Version | Writer | Reader Compatibility | Failure Mode |
| --- | --- | --- | --- | --- |
| `CacheInvalidationFrame` | `1` | `hydracache` invalidation bus publishers | Readers accept version `1` only. Unknown versions are rejected before apply. | Decode error is reported and the receiver continues. |
| `hydracache_invalidation_outbox` schema | `1` | `hydracache-db` outbox writers or application SQL writers | Workers accept schema version `1`. Unknown future versions must fail loud before draining. | Worker refuses to start; intent is left durable and pending. |
| `hydracache_hook_schema` schema | `1` | `hydracache-db` generated hook installers | Reconciliation expects version `1` for installed hook plans. Missing or mismatched rows report drift. | Staging/release gates can fail before silently trusting disabled or stale hooks. |
| `RaftLogStore` in-memory format | `1` | `hydracache-cluster-raft` metadata runtime | 0.41 tests cover append/replay, snapshot recovery, suffix truncation, and compaction guard semantics. Future durable engines must register their own format before rollout. | Runtime fails loud on store errors; unknown future durable formats must refuse startup. |
| HTTP replication/peer encoded-value transport | `1` | `hydracache-cluster-transport-axum` clients | Strict routes require `x-hydracache-wire-version: 1`; mismatches are rejected before payload apply. | Route returns upgrade-required style safe rejection; counters can record wire-version failures. |
| `DurableRaftLogStore` format | `1` | `hydracache-cluster-raft` durable-log feature | Readers accept format `1` and refuse unknown future versions before opening a store. | Store open fails loud; no committed command is acknowledged from an unknown format. |
| `ReplicatedValueRecord` durable format | `1` | `hydracache` durable-values feature | Readers accept format `1`; records carry partition, version, epoch, and value/tombstone state. | Unknown future formats must refuse startup before serving replicated values. |
| `ControlPlaneSnapshot` format | `1` | `hydracache` self-heal snapshot helpers | Readers accept format `1` and refuse unknown future versions before restore. | Restore fails loud before rebuilding topology from an unsupported snapshot. |

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

## 0.38 Correctness Automation

`0.38.0` adds hook-schema compatibility tracking and reconciliation drift
reports. These reports are assisted-mode guardrails: they make missing hook
schema rows, mismatched hook versions, outbox backlog, and dead-lettered rows
visible to CI/staging gates. They do not make HydraCache a transparent DB proxy
and do not remove the need to install hooks/outbox migrations in the database.

## 0.41 Grid Slice

`0.41.0` registers the first distributed-grid durable and wire-visible seams:
`RaftLogStore` format version `1` for the metadata log seam and HTTP wire
version `1` for encoded replicated/peer value transport. The release ships an
in-memory store and feature-gated example path only; production durable engine
selection remains future hardening work and must add its concrete on-disk format
to this register.

## 0.42 Grid Hardening

`0.42.0` registers the supported durable raft-log format version `1` and the
replicated value-record format version `1`. The durable raft seam refuses unknown
future format versions before opening a store. Replicated value records persist
sealed bytes plus `(partition, version, epoch)` and tombstone state so restart and
anti-entropy can converge without resurrecting deleted keys.

## 0.43 Geo-Distribution And Elasticity

`0.43.0` registers control-plane snapshot format version `1` for operational
self-healing backup/restore. Upgrade checks keep the 0.42 -> 0.43 rolling window
bounded to raft-log format `1`, replicated value-record format `1`, and
invalidation wire frame version `1`; incompatible jumps fail loud before a mixed
cluster step is accepted.
