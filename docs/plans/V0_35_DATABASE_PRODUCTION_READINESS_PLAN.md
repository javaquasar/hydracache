# HydraCache 0.35.0 Database Production Readiness Plan

`0.35.0` is a database production-readiness release.

The goal is not to turn HydraCache into a query engine, ORM, CDC system, or
transparent Hibernate-like second-level cache. The goal is to make the existing
embedded database result-cache layer safe enough, explicit enough, observable
enough, and well-documented enough for real production adoption in Rust
services.

The current database layer is already suitable for controlled production use in
read-heavy paths with explicit invalidation. This release should reduce the
remaining operational footguns: transaction timing, cache-key discipline,
adapter error context, write-path invalidation, database-specific examples, and
production observability.

## Executive Summary

HydraCache database caching currently sits around **8/10** for embedded,
local-first query-result caching.

It is strong when:

- reads are expensive or high-volume;
- values can tolerate explicit TTL or bounded stale behavior;
- the application can identify entity and collection invalidation tags;
- the service owns all read and write paths that affect cached data;
- operators can observe hit ratio, loader calls, stale fallback, invalidation,
  and load failures;
- query execution remains inside SQLx, Diesel, SeaORM, or repository code.

It is weaker when:

- the application expects automatic table dependency detection;
- writes happen outside the service without external invalidation;
- cache keys omit tenant, authorization, filters, pagination, sorting, locale,
  feature flags, or other result-shaping dimensions;
- the workload requires strong read-after-write consistency across multiple
  processes or nodes;
- adapter-specific database errors need typed recovery after sharing an
  in-flight load across callers;
- teams do not have a documented invalidation policy.

`0.35.0` should make the production contract explicit:

> HydraCache is production-candidate for explicit, local-first database
> result caching in Rust services when keys, tags, TTLs, transaction boundaries,
> invalidation paths, and observability follow the documented checklist.

## Release Theme

Move the database adapters from "usable in controlled production" toward
"production-ready by default when the documented rules are followed".

This release should make these rules hard to miss:

- Every cacheable database read must have an explicit freshness model.
- Every write path must define when and how invalidation happens.
- Cache keys must include every dimension that can change the visible result.
- Entity and collection tags must match the write-side invalidation model.
- Adapter helpers must preserve enough context for production debugging.
- Production diagnostics must clearly show cache hits, misses, loader calls,
  stale fallbacks, single-flight joins, load failures, and invalidation results.
- SQLx should become the reference-grade adapter path.
- Diesel and SeaORM should be brought closer to SQLx confidence through parity
  tests, examples, and documentation.

## Current Readiness Assessment

### What Is Already Strong

- `hydracache-db` is database-neutral and keeps SQLx, Diesel, SeaORM, or a
  custom repository as the query authority.
- `DbCache`, `DbQuery`, `QueryCachePolicy`, and `PreparedQueryPolicy` provide a
  stable vocabulary for query-result caching.
- `HydraCacheEntity` and `CacheEntity` reduce repeated key/tag boilerplate for
  entity-shaped values.
- SQLx, Diesel, and SeaORM share the same conceptual model:
  - exactly-one query result,
  - optional query result,
  - collection query result,
  - explicit key/tag/TTL metadata,
  - database-owned query execution.
- SQLx has real SQLite tests and Postgres testcontainers coverage.
- Diesel and SeaORM have real SQLite-backed tests.
- The core cache layer already covers:
  - local single-flight,
  - per-entry TTL,
  - default TTL,
  - explicit key invalidation,
  - explicit tag invalidation,
  - generation-safe invalidation/load race handling,
  - stale-while-revalidate,
  - stale-on-loader-error,
  - diagnostics and event listeners.
- Production examples and policy docs exist:
  - `docs/PRODUCTION_EXAMPLE.md`
  - `docs/POLICY_GUIDE.md`
  - `docs/OBSERVABILITY_CONTRACT.md`

### What Still Needs Hardening

- The write-side invalidation story needs stronger production guidance.
- Transaction timing needs explicit examples:
  - invalidate after commit,
  - do not invalidate after rollback,
  - invalidate both entity and collection tags when list membership changes.
- Cache-key safety needs a production checklist.
- Adapter error context can be clearer.
- SQLx should have the most complete reference examples.
- Diesel and SeaORM need more parity and edge-case coverage.
- The sandbox should demonstrate DB cache behavior in a way that maps to
  production rollout checks.
- Documentation should separate "safe for controlled production" from "not an
  automatic consistency system".

## Production Definition For This Release

For `0.35.0`, "production-ready database caching" means:

- A user can pick one read-heavy DB path and wrap it safely.
- The docs explain when not to cache.
- The docs explain how to design keys and tags.
- The docs explain how to invalidate after writes.
- Tests cover the documented happy paths and failure paths.
- SQLx, Diesel, and SeaORM expose comparable APIs for common result shapes.
- Operators can inspect meaningful diagnostics after a workload.
- The release process verifies adapter tests, doctests, and publishable
  packages.

It does not mean:

