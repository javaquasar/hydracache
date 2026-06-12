# HydraCache 0.34.0 Production Validation Plan

`0.34.0` is a production-confidence release. The goal is not to add a large new
feature family, but to make the existing local cache, function cache, database
cache adapters, observability layer, and release process easier to validate in a
real service.

The release intentionally keeps the public direction from `0.33.0`: local-first,
embedded, explicit invalidation, optional database adapters, optional cluster
synchronization, and no hidden database/query engine replacement.

## Release Theme

Make HydraCache easier to trust before a production rollout:

- Show one coherent production-style example instead of isolated snippets.
- Stress the most dangerous correctness boundaries.
- Stabilize the observability surface enough for dashboards and alerts.
- Verify feature combinations so users can keep dependency footprint small.
- Add practical policy-selection guidance.
- Harden the post-publish release flow.

## Non-Goals

- Do not introduce external cache servers or mandatory network components.
- Do not replace SQLx, Diesel, SeaORM, or custom repository code.
- Do not add implicit SQL parsing or transparent query interception.
- Do not preserve old experimental API if simplifying the production path
  requires cleanup.
- Do not make sandbox-only helpers part of the stable public API.

## 1. Unified Production Example

### Problem

The README and crate docs show many focused examples. That is useful for API
learning, but it does not answer the production question:

> What does one real application look like when HydraCache is wired around
> local caching, SQLx, Diesel, SeaORM, invalidation, refresh/stale behavior,
> and observability?

### Desired Outcome

Add one unified production-style example that:

- Uses one logical database model shared by SQLx, Diesel, and SeaORM demos.
- Uses one `HydraCache` runtime and one database-neutral policy vocabulary.
- Demonstrates cache hit/miss behavior with loader-call counters.
- Demonstrates entity invalidation and collection invalidation.
- Demonstrates `RefreshPolicy` / `RefreshOptions` for stale reads.
- Demonstrates observability snapshots after the workload.
- Can be executed as a test or example without requiring Docker.
- Uses SQLite in memory or an embedded deterministic repository where an ORM
  dependency would otherwise make the example too fragile.

### Implementation Shape

- Prefer a new integration test/example module over a large README-only sample.
- Keep the example small enough to compile quickly.
- Reuse the existing adapter APIs instead of inventing sandbox-only paths.
- If a real ORM setup is too heavy for one example, model the shared production
  workflow with the database-neutral `hydracache-db` API and keep adapter
  equivalence covered by adapter tests.

### Acceptance Criteria

- The example compiles under the workspace test suite.
- It exercises at least:
  - repeated reads becoming cache hits,
  - invalidation forcing reload,
  - stale fallback or stale refresh behavior,
  - diagnostics/stats reporting.
- README links to the example and explains when to use it.
- Release notes mention the production validation example.

## 2. Correctness Stress Suite

### Problem

The riskiest cache bugs happen around timing:

- refresh starts while invalidation is in flight,
- stale reads hide loader failures,
- tag invalidation races with an in-flight load,
- background reload stores a stale value after a newer write,
- concurrent callers accidentally fan out loader work.

Several of these are already tested, but `0.34.0` should gather the most
important production scenarios into an intentional correctness suite.

### Desired Outcome

Add tests that make the race boundaries explicit:

- `refresh_ahead` must not overwrite a value invalidated during refresh.
- `stale_while_revalidate` must return stale quickly and eventually refresh.
- `stale_on_loader_error` must be bounded by the configured stale window.
- `invalidate_tag` during loader execution must force the next caller to reload.
- concurrent refresh callers should not explode loader calls for the same key.
- database-cache refresh policy should preserve the same semantics as local
  cache refresh.

### Implementation Shape

- Place local-cache race tests near existing invalidation/refresh tests.
- Place database refresh stress tests in `hydracache-db` tests.
- Use deterministic `tokio::sync` primitives where possible instead of sleeps.
- Keep sleeps short and bounded only where TTL behavior needs real time.

### Acceptance Criteria

- New tests are deterministic and pass on Windows and CI.
- No ignored tests are needed for the new correctness suite.
- Tests fail for the dangerous behavior they are meant to protect against.
- Release notes list the new correctness coverage.

## 3. Observability Contract

### Problem

HydraCache already exposes stats, diagnostics, events, observability registry,
and actuator routes. Production users need to know what fields are safe to build
dashboards and smoke checks around.

### Desired Outcome

Document and test an observability contract:

- Stable cache stats fields:
  - hits,
  - misses,
  - loads,
  - load failures,
  - invalidations,
  - stale load discards,
  - single-flight joins,
  - bus publish/receive failures when relevant.
- Stable diagnostic concepts:
  - hit ratio,
  - total requests,
  - cache activity,
  - cluster role/node/generation when cluster is enabled.
- Stable actuator JSON route shape for:
  - health,
  - cache list,
  - cache overview,
  - individual cache stats/diagnostics.

