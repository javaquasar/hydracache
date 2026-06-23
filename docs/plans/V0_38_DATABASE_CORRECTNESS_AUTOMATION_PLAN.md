# HydraCache 0.38.0 Database Correctness Automation Plan

> **At a glance**
> - **What:** SQL dependency lint, generated DB hooks + CDC connector, named consistency modes, dimension profiles, SQLx transaction companion, reconciliation drift.
> - **Why:** make invalidation correctness **assisted and checkable**, not manual TTL guessing.
> - **After (depends on):** 0.37.
> - **Unblocks:** 0.39 (starts the cluster track).
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md)

Status: implemented in `0.38.0`. Release notes are in
`docs/releases/0.38.0.md`.

`0.38.0` builds on the `0.37.0` database production-hardening release by adding
**assisted correctness automation** on top of the explicit database result-cache
model. It does not change the product class: HydraCache stays an explicit,
embedded, local-first database result cache. `0.38.0` only makes common
correctness mistakes visible in CI, runtime diagnostics, sandbox examples, and
release gates.

This document is detailed enough to be implemented step by step by another
engineer or agent. Every new code path described here ships with tests, and the
testing subsections name concrete files, test functions, assertions, and the
exact `cargo` invocations.

---

## Positioning: Assisted vs Fully Automatic

The strategic framing comes from `V0_38_COMPLEXITY_NOTES.md` and must be made
public (README + release notes), not buried in an internal note.

> HydraCache is an explicit database result-cache layer with production
> correctness *assistance*.

It is explicitly **not**:

> HydraCache automatically understands and invalidates every database result in
> every topology.

The second promise is attractive but is a different and far riskier product
class. "Fully automatic" would require a database proxy, an ORM second-level
cache, a CDC platform, and a distributed transaction coordinator combined. From
the complexity notes:

| Direction | Internal complexity estimate | Notes |
| --- | ---: | --- |
| Assisted correctness mode (this release) | ~6.5/10 | Realistic if incremental and opt-in. |
| Fully automatic DB cache | ~9.5-10/10 | Requires becoming database middleware. |

Those numbers are *internal complexity estimates only*. They are deliberately
**not** used as release criteria — see the boolean Release Gates and Final
Release Decision sections. There is no "9.4-9.6/10 product score" anywhere in
this plan.

What "assisted" means concretely:

- users still declare cache keys, tags, dependencies, freshness, and write-side
  invalidation; declarations remain the canonical source of truth;
- HydraCache can *verify many declarations* against SQL/parser/catalog evidence
  off-runtime and in CI;
- database hooks can make external writes visible through the same outbox
  contract from `0.37`;
- consistency-sensitive flows can request a *named* consistency behavior;
- CI can fail when production policies miss required business dimensions;
- transaction helper APIs reduce boilerplate while leaving commit/rollback
  ownership visible;
- operators can observe dependency-lint failures, hook lag, outbox lag,
  consistency timeouts, missing dimensions, and reconciliation drift.

---

## Non-Goals (with ReadySet anti-scope)

- Do not claim perfect SQL dependency detection (dynamic SQL, views, functions,
  stored procedures, RLS, triggers, ORM builders, and external writers make it
  undecidable in practice).
- Do not parse arbitrary dynamic SQL as a correctness *guarantee*; the linter
  reports `Inconclusive` instead of `Clean`.
- Do not invalidate from database writes unless hooks, outbox, CDC connector, or
  a user-provided bridge is configured.
- Do not provide global serializable consistency across unavailable nodes.
- Do not infer hidden business dimensions such as authorization scope.
- Do not own every database transaction in the application.
- Do not require triggers, CDC, or generated hooks for simple local-first users.
- Do not add a required external broker.
- **Do not turn HydraCache into a transparent Postgres/MySQL proxy.**

### ReadySet/Noria anti-scope (explicit)

HydraCache emits **invalidation intent** only and never sits in the data path.
The following ReadySet-class capabilities are permanent non-goals and are tied
to the positioning above:

- no dataflow engine;
- no MIR/SQL compiler or query planner;
- no partial materialization, upquery, or replay;
- no RocksDB/embedded materialized state;
- no SQL wire-proxy;
- no value-serving from change events.

CDC, when present, exists **only as a connector crate** (for example
`hydracache-cdc-postgres`) that converts change events into invalidation intent
published onto the existing invalidation bus. It re-uses the same outbox/intent
contract; it does not re-execute queries or serve values. ReadySet here is a
"where not to go" boundary, not an implementation template.

---

## Inherited from 0.37 Boundary

Division of ownership with `0.37` must be explicit to avoid the two releases
re-implementing overlapping code.

| Concern | Owner | Notes |
| --- | --- | --- |
| `required_dimensions` **mechanism** (static macro/policy check that named labels exist) | **0.37** | Cheap, isolated static check at macro/policy level. `0.38` consumes it. |
| `required_dimensions` **profiles** (`tenant_scoped`, `paged_search`, …) + CI `deny` mode | **0.38** | This release. See Section 4. |
| Transactional invalidation outbox table + worker + idempotency key | **0.37** | `0.38` hooks and reconciliation depend on it. |
| Read-after-write receipts/barriers (`InvalidationWait`, local + best-effort) | **0.37** | `0.38` builds named consistency modes on top. `quorum` matures in 0.40. |
| Attribute macro on repository methods, `prepared_query_policy!` | **0.38** | Moved into `0.38` from `0.37` (0.37 was overloaded). |
| SQL-dependency lint | **0.38** | Moved into `0.38`. Off-runtime, opt-in CI. |
| Full testcontainers matrices (Diesel/SeaORM × PG/MySQL) | **0.38** | Moved into `0.38`. Minimum mandatory matrix is small; rest is `#[ignore]`. |