- automatic invalidation from SQL text;
- automatic invalidation from database writes;
- serializable cache consistency;
- cross-node strong consistency;
- transparent query interception;
- replacing database indexes, materialized views, or application-specific
  caching decisions.

## Primary Users

### Service Developer

Wants to cache an expensive query result with minimal ceremony while keeping
SQLx, Diesel, SeaORM, or repository code unchanged.

Needs:

- a clear API;
- examples that compile;
- key/tag guidance;
- predictable error behavior;
- confidence that the DB loader is called only on misses.

### Backend Tech Lead

Wants to approve cache usage in production without introducing silent data
leaks or stale-data incidents.

Needs:

- rollout checklist;
- security-sensitive keying guidance;
- write-path invalidation patterns;
- "what not to cache" section;
- test patterns the team can copy.

### Operator / SRE

Wants to know whether caching is helping or hiding failures.

Needs:

- hit ratio;
- loader-call reduction;
- stale fallback count;
- load failure visibility;
- single-flight activity;
- invalidation counts;
- dashboard and alert guidance.

### Library Maintainer

Wants adapters to stay thin and consistent.

Needs:

- adapter parity matrix;
- API invariants;
- regression tests;
- release verification commands;
- clear non-goals to prevent scope drift.

## Supported Production Use Cases

These should be described as good fits:

- read-mostly entity by id;
- product/catalog item lookup;
- profile and preferences where bounded stale reads are acceptable;
- reference data with reliable explicit invalidation;
- short-lived search/list results;
- negative caching for missing optional rows;
- expensive repository methods where the repository remains the DB authority;
- high-concurrency hot reads that benefit from local single-flight;
- fragile upstream reads where stale-on-loader-error is acceptable.

## Risky Or Unsupported Use Cases

These should be called out clearly:

- authorization results without principal, tenant, resource, and action in the
  key;
- hidden request/session state not represented in the key;
- values that require strict read-after-write semantics;
- writes performed by another service without invalidation propagation;
- query results shaped by feature flags, locale, region, AB bucket, or role
  unless those dimensions are part of the key;
- ad-hoc SQL where affected invalidation tags are not understood;
- distributed database result caching where remote invalidation delivery is
  treated as a strong consistency guarantee;
- very large result sets without size and memory-pressure consideration.

## Non-Goals

- Do not add transparent SQL parsing or automatic table dependency detection.
- Do not add CDC, database triggers, or database-side invalidation.
- Do not hide SQLx, Diesel, SeaORM, or repository transactions behind a second
  query engine.
- Do not make cache usage implicit for every query.
- Do not promise strong distributed consistency for database result caching.
- Do not add mandatory Redis, Postgres LISTEN/NOTIFY, Kafka, NATS, or other
  external infrastructure.
- Do not make sandbox-only convenience APIs stable unless they are small,
  documented, and covered by tests.
- Do not preserve old experimental API if simplifying production semantics
  requires cleanup.

## Architectural Boundaries

### HydraCache Owns

- local cache storage;
- cache key namespace;
- encoding/decoding through the configured codec;
- TTL and expiration;
- tag index and tag invalidation;
- key invalidation and remove;
- single-flight miss deduplication;
- refresh/stale behavior;
- invalidation/load generation safety;
- cache events and diagnostics;
- optional local or cluster-aware invalidation propagation.

### Database Library Owns

- connection pools;
- transactions;
- SQL construction;
- compile-time query checking;
- row mapping;
- query cancellation semantics;
- database-specific errors;
- migration and schema ownership;
- isolation levels;
- locking behavior;
- retry policy for transactional writes.

### Application Owns

- deciding what is safe to cache;
- cache key design;
- invalidation tag design;
- invalidation after writes;
- tenant and authorization dimensions;
- feature flags and rollout;
- choosing TTL and stale policies;
- monitoring and alert thresholds;
- deciding whether stale reads are acceptable.

## Production Invariants

The implementation and docs should preserve these invariants:

- A cacheable DB operation without a key fails before the loader runs.
- Cache hits do not execute the loader.
- Loader errors are not cached.
- Optional `None` results may be cached when the adapter helper says so.
- Empty vectors may be cached when the adapter helper says so.
- Invalidating an entity tag removes entity-shaped cached results.
- Invalidating a collection tag removes list-shaped cached results.
- A write rollback should not invalidate unrelated existing cached values.
- A write commit should invalidate all affected keys/tags.
- Cache keys must be deterministic and stable across process restarts.
- Escaped key segments must prevent accidental key-shape collisions.
- Tag invalidation during an in-flight load must not store stale loaded values.
- Single-flight joins must share one loader execution for the same key.
- Adapter helpers must stay thin and must not introduce their own query engine.

## Adapter Readiness Scorecard

