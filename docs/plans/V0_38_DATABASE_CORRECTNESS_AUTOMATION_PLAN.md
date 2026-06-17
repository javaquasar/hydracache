# HydraCache 0.38.0 Database Correctness Automation Plan

`0.38.0` should build on the `0.37.0` database production-hardening release by
adding assisted correctness automation.

`0.37.0` keeps the database cache model explicit: declared dependencies,
transactional invalidation outbox, optional runtime adapter matrix, search/list
key reviewability, prepared repository contracts, attribute macros, external
writer bridges, and cross-node barriers. That makes the layer production-hardened
for explicit database result caching.

`0.38.0` should target the remaining gap between "explicit and production-ready"
and "nearly automatic correctness":

- SQL dependency declarations should be checked against SQL/parser/catalog
  signals where possible.
- Database writes should be able to install generated hooks that publish
  invalidation intent without every writer manually remembering it.
- Cross-node consistency should support named consistency modes and
  read-your-writes tokens.
- Cache keys should be checked against required business-dimension profiles.
- Transaction and outbox usage should have a companion API that reduces
  repository boilerplate without hiding transaction ownership.

The theme is **assisted correctness mode**: HydraCache still does not pretend to
be a transparent ORM cache, DB proxy, CDC platform, or distributed transaction
coordinator, but it should make common correctness mistakes visible in CI,
runtime diagnostics, sandbox examples, and release gates.

## Executive Summary

After `0.37.0`, HydraCache should be around **9/10** for production-grade
explicit database result caching. `0.38.0` should move that toward **9.4-9.6/10**
for the same explicit model by adding automation around the remaining risks.

This is not a promise of perfect automatic correctness. These problems are not
fully solvable inside a local-first Rust caching library:

- arbitrary SQL dependency detection is undecidable in practice because of
  dynamic SQL, views, functions, stored procedures, RLS, triggers, ORM builders,
  and external writers;
- invalidation from every database write requires database hooks, CDC, triggers,
  an outbox, or a proxy;
- globally serializable consistency across unavailable nodes requires a
  different product class;
- business dimensions such as tenant, permission, locale, or feature flag cannot
  be guessed safely by a cache library;
- full transaction ownership would make HydraCache a database transaction
  framework, which is not the goal.

The `0.38.0` goal is to provide strong assisted modes:

- strict SQL dependency linting;
- generated trigger/outbox/CDC hooks;
- named consistency modes and read-your-writes tokens;
- required dimension profiles for policies;
- transaction companion helpers;
- reconciliation and drift detection;
- first-class diagnostics and release gates for all of the above.

## Release Theme

Move from "production-hardened explicit database caching" to
"assisted correctness for explicit database caching".

This means:

- users still declare cache keys, tags, dependencies, freshness, and write-side
  invalidation;
- HydraCache can verify many declarations against SQL/parser/catalog evidence;
- database hooks can make external writes visible through the same outbox
  contract;
- consistency-sensitive flows can ask for named consistency behavior;
- CI can fail when production policies miss required dimensions;
- transaction helper APIs can reduce boilerplate while leaving commit/rollback
  ownership understandable;
- operators can see dependency-lint failures, hook lag, CDC lag, consistency
  timeouts, missing dimensions, and reconciliation drift.

## Non-Goals

- Do not claim perfect SQL dependency detection.
- Do not parse arbitrary dynamic SQL as a correctness guarantee.
- Do not invalidate cache entries from database writes unless hooks, outbox, CDC,
  or a user-provided bridge is configured.
- Do not provide global serializable consistency across unavailable nodes.
- Do not infer hidden business dimensions such as authorization scope.
- Do not own every database transaction in the application.
- Do not require triggers, CDC, or generated hooks for simple local-first users.
- Do not add a required external broker.
- Do not turn HydraCache into a transparent Postgres/MySQL proxy.

## Upgrade Story From 0.37

`0.37.0` should remain the stable explicit baseline.

