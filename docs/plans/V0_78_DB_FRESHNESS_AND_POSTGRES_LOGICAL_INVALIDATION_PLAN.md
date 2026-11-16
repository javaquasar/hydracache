# HydraCache 0.78.0 Database Freshness and PostgreSQL Logical-Message Invalidation Plan

> **At a glance**
> - **What:** repair the hook-generated outbox identity, add transactional PostgreSQL logical-message
>   invalidation with a real `hydracache-cdc-postgres` consumer, typed commit positions, a
>   session-scoped bypass for a caller's own pending writes, per-call load observation and
>   per-query-family metrics, an opt-in wasted-fill cooldown, volatility/session/RLS SQL lint findings
>   and concurrent freshness oracles on PostgreSQL 16 and 18.
> - **Why:** the database adapter is the core of HydraCache's assisted-correctness positioning, but
>   its CDC connector is a stub, read-after-write either waits by polling or does nothing, the lint
>   cannot see results that depend on volatile functions or caller identity, and operators cannot
>   tell why a request was not served from cache.
> - **After (depends on):** 0.77.0 (release sequence). The work touches `hydracache-db`,
>   `hydracache-cdc-postgres`, `hydracache-sqlx`, `hydracache-sql-lint` and one additive
>   `hydracache` API, so it does not depend on 0.75–0.77 functionality.
> - **Unblocks:** a workload-fit analyzer over `hydracache-cache-sim`, a Management Center
>   database view, and later non-PostgreSQL change-stream connectors.
> - **Status:** planned
>
> Roadmap & sequencing: [`docs/plans/INDEX.md`](INDEX.md) · rules: [`docs/RULES.md`](../RULES.md)

Read the [repository entry point](../../CLAUDE.md), [rules](../RULES.md),
[compatibility register](../COMPAT.md), the [0.37](V0_37_DATABASE_PRODUCTION_HARDENING_PLAN.md) and
[0.38](V0_38_DATABASE_CORRECTNESS_AUTOMATION_PLAN.md) database plans and the
[0.69 migration conformance plan](V0_69_MIGRATION_CONFORMANCE_PLAN.md). The release inherits R-1
through R-11. R-9 (assisted, not magic) and R-10 (opt-in, no regression) bound every work item.

## Release theme

HydraCache keeps the application's database library as the query authority. The application
declares keys and tags, and HydraCache guarantees what it does with them: single-flight, generation
safety for in-flight loads (ADR-0014), durable invalidation intent and explicit consistency modes.
This release strengthens that contract where the current code is weakest:

1. Generated database hooks can lose an invalidation (W1).
2. Invalidation intent can only travel through an outbox table drained by a polling worker (W3,
   W4).
3. A caller that just wrote can either block on a polling wait or read stale data (W6).
4. Neither the caller nor the operator can see the outcome of a cached read (W7).
5. Results that depend on volatile functions, session identity or row-level security can be cached
   under a key that does not capture that dependence (W9).

## Non-goals

- No transparent proxy, wire-protocol interception, SQL rewriting, query decorrelation, predicate
  subsumption or materialized views (R-9). The application still writes SQL through its own
  library.
- No decoding of row changes into cache values. The consumer reads only invalidation intent that
  the application or a generated hook emitted on purpose. Row-image decoding stays out of scope.
- No distributed transactions and no cross-region ordering claim (R-2). Ordering applies within one
  PostgreSQL primary's commit order.
- No MySQL binlog or SQLite change stream in this release. Those engines keep the outbox path.
- No change to default behavior. Every new mechanism is opt-in, except the W1 defect fix, which
  restores the documented contract (R-10).

## Inherited boundary

| Shipped artifact | Location | What 0.78 must keep |
| --- | --- | --- |
| Normalized intents (`Key`, `Tag`, `Entity`, `Collection`, `Flush`) and their SHA-256 target hash | [`outbox.rs:23`](../../crates/hydracache-db/src/outbox.rs) | Same variants, same hash input. A new transport carries the same intents. |
| `InvalidationOutbox` trait, `InMemoryInvalidationOutbox`, `InvalidationOutboxWorker`, `InvalidationApplier` for `HydraCache` | [`outbox.rs:634`, `:939`, `:980`](../../crates/hydracache-db/src/outbox.rs) | Remains the default and the fallback for every engine. |
| Outbox schema version `1` and hook schema version `1` | [`COMPAT.md`](../COMPAT.md) lines 12–13, [`sqlx_outbox.rs:15`](../../crates/hydracache-db/src/sqlx_outbox.rs) | Forward-only, idempotent migrations. Unknown future versions fail loud (R-4). |
| `InvalidationWait` read-after-write modes `NoWait`, `Local`, `BestEffort` | [`outbox.rs:397`, `:504`](../../crates/hydracache-db/src/outbox.rs) | Unchanged. W6 adds a separate, non-blocking mode. |
| Tag-generation snapshot before a load and `put_bytes_if_fresh` stale-load discard | [`cache.rs:1831`, `:1862`](../../crates/hydracache/src/cache.rs), [ADR-0014](../adr/0014-tag-generation-invalidation-safety.md) | Every new path either goes through `shared_load` or does not fill. |
| PostgreSQL 16/18 cached-vs-direct differential lanes | [`ci.yml` job `migration-conformance-postgres-069`](../../.github/workflows/ci.yml), [`cached_vs_direct_postgres.rs`](../../crates/hydracache-db/tests/cached_vs_direct_postgres.rs) | Stay green and ship-mandatory. W10 adds lanes and does not replace them. |
| Reconciliation report with reserved `CdcOffsetLag` and `GenerationDrift` extensions | [`reconcile.rs:138`, `:146`](../../crates/hydracache-db/src/reconcile.rs) | W4 fills `CdcOffsetLag`. Existing drift reasons keep their meaning. |

## External design inputs and provenance

This release studies four external projects as prior art. HydraCache implements every mechanism
independently from its own contracts, public PostgreSQL documentation and published literature.
Reading these projects informs the problem statement and test ideas only.

| Project | Revision studied | License | Use in this release |
| --- | --- | --- | --- |
| pgcache | `7a5fc23` | Elastic License 2.0 | Design reference only |
| ReadySet | `3e366694a` | Business Source License 1.1 | Design reference only |
| Hitbox | `264db36` | MIT | Design reference only |
| ZooCache | `7169da6` | MIT | Design reference only |

Rules:

- No source code, SQL, comments, documentation text, diagrams, test fixtures, generated data or
  identifiers from these projects enter the HydraCache repository, build, packages or SBOM.
- Each mechanism cites a neutral primary source (PostgreSQL documentation, a paper or a standard)
  first. External projects are cited as observed prior art.
- Default parameters (windows, back-off bounds, budgets, poll intervals) come from HydraCache
  experiments recorded in this release's evidence. External values may only bound the search range.
- HydraCache terminology, types and wire/config names are defined in this plan and are not derived
  from external identifiers.
- Contributors record in the PR description which external sources they consulted. Reviewers
  confirm that the change contains no copied material.