Anything `0.38` consumes from `0.37` must be guarded: if `0.37` ships a
hedged item as a documented `NotImplemented` stub, `0.38` features built on it
degrade to the same stub rather than silently breaking.

---

## Dependency Graph on 0.37

```text
0.37 deliverables                         0.38 features that depend on them
-----------------                         ---------------------------------
outbox table + worker        ───────────► Section 2 generated DB hooks
  + idempotency key (txid,                  (hooks write outbox rows)
   sha256(target))           ───────────► Section 5 transaction companion (durable mode)
                             ───────────► Section 6 reconciliation (outbox backlog signal)

hook_schema version table    ───────────► Section 6 reconciliation (hook/schema drift signal)
(0.37 ships the table;
 0.38 generates the hooks)

read-after-write receipts/   ───────────► Section 3 named consistency modes
barriers (local+best-effort)               (Eventual / *ReadYourWrites / FailClosed / DegradedOk)

required_dimensions MECHANISM ──────────► Section 4 profiles + CI deny mode

TagSet escaping + key model  ───────────► Sections 2,5 (outbox intent serialization)
```

Hard blocking edges (must exist before the dependent 0.38 item can be claimed
as delivered):

- **Section 2 generated hooks ← `0.37` outbox table.** No outbox table → hooks
  have nowhere to write; ship hook *renderers* (pure SQL generation, fully
  testable) but mark runtime install as deferred stub if outbox is stubbed.
- **Section 6 reconciliation ← `0.37` outbox backlog + hook versions.** These
  are the two mandatory drift signals (see Section 6). If either signal source
  is stubbed in `0.37`, reconciliation degrades to the available signal and
  documents the gap.

---

## Global Test and Commit Rule

Every new code path in `0.38.0` must be covered by tests appropriate to its
risk, and each implementation step is committed only after its focused tests
pass:

- parser/linter logic: unit fixtures + negative cases (`unit`/`property`);
- a `cargo deny check` + `cargo tree` test that asserts `sqlparser` is absent
  from the runtime (non-dev) dependency graph;
- new macro syntax: passing and failing `trybuild` tests;
- `compile_error!` deferral stubs: `trybuild` compile-fail tests proving the
  error message and feature gating;
- DB hook generation: SQL snapshot tests + SQLite runtime tests + `#[ignore]`
  Docker tests for PG/MySQL;
- transaction helper: commit, rollback, retry, enqueue-failure, commit-failure
  paths;
- consistency modes: in-process multi-node tests + timeout/degraded tests;
- diagnostics/actuator: serialization + counter-movement tests;
- optional Docker-backed tests: documented and `#[ignore]`d cleanly when not
  requested.

---

## 1. SQL Dependency Assistant and Strict Lint Mode

Status: implemented in `0.38.0`. Owner: 0.38 (moved from 0.37).

### (a) Problem / motivation

`0.37` makes dependencies explicit, e.g. `depends_on = [table("users"),
table("user_roles")]`. That is correct as the source of truth but still depends
on a human declaring the right list. A reviewer must *notice* that `roles` or
`user_roles` is missing. `0.38` adds an assistant that compares declared
dependencies against evidence from SQL text, SQLx metadata, Diesel rendered SQL,
SeaORM statements, and (optionally) database catalogs.

### (b) Design / contract

The dependency dictionary is modelled on ReadySet's
`MirNode.owners: HashSet<Relation>`: a query declares the base `Relation`s it
reads; the lint reference-counts referenced relations and reports declared-vs-
observed differences. The mechanism is reference-counted so a relation
referenced by zero remaining queries is GC'd from the lint index (mirrors
ReadySet's orphaned-node GC).

**Hard invariant: the parser is strictly off-runtime.** `sqlparser` must never
be a dependency of any runtime crate, even transitively. The split mirrors
`sqlx`: heavy build logic lives in `sqlx-macros-core`, and `sqlx-core`
(runtime) does **not** depend on it. Concretely, the parser and lint engine live
in a new crate **`hydracache-sql-lint`** that is invoked only at build/CI time.
Runtime crates (`hydracache-core`, `hydracache`, `hydracache-db`,
`hydracache-sqlx`) must not depend on it outside `dev-dependencies`. A
`deny.toml` rule makes this a machine-checkable invariant.

Lint statuses:

- `Clean` — every observed relation is declared.
- `MissingDependencies(Vec<Relation>)` — observed but not declared.
- `ExtraDependencies(Vec<Relation>)` — declared but not observed (informational).
- `Inconclusive(reason)` — dynamic SQL, unsupported syntax, or dialect gap.

Modes: `Warn`, `DenyMissingDependencies`. `Inconclusive` never silently maps to
`Clean`.

### (c) Rust sketch

```rust
// crates/hydracache-sql-lint/src/lib.rs  (NEW build/CI-only crate)
use sqlparser::dialect::{PostgreSqlDialect, MySqlDialect, SQLiteDialect};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Relation {
    pub schema: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintStatus {
    Clean,
    MissingDependencies(Vec<Relation>),
    ExtraDependencies(Vec<Relation>),
    Inconclusive(String),
}

#[derive(Debug, Clone, Copy)]
pub enum SqlDialect { Postgres, MySql, Sqlite }

#[derive(Debug, Clone, Copy)]
pub enum DependencyLintMode { Warn, DenyMissingDependencies }

pub struct DependencyLint {
    dialect: SqlDialect,
    mode: DependencyLintMode,
}

impl DependencyLint {
    pub fn new(dialect: SqlDialect, mode: DependencyLintMode) -> Self { /* ... */ }

    /// Reference-counted observed-relation index (ReadySet `owners` model).
    pub fn observed_relations(&self, sql: &str) -> Result<Vec<Relation>, LintError> { /* ... */ }

    pub fn check(&self, sql: &str, declared: &[Relation]) -> LintStatus { /* ... */ }
}
```