`0.38.0` adds opt-in strictness:

```text
0.37:
  declare dependencies
  use outbox/triggers/CDC examples
  use explicit barriers
  test key dimensions
  use prepared policies and attribute macros

0.38:
  lint dependencies against SQL/catalog evidence
  generate DB hooks from metadata
  use named consistency modes and read-your-writes tokens
  enforce required key dimension profiles in tests/CI
  reduce transaction/outbox boilerplate with companion helpers
  reconcile DB state and cache invalidation state
```

Everything that is strict should be opt-in:

- warn mode for adoption;
- deny mode for CI/release gates;
- runtime degraded mode when the service chooses availability over strictness;
- documented escape hatches for dynamic SQL and service-specific policies.

## Global Test And Commit Rule

Every new code path in `0.38.0` must be covered by tests:

- parser/linter logic must have unit fixtures and negative cases;
- new macro syntax must have passing and failing `trybuild` tests;
- database hook generation must have SQL snapshot tests and runtime tests where
  practical;
- transaction helper APIs must have commit, rollback, retry, and error-path
  tests;
- consistency modes must have in-process multi-node tests and timeout tests;
- diagnostics/actuator additions must have serialization and counter movement
  tests;
- optional Docker-backed tests must be documented and skipped/ignored cleanly
  when not requested;
- each implementation step should be committed after its focused tests pass.

## 1. SQL Dependency Assistant And Strict Lint Mode

Status: planned.

### Problem

`0.37.0` should make dependencies explicit with metadata such as:

```rust
depends_on = [table("users"), table("user_roles")]
```

That is correct as the source of truth, but it still depends on humans declaring
the right list. `0.38.0` should add an assistant that can compare declared
dependencies against evidence from SQL text, SQLx metadata, Diesel rendered SQL,
SeaORM statements, and database catalogs.

### What Changes

Before:

```rust
let policy = query_cache_policy!(
    name = "load-user-permissions",
    key_segments = ["tenant", tenant_id, "user", user_id, "permissions"],
    depends_on = [table("users")],
    ttl_secs = 300,
);
```

Reviewers must notice that `user_roles` or `roles` may be missing.

After:

```rust
let policy = query_cache_policy!(
    name = "load-user-permissions",
    key_segments = ["tenant", tenant_id, "user", user_id, "permissions"],
    sql = "select u.id, r.name from users u join user_roles ur on ... join roles r on ...",
    depends_on = [table("users"), table("user_roles"), table("roles")],
    dependency_lint = deny_missing_dependencies,
    ttl_secs = 300,
);
```

The linter can report:

```text
query load-user-permissions reads users,user_roles,roles
declared dependencies: users
missing dependencies: user_roles,roles
```

### Planned API Shape

Candidate policy-level syntax:

```rust
let policy = query_cache_policy!(
    name = "search-users",
    key_segments = ["tenant", tenant_id, "q", query, "page", page],
    sql = "select id, name from users where tenant_id = $1 and name ilike $2",
    depends_on = [table("users")],
    dependency_lint = warn,
    ttl_secs = 30,
);
```

Candidate builder API:

```rust
let policy = QueryCachePolicy::new()
    .key(key)
    .depends_on(SqlDependency::table("users"))
    .sql_text(SQL)
    .dependency_lint(DependencyLintMode::DenyMissingDependencies);
```

Candidate CI helper:

```rust
let report = DependencyLint::new()
    .dialect(SqlDialect::Postgres)
    .policy(policy)
    .check()?;

assert!(report.is_clean(), "{report}");
```

Candidate catalog-assisted helper:

```rust
let report = PgDependencyCatalog::connect(&pool)
    .expand_views(true)
    .lint_policy(&policy)
    .await?;
```

### Pluses

- Reviewers see missing table dependencies before production.
- CI can fail for obvious policy mistakes.
- View/materialized-view dependencies can be expanded when the database catalog
  can provide them.