| Problem observed in prior art | HydraCache design (independent) | Primary source |
| --- | --- | --- |
| Invalidation intents written to a side table need polling and a separate commit path. | W3/W4: the application emits a transactional logical-decoding message under a HydraCache prefix inside its own transaction. `hydracache-cdc-postgres` reads committed messages from a logical slot and applies them in commit order. The outbox remains the fallback. | PostgreSQL docs: `pg_logical_emit_message`, `pgoutput` `messages` option, "Logical Replication Message Formats", replication slot functions, `max_slot_wal_keep_size` |
| DDL can change what a cached result means without any row change. | W5: an optional event trigger emits a relation-changed message; the consumer maps the relation to configured tags or flushes the namespace. | PostgreSQL docs: event triggers, `pg_event_trigger_ddl_commands()`, `pg_event_trigger_dropped_objects()` |
| Blocking read-after-write waits add latency to unrelated reads and need a timeout path. | W6: a session-scoped `SessionFence` of pending own writes. A read whose targets intersect the fence bypasses the cache and does not fill it. Other reads are unaffected. | Terry et al., "Session Guarantees for Weakly Consistent Replicated Data", PDIS 1994 |
| Entries invalidated before they are ever reused waste origin and cache work. | W8: per-key tracking of fills removed before first reuse, with a bounded exponential pass-through cooldown. Parameters come from `E-CHURN`. | Standard exponential back-off; HydraCache experiment `E-CHURN` |
| Results that depend on volatile functions, session identity or row-level security can be reused for the wrong caller. | W9: `hydracache-sql-lint` safety findings backed by a catalog snapshot that HydraCache generates from the target server's own catalog. | PostgreSQL docs: function volatility categories, `pg_proc.provolatile`, `pg_class.relrowsecurity`, session information functions, row security policies |
| Operators cannot tell why a request was not served from cache. | W7: per-call `LoadObservation` in `hydracache`, per-query-family counters and a closed enum of bypass reasons in `hydracache-db`. | HydraCache [observability contract](../OBSERVABILITY_CONTRACT.md), R-6 |
| Equality checks at quiescent points miss torn or regressing reads under concurrency. | W10: concurrent-load oracles over versioned groups (atomic group read, no regression, paired-group equality, own-write visibility, convergence). | Adya, "Weak Consistency: A Generalized Theory and Optimistic Implementations for Distributed Transactions", MIT PhD thesis, 1999 |

Release gate: the provenance audit in W11 fails if the build, dependency graph, SBOM or packaged
files reference any of the four projects. A mutation canary adds a dependency on one of them and
must turn the gate red.

## Current-state audit

These findings come from reading the 0.75 worktree at commit `6172a815b`. W0 re-verifies each one
against the published 0.77 source before any change.

| ID | Finding | Evidence | Consequence |
| --- | --- | --- | --- |
| D-1 | Generated hooks build `commit_position` as `'hook:<table>:<op>:' \|\| <row value>`. This value contains no transaction identity. Outbox rows are unique on `(namespace, commit_position, target_hash)`, and published rows are never deleted. A second write to the same row, operation and target after the first row was published hits `on conflict do nothing` (PostgreSQL), `insert or ignore` (SQLite) or `insert ignore` (MySQL). No pending row is created. A value cached between the first publish and the second commit stays stale until TTL. The same loss happens when the second write commits while the first row is claimed and its invalidation is applied before that commit is visible. | [`hooks.rs:463-503`](../../crates/hydracache-db/src/hooks.rs), [`sqlx_outbox.rs:927-933`, `:962`, `:1000`](../../crates/hydracache-db/src/sqlx_outbox.rs), `mark_published` at [`sqlx_outbox.rs:353-373`](../../crates/hydracache-db/src/sqlx_outbox.rs). Existing tests ([`sqlite_hooks.rs`](../../crates/hydracache-db/tests/sqlite_hooks.rs)) fire each trigger once. | Silent lost invalidation (R-3 violation). This is suspected from source reading; W1 starts by turning it into a red test. |
| D-2 | Published outbox rows have no retention. | No delete/prune statement in `hydracache-db/src` | Unbounded table growth. Also the precondition for D-1. |
| D-3 | `CommitPosition` is a `String` with derived `Ord`. PostgreSQL LSN text such as `0/A` sorts after `0/10` as a string. | [`outbox.rs:221-222`](../../crates/hydracache-db/src/outbox.rs) | Any watermark comparison over `CommitPosition` would be wrong. W2 adds a typed position and forbids string ordering. |
| D-4 | `PostgresCdcConnector::next_intents` always returns `CdcError::NotImplemented`. | [`hydracache-cdc-postgres/src/lib.rs:81-85`](../../crates/hydracache-cdc-postgres/src/lib.rs) | 0.38's connector promise is unmet. W4 implements it. |
| D-5 | `SqlxTransactionCompanion` supports only SQLite (`sqlite_durable`, `sqlite_local`). PostgreSQL users must call `enqueue_in_postgres_tx` by hand. | [`hydracache-sqlx/src/transaction.rs:92`, `:170`](../../crates/hydracache-sqlx/src/transaction.rs) | W3 adds PostgreSQL companions. |
| D-6 | `InvalidationWait::wait` polls `status_for_commit` every 10 ms until publish, dead letter or timeout. | [`outbox.rs:560-629`](../../crates/hydracache-db/src/outbox.rs) | Read-after-write costs at least one poll interval plus worker latency on every guarded read. W6 compares this with a bypass. |
| D-7 | `get_or_load` and `get_or_load_with_refresh` return only the value. Hit, miss, join and stale outcomes exist only as global `CacheStats` counters and events. | [`cache.rs:1215`, `:1294`](../../crates/hydracache/src/cache.rs), [`hydracache-core/src/stats.rs:25`](../../crates/hydracache-core/src/stats.rs) | `hydracache-db` cannot attribute outcomes to a query family. W7 adds an observed variant. |
| D-8 | `DependencyLint::check` returns only the first status (missing before extra). It knows only relations, not functions or row security. | [`hydracache-sql-lint/src/lib.rs:230-248`](../../crates/hydracache-sql-lint/src/lib.rs) | W9 adds a multi-finding safety pass. |
| D-9 | GitHub Actions service containers cannot pass server arguments, so the 0.69 PostgreSQL services run with the default `wal_level=replica`. | [`ci.yml:1080-1093`](../../.github/workflows/ci.yml) | Logical-slot tests need a container started in a step with `-c wal_level=logical`. |

## Execution order

```text
W0 contract, pins, red reproductions
 ├─ W1 hook identity + retention ───────────────────────────────┐
 ├─ W2 typed positions ─┬─ W3 emitter + companions ─ W4 consumer ┼─ W5 DDL signal (optional)
 │                      └─ W6 session fence ─────────────────────┤
 ├─ W7 load observation + family metrics ─ W8 churn cooldown ────┤
 ├─ W9 safety lint ──────────────────────────────────────────────┤
 └──────────────────────────── W10 concurrent oracles (PG16/PG18) ┴─ W11 compat, docs, evidence
```

W1 has no dependency besides W0 and ships even if every later item is cut. W4 needs W2 and W3. W6
needs W2 and at least one clearance source (outbox status or the W4 watermark). W8 needs W7.

### Comparative decision gates

| Experiment | Decision | Evidence required before selection | Result if evidence is insufficient |
| --- | --- | --- | --- |
| `E-HOOK-ID` | Repair D-1 with (A) a row `revision` plus re-arm on conflict, or (B) a per-firing unique commit identity | Both variants pass the W1 red tests on SQLite, PostgreSQL 16/18 and MySQL; migration from schema `1`; outbox growth per 10⁵ writes; claim/publish race test | Ship (A). It preserves deduplication within a transaction and closes the claimed-row race. |
| `E-CDC-READ` | Consume with SQL peek-and-advance polling, or with the streaming replication protocol | Apply-lag p50/p99, CPU on PostgreSQL and client, slot retention under 100/1,000/5,000 committed messages/s, idle-system watermark progress | Ship SQL polling only. The streaming protocol needs its own dependency review and becomes deferred work. |
| `E-RYW` | Adopt `SessionFence` as the recommended PostgreSQL read-after-write mode | Same workload with `InvalidationWait::local` vs `SessionFence`: latency of unrelated reads, latency of related reads, database reads added by bypass, W10 own-write oracle green for both | Document both. Keep `InvalidationWait` as the recommendation. |
| `E-CHURN` | Defaults for reuse window, consecutive-waste threshold and cooldown bounds | Write-hot, read-hot and mixed synthetic families: loader calls per read, hit ratio, bypass ratio and cooldown oscillation, under paused time and on PostgreSQL | Ship `ChurnGuard` disabled by default with no recommended defaults. |