```rust
// crates/hydracache-db/src/lint.rs  (runtime-side metadata ONLY, no parser)
// The policy macro records sql text + declared relations as plain metadata.
// Linting happens in hydracache-sql-lint at CI time over this metadata.
pub struct PolicyLintMetadata {
    pub name: &'static str,
    pub sql: Option<&'static str>,
    pub declared: &'static [DeclaredRelation],
    pub mode: DeclaredLintMode, // Warn | DenyMissingDependencies
}
```

Macro surface (in `hydracache-macros`, re-exported via `hydracache-db`):

```rust
let policy = query_cache_policy!(
    name = "load-user-permissions",
    key_segments = ["tenant", tenant_id, "user", user_id, "permissions"],
    sql = "select u.id, r.name from users u
           join user_roles ur on ur.user_id = u.id
           join roles r on r.id = ur.role_id",
    depends_on = [table("users"), table("user_roles"), table("roles")],
    dependency_lint = deny_missing_dependencies,
    ttl_secs = 300,
);
```

Optional catalog-assisted helper (testcontainers only):

```rust
// crates/hydracache-sql-lint/src/catalog/postgres.rs
let report = PgDependencyCatalog::connect(&pool).await?
    .expand_views(true)
    .lint_policy(&policy_metadata)
    .await?;
```

### (d) Implementation steps

1. Create crate `hydracache-sql-lint` with `sqlparser` as a normal dependency;
   it is NOT a workspace runtime dependency.
2. Implement `Relation`, `LintStatus`, `observed_relations`, `check` for
   Postgres/MySQL/SQLite dialects.
3. Add `PolicyLintMetadata` to `hydracache-db` (no parser dependency).
4. Extend `query_cache_policy!` in `hydracache-macros` to accept `sql` and
   `dependency_lint`, emitting `PolicyLintMetadata`.
5. Add a CI binary/test harness that collects all policies and runs the lint.
6. Add `deny.toml` ban on transitive `sqlparser` in the non-dev graph.
7. Add Postgres catalog helper (`pg_catalog` view expansion) behind a feature.
8. Wire counters into the actuator (Section 7).

### (e) TESTING

Unit/parser (`crates/hydracache-sql-lint/tests/parser.rs`, `cargo test -p hydracache-sql-lint`):

- `fn observes_single_table()` — `select * from users` → `[users]`.
- `fn observes_three_way_join()` — asserts `{users, user_roles, roles}`.
- `fn resolves_aliases()` — `users u` resolves to `users`.
- `fn schema_qualified()` — `app.users` → `Relation{schema:Some("app"),name:"users"}`.
- `fn cte_and_subquery()` — CTE name is NOT reported as a base relation.
- `fn ignores_string_literals_and_comments()` — `'users'` / `-- users` excluded.
- `fn dialect_placeholders()` — `$1` (PG), `?` (MySQL/SQLite) parse cleanly.

Property (`tests/parser_prop.rs`, `proptest`):

- `fn declared_superset_is_never_missing()` — if declared ⊇ observed, status is
  never `MissingDependencies`.

Negative (`tests/lint_modes.rs`):

- `fn warn_mode_reports_missing()` — missing `roles` → `MissingDependencies`.
- `fn deny_mode_fails_on_missing()` — `check` non-clean in deny mode.
- `fn dynamic_sql_is_inconclusive()` — concatenated SQL → `Inconclusive`, never `Clean`.
- `fn unsupported_syntax_does_not_panic()` — returns `Inconclusive`, no panic.

Deny / cargo tree invariant (`crates/hydracache-db/tests/runtime_dep_isolation.rs`, integration):

- `#[test] fn sqlparser_absent_from_runtime_graph()` — runs
  `cargo tree -p hydracache-db --edges normal --prefix none` via
  `std::process::Command`, asserts output does NOT contain `sqlparser`.
- `#[test] fn cargo_deny_runtime_ban_passes()` — runs `cargo deny check bans`,
  asserts exit code 0; `deny.toml` contains a `[[bans.deny]]` entry for
  `sqlparser` scoped to the non-dev profile.
- Exact invocation: `cargo test -p hydracache-db --locked runtime_dep_isolation`.

Macro/compile (`crates/hydracache-db/tests/policy/`, `trybuild`):

- `pass_sql_lint_warn.rs` — `sql = "...", dependency_lint = warn` compiles.
- `pass_sql_lint_deny.rs` — `dependency_lint = deny_missing_dependencies` compiles.
- `fail_unknown_lint_mode.rs` (+`.stderr`) — unknown mode is a compile error.
- `fail_duplicate_sql.rs` (+`.stderr`) — duplicate `sql` key rejected.

Catalog integration (`crates/hydracache-sql-lint/tests/postgres_catalog.rs`, `#[ignore]`):

- `#[ignore] fn pg_view_dependency_expansion()` — view over `users` expands to
  base table. Run: `cargo test -p hydracache-sql-lint --test postgres_catalog --locked -- --ignored`.
- `#[ignore] fn pg_missing_permission_clear_error()` — readonly role without
  catalog grant returns an actionable error, not a panic.

### (f) Pros

- Reviewers see missing table dependencies before production.
- CI can fail on obvious policy mistakes; runtime metadata stays canonical.
- View dependencies expand when the catalog can provide them.

