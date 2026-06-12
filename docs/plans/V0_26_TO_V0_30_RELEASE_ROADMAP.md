# HydraCache 0.26.0 To 0.30.0 Release Roadmap

Date: 2026-06-12.

Purpose: keep the next five releases coherent after the `0.25.0` owner-side
loading release. The direction is to move from "the cluster and database cache
surfaces exist" toward "the library is cheap on the local hot path, easier to
integrate, and more operationally explainable."

## Release Themes

| Release | Theme | Primary Outcome |
|---|---|---|
| `0.26.0` | Hot path and allocation hardening | Event/listener diagnostics stop charging allocations when nothing can observe them. |
| `0.27.0` | Prepared query cache policies | Repeated DB repository methods reuse precomputed cache metadata and bind only dynamic ids on the hot path. |
| `0.28.0` | Cluster runtime lifecycle | Background cluster components gain explicit start/stop/status/diagnostics boundaries. |
| `0.29.0` | Hot remote cache and owner pressure control | Remote owner reads can use a bounded near-cache policy separate from owned values. |
| `0.30.0` | Production cluster readiness pass | Security, compatibility, durable metadata seams, and post-publish consumer checks become first-class. |

## 0.26.0: Hot Path And Allocation Hardening

Goal: keep the local cache fast and boring even after listeners, diagnostics,
and sandbox reporting have grown.

Planned work:

- add an internal event-publication preflight before constructing owned
  `CacheEvent` payloads;
- avoid key/tag/event allocation when the event kind is disabled or no
  subscriber exists;
- keep mutation listeners source-compatible;
- add focused tests proving unobserved access events do not construct tags;
- add allocation/performance smoke coverage for subscriber and no-subscriber
  modes;
- expose a sandbox report that demonstrates event preflight behavior;
- update README, generated rustdoc examples, and testing docs.

## 0.27.0: Prepared Query Cache Policies

Goal: make database result caching easier and cheaper for repository methods
that run many times.

Planned work:

- add a prepared policy/descriptor API in `hydracache-db`;
- precompute entity labels, collection tags, TTL, diagnostics name, and static
  key prefixes;
- let SQLx helpers use the prepared path without making SQLx the identity of
  the product;
- document the adapter-neutral contract for future Diesel and SeaORM wrappers;
- cover prepared query flows with unit tests and real Postgres/SQLite
  integration tests.

Primary plan:

- [Prepared query policies plan](./V0_27_PREPARED_QUERY_POLICIES_PLAN.md)

## 0.28.0: Cluster Runtime Lifecycle

Goal: make long-running cluster pieces observable and gracefully stoppable.

Planned work:

- introduce an internal lifecycle model for membership watchers, admission
  bridges, invalidation receivers, peer-fetch services, and owner-load services;
- expose component status, last error, start count, stop count, and shutdown
  state through diagnostics;
- test graceful shutdown, restart, and failure reporting;
- add actuator and sandbox read-only cluster health snapshots.

## 0.29.0: Hot Remote Cache And Owner Pressure Control

Goal: reduce pressure on owner members without pretending that HydraCache is a
fully replicated data grid.

Planned work:

- distinguish owned entries from remote near-cache entries in diagnostics;
- add a configurable hot-remote TTL/capacity policy around owner-read
  hydration;
- ensure key/tag invalidation clears both owned and hot-remote copies;
- add owner pressure load tests for hot keys and multiple clients;
- document when to use plain peer fetch, read-through hydration, and
  owner-side load-on-miss.

## 0.30.0: Production Cluster Readiness Pass

Goal: make the cluster story safer to evaluate in staging.

Planned work:

- add an HTTP transport authentication boundary, initially token/header based;
- add protocol and wire-version compatibility checks;
- introduce durable metadata storage traits for the raft runtime, backed by a
  test implementation first;
- add external consumer checks for all cluster crates after publication;
- write a clear production-readiness document that separates stable local/DB
  APIs from experimental cluster surfaces.

## Guiding Constraints

- Local caching remains the center of gravity.
- DB caching stays adapter-shaped; SQLx is the first adapter, not the whole
  product.
- Cluster APIs remain layered: discovery, metadata, invalidation, peer fetch,
  owner load, and future replication are separate concerns.
- Sandbox scenarios must teach behavior and serve as regression evidence.
- Every release needs tests, generated rustdoc examples, and release notes
  before publication.