Freeze each experiment's alternatives, workload, seeds, metrics and decision rule in W0 before
running it. Keep losing results in the evidence directory.

### W0. Freeze the baseline, register contracts and reproduce defects

**Implementation.**

- Pin the published 0.77 tag, the PostgreSQL 16 and 18 image digests already used by
  `migration-conformance-postgres-069`, the MySQL image used by `mysql_hooks.rs`, and the Rust
  toolchain.
- Create `docs/testing/db/0.78/contract.toml`. It holds:
  - the experiment registry (`E-HOOK-ID`, `E-CDC-READ`, `E-RYW`, `E-CHURN`) with alternatives,
    workloads, seeds and decision rules;
  - the logical-message envelope version;
  - the slot and publication naming rules;
  - the list of new gated tests.
- Re-verify D-1 to D-9 against the published 0.77 source and record each as `confirmed`,
  `not_reproduced` or `changed`.
- Write red tests for D-1 and D-3 before any product change:
  - `hook_second_update_after_publish_is_not_lost` (SQLite, PostgreSQL, MySQL variants);
  - `commit_position_string_order_is_not_lsn_order`.
- Add an ADR stub for the logical-message invalidation transport and one for the session fence and
  clearance, using the next free ADR numbers in `docs/adr/` at that time (0.75 owner decisions may
  take the next numbers first).

**Tests.** Add `crates/xtask/tests/db_freshness_contract_078.rs` with:

- `rejects_unpinned_postgres_image`;
- `rejects_experiment_without_frozen_decision_rule`;
- `rejects_missing_red_reproduction_for_confirmed_defect`;
- `rejects_external_project_dependency`.

Run:

```powershell
cargo test -p xtask --test db_freshness_contract_078 --locked
cargo test -p hydracache-db --features sqlx-outbox --test sqlite_hooks --locked
cargo xtask doc-check
```

**Exit.** Every defect has a recorded disposition. Every confirmed defect has a test that is red on
the predecessor.

### W1. Repair hook-generated outbox identity and bound published-row retention

**Problem.** D-1 and D-2. A generated trigger can drop an invalidation, and the outbox grows without
bound.

**Design (variant A, default for `E-HOOK-ID`).**

- Add `revision bigint not null default 0` to `hydracache_invalidation_outbox`. This is outbox
  schema `2`, a forward-only, idempotent migration registered in `COMPAT.md` (R-4).
- Generated hooks and `enqueue_in_*_tx` replace "ignore on conflict" with an upsert that:
  - increments `revision` on every firing;
  - when the row is `published`, moves it back to `pending` with `available_at_ms = now`,
    `attempts = 0`, `published_at_ms = null`, `last_error = null`;
  - leaves `pending` and `dead` rows in their state but still increments `revision`.
- The worker remembers the `revision` it claimed. `mark_published` succeeds only when the stored
  `revision` still equals the claimed one. If a later firing raced the apply, the row stays
  `pending` and is applied again. Invalidation is idempotent, so a repeated apply is safe.
- Variant B (compared in `E-HOOK-ID`) instead puts a per-firing identity in `commit_position`:
  - PostgreSQL: `pg_current_xact_id()`;
  - SQLite: a sequence table;
  - MySQL: a per-firing sequence.

  It removes conflicts, but rows grow with every firing, so variant B needs the same retention.

**API and code changes.**

- [`outbox.rs`](../../crates/hydracache-db/src/outbox.rs):
  - add `pub revision: u64` to `OutboxRow`;
  - add `InvalidationOutbox::mark_published_revisions(&self, rows: &[(String, u64)])` with a
    default implementation that calls `mark_published`;
  - add `InvalidationOutbox::prune_published(&self, namespace, older_than: Duration, limit: usize)
    -> Result<u64>` with a default implementation that returns `Ok(0)` and a diagnostics flag
    `retention_unsupported`.
  - `InvalidationOutboxWorker::run_once` calls `mark_published_revisions`.
  - `InMemoryInvalidationOutbox` implements both methods.
- Adding a public field to `OutboxRow` is a source-level change for custom outbox implementors.
  Register it like the 0.69 change and extend
  [`custom_outbox_contract.rs`](../../crates/hydracache-db/tests/custom_outbox_contract.rs).
- [`sqlx_outbox.rs`](../../crates/hydracache-db/src/sqlx_outbox.rs):
  - change `POSTGRES_INSERT_OUTBOX` and `SQLITE_INSERT_OUTBOX` to the re-arm upsert;
  - change the `mark_published` statements to `... where id = ? and revision = ?`;
  - make the claim queries return `revision`;
  - add `prune_published` as a bounded `delete ... where state = 'published' and published_at_ms
    < ? ... limit` (PostgreSQL uses a `ctid`/`id in (select ... limit)` subquery);
  - add migration statements for schema `1 → 2` and make `check_schema` accept only `2` after
    migration.
- [`hooks.rs`](../../crates/hydracache-db/src/hooks.rs):
  - `render_postgres_insert`, `render_sqlite_insert` and `render_mysql_insert` render the same
    re-arm upsert;
  - bump `HOOK_SCHEMA_VERSION` to `2` so that reconciliation reports `HookVersionMismatch` for
    triggers installed by older binaries until they are reinstalled.
- [`reconcile.rs`](../../crates/hydracache-db/src/reconcile.rs): add
  `DriftReason::OutboxRetentionBacklog { published_rows, max_published_rows }` driven by a new
  `OutboxLagPolicy::max_published_rows` (default `u64::MAX`, so this is opt-in).

**Tests.**

- `crates/hydracache-db/tests/outbox_identity_078.rs`:
  - `hook_second_update_after_publish_is_not_lost` (SQLite);
  - `hook_update_during_claim_reapplies_after_revision_change`;
  - `same_transaction_duplicate_firings_create_one_pending_row`;
  - `mark_published_with_stale_revision_keeps_row_pending`;
  - `prune_published_never_deletes_pending_or_dead`;
  - `schema_v1_migrates_to_v2_idempotently`;
  - `v1_binary_refuses_v2_schema_loudly`.
- The PostgreSQL and MySQL variants of the same tests run behind `#[ignore]` with
  `HYDRACACHE_TEST_POSTGRES_URL` and `HYDRACACHE_TEST_MYSQL_URL`.
- Extend [`outbox_property.rs`](../../crates/hydracache-db/tests/outbox_property.rs) with a
  property: for any interleaving of firings, claims, applies and publishes, every firing that
  commits after a claim is followed by at least one apply.
- Canary: restore "ignore on conflict" in the SQLite hook; `hook_second_update_after_publish_is_not_lost`
  must turn red.

```powershell
cargo test -p hydracache-db --features sqlx-outbox --test outbox_identity_078 --test outbox_property --test custom_outbox_contract --test sqlite_hooks --locked
cargo test -p hydracache-db --features sqlx-outbox --test outbox_identity_078 --test postgres_hooks --test mysql_hooks --locked -- --ignored
```

**Exit.** D-1 is green on all three dialects. The schema migration is registered, and published-row
retention is bounded when configured.

