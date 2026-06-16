# HydraCache 0.37.0 Database Production Hardening Plan

`0.37.0` should close the remaining database-production gaps left after the
`0.35.0` readiness release and the `0.36.0` rollout release.

`0.36.0` made database caching easier to roll out safely: macro ergonomics,
feature-flag guidance, cache-key review checklists, staged invalidation,
freshness budgets, adapter-matrix documentation, and deterministic soak
validation. The next step is to harden the parts that still require too much
manual discipline in real production systems:

- SQL dependencies are explicit but not tool-assisted.
- Database writes do not automatically publish invalidation intent.
- Diesel and SeaORM Postgres/MySQL runtime behavior is not yet release-gate
  coverage.
- Cross-node read-after-write behavior is still eventual unless the application
  adds its own coordination.
- External writers must remember to emit invalidation intent manually.

The goal is not to make HydraCache a transparent ORM cache or a full CDC
platform. The goal is to provide explicit, test-backed building blocks that
make database cache correctness reviewable, observable, crash-tolerant, and
usable across multi-node services.

## Executive Summary

The database layer after `0.36.0` is a strong controlled-rollout candidate for
explicit, read-heavy database result caching. It is strongest when a service
owns the read path, write path, cache key dimensions, invalidation tags, and
freshness budgets.

`0.37.0` should raise confidence by adding production hardening around five
remaining gaps:

- **Declared SQL dependencies plus optional linting.** Keep explicit
  dependencies as the source of truth, but add metadata and best-effort linting
  so reviewers can see which tables/entities a cached query depends on.
- **Transactional invalidation outbox.** Let writes persist invalidation intent
  in the same database transaction as the data change, then publish after commit
  with retry, idempotency, and lag metrics.
- **Diesel/SeaORM Postgres/MySQL runtime matrix.** Move the documented adapter
  matrix beyond SQLite and contract tests by adding optional Docker-backed
  runtime tests for Postgres and MySQL where practical.
- **Cross-node read-after-write barrier.** Do not promise serializable cache
  consistency, but provide an explicit invalidation receipt/barrier API so a
  service can wait for local or cluster propagation before serving a dependent
  read.
- **External writer bridge.** Treat external writers as first-class production
  actors by documenting and testing trigger/outbox/CDC-style invalidation
  patterns.

## Release Theme

Move the database cache layer from "repeatable controlled rollout" to
"production-hardened explicit database caching".

This means:

- query dependencies are declared, inspectable, and optionally linted;
- write-side invalidation survives process crashes between commit and publish;
- external database writers have a documented invalidation protocol;
- multi-node read-after-write behavior has an explicit barrier, timeout, and
  degraded-mode story;
- adapter runtime support is backed by real database tests, not only API
  contract coverage;
- observability surfaces outbox lag, dependency metadata, barrier waits,
  publish failures, and external invalidation volume;
- every new code path added for this release has focused automated tests.

## Non-Goals

- Do not add transparent SQL interception.
- Do not infer all dependencies perfectly from arbitrary SQL.
- Do not make SQL parsing a runtime correctness requirement.
- Do not own user database transactions inside `DbCache`.
- Do not promise serializable, globally strong cache consistency.
- Do not hide cache keys, tags, or freshness budgets behind global state.
- Do not require Kafka, Debezium, Redis, or any external service for the core
  release path.
- Do not make Docker-backed database tests mandatory for every local developer
  command.

## Production Definition For This Release

For `0.37.0`, "production-hardened database caching" means:

- A cached database query can declare the tables, entities, and collections it
  depends on.
- A repository write can persist invalidation intent in the same transaction as
  the data change.
- A background publisher can publish that intent after commit, retry failures,
  and expose lag/failure metrics.
- External writers can use the same invalidation intent protocol through
  triggers, direct outbox inserts, or a documented bridge.
- A multi-node service can request bounded read-after-write behavior through an
  explicit invalidation receipt/barrier API.