### (g) Risks / fallback

- False positives on dynamic SQL/RLS/functions → mitigated by `Inconclusive`.
- Dialect drift → covered by per-dialect fixtures.
- **Fallback**: if catalog expansion does not land, ship parser-only linting and
  a documented `NotImplemented` stub for `PgDependencyCatalog` + an ADR; this
  still satisfies the work item.

### (h) Suppression and baseline (incremental adoption)

Motivation: a lint with no escape hatch is unadoptable on an existing codebase —
a single legitimately-inconclusive policy in `deny` mode blocks the whole CI, so
teams just disable the lint. The lint must support **inline suppression** for
known-acceptable cases and a **baseline** so a codebase can turn `deny` on for
new code without first fixing every pre-existing finding.

Design / contract:

- **Inline suppression** at the policy site, with a mandatory reason, so it is
  greppable and reviewable:

```rust
query_cache_policy! {
    name = "tenant_dashboard",
    sql = "select ... from dynamic_view",
    dependency_lint = deny_missing_dependencies,
    // suppress a specific finding; reason is required and recorded in metadata
    lint_allow = [(LintFinding::Inconclusive, "dynamic SQL audited 2026-06, see TICKET-123")],
}
```

  A bare `lint_allow` without a reason is a compile error (mirrors the
  required-reason discipline of `required_dimensions`).

- **Baseline file** `lint-baseline.json` records the fingerprint
  (`sha256(policy_name + relation_set + finding)`) of every currently-accepted
  finding. In `deny` mode the checker fails only on findings **absent** from the
  baseline; a finding that disappears is reported as "baseline entry now stale"
  so the file can be pruned. Baseline is regenerated with
  `cargo run -p hydracache-sql-lint --bin lint -- --update-baseline`.

- Suppression and baseline are **off-runtime** like the rest of the lint; they
  live in the lint crate and CI harness only.

Implementation steps:

1. Add `LintFinding` + `lint_allow` parsing to `query_cache_policy!`
   (`hydracache-macros`), emitting suppressions into `PolicyLintMetadata`.
2. Add `Baseline { entries: HashSet<Fingerprint> }` load/save + the
   `--update-baseline` bin flag to `hydracache-sql-lint`.
3. The CI harness applies suppressions first, then diffs remaining findings
   against the baseline.

TESTING (`crates/hydracache-sql-lint/tests/suppression_baseline.rs`):

- `fn suppressed_finding_does_not_fail_deny()` — a finding listed in
  `lint_allow` is dropped before the deny check.
- `fn baseline_finding_does_not_fail_but_new_one_does()` — a baselined finding
  passes; an identical finding on a *different* policy fails.
- `fn stale_baseline_entry_is_reported()` — a baseline entry with no matching
  finding is flagged for pruning.
- `fn fingerprint_is_stable_across_runs()` — same input → same fingerprint
  (deterministic, Windows-safe).
- Macro compile-fail (`crates/hydracache-db/tests/policy/fail_lint_allow_no_reason.rs`
  + `.stderr`) — `lint_allow` without a reason is a compile error.
- Invocation: `cargo test -p hydracache-sql-lint --locked suppression_baseline`.

Pros: makes the strict lint adoptable on real codebases; suppressions stay
visible and reasoned. Risk: baseline rot / blanket suppression — mitigated by the
stale-entry report and the mandatory reason, which keep both auditable in review.

---

## 2. Generated DB Hooks and Semi-Transparent Invalidation

Status: implemented in `0.38.0`. Depends on: `0.37` outbox table.

### (a) Problem / motivation

`0.37` provides the transactional outbox and documented trigger/CDC patterns,
but external writers still must remember to write outbox rows or install
triggers by hand. `0.38` generates database hooks from HydraCache metadata so
external writes become visible through the same outbox contract.

### (b) Design / contract

`HookPlan` renders **reviewable migration SQL** (it never mutates schema
automatically). Generated triggers write into the `0.37`
`hydracache_invalidation_outbox` table using the same intent serialization and
idempotency key `(txid/commit_lsn, sha256(invalidation_target))`. Optional
Postgres `LISTEN/NOTIFY` is intent-only (at-most-once); the durable path is
always the outbox — this avoids "silently lost invalidation". CDC is a separate
connector crate (`hydracache-cdc-postgres`) per the anti-scope.

### (c) Rust sketch

```rust
// crates/hydracache-db/src/hooks.rs
pub struct HookPlan { dialect: SqlDialect, table: String, ops: Vec<HookOp> }

pub enum HookOp {
    OnInsert(InvalidationTarget),
    OnUpdate(InvalidationTarget),
    OnDelete(InvalidationTarget),
}

impl HookPlan {
    pub fn postgres(table: &str) -> Self { /* ... */ }
    pub fn sqlite(table: &str) -> Self { /* ... */ }
    pub fn mysql(table: &str) -> Self { /* ... */ }
    pub fn on_insert(self, t: InvalidationTarget) -> Self { /* ... */ }
    pub fn on_update(self, t: InvalidationTarget) -> Self { /* ... */ }
    pub fn on_delete(self, t: InvalidationTarget) -> Self { /* ... */ }
    /// Pure, deterministic SQL render for migration review.
    pub fn render_sql(&self) -> Result<String, HookError> { /* ... */ }
    pub fn schema_version(&self) -> HookSchemaVersion { /* ... */ }
}
```

