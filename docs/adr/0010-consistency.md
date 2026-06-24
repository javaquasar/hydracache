# ADR-0010: Database Cache Consistency Levels

## Status

Proposed.

## Context

`0.37.0` adds transactional invalidation intent and read-after-write barriers.
The release does not have the fixed production topology needed to guarantee
quorum or all-peer freshness across unavailable nodes.

## Decision

HydraCache's default database-cache consistency remains eventual. `0.37.0`
supports explicit local and best-effort read-after-write barriers with bounded
timeouts and degraded results. Quorum and all-peer barriers are deferred until a
later production-pilot topology exists.

## Consequences

- A service can wait for local outbox drain or best-effort transport progress.
- Timeouts are visible and do not pretend to be strong consistency.
- Global serializable read-after-write is not claimed by the database layer.