- SQLx, Diesel, and SeaORM docs clearly distinguish deterministic local gates,
  optional Docker-backed database/runtime tests, and unsupported combinations.
- Release gates include enough automated coverage to make the new correctness
  claims honest.

It still does not mean:

- automatic invalidation for arbitrary SQL text;
- automatic discovery of every table touched by dynamic SQL;
- automatic invalidation from database writes unless the service opts into an
  outbox, trigger, or external bridge;
- strong consistency across unavailable nodes;
- replacing database constraints, transactions, indexes, queues, or CDC systems.

## Global Test And Commit Rule

Every implementation step in this release must follow the same rule:

- New public API must have unit tests and at least one usage test.
- New macro syntax must have passing and failing `trybuild` coverage.
- New database behavior must have deterministic SQLite or in-memory tests and,
  when the feature claims Postgres/MySQL support, optional Docker-backed runtime
  tests.
- New observability counters must have tests that prove the counter changes on
  the relevant success and failure paths.
- New docs examples that are meant to compile should be covered by doctests or
  integration tests.
- Each completed implementation step should be committed separately after the
  relevant tests pass.

## 1. Declared SQL Dependency Metadata And Optional Linting

Status: planned.

### Problem

HydraCache currently keeps cache keys, tags, and invalidation explicit. That is
the right safety model, but production review still has a blind spot: reviewers
cannot inspect a cached query and see a normalized dependency list.

Examples of risky situations:

- a query reads `users` and `user_roles`, but only the `users` tag is invalidated;
- a list query depends on a join table, but the join table is not represented in
  policy metadata;
- a query changes from one table to a join, but its invalidation tags are not
  reviewed;
- dynamic SQL makes automatic detection unreliable, so people assume more
  safety than the library actually provides.

### Desired Outcome

Add explicit dependency metadata to database policies and prepared policies.
The metadata should be inspectable by tests, diagnostics, and review tooling.

Optional SQL linting can compare declared dependencies with best-effort parsed
SQL, but explicit declarations remain the production source of truth.

### Proposed API Shape

The exact names can change during implementation, but the intended user shape
should be close to:

```rust
let policy = query_cache_policy!(
    name = "load-user-permissions",
    entity = User,
    id = user_id,
    tag_segments = [["tenant", tenant_id, "users"]],
    depends_on = [
        table("users"),
        table("user_roles"),
        entity(User),
        collection("users"),
    ],
    ttl_secs = 300,
);
```

Prepared policies should support the same metadata:

```rust
let load_user = prepared_query_policy!(
    per_entity = User,
    name = "load-user",
    depends_on = [table("users"), collection("users")],
    ttl_secs = 300,
);
```

Repository code should also be able to attach dependencies without macros:

```rust
let policy = QueryCachePolicy::new("search-users", key)
    .tag(users_tag)
    .depends_on(SqlDependency::table("users"))
    .depends_on(SqlDependency::table("user_roles"));
```

### Candidate Work

- Add dependency metadata types in `hydracache-db`, for example
  `SqlDependency`, `DependencyKind`, and `DependencySet`.
- Support table, schema-qualified table, entity, collection, tag, and custom
  dependency kinds.
- Normalize duplicates so repeated declarations do not produce noisy review
  output.
- Add builder methods on `QueryCachePolicy` and `PreparedQueryPolicy`.
- Extend `query_cache_policy!` and `prepared_query_policy!` with a
  `depends_on = [...]` form.
- Add diagnostics output that exposes dependency metadata without exposing
  cached values.
- Add an optional feature-gated SQL lint helper that can parse simple SQL and
  compare best-effort table references against declared dependencies.
- Keep SQL linting out of the default runtime path.

### SQL Lint Scope

The lint helper should be deliberately conservative:

- Good candidates:
  - simple `SELECT ... FROM table`;
  - joins;
  - schema-qualified table names;
  - aliases;
  - CTEs where the underlying base tables are visible;
  - subqueries where `sqlparser` can expose table factors.
