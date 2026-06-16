# HydraCache 0.36.0 Database Production Rollout Plan

`0.36.0` should turn the `0.35.0` database production-readiness work into a
safer rollout story for real services.

`0.35.0` made the database cache layer suitable for controlled production use:
explicit keys, explicit invalidation, adapter parity tests, contextual errors,
and database-focused observability. The next step is to reduce adoption risk
when a team enables database caching behind feature flags, canaries, real
traffic, and service-specific query policies.

## Executive Summary

The database layer is currently a strong **controlled production rollout**
candidate, roughly **8/10** for explicit, local-first, read-heavy database
result caching.

`0.36.0` should focus on the remaining production adoption risks:

- write-side invalidation is still a repository/service-layer discipline;
- cache-key safety depends on reviewing each real query policy;
- stale fallback and refresh-ahead need service-specific TTL/SLO guidance;
- SQLx, Diesel, and SeaORM need broader runtime/database matrix confidence;
- the project needs a repeatable soak/load workflow before calling the layer
  mature for high-volume production use.

The goal for `0.36.0` is not to add transparent database interception,
SQL parsing, CDC, automatic table dependency detection, or strong distributed
read-after-write consistency. The goal is to make controlled rollout boring,
observable, reversible, and reviewable.

## Release Theme

Move the database cache layer from "ready for controlled production use" to
"ready for repeatable production rollout".

This means:

- a service can enable cached database reads behind a feature flag;
- operators can compare cached and uncached behavior during a canary;
- reviewers have a checklist for cache keys, tags, TTLs, and write paths;
- repository code has a documented transaction/invalidation pattern;
- stale behavior is tied to explicit service-level freshness budgets;
- adapter tests cover more database/runtime combinations where practical;
- release validation includes a deterministic soak/load scenario.

## Non-Goals

- Do not add implicit invalidation from SQL text.
- Do not parse queries to infer table dependencies.
- Do not own database transactions inside HydraCache adapters.
- Do not replace database indexes, materialized views, queues, or CDC.
- Do not make cross-node strong consistency a database-cache guarantee.
- Do not require an external cache server for the local-first database layer.

## 1. Rollout Playbook And Feature-Flag Pattern

### Problem

`0.35.0` explains how to cache a database query safely. Production teams also
need a rollout procedure:

- how to start with one read-heavy path;
- how to compare cached and uncached behavior;
- how to disable caching quickly;
- which metrics to watch during a canary;
- when to widen rollout.

### Desired Outcome

Add a database-cache rollout playbook that covers:

- feature-flagged read-through enablement;
- bypass mode for debugging or incident response;
- canary sizing and rollback guidance;
- recommended dashboard panels;
- alert examples for loader errors, stale fallback, hit ratio collapse, and
  unexpected invalidation volume;
- guidance for keeping the uncached path available during rollout.

### Candidate Work

- Extend `docs/DB_PRODUCTION_READINESS.md` with a rollout section.
- Add a compact README link from the database adapter section.
- Add a sandbox route or deterministic test scenario that can run cached and
  uncached variants and compare observed loader calls.

### Acceptance Criteria

- Documentation includes a step-by-step rollout checklist.
- The checklist includes explicit rollback and bypass instructions.
- A test or sandbox scenario demonstrates cached vs uncached comparison.
- Release notes call out the rollout playbook.

## 2. Repository Transaction And Invalidation Guardrails

### Problem

HydraCache intentionally does not own database transactions. That keeps the
library honest, but it means correctness depends on repository/service code:

- invalidate only after commit;
- keep rollback from evicting valid cached values;
- invalidate both entity and collection tags when list membership changes;
- document write paths that happen outside the service.

### Desired Outcome

Make the recommended repository pattern hard to miss and easy to copy.

The release should provide a small transaction/invalidation guide that shows:

- commit-then-invalidate for update/delete/insert flows;
- rollback preserving the previous cached value;
- entity tag and collection tag invalidation together;
- compensating strategies for external writers.

### Candidate Work

- Add a documented `PendingInvalidations` or `InvalidationPlan` example if it
  can stay database-neutral and avoid owning transactions.
- Add SQLx/Diesel/SeaORM examples that stage invalidations during the
  transaction and execute them only after commit.
- Add tests proving staged invalidations are not executed on rollback.

### Acceptance Criteria

- The docs show a repository-level pattern for staged invalidation.
- Adapter examples demonstrate the pattern without hiding transaction ownership.
- Tests cover commit, rollback, external write caveat, and repeated
  invalidation idempotency.

## 3. Cache-Key Review Checklist

### Problem

The biggest production correctness risk is an unsafe key:

- tenant missing from the key;
- authorization scope missing from the key;
- filters, pagination, sorting, locale, region, or feature flags omitted;
- time-window or "as of" dimensions omitted;
- collection tags used as if they were unique keys.

`0.35.0` documents these risks. `0.36.0` should make them reviewable.