```rust
// crates/hydracache-cdc-postgres/src/lib.rs  (NEW connector crate, intent only)
// Wraps logical replication; emits invalidation intent onto the bus.
// NO value serving, NO dataflow, NO wire proxy.
pub struct PostgresCdcConnector { /* slot, last_offset (LSN) */ }
impl PostgresCdcConnector {
    pub async fn next_intents(&mut self) -> Result<(Vec<Intent>, ReplicationOffset), CdcError> { /* ... */ }
}
```

### (d) Implementation steps

1. Add `hooks.rs` to `hydracache-db` with `HookPlan` + deterministic renderers.
2. Add `HookSchemaVersion` written to the `0.37` `hydracache_hook_schema` table.
3. Add SQLite runtime install/verify path (single-process safe).
4. Add `LISTEN/NOTIFY` wrapper around SQLx `PgListener` (intent-only).
5. Create `hydracache-cdc-postgres` connector crate (offset-tracked).
6. Add `hydracache-hooks` CLI wrapper for migration generation.
7. Wire hook counters (Section 7).

### (e) TESTING

Snapshot (`crates/hydracache-db/tests/hook_sql_snapshot.rs`, unit, `insta`):

- `fn pg_insert_update_delete_snapshot()` — stable SQL across runs.
- `fn sqlite_snapshot()`, `fn mysql_snapshot()`.
- `fn render_rejects_missing_tag_columns()` — `Err(HookError)`.
- Run: `cargo test -p hydracache-db --locked hook_sql_snapshot`.

SQLite runtime integration (`crates/hydracache-db/tests/sqlite_hooks.rs`, integration):

- `fn trigger_writes_outbox_on_insert()` / `_update` / `_delete`.
- `fn worker_publishes_trigger_row_and_invalidates()`.
- `fn duplicate_trigger_rows_are_idempotent()` — asserts dedupe via idempotency key.
- `fn namespace_isolation_with_generated_triggers()`.
- Run: `cargo test -p hydracache-db --locked sqlite_hooks`.

Docker (`crates/hydracache-db/tests/postgres_hooks.rs` / `mysql_hooks.rs`, `#[ignore]`):

- `#[ignore] fn pg_trigger_outbox_worker_end_to_end()`.
- `#[ignore] fn pg_listen_notify_wakeup()`.
- `#[ignore] fn mysql_trigger_outbox_worker_end_to_end()`.
- Run: `cargo test -p hydracache-db --test postgres_hooks --locked -- --ignored`.

CDC connector (`crates/hydracache-cdc-postgres/tests/synthetic.rs`):

- `fn synthetic_event_to_intent()` — fake WAL event → expected `Intent` + offset.
- `#[ignore] fn pg_logical_replication_smoke()` — real slot, Docker.

Failure (`crates/hydracache-db/tests/hook_failures.rs`):

- `fn missing_outbox_table_clear_error()`.
- `fn hook_version_mismatch_detected_at_startup()`.

### (f) Pros

- External writers become visible once hooks are installed; same outbox schema;
  generated SQL is versioned and reviewable; per-table opt-in.

### (g) Risks / fallback

- Trigger SQL is DB-specific; triggers add write overhead; recursive/duplicate
  storms mitigated by idempotency key.
- **Fallback**: if PG/MySQL runtime or CDC connector does not land, ship SQLite
  runtime + deterministic renderers for all three dialects, mark PG/MySQL
  runtime as `#[ignore]` Docker tests, and ship `hydracache-cdc-postgres` as a
  documented `NotImplemented` stub + ADR.

---

## 3. Named Consistency Modes and Read-Your-Writes Tokens

Status: implemented in `0.38.0`. Depends on: `0.37` receipts/barriers (local + best-effort).

### (a) Problem / motivation

`0.37` adds receipts/barriers for cross-node read-after-write. `0.38` turns that
into a *named* consistency model users can reason about, instead of ad hoc
waits. The API must never imply serializable global consistency.

### (b) Design / contract

A `ConsistencyToken` carries generation/namespace/origin metadata. Modes name
the tradeoff. Default is `Eventual`. Cluster-strength modes (`Quorum`, `Leader`)
reuse `0.37` barriers where available and mature in `0.40`; if `0.37`'s barrier
is local-only, those modes degrade to `DegradedOk`/`FailClosed` honestly.

### (c) Rust sketch

```rust
// crates/hydracache/src/consistency.rs
#[derive(Debug, Clone)]
pub enum ConsistencyMode {
    Eventual,                                  // default
    LocalReadYourWrites,
    ClusterReadYourWrites { timeout: Duration },
    Quorum { timeout: Duration },              // matures in 0.40
    Leader,                                    // 0.40
    FailClosed,
    DegradedOk,
}

#[derive(Debug, Clone)]
pub struct ConsistencyToken {
    pub generation: u64,
    pub namespace: Namespace,
    pub origin_node: NodeId,
}

pub enum ConsistencyOutcome<T> {
    Fresh(T),
    Degraded { value: T, reason: DegradeReason },
    TimedOut,
    FailedClosed,
}
```

### (d) Implementation steps

1. Add `consistency.rs`; define modes, token, outcome.
2. Add `invalidate_after_write(tag).consistency(mode)` returning a token.
3. Add `get_with_consistency(key, token, loader)`.
4. Bridge to `0.37` `InvalidationWait` for local/cluster modes.
5. `Quorum`/`Leader`: integrate where cluster supports it, else degrade.
6. Wire success/timeout/degraded counters (Section 7).

### (e) TESTING

Unit (`crates/hydracache/src/tests/consistency.rs`):

- `fn default_mode_is_eventual()`.
- `fn token_carries_generation_namespace_origin()`.
- `fn degraded_outcome_shape()`.

Concurrency (`crates/hydracache/tests/consistency_concurrency.rs`, integration):