**Risks.** Re-arming increases apply work for hot rows. Measure it in `E-HOOK-ID` as applies per
committed write. A trigger installed by an older binary keeps the lossy SQL until reinstalled.
Reconciliation reports this through the hook version bump, and the release notes call it out.

### W2. Add typed commit positions and a clearance contract

**Problem.** D-3. A watermark needs an ordered position type, and W6 needs a way to ask whether a
specific write's invalidation has been applied.

**Design.**

- New module `crates/hydracache-db/src/position.rs`:
  - `pub struct PgLsn(u64)` with `parse("X/Y")`, `Display` in PostgreSQL's `X/Y` hex form and
    numeric `Ord`;
  - `CommitPosition::from_pg_lsn(PgLsn)` and `CommitPosition::as_pg_lsn() -> Option<PgLsn>` for
    values created by this constructor. The text form gets an `lsn:` prefix so that it never
    collides with txid or `sqlite:` positions.
  - `CommitPosition` keeps its derived `Ord` for map keys. Its rustdoc states that the order is
    not commit order. A clippy `disallowed_methods` entry is not possible for derived ordering, so
    W2 adds a unit test documenting the counterexample and a grep-based xtask check that no
    `hydracache-db` or `hydracache-cdc-postgres` source compares `CommitPosition` with `<` or
    `>`.
- Clearance contract:

```rust
/// Answers whether the invalidation for one committed write has been applied locally.
#[async_trait]
pub trait ClearanceSource: Send + Sync + 'static {
    async fn is_cleared(&self, write: &PendingWrite) -> Result<Clearance>;
}

pub enum Clearance { Cleared, Pending, Failed { reason: ClearanceFailure } }

pub struct PendingWrite {
    namespace: String,
    position: WritePosition,       // Outbox(CommitPosition) | Lsn(PgLsn)
    targets: Vec<CacheInvalidation>,
}
```

- Implementations:
  - `OutboxClearance<O: InvalidationOutbox>` calls `status_for_commit` at most once per
    configured interval per pending write and caches `Cleared`. `dead_lettered > 0` maps to
    `Failed { DeadLettered }`.
  - `LsnClearance` reads the `applied_through` watermark published by the W4 consumer as a
    `PgLsn` (an `AtomicU64`) and returns `Cleared` when `applied_through >= write.position`.

**Tests.** `crates/hydracache-db/tests/positions_078.rs`:

- `pg_lsn_round_trips_and_orders_numerically`;
- `commit_position_string_order_is_not_lsn_order`;
- `outbox_clearance_caches_cleared_and_rate_limits_queries`;
- `outbox_clearance_reports_dead_letter_as_failed`;
- `lsn_clearance_compares_numerically`.

Add a proptest for random `u64` round trips and ordering. Add the xtask check test
`rejects_commit_position_ordering_comparison` to `db_freshness_contract_078.rs`.

```powershell
cargo test -p hydracache-db --test positions_078 --locked
cargo test -p xtask --test db_freshness_contract_078 --locked
```

**Exit.** Every watermark comparison in later items uses `PgLsn`.

### W3. Emit invalidation intent as a transactional logical-decoding message

**Problem.** The outbox needs a table, a worker and polling. PostgreSQL can carry an
application-defined message in WAL atomically with the transaction's data.

**Design.**

- Envelope `hydracache.invalidation.v1`, registered in `COMPAT.md`:
  - the message prefix is the fixed string `hydracache.inv.v1`;
  - the content is a JSON object `{"v":1,"ns":<namespace>,"reason":<text>,"intents":[...]}`;
  - each intent is `{"kind":"key"|"tag"|"entity"|"collection"|"flush","value":<text>,
    "entity":<text>?}`, the same normalization as `InvalidationIntent::kind`/`value`;
  - maximum content size is configured (default from W0; the server-side limit is PostgreSQL's
    message size).
  - Unknown `v`, unknown `kind` and oversize content are rejected by the reader (R-3, R-4).
- Writer: new `crates/hydracache-db/src/logical_message.rs` behind feature `pg-logical`, which
  depends on `sqlx-outbox` for the PostgreSQL pool type.
  - `LogicalMessageEmitter::emit_in_postgres_tx(&self, tx: &mut Transaction<'_, Postgres>,
    namespace: &str, batch: &InvalidationIntentBatch) -> Result<()>` runs
    `select pg_logical_emit_message(true, $1, convert_to($2, 'UTF8'))`.
  - The first argument `true` makes the message transactional: it is decoded only if the
    transaction commits, at the transaction's position in commit order.
  - The function's returned LSN is not a commit position and is not recorded.
- Write receipt:
  - after `tx.commit()` succeeds, the companion runs `select pg_current_wal_insert_lsn()` on a
    pool connection and records it as an upper bound of the commit record's position;
  - the result is `PendingWrite { position: Lsn(bound), targets }`;
  - correctness relies only on `bound >= commit end`. The insert position covers the commit record
    as soon as `COMMIT` returns, including with `synchronous_commit = off`, where the record may
    not yet be written or flushed. `pg_current_wal_lsn()` (the write position) is not used because
    it can trail an asynchronously committed record.
- Companions in [`hydracache-sqlx/src/transaction.rs`](../../crates/hydracache-sqlx/src/transaction.rs):
  - `postgres_durable(pool, reason, body)` uses the existing outbox with
    `SqlxInvalidationOutbox::postgres_commit_position` (closes D-5);
  - `postgres_logical(pool, reason, body)` uses the emitter;
  - both return `SqlxTransactionReport` extended with `pending_write: Option<PendingWrite>`;
  - `SqlxTransactionDiagnostics` gains `logical_messages_emitted` and `emit_failures`.
- Generated hooks: `HookPlan::postgres(table).sink(HookSink::LogicalMessage)` renders PL/pgSQL
  that `perform`s `pg_logical_emit_message(true, 'hydracache.inv.v1', <json built with
  json_build_object>)` instead of inserting an outbox row.
  - `HookSink::Outbox` stays the default.
  - The hook schema row records the sink so that reconciliation can report a sink mismatch.

**Tests.**

- `crates/hydracache-db/tests/logical_message_envelope_078.rs`:
  - `envelope_round_trips_every_intent_kind`;
  - `unknown_version_and_kind_are_rejected`;
  - `oversize_content_is_rejected_before_emit`;
  - `hook_sql_snapshot_for_logical_sink`, which extends the existing `hook_sql_snapshot.rs`
    pattern.
- `crates/hydracache-sqlx/tests/postgres_companion_078.rs`, `#[ignore]`, needs PostgreSQL with
  `wal_level=logical`:
  - `postgres_durable_enqueues_with_txid_position`;
  - `postgres_logical_emits_one_message_per_commit`;
  - `rolled_back_body_emits_no_message`;
  - `pending_write_bound_is_at_or_after_commit` (the test reads the commit's end position from the
    W4 decoder and compares).
- Canary: an emitter variant that passes `false` (non-transactional) must fail
  `rolled_back_body_emits_no_message`.

```powershell
cargo test -p hydracache-db --features pg-logical --test logical_message_envelope_078 --locked
cargo test -p hydracache-sqlx --test postgres_companion_078 --locked -- --ignored
```

**Exit.** A committed transaction carries exactly one envelope per batch, and a rolled-back one
carries none.

### W4. Implement the `hydracache-cdc-postgres` logical-message consumer

**Problem.** D-4.

**Design.**