### Desired Outcome

Add a cache-policy review checklist that a team can apply before enabling a
new cached query in production.

### Candidate Work

- Add a review template under the database production-readiness docs.
- Add examples of safe and unsafe query-policy reviews.
- Add optional test helpers or documentation patterns for asserting key
  dimensions in service tests.

### Acceptance Criteria

- Each review item maps to a concrete production risk.
- Examples include tenant, authorization, filter, sort, pagination, locale,
  region, feature flag, and time-window dimensions.
- Release notes describe the checklist as a pre-rollout guardrail.

## 4. Freshness Budgets For Stale And Refresh Policies

### Problem

Stale-on-loader-error and refresh-ahead are production features only when teams
set explicit freshness budgets. Without that, they can hide upstream failures
or serve stale data longer than the service can tolerate.

### Desired Outcome

Document and test freshness budget patterns:

- short-lived negative cache for missing rows;
- stale fallback bounded by an explicit duration;
- refresh-ahead for read-mostly catalog data;
- no stale fallback for security-sensitive or strongly fresh reads;
- metric expectations for stale fallback and refresh behavior.

### Candidate Work

- Add policy examples in `docs/POLICY_GUIDE.md`.
- Add tests that encode freshness intent for read-mostly, fragile upstream,
  security-sensitive, and negative-cache policies.
- Add observability notes for detecting stale fallback during incidents.

### Acceptance Criteria

- Docs include a freshness decision table.
- Tests prove each policy shape encodes the intended TTL/stale behavior.
- Observability docs explain how to spot stale fallback and refresh activity.

## 5. Adapter Runtime Matrix

### Problem

SQLx, Diesel, and SeaORM have parity coverage, but production confidence still
depends on runtime and database combinations.

### Desired Outcome

Define and expand the supported adapter matrix for `0.36.0`.

The matrix should be honest:

- mark combinations as tested, smoke-tested, documented, or out of scope;
- keep Docker-dependent checks optional and graceful;
- avoid promising every database backend for every ORM unless tested.

### Candidate Work

- Add an adapter support matrix to the database readiness guide.
- Add or strengthen smoke tests where they are cheap and stable.
- Keep Postgres Docker coverage optional but visible.
- Consider additional SQLite transaction examples for Diesel and SeaORM where
  they reveal real adapter behavior.

### Acceptance Criteria

- The docs distinguish "tested in CI/local gate" from "expected by adapter
  contract".
- SQLx Postgres smoke coverage remains optional and skips gracefully.
- Adapter parity tests stay deterministic on Windows.
- Release notes list the tested matrix.

## 6. Soak And Load Validation

### Problem

Unit and integration tests prove correctness boundaries, but they do not show
long-running production behavior under repeated reads, writes, invalidations,
stale fallback, and loader failures.

### Desired Outcome

Add a deterministic soak/load validation workflow for the database cache layer.

It should report:

- total requests;
- hit ratio;
- loader calls avoided;
- single-flight joins;
- invalidation counts;
- stale fallback counts or observable equivalents;
- load failures and retries;
- latency distribution if available without pulling in a heavy benchmark stack.

### Candidate Work

- Add a sandbox scenario or test binary that runs a bounded DB-cache workload.
- Keep the default run short enough for local release gates.
- Add an opt-in longer run for manual release validation.
- Emit a machine-readable summary for future CI dashboards.

### Acceptance Criteria

- Short soak scenario runs in the release gate or as a documented release
  validation command.
- Long soak command is documented for manual pre-release validation.
- The summary includes enough counters to compare cached vs uncached behavior.
- The scenario covers miss, hit, write, invalidate, reload, rollback, loader
  failure, and stale fallback.

## 7. Release Gate Updates

### Problem

`0.35.0` hardened the release gate and exposed Windows/Cargo stderr edge cases.
`0.36.0` should keep the gate stable while adding DB rollout validation.

### Desired Outcome

The release gate should make DB rollout confidence visible without becoming
too slow or flaky.

### Candidate Work

- Add a lightweight DB rollout/soak check to the release-readiness script if it
  is deterministic enough.
- Document `CARGO_BUILD_JOBS=1` as a Windows workaround for intermittent
  `LNK1104` linker file locks during full workspace gates.
- Keep Docker-dependent checks optional and non-fatal when Docker is absent.

### Acceptance Criteria

- Full release readiness still passes on Windows.
- The release guide explains the serial build workaround.
- DB rollout validation is included or documented with a clear command.

## Proposed Verification

`0.36.0` should pass at least:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo test --workspace --all-targets --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --doc --workspace --locked`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`
- `.\scripts\verify-release-readiness.ps1 -Version 0.36.0 -RunGate`
- DB rollout/soak validation command added during the release.

Optional checks:

- SQLx Postgres testcontainers smoke test.
- Longer DB soak/load scenario.
- Consumer check after crates.io publish.

