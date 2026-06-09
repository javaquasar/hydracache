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
- Committed scenario file and suite runners for JSON/YAML recipes.
- Timeline assertions so recipes can validate event ordering, not only counters.
- Flow catalog and retained-flow replay for bug-report style reproduction.
- Dashboard textarea editor for JSON/YAML scenario documents.
- Seeded users/products/order-summary query-cache demos.
- Benchmark operation percentiles, loader-call ratio, p95 diff, and comparison
  verdicts.
- OpenAPI generated-client smoke check for the committed fetch client fixture.

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
  client check/smoke, dashboard links, committed scenario files/suites, flow
  replay, seeded query-cache demos, and OpenAPI schemas.
- `cargo test -p hydracache-sandbox --lib` should pass before release.
- Full workspace tests should remain green before tagging.