- Explicitly best-effort:
  - dynamic SQL;
  - database-specific functions;
  - stored procedures;
  - views and materialized views;
  - permissions or row-level-security effects;
  - SQL assembled across many branches.

Lint failures should be review signals, not runtime invalidation behavior.

### Required Tests

- Unit tests for dependency normalization and duplicate removal.
- Unit tests for table, schema-table, entity, collection, tag, and custom
  dependency rendering.
- Policy builder tests proving dependencies survive cloning/preparation.
- Macro `trybuild` tests for valid dependency syntax.
- Macro `trybuild` tests for invalid dependency syntax and helpful errors.
- Optional SQL lint tests for simple select, join, alias, schema-qualified
  table, CTE, subquery, comments, and string literals.
- Negative lint tests where declared dependencies miss a parsed table.
- Tests proving the default build does not require the SQL parser feature.
- Diagnostics tests proving dependency metadata appears in safe review output.

### Documentation

- Extend `docs/DB_PRODUCTION_READINESS.md` with a dependency declaration
  checklist.
- Extend `docs/POLICY_GUIDE.md` with examples for entity, collection, join, and
  search queries.
- Document that dependency metadata helps review but does not invalidate
  anything by itself.

### Acceptance Criteria

- [ ] A cached database query can declare dependencies in builder and macro
  forms.
- [ ] Prepared policies can declare the same dependency metadata.
- [ ] Dependency metadata is visible in diagnostics/review output.
- [ ] Optional linting can flag obvious mismatches for simple SQL.
- [ ] Dynamic SQL limitations are documented clearly.
- [ ] Every new dependency metadata path has tests.

## 2. Transactional Invalidation Outbox

Status: planned.

### Problem

`0.36.0` added staged invalidation so repository code can invalidate after a
successful commit and skip invalidation after rollback. That solves a major
timing problem, but there is still a crash window:

1. The service commits the database write.
2. The process crashes before publishing cache invalidation.
3. Other reads can keep serving stale cached values until TTL or manual
   invalidation.

For production systems, invalidation intent should be persisted in the same
transaction as the data change.

### Desired Outcome

Add a database-backed invalidation outbox pattern:

- write transaction mutates application data;
- the same transaction inserts invalidation intent rows;
- rollback removes both the data change and the invalidation intent;
- a publisher claims committed outbox rows;
- the publisher invalidates keys/tags through HydraCache;
- rows are marked published, retried, or dead-lettered with observable state.

### Proposed Schema Shape

The first implementation can be SQLx/SQLite-first and documented as the
reference schema. Postgres and MySQL variants should preserve the same logical
fields.

```sql
CREATE TABLE hydracache_invalidation_outbox (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    intent_kind TEXT NOT NULL,
    cache_key TEXT NULL,
    cache_tag TEXT NULL,
    entity_name TEXT NULL,
    collection_name TEXT NULL,
    reason TEXT NULL,
    payload_json TEXT NULL,
    dedupe_key TEXT NULL,
    created_at_ms INTEGER NOT NULL,
    available_at_ms INTEGER NOT NULL,
    claimed_at_ms INTEGER NULL,
    claim_owner TEXT NULL,
    published_at_ms INTEGER NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT NULL
);
```

Important properties:

- `id` is stable and unique.
- `dedupe_key` lets a writer avoid flooding the outbox during repeated writes.
- `available_at_ms` supports retry backoff.
- `claim_owner` and `claimed_at_ms` support safe polling workers.
- `published_at_ms` makes lag and backlog measurable.

### Proposed API Shape

The exact trait shape should be guided by Rust type constraints, but the user
flow should stay explicit:

```rust
let mut intent = InvalidationIntentBatch::new("user-email-update")
    .invalidate_key(User::cache_key(user_id))
    .invalidate_tag(User::cache_tag(user_id))
    .invalidate_tag(User::collection_tag());

sqlx::query!("UPDATE users SET email = ? WHERE id = ?", email, user_id)
    .execute(&mut *tx)
    .await?;

outbox.enqueue_sqlx(&mut tx, intent).await?;
tx.commit().await?;
```