- SQLx/Diesel/SeaORM users get a shared lint report shape.
- The explicit `depends_on` contract remains the canonical runtime metadata.

### Risks

- False positives for dynamic SQL, database-specific syntax, functions, views,
  stored procedures, and RLS.
- False negatives when a table is touched indirectly through triggers, functions,
  or external systems.
- SQL dialect differences can make parser behavior inconsistent.
- Lint evidence can create false confidence if docs do not repeat that declared
  dependencies remain the source of truth.

### Database Setup

Basic parser linting should not require database setup.

Catalog-assisted linting may require:

- a read-only database connection;
- permission to inspect table/view metadata;
- Postgres catalog access such as `pg_catalog` view definitions;
- MySQL `information_schema` access;
- optional offline catalog fixtures for CI;
- a configured dialect: `postgres`, `mysql`, or `sqlite`.

### Required Tests

Unit/parser tests:

- simple `SELECT FROM users`;
- joins across `users`, `user_roles`, and `roles`;
- aliases;
- schema-qualified tables;
- CTEs;
- subqueries;
- quoted identifiers;
- comments and string literals that mention table-like names;
- insert/update/delete statements if write dependency lint is added;
- dialect-specific placeholders for Postgres/MySQL/SQLite.

Negative tests:

- missing declared dependency produces a warning in warn mode;
- missing declared dependency fails in deny mode;
- declared extra dependency is allowed or reported as informational according to
  documented behavior;
- dynamic SQL returns inconclusive instead of clean;
- unsupported SQL reports an actionable lint status, not a panic.

Catalog integration tests:

- Postgres view dependency expansion with testcontainers;
- Postgres materialized-view dependency expansion if practical;
- MySQL view dependency expansion if practical;
- SQLite view parsing from `sqlite_master`;
- missing catalog permission produces a clear error.

Macro/compile tests:

- passing `trybuild` for `sql = "...", dependency_lint = warn`;
- passing `trybuild` for `dependency_lint = deny_missing_dependencies`;
- failing `trybuild` for unknown lint mode;
- failing `trybuild` for duplicate `sql` or duplicate lint option.

Docs/sandbox tests:

- sandbox route showing clean policy;
- sandbox route showing missing dependency warning;
- JSON snapshot of lint report shape.

## 2. Generated DB Hooks And Semi-Transparent Invalidation

Status: planned.

### Problem

`0.37.0` should provide the transactional outbox and documented trigger/CDC
patterns. However, external writers still need to remember to write outbox rows
or install triggers manually.

`0.38.0` should automate more of this setup by generating database hooks from
HydraCache metadata.

### What Changes

Before:

```sql
INSERT INTO hydracache_invalidation_outbox(kind, value)
VALUES ('tag', 'users');
```

External writers or triggers must be written by hand.

After:

```powershell
hydracache-hooks generate `
  --database postgres `
  --table users `
  --tag users `
  --entity user:id `
  --out migrations/2026_..._hydracache_users_hooks.sql
```

Or from Rust metadata:

```rust
let hooks = HookPlan::postgres()
    .table("users")
    .on_insert(CollectionTag::new("users"))
    .on_update(EntityTag::from_columns("user", ["id"]))
    .on_delete(CollectionTag::new("users"))
    .render_sql()?;