| Area | Current Level | Target For 0.35.0 |
| --- | --- | --- |
| `hydracache-db` neutral API | Strong | Production reference vocabulary |
| SQLx helper API | Strongest | Reference adapter |
| Diesel helper API | Production-candidate, newer | Parity with documented caveats |
| SeaORM helper API | Production-candidate, newer | Parity with documented caveats |
| Key/tag docs | Good but scattered | Central checklist |
| Write invalidation docs | Present indirectly | Explicit transaction-safe guide |
| Error context | Basic | Better operation/key/adapter context |
| Observability docs | Core-level | DB-cache interpretation guide |
| Sandbox DB demo | Useful | Production-readiness demo path |
| Release verification | Strong | Adapter-focused release gate |

## 1. Transaction-Safe Invalidation Guidance

### Problem

HydraCache invalidation is intentionally explicit. That is good for control, but
production users can still invalidate too early, forget collection tags, or
invalidate before a transaction commits.

The dangerous sequence is:

1. Invalidate cache.
2. Attempt database write.
3. Database write fails or rolls back.
4. Cache is now empty even though data did not change.

The safer sequence is:

1. Start transaction.
2. Perform write.
3. Commit.
4. Invalidate affected entity and collection tags.

### Desired Outcome

Document and test the recommended write-path shape:

1. Start transaction in SQLx, Diesel, SeaORM, or repository code.
2. Perform writes.
3. Commit successfully.
4. Invalidate affected entity and collection tags.
5. Optionally run a read-after-write smoke check in critical flows.

### Detailed Requirements

- Add a production guide section called "Invalidate After Commit".
- Show update, insert, and delete examples.
- Show a rollback example that does not invalidate.
- Show collection invalidation after inserts and deletes.
- Show entity invalidation after updates and deletes.
- Explain that invalidating before commit is only acceptable if the service
  intentionally prefers availability over temporary cache misses and understands
  the rollback behavior.
- Explain that if writes happen outside the service, some external invalidation
  mechanism is required.

### Implementation Shape

- Add `docs/DB_PRODUCTION_READINESS.md`.
- Add transaction-safe examples to README database section.
- Add a deterministic repository test in `hydracache-db`:
  - seed user `42`;
  - cache user `42`;
  - simulate failed write;
  - verify cached value remains;
  - simulate committed write;
  - invalidate `user:42` and `users`;
  - verify reload returns new value.
- Add SQLx SQLite or Postgres transaction coverage if feasible.
- Add Diesel and SeaORM equivalent tests if they remain small and stable.

### Test Cases

- `failed_write_does_not_invalidate_cached_entity`
- `committed_update_invalidates_entity_and_reloads`
- `committed_insert_invalidates_collection`
- `committed_delete_invalidates_entity_and_collection`
- `rollback_keeps_existing_cached_value`
- `double_invalidation_after_commit_is_idempotent`

### Acceptance Criteria

- Documentation clearly says invalidation should happen after successful commit.
- Tests prove failed writes do not invalidate existing cached values.
- Tests prove successful writes invalidate entity and collection results.
- Examples compile as doctests or integration tests.

## 2. Cache Key And Tag Safety Checklist

### Problem

The biggest production risk is not the cache runtime; it is incorrect key
design. Permission checks, tenant boundaries, filter parameters, pagination, and
sort order must be part of cache keys when they affect results.

Incorrect key:

```text
users:active
```

Safer key:

```text
tenant:7:users:active:role=admin:page=1:sort=name_asc
```

### Desired Outcome

Make key/tag design auditable before rollout.

### Detailed Requirements

- Add a checklist for key dimensions:
  - tenant id,
  - user/principal id when visible data depends on caller,
  - role or permission version,
  - resource id,
  - action,
  - filters,
  - pagination cursor/page,
  - sort order,
  - locale,
  - region,
  - feature flag or experiment variant,
  - soft-delete visibility,
  - time bucket if the query is time-windowed.
- Add a checklist for tags:
  - entity tag,
  - collection/list tag,
  - tenant tag,
  - permission/principal tag,
  - reference-data group tag,
  - search/list group tag.
- Add examples of unsafe vs safe policy construction.
- Explain that tags are invalidation handles, not lookup keys.
- Explain that a value can have many tags.
- Explain that a key must identify exactly one cached result shape.

### Possible Helper

Consider a small helper only if it remains simple:

```rust
let key = DbCacheKey::new()
    .tenant(tenant_id)
    .entity("user", user_id)
    .segment("profile")
    .build();
```

This helper should be optional. It must not become a security policy engine.

### Test Cases

- `tenant_dimension_changes_physical_key`
- `permission_dimension_changes_physical_key`
- `filters_are_escaped_as_key_segments`
- `pagination_and_sort_are_part_of_list_key`
- `collection_tag_does_not_replace_unique_key`
- `unsafe_key_examples_are_documented_not_runtime_enforced`

### Acceptance Criteria

- README links to the checklist from the database caching section.
- `docs/POLICY_GUIDE.md` points to the production checklist.
- Tests cover key escaping for tenant/filter/security shaped keys.
- Documentation gives copy-pasteable safe examples.

## 3. Adapter Error Context Hardening

### Problem

The generic cache path can turn loader errors into cache-layer errors. That is
acceptable for common cache miss/retry behavior, but production adapter helpers
should preserve enough context for logs and troubleshooting.