Publishing should be explicit and runnable in a service background task:

```rust
let worker = InvalidationOutboxWorker::new(outbox, cache)
    .batch_size(100)
    .claim_ttl(Duration::from_secs(30))
    .retry_backoff(Duration::from_secs(5));

let report = worker.run_once().await?;
```

### Candidate Work

- Add `InvalidationIntent`, `InvalidationIntentBatch`, `InvalidationOutbox`,
  `InvalidationOutboxWorker`, and `OutboxPublishReport` types.
- Provide a SQLx reference implementation for SQLite first.
- Add Postgres SQL schema and optional runtime tests if SQLx Postgres coverage
  is practical in the release window.
- Keep Diesel and SeaORM integration as explicit examples unless generic
  transaction typing stays simple.
- Add migration snippets for SQLite, Postgres, and MySQL.
- Add idempotent publish behavior: publishing the same intent twice must not
  corrupt cache state.
- Add retry/backoff and claim timeout behavior.
- Add dead-letter or permanent-failure reporting after configurable attempts.
- Expose observability counters for pending rows, lag, publish attempts,
  publish success, publish failure, retries, and dead letters.

### Required Tests

- Commit test: data write plus outbox row commit together.
- Rollback test: data write rollback also removes outbox row.
- Publish test: committed outbox row invalidates the expected key/tag.
- Retry test: a failing publisher leaves the row available for retry.
- Idempotency test: publishing the same intent twice is safe.
- Claim timeout test: a stuck claim becomes publishable again after timeout.
- Dedupe test: repeated writes with the same dedupe key do not flood the queue.
- Batch test: worker respects batch size and reports partial success.
- Metrics test: pending, lag, attempts, failures, and published counters move.
- SQLite integration tests for all core behavior.
- Optional Postgres testcontainers coverage for the reference SQLx path.

### Documentation

- Add an outbox section to `docs/DB_PRODUCTION_READINESS.md`.
- Add transaction diagrams showing write, outbox insert, commit, publish, retry,
  and rollback paths.
- Add repository examples for SQLx.
- Document how to operate the outbox worker and alert on lag.

### Acceptance Criteria

- [ ] A service can persist invalidation intent in the same transaction as a
  database write.
- [ ] A rollback cannot publish invalidation intent.
- [ ] A process crash after commit can be recovered by the outbox worker.
- [ ] The worker is idempotent and retryable.
- [ ] Outbox lag and publish failures are observable.
- [ ] New outbox code has unit and integration tests.

## 3. Diesel And SeaORM Postgres/MySQL Runtime Matrix

Status: planned.

### Problem

The current adapter matrix is honest but incomplete:

- SQLx has the strongest runtime confidence.
- Diesel and SeaORM have deterministic SQLite coverage.
- Diesel and SeaORM Postgres/MySQL are documented more as adapter-contract
  coverage than as runtime-verified release gates.

For production claims, users need to know which database/runtime combinations
have actually been exercised.

### Desired Outcome

Add optional runtime tests that validate Diesel and SeaORM behavior against
Postgres and MySQL where practical. Keep these tests optional so normal local
development is not blocked by Docker availability.

### Candidate Matrix

Deterministic local gate:

- `hydracache-db` contract and helper tests.
- SQLx SQLite tests.
- Diesel SQLite tests.
- SeaORM SQLite tests.
- Sandbox DB soak route in short mode.

Optional Docker-backed gate:

- SQLx Postgres smoke and adapter behavior tests.
- Diesel Postgres runtime tests.
- Diesel MySQL runtime tests.
- SeaORM Postgres runtime tests.
- SeaORM MySQL runtime tests.
- Sandbox Postgres smoke.

### Required Runtime Scenarios

