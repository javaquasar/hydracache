# HydraCache 0.37.0 Database Production Hardening Plan

> **At a glance**
> - **What:** transactional outbox, read-after-write barrier, observability, criterion perf budget, byte weigher, required dimensions, compatibility register.
> - **Why:** make DB query-result caching safe to run in production — no stale-after-write, bounded entries, measurable behavior.
> - **After (depends on):** v0 foundations.
> - **Unblocks:** 0.38 (correctness automation builds on the outbox/CDC substrate).
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md)

Status: implemented in `0.37.0`. Release notes are in
`docs/releases/0.37.0.md`.

`0.37.0` closes the remaining database-production gaps left after the `0.35.0`
readiness release and the `0.36.0` rollout release, but it does so with a
deliberately narrowed scope. The review of releases `0.37`–`0.41`
(`V0_37_41_REVIEW_AND_IMPROVEMENTS.md`) concluded that the original `0.37` plan
carried eight mini-projects under one "production-hardened" banner. That is not
honestly shippable in a single release.

This rewrite therefore splits the release. The `0.37.0` **release core** is:

1. Transactional invalidation outbox (durable intent committed with the data).
2. Read-after-write barriers, **local + best-effort/timeout-degraded only**.
3. Observability for the outbox and barriers.
4. Criterion benchmarks that turn "boring read path" and "production-hardened"
   from claims into measured evidence.

Two cheap, isolated, statically-checkable items also stay in `0.37` because they
do not pull in cluster or runtime weight:

- the `required_dimensions` **mechanism** (a static macro/policy check) — its
  named profiles move to `0.38`;
- a byte-based weigher and a pre-insert `max_entry_bytes` reject in the DB
  adapter.

Everything else — the SQL-dependency lint, `prepared_query_policy!` /
repository attribute-macro hardening, and the full four-way testcontainers
matrix — moves to `0.37.x`/`0.38`. See **"What Moved Out of 0.37"** below.

HydraCache is not becoming a transparent ORM cache, a CDC platform, a dataflow
engine, a SQL planner, or a wire proxy. `0.37` ships explicit, test-backed,
crash-tolerant building blocks for database cache correctness.

## Executive Summary

After `0.36.0`, the database layer is a strong controlled-rollout candidate for
explicit, read-heavy database result caching. The biggest remaining engineering
mass is the **transactional outbox + read-after-write barriers**, confirmed by
code audit: a grep for `[Oo]utbox` finds nothing in the workspace, and there is
no `ConsistencyMode` / `InvalidationReceipt` / read-your-writes machinery and no
`benches/` directory anywhere. These are genuine net-new work, and they are the
core of this release.

`0.37.0` raises production confidence by delivering:

- **Transactional invalidation outbox** with an idempotency contract defined
  *before* the table schema, a circuit-breaker drain worker, advance-the-durable
  -frontier-after-apply ordering, and a read-only `SHOW`-style status surface.
- **Local + best-effort read-after-write barriers** (`InvalidationWait`) with
  explicit timeouts and a degraded-mode result. The `quorum`/`all-peers` barrier
  is **explicitly deferred to `0.40`**, where a fixed 2–5 member pilot topology
  exists.
- **LISTEN/NOTIFY intent transport** by wrapping sqlx `PgListener` directly —
  carrying invalidation *intent* only, with the outbox as the durable backstop.
- **Byte-based weigher + pre-insert `max_entry_bytes` reject** for query results.
- **Criterion benchmarks** for hit/miss, single-flight, event publish with/
  without subscriber, and write-with-outbox vs without.
- **Observability** for outbox lag/backlog/failure and barrier waits/timeouts.
- The **`required_dimensions` mechanism** (static check only).
- An **ADR skeleton** (ownership/replication/consistency/transport/durability)
  started now and finalized in `0.41`.

The release decision is expressed as **boolean checkable gates**, not a numeric
self-score.

## Release Theme

Move the database cache write path from "best-effort invalidation after commit"
to "durable invalidation intent committed with the data, with measured cost and
an explicit read-after-write barrier" — without owning user transactions,
intercepting SQL, or replicating values.

## Non-Goals

These are firm boundaries for `0.37` and beyond. The first two are retained from
the original plan; the rest are tightened from the readyset anti-scope analysis
(review section 9).

- Do **not** add transparent SQL interception.
- Do **not** provide an ORM second-level cache.
- Do **not** build a dataflow engine or materialization layer (no MIR-style
  compiler, no partial materialization, no upquery/replay).
- Do **not** build a SQL planner or re-execute user queries.
- Do **not** build a wire proxy / pgbouncer-style data path.
- Do **not** serve values from CDC. HydraCache emits **intent + invalidation
  only** and never sits in the data path.
- Do **not** infer all dependencies perfectly from arbitrary SQL.
- Do **not** make SQL parsing a runtime correctness requirement, and never link
  `sqlparser` into a runtime crate (machine-checked via `deny.toml`).
- Do **not** own user database transactions inside `DbCache`.
- Do **not** promise serializable, globally strong cache consistency.
- Do **not** require Kafka, Debezium, Redis, or any external service for the
  core release path.
- Do **not** make Docker-backed database tests mandatory for every local
  developer command.

## Production Definition For This Release

For `0.37.0`, "production-hardened database caching" means:

- A repository write can persist invalidation intent in the **same transaction**
  as the data change; rollback removes both.
- A background worker publishes only committed intent, is **idempotent** under
  crash/retry, advances its durable frontier only **after** applying, and
  degrades to a documented dead-letter state under repeated failure.
- A multi-node service can request **bounded local / best-effort** read-after
  -write behavior with an explicit timeout and a degraded result.
- Postgres `LISTEN/NOTIFY` can wake the worker faster, carrying intent only; the
  outbox remains the durable source of truth.
- Outbox backlog/lag/failures and barrier waits/timeouts are observable.
- The read path cost and the write-with-outbox cost are **measured** by criterion
  benches checked into the repo.