The challenge is that local single-flight shares one loader result across
multiple callers. This means the core cache should keep a database-neutral error
shape, while adapter APIs should still give useful context.

### Desired Outcome

Improve clarity of SQLx, Diesel, SeaORM, and generic repository loader errors
without breaking single-flight semantics.

### Desired Error Context

Where available, errors should include:

- adapter kind:
  - `generic`,
  - `sqlx`,
  - `diesel`,
  - `seaorm`;
- operation name;
- namespace;
- physical cache key;
- result shape:
  - `one`,
  - `optional`,
  - `all`,
  - `custom`;
- whether the loader ran or the error happened before the loader;
- whether the error came from cache metadata, cache codec, or loader.

### Implementation Options

Option A: Documentation-only clarification.

- Low risk.
- No API churn.
- Does not improve logs automatically.

Option B: Add `DbOperationContext`.

```rust
pub struct DbOperationContext {
    pub adapter: DbAdapterKind,
    pub operation: Option<String>,
    pub namespace: String,
    pub physical_key: Option<String>,
    pub result_shape: DbResultShape,
}
```

- Better diagnostics.
- More public API surface.
- Must be designed carefully.

Option C: Add context only to error display.

- Less API surface.
- Easier to adopt.
- Harder for users to inspect programmatically.

The preferred path is Option B only if it stays small. Otherwise choose Option C
for `0.35.0` and keep a typed context object for a later release.

### Test Cases

- `missing_key_error_includes_operation_name`
- `missing_key_error_includes_adapter_name`
- `loader_failure_is_not_cached`
- `loader_failure_can_retry`
- `single_flight_loader_failure_is_shared_predictably`
- `adapter_error_display_contains_operation_context`

### Acceptance Criteria

- Missing-key errors include operation name and are covered for all adapters.
- Loader failures include enough diagnostic context for production logs.
- Tests verify that loader failures are not cached and retry remains possible.
- Documentation explains what error information is preserved and what is not.

## 4. SQLx Reference Adapter Maturity Pass

### Problem

SQLx is currently the strongest database adapter. It should become the canonical
reference adapter for production behavior.

### Desired Outcome

SQLx examples and tests should show the recommended production path clearly.

### Required Examples

- `sqlx_one` for exactly one row.
- `sqlx_optional` for optional row and negative cache.
- `sqlx_all` for list results.
- `fetch_with` for `sqlx::query_as!`.
- `fetch_with` for repository methods.
- Transaction-owned update followed by invalidation.
- Rollback that does not invalidate.
- Tenant-aware key.
- Collection invalidation after insert/delete.

### Postgres Testcontainers Coverage

Expand `crates/hydracache-sqlx/tests/postgres_testcontainers.rs` if gaps remain:

- failed loader then retry;
- transaction rollback does not invalidate;
- commit followed by invalidation reloads fresh data;
- tenant-aware key example;
- optional missing row caches `None`;
- empty list caches `Vec::new()`;
- collection invalidation reloads list;
- operation name appears in missing-key or loader context.

### SQLite Coverage

Keep SQLite coverage fast enough for normal workspace tests:

- prepared policy by id;
- collection query;
- optional missing row;
- TTL expiration;
- cache entity metadata;
- invalidation by entity and collection tags.

### Acceptance Criteria

- SQLx adapter can be used as the canonical README example.
- Postgres testcontainers still skip gracefully when Docker is unavailable.
- All new examples compile as doctests or integration tests.
- SQLx docs clearly say when to use helpers and when to use `fetch_with`.

## 5. Diesel Parity Pass

### Problem

Diesel is synchronous. The adapter correctly runs loaders through
`tokio::task::spawn_blocking`, but production users need clearer guidance and
tests around connection ownership, blocking behavior, and error handling.

### Desired Outcome

Diesel should feel like a first-class production-candidate adapter while still
being honest about blocking execution.

### Required Documentation

- Loader should acquire or own the Diesel connection inside the closure.
- Do not hold async locks across Diesel blocking work.
- Use a pool for real services.
- `diesel_optional` maps `NotFound` to cached `None`.
- Other Diesel errors are loader failures and must not be cached.
- Use explicit invalidation after committed writes.

### Test Cases

- `diesel_one_caches_real_sqlite_query_until_invalidation`
- `diesel_optional_caches_not_found_without_reloading`
- `diesel_optional_found_value_is_cached_until_invalidated`
- `diesel_all_caches_collection_results`
- `diesel_all_caches_empty_collections`
- `diesel_all_reloads_after_collection_tag_invalidation`
- `diesel_one_reloads_after_ttl_expiration`
- `diesel_one_loader_errors_are_not_cached_and_can_retry`
- `diesel_one_concurrent_same_key_joins_single_flight`
- `diesel_committed_write_invalidates_after_commit`
- `diesel_failed_write_does_not_invalidate`

### Acceptance Criteria

- Diesel tests cover common production result shapes.
- Diesel docs explain blocking behavior clearly.
- Diesel examples mirror SQLx vocabulary where possible.

## 6. SeaORM Parity Pass

### Problem

SeaORM is async and fits the adapter model well, but it is newer than SQLx and
needs parity coverage.