Each ORM/database pair should cover the same behavior where the ORM supports it:

- exactly-one entity load;
- optional entity miss and negative cache behavior;
- collection/list load;
- cache hit avoids second database loader call;
- key invalidation reloads data;
- tag invalidation reloads data;
- staged invalidation executes only after commit;
- rollback preserves the previous cached value;
- loader error is surfaced with context;
- stale-on-loader-error works only when configured;
- policy metadata survives prepared policy reuse;
- diagnostics expose hits, misses, invalidations, and failures.

### Candidate Work

- Add Docker/testcontainers helpers shared across adapter crates where possible.
- Add Diesel Postgres and MySQL test modules behind ignored or feature-gated
  test targets.
- Add SeaORM Postgres and MySQL test modules behind ignored or feature-gated
  test targets.
- Add CI/documented commands for running the optional matrix.
- Update `docs/DB_PRODUCTION_READINESS.md`, `docs/FEATURE_MATRIX.md`, and
  release notes with exact support labels.
- Keep unsupported or untested combinations explicitly marked as such.

### Required Tests

- Local tests must continue to pass without Docker.
- Optional Docker tests must skip or be ignored cleanly when Docker is not
  requested.
- Each runtime test must create its schema, seed data, execute cache behavior,
  and clean up its container/database.
- Matrix tests must fail loudly if a claimed combination regresses.
- Documentation tests should assert the published matrix labels where possible.

### Acceptance Criteria

- [ ] Diesel Postgres runtime coverage exists or is explicitly deferred with a
  documented reason.
- [ ] Diesel MySQL runtime coverage exists or is explicitly deferred with a
  documented reason.
- [ ] SeaORM Postgres runtime coverage exists or is explicitly deferred with a
  documented reason.
- [ ] SeaORM MySQL runtime coverage exists or is explicitly deferred with a
  documented reason.
- [ ] The feature matrix distinguishes local gate, optional Docker gate,
  adapter contract coverage, and unsupported combinations.
- [ ] Every new runtime test path is documented with the command to run it.

## 4. Cross-Node Read-After-Write Barrier

Status: planned.

### Problem

HydraCache invalidation is explicit and generation-safe locally, but cross-node
read-after-write consistency is not guaranteed by the database layer. In a
multi-node service:

1. Node A writes to the database.
2. Node A invalidates its local cache and publishes cluster invalidation.
3. Node B may still serve a stale value until it receives and applies the
   invalidation.

That eventual behavior is acceptable for many caches, but some production
flows need an explicit way to wait before serving a dependent read.

### Desired Outcome

Add an explicit barrier/receipt API. The API should make the consistency model
clear:

- default reads remain fast and eventual across nodes;
- services can request a local or cluster invalidation barrier when needed;
- barriers have timeouts and report degraded consistency instead of silently
  pretending strong consistency;
- unavailable nodes do not block forever;
- the API is explicit enough that reviewers can see where stronger behavior is
  required.

### Proposed API Shape

The exact API can evolve during implementation, but the intended user flow is:

```rust
let receipt = cache
    .invalidate_tag_with_receipt(User::cache_tag(user_id))
    .await?;

cluster
    .wait_for_invalidation(
        &receipt,
        InvalidationWait::quorum()
            .timeout(Duration::from_millis(250)),
    )
    .await?;
```

For single-process usage:

```rust
let receipt = cache.invalidate_key_with_receipt(key).await?;
receipt.wait_local_applied().await?;
```

For reads that must not observe a pre-invalidation generation:

```rust
let value = cache
    .get_with_consistency(
        key,
        Consistency::read_your_writes(&receipt)
            .timeout(Duration::from_millis(100)),
        loader,
    )
    .await?;
```

### Candidate Work

- Add `InvalidationReceipt` with invalidation id, namespace, key/tag, origin
  node, local generation, submitted timestamp, and applied timestamp.
