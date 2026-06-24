# ADR-0015: Transaction Companion Adapter Scope

## Status

Accepted for 0.38.

## Context

The safe production pattern is still explicit transaction ownership:

1. Application code opens the database transaction.
2. Business writes happen in that transaction.
3. HydraCache invalidation intent is collected beside the write.
4. Durable outbox rows are written before commit.
5. The transaction commits or rolls back as one unit.

SQLx exposes async transaction primitives that match this shape directly. Diesel
is synchronous and usually requires `spawn_blocking` or a pool-specific wrapper.
SeaORM has a different transaction abstraction over `DatabaseTransaction`.

## Decision

0.38 ships the SQLx transaction companion first. Diesel and SeaORM expose
explicit deferred stubs so users do not mistake them for production-ready
transaction companions.

The deferred adapter path is:

- use the existing query helpers for reads;
- use manual ORM transactions for writes;
- collect invalidation intent with `InvalidationCollector`;
- enqueue it manually through the outbox or run the SQLx companion where SQLx is
  the chosen write client.

## Consequences

This keeps the production-safe path tested without hiding ORM-specific
transaction semantics. Diesel/SeaORM companions can be implemented later with
their own runtime matrices instead of inheriting SQLx assumptions.