### Desired Outcome

SeaORM should feel like a first-class production-candidate adapter with clear
async loader semantics.

### Required Documentation

- `sea_one` expects exactly one value from caller-provided async code.
- `sea_optional` caches `Option<T>`, including `None`.
- `sea_all` caches vectors, including empty vectors.
- Transactions remain owned by SeaORM/repository code.
- Invalidation should happen after successful transaction commit.
- Use `fetch_with` for custom repository shapes if the convenience helpers do
  not fit.

### Test Cases

- `sea_one_caches_scalar_or_model_value`
- `sea_optional_caches_real_sqlite_query_until_invalidation`
- `sea_optional_caches_none_without_reloading`
- `sea_optional_found_value_is_cached_until_invalidated`
- `sea_all_caches_collection_results`
- `sea_all_caches_empty_collections`
- `sea_all_reloads_after_collection_tag_invalidation`
- `sea_optional_reloads_after_ttl_expiration`
- `sea_one_loader_errors_are_not_cached_and_can_retry`
- `sea_one_concurrent_same_key_joins_single_flight`
- `seaorm_committed_write_invalidates_after_commit`
- `seaorm_failed_write_does_not_invalidate`

### Acceptance Criteria

- SeaORM tests cover common production result shapes.
- SeaORM docs explain async loader behavior.
- SeaORM examples mirror SQLx vocabulary where possible.

## 7. Database-Neutral Repository Pattern

### Problem

Not every application wants adapter-specific helper methods. Some teams wrap DB
access behind repositories and want the cache layer around repository methods.

### Desired Outcome

Make the database-neutral pattern feel first-class, not like a fallback.

### Documentation Requirements

Show a repository like:

```rust
struct UserRepository;

impl UserRepository {
    async fn load_user(&self, id: i64) -> Result<User, RepositoryError> {
        todo!()
    }
}
```

Then show HydraCache wrapping it:

```rust
let user = queries
    .for_entity::<User>(id)
    .load(move || async move { repository.load_user(id).await })
    .await?;
```

### Test Cases

- `repository_loader_runs_once_on_repeated_reads`
- `repository_loader_retries_after_failure`
- `repository_loader_uses_prepared_policy`
- `repository_write_commit_then_invalidate`
- `repository_rollback_does_not_invalidate`

### Acceptance Criteria

- README shows that SQLx/Diesel/SeaORM are optional.
- `hydracache-db` rustdoc contains a repository-centered example.
- Tests prove repository-style loaders behave the same as adapter helpers.

## 8. Policy Presets And Freshness Guidance

### Problem

The policy presets exist, but production users need stronger guidance about
when each one is safe.

### Desired Outcome

Make policy selection easy and conservative.

### Required Guidance

Document the recommended presets:

- `short_lived()`:
  - burst smoothing;
  - search/list results;
  - permission checks when eventual consistency is acceptable;
  - default TTL: 30 seconds.
- `read_mostly()`:
  - product catalog;
  - profiles;
  - reference-ish data;
  - default TTL: 5 minutes.
- `per_entity()`:
  - entity by id;
  - key and tags should come from `CacheEntity`;
  - default TTL: 5 minutes.
- `negative_cache()`:
  - missing rows;
  - optional results;
  - avoid long absence caching;
  - default TTL: 30 seconds.
- `no_ttl_explicit_invalidation()`:
  - reference data with strong invalidation ownership;
  - risky for operational data;
  - should require review before production use.

### Test Cases

- policy preset TTLs remain stable;
- presets compose with `for_cache_entity`;
- presets compose with `PreparedQueryPolicy`;
- negative cache policy caches `None`;
- no-TTL policy remains until explicit invalidation.

### Acceptance Criteria

- `docs/POLICY_GUIDE.md` has a production-focused decision table.
- README links to the guide.
- Tests cover preset semantics.

## 9. Prepared Query Production Ergonomics

### Problem

Hot repository paths should not rebuild policy metadata every call. Prepared
query policies already exist, but production docs should make their role
obvious.

### Desired Outcome

Prepared query policies should be the recommended shape for hot paths.

### Documentation Requirements

Show:

- preparing metadata once at service startup;
- binding ids per request;
- static collection queries;
- prepared entity policy with collection tag;
- prepared policy with refresh options;
- prepared policy with tenant-aware key prefix if supported or manual key.

### Test Cases

- `prepared_entity_policy_binds_id_without_losing_collection_tag`
- `prepared_static_collection_policy_loads_list`
- `prepared_policy_refreshes_with_stale_behavior`
- `prepared_missing_id_policy_fails_before_loader`
- `prepared_policy_works_with_sqlx_helper`
- `prepared_policy_works_with_repository_loader`

### Acceptance Criteria

- Prepared query docs are linked from README.
- Tests prove prepared policies are equivalent to ad-hoc policies.

## 10. Database Cache Observability Contract

### Problem

Core diagnostics exist, but database users need an easy way to answer:

