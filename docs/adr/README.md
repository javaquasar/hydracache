# Architecture Decision Records

Use ADRs for decisions that affect architecture, public API, durable formats,
wire compatibility, or long-term product direction.

## Index

| ADR | Status | Decision |
| --- | --- | --- |
| [ADR-0001](0001-gossip-liveness-vs-raft-topology.md) | Accepted for 0.41 | Gossip/discovery is liveness input; Raft/topology is authority. |
| [ADR-0002](0002-raft-log-store-durability-contract.md) | Accepted for 0.41 | Raft log storage contract and durability seam. |
| [ADR-0003](0003-replication-strategy-and-effective-map.md) | Accepted for 0.41 | Replication strategy and effective owner/replica map. |
| [ADR-0004](0004-rebalance-plan-as-data.md) | Accepted for 0.41 | Rebalance is modeled as explicit plan data. |
| [ADR-0005](0005-tombstone-gc-vs-repair-boundary.md) | Accepted for 0.41 | Tombstone GC is bounded by repair safety. |
| [ADR-0006](0006-why-not-clone-hibernate-hikaricp.md) | Accepted for 0.49 | Provide a Hibernate L2 provider; do not clone Hibernate/HikariCP. |
| [ADR-0007](0007-client-wire-framing.md) | Accepted for 0.49 | Use custom length-prefixed binary framing over HTTP/2, not gRPC. |
| [ADR-0008](0008-ownership.md) | Proposed for 0.37 | HydraCache owns cache metadata, not the user's DB transaction. |
| [ADR-0009](0009-replication.md) | Proposed for 0.37 | Invalidation buses carry intent, not cached values. |
| [ADR-0010](0010-consistency.md) | Proposed for 0.37 | Database cache consistency defaults to eventual with explicit barriers. |
| [ADR-0011](0011-transport.md) | Proposed for 0.37 | Notification transports are wake-up signals; durable outbox is source of truth. |
| [ADR-0012](0012-durability.md) | Proposed for 0.37 | Outbox durability and idempotency advance only after apply. |
| [ADR-0013](0013-local-single-flight.md) | Accepted for 0.2 | Local cache misses for the same key share one loader execution. |
| [ADR-0014](0014-tag-generation-invalidation-safety.md) | Accepted for 0.4 | Tag generations prevent write-after-invalidate races. |
| [ADR-0015](0015-transaction-companion-adapters.md) | Accepted for 0.38 | SQLx transaction companion is explicit; Diesel/SeaORM companions are deferred. |

## Naming

ADR files use a single monotonic filename scheme:

```text
0001-short-kebab-title.md
0002-short-kebab-title.md
```

Do not introduce `ADR-0001-*` filenames or reuse an existing numeric prefix.

## Template

```text
# ADR-0001: Short Title

## Status

Accepted | Proposed | Replaced | Superseded

## Context

What problem are we solving?

## Options Considered

- Option A
- Option B
- Option C

## Decision

What did we choose?

## Consequences

What gets easier?
What gets harder?
What must we watch?

## Revisit When

What would cause this decision to change?
```
