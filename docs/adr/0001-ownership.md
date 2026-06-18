# ADR-0001: Cache Ownership Boundary

## Status

Proposed.

## Context

HydraCache already has local-first caching and experimental cluster ownership.
Database hardening in `0.37.0` must not accidentally turn the database adapter
into an owner of SQL execution, ORM identity maps, or business transactions.

## Decision

HydraCache owns cache metadata, local entries, invalidation intent, and optional
cluster routing metadata. The user's database client, ORM, repository, or
application transaction remains the authority for data reads and writes.

The database adapter may help persist invalidation intent in the caller's
transaction, but it does not commit or roll back that transaction.

## Consequences

- Read/write correctness stays explicit and reviewable at repository call sites.
- Transactional outbox support composes with SQLx, Diesel, SeaORM, and custom
  application outboxes.
- Transparent ORM second-level caching remains out of scope.