It still does **not** mean: automatic invalidation for arbitrary SQL; automatic
discovery of every table touched by dynamic SQL; automatic invalidation from DB
writes unless the service opts into the outbox/trigger contract; strong
consistency across unavailable nodes; or `quorum` read-after-write (that lands
in `0.40`).

## Global Test And Commit Rule

Every implementation step must follow the same rule:

- New public API has unit tests and at least one usage/integration test.
- New macro syntax has passing and failing `trybuild` coverage.
- New durable/database behavior has deterministic SQLite or in-memory tests and,
  where the feature claims Postgres support, `#[ignore]` testcontainers tests.
- Serialization/escaping boundaries get a **property test**, not a single case.
- New observability counters have tests proving the counter moves on the
  relevant success and failure paths.
- Each completed step is committed separately after its tests pass.

---

## 1. Transactional Invalidation Outbox

Status: planned. **Release core.**

### Problem / Motivation

`0.36.0` added staged invalidation: repository code invalidates after a
successful commit and skips invalidation after rollback. That fixes commit/
rollback ordering but leaves a crash window:

1. The service commits the database write.
2. The process crashes (or is killed during deploy, or loses network) before
   publishing the cache invalidation.
3. Other nodes keep serving stale values until TTL or a manual invalidation.

External writers (admin scripts, ETL, other services on the same database,
triggers) are also invisible unless they call the same Rust path.

Invalidation **intent** must be persisted in the same transaction as the data
change, then published after commit with idempotency, retry, and observable lag.

### Design / Contract

**The idempotency contract is defined before the table schema.** This is the
ordering the original plan got wrong; the review (sections 3 and 9) makes it a
hard requirement.

Idempotency key = `(commit_position, sha256(invalidation_target))`, where:

- `commit_position` is the database transaction identity at commit time
  (Postgres `txid` / `pg_current_xact_id`, or commit LSN where available; a
  monotonic per-namespace sequence for SQLite/MySQL fallback). This mirrors
  readyset's durable `ReplicationOffset` (every action is tagged with a durable
  offset, so a restart resumes from the persisted offset).
- `sha256(invalidation_target)` is the content hash of the normalized intent
  (kind + escaped key/tag/entity/collection). This mirrors sqlx's offline
  content-hash files `.sqlx/query-{sha256}.json`, which are created with atomic
  `create_new(true)` and treat `AlreadyExists` as `Ok(())`.

Consequences that fall out of the contract:

- Re-draining after a crash is idempotent: the same `(commit_position, hash)`
  collapses to the same row; a second publish of the same intent is a no-op.
- Duplicate intent from the same transaction (e.g. two code paths invalidating
  the same tag) collapses without a separate dedupe mechanism.
- **No separate `dedupe_key` machinery is needed** — the idempotency key *is* the
  dedupe key.

**Drain worker = circuit breaker + advance-after-apply.** The worker design is
taken directly from pgcat's ban/unban backend circuit breaker and readyset's
offset ordering:

- A batch is claimed, applied to the cache/bus, and **then** the durable frontier
  is advanced (`persisted_offset <= stream_position` — never persist an offset
  past what was actually applied).
- On repeated publish failure for a backend/intent, the worker enters a
  ban-style backoff (pgcat bans on `FailedHealthCheck` / `MessageSendFailed` /
  `StatementTimeout`, unbans after `ban_time`). After the configured attempt
  ceiling, the row is **dead-lettered**.
- `unban-all` has an analogue: an operator-triggered reset that re-enables all
  dead-lettered/banned rows. This is a deliberate, explicit operator action.

**Status surface is read-only.** Worker/backlog status is exposed through a
`SHOW`-like read surface (model: pgcat `SHOW POOLS/BANS/STATS`), never a write
control. Operators inspect; they do not steer the worker through this surface.

**Serialization escaping is a property test, not one case.** Keys and tags can
contain delimiters; a naive `kind:value` concatenation can let two distinct
intents collide on the same hash. The collision case must be a `proptest`.

### Adoption Levels

```text
default path:        InvalidationPlan after commit, no DB schema changes.
production durable:  hydracache_invalidation_outbox table + worker.
custom enterprise:   user-provided outbox adapter mapping InvalidationIntent
                     into an existing application outbox/transport.
```

HydraCache provides copyable migrations (SQLite/Postgres/MySQL), a startup
schema check, a minimal SQL writer contract, the trait-based adapter, and
retention helpers. It never runs migrations automatically.

### Schema (derived from the contract above)

```sql
CREATE TABLE hydracache_invalidation_outbox (
    id              TEXT PRIMARY KEY,        -- uuid/text, surrogate
    namespace       TEXT    NOT NULL,
    commit_position TEXT    NOT NULL,        -- txid / commit LSN / seq
    target_hash     TEXT    NOT NULL,        -- sha256(normalized intent)
    intent_kind     TEXT    NOT NULL,        -- key|tag|entity|collection|flush
    cache_key       TEXT    NULL,
    cache_tag       TEXT    NULL,
    entity_name     TEXT    NULL,
    collection_name TEXT    NULL,
    reason          TEXT    NULL,
    payload_json    TEXT    NULL,
    created_at_ms   INTEGER NOT NULL,
    available_at_ms INTEGER NOT NULL,        -- retry backoff
    claimed_at_ms   INTEGER NULL,
    claim_owner     TEXT    NULL,
    published_at_ms INTEGER NULL,            -- lag/backlog measurement
    attempts        INTEGER NOT NULL DEFAULT 0,
    state           TEXT    NOT NULL DEFAULT 'pending', -- pending|published|dead
    last_error      TEXT    NULL,
    UNIQUE (namespace, commit_position, target_hash)   -- idempotency key
);
```

The `UNIQUE (namespace, commit_position, target_hash)` constraint encodes the
idempotency contract directly. One shared table, no foreign keys into business
tables, `payload_json` for extension instead of frequent `ALTER TABLE`, indexes
on `available_at_ms`, `claim_owner`, and `published_at_ms`.

### Rust API / Type Sketch

In `hydracache-db`:

