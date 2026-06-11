# Roadmap

This roadmap tracks the current product direction. Historical release-by-release
details live in `docs/releases/` and the implementation plans in `docs/plans/`.

## Completed v0 Foundations

- Modular workspace with separate runtime, core codec, macro, database,
  observability, actuator, sandbox, and cluster crates.
- Local async cache runtime with TTL, tag invalidation, key invalidation,
  typed views, codec-backed storage, and Moka-backed capacity control.
- Local single-flight loader coordination with stale-load protection based on
  tag generations.
- Cache events/listeners with opt-in access/load events, bounded subscriber
  buffers, and diagnostics for lag.
- Database-neutral query cache descriptors and the SQLx adapter helpers for
  `fetch_one`, `fetch_optional`, and `fetch_all`.
- Macro ergonomics for cacheable ordinary async functions and
  `HydraCacheEntity` metadata derivation.
- Framework-neutral observability snapshots and optional read-only Axum
  actuator routes.
- Non-published sandbox backend with Swagger/OpenAPI, scenario DSL, event
  reports, benchmark reports, database modes, listener demos, and cluster demos.

## Completed Cluster Foundations

- `HydraCache::client()` and `HydraCache::member()` roles for application
  near-cache and in-process cluster member modes.
- In-process distributed invalidation bus for key/tag/flush freshness messages.
- Cluster diagnostics for role, node id, generation, epoch, members, clients,
  bootstrap nodes, and invalidation subscribers.
- Chitchat-backed soft discovery adapter in `hydracache-cluster-chitchat`.
- Raft-rs-backed metadata runtime in `hydracache-cluster-raft`.
- Composition helpers in `hydracache-cluster`.
- Sandbox flows for cluster lifecycle, real chitchat + raft adapters,
  generation-safe leave/publish semantics, and event/report visualization.

## Next Priorities

- Reduce hot-path allocations around event publication, tag snapshots,
  invalidation frames, and cluster diagnostics.
- Harden cluster behavior beyond the current local/single-process composition:
  real multi-node Raft transport, durable metadata storage, ownership/routing,
  and failover semantics.
- Add optional external invalidation transports such as Postgres
  LISTEN/NOTIFY, Redis, or NATS without making them part of the local cache
  fast path.
- Continue improving the sandbox as a regression and teaching lab: scenario
  catalogs, report diffs, generated clients, and observability examples.
- Keep `HydraCache::local()` small and dependency-light while advanced DB,
  actuator, observability, and cluster features remain opt-in crates.