- `fn local_generation_prevents_stale_overwrite()`.
- `fn strict_mode_does_not_return_pre_invalidation_value()`.
- `fn strict_mode_timeout_returns_explicit_error()`.

Cluster (`crates/hydracache/tests/consistency_cluster.rs`, in-process multi-node):

- `fn two_node_read_your_writes_success()`.
- `fn two_node_timeout_when_peer_silent()`.
- `fn quorum_success_three_nodes()` / `fn quorum_failure_insufficient_acks()`.
- `fn partition_reports_degraded_not_success()`.
- Run: `cargo test -p hydracache --locked consistency`.

Observability (`crates/hydracache/tests/consistency_metrics.rs`):

- `fn wait_success_timeout_degraded_counters_move()`.

### (f) Pros

- Per-path correctness/performance tradeoffs; critical flows fail closed;
  timeouts/degraded reads observable; the API names the model.

### (g) Risks / fallback

- Stronger modes reduce availability; partitions must not look like success.
- **Fallback**: ship `Eventual`, `LocalReadYourWrites`, `FailClosed`,
  `DegradedOk` fully tested; `Quorum`/`Leader` ship as documented
  `NotImplemented` returning `FailClosed` with a clear reason + ADR pointing at
  0.40.

---

## 4. Required Dimension Profiles and Strict Key Review

Status: implemented in `0.38.0`. Owner: 0.38 (profiles + CI deny). Mechanism inherited from 0.37.

### (a) Problem / motivation

`0.37` provides the `required_dimensions` *mechanism* (static check that named
labels exist). `0.38` adds reusable **profiles** and a CI **deny** mode so teams
encode review policy once and CI fails when production policies miss required
business dimensions.

### (b) Design / contract — anti-gaming link rule

The known risk is that users add meaningless labels to pass the check. To
mitigate, a profile requirement is satisfied only when there is a **link**
between a key segment and a loader argument: a required dimension (e.g.
`tenant`) must appear in **both** the key segments and the tag set, not just
exist as a free label. This does not prove semantics but rejects empty labels.

### (c) Rust sketch

```rust
// crates/hydracache-db/src/profiles.rs
pub enum DimensionProfile {
    TenantScoped,
    PermissionScoped,
    TenantPermissionScoped,
    PagedSearch,
    CursorList,
    LocaleRegionScoped,
    FeatureFlagScoped,
    Custom(&'static CustomProfile),
}

pub struct DimensionRequirement {
    pub label: &'static str,
    /// Anti-gaming: must be present in BOTH key segments and tag set.
    pub require_key_tag_link: bool,
}

pub enum ProfileValidation {
    Pass,
    MissingDimensions(Vec<&'static str>),
    UnlinkedDimensions(Vec<&'static str>), // present in key but not linked to tag
}
```

Macro surface:

```rust
let policy = query_cache_policy!(
    name = "search-users",
    profile = tenant_permission_search,
    key_segments = ["tenant", tenant_id, "permission", permission_hash,
                    "q", query, "page", page, "sort", sort],
    required_dimensions = ["tenant", "permission", "q", "page", "sort"],
    ttl_secs = 30,
);
```

### (d) Implementation steps

1. Add built-in profiles + `CustomProfile` registration.
2. Implement the key↔tag link rule in profile validation.
3. Extend `query_cache_policy!` to accept `profile = ...`.
4. Add `Warn`/`Deny` validation modes and an allowlist requiring reason text.
5. Add a CI validation entry point and wire diagnostics (Section 7).

### (e) TESTING

Unit (`crates/hydracache-db/tests/profiles.rs`):

- `fn profile_passes_when_linked()`.
- `fn fails_when_tenant_missing()` / `fn fails_when_permission_missing()` /
  `fn fails_when_page_or_cursor_missing()`.
- `fn unlinked_label_is_rejected()` — label in key but not in tag → `UnlinkedDimensions`.
- `fn custom_profile_reused()`.
- `fn allowlist_requires_reason_text()`.

Macro/compile (`crates/hydracache-db/tests/policy/`, `trybuild`):

- `pass_required_dimensions.rs`.
- `fail_missing_required_dimension.rs` (+`.stderr`) — statically checkable miss.
- `fail_duplicate_required_dimension.rs` (+`.stderr`).
- `fail_unknown_builtin_profile.rs` (+`.stderr`).
- `pass_runtime_validation_when_not_static.rs` — dynamic case validates at runtime.

Runtime (`crates/hydracache-db/tests/profile_runtime.rs`):

- `fn diagnostics_include_profile_and_required_labels()`.
- `fn warn_mode_warns_does_not_fail()` / `fn deny_mode_fails_gate()`.
- Run: `cargo test -p hydracache-db --locked dimension`.

### (f) Pros

- Review policy encoded once and reused; CI catches missing
  tenant/permission/page/sort; new engineers read requirements from code.

### (g) Risks / fallback

- Profiles can be too rigid; labels still do not prove value correctness.
- **Fallback**: ship built-in profiles + key↔tag link rule; if static macro
  checking proves impossible for some shapes, validate at runtime and document
  it (no stub needed — the mechanism is from 0.37).

---

## 5. Transaction Companion API

Status: implemented in `0.38.0`. SQLx only in 0.38; Diesel/SeaORM are compiling stubs.

### (a) Problem / motivation

HydraCache must not magically own transactions, but the safe
transaction-plus-outbox pattern is verbose. `0.38` provides a companion helper
that makes the recommended flow easy without hiding the transaction boundary.

### (b) Design / contract