- Is this query hitting cache?
- How many database loader calls were avoided?
- Are stale fallbacks happening?
- Are invalidations removing expected entries?
- Are single-flight joins happening under load?
- Are load failures visible?

### Desired Outcome

Add DB-focused observability guidance and examples without expanding the core
metrics surface unnecessarily.

### Required Documentation

Extend `docs/OBSERVABILITY_CONTRACT.md` with a DB cache section:

- `hits` means loader avoided.
- `misses` means cache lookup missed and loader may run.
- `loads` means loader executed.
- `single_flight_joins` means concurrent DB load suppression.
- `stale_load_discards` means invalidation/load race safety was exercised.
- `invalidations` means explicit invalidation removed entries.
- `load_failed` events should be correlated with database errors.
- stale-on-loader-error should be visible through stats or event flow.

### Sandbox Requirements

The sandbox should expose a DB cache diagnostics route or expand an existing
route so users can see:

- query name;
- key;
- tags;
- TTL;
- first read source;
- second read source;
- loader-call delta;
- invalidation count;
- reload source;
- diagnostics snapshot.

### Test Cases

- `db_flow_updates_hit_miss_load_counters`
- `db_flow_records_single_flight_joins`
- `db_flow_records_invalidation_count`
- `db_flow_records_stale_fallback_when_loader_fails`
- `sandbox_db_report_contains_loader_call_delta`

### Acceptance Criteria

- Users can map core counters to DB-cache behavior.
- Sandbox and README expose a simple way to demonstrate DB cache diagnostics.
- Tests prove diagnostic counters move in expected directions.

## 11. Sandbox Production Demo

### Problem

The sandbox is useful, but production readiness improves when it can reproduce
real DB cache flows end to end.

### Desired Outcome

Make the sandbox a small production-readiness lab for database caching.

### Desired Routes

- `GET /demo/query/users/{id}/sqlx`
- `GET /demo/query/users/{id}/diesel`
- `GET /demo/query/users/{id}/seaorm`
- `POST /demo/query/users/{id}/update`
- `POST /demo/query/users/{id}/rollback`
- `POST /demo/query/users/{id}/invalidate`
- `GET /demo/query/users/{id}/report`
- `GET /demo/query/users/{id}/orm-comparison`

The exact route names can differ, but the sandbox should let a user reproduce:

1. first read miss;
2. second read hit;
3. database update without invalidation still returns cached value;
4. invalidate tag;
5. next read reloads;
6. rollback does not invalidate;
7. diagnostics explain what happened.

### OpenAPI Requirements

- Swagger descriptions should explain cache behavior, not just list fields.
- Request/response schemas should include:
  - cache source,
  - cache key,
  - tags,
  - loader calls,
  - invalidation result,
  - diagnostics summary.
- Examples should be provided for hit, miss, and invalidation flows.

### Test Cases

- route handler test for first miss then hit;
- route handler test for invalidate then reload;
- route handler test for rollback behavior;
- OpenAPI generation includes DB cache routes;
- response schema includes cache source and diagnostics.

### Acceptance Criteria

- Sandbox docs explain how to run memory, SQLite, and Postgres modes.
- Swagger can reproduce key DB cache features.
- Tests cover route logic without requiring Docker.
- Optional Postgres smoke test skips gracefully when Docker is unavailable.

## 12. Security And Multi-Tenant Safety

### Problem

Database caches can accidentally leak data if key design ignores tenant or
authorization dimensions.

### Desired Outcome

Make security-sensitive keying impossible to miss.

### Documentation Requirements

Add a section called "Security-Sensitive Cache Keys":

- include tenant id when data is tenant-scoped;
- include principal/user id when visible rows depend on caller;
- include role, permission version, or policy version when authorization affects
  visibility;
- include locale/region if content changes by locale/region;
- include feature flag or experiment variant if output changes;
- do not cache permission checks unless stale permission behavior is explicitly
  accepted;
- avoid caching data derived from request-local hidden state.

### Test Cases

- `tenant_keys_do_not_collide`
- `permission_keys_do_not_collide`
- `feature_flag_keys_do_not_collide`
- `locale_keys_do_not_collide`

### Acceptance Criteria

- README and production guide both link to security keying guidance.
- Examples use safe key shapes by default.

## 13. Load And Stability Tests

### Problem

The cache layer has concurrency tests, but database production readiness should
include realistic adapter-level load tests.

### Desired Outcome

Add focused load/stability tests that are useful before release but not too slow
for normal CI.

### Test Categories

Fast CI tests:

- concurrent same-key single-flight;
- concurrent different-key loads;
- invalidation while load is in progress;
- repeated optional missing row;
- repeated empty collection;
- TTL expiry under repeated reads.

Ignored/manual tests:

- high concurrency SQLx SQLite;
- Postgres testcontainers load flow;
- sandbox HTTP scenario load;
- long-running stale-on-loader-error behavior.

### Desired Commands

```powershell
cargo test -p hydracache-db --locked
cargo test -p hydracache-sqlx --locked
cargo test -p hydracache-diesel --locked
cargo test -p hydracache-seaorm --locked
cargo test -p hydracache-sqlx --test postgres_testcontainers --locked -- --nocapture
```