```rust
/// Normalized, transport-neutral invalidation intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationIntent {
    Key { key: String },
    Tag { tag: String },
    Entity { entity: String, key: String },
    Collection { collection: String },
    Flush,
}

impl InvalidationIntent {
    /// Stable content hash used in the idempotency key. The encoding MUST be
    /// collision-free across distinct intents (property-tested).
    pub fn target_hash(&self) -> [u8; 32] { /* sha256 of length-prefixed parts */ }

    /// Map onto the existing cross-process invalidation type in
    /// `hydracache::invalidation_bus::CacheInvalidation`.
    pub fn to_cache_invalidation(&self) -> hydracache::CacheInvalidation { /* ... */ }
}

#[derive(Debug, Clone)]
pub struct InvalidationIntentBatch {
    reason: String,
    intents: Vec<InvalidationIntent>,
}

impl InvalidationIntentBatch {
    pub fn new(reason: impl Into<String>) -> Self { /* ... */ }
    pub fn invalidate_key(self, key: impl Into<String>) -> Self { /* ... */ }
    pub fn invalidate_tag(self, tag: impl Into<String>) -> Self { /* ... */ }
}

/// Identity of a committed write, used to build the idempotency key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPosition(pub String);

/// Durable outbox transport.
#[async_trait::async_trait]
pub trait InvalidationOutbox: Send + Sync {
    /// Enqueue intent inside the caller's transaction (the executor is borrowed,
    /// HydraCache never commits/rolls back).
    async fn enqueue<'e, E>(&self, tx: E, batch: &InvalidationIntentBatch)
        -> hydracache_core::Result<()>
    where E: /* sqlx Executor bound */;

    /// Claim up to `limit` committed rows ordered by available_at_ms then created_at_ms.
    async fn claim(&self, owner: &str, limit: usize, claim_ttl: std::time::Duration)
        -> hydracache_core::Result<Vec<OutboxRow>>;

    /// Mark rows published; advances the durable frontier AFTER apply.
    async fn mark_published(&self, ids: &[String]) -> hydracache_core::Result<()>;

    /// Mark a row failed (backoff) or dead (attempts exhausted).
    async fn mark_failed(&self, id: &str, err: &str, dead: bool)
        -> hydracache_core::Result<()>;

    /// Read-only status snapshot (SHOW-like surface; not a control channel).
    async fn status(&self, namespace: &str) -> hydracache_core::Result<OutboxStatus>;
}

#[derive(Debug, Clone)]
pub struct OutboxStatus {
    pub pending: u64,
    pub oldest_pending_age_ms: u64,
    pub dead_lettered: u64,
    pub last_published_at_ms: Option<u64>,
}

/// Circuit-breaker drain worker.
pub struct InvalidationOutboxWorker<O> {
    outbox: O,
    cache: hydracache::HydraCache,
    batch_size: usize,
    claim_ttl: std::time::Duration,
    backoff: std::time::Duration,
    max_attempts: u32,        // -> dead-letter
}

#[derive(Debug, Clone)]
pub struct OutboxPublishReport {
    pub claimed: usize,
    pub published: usize,
    pub retried: usize,
    pub dead_lettered: usize,
}

impl<O: InvalidationOutbox> InvalidationOutboxWorker<O> {
    /// Claim -> apply via cache.invalidate_* -> mark_published (advance frontier).
    pub async fn run_once(&self) -> hydracache_core::Result<OutboxPublishReport> { /* ... */ }

    /// Operator reset; re-enables dead-lettered rows (the "unban-all" analogue).
    pub async fn reset_dead_letters(&self, namespace: &str) -> hydracache_core::Result<u64> { /* ... */ }
}
```

The worker publishes by calling the existing `HydraCache::invalidate_key` /
`invalidate_tag` (see `crates/hydracache/src/cache.rs:886,930`) or, in cluster
mode, the existing `CacheInvalidationBus::publish` with a
`CacheInvalidationMessage` (`crates/hydracache/src/invalidation_bus.rs:106`).
Intent maps cleanly onto the existing `CacheInvalidation` enum
(`invalidation_bus.rs:21`).

### Implementation Steps

1. Add `InvalidationIntent`, `target_hash`, `InvalidationIntentBatch`,
   `CommitPosition` to `hydracache-db`. Implement length-prefixed hashing so
   distinct intents cannot collide.
2. Add the `InvalidationOutbox` trait and `OutboxRow` / `OutboxStatus` types.
3. Implement `SqlxInvalidationOutbox` for SQLite first (reference), then add the
   Postgres SQL variant and `commit_position` derivation (`pg_current_xact_id`).
4. Add migrations for SQLite/Postgres/MySQL and a startup schema check
   (missing table, missing required column, incompatible version).
5. Implement `InvalidationOutboxWorker::run_once` with claim → apply →
   `mark_published` ordering. Never advance the frontier before apply.
6. Add circuit-breaker backoff + dead-letter on `max_attempts`, plus
   `reset_dead_letters`.
7. Add the read-only `status()` surface and wire it to observability (section 4).
8. Add the custom-adapter path: a fake application outbox implementing the trait.
9. Document the minimal SQL writer contract (`intent_kind` + value) and a trigger
   example. (Full trigger/CDC docs move to `0.37.x` — see "What Moved Out".)

### TESTING

Files and test functions:

- `crates/hydracache-db/src/outbox.rs` (unit, in `#[cfg(test)] mod tests`):
  - `intent_target_hash_is_stable` — same intent → same hash across runs.
  - `intent_to_cache_invalidation_maps_each_kind`.
- `crates/hydracache-db/tests/outbox_idempotency.rs` (integration, SQLite):
  - `commit_persists_outbox_row_with_data` — write + enqueue commit together.
  - `rollback_removes_outbox_row` — rollback drops both data and intent.
  - `crash_window_replays_durable_row` — enqueue+commit, drop worker before
    publish, new worker publishes the row.
  - `double_drain_is_idempotent` — run_once twice; assert the cache invalidation
    is applied once and the unique key prevents a duplicate row.
  - `frontier_advances_only_after_apply` — inject an apply failure; assert the
    row stays `pending` and is **not** marked published.
