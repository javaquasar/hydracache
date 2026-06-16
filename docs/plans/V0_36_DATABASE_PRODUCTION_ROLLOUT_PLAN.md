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
- macro ergonomics remove repetitive key/tag/policy boilerplate without hiding
  cache boundaries;
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

## 7. Macro Ergonomics And Boilerplate Reduction

### Problem

`0.35.0` makes database caching explicit enough for production review, but the
macro layer still removes only part of the user-facing ceremony.

The current macros are safe and honest:

- `HydraCacheEntity` removes manual `CacheEntity` implementations;
- `query_cache_policy!` shortens declarative `QueryCachePolicy` construction;
- `cacheable!` and `cacheable_infallible!` shorten local-cache call sites.

The remaining user pain is around repeated metadata:

- entity id types are repeated in `#[hydracache(..., id = Type)]`;
- key and tag dimensions are still often built with strings or `format!`;
- freshness intent is separate from `query_cache_policy!`;
- hot repository methods still need explicit prepared-policy builder chains;
- ordinary function caching still requires a function-like macro around a
  loader closure instead of a function attribute.

The goal is not to make HydraCache magical. The macro layer should remain sugar
over the explicit API. It should not discover a global cache, infer database
transactions, parse SQL, hide tenant/security dimensions, or bypass the
repository/service layer.

### Desired Outcome

Improve macro ergonomics enough that a production user can express common
cache policies declaratively, while reviewers can still see every key, tag,
TTL, stale budget, and invalidation dimension.

The target state is:

- less repeated metadata;
- fewer raw string/`format!` key constructions;
- policy macros that encode freshness intent;
- prepared repository metadata that is easy to declare once;
- optional function attributes for non-database expensive work;
- compile-time diagnostics and tests for every new macro form.

### 7.1 `HydraCacheEntity` Id Inference

#### Current Shape

Today users repeat the id type in the macro attribute:

```rust
use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User {
    id: i64,
}
```

This works, but it repeats the same fact twice. If the field type changes and
the attribute does not, the generated metadata can drift away from the domain
model.

#### Target Shape

Allow an id field marker so the derive macro can infer the id type:

```rust
use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
}
```

The explicit `id = Type` form should remain supported for tuple structs,
computed ids, generated models, or teams that prefer keeping metadata outside
fields.

#### Acceptance Criteria

- `#[hydracache(id)]` infers `CacheEntity::Id` from a named struct field.
- The old `id = Type` form remains supported and documented.
- The derive rejects multiple `#[hydracache(id)]` fields.
- The derive rejects missing id metadata when neither form is present.
- The derive rejects conflicting `id = Type` plus field marker unless a clear
  compatibility rule is chosen and tested.

### 7.2 `query_cache_policy!` Presets And Freshness

#### Current Shape

Today `query_cache_policy!` can express key/tag/TTL metadata:

```rust
use hydracache_db::query_cache_policy;

let policy = query_cache_policy!(
    name = "load-user",
    entity = User,
    id = user_id,
    tag = format!("tenant:{tenant_id}"),
    ttl_secs = 60,
);
```

The production freshness model still has to be added outside the macro with
builder calls. That makes the macro less useful for `0.36.0` freshness-budget
guidance.

#### Target Shape

Let the policy macro encode common presets and bounded stale/refresh intent:

```rust
use hydracache_db::query_cache_policy;

let policy = query_cache_policy!(
    preset = read_mostly,
    name = "load-user",
    entity = User,
    id = user_id,
    tag = format!("tenant:{tenant_id}"),
    refresh_ahead_secs = 10,
    stale_while_revalidate_secs = 300,
);
```

The macro should remain declarative. It should produce the same
`QueryCachePolicy` a user could build by hand with `QueryCachePolicy` and
`RefreshPolicy`.

#### Acceptance Criteria

- `preset = short_lived`, `read_mostly`, `per_entity`,
  `no_ttl_explicit_invalidation`, and `negative_cache` map to the documented
  policy presets.
- `refresh_ahead_secs`, `stale_while_revalidate_secs`, and
  `stale_on_loader_error_secs` map to `RefreshPolicy`/`RefreshOptions` without
  changing runtime semantics.
- Conflicting freshness options produce compile-fail diagnostics.
- Runtime tests verify that generated policies encode the same TTL/stale
  behavior as explicit builder code.

### 7.3 Safe Key And Tag Segment DSL

#### Current Shape

For list/search queries, the safest current approach is explicit but verbose:

```rust
use hydracache::CacheKeyBuilder;
use hydracache_db::query_cache_policy;

let key = CacheKeyBuilder::new()
    .segment("tenant")
    .segment(tenant_id)
    .segment("permission")
    .segment(permission_hash)
    .segment("q")
    .segment(query)
    .segment("page")
    .segment(page)
    .segment("sort")
    .segment(sort)
    .build_string();

let policy = query_cache_policy!(
    name = "search-users",
    key = key,
    collection_tag = "users",
    ttl_secs = 30,
);
```

This is reviewable, but it invites repeated `format!` usage and makes it easy
to forget result-shaping dimensions.

#### Target Shape

Add segment-oriented macro options for keys and tags:

```rust
use hydracache_db::query_cache_policy;

let policy = query_cache_policy!(
    name = "search-users",
    key_segments = [
        "tenant", tenant_id,
        "permission", permission_hash,
        "q", query,
        "page", page,
        "sort", sort,
    ],
    collection_tag = "users",
    ttl_secs = 30,
);
```