```

### Planned Hook Types

- Postgres trigger function that writes outbox rows.
- SQLite trigger that writes outbox rows.
- MySQL trigger that writes outbox rows.
- Optional Postgres `LISTEN/NOTIFY` wakeup after outbox insert.
- Optional CDC bridge that converts change events into invalidation intent.
- Optional generated migration snippets, not automatic schema mutation.

### Pluses

- External writers become visible once DB hooks are installed.
- Teams do not hand-write fragile trigger SQL for every table.
- Trigger output uses the same outbox schema as repository writes.
- The generated SQL can be reviewed and versioned as a normal migration.
- Production systems can choose hooks per table instead of all-or-nothing.

### Risks

- Trigger SQL is database-specific.
- Triggers add write overhead and can fail writes if misconfigured.
- Trigger deployment requires migration discipline and rollback plans.
- Generated hooks must avoid recursive writes or duplicate outbox storms.
- CDC bridges have operational complexity and lag.
- MySQL binlog and Postgres logical replication require special privileges and
  environment-specific configuration.

### Database Setup

Common setup:

- `hydracache_invalidation_outbox` table from `0.37`;
- hook metadata/version table, for example `hydracache_hook_schema`;
- indexes for outbox polling;
- application migration runner applies generated SQL;
- outbox publisher worker enabled.

Postgres setup:

- trigger functions per table or shared generic function;
- `pgcrypto` or UUID generation strategy if generated IDs are database-side;
- optional `LISTEN/NOTIFY` channel such as `hydracache_invalidation`;
- permissions to create functions/triggers;
- optional logical replication slot for CDC prototype.

MySQL setup:

- triggers per table;
- UUID/time function compatibility;
- optional binlog row format for CDC bridge;
- replication permissions if binlog reading is supported.

SQLite setup:

- triggers per table;
- timestamp/id generation strategy compatible with SQLite;
- single-process or local-worker assumptions documented.

### Required Tests

SQL generation snapshot tests:

- Postgres insert/update/delete trigger SQL;
- MySQL insert/update/delete trigger SQL;
- SQLite insert/update/delete trigger SQL;
- generated SQL includes namespace, kind, value, timestamps, and dedupe data;
- generated SQL is stable across runs for reviewable migrations.

Runtime integration tests:

- SQLite trigger writes outbox row on insert;
- SQLite trigger writes outbox row on update;
- SQLite trigger writes outbox row on delete;
- worker publishes trigger-created outbox row and invalidates cache;
- duplicate trigger rows are idempotent;
- namespace isolation works with generated triggers.

Optional Docker integration tests:

- Postgres trigger + outbox + worker end-to-end;
- Postgres trigger + `LISTEN/NOTIFY` wakeup if implemented;
- MySQL trigger + outbox + worker end-to-end;
- CDC bridge unit test with synthetic Postgres event;
- CDC bridge unit test with synthetic MySQL binlog event if implemented.

Failure tests:

- missing outbox table gives clear hook installation/check error;
- trigger SQL render rejects missing tag/entity columns;
- publisher retry handles malformed external rows;
- generated hook version mismatch is detected at startup.

Docs/sandbox tests:

- sandbox shows generated hook SQL preview;
- sandbox demonstrates external raw SQL update invalidating cache through hook;
- docs include migration and rollback examples.

## 3. Named Consistency Modes And Read-Your-Writes Tokens

Status: planned.

### Problem

`0.37.0` should add receipts/barriers for cross-node read-after-write behavior.
`0.38.0` should turn that into a named consistency model users can reason about.

### What Changes

Before:

```rust
let receipt = cache.invalidate_tag_with_receipt(tag).await?;
cluster.wait_for_invalidation(&receipt, InvalidationWait::quorum()).await?;
```

After:

```rust
let token = db_cache
    .invalidate_after_write(tag)
    .consistency(ConsistencyMode::ReadYourWrites)
    .await?;

let value = db_cache
    .get_with_consistency(key, token, loader)
    .await?;
```

Or:

```rust
let value = query
    .consistency(ConsistencyMode::Quorum {
        timeout: Duration::from_millis(250),
    })
    .fetch_with(loader)
    .await?;