- `crates/hydracache-db/tests/outbox_property.rs` (property, `proptest`):
  - `escaping_never_collides` — for arbitrary key/tag pairs containing `:`, `/`,
    whitespace, empty strings, assert distinct intents → distinct `target_hash`.
- `crates/hydracache-db/tests/outbox_worker.rs` (integration):
  - `worker_retries_failed_publish_with_backoff`.
  - `worker_dead_letters_after_max_attempts`.
  - `reset_dead_letters_reenables_rows`.
  - `claim_order_is_oldest_first` (no starvation).
  - `namespace_isolation` — a worker for ns A ignores ns B rows.
  - `status_reports_pending_oldest_and_dead` (read-only surface).
  - `custom_adapter_persists_and_replays_intent`.
- `crates/hydracache-db/tests/outbox_postgres.rs` (`#[ignore]`, testcontainers):
  - `pg_commit_position_uses_txid`, `pg_crash_window_replays`. **Mandatory
    minimum** Postgres coverage for the reference SQLx path.

Invocations:

```powershell
cargo test -p hydracache-db --locked outbox
cargo test -p hydracache-db --test outbox_property --locked
cargo test -p hydracache-db --test outbox_postgres --locked -- --ignored
```

### Pros

- Invalidation survives crashes, restarts, and deploys.
- External writers can participate via plain SQL inserts.
- Idempotency and retry become library behavior, not per-service reinvention.
- Backlog/lag is observable database state instead of hidden stale-cache risk.

### Risks / Fallback

- **Write amplification** on hot write paths. Mitigated by the criterion bench
  (section 5) measuring write-with-outbox vs without, and by `dedupe`-free
  collapse via the idempotency key.
- **Diesel/SeaORM transaction typing** may not stay simple. **Fallback:** ship
  SQLx + a documented `NotImplemented` stub for Diesel/SeaORM enqueue plus an
  ADR recording the deferral; that counts as done for this item.

---

## 2. Read-After-Write Barrier (Local + Best-Effort Only)

Status: planned. **Release core.**

### Problem / Motivation

Local invalidation is generation-safe (the core already carries
`ClusterGeneration` on messages — see `invalidation_bus.rs:108`), but cross-node
read-after-write is eventual: node A writes and publishes, node B may still serve
a stale value until it applies the invalidation. Some flows need to wait before
serving a dependent read.

### Design / Contract

Ship **only** two barrier modes in `0.37`:

- `InvalidationWait::local()` — wait until the local cache has applied the
  invalidation (uses the existing generation machinery).
- `InvalidationWait::best_effort().timeout(d)` — wait up to `d` for observed
  peers to acknowledge via the existing bus, then return a **degraded** result
  rather than blocking forever.

`InvalidationWait::quorum()` / `all_peers()` are **explicitly deferred to
`0.40`**. They require real member accounting and partition-timeout handling,
which only mature when the pilot's fixed 2–5 member topology exists. Stating this
dependency is part of the deliverable.

### Rust API / Type Sketch

```rust
/// Receipt returned by an invalidation, used to wait for propagation.
#[derive(Debug, Clone)]
pub struct InvalidationReceipt {
    pub namespace: String,
    pub target: hydracache::CacheInvalidation,
    pub origin_node: String,
    pub local_generation: hydracache::ClusterGeneration,
    pub submitted_at_ms: u64,
}

#[derive(Debug, Clone)]
pub enum InvalidationWait {
    /// Local cache has applied the invalidation.
    Local,
    /// Wait for observed peers up to a timeout, then degrade.
    BestEffort { timeout: std::time::Duration },
    // Quorum / AllPeers DEFERRED to 0.40 (pilot topology dependency).
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarrierOutcome {
    Applied,
    DegradedTimeout { observed: u32, expected: u32 },
}

impl hydracache::HydraCache {
    pub async fn invalidate_tag_with_receipt(&self, tag: &str)
        -> hydracache_core::Result<InvalidationReceipt> { /* ... */ }
    pub async fn invalidate_key_with_receipt(&self, key: &str)
        -> hydracache_core::Result<InvalidationReceipt> { /* ... */ }
    pub async fn wait_for_invalidation(
        &self,
        receipt: &InvalidationReceipt,
        wait: InvalidationWait,
    ) -> hydracache_core::Result<BarrierOutcome> { /* ... */ }
}
```

`wait_for_invalidation` reuses the existing publish preflight
(`may_publish` / `*_if_observed`, `cache.rs:1354`, `events.rs:48`) so that
barriers add zero cost when there are no subscribers.

### Implementation Steps

1. Add `InvalidationReceipt`, `InvalidationWait`, `BarrierOutcome` to
   `hydracache`.
2. Make `invalidate_*_with_receipt` capture `local_generation` from the existing
   generation counter.
3. `wait_for_invalidation(Local)` resolves once the local apply generation moves
   past the receipt generation.
4. `wait_for_invalidation(BestEffort)` counts acks from observed peers on the
   existing bus; on timeout return `DegradedTimeout`.
5. Emit barrier metrics (section 4).

### TESTING

- `crates/hydracache/tests/barrier_local.rs` (integration):
  - `local_receipt_carries_generation`.
  - `wait_local_resolves_after_apply`.
  - `stale_load_cannot_overwrite_newer_generation` (race/property over
    interleavings).
- `crates/hydracache/tests/barrier_best_effort.rs` (integration, in-process
  two-cache bus):
  - `best_effort_resolves_when_peer_applies`.
  - `best_effort_times_out_into_degraded` — assert `DegradedTimeout { .. }`.
  - `default_reads_work_without_barrier` (backward compat).
- `crates/hydracache/tests/barrier_metrics.rs`:
  - `barrier_wait_and_timeout_counters_move`.

```powershell
cargo test -p hydracache --locked barrier
```

### Pros

- Explicit, reviewable read-your-writes for the cases that need it.
- No dependency on an immature cluster; degrades visibly instead of lying.

### Risks / Fallback