The helper begins a transaction, passes the transaction and an invalidation
collector to user code, enqueues outbox intent before commit, commits on
success, and rolls back / drops invalidation on error. Query execution and
business logic stay in user code.

**Adapter scope (decided):** SQLx only in `0.38`. Diesel (sync,
`spawn_blocking`) and SeaORM have different transaction models
(internal complexity ~6-7/10). They ship as **compiling stubs** behind feature
flags that emit `compile_error!` / a documented `NotImplemented` deferral so
they never block the release.

### (c) Rust sketch

```rust
// crates/hydracache-sqlx/src/transaction.rs
impl DbCache {
    pub async fn transaction<F, Fut, E>(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        body: F,
    ) -> Result<(), TransactionError<E>>
    where
        F: FnOnce(&mut sqlx::Transaction<'_, sqlx::Sqlite>, &mut InvalidationCollector) -> Fut,
        Fut: Future<Output = Result<(), E>>,
    {
        let mut tx = pool.begin().await?;
        let mut collector = InvalidationCollector::default();
        match body(&mut tx, &mut collector).await {
            Ok(()) => {
                self.outbox.enqueue_sqlx(&mut tx, collector.into_intents()).await?;
                tx.commit().await?;
                Ok(())
            }
            Err(e) => { tx.rollback().await.ok(); Err(TransactionError::Body(e)) }
        }
    }
}
```

```rust
// crates/hydracache-db/src/transaction_stub.rs
#[cfg(feature = "diesel")]
compile_error!(
    "hydracache transaction companion for Diesel is deferred to a later release; \
     see docs/adr/0038-transaction-companion-adapters.md. \
     Use the SQLx companion or manual transaction + outbox enqueue."
);
```

### (d) Implementation steps

1. Add `InvalidationCollector` (key/tag/entity/collection, namespace-preserving).
2. Implement SQLx `transaction` helper (durable + non-durable `InvalidationPlan`).
3. Add Diesel/SeaORM `compile_error!` stubs behind feature flags + ADR.
4. Wire commit/rollback/enqueue-failure counters (Section 7).

### (e) TESTING

SQLx/SQLite (`crates/hydracache-sqlx/tests/transaction_companion.rs`, integration):

- `fn success_commits_and_enqueues()`.
- `fn closure_error_rolls_back_no_outbox_row()`.
- `fn enqueue_failure_rolls_back()`.
- `fn commit_failure_does_not_publish()`.
- `fn collector_supports_key_tag_entity_collection()`.
- `fn custom_namespace_preserved()`.
- `fn direct_invalidation_plan_without_outbox_table()`.
- `fn closure_panic_does_not_publish()`.
- `fn retry_after_failure_succeeds()`.
- Run: `cargo test -p hydracache-sqlx --locked transaction`.

Compile-fail (`crates/hydracache-db/tests/transaction_stub/`, `trybuild`):

- `fail_diesel_companion.rs` (+`.stderr`) — asserts the `compile_error!` message
  when the `diesel` feature is enabled.
- `fail_seaorm_companion.rs` (+`.stderr`).

Custom trait (`crates/hydracache-db/tests/transaction_trait_fake.rs`):

- `fn fake_verifies_begin_commit_rollback_ordering()`.

### (f) Pros

- Less repository boilerplate; standardized rollback; common pattern tested once;
  transaction boundary stays visible.

### (g) Risks / fallback

- Over-abstraction can hide semantics; ORM transaction models differ.
- **Fallback**: SQLx companion is the deliverable; Diesel/SeaORM are
  pre-committed `compile_error!` stubs + ADR `0038-transaction-companion-adapters.md`.

---

## 6. Reconciliation and Drift Detection

Status: implemented in `0.38.0`. Depends on: `0.37` outbox backlog + hook versions.

### (a) Problem / motivation

Even with strict policies and hooks, systems drift: a writer bypasses hooks, a
hook is disabled during migration, the outbox worker is down, or entries live
too long. `0.38` adds reconciliation tooling that detects likely drift.

### (b) Design / contract — two mandatory signals

Start with exactly **two mandatory signals**:

1. **Outbox lag** (backlog/age of unpublished rows).
2. **Hook/schema drift** (installed `HookSchemaVersion` vs expected).

Everything else (CDC offset staleness, cache generations, table update
timestamps) is an **extension**, not a release requirement.

### (c) Rust sketch

```rust
// crates/hydracache-db/src/reconcile.rs
pub struct ReconciliationReport {
    pub outbox_lag: OutboxLag,            // mandatory
    pub hook_drift: HookDrift,            // mandatory
    pub cdc_offset: Option<CdcOffsetLag>, // extension
    pub generations: Option<GenerationDrift>, // extension
}

pub enum DriftStatus { Clean, Drift(Vec<DriftReason>) }

impl ReconciliationReport {
    pub fn status(&self) -> DriftStatus { /* outbox_lag + hook_drift first */ }
}
```

### (d) Implementation steps

1. Implement outbox-lag query against the `0.37` outbox table.
2. Implement hook-version comparison against `hydracache_hook_schema`.
3. Add `ReconciliationReport::status()` (mandatory signals only initially).
4. Add optional CDC/generation extensions behind features.
5. Add actuator endpoint + sandbox drift route (Section 7).

### (e) TESTING

Unit/integration (`crates/hydracache-db/tests/reconcile.rs`):

- `fn clean_state_reports_clean()`.
- `fn missing_hook_version_reports_drift()` (mandatory).
- `fn outbox_backlog_reports_lag()` (mandatory).
- `fn manual_invalidation_clears_drift_where_applicable()`.
- `fn stale_cdc_offset_reports_lag()` (extension, feature-gated).
- `fn actuator_json_shape()`.
- Run: `cargo test -p hydracache-db --locked reconcile`.