```

### Candidate Modes

- `Eventual`: default fast mode.
- `LocalReadYourWrites`: local generation must be applied.
- `ClusterReadYourWrites`: wait for propagation according to configured policy.
- `Quorum`: wait for quorum acknowledgement.
- `Leader`: route through a leader/control-plane path if available.
- `FailClosed`: return an error when consistency cannot be proven.
- `DegradedOk`: report degraded consistency but allow read.

### Pluses

- Users can choose correctness/performance tradeoffs per path.
- Product-critical flows can fail closed.
- Read-heavy non-critical flows can stay eventual.
- Timeouts and degraded results become observable.
- The API names the model instead of hiding it in ad hoc waits.

### Risks

- Stronger modes reduce availability or increase latency.
- Network partitions must not look like success.
- Quorum/leader modes need careful cluster integration.
- Users may overuse strict modes and hurt performance.
- The API must avoid implying serializable global consistency.

### Database Setup

No extra database setup should be required for local consistency modes.

Cluster consistency may require:

- cluster membership configured;
- invalidation bus enabled;
- peer acknowledgement support;
- node IDs stable enough for receipts;
- optional leader/control-plane configuration;
- actuator endpoint for consistency health.

### Required Tests

Unit tests:

- mode parsing/builders;
- timeout/degraded result shapes;
- token contains generation/namespace/origin metadata;
- default mode remains `Eventual`.

Concurrency tests:

- local generation prevents stale load overwrite;
- invalidation racing with load does not return pre-invalidation cached value
  when strict mode is requested;
- strict mode timeout returns an explicit error/degraded result.

Cluster/integration tests:

- two-node read-your-writes success;
- two-node timeout when peer does not acknowledge;
- quorum success with three nodes;
- quorum failure with insufficient acknowledgements;
- leader mode routes through expected leader path if implemented;
- partition simulation reports degraded/timeout instead of success.

Observability tests:

- wait success counter;
- timeout counter;
- degraded read counter;
- wait latency histogram/snapshot;
- actuator output for consistency health.

## 4. Required Dimension Profiles And Strict Key Review

Status: planned.

### Problem

`0.37.0` should make search/list key dimensions visible and testable, but the
library still cannot know every business dimension. `0.38.0` should let teams
declare required dimension profiles and fail CI when production policies do not
include them.

### What Changes

Before:

```rust
let policy = query_cache_policy!(
    name = "search-users",
    key_segments = ["tenant", tenant_id, "q", query, "page", page],
    ttl_secs = 30,
);
```

A reviewer must notice that `permission` and `sort` are missing.

After:

```rust
let policy = query_cache_policy!(
    name = "search-users",
    profile = tenant_permission_search,
    key_segments = [
        "tenant", tenant_id,
        "permission", permission_hash,
        "q", query,
        "page", page,
        "sort", sort,
    ],
    required_dimensions = ["tenant", "permission", "q", "page", "sort"],
    ttl_secs = 30,
);
```

CI can fail if the profile requires dimensions that are not present.

### Candidate Profiles

- `tenant_scoped`
- `permission_scoped`
- `tenant_permission_scoped`
- `paged_search`
- `cursor_list`
- `locale_region_scoped`
- `feature_flag_scoped`
- custom service profiles loaded from code or config

### Pluses

- Teams encode review policy once and reuse it.
- Production queries can opt into strict key-dimension checks.
- CI catches missing tenant/permission/page/sort.
- Diagnostics show which policies are weak.
- New engineers see required dimensions from policy code, not tribal knowledge.

### Risks

- Profiles can become too rigid for legitimate dynamic queries.
- Users may add meaningless labels just to satisfy the checker.
- Required labels still do not prove the values are semantically correct.
- Overly broad defaults can create noisy false positives.

### Database Setup

No database setup required.

Optional service configuration:

- checked-in profile definitions;
- CI command for strict policy validation;
- allowlist for intentionally unsafe or internal-only policies;
- diagnostics export enabled in production.

### Required Tests

Unit tests:

- profile requires all expected dimensions;
- profile passes when all labels exist;
- profile fails when tenant is missing;
- profile fails when permission is missing;
- profile fails when page/cursor is missing;
- custom profile can be defined and reused;
- allowlist requires reason text.

Macro/compile tests:

- passing `trybuild` for `required_dimensions`;
- failing `trybuild` for required dimension missing from literal key labels when
  statically checkable;
- failing `trybuild` for duplicate required dimension;
- failing `trybuild` for unknown built-in profile;
- passing dynamic/runtime validation when compile-time check is impossible.

Runtime tests:

- policy diagnostics include profile name and required labels;
- strict validation report fails a policy with missing label;
- warn mode emits warning but does not fail;
- deny mode fails release gate.

Sandbox/docs tests:

- unsafe policy example missing permission;
- fixed policy example;
- JSON report of dimension validation.

## 5. Transaction Companion API

Status: planned.

### Problem

HydraCache should not magically own database transactions, but safe transaction
plus invalidation/outbox patterns can still be verbose. `0.38.0` should provide
a companion helper that makes the recommended flow easy without hiding the
transaction.

### What Changes

Before:

```rust
let mut tx = pool.begin().await?;
repo.update_user(&mut tx, user).await?;
outbox.enqueue_sqlx(&mut tx, invalidations).await?;
tx.commit().await?;
```

After:

```rust
db_cache
    .transaction(pool, |tx, invalidations| async move {
        repo.update_user(tx, user).await?;
        invalidations.tag(User::cache_tag(user.id));
        invalidations.tag(User::collection_tag());
        Ok(())
    })
    .await?;