Optional ignored tests:

```powershell
cargo test -p hydracache-db --locked -- --ignored --nocapture
cargo test -p hydracache-sqlx --locked -- --ignored --nocapture
```

### Acceptance Criteria

- Fast tests remain suitable for normal CI.
- Slow tests are documented and ignored by default.
- Load tests assert behavior, not only "does not panic".

## 14. Documentation Deliverables

### New Or Updated Documents

- `docs/DB_PRODUCTION_READINESS.md`
- `docs/POLICY_GUIDE.md`
- `docs/PRODUCTION_EXAMPLE.md`
- `docs/OBSERVABILITY_CONTRACT.md`
- `README.md`
- adapter crate READMEs:
  - `crates/hydracache-sqlx/README.md` if added or needed,
  - `crates/hydracache-diesel/README.md`,
  - `crates/hydracache-seaorm/README.md`
- release notes:
  - `docs/releases/0.35.0.md`

### Documentation Topics

- What HydraCache DB caching is.
- What it is not.
- Safe rollout checklist.
- Key/tag checklist.
- Transaction-safe invalidation.
- SQLx reference examples.
- Diesel blocking-loader guidance.
- SeaORM async-loader guidance.
- Repository loader pattern.
- Policy preset decision table.
- Security-sensitive keying.
- Observability interpretation.
- Sandbox DB cache demo.
- Testcontainers behavior.

### Acceptance Criteria

- Every public API added or changed has rustdoc.
- Every README example either compiles as doctest or points to an executable
  test/example.
- Release notes are honest about readiness level and caveats.

## 15. API Design Rules

This release may adjust API if needed because the project is still pre-1.0, but
the direction should stay stable.

### Keep

- explicit cache keys;
- explicit tags;
- explicit TTL/stale policy;
- database-neutral `hydracache-db` core;
- thin adapter crates;
- `fetch_with` / `load` escape hatch;
- prepared policy path;
- `HydraCacheEntity` metadata.

### Avoid

- implicit query interception;
- parsing SQL to derive tags;
- adapter-specific policy models;
- hiding transaction ownership;
- global mutable cache registry as the default;
- magic invalidation from table names;
- storing DB connection/pool inside `DbCache`.

### Evaluate Carefully

- typed error context;
- key audit helper;
- transaction invalidation helper;
- DB diagnostics helper;
- sandbox route helpers that might tempt public API expansion.

## 16. Implementation Milestones

### Milestone 1: Planning And Docs Skeleton

- Create `docs/DB_PRODUCTION_READINESS.md`.
- Create `docs/releases/0.35.0.md`.
- Add README links.
- Add TODO checklist in release plan if needed.

Commit theme:

```text
docs: plan database production readiness for 0.35.0
```

### Milestone 2: Transaction-Safe Invalidation Tests

- Add repository or SQLite tests for commit/rollback invalidation.
- Cover update, insert, delete if feasible.
- Keep tests deterministic and fast.

Commit theme:

```text
test: cover transaction-safe database invalidation
```

### Milestone 3: Key/Tag Checklist And Tests

- Add key/tag safety docs.
- Add key-shape tests.
- Add security-sensitive examples.

Commit theme:

```text
docs: add database cache key safety checklist
```

### Milestone 4: Adapter Error Context

- Improve error context if API design remains small.
- Add tests for missing-key and loader-failure context.
- Update docs.

Commit theme:

```text
feat: improve database cache error context
```

### Milestone 5: SQLx Reference Pass

- Expand SQLx examples and tests.
- Verify Postgres testcontainers behavior.

Commit theme:

```text
test: strengthen sqlx production cache scenarios
```

### Milestone 6: Diesel And SeaORM Parity

- Expand adapter tests.
- Update adapter docs.

Commit theme:

```text
test: strengthen diesel and seaorm cache parity
```

### Milestone 7: Observability And Sandbox Demo

- Extend observability docs.
- Add or improve sandbox DB reports.
- Cover route/report logic with tests.

Commit theme:

```text
feat: expose database cache production diagnostics
```

### Milestone 8: Release Gate And Final Polish

- Update release notes.
- Run release readiness scripts.
- Run adapter-focused tests.
- Package publishable crates.

Commit theme:

```text
chore: prepare 0.35.0 database readiness release
```

## 17. Test Matrix

| Area | `hydracache-db` | SQLx | Diesel | SeaORM | Sandbox |
| --- | --- | --- | --- | --- | --- |
| Missing key fails before loader | Required | Required | Required | Required | Optional |
| Hit avoids loader | Required | Required | Required | Required | Required |
| Loader failure not cached | Required | Required | Required | Required | Optional |
| Retry after loader failure | Required | Required | Required | Required | Optional |
| Optional `None` cached | Generic if possible | Required | Required | Required | Required |
| Empty `Vec` cached | Generic if possible | Required | Required | Required | Required |
| TTL expiry reloads | Required | Required | Required | Required | Optional |
| Entity invalidation reloads | Required | Required | Required | Required | Required |
| Collection invalidation reloads | Required | Required | Required | Required | Required |
| Commit then invalidate | Required | Required | Desired | Desired | Required |
| Rollback does not invalidate | Required | Required | Desired | Desired | Required |
| Concurrent same-key single-flight | Required | Required | Required | Required | Optional |
| Key escaping | Required | Optional | Optional | Optional | Optional |
| Tenant/security key examples | Required | Required | Optional | Optional | Optional |
| Diagnostics counters | Required | Desired | Desired | Desired | Required |
| Real Postgres | Not applicable | Required optional-skip | Not required | Not required | Desired |
| Real SQLite | Optional | Required | Required | Required | Desired |