For richer invalidation metadata, support tag segment groups:

```rust
let policy = query_cache_policy!(
    name = "search-users",
    key_segments = [
        "tenant", tenant_id,
        "permission", permission_hash,
        "q", query,
        "page", page,
        "sort", sort,
    ],
    tag_segments = [
        ["tenant", tenant_id],
        ["users"],
    ],
    ttl_secs = 30,
);
```

The macro should build through the same escaping rules as `CacheKeyBuilder` and
`TagSet`, so the segment DSL is safer than manual string concatenation.

#### Acceptance Criteria

- `key_segments = [...]` generates a physical key through `CacheKeyBuilder`.
- `tag_segments = [[...], [...]]` generates tags through the same segment
  escaping rules.
- The macro rejects `key` plus `key_segments` conflicts.
- The macro rejects malformed segment groups with clear compile diagnostics.
- Tests cover tenant, permission, filter, pagination, sort, locale, region,
  feature flag, and time-window examples.

### 7.4 Prepared Policy Macro

#### Current Shape

Today hot repository methods can prepare stable metadata with builder code:

```rust
use hydracache_db::PreparedQueryPolicy;

let load_user = queries.prepare::<User>(
    PreparedQueryPolicy::per_entity()
        .cache_entity::<User>()
        .with_name("load-user")
        .ttl(std::time::Duration::from_secs(300)),
);

let user = load_user
    .load_id(user_id, move || async move {
        Ok::<_, std::io::Error>(repo.load_user(user_id).await?)
    })
    .await?;
```

This is clear but still repetitive when many repository methods share the same
shape.

#### Target Shape

Add a declarative prepared-policy macro:

```rust
use hydracache_db::prepared_query_policy;

let load_user = prepared_query_policy!(
    per_entity = User,
    name = "load-user",
    ttl_secs = 300,
);

let user = queries
    .prepare::<User>(load_user)
    .load_id(user_id, move || async move {
        Ok::<_, std::io::Error>(repo.load_user(user_id).await?)
    })
    .await?;
```

The macro should only declare reusable cache metadata. Query execution,
database transactions, and loader ownership remain in repository code.

#### Acceptance Criteria

- The macro supports entity, collection, and manual-key prepared policy forms.
- Generated prepared policies match explicit `PreparedQueryPolicy` builder
  output in tests.
- The macro is re-exported from `hydracache-db` and adapter crates where it is
  useful.
- Compile-fail tests cover missing key source, conflicting key sources,
  duplicate options, and unsupported options.

### 7.5 Attribute Macro For Ordinary Functions

#### Current Shape

Today ordinary function caching uses a function-like macro around a loader:

```rust
use hydracache::{cacheable, HydraCache};

let cache = HydraCache::local().build();
let profile_id = 42_u64;

let profile = cacheable!(
    cache = cache,
    key = format!("profile:{profile_id}"),
    tags = ["profiles"],
    ttl_secs = 60,
    load = move || async move {
        Ok::<_, LoadError>(load_profile(profile_id).await?)
    },
)
.await?;
```

This keeps the boundary explicit, but the call site still needs a loader
closure and repeated key/tag metadata.

#### Target Shape

Explore an attribute macro for ordinary expensive async functions:

```rust
use hydracache::cacheable;

#[cacheable(
    cache = cache,
    key_segments = ["profile", profile_id],
    tags = ["profiles"],
    ttl_secs = 60
)]
async fn load_profile(profile_id: u64) -> Result<Profile, LoadError> {
    repo.load_profile(profile_id).await
}
```

This should be treated as the highest-risk macro ergonomics item. It must not
discover a global cache, hide key dimensions, or interfere with database
transactions. It should be sugar over the same explicit cache call a user would
write by hand.

#### Acceptance Criteria

- The attribute macro requires an explicit cache expression or an explicit
  generated-argument convention.
- It supports explicit `key`, `key_segments`, `tags`, and TTL options.
- It does not target SQLx/Diesel/SeaORM query functions until the transaction
  and loader ownership story is reviewed.
- Compile-fail tests cover missing cache, missing key metadata, unsupported
  options, and conflicting TTL/key options.

### Testing Requirements For Macro Work

Every new macro feature in `0.36.0` must land with tests. Documentation-only
examples are not enough for macro work.

Required test layers:

- Unit tests in `crates/hydracache-macros` for parsing, validation, and token
  generation.
- `trybuild` pass fixtures for the public user-facing syntax.
- `trybuild` compile-fail fixtures for diagnostics and invalid combinations.
- Runtime tests in `hydracache`, `hydracache-db`, or adapter crates when the
  generated code affects cache behavior.
- Re-export tests for `hydracache-db`, `hydracache-sqlx`,
  `hydracache-diesel`, and `hydracache-seaorm` when a new macro is intended to
  be available through those crates.
- README or crate-doc examples for stable syntax after implementation.

At minimum, macro implementation work should pass:

- `cargo test -p hydracache-macros --locked`
- `cargo test -p hydracache --test cacheable_ui --locked`
- `cargo test -p hydracache-db --test derive_ui --locked`
- the runtime tests for whichever crate receives generated behavior.

## 8. Release Gate Updates

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
- Macro-specific tests for every implemented macro ergonomics item.

Optional checks:

- SQLx Postgres testcontainers smoke test.
- Longer DB soak/load scenario.
- Consumer check after crates.io publish.
