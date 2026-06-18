# ADR-0002: Invalidation Bus Does Not Replicate Values

## Status

Proposed.

## Context

The invalidation bus moves freshness signals between cache instances. It is
tempting to extend the same path into value replication, but that changes the
product class from embedded cache coordination into distributed storage.

## Decision

The bus carries invalidation intent only: key invalidation, tag invalidation, or
flush. It never carries cached database values in `0.37.0`.

Value replication, if added later, must use a separately versioned artifact and
must be registered in `docs/COMPAT.md` before any production-grid claim.

## Consequences

- External transports can be simple and low-risk.
- Outbox rows stay small and safe to insert from triggers or application SQL.
- A lost notification is recoverable by polling the durable outbox.
- Read misses still load from the database or repository authority.