- Setup, through `LogicalMessageConsumer::provision(&PgPool, &ConsumerConfig)`, which is idempotent:
  - a publication with no tables, named by config (default `hydracache_inv`). `pgoutput` requires a
    publication name. Messages are delivered with `messages 'true'` regardless of the
    publication's tables, so no table is ever published.
  - a logical slot, created with `pg_create_logical_replication_slot(<slot>, 'pgoutput')`. The
    role needs the `REPLICATION` attribute. A missing privilege fails with a typed error, not a
    retry loop.
- Poll loop. One iteration:
  1. `upto := pg_current_wal_flush_lsn()`. Logical decoding reads only flushed WAL, and with
     `upto_lsn` it includes only transactions that commit before that position. Sampling the flush
     position, not the write or insert position, means "nothing returned before `upto`" really
     covers every commit before `upto`.
  2. `select lsn, xid, data from pg_logical_slot_peek_binary_changes(<slot>, upto, <max_changes>,
     'proto_version','1','publication_names',<pub>,'messages','true')`.
  3. Decode the `pgoutput` message types used here, implemented from the PostgreSQL
     "Logical Replication Message Formats" documentation:
     - `B` begin: final LSN, commit time, xid;
     - `M` message: flags, LSN, prefix, content;
     - `C` commit: flags, commit LSN, end LSN, commit time.
     Ignore relation and row messages; there should be none, because the publication has no
     tables. Count any that appear.
  4. Group messages by transaction. Keep only transactional messages with prefix
     `hydracache.inv.v1`. A non-transactional message with this prefix is rejected and counted
     (`non_transactional_rejected`).
  5. For each committed transaction in commit order, decode the envelope and apply each intent
     through `InvalidationApplier` (the existing `HydraCache` implementation).
  6. Advance: `pg_replication_slot_advance(<slot>, <end LSN of last applied commit>)`.
  7. Publish the watermark:
     - if the peek returned fewer than `max_changes` rows, everything committed at or before
       `upto` has been seen, and `applied_through := max(applied_through, upto)`;
     - otherwise `applied_through := end LSN of the last applied commit`.
     This makes the watermark advance on an idle server.
- Delivery semantics:
  - at least once: a crash between apply and advance re-applies, which is safe because invalidation
    is idempotent;
  - never at most once: the slot advances only after apply succeeds;
  - an apply error stops the loop for that transaction, counts it and retries with back-off, so
    the watermark cannot pass a failed transaction (R-3).
- Slot health: each iteration reads `pg_replication_slots` for `wal_status`, `restart_lsn` and
  `confirmed_flush_lsn`.
  - `wal_status = 'lost'` means intent may be missing. The consumer flushes the configured
    namespaces, reports `DriftReason::CdcSlotLost`, sets readiness to unavailable until
    re-provisioned and never resumes silently.
  - Lag `pg_wal_lsn_diff(pg_current_wal_lsn(), confirmed_flush_lsn)` fills the existing
    `CdcOffsetLag`.
- Public API in [`hydracache-cdc-postgres/src/lib.rs`](../../crates/hydracache-cdc-postgres/src/lib.rs):
  - `LogicalMessageConsumer::run_once() -> Result<ConsumerReport>`;
  - `run(shutdown)`;
  - `applied_through() -> PgLsn`;
  - `clearance() -> LsnClearance`;
  - `diagnostics()`.
  - `PostgresCdcConnector::deferred` and `CdcIntentMapping` remain for source compatibility.
    `next_intents` now delegates to a one-shot decode and documents that it does not advance the
    slot.
  - Dependencies added to the crate: `sqlx` (workspace, `postgres` feature), `tokio`,
    `serde_json`. No new third-party replication dependency in this release (`E-CDC-READ`
    default).

**Comparative experiment `E-CDC-READ`.** SQL peek-and-advance at poll intervals frozen in W0,
against a streaming replication client prototype that is outside the product build. Measure:

- apply lag from commit to `applied_through`, p50/p99;
- server and client CPU per committed message;
- slot retention bytes;
- behavior after consumer restart.

Rates are 100, 1,000 and 5,000 committed messages/s on PostgreSQL 16 and 18 with a fixed
background row-write load that emits no messages. The decoder still reads that WAL, and the
experiment records the cost.

**Tests.**

- Unit tests in `crates/hydracache-cdc-postgres/tests/pgoutput_decode_078.rs`:
  - `decodes_begin_message_commit_frames`;
  - `rejects_truncated_frame`;
  - `groups_messages_by_transaction`;
  - `ignores_foreign_prefix`;
  - `rejects_non_transactional_hydracache_prefix`.
  The decoder has a `cargo fuzz` target `fuzz_pgoutput_frames` in the scheduled lane.
- Real PostgreSQL tests in `crates/hydracache-cdc-postgres/tests/logical_consumer_078.rs`,
  `#[ignore]`, with `wal_level=logical`:
  - `committed_message_invalidates_cache`;
  - `rolled_back_message_never_applies`;
  - `apply_order_follows_commit_order_not_emit_order` (two transactions emit in one order and
    commit in the other);
  - `crash_between_apply_and_advance_reapplies`;
  - `idle_server_watermark_reaches_sampled_upto`;
  - `async_commit_is_not_cleared_before_flush` (writer session with `synchronous_commit = off`;
    `LsnClearance` must report `Pending` until the consumer has decoded the commit);
  - `slot_lost_flushes_and_reports_unavailable` (forced with a tiny `max_slot_wal_keep_size` and
    generated WAL);
  - `missing_replication_privilege_fails_typed`;
  - `consumer_restart_resumes_from_confirmed_position`.
- Canaries:
  - advance the slot before apply; `crash_between_apply_and_advance_reapplies` must go red;
  - publish `applied_through := upto` when the peek was truncated; a W10 own-write oracle must go
    red;
  - sample `upto` from `pg_current_wal_insert_lsn()` instead of the flush position;
    `async_commit_is_not_cleared_before_flush` must go red.

```powershell
cargo test -p hydracache-cdc-postgres --test pgoutput_decode_078 --locked
cargo test -p hydracache-cdc-postgres --test logical_consumer_078 --locked -- --ignored
```

**Exit.** Committed intent is applied in commit order at least once. The watermark never passes an
unapplied commit. Slot loss is loud and forces a flush.

### W5. Signal schema changes through an optional event trigger

**Problem.** `ALTER TABLE`, `DROP` or a changed view definition can change what a cached result
means with no row write.

**Design.**

- `HookPlan::postgres_schema_signal()` renders:
  - one PL/pgSQL event-trigger function;
  - two event triggers on `ddl_command_end` and `sql_drop`.
  The function emits a transactional `hydracache.inv.v1` envelope with intent kind
  `relation_changed`. This is a new kind accepted only from this function's prefix variant; its
  `value` is the schema-qualified relation name from `pg_event_trigger_ddl_commands()` or
  `pg_event_trigger_dropped_objects()`.
- `ConsumerConfig::relation_tags: BTreeMap<RelationName, Vec<String>>` maps a relation to tags.
  An unmapped relation flushes the namespace (fail-safe, counted as
  `schema_change_unmapped_flush`).
- Creating event triggers requires superuser. When the role cannot create them, provisioning
  returns `SchemaSignal::Unavailable`, and reconciliation reports `DriftReason::SchemaSignalMissing`
  until the operator installs the generated SQL with sufficient privileges.

**Tests.** `crates/hydracache-cdc-postgres/tests/schema_signal_078.rs`, `#[ignore]`:

- `alter_table_on_mapped_relation_invalidates_mapped_tags`;
- `drop_unmapped_relation_flushes_namespace`;
- `ddl_in_rolled_back_transaction_emits_nothing`;
- `missing_privilege_reports_drift`.