- Add local receipt generation for key and tag invalidation.
- Add optional cluster propagation acknowledgements where the current cluster
  transport can support them.
- Add `InvalidationWait` policies:
  - local applied;
  - all known peers;
  - quorum;
  - best effort;
  - timeout with degraded result.
- Add read options that can require a minimum local generation before returning
  a cached value.
- Document that barriers do not make database writes serializable and cannot
  force an unavailable node to apply invalidation.

### Required Tests

- Local generation tests for key invalidation.
- Local generation tests for tag invalidation.
- Race tests proving a stale load cannot overwrite a newer invalidation
  generation.
- Two-node in-process cluster test:
  - node B has stale value;
  - node A writes and invalidates;
  - barrier waits for node B;
  - node B reloads instead of serving stale cached data.
- Timeout test proving barrier failure is visible.
- Degraded-mode test proving the caller can choose whether to fail closed or
  continue.
- Backward-compatibility test proving default reads still work without barriers.
- Metrics tests for wait success, wait timeout, wait latency, and degraded mode.

### Documentation

- Add a "Cross-node consistency model" section to
  `docs/DB_PRODUCTION_READINESS.md`.
- Add examples for eventual default reads and explicit read-after-write flows.
- Add operator guidance for barrier timeout alerts.

### Acceptance Criteria

- [ ] Local invalidation can return a receipt.
- [ ] Cluster invalidation can optionally wait for peer application or report a
  timeout/degraded result.
- [ ] A caller can express read-your-writes behavior explicitly.
- [ ] Documentation says exactly what is and is not guaranteed.
- [ ] Tests cover success, timeout, degraded mode, and default eventual mode.

## 5. External Writer Contract, Trigger Bridge, And CDC Path

Status: planned.

### Problem

HydraCache cannot know about writes performed outside the service unless those
writes publish invalidation intent. External writers include:

- admin scripts;
- ETL jobs;
- another microservice writing the same database;
- database triggers or stored procedures;
- direct SQL console changes;
- CDC pipelines.

Today the rule is documented as an obligation. `0.37.0` should turn that rule
into a concrete integration path.

### Desired Outcome

External writers should have a small number of supported ways to invalidate
HydraCache:

- insert invalidation intent into the shared outbox table;
- use database triggers to insert outbox rows;
- use a lightweight bridge to poll the outbox and publish invalidations;
- optionally wake the bridge through Postgres `LISTEN/NOTIFY` when available;
- document CDC integration boundaries for teams that already use Debezium or
  another log-based system.

### Trigger Examples

The release should include documented SQL sketches for common cases.

Postgres example shape:

```sql
CREATE FUNCTION hydracache_users_invalidation() RETURNS trigger AS $$
BEGIN
    INSERT INTO hydracache_invalidation_outbox (
        id,
        namespace,
        intent_kind,
        cache_tag,
        collection_name,
        reason,
        created_at_ms,
        available_at_ms
    )
    VALUES (
        gen_random_uuid()::text,
        'default',
        'tag',
        'users',
        'users',
        TG_OP,
        floor(extract(epoch from clock_timestamp()) * 1000),
        floor(extract(epoch from clock_timestamp()) * 1000)
    );
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;
```

MySQL and SQLite examples should be simpler but preserve the same outbox
contract.

### Candidate Work

- Define the external invalidation intent contract in docs and types.
- Make outbox rows easy to write from SQL without requiring Rust-specific
  serialization for simple key/tag invalidations.
- Add trigger examples for:
  - entity row update invalidating an entity tag;
  - insert/delete invalidating a collection tag;
  - bulk update invalidating a broader collection/search tag.
- Add a small bridge API that can poll outbox rows and publish them through
  HydraCache.
- Add optional Postgres `LISTEN/NOTIFY` wake-up support if it can stay small and
  well-tested.
- Document how Debezium or another CDC system can publish the same intent into
  the outbox or directly into a service endpoint.
- Add reconciliation guidance for detecting writers that bypass invalidation.

