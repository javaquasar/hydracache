# HydraCache 0.16.0 Observability Plan

Status: implemented in `0.16.0`.

## Goal

Make it easy for a user to confirm that HydraCache is working without adding a
metrics stack, tracing backend, or external dashboard.

The common first question after wiring a cache is:

```text
Did the second call actually hit the cache?
```

`0.16.0` answers that with small, local, test-friendly diagnostics.

## Implemented Scope

- `CacheStats::total_requests()`
- `CacheStats::hit_ratio()`
- `CacheStats::has_single_flight_activity()`
- `CacheStats::has_stale_load_discards()`
- `CacheDiagnostics`
- `HydraCache::diagnostics().await`
- `TypedCache::diagnostics().await`
- New `hydracache-observability` crate.
- New `hydracache-actuator-axum` crate.
- `HydraCacheRegistry` for named cache registration.
- `HydraCacheProbe` for adapting `HydraCache` into a registry probe.
- Serializable `CacheStatsSnapshot`.
- Serializable `CacheDiagnosticsSnapshot`.
- Serializable `HydraCacheOverview`.
- Non-published `hydracache-sandbox` crate for manual checks.
- Sandbox modes:
  - `memory`
  - `sqlite-memory`
  - `sqlite-file`
  - `postgres-docker`
- Sandbox local `.env` support through a committed safe demo profile at
  `crates/hydracache-sandbox/.env` plus `.env.example` as a reference.
- Sandbox profile presets through `HYDRACACHE_SANDBOX_PROFILE` and `--profile`.
- Sandbox OpenAPI generated from Rust route/schema declarations through
  `utoipa`.
- Sandbox Swagger UI served from local embedded assets through
  `utoipa-swagger-ui`, without a CDN dependency.
- Sandbox HTTP collection and PowerShell demo scripts.
- Optional Postgres Docker smoke test with graceful skip when Docker is
  unavailable.
- Read-only Axum routes:
  - `GET /`
  - `GET /health`
  - `GET /caches`
  - `GET /caches/{name}/diagnostics`
  - `GET /caches/{name}/stats`

## Design Notes

`CacheStats` remains a lightweight counter snapshot. It does not become a
metrics registry and it does not own labels, exporters, histograms, or durable
storage.

`CacheDiagnostics` combines `CacheStats` with an approximate local backend
entry count. `HydraCache::diagnostics().await` first lets the Moka backend run
pending maintenance tasks, then reads the entry count. The entry count is still
diagnostic-only: useful for smoke checks, tests, and examples, but not for
billing, quotas, or strict accounting.

The actuator modules stay outside the base `hydracache` crate. This keeps the
embedded runtime HTTP-free while still allowing applications to opt in to a
Spring Boot-style read-only diagnostics surface when they already use Axum.

The actuator is read-only in `0.16.0`. Mutation endpoints such as `flush`,
`invalidate-key`, and `invalidate-tag` are deliberately deferred until there is
an explicit security and deployment model.

## Example

```rust
use hydracache::{CacheOptions, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();

let first = cache
    .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
    .await?;
let second = cache
    .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
    .await?;

let diagnostics = cache.diagnostics().await;

assert_eq!((first, second), (42, 42));
assert_eq!(diagnostics.stats.loads, 1);
assert_eq!(diagnostics.stats.hits, 1);
assert_eq!(diagnostics.total_requests(), 2);
assert_eq!(diagnostics.hit_ratio(), Some(0.5));
# Ok(())
# }
```

## Axum Actuator Example

```rust
use axum::Router;
use hydracache::HydraCache;
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_observability::HydraCacheRegistry;

let cache = HydraCache::local().build();
let registry = HydraCacheRegistry::new().with_cache("main", cache);

let app: Router = Router::new().nest(
    "/actuator/hydracache",
    HydraCacheActuator::new(registry).routes(),
);
# let _ = app;
```

## Manual Sandbox Example

`hydracache-sandbox` is a workspace-only manual backend. It is not published to
crates.io.

```powershell
cargo run -p hydracache-sandbox
```

The committed `.env` profile is useful for the usual local backend and contains
only non-secret demo settings. CLI flags still override it for one-off checks:

```powershell
cargo run -p hydracache-sandbox -- --profile memory
cargo run -p hydracache-sandbox -- --profile sqlite-memory
cargo run -p hydracache-sandbox -- --profile sqlite-file --sqlite-path target/hydracache-sandbox.sqlite
cargo run -p hydracache-sandbox -- --profile postgres-docker
```

The sandbox exposes:

```text
GET  /swagger-ui
GET  /openapi.json
GET  /actuator/hydracache/health
GET  /actuator/hydracache/caches/main/diagnostics
POST /demo/load/{id}
GET  /demo/users/{id}
POST /demo/users/{id}
POST /demo/invalidate/user/{id}
POST /demo/flush
```

Manual request helpers live next to the sandbox crate:

```text
crates/hydracache-sandbox/http/sandbox.http
crates/hydracache-sandbox/scripts/run-demo-flow.ps1
crates/hydracache-sandbox/scripts/start-profile.ps1
```

## Deferred

- Event listeners.
- Tracing spans.
- Metrics exporters.
- Backend eviction listener integration.
- Exact memory accounting.
- Actix-web adapter.
- Poem adapter.
- Write-enabled admin endpoints.
- Prometheus exporter.