```powershell
cargo test -p hydracache-cdc-postgres --test schema_signal_078 --locked -- --ignored
```

**Exit.** Either schema changes invalidate, or their absence is reported as drift. W5 can be cut
without affecting W4.

### W6. Bypass the cache for a session's own pending writes

**Problem.** D-6. A guarded read today either blocks on `InvalidationWait` or risks reading the
value its own transaction just replaced.

**Design.**

- New `crates/hydracache-db/src/session.rs`:

```rust
pub struct SessionFence {
    pending: Vec<PendingWrite>,                 // bounded by `max_pending`
    clearance: Arc<dyn ClearanceSource>,
    overflowed: bool,                           // set when `max_pending` is exceeded
    counters: Arc<SessionFenceCounters>,
}

impl SessionFence {
    pub fn record(&mut self, write: PendingWrite);
    pub async fn decide(&mut self, read: &ReadTargets) -> Result<FenceDecision>;
}

pub enum FenceDecision { UseCache, Bypass(BypassReason) }
```

- `ReadTargets` is built from `DbQuery`: the physical key from `physical_key()` and every tag from
  `tags_value()`.
- `PendingWrite.targets` are the `CacheInvalidation` values produced by
  `InvalidationIntent::to_cache_invalidation()` (entity and collection intents already map to
  tags).
- Decision order:
  1. Drop pending writes whose clearance returns `Cleared`.
  2. `Flush` in any pending write, or `overflowed`, → `Bypass(PendingOwnWrite)` for every read
     until those writes clear.
  3. If a pending key equals the read's physical key, or a pending tag is in the read's tags →
     `Bypass(PendingOwnWrite)`.
  4. Otherwise → `UseCache`.
  `Clearance::Failed` keeps the write pending, counts `clearance_failed` and never clears it by
  timeout. A write that remains pending past `overdue_after` is counted as `fence_overdue`. The
  read still bypasses (R-3: never serve possibly stale data to the writer).
- Integration in [`query.rs`](../../crates/hydracache-db/src/query.rs):
  - `DbQuery::with_session(self, fence: &mut SessionFence)`.
  - In `fetch_value_with` (line 822), `Bypass` runs `loader()` directly. It does not read the
    cache, join single-flight or fill. `UseCache` takes the existing path unchanged.
  - Without a fence, behavior is byte-for-byte the predecessor (R-10).
- Companions from W3 return `pending_write`, and the caller records it in its fence. A typical
  per-request fence lives in request state or is carried along by the application. HydraCache does
  not attach it to connections.

**Comparative experiment `E-RYW`.** On PostgreSQL 16, a workload where each session writes one
entity, then reads that entity and 20 unrelated entities. Compare:

- `InvalidationWait::local(timeout)` before all reads;
- `SessionFence` with `OutboxClearance`;
- `SessionFence` with `LsnClearance`.

Report, for related and unrelated reads separately:

- read latency p50/p99;
- database reads added;
- the W10 own-write oracle result.

**Tests.**

- `crates/hydracache-db/tests/session_fence_078.rs`, in-memory outbox with paused time:
  - `related_read_bypasses_and_does_not_fill`;
  - `unrelated_read_uses_cache`;
  - `flush_intent_bypasses_all_reads_until_cleared`;
  - `overflow_bypasses_all_reads_and_counts`;
  - `dead_letter_keeps_write_pending_and_counts_failure`;
  - `cleared_write_stops_bypassing`;
  - `no_fence_path_is_unchanged`.
  The last test compares hit/miss/load counters with a predecessor run.
- PostgreSQL, `#[ignore]`: `session_reads_own_write_with_outbox_clearance` and
  `session_reads_own_write_with_lsn_clearance`.
- Canary: clear pending writes on a timer instead of clearance; the W10 own-write oracle must go
  red.

```powershell
cargo test -p hydracache-db --test session_fence_078 --locked
cargo test -p hydracache-db --features pg-logical --test session_fence_078 --locked -- --ignored
```

**Exit.** A session never reads a cached value older than its own committed write. Unrelated reads
keep the cached path.

### W7. Observe each load and attribute outcomes to query families

**Problem.** D-7, plus the operator question "why was this not served from cache".

**Design in `hydracache`.**

- Add `get_or_load_observed` and `get_or_load_with_refresh_observed` next to the existing methods
  in [`cache.rs`](../../crates/hydracache/src/cache.rs). Each returns `(T, LoadObservation)`:

```rust
pub struct LoadObservation {
    pub source: LoadSource,          // Hit | StaleHit | Loaded | Joined | LoadedNotStored(NotStoredReason)
    pub entry_version: Option<u64>,  // version from CacheEntry for Hit/StaleHit/Loaded-and-stored
    pub encoded_bytes: Option<usize>,
    pub loader_elapsed: Option<Duration>,
}
pub enum NotStoredReason { StaleGeneration, Oversize, BreakerOpen }
```

- The existing `get_or_load` and `get_or_load_with_refresh` keep their signatures. Internally they
  call a shared implementation that skips building the observation (R-10).
- `put_bytes_if_fresh` already knows `StaleGeneration` and `Oversize`. Return that reason instead of
  `bool` inside the crate.
- Hot-path guard: rerun [`benches/cache_hot_path.rs`](../../crates/hydracache/benches/cache_hot_path.rs)
  against the predecessor baseline. Add a `get_or_load_hit` case there if it is missing. The
  predecessor's budget files apply unchanged.

**Design in `hydracache-db`.**

- `DbCacheStats`, keyed by query family. The family is `QueryCachePolicy::name()`, falling back
  to `"<namespace>:unnamed"` as in `default_operation_label`.
- The registry is bounded (`max_families`, default 256). Overflow goes to the `"other"` family and
  counts `family_overflow` (R-6).
- Per family:
  - `hits`, `stale_hits`, `misses_loaded`, `joins`, `not_stored{reason}`;
  - `bypass{reason}` with `BypassReason ∈ {PendingOwnWrite, ChurnCooldown, LintDenied,
    PolicyDisabled}`, a closed `#[non_exhaustive]` enum;
  - `fill_bytes_total`;
  - a fixed-bucket histogram for `loader_elapsed`.
- `DbQuery::fetch_value_with` switches to the observed core methods and records into the stats.
- Expose `DbCache::stats_snapshot() -> DbCacheStatsSnapshot`.
- In `hydracache-observability`, metrics `hydracache_db_reads_total{family,outcome}` and
  `hydracache_db_bypass_total{family,reason}`. `family` is allowed as a label only because the
  registry bounds it. Document this exception against R-6 in `OBSERVABILITY_CONTRACT.md`, together
  with the overflow behavior.

**Tests.**

- `crates/hydracache/src/tests/load_observation.rs`:
  - `hit_reports_entry_version`;
  - `join_reports_joined`;
  - `invalidation_during_load_reports_stale_generation`;
  - `oversize_reports_not_stored`;
  - `breaker_open_reports_not_stored`;
  - `stale_while_revalidate_reports_stale_hit`;
  - `unobserved_api_counters_match_observed_api`.
- `crates/hydracache-db/tests/family_stats_078.rs`:
  - `family_counts_follow_policy_name`;
  - `unnamed_policy_uses_namespace_fallback`;
  - `family_registry_overflow_goes_to_other`;
  - `bypass_reasons_are_counted_per_family`.
- Extend the metrics drift test (alert rules may reference only registered metrics).

```powershell
cargo test -p hydracache --lib load_observation --locked
cargo test -p hydracache-db --test family_stats_078 --locked
cargo bench -p hydracache --bench cache_hot_path -- --save-baseline 078-observed
```