- Best-effort ack counting depends on the in-process/framed bus. **Fallback:**
  if peer ack plumbing does not fit, ship `Local` fully and a documented
  `NotImplemented` stub for `BestEffort` peer-ack + an ADR; `Local` + the stub
  counts as done.

---

## 3. LISTEN/NOTIFY Intent Transport

Status: planned. **Release core (Postgres only).**

### Problem / Motivation

Polling the outbox is correct but adds latency. Postgres `LISTEN/NOTIFY` can wake
the worker immediately. But it is **at-most-once**: sqlx's `PgListener` (a
1-connection pool that reconnects and re-subscribes on
`ConnectionAborted`/`UnexpectedEof`) **loses** notifications delivered during the
disconnect window. Therefore NOTIFY must never be the durable path.

### Design / Contract

- Wrap sqlx `PgListener` **directly** — do not reimplement LISTEN/NOTIFY.
- NOTIFY carries **invalidation intent only** (channel + key/tag). The durable
  path is always the outbox; a missed NOTIFY just means the next poll catches it.
- Channel names are escaped with sqlx's `ident()` (the same escaping `PgListener`
  uses internally).

### Rust API / Type Sketch

In `hydracache-sqlx`:

```rust
pub struct PgNotifyIntentSource {
    listener: sqlx::postgres::PgListener,
    channel: String, // escaped via ident()
}

impl PgNotifyIntentSource {
    pub async fn connect(pool: &sqlx::PgPool, channel: &str)
        -> hydracache_core::Result<Self> { /* listener.listen(ident(channel)) */ }

    /// Returns a hint that the worker should drain now. Lost notifications are
    /// harmless: the outbox poll is the backstop.
    pub async fn next_wake(&mut self) -> WakeHint { /* ... */ }
}

pub enum WakeHint { Drain, Reconnected, Closed }
```

### Implementation Steps

1. Add `PgNotifyIntentSource` wrapping `PgListener`.
2. Combine it with the outbox worker: `select!` between a poll ticker and
   `next_wake()`; either triggers `run_once()`.
3. Document that NOTIFY is a latency optimization, never a correctness guarantee.

### TESTING

- `crates/hydracache-sqlx/tests/pg_notify.rs` (`#[ignore]`, testcontainers):
  - `notify_wakes_worker_and_outbox_publishes`.
  - `lost_notify_is_recovered_by_poll` — drop the listener mid-stream, assert the
    next poll still publishes (proves at-most-once is tolerated).
  - `channel_name_is_ident_escaped`.

```powershell
cargo test -p hydracache-sqlx --test pg_notify --locked -- --ignored
```

### Pros / Risks

- Pro: low-latency wake without giving up durability. Risk: none to correctness
  by construction. **Fallback:** if `PgListener` integration slips, ship
  poll-only + an ADR noting NOTIFY deferred; poll-only counts as done.

---

## 4. Observability For Outbox And Barriers

Status: planned. **Release core.**

### Design / Contract

Add counters/snapshots to `hydracache-observability` so operators can answer:
how many outbox rows are pending, how old is the oldest, how often do publishes
fail/retry/dead-letter, and are barriers timing out. The actuator surface stays
**read-only**.

### Candidate Metrics

```text
hydracache_db_outbox_pending            (gauge)
hydracache_db_outbox_oldest_age_ms      (gauge)
hydracache_db_outbox_publish_attempt_total
hydracache_db_outbox_publish_success_total
hydracache_db_outbox_publish_failure_total
hydracache_db_outbox_dead_letter_total
hydracache_db_barrier_wait_total
hydracache_db_barrier_timeout_total
hydracache_db_barrier_wait_ms           (histogram)
```

### TESTING

- `crates/hydracache-observability/tests/db_outbox_metrics.rs`:
  - `pending_and_oldest_age_track_outbox_status`.
  - `publish_failure_and_dead_letter_counters_move`.
  - `barrier_timeout_counter_moves_on_degraded`.
  - `actuator_snapshot_serializes_outbox_and_barrier_fields` (read-only).

```powershell
cargo test -p hydracache-observability --locked outbox
```

### Pros / Risks

- Pro: turns the new write path into observable state. **Fallback:** if a field
  cannot be sourced cheaply, expose the read-only `OutboxStatus` and defer the
  histogram with an ADR.

---

## 5. Criterion Benchmarks

Status: planned. **Release core.** Closes the evidence gap: there is no
`benches/` directory anywhere in the workspace today.

### Problem / Motivation

The release claims a "boring read path" and "production-hardened" write path.
Both are unfalsifiable without measurement (review section 2.6). moka/caffeine
analysis (review section 8) confirms maintenance must stay off the read path; a
bench is how we prove it.

### Design / Contract

Add a criterion bench target. Bench groups:

- `hit` / `miss` on the local cache.
- `single_flight` (N concurrent loads of one key collapse to one loader call).
- `event_publish_no_subscriber` vs `event_publish_with_subscriber` (proves the
  `may_publish` preflight is cheap when nobody listens).
- `write_with_outbox` vs `write_without_outbox` (quantifies write amplification).

### Rust Sketch

```rust
// crates/hydracache/benches/cache_hot_path.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_hit(c: &mut Criterion) { /* get on a pre-populated key */ }
fn bench_single_flight(c: &mut Criterion) { /* concurrent get_or_load */ }
fn bench_event_publish(c: &mut Criterion) { /* with/without subscriber */ }

criterion_group!(hot_path, bench_hit, bench_single_flight, bench_event_publish);
criterion_main!(hot_path);
```

```toml
# crates/hydracache/Cargo.toml
[[bench]]
name = "cache_hot_path"
harness = false

[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
```

A second bench `crates/hydracache-db/benches/outbox_write.rs` covers
`write_with_outbox` vs `write_without_outbox`.

### TESTING / Invocation

Benches are not pass/fail tests; they must **compile and run**:

```powershell
cargo bench -p hydracache --bench cache_hot_path
cargo bench -p hydracache-db --bench outbox_write
```