```

The helper should:

- begin transaction;
- pass transaction and invalidation collector to user code;
- enqueue outbox intent before commit;
- commit on success;
- rollback/drop invalidation on error;
- leave DB query execution and business logic inside user code.

### Pluses

- Less repository boilerplate.
- Fewer commit/outbox ordering mistakes.
- Rollback behavior is standardized.
- Tests can cover the common pattern once.
- Users still see transaction boundaries and invalidation collection.

### Risks

- Too much abstraction can hide transaction semantics.
- Generic transaction APIs differ across SQLx/Diesel/SeaORM.
- Nested transactions/savepoints are tricky.
- Long-running user closures can hold transactions too long.
- Users may expect the helper to solve all external writer problems.

### Database Setup

Required only for durable mode:

- outbox table installed;
- schema validation enabled;
- worker configured.

Non-durable companion mode can use `InvalidationPlan` after commit without DB
schema changes.

Adapter-specific setup:

- SQLx helper first;
- Diesel helper only if blocking transaction ownership remains clear;
- SeaORM helper only if transaction trait bounds stay ergonomic;
- custom transaction trait for repositories that do not use supported ORMs.

### Required Tests

SQLx/SQLite tests:

- success commits data and enqueues outbox intent;
- closure error rolls back data and no outbox row remains;
- enqueue failure rolls back transaction;
- commit failure reports error and does not publish;
- invalidation collector supports key, tag, entity, collection;
- custom namespace is preserved;
- direct `InvalidationPlan` mode works without outbox table.

Adapter tests:

- Diesel transaction companion is either implemented and tested or explicitly
  deferred;
- SeaORM transaction companion is either implemented and tested or explicitly
  deferred;
- custom transaction trait fake verifies begin/commit/rollback ordering.

Concurrency/error tests:

- user closure panic or error does not publish invalidation;
- retrying after failure can succeed;
- nested transaction attempt returns clear unsupported error if not supported.

Docs/sandbox tests:

- manual transaction/outbox example versus companion helper;
- rollback example;
- outbox-lag diagnostics after helper enqueue.

## 6. Reconciliation And Drift Detection

Status: planned.

### Problem

Even with strict policies and hooks, production systems can drift:

- a writer bypasses hooks;
- a hook is disabled during migration;
- an outbox worker is down;
- a CDC bridge lags;
- cache entries live longer than expected.

`0.38.0` should add reconciliation tooling that detects likely drift and guides
operators toward repair.

### Candidate Work

- Reconciliation report comparing:
  - outbox backlog;
  - hook schema versions;
  - last processed CDC position;
  - cache generations;
  - table update timestamps if configured.
- Optional reconciliation query per entity/table.
- Actuator endpoint for reconciliation health.
- Sandbox route demonstrating drift detection.
- Manual repair guidance:
  - invalidate tag;
  - replay outbox;
  - bypass cache temporarily;
  - rebuild hook.

### Required Tests

- report clean state;
- missing hook version reports drift;
- outbox backlog reports lag;
- stale CDC offset reports lag;
- manual invalidation clears drift report where applicable;
- actuator JSON shape test;
- sandbox drift scenario test.

## 7. Observability, Actuator, Sandbox, And Release Gates

Status: planned.

### Required Observability

Add counters/snapshots for:

- dependency lint warnings/errors;
- dependency lint inconclusive results;
- generated hook versions;
- hook-generated invalidation rows;
- CDC lag;
- outbox lag;
- consistency mode success/timeout/degraded;
- required dimension profile violations;
- transaction companion commit/rollback/enqueue failures;
- reconciliation drift.

### Required Sandbox Examples

- SQL dependency lint clean/missing/inconclusive.
- Generated trigger SQL preview.
- SQLite trigger/outbox end-to-end.
- Required dimension profile pass/fail.
- Consistency mode success/timeout/degraded.
- Transaction companion success/rollback.
- Reconciliation drift and repair.

### Required Release Gates

Local gate:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
.\scripts\verify-release-readiness.ps1 -Version 0.38.0 -RunGate
```