**Exit.** Every `DbQuery` read has an attributed outcome. The unobserved core API has no measurable
hit-path regression under the predecessor budget.

### W8. Add an opt-in wasted-fill cooldown

**Problem.** For write-hot keys, every fill is invalidated before it is read again. Those fills
cost a database read plus encode, store and tag-index work, and they do nothing useful.

**Design.**

- `QueryCachePolicy::churn_guard(ChurnGuardPolicy)`. Disabled by default (R-10).
- `ChurnGuardPolicy { reuse_window, waste_threshold, cooldown_min, cooldown_max, tracker_capacity }`.
  Defaults come from `E-CHURN` or remain unset.
- `ChurnTracker`, bounded LRU per `DbCache`, keyed by physical key. Each record holds
  `last_fill_version`, `filled_at`, `ttl`, `hits_since_fill`, `consecutive_waste` and
  `cooldown_until`.
- Driven only by W7 observations:
  - `Loaded` with a stored version starts a new record. Before that, the previous record is
    classified:
    - wasted when `hits_since_fill == 0` and `filled_at.elapsed() < min(ttl, reuse_window)`, which
      means the entry was removed early by invalidation or eviction;
    - otherwise `consecutive_waste` resets to 0.
  - `Hit`/`StaleHit` with the same `entry_version` increments `hits_since_fill`.
  - When `consecutive_waste >= waste_threshold`, set
    `cooldown_until = now + clamp(cooldown_min * 2^(consecutive_waste - waste_threshold),
    cooldown_min, cooldown_max)`.
- While `now < cooldown_until`, `fetch_value_with` runs the loader directly without filling and
  counts `bypass{ChurnCooldown}`.
- The first read after cooldown fills normally. If that fill receives a hit, the counter resets.
- Time uses `tokio::time::Instant`, so tests run under paused time (R-5). Churn is a performance
  policy, not a correctness signal, so R-1's wall-clock rule is not engaged. The guard can only
  make reads go to the database.
- Tracker overflow evicts the least recently used record and counts `tracker_evictions`. An
  evicted key is treated as never tracked, so it is cached normally.

**Comparative experiment `E-CHURN`.** Synthetic families under paused time, then PostgreSQL:

- write-hot: invalidation interval shorter than the read interval;
- read-hot;
- bursty: alternating phases;
- mixed families sharing one `DbCache`.

Grid: `reuse_window`, `waste_threshold` ∈ {2, 3, 4}, `cooldown_min`/`cooldown_max` grid frozen in
W0. Report:

- loader calls per read;
- hit ratio of read-hot families, which must not drop;
- bypass ratio of write-hot families;
- oscillation count.

Select defaults only if read-hot hit ratio stays within the frozen tolerance and write-hot loader
calls drop.

**Tests.** `crates/hydracache-db/tests/churn_guard_078.rs`:

- `fill_removed_before_reuse_counts_as_waste`;
- `fill_with_hit_resets_waste`;
- `expiry_by_ttl_is_not_waste`;
- `cooldown_bypasses_without_fill_and_expires`;
- `cooldown_grows_exponentially_and_is_capped`;
- `tracker_overflow_falls_back_to_normal_caching`;
- `disabled_policy_is_byte_for_byte_predecessor`.

Add a proptest over random read/invalidate/time schedules: during cooldown nothing is stored, and
outside cooldown behavior equals the unguarded run.

```powershell
cargo test -p hydracache-db --test churn_guard_078 --locked
```

**Exit.** The guard reduces loader work on write-hot keys in `E-CHURN` without reducing read-hot hit
ratio, or it ships disabled with the negative result recorded.

### W9. Add volatility, session and row-security lint findings

**Problem.** D-8. A query that calls `now()`, depends on `current_user` or reads a table with
row-level security can be cached under a key without the tenant or permission dimension.

**Design.**

- New module `crates/hydracache-sql-lint/src/safety.rs`.
- `CatalogFacts` (format `hydracache-catalog.v1.json`, registered in `COMPAT.md`) contains:
  - `functions: BTreeMap<String, Volatility>` with `Volatility ∈ {Immutable, Stable, Volatile}`;
  - `session_dependent: BTreeSet<String>`;
  - `rls_relations: BTreeSet<Relation>`;
  - `server_version`;
  - `generated_at`.
- New CLI subcommand `lint catalog --database-url <url> --out <path>` in
  [`src/bin/lint.rs`](../../crates/hydracache-sql-lint/src/bin/lint.rs), behind a `catalog` feature
  that adds `sqlx`. It reads `pg_proc` (`proname`, `provolatile`, overloads collapsed to the most
  volatile) and `pg_class.relrowsecurity` for user relations.
  - `session_dependent` is a fixed list from the PostgreSQL "Session Information Functions"
    section: `current_user`, `session_user`, `current_role`, `user`, `current_schema`,
    `current_schemas`, `current_setting`, `inet_client_addr`, `pg_backend_pid`, and others listed
    there, checked into `crates/hydracache-sql-lint/data/session_functions.txt` with the
    documentation link.
- Without a catalog file, a built-in fallback uses the same session list plus a small set of
  functions that the PostgreSQL documentation states are volatile (`random`, `clock_timestamp`,
  `timeofday`, `nextval`, `gen_random_uuid`). It also covers `now`/`current_timestamp`, which are
  stable but transaction-time dependent and therefore flagged as time-dependent.
- `SafetyLint::check(sql, &CatalogFacts, &PolicyDimensions) -> Vec<SafetyFinding>` walks the
  `sqlparser` AST: function calls in every expression, relations from the existing collector, and
  special SQL value keywords such as `CURRENT_USER`. Findings:
  - `VolatileFunction { name }`: deny unless suppressed;
  - `TimeDependentFunction { name }`: warn; requires a TTL in the policy, otherwise deny;
  - `SessionDependentFunction { name }`: requires a key dimension labeled `tenant`, `permission`
    or a custom label declared in the policy's `DimensionProfile`
    ([`profiles.rs:4`](../../crates/hydracache-db/src/profiles.rs)), otherwise a finding;
  - `RowSecurityRelation { relation }`: same requirement;
  - `UnqualifiedRelationUnderSearchPath { relation }`: warn only.
- Compatibility:
  - existing `LintFinding`, `LintFindingKind` and their fingerprints are unchanged;
  - safety findings use a separate `#[non_exhaustive] SafetyFindingKind` and fingerprints with a
    `safety:v1:` prefix, so existing baselines remain valid;
  - `PolicyLintMetadata` gains `with_safety_suppression(kind, reason)` (additive, empty reason
    rejected).

**Tests.**

- `crates/hydracache-sql-lint/tests/safety_078.rs`:
  - `random_is_volatile_and_denied`;
  - `now_requires_ttl`;
  - `current_user_requires_tenant_or_permission_dimension`;
  - `rls_relation_requires_dimension`;
  - `suppression_requires_reason`;
  - `catalog_overrides_builtin_volatility`;
  - `existing_dependency_fingerprints_unchanged`;
  - `multiple_findings_are_all_reported`.
- Proptest: the safety pass never panics on generated SQL from the existing `parser_prop.rs`
  strategy.
- `#[ignore]` PostgreSQL test `catalog_snapshot_reads_volatility_and_rls` creates a function of
  each volatility and an RLS table, then checks the snapshot.

```powershell
cargo test -p hydracache-sql-lint --test safety_078 --test suppression_baseline --test parser_prop --locked
cargo test -p hydracache-sql-lint --features catalog --test safety_078 --locked -- --ignored
```

**Exit.** Policies whose SQL depends on volatility, session identity or row security either declare
the matching key dimension and TTL or carry a reasoned suppression.