A smoke check ensures the bench compiles in the gate:

```powershell
cargo check -p hydracache --benches --locked
cargo check -p hydracache-db --benches --locked
```

### Pros / Risks

- Pro: cheap, and it converts marketing claims into numbers. **Fallback:** if a
  group does not stabilize in time, ship the `hit`/`miss` group + a `// TODO`
  ADR; the bench target existing and running counts as done.

### Performance Budget (regression gate)

A bench that nobody gates on is just numbers (review follow-up). This release
defines an explicit, machine-checkable budget so "boring read path" and "bounded
write amplification" become CI-enforceable rather than aspirational.

Contract:

- Commit a baseline file `benches/baseline/0_37.json` (criterion's
  `--save-baseline`) on a designated reference machine/CI runner.
- A budget descriptor `benches/budget.toml` declares the allowed regression per
  group:

```toml
# benches/budget.toml
[hot_path.hit]            # read path must stay flat
max_regression_pct = 5
[hot_path.single_flight]
max_regression_pct = 8
[hot_path.event_publish_no_subscriber]
max_ns_absolute = 50      # preflight must be ~free when nobody listens
[outbox_write.write_with_vs_without]
max_amplification_x = 2.5 # write-with-outbox / write-without-outbox
```

- A small checker `xtask bench-budget` (or a script
  `scripts/check_bench_budget.py`) parses criterion's `estimates.json`, compares
  against the baseline, and exits non-zero if any group exceeds its budget.

Implementation steps:

1. Land the benches (above) and capture the first baseline.
2. Add `benches/budget.toml` and the checker behind `cargo xtask bench-budget`.
3. Wire it as a **nightly / pre-release** gate, not a per-PR gate: criterion is
   too noisy on shared PR runners, so the per-PR tier runs `cargo check
   --benches` (compile only) while the nightly tier runs the budget comparison.

TESTING:

- `crates/xtask/tests/bench_budget.rs` — unit tests on the parser/comparator:
  `fn within_budget_passes`, `fn over_pct_budget_fails`,
  `fn missing_baseline_is_explicit_error`, `fn amplification_ratio_computed`.
  Feed synthetic `estimates.json` fixtures, no real benchmark run required, so
  the test is deterministic and Windows-safe.
- Invocation: `cargo test -p xtask --locked bench_budget`.

Pros: turns the evidence from section 5 into a guardrail. Risk: baseline drift
across machines — mitigate by pinning one reference runner and treating the
budget as nightly, never a hard per-PR block.

---

## 5a. Compatibility And Migration Discipline (cross-release)

Status: planned. **Release core (starts the discipline used through 0.41).**

### Problem / Motivation

Across `0.37`→`0.41` the project introduces persistent, wire-visible artifacts:
the outbox table schema, the `CacheInvalidationFrame` version
(`crates/hydracache/src/invalidation_bus.rs` already carries `version: u16`), and
later the raft log format (`RaftLogStore`, `0.41`) and value-replication wire
format. Without a single versioning-and-migration story, the first rolling
upgrade of a pilot is unsafe and the `0.41` "rolling upgrade" claim has no
foundation underneath it.

### Design / Contract

Establish one cross-release contract now and keep filling it:

- **Versioned artifacts register.** A doc table `docs/COMPAT.md` lists every
  durable/wire artifact, its current version, and its compatibility window
  (which reader versions accept which writer versions).
- **Outbox schema migrations are forward-only and idempotent.** Ship migration
  files under `crates/hydracache-db/migrations/` with a checked-in
  `schema_version` row; the drain worker refuses to start against an unknown
  future schema version (fail loud, not silent).
- **Frame/wire version negotiation.** `CacheInvalidationFrame::version` is the
  single source of truth; receivers reject unknown major versions and the
  rejection is a counter, not a panic. Adding fields uses
  `#[non_exhaustive]` + optional fields so a new writer is readable by an old
  reader within the same major.
- **Public enums are `#[non_exhaustive]`** (`ConsistencyMode`, future
  `RoutingMode`, `ClusterHealthState`) so later releases add variants without a
  breaking change.

### Implementation Steps

1. Create `docs/COMPAT.md` with the initial rows: outbox schema v1,
   `CacheInvalidationFrame` v(current).
2. Add the `schema_version` row + the "refuse unknown future version" guard to
   the outbox migration runner.
3. Audit public enums added in this release and mark them `#[non_exhaustive]`.

### TESTING

- `crates/hydracache-db/tests/outbox_schema_migration.rs`:
  `fn applies_clean_then_idempotent_reapply`,
  `fn refuses_unknown_future_schema_version`,
  `fn drain_worker_aborts_on_version_mismatch` (integration, testcontainers
  Postgres, behind the same Docker gate as the outbox tests).
- `crates/hydracache/tests/frame_version_compat.rs`:
  `fn old_reader_accepts_new_minor_frame`,
  `fn reader_rejects_unknown_major_and_counts_it` (unit, deterministic).
- Invocation: `cargo test -p hydracache --locked frame_version_compat` and the
  Docker-gated `cargo test -p hydracache-db --locked --ignored outbox_schema`.

### Pros / Risks

- Pro: makes every later release's upgrade story concrete and testable; closes
  the largest gap that the review did not cover. Risk: discipline overhead —
  kept minimal by limiting it to durable/wire artifacts only, not internal types.

---

## 6. Byte-Based Weigher And Pre-Insert `max_entry_bytes` Reject

Status: planned. **Lands in the DB adapter now.**

### Problem / Motivation

`fetch_all` results are wildly heterogeneous in size; count-based capacity is
misleading (review section 8, idea 17). moka's behavior was confirmed in code:
its weigher makes `max_capacity` a byte budget, but an over-budget entry is
**admitted then evicted** (`RemovalCause::Size`) — extra churn, no pre-insert
reject.

### Design / Contract

- Byte weigher on the DB result cache:
  `.weigher(|_k, v| v.encoded_len().clamp(1, u32::MAX as usize) as u32)`.
