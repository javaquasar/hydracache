# HydraCache Sandbox Compose

Docker Compose files for running sandbox infrastructure locally.

## Recommended Profiles

Use the unified compose file when possible:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.yml --profile postgres up -d
cargo run -p hydracache-sandbox -- --profile postgres-compose
```

Run the full stack, including a prebuilt sandbox API image:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.yml --profile full up --build
```

The full profile builds `hydracache-sandbox:local` from
`Dockerfile.sandbox`. This avoids compiling the Rust workspace on every
container startup.

Open:

```text
http://127.0.0.1:3000/demo/ui
http://127.0.0.1:3000/swagger-ui
http://127.0.0.1:3000/openapi.json
http://127.0.0.1:3000/demo/config
http://127.0.0.1:3000/demo/presets
http://127.0.0.1:3000/demo/report
http://127.0.0.1:3000/demo/events
http://127.0.0.1:3000/demo/export
http://127.0.0.1:3000/demo/scenarios/files
http://127.0.0.1:3000/demo/scenarios/file/run
http://127.0.0.1:3000/demo/scenarios/suite/file/run
http://127.0.0.1:3000/demo/scenarios/document/run
http://127.0.0.1:3000/demo/flows
http://127.0.0.1:3000/demo/benchmarks/compare
http://127.0.0.1:3000/demo/distributed/invalidation/run
http://127.0.0.1:3000/demo/observability/prometheus
http://127.0.0.1:3000/demo/db/seed-report
http://127.0.0.1:3000/demo/openapi/client-smoke
http://127.0.0.1:3000/demo/security
```

The full-stack sandbox service has a Docker healthcheck against `/ready`.
Inside the UI or Swagger, `POST /demo/self-test` runs a quick end-to-end
scenario and returns step-level results plus correlated events. The same
sandbox API also exposes scenario runner, committed scenario files/suites,
timeline, flow catalog/replay, profile comparison, replay, fault-injection,
scenario document DSL, benchmark comparison, Prometheus/trace demo, DB seed
report, seeded users/products/order-summary query-cache demos, session import,
OpenAPI client smoke, distributed invalidation bus demo, and manual benchmark
endpoints.

The distributed invalidation demo response includes a compact timeline
(`source-publish`, `target-apply`, `diagnostics`) and diagnostics counters for
published, received, applied, lagged, publish-failure, and receiver-closed bus
states. The Prometheus endpoint exposes the same bus health counters.

Set `HYDRACACHE_SANDBOX_TOKEN` if you want the local sandbox routes to require
`Authorization: Bearer <token>`.

## Postgres Only

Compatibility shortcut for running only the local Postgres dependency:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.postgres.yml up -d
cargo run -p hydracache-sandbox -- --profile postgres-compose
```

The Postgres-only compose file publishes the database on `127.0.0.1:54329`.
The committed sandbox `.env` already points `HYDRACACHE_SANDBOX_DATABASE_URL`
at that address.

## Full Stack

Compatibility shortcut for running both Postgres and the sandbox API:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.full.yml up --build
```

Open:

```text
http://127.0.0.1:3000/demo/ui
http://127.0.0.1:3000/swagger-ui
```

The compatibility full stack also uses `Dockerfile.sandbox` and the same
`/ready` healthcheck.

## Stop

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.yml --profile postgres down
docker compose -f crates/hydracache-sandbox/compose/docker-compose.yml --profile full down
docker compose -f crates/hydracache-sandbox/compose/docker-compose.postgres.yml down
docker compose -f crates/hydracache-sandbox/compose/docker-compose.full.yml down
```

Add `-v` if you also want to remove persisted Postgres volumes.