## 18. Release Verification

### Required Commands

Run the normal release gate:

```powershell
.\scripts\verify-release-readiness.ps1 -Version 0.35.0 -RunGate
```

Run adapter-focused checks:

```powershell
cargo test -p hydracache-db --locked
cargo test -p hydracache-sqlx --locked
cargo test -p hydracache-diesel --locked
cargo test -p hydracache-seaorm --locked
cargo test --doc -p hydracache-db --locked
cargo test --doc -p hydracache-sqlx --locked
cargo test --doc -p hydracache-diesel --locked
cargo test --doc -p hydracache-seaorm --locked
```

Run optional real Postgres coverage when Docker is available:

```powershell
cargo test -p hydracache-sqlx --test postgres_testcontainers --locked -- --nocapture
```

Run package checks:

```powershell
cargo package -p hydracache-db --allow-dirty
cargo package -p hydracache-sqlx --allow-dirty
cargo package -p hydracache-diesel --allow-dirty
cargo package -p hydracache-seaorm --allow-dirty
```

### Acceptance Criteria

- Workspace tests pass.
- Adapter tests pass.
- Doctests pass.
- Postgres testcontainers path either passes or skips gracefully.
- Package checks pass.
- Release notes state the DB-layer readiness level honestly.

## 19. Production Rollout Checklist

Before enabling HydraCache on a DB query in production:

- Identify the exact query or repository method.
- Confirm the result is worth caching.
- Confirm stale data is acceptable or TTL is short enough.
- Define the cache key.
- Include tenant/security dimensions in the key.
- Define entity tags.
- Define collection/list tags.
- Define TTL or explicit invalidation only.
- Decide whether stale-while-revalidate is acceptable.
- Decide whether stale-on-loader-error is acceptable.
- Add write-side invalidation after successful commit.
- Add tests for hit, invalidation, reload, and loader failure.
- Add diagnostics or logging around loader calls.
- Roll out behind a feature flag.
- Watch hit ratio, loader calls, invalidation count, stale fallback, and load
  failures.
- Keep an emergency disable path.

## 20. Definition Of Done

`0.35.0` is done when:

- `docs/DB_PRODUCTION_READINESS.md` exists and is linked from README.
- Transaction-safe invalidation is documented and tested.
- Cache key and tag safety checklist is documented.
- SQLx has reference-grade production examples.
- Diesel and SeaORM parity tests are expanded or explicitly documented as
  remaining work.
- DB observability interpretation is documented.
- Sandbox demonstrates DB cache behavior and reports useful diagnostics.
- New or changed public API has rustdoc.
- New behavior is covered by tests.
- Release notes summarize readiness level and caveats.
- Release readiness gate passes.

## 21. Expected Outcome

After `0.35.0`, the database layer should be reasonable to describe as:

> Production-candidate for explicit, local-first database result caching in Rust
> services, especially SQLx-backed services, when keys, tags, TTLs, transaction
> boundaries, invalidation paths, and observability follow the documented
> production checklist.

SQLx should be considered the reference adapter. Diesel and SeaORM should be
usable for production-candidate workloads with the same documented caveats and
test coverage, while still being described as newer adapter paths if gaps
remain.

## 22. Open Questions

- Should `DbCacheError` gain structured operation context, or should this
  release improve display strings only?
- Should a `DbOperationContext` type be public API now or deferred?
- Should a key audit helper be introduced, or should documentation remain the
  primary safety mechanism?
- Should transaction-safe invalidation helpers exist, or would they imply
  transaction ownership that HydraCache deliberately avoids?
- Should sandbox DB routes become examples only, or should some helper structs
  move into published crates?
- Should SQLx Postgres testcontainers become part of the release gate, or remain
  optional because Docker availability varies?
- Should Diesel and SeaORM get Postgres testcontainers later, or is SQLite
  enough for adapter behavior because the DB client owns SQL semantics?

## 23. Future Work After 0.35.0

Potential follow-up releases:

- external invalidation transport adapters for DB cache write paths;
- optional Postgres LISTEN/NOTIFY invalidation bus;
- optional Redis/NATS invalidation bus;
- stronger typed diagnostics for DB operations;
- macro sugar for repository-level cache policies;
- policy linting helpers for tenant/security key dimensions;
- benchmark suite for query-result cache overhead;
- memory pressure guidance for large list results;
- production examples for Axum services using SQLx pools;
- distributed DB cache caveats and safe patterns.