### Required Tests

- SQLite trigger test:
  - cache value;
  - perform raw SQL update outside repository code;
  - trigger writes outbox row;
  - bridge publishes invalidation;
  - next read reloads fresh data.
- Collection trigger test for insert/delete.
- Bulk update test invalidating a broad collection tag.
- Duplicate trigger/outbox row test proving idempotent invalidation.
- External writer failure test proving outbox lag is observable.
- Optional Postgres trigger test when Docker is enabled.
- Documentation tests for the simple SQL contract where possible.

### Documentation

- Add an "External writers" section to `docs/DB_PRODUCTION_READINESS.md`.
- Add copyable trigger/outbox examples under database adapter docs.
- Add an incident runbook:
  - how to detect outbox lag;
  - how to replay failed invalidations;
  - how to temporarily bypass cache for affected reads;
  - how to repair after an external writer bypassed invalidation.

### Acceptance Criteria

- [ ] External writers have a documented invalidation contract.
- [ ] Trigger examples exist for SQLite, Postgres, and MySQL where practical.
- [ ] At least one tested trigger/outbox/bridge flow invalidates a cached value.
- [ ] Outbox lag and bridge failures are observable.
- [ ] Docs clearly say that writers bypassing the contract can still serve
  stale data until TTL or manual invalidation.

## 6. Observability And Actuator Hardening

Status: planned.

### Problem

The new production-hardening features are only useful if operators can see
their health. `0.37.0` should add diagnostics for dependency metadata, outbox
publishing, external writers, and invalidation barriers.

### Desired Outcome

Expose enough counters and snapshots that an operator can answer:

- Which cached DB policies have no declared dependencies?
- How many outbox rows are pending?
- How old is the oldest pending outbox row?
- How often do outbox publishes fail or retry?
- Are external writer invalidations flowing?
- Are read-after-write barriers timing out?
- Which cache policies are serving stale values during an upstream incident?

### Candidate Metrics

- `hydracache_db_dependency_missing_total`
- `hydracache_db_dependency_lint_warning_total`
- `hydracache_db_outbox_pending`
- `hydracache_db_outbox_oldest_age_ms`
- `hydracache_db_outbox_publish_attempt_total`
- `hydracache_db_outbox_publish_success_total`
- `hydracache_db_outbox_publish_failure_total`
- `hydracache_db_outbox_dead_letter_total`
- `hydracache_db_external_invalidation_total`
- `hydracache_db_barrier_wait_total`
- `hydracache_db_barrier_timeout_total`
- `hydracache_db_barrier_wait_ms`

Names can change to match the existing observability style, but the semantics
should be this explicit.

### Candidate Work

- Extend observability snapshots with DB hardening counters.
- Add actuator output for outbox backlog and barrier health where the actuator
  crate can remain read-only.
- Add sandbox routes that demonstrate outbox lag, external invalidation, and
  barrier timeout behavior.
- Document dashboard panels and alert examples.

### Required Tests

- Unit tests for every new counter/snapshot field.
- Actuator serialization tests for new fields.
- Sandbox route tests for JSON shape and counter movement.
- Failure-path tests proving retries/timeouts are visible.

### Acceptance Criteria

- [ ] Operators can inspect outbox backlog and lag.
- [ ] Barrier waits and timeouts are visible.
- [ ] External invalidation volume is visible.
- [ ] Dependency metadata/lint warnings are visible.
- [ ] Actuator output remains read-only.

## 7. Documentation, Sandbox, And Examples

Status: planned.

### Problem

The production-hardening features will add new concepts. Users need to see the
verbose implementation and the safe helper path side by side, especially for
outbox, external writers, and barriers.

### Desired Outcome

The sandbox should let users inspect realistic examples without guessing how
the pieces compose.

### Candidate Work

- Add sandbox examples for:
  - manual staged invalidation;
  - transactional outbox invalidation;
  - trigger/outbox external writer invalidation;
  - dependency metadata review;
  - optional SQL lint warning;
  - cross-node read-after-write barrier success;
  - cross-node barrier timeout/degraded mode.
