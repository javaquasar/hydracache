# ADR-0012: Outbox Durability And Idempotency

## Status

Proposed.

## Context

The pre-`0.37.0` write path can invalidate after commit, but a process crash
between commit and publish leaves other nodes stale until TTL or manual repair.

## Decision

Durable invalidation intent is committed with the data write. The idempotency key
is `(namespace, commit_position, target_hash)`, where `target_hash` is a stable
hash of a normalized invalidation target. A drain worker claims committed rows,
applies invalidation, and only then marks rows published.

## Consequences

- Re-draining after a crash is safe.
- Duplicate intent in the same transaction collapses at the schema boundary.
- The durable frontier never advances past invalidation work that was not
  actually applied.
- Dead-letter and reset behavior are explicit operator actions.
