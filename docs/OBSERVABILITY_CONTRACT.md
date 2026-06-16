# Observability Contract

HydraCache exposes observability in three layers:

- runtime snapshots from `HydraCache::stats()` and `HydraCache::diagnostics()`,
- framework-neutral serializable snapshots from `hydracache-observability`,
- optional read-only Axum routes from `hydracache-actuator-axum`.

The contract below describes the fields users can build smoke checks,
dashboards, and alerts around in the v0 line. New fields may be added in v0, but
the fields listed here should not be removed or renamed without an explicit
release note.

## Runtime Stats

`HydraCache::stats()` returns `CacheStats`.

Stable counters:

- `hits` - successful cache lookups.
- `misses` - cache lookups that did not return a usable value.
- `loads` - loader closures executed by `get_or_load`,
  `get_or_load_with_refresh`, `get_or_insert_with`, or adapter helpers.
- `single_flight_joins` - calls that joined an already running load.
- `stale_load_discards` - loader results discarded because invalidation made
  their generation stale.
- `invalidations` - entries removed by invalidation APIs.
- `evictions` - backend eviction observations. This remains `0` until backend
  eviction listeners are wired.
- `events_published` - cache events delivered to at least one subscriber.
- `event_subscriber_lagged` - event notifications skipped by slow subscribers.
- `distributed_invalidations_published` - invalidation messages published to an
  attached bus.
- `distributed_invalidations_received` - invalidation messages received from an
  attached bus.
- `distributed_invalidations_applied` - received invalidation messages applied
  to the local cache.
- `distributed_invalidation_lagged` - receiver lag on the invalidation bus.
- `distributed_invalidation_decode_errors` - invalidation frames that could not
  be decoded.
- `distributed_invalidation_publish_failures` - invalidation publish attempts
  that failed.
- `distributed_invalidation_receiver_closed` - bus receiver closed events.

Stable helper methods:

- `total_requests()` returns `hits + misses`.
- `hit_ratio()` returns `None` when no requests were observed, otherwise
  `hits / (hits + misses)`.
- `has_single_flight_activity()` reports whether at least one caller joined an
  in-flight load.
- `has_stale_load_discards()` reports whether invalidation safety discarded at
  least one stale loader result.
- `has_event_subscriber_lag()` reports slow local event subscribers.
- `has_distributed_invalidation_activity()` reports any bus activity.
- `has_distributed_invalidation_bus_issues()` reports bus lag/decode/publish or
  receiver-close issues.

## Runtime Diagnostics

`HydraCache::diagnostics()` returns `CacheDiagnostics`.

Stable fields:

- `stats` - the `CacheStats` snapshot described above.
- `estimated_entries` - approximate local backend entry count.

Stable helper methods:

- `total_requests()`.
- `hit_ratio()`.
- `is_empty()`.

`estimated_entries` is intentionally approximate. It is useful for smoke checks
and debugging, but it should not be used for billing, quotas, or exact capacity
accounting.

## Serializable Snapshots

`hydracache-observability` exposes JSON-friendly DTOs:

- `CacheStatsSnapshot`
- `CacheDiagnosticsSnapshot`
- `HydraCacheOverview`

`CacheStatsSnapshot` includes all stable `CacheStats` counters plus derived
fields:

- `total_requests`
- `hit_ratio`
- `single_flight_active`
- `stale_load_discards_seen`
- `event_subscriber_lag_seen`
- `distributed_invalidation_active`
- `distributed_invalidation_bus_issues`

`CacheDiagnosticsSnapshot` includes:

- `name`
- `stats`
- `estimated_entries`
- `empty`

`HydraCacheOverview` includes:

- `caches`

## Database Cache Interpretation

For `hydracache-db`, `hydracache-sqlx`, `hydracache-diesel`, and
`hydracache-seaorm`, the same runtime counters can be read as database-cache
signals:

- `hits` means the database, ORM, or repository loader was avoided.
- `misses` means the cache lookup did not return a usable value and a loader
  may run.
- `loads` means the database, ORM, or repository loader executed.
- `single_flight_joins` means concurrent same-key database work was suppressed.
- `stale_load_discards` means invalidation won a race against an in-flight
  database load, so the stale loaded value was not stored.
- `invalidations` means explicit key or tag invalidation removed cached query
  results.
- `LoadFailed` events should be correlated with database client, ORM, or
  repository errors.

When `stale_on_loader_error` serves a stale value, `loads` still increases
because the database loader was attempted. Pair that counter with application
logs from the database client or repository so stale fallback is treated as an
availability signal, not as a hidden success.

## Freshness And Incident Signals

Refresh and stale policies should have service-level budgets attached to them:

- `refresh_ahead` can increase `loads` before the visible TTL expires. During a
  healthy warm cache, hit ratio should remain stable while loader executions
  happen ahead of expiry for hot keys.
- `stale_while_revalidate` should be visible as background loader activity
  after entries become eligible for refresh. A stale value served during this
  window is acceptable only inside the documented freshness budget.
- `stale_on_loader_error` should coincide with repository/database loader error
  logs. Treat it as an incident-availability signal: users are getting a value,
  but the loader path is failing.
- Security-sensitive reads should not enable stale fallback unless the service
  explicitly accepts eventual consistency. For those paths, loader errors should
  surface instead of being hidden behind stale values.
- `stale_load_discards` indicates invalidation won a race against an in-flight
  load. That is a safety signal, but recurring discards can point to hot write
  paths or over-broad tags.

The sandbox DB routes expose these concepts through user-load and
ORM-comparison responses:

- `source = "loader"` for the first miss;
- `source = "cache"` for the second hit;
- `loader_calls` or `loader_calls_delta` for avoided database work;
- `diagnostics` snapshots for hit/miss/load/invalidation counters.

## Axum Actuator Routes

`hydracache-actuator-axum` exposes read-only routes that can be nested under an
application prefix such as `/actuator/hydracache`.

Stable route set:

- `GET /health`
- `GET /caches`
- `GET /caches/{name}/diagnostics`
- `GET /caches/{name}/stats`
- `GET /`

Stable response shape:

- `/health` returns `status` and `cache_count`.
- `/caches` returns `caches`.
- `/caches/{name}/diagnostics` returns `CacheDiagnosticsSnapshot`.
- `/caches/{name}/stats` returns `CacheStatsSnapshot`.
- `/` returns `HydraCacheOverview`.

Unknown cache names return `404` for cache-specific routes.

## What Is Not Stable Yet

These details are intentionally not part of the v0 contract:

- exact human-readable `Debug` output,
- exact ordering of fields in serialized JSON,
- exact timing of background refresh events,
- exact backend `estimated_entries` after an entry expires,
- write-enabled actuator/admin endpoints.

## Recommended Alerts

Start with low-noise alerts:

- `distributed_invalidation_bus_issues == true`
- `stale_load_discards_seen == true` on paths where invalidation races should
  be rare
- high `misses` or low `hit_ratio` after warmup
- unexpected growth in `loads` for hot keys
- non-zero `event_subscriber_lagged` when event consumers are operationally
  important