- A **pre-insert** `max_entry_bytes` reject (unlike moka's admit-then-evict): if
  `weight > max_entry_bytes`, the value is **not** inserted.
- A **separate** counter `rejected_oversize`, distinct from `evicted_size`, so
  operators can tell "too big to cache" apart from ordinary eviction.

### Rust Sketch

```rust
pub struct DbResultCacheConfig {
    pub max_bytes: u64,
    pub max_entry_bytes: u32,
}

fn try_insert<V: Encodable>(cfg: &DbResultCacheConfig, key: &str, value: &V, m: &Metrics) {
    let weight = value.encoded_len().clamp(1, u32::MAX as usize) as u32;
    if weight > cfg.max_entry_bytes {
        m.rejected_oversize.inc(); // distinct from evicted_size
        return;                    // pre-insert reject; never touches the map
    }
    // insert with moka weigher = byte budget
}
```

### TESTING

- `crates/hydracache-db/tests/weigher.rs`:
  - `weigher_uses_encoded_byte_length`.
  - `oversize_entry_is_rejected_before_insert` — assert the key is absent
    afterward.
  - `rejected_oversize_counter_is_distinct_from_evicted_size`.
  - `byte_budget_evicts_when_total_exceeds_max_bytes`.

```powershell
cargo test -p hydracache-db --locked weigher
```

### Pros / Risks

- Pro: honest memory accounting, less churn than moka. Risk: `encoded_len` must
  be cheap; **fallback:** if a value type cannot report length cheaply, fall back
  to count-based for that type and document it.

---

## 7. `required_dimensions` Mechanism (Static Check Only)

Status: planned. **Mechanism in 0.37; profiles deferred to 0.38.**

### Problem / Motivation

Search/list keys silently leak data when a result-shaping dimension is missing
(tenant, permission, page, sort, locale...). The review (section 2.2) assigns the
**mechanism** to `0.37` (cheap, static, isolated) and the named **profiles**
(`tenant_scoped`, `paged_search`, ...) plus a CI `deny` mode to `0.38`.

### Design / Contract

`required_dimensions = [...]` on `query_cache_policy!` is a **compile-time /
static** assertion that each listed label appears among the `key_segments`
labels. It is a review aid, not a runtime correctness proof.

```rust
let policy = query_cache_policy!(
    name = "search-users",
    key_segments = ["tenant", tenant_id, "permission", permission_hash,
                    "users", "search", "query", normalized_query,
                    "page", page, "sort", sort],
    required_dimensions = ["tenant", "permission", "query", "page", "sort"],
    tag_segments = [["tenant", tenant_id, "users"]],
    ttl_secs = 30,
);
```

### Implementation Steps

1. Parse `required_dimensions` in the `query_cache_policy!` macro.
2. Emit a `compile_error!` if a required label is absent from `key_segments`
   labels, or if `required_dimensions` is used without `key_segments`.
3. Expose the declared dimensions in policy diagnostics (no sensitive values).

### TESTING

- `crates/hydracache-db-macros/tests/trybuild/required_dimensions_pass.rs` —
  passing `trybuild`.
- `.../required_dimensions_missing_label.rs` — failing `trybuild`, helpful error.
- `.../required_dimensions_without_key_segments.rs` — failing `trybuild`.
- `crates/hydracache-db/tests/required_dimensions.rs`:
  - `diagnostics_expose_required_dimension_labels`.

```powershell
cargo test -p hydracache-db-macros --locked required_dimensions
cargo test -p hydracache-db --locked required_dimensions
```

### Pros / Risks

- Pro: cheap static guard against the highest-risk class of cache key bug. Risk:
  teams may add empty labels to pass the check — this is acknowledged and the
  *binding-to-loader-arg* refinement is explicitly a `0.38` profile concern.

---

## 8. `deny.toml` Rule: `sqlparser` Out Of Runtime

Status: planned. **Machine-checkable invariant.**

### Problem / Motivation

The (deferred) SQL lint uses `sqlparser`. It must never reach a runtime crate,
even transitively (review section 9; mirrors the sqlx `sqlx-macros-core` vs
`sqlx-core` crate split).

### Design / Contract

Add a `deny.toml` ban so the parser may appear only under dev/test profiles, and
fails CI if it leaks into any runtime crate's dependency tree.

```toml
# deny.toml
[bans]
deny = [
  { name = "sqlparser", wrappers = [] },  # only allowed via dev-dependencies
]
```

### TESTING / Invocation

```powershell
cargo deny check bans
cargo tree -p hydracache-db -e no-dev | Select-String sqlparser   # must be empty
```

A CI step asserts `cargo deny check bans` passes. **Fallback:** none needed; this
is a config + check.

---

## 9. ADR Skeleton

Status: planned. Start now, fill across `0.37`–`0.40`, finalize in `0.41`
(review sections 3/0.41 and 5).

Create `docs/adr/` with skeletons (Context / Decision / Status: Proposed):

- `0008-ownership.md` — rendezvous ownership (already in
  `crates/hydracache/src/cluster.rs`).
- `0009-replication.md` — values are never replicated by the bus.
- `0010-consistency.md` — eventual default; local + best-effort barriers in
  `0.37`; quorum deferred to `0.40`.
- `0011-transport.md` — bus transport; LISTEN/NOTIFY intent-only.
- `0012-durability.md` — outbox idempotency key `(commit_position, target_hash)`
  + advance-after-apply.

Each hedged/deferred decision in this plan records its ADR here.

### TESTING

- `docs` link check / a doc test asserting every ADR file exists and has the
  required headings (a small script test under `scripts/`).

---

## What Moved Out Of 0.37 (to 0.37.x / 0.38)

Stated explicitly so reviewers do not expect these in `0.37`:

- **SQL-dependency metadata + optional `sqlparser` lint** → `0.37.x`/`0.38`.
  (`SqlDependency`, `depends_on(...)` ergonomics, lint helper.) The runtime ban
  via `deny.toml` (section 8) stays in `0.37`.
- **`prepared_query_policy!` repository-method hardening** → `0.38`.
- **`#[hydracache::cacheable]` repository-attribute hardening** → `0.38`.
- **`required_dimensions` named profiles** (`tenant_scoped`, `paged_search`) +
  CI `deny` mode → `0.38`. The static mechanism (section 7) stays in `0.37`.
- **Full four-way testcontainers matrix** (Diesel/SeaORM × Postgres/MySQL) →
  `0.38`. `0.37` mandatory minimum is **SQLx×Postgres + one ORM×Postgres**;
  every other pair is `#[ignore]` + a documented blocker, never a release gate.
- **Trigger/CDC external-writer docs and bridge** → `0.37.x`. CDC is a
  connector-crate concern (`hydracache-cdc-postgres`) that publishes intent into
  the existing bus; it does not enter `0.37` core.

## Dependency On Later Releases

- **`quorum` / `all-peers` read-after-write barrier → `0.40`.** Requires the
  pilot's fixed 2–5 member topology and partition-timeout accounting. `0.37`
  ships only `Local` + `BestEffort`. This is a hard, stated dependency.
- **CDC value-free intent connector → `0.38`+.** Out of `0.37` scope by the
  readyset anti-scope (no value-serving CDC).
- **Durable Raft / value replication → `0.41`.** The byte weigher (section 6)
  is the early down-payment on `max_replicated_entry_bytes`.

---

## Release Gates

### Required Local Gate

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo check -p hydracache --benches --locked
cargo check -p hydracache-db --benches --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
cargo deny check bans
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
.\scripts\verify-release-readiness.ps1 -Version 0.37.0 -RunGate
```

If Windows linker locks reappear:

```powershell
$env:CARGO_BUILD_JOBS = "1"
cargo test --workspace --all-targets --locked
```

### Required Focused Gates

```powershell
cargo test -p hydracache-db --locked outbox
cargo test -p hydracache-db --test outbox_property --locked
cargo test -p hydracache-db --locked weigher
cargo test -p hydracache --locked barrier
cargo test -p hydracache-observability --locked outbox
cargo test -p hydracache-db --locked required_dimensions
cargo test -p hydracache-db-macros --locked required_dimensions
```

### Mandatory-Minimum Docker Gate

```powershell
cargo test -p hydracache-db  --test outbox_postgres --locked -- --ignored
cargo test -p hydracache-sqlx --test pg_notify       --locked -- --ignored
```

Every other ORM×backend pair is `#[ignore]` with a documented blocker and is
**not** a release blocker.

### Benchmark Gate (run, not pass/fail)

```powershell
cargo bench -p hydracache    --bench cache_hot_path
cargo bench -p hydracache-db --bench outbox_write
```

### Packaging Gate

```powershell
.\scripts\package-publishable.ps1 -Set bootstrap
.\scripts\package-publishable.ps1 -Set runtime
.\scripts\package-publishable.ps1 -Set adapters
```

---

## Implementation Order

1. Add this plan and the ADR skeleton (`docs/adr/`).
2. Add the `deny.toml` `sqlparser` ban + CI check.
3. Add `InvalidationIntent` + `target_hash` + the escaping property test.
4. Add the outbox trait, SQLite reference impl, migrations, schema check.
5. Add the circuit-breaker worker (claim → apply → advance frontier),
   dead-letter, and `reset_dead_letters`.
6. Add the read-only `OutboxStatus` surface.
7. Add `InvalidationReceipt` + `Local`/`BestEffort` barriers.
8. Add `PgNotifyIntentSource` (`PgListener` wrapper).
9. Add observability counters + actuator serialization.
10. Add the byte weigher + pre-insert `max_entry_bytes` reject + counters.
11. Add the `required_dimensions` static mechanism + `trybuild` tests.
12. Add criterion benches and the `--benches` compile check to the gate.
13. Add the mandatory-minimum Postgres testcontainers tests.
14. Update release notes; bump versions; verify; tag; package; publish.

Run the narrowest meaningful test set after each commit; run the full local gate
before the release commit.

## Final Release Decision

`0.37.0` may claim "production-hardened database caching" **only if every one of
these boolean conditions is true.** There is no numeric self-score.

- [ ] Outbox intent commits/rolls back atomically with the data write.
- [ ] The idempotency key is `(commit_position, sha256(target))` and a double
      drain after a simulated crash is idempotent (test passes).
- [ ] The worker advances its durable frontier **only after** applying, proven
      by `frontier_advances_only_after_apply`.
- [ ] The worker has circuit-breaker backoff, dead-lettering, and an operator
      `reset_dead_letters` (tests pass).
- [ ] Worker status is exposed through a **read-only** surface only.
- [ ] Intent escaping has a passing **property** test against collisions.
- [ ] `Local` and `BestEffort` barriers pass success, timeout-degraded, and
      backward-compat tests. `quorum` is documented as deferred to `0.40`.
- [ ] `PgListener` intent transport tolerates lost notifications (poll backstop
      test passes), or is shipped poll-only with an ADR.
- [ ] Outbox lag/backlog/failure and barrier wait/timeout counters move on the
      right paths (tests pass); actuator output is read-only.
- [ ] `cargo bench` for `cache_hot_path` and `outbox_write` builds and runs, and
      `--benches` compiles in the gate.
- [ ] Byte weigher + pre-insert `max_entry_bytes` reject ship with a **distinct**
      `rejected_oversize` counter (tests pass).
- [ ] The `required_dimensions` static mechanism has passing/failing `trybuild`
      coverage.
- [ ] `cargo deny check bans` passes and `sqlparser` is absent from every runtime
      crate's non-dev dependency tree.
- [ ] The ADR skeleton exists and each hedged/deferred decision is recorded.
- [ ] Mandatory-minimum SQLx×Postgres + one Postgres path passes under
      `--ignored`; all other pairs are documented blockers, not gates.
- [ ] Every new code path added in this release has tests.
- [ ] Docs state exactly what is **not** guaranteed (no transparent SQL, no ORM
      L2 cache, no dataflow/planner/proxy, no value-serving CDC, no `quorum`
      barrier until `0.40`).