- Keep examples side by side:
  - verbose repository implementation;
  - equivalent helper-based implementation;
  - expected diagnostics.
- Update release docs with exact examples and tradeoffs.

### Required Tests

- Route tests for every new sandbox example.
- Snapshot or shape tests for JSON example output.
- Tests proving the examples use the same keys/tags/dependencies as documented.

### Acceptance Criteria

- [ ] Sandbox examples cover all five release-37 hardening themes.
- [ ] Each example shows explicit keys, tags, dependencies, and freshness.
- [ ] Each new route has tests.
- [ ] Docs link to the sandbox examples from the relevant production guide
  sections.

## 8. Release Gates

Status: planned.

### Required Local Gate

The release should pass:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
.\scripts\verify-release-readiness.ps1 -Version 0.37.0 -RunGate
```

If Windows linker locks appear again, use the documented workaround:

```powershell
$env:CARGO_BUILD_JOBS = "1"
cargo test --workspace --all-targets --locked
```

### Required Focused Gates

Expected focused commands should include:

```powershell
cargo test -p hydracache-db --locked
cargo test -p hydracache-sqlx --locked
cargo test -p hydracache-diesel --locked
cargo test -p hydracache-seaorm --locked
cargo test -p hydracache-sandbox --locked db_
cargo test -p hydracache --locked invalidation
```

### Optional Docker Gate

The exact test names can change, but the release should document commands like:

```powershell
cargo test -p hydracache-sqlx --test postgres_testcontainers --locked -- --ignored
cargo test -p hydracache-diesel --test postgres_runtime --locked -- --ignored
cargo test -p hydracache-diesel --test mysql_runtime --locked -- --ignored
cargo test -p hydracache-seaorm --test postgres_runtime --locked -- --ignored
cargo test -p hydracache-seaorm --test mysql_runtime --locked -- --ignored
```

### Packaging Gate

Before publishing `0.37.0`, run:

```powershell
.\scripts\package-publishable.ps1 -Set bootstrap
.\scripts\package-publishable.ps1 -Set runtime
.\scripts\package-publishable.ps1 -Set adapters
```

### Acceptance Criteria

- [ ] Local release gate passes.
- [ ] Focused DB hardening tests pass.
- [ ] Optional Docker matrix is either passing or explicitly documented with
  deferred combinations.
- [ ] Package verification passes for bootstrap, runtime, and adapter sets.
- [ ] Release notes list the new production guarantees and remaining non-goals.

## Implementation Order

The release should be implemented in small commits:

1. Add this release plan and keep it updated.
2. Add dependency metadata types, builders, macro syntax, diagnostics, docs, and
   tests.
3. Add optional SQL dependency linting, docs, and tests.
4. Add transactional invalidation outbox schema, SQLx SQLite implementation,
   worker, metrics, docs, and tests.
5. Add external writer trigger/outbox bridge examples and tests.
6. Add cross-node invalidation receipt/barrier API and tests.
7. Add Diesel/SeaORM Postgres/MySQL optional runtime matrix.
8. Add observability/actuator/sandbox hardening examples and tests.
9. Update release notes and release gates.
10. Bump versions, verify, tag, package, publish, and clean build artifacts.

After each implementation commit, run the narrowest meaningful test set first.
Before the release commit, run the full local release gate.

## Final Release Decision

`0.37.0` should only claim "production-hardened database caching" if these
statements are true:

- dependency metadata is explicit and test-covered;
- outbox invalidation survives rollback, retry, and process-crash windows;
- external writer invalidation has a tested path;
- cross-node read-after-write behavior has an explicit barrier and timeout
  story;
- Diesel and SeaORM runtime support labels are backed by tests or clearly
  marked as not yet covered;
- all new code has tests;
- docs say where HydraCache still does not provide automatic consistency.