Sandbox (`crates/hydracache-sandbox/tests/drift_scenario.rs`):

- `fn drift_then_repair_scenario()`.

### (f) Pros

- Detects the two highest-value drift conditions cheaply; guides repair
  (invalidate tag / replay outbox / rebuild hook).

### (g) Risks / fallback

- Usefulness depends on available signals.
- **Fallback**: if a signal source is stubbed in `0.37`, reconcile reports the
  available signal and documents the gap; CDC/generation stay extensions.

---

## 7. Observability, Actuator, Sandbox

Status: implemented in `0.38.0`.

### Counters / snapshots

dependency-lint warnings/errors/inconclusive; generated hook versions;
hook-generated invalidation rows; outbox lag; CDC lag; consistency
success/timeout/degraded; required-dimension profile violations; transaction
companion commit/rollback/enqueue failures; reconciliation drift.

### Sandbox examples (`hydracache-sandbox`)

dependency lint clean/missing/inconclusive; generated trigger SQL preview;
SQLite trigger/outbox end-to-end; profile pass/fail; consistency
success/timeout/degraded; transaction companion success/rollback; reconciliation
drift + repair.

### TESTING

- `crates/hydracache-actuator-axum/tests/correctness_json.rs`:
  `fn actuator_exposes_correctness_counters()` (serialization + shape).
- `crates/hydracache-sandbox/tests/correctness_routes.rs`:
  `fn each_sandbox_route_returns_expected_report()`.
- Run: `cargo test -p hydracache-sandbox --locked correctness`.

---

## Release Gates (boolean, checkable)

### Focused gates (PowerShell)

```powershell
cargo test -p hydracache-sql-lint --locked
cargo test -p hydracache-db --locked runtime_dep_isolation
cargo test -p hydracache-db --locked dependency
cargo test -p hydracache-db --locked dimension
cargo test -p hydracache-db --locked hook_sql_snapshot
cargo test -p hydracache-db --locked sqlite_hooks
cargo test -p hydracache-db --locked reconcile
cargo test -p hydracache-sqlx --locked transaction
cargo test -p hydracache --locked consistency
cargo test -p hydracache-sandbox --locked correctness
cargo deny check bans
```

### Full gate (PowerShell)

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
cargo deny check
.\scripts\verify-release-readiness.ps1 -Version 0.38.0 -RunGate
.\scripts\verify-feature-matrix.ps1
.\scripts\package-publishable.ps1 -Set bootstrap
.\scripts\package-publishable.ps1 -Set runtime
.\scripts\package-publishable.ps1 -Set adapters
```

### Optional Docker gate (PowerShell)

```powershell
cargo test -p hydracache-sql-lint --test postgres_catalog --locked -- --ignored
cargo test -p hydracache-db --test postgres_hooks --locked -- --ignored
cargo test -p hydracache-db --test mysql_hooks --locked -- --ignored
cargo test -p hydracache-cdc-postgres --locked -- --ignored
```

---

## Implementation Order

1. Land this plan; create `hydracache-sql-lint` crate + `deny.toml` ban.
2. Dependency lint model, parser fixtures, report types, runtime-isolation test.
3. SQLx/Diesel/SeaORM lint-evidence adapters where practical.
4. Postgres catalog-assisted linting (feature-gated, `#[ignore]`).
5. `HookPlan` SQL renderers + snapshot tests.
6. SQLite trigger/outbox runtime integration.
7. Optional PG/MySQL hook runtime + `hydracache-cdc-postgres` connector.
8. Named consistency modes + read-your-writes tokens.
9. Required-dimension profiles + key↔tag link rule + CI deny.
10. SQLx transaction companion + Diesel/SeaORM `compile_error!` stubs + ADR.
11. Reconciliation (two mandatory signals first).
12. Observability/actuator/sandbox.
13. Release notes, feature matrix, positioning docs.
14. Bump versions, verify, tag, package, publish, clean artifacts.

---

## Final Release Decision (boolean conditions, no score)

`0.38.0` ships only if **all** of the following are TRUE:

- [ ] `cargo deny check bans` and the `cargo tree` runtime-isolation test prove
      `sqlparser` is absent from the non-dev dependency graph of every runtime
      crate.
- [ ] Dependency linting catches obvious missing dependencies and reports
      `Inconclusive` (never `Clean`) for dynamic/unsupported SQL.
- [ ] Generated hooks write outbox invalidation intent for at least one
      deterministic local backend (SQLite, runtime-tested); PG/MySQL runtime are
      either tested or shipped as documented `#[ignore]` Docker tests.
- [ ] Named consistency modes have passing success, timeout, and degraded tests;
      `Quorum`/`Leader` are either implemented or shipped as documented
      `NotImplemented`/`FailClosed` stubs + ADR.
- [ ] Required-dimension profiles fail CI/release gates for missing or unlinked
      key labels (key↔tag link rule enforced).
- [ ] The SQLx transaction companion reduces boilerplate without hiding
      transaction ownership; Diesel/SeaORM are compiling `compile_error!` stubs
      + ADR (compile-fail tests pass).
- [ ] Reconciliation detects **both** mandatory signals: outbox lag AND
      hook/schema drift.
- [ ] Every new code path is covered by unit, property, compile-fail,
      integration, docs, or sandbox tests appropriate to its risk.
- [ ] Docs state clearly that HydraCache is not a transparent DB proxy, not a
      perfect SQL dependency oracle, not a CDC platform, and not a distributed
      transaction coordinator (ReadySet anti-scope reproduced in positioning).