### W10. Prove freshness with concurrent oracles on PostgreSQL 16 and 18

**Problem.** The 0.69 differential compares cached and direct results at checkpoints. Torn,
regressing or own-write-stale reads under concurrency are not checked.

**Design.**

- Harness `crates/hydracache-db/tests/support/freshness.rs`.
- Two schemas:
  - single-table: `fresh_items(id, grp, version, payload)`;
  - join: `fresh_groups(grp, version)` plus `fresh_items(id, grp, payload)`, read through a join,
    so item inserts change join results and keep refilling the cache.
- Writer tasks run transactions that:
  - bump one group's version;
  - bump two "paired" groups in one transaction;
  - insert or delete items without touching versions.
- Readers use `DbCache` with tags `grp:<n>` and `pair:<p>` and record
  `(session, grp, version, source ∈ {cache, loader, bypass})`.
- Invalidation transports, one per run: outbox + worker, W1-fixed hooks, W3/W4 logical messages.
- Oracles:
  1. **Atomic group read.** One cached result never shows two versions of one group.
  2. **No regression after a fresher observation.** Within one reader, a cache-served read never
     shows a version older than an earlier read *served by the loader* of the same group whose
     transport has since applied that commit. The exemption is reads before clearance, which the
     oracle derives from the transport's applied position.
  3. **Paired equality.** A cached result containing both paired groups shows equal versions.
  4. **Own write.** A session using `SessionFence` never reads a version older than its last
     committed write.
  5. **Convergence.** After writers stop and the transport reports clearance, every cached result
     equals a direct read.
- Seeds and schedules are logged and replayable (R-5). Correctness uses versions and positions,
  never wall-clock thresholds.

**Canaries (must go red).**

- Drop every 50th intent.
- Apply intents out of commit order.
- Advance the slot before apply.
- Clear fences on a timer.
- Revert the W1 fix.
- Publish `applied_through` past a truncated peek.

**CI.** New job `db-freshness-postgres-078` with a matrix of PostgreSQL 16 and 18 using the same
pinned digests as 0.69.

- Because service containers cannot pass server arguments (D-9), a step starts the container:
  `docker run -d --name pg -p 5432:5432 ... <image> -c wal_level=logical -c max_replication_slots=4
  -c max_wal_senders=4`, then waits with `pg_isready`.
- Steps:
  - happy lane per transport;
  - canary lane (PostgreSQL 16);
  - bounded soak of 20 minutes (PostgreSQL 16);
  - `ci-admission-status` receipts as in 0.69.
- Register each test in `docs/testing/gated-test-registry.toml` with `owner_release = "0.78.0"`.

```powershell
cargo test -p hydracache-db --features "sqlx-outbox pg-logical" --test freshness_oracles_078 --locked -- --ignored
cargo run -p xtask --locked -- evidence-run --release 0.78 --gate ignored.hydracache-db.freshness-oracles-postgres-16-078
cargo run -p xtask --locked -- evidence-run --release 0.78 --gate ignored.hydracache-db.freshness-oracles-canary-078
```

**Exit.** All oracles are green for every shipped transport on both PostgreSQL series. Every canary
is red. The 0.69 lanes remain green.

### W11. Register compatibility, operations and release evidence

**Implementation.**

- `COMPAT.md`:
  - outbox schema `2` with the `revision` column and the re-arm semantics;
  - hook schema `2` with the sink field;
  - logical-message envelope `hydracache.inv.v1`;
  - `hydracache-catalog.v1.json`;
  - slot and publication naming;
  - source-level change notes for `OutboxRow.revision` and the new `InvalidationOutbox` default
    methods.
- `PRODUCTION_GUIDE.md`, database section:
  - required server settings (`wal_level=logical`, a restart, `max_replication_slots`,
    `max_wal_senders`, `max_slot_wal_keep_size`);
  - role privileges (`REPLICATION`; superuser only for W5);
  - slot retention and catalog-xmin effects on vacuum;
  - monitoring with `CdcOffsetLag` and slot `wal_status`;
  - recovery after slot loss;
  - when to prefer the outbox;
  - how to use `SessionFence` per request.
- Update [`docs/adr`](../adr): the logical-message transport ADR and the session fence and
  clearance ADR move from stub to accepted with the chosen `E-CDC-READ` and `E-RYW` results.
- Provenance audit: extend the xtask SBOM/dependency check with a deny list containing the four
  external projects' crate and package names. The canary adds one as a dev-dependency and must
  fail.
- Release evidence:
  - wire 0.78 gates into `release-evidence`;
  - record each experiment's disposition;
  - publish negative results.

**Tests.**

- Extend `db_freshness_contract_078.rs`:
  - `rejects_unregistered_persisted_format`;
  - `rejects_shipped_transport_without_oracle_receipt`;
  - `rejects_external_project_dependency_canary_green`;
  - `admits_release_with_cut_optional_w5`.

```powershell
cargo test -p xtask --test db_freshness_contract_078 --locked
cargo xtask doc-check
cargo xtask verify
cargo xtask release-evidence --release 0.78 --receipts-dir target/release-evidence/receipts --require-ship
```

**Exit.** Every persisted or transmitted artifact is registered. Operators have a documented
procedure for each failure the consumer can report. The provenance gate is green, and its canary is
red.

## Deferred

- **Streaming replication client for the consumer.** Deferred unless `E-CDC-READ` shows SQL polling
  misses its lag budget. It needs a dependency review.
- **Workload-fit analyzer.** Reads `pg_stat_statements` exports, runs the W9 safety lint and replays
  through `hydracache-cache-sim`/`hydracache-sim-wasm`. Needs W7/W9 first. Proposed for the next DB
  release.
- **Management Center database view** over `DbCacheStatsSnapshot`.
- **Hedged single-flight loaders, hierarchical tag expansion and an external Hitbox backend
  adapter.** These are local-cache ergonomics, not freshness.
- **MySQL binlog and other change-stream connectors.**

## Release gates

```powershell
cargo test -p hydracache-db --features "sqlx-outbox pg-logical" --locked
cargo test -p hydracache-cdc-postgres --locked
cargo test -p hydracache-sqlx --locked
cargo test -p hydracache-sql-lint --locked
cargo test -p hydracache --lib load_observation --locked
cargo test -p hydracache-db --features "sqlx-outbox pg-logical" --locked -- --ignored
cargo test -p hydracache-cdc-postgres --locked -- --ignored
cargo test -p xtask --test db_freshness_contract_078 --locked
cargo xtask doc-check
cargo xtask verify
```

## Final release decision

- [ ] D-1 is fixed on SQLite, PostgreSQL and MySQL. The W1 canary is red, and schema `2` is
      registered with a tested migration.
- [ ] Each shipped transport (outbox, W1 hooks, logical messages) passes all W10 oracles on
      PostgreSQL 16 and 18, and every W10 canary is red.
- [ ] The consumer never advances past an unapplied commit, and slot loss forces a flush with a
      reported reason.
- [ ] `SessionFence` passes the own-write oracle with every shipped clearance source. `E-RYW`
      decides whether it is the recommended mode.
- [ ] `ChurnGuard` ships with `E-CHURN` defaults or ships disabled with the negative result.
- [ ] The safety lint reports every new finding kind, and existing baselines are unchanged.
- [ ] The unobserved `get_or_load` hit path meets the predecessor budget.
- [ ] The provenance gate is green, and its canary is red.
- [ ] Release notes state the supported PostgreSQL versions, required server settings and
      privileges, at-least-once delivery, and what remains the application's responsibility under
      R-9.