### Implementation Shape

- Add a dedicated `docs/OBSERVABILITY_CONTRACT.md`.
- Add focused tests that serialize or inspect representative snapshots/routes.
- Avoid over-promising exact formatting for human-readable debug strings.
- Clearly mark additive fields as allowed in v0.

### Acceptance Criteria

- The contract document is linked from README and production guide.
- Tests assert required fields or route shape.
- The contract says what is stable now and what remains v0/additive.
- Release notes mention the contract.

## 4. Feature Matrix

### Problem

HydraCache has many optional crates. Users need confidence that they can depend
only on the pieces they need:

- local cache only,
- database-neutral cache only,
- SQLx only,
- Diesel only,
- SeaORM only,
- actuator only,
- cluster crates only.

Without explicit checks, accidental dependency coupling can creep in.

### Desired Outcome

Add a feature/dependency matrix document and lightweight compile verification.

### Implementation Shape

- Add `docs/FEATURE_MATRIX.md`.
- Add a script that performs fast package-level checks for common combinations.
- Keep the script deterministic and easy to run locally.
- Avoid trying to solve every possible feature permutation.

### Minimum Matrix

- `hydracache-core`
- `hydracache`
- `hydracache-db`
- `hydracache-sqlx`
- `hydracache-diesel`
- `hydracache-seaorm`
- `hydracache-observability`
- `hydracache-actuator-axum`
- `hydracache-cluster-chitchat`
- `hydracache-cluster-raft`
- `hydracache-cluster`
- `hydracache-cluster-transport-axum`

### Acceptance Criteria

- The matrix is documented.
- A script can check the supported combinations.
- The script is referenced from testing and publishing docs.
- At least one test or CI-friendly command covers the script path.

## 5. Policy Selection Guide

### Problem

`0.33.0` added policy presets, but users still need practical advice:

- Which policy should I use for product catalogs?
- Should permission checks use negative caching?
- When is stale-while-revalidate safe?
- Which tags should be attached to list queries?
- What should not be cached?

### Desired Outcome

Add a practical policy guide with examples.

### Implementation Shape

- Extend `docs/PRODUCTION_GUIDE.md` or add `docs/POLICY_GUIDE.md`.
- Link the guide from README.
- Keep examples aligned with actual API.
- Cover both local cache and database cache use.

### Required Scenarios

- User/profile by id.
- Product catalog/read-mostly data.
- Search/list results.
- Permission/authorization checks.
- Negative cache for missing rows.
- Explicit-invalidation-only cache entries.
- Refresh-ahead for hot keys.
- Stale-on-loader-error for fragile upstreams.

### Acceptance Criteria

- The guide contains code examples or concrete API snippets.
- Examples compile if embedded in rustdoc, or are clearly marked as policy
  sketches.
- Release notes mention the policy guide.

## 6. Post-Publish Automation

### Problem

Publishing a multi-crate workspace has ordering and registry propagation
constraints:

- bootstrap crates must publish first,
- runtime crate waits for bootstrap,
- adapters wait for runtime and database-neutral crates,
- consumer check runs only after crates.io has the new version.

Manual release is possible but easy to mis-order.

### Desired Outcome

Strengthen release scripts and docs so a maintainer has a predictable staged
flow.

### Implementation Shape

- Add a release checklist script that prints or verifies:
  - current workspace version,
  - matching local tag,
  - clean tracked tree,
  - publish order,
  - commands to run.
- Keep publishing itself explicit unless the script is already safe enough.
- Preserve existing `package-publishable.ps1` and
  `verify-crates-io-consumer.ps1`.
- Add docs for the exact staged sequence.

### Acceptance Criteria

- The new script has tests or at least a deterministic dry-run mode.
- Publishing docs reference the script.
- Release notes mention the release automation hardening.
- The final release gate includes:
  - fmt,
  - check,
  - test,
  - clippy,
  - doctest,
  - docs with warnings denied,
  - package checks,
  - post-publish consumer check.

## Step-by-Step Execution

1. Add this plan and `docs/releases/0.34.0.md`.
2. Implement the unified production example and tests.
3. Add correctness stress tests for refresh/stale/invalidation boundaries.
4. Add observability contract docs and tests.
5. Add feature matrix docs and verification script.
6. Add policy guide docs and README links.
7. Add post-publish automation/dry-run script and publishing docs.
8. Bump workspace version to `0.34.0`.
9. Run the full release gate.
10. Publish in the staged order after the user confirms/tagging is ready.

## Verification Commands

The release should pass:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
.\scripts\package-publishable.ps1 -Set bootstrap -AllowDirty
.\scripts\verify-feature-matrix.ps1
.\scripts\verify-release-readiness.ps1 -Version 0.34.0 -DryRun
```

After publishing:

```powershell
.\scripts\verify-crates-io-consumer.ps1 -Version 0.34.0
```
