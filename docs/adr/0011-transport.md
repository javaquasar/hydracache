# ADR-0011: Intent-Only Transport

## Status

Proposed.

## Context

Database writes need a fast wake-up path so cache invalidation does not wait for
slow polling. Postgres `LISTEN/NOTIFY` can provide wake-ups, but notifications
are not durable.

## Decision

HydraCache transports carry intent only. The durable source of truth is the
transactional outbox table. Notification transports such as Postgres
`LISTEN/NOTIFY` may wake a worker, but correctness must come from replaying
committed outbox rows.

## Consequences

- Lost notifications degrade latency but not correctness.
- Workers can always poll as a backstop.
- CDC/trigger integrations must write intent, not cached values.
