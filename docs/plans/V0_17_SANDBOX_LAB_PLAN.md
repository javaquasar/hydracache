# HydraCache 0.17.0 Sandbox Lab Plan

Status: implemented in `0.17.0`.

## Goal

Turn `hydracache-sandbox` from a manual demo backend into a reusable behavior
lab for cache scenarios, assertions, release checks, and bug-report replay.

## Scope

- Scenario documents in JSON and a small YAML subset.
- Step-level scenario assertions over cache hits, misses, loads, loader calls,
  single-flight joins, invalidations, stale-load discards, event counts, and
  passed/failed step counts.
- Visual timeline support in the dashboard for flow-id correlated events.
- Benchmark comparison for baseline/candidate workloads.
- Dependency-free Prometheus text metrics and OpenTelemetry-style trace demo
  views derived from the sandbox event log.
- SQLite/Postgres migration and seed files for users, products, and orders.
- Session import for event streams exported through `/demo/export`.
- OpenAPI generated-client contract check plus a minimal fetch client example.

## Non-Goals

- The sandbox remains workspace-only and `publish = false`.
- The YAML parser intentionally supports only the small scenario-document subset
  used by committed recipes. It is not a general-purpose YAML implementation.
- Session import restores event context, not in-memory cache contents.
- The observability endpoints are teaching/demo views, not a full collector or
  production metrics exporter.

## Validation

- Unit and route tests cover the parser, executor, assertions, benchmark diff,
  session import, Prometheus text output, trace demo, seed report, OpenAPI
  client check, dashboard links, and OpenAPI schemas.
- `cargo test -p hydracache-sandbox --lib` should pass before release.
- Full workspace tests should remain green before tagging.