Focused gates:

```powershell
cargo test -p hydracache-db --locked dependency
cargo test -p hydracache-db --locked dimension
cargo test -p hydracache-db --locked outbox
cargo test -p hydracache-sandbox --locked correctness
cargo test -p hydracache --locked consistency
```

Optional Docker gates:

```powershell
cargo test -p hydracache-db --test postgres_dependency_catalog --locked -- --ignored
cargo test -p hydracache-db --test postgres_hooks --locked -- --ignored
cargo test -p hydracache-db --test mysql_hooks --locked -- --ignored
```

Packaging gate:

```powershell
.\scripts\package-publishable.ps1 -Set bootstrap
.\scripts\package-publishable.ps1 -Set runtime
.\scripts\package-publishable.ps1 -Set adapters
```

## Implementation Order

The release should be implemented in small commits:

1. Add this release plan and keep it updated.
2. Add dependency lint model, parser fixtures, report types, docs, and tests.
3. Add SQLx/Diesel/SeaORM lint evidence adapters where practical.
4. Add catalog-assisted dependency linting for Postgres first.
5. Add generated DB hook SQL renderers and snapshot tests.
6. Add SQLite trigger/outbox runtime integration.
7. Add optional Postgres/MySQL hook runtime tests.
8. Add named consistency modes and read-your-writes tokens.
9. Add required dimension profiles and strict policy validation.
10. Add transaction companion API for SQLx first.
11. Add reconciliation and drift detection.
12. Add observability/actuator/sandbox examples.
13. Update release notes, feature matrix, and production docs.
14. Bump versions, verify, tag, package, publish, and clean build artifacts.

## Final Release Decision

`0.38.0` should only claim "assisted correctness automation" if these statements
are true:

- dependency linting can catch obvious missing dependencies and report
  inconclusive cases honestly;
- generated hooks can write outbox invalidation intent for at least one
  deterministic local backend and one optional production backend if claimed;
- named consistency modes have success, timeout, and degraded tests;
- required dimension profiles can fail CI/release gates for missing key labels;
- transaction companion helpers reduce boilerplate without hiding transaction
  ownership;
- reconciliation can detect at least outbox lag and hook/schema drift;
- all new code is covered by unit, compile, integration, docs, or sandbox tests
  appropriate to its risk;
- docs still state clearly that HydraCache is not a transparent DB proxy,
  perfect SQL dependency oracle, or distributed transaction coordinator.
