# HydraCache 0.55.0 Durable Store Hardening & Cluster-Wide Checkpoints — Codex Execution Plan

> **At a glance**
> - **What:** harden the shipped `0.51` durable value plane and add cluster-wide consistency to
>   its snapshots: (1) **extend the existing `ReplicatedValueStore` trait** (hardening.rs:367,
>   already implemented by the sled `DurableValueStore` and the in-memory store — **no new trait**)
>   with the hardening methods (`scan_all`/`remove`/`compact`), keeping redb/RocksDB drop-in
>   (TD-0003); (2) add an
>   **inspect/dump tool + background scrubber** so corruption is found proactively and fail-loud;
>   (3) **maintenance** — tombstone GC, compaction controls, byte-budget hardening — leveraging
>   sled, not replacing it; (4) a **barrier-aligned cluster-wide consistent checkpoint** and a
>   **rescale-with-checkpoint** flow (arroyo pattern) beyond today's per-namespace scheduled
>   snapshot; (5) a **poison-load circuit-breaker** over the single-flight loader.
> - **Why:** `0.51` shipped `DurableValueStore` (sled, versioned+checksummed records, per-namespace
>   scheduled snapshots, fail-loud recovery). The remaining gaps are **operability** (no domain
>   inspector/scrubber), **engine flexibility** (sled hard-coded), **cluster-wide** consistency
>   (snapshots are per-namespace, not a coordinated cut), and **loader resilience** (a key whose
>   loader keeps failing hammers the DB). Blueprints: blazingmq `mqbs` (storage discipline/inspect),
>   tigerbeetle §6.2 (background scrub), tantivy §3.1 / tikv engine_traits (pluggable engine),
>   arroyo controller §4.5 (barrier checkpoint), blazingmq poison-pill (poison-load).
> - **After (depends on):** `0.51` (persistence), `0.43` (tiered store + online reshard), validated
>   by `0.44` DST. Independent of `0.52`/`0.53`/`0.54`.
> - **Promoted from** `V0_DRAFT_DURABLE_STORE_HARDENING_PLAN.md` (D1–D4 expanded into W1–W6 with
>   an **honest sled reframing** — see below).
> - **Status:** planned.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> storage direction: [`../STORAGE_AND_DATA_PLATFORM_EVOLUTION.md`](../STORAGE_AND_DATA_PLATFORM_EVOLUTION.md) ·
> competitive: [`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its
Definition of Done **and** `cargo xtask verify`; never push red. Crash/restart behavior is added
to the `0.44` `hydracache-sim` deterministic harness (R-5).

## Justification (why this, why now — and the honest sled reframing)

Verified against the code, `0.51` already delivers more than the draft assumed:

- `crates/hydracache/src/grid/durable_store.rs`: `DurableValueStore` over **sled** (`sled::Db`),
  `DURABLE_VALUE_FORMAT_VERSION: u32 = 1`, a `FORMAT_KEY` marker validated on open
  (`validate_or_initialize_format` — **refuses an unknown future format, loud**), **checksummed**
  records (encode/decode with a format version + checksum), a **byte budget** with
  `rejected_total()`, and `open`/`upsert`/`get`/`iter`/`total_bytes`/`flush`.
- `crates/hydracache/src/grid/durability.rs`: the write path + snapshot scheduler +
  `DurabilitySnapshotManifest` (COMPAT-registered format `1`, covering `(partition, version,
  epoch)` watermark). `crates/hydracache/src/grid/recovery.rs`: fail-loud full-restart recovery.
  `crates/hydracache/src/grid/persistence_config.rs`: the per-namespace/region policy resolver.

**Honest reframing of the draft's blazingmq file-store items (D1/D2):** because the backend is
**sled**, HydraCache does **not** hand-roll a custom on-disk file-store format or file-set rotation
— sled owns files, compaction, and its own format. So the draft's "versioned file-store format +
iterators + file-set rotation" (blazingmq `mqbs_filestore`) is reframed to what sled does **not**
give and what actually hardens the plane:

- **Engine flexibility** — the abstraction **already exists**: `pub trait ReplicatedValueStore`
  (`grid/hardening.rs:367`, methods `upsert`/`get`/`tombstone`/`scan_owned`) is implemented by both
  `InMemoryReplicatedValueStore` (hardening.rs:397) and `DurableValueStore` (durable_store.rs:116).
  So W1 does **not** create a parallel `DurableValueBackend`; it **extends `ReplicatedValueStore`**
  with the methods hardening needs (`scan_all`, `remove`, `compact`) so redb/RocksDB stay drop-in
  later (TD-0003). **Note:** the trait is **not object-safe** (`upsert(impl Into<String>)`), so it
  is used as a generic bound `B: ReplicatedValueStore`, never `dyn` — W1 keeps that shape.
- **Operability** instead of a bespoke print-util format → a **domain-aware inspect/dump tool** +
  a **background scrubber** over the existing checksums (tigerbeetle §6.2) (W2).
- **Maintenance** instead of custom file-set rotation → **tombstone GC + compaction controls +
  budget hardening**, driving sled's compaction rather than replacing it (W3).

The genuinely-new items are cluster-wide **consistent checkpoints** (W4, arroyo) and the
**poison-load circuit-breaker** (W5). Nothing here turns HydraCache into a database (R-9); sled/the
durable plane remains a *cache* survivability layer, the SQL DB stays the system of record.

## Release Theme

Make the `0.51` durable plane **engine-flexible, inspectable, self-scrubbing, and maintainable**,
add a **cluster-wide consistent checkpoint** and **rescale-with-checkpoint**, and add a
**poison-load circuit-breaker** — without a new consistency level (R-1), without becoming a
database (R-9), and without regressing the RAM-only default (R-10).

## Non-Goals

- **No new consistency level (R-1).** Checkpoints add *coordination*, not a tunable mode; recovery
  still reconciles against the epoch/version authority.
- **Not a database / not a source of truth (R-9).** The durable plane survives a restart; the SQL
  DB stays authoritative. No distributed transactions, no cross-node atomic multi-key durability.
- **No always-on cost (R-10).** Everything is opt-in per the `0.51` persistence policy; unconfigured
  namespaces stay RAM-only, byte-for-byte identical to prior releases.
- **No storage-engine *research*.** W1 **extends the existing `ReplicatedValueStore` trait**; sled
  stays the one durable reference impl. Choosing redb/RocksDB as a default is TD-0003, not this
  release.
- **No silent degradation (R-3).** Corruption, unrecoverable checkpoints, or over-budget writes are
  **loud + counted**, never a quiet partial.

## Inherited Boundary (assumes 0.51 + 0.43 + 0.44)

- **`ReplicatedValueStore` (hardening.rs:367) + `DurableValueStore` (durable_store.rs:116) +
  `InMemoryReplicatedValueStore` (hardening.rs:397)**: the existing engine seam W1 **extends** (not
  replaces); keep sled's format version, checksum, byte budget, and the loud unknown-format reject
  (`validate_or_initialize_format`, durable_store.rs:178) unchanged. Trait stays a generic bound
  (not object-safe).
- **`durability.rs` + `DurabilitySnapshotManifest`**: the per-namespace snapshot path W4 extends
  into a cluster-wide cut; reuse the `(partition, version, epoch)` watermark and its COMPAT entry.
- **`recovery.rs`**: fail-loud full-restart recovery; the cluster-wide checkpoint restores through
  the same epoch/version authority (R-1).
- **`grid/elasticity.rs` (0.43 online reshard)**: W4's rescale-with-checkpoint drives this; do not
  reinvent rebalance.
- **`grid/hardening.rs` `ReplicatedValueRecord` + checksum**: the scrubber (W2) verifies these.
- **Single-flight loader (`refresh.rs`/`cache.rs`, ADR 0013)**: the seam W5's poison-load breaker
  wraps; do not change the fast-path loader semantics for healthy keys.
- **`0.44` DST harness + scrubber/checksums**: the validation substrate (W6).

## Dependency Graph

```
0.51 DurableValueStore (sled) + 0.43 reshard + 0.44 DST
        │
        ▼
W1 extend existing ReplicatedValueStore trait (scan_all/remove/compact; no behaviour change)  ◄ foundation
        │
        ├──────────► W2 inspect tool + background scrubber (corruption fail-loud)
        ├──────────► W3 maintenance: tombstone GC + compaction + budget hardening
        ▼
W4 cluster-wide consistent checkpoint (barrier) + rescale-with-checkpoint
        │
        ▼
W5 poison-load circuit-breaker over the single-flight loader
        │
        ▼
W6 DST validation + COMPAT + operability gates (cross-cutting)
```

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests) / Risk & rollback.**

---

## W1. Extend the existing `ReplicatedValueStore` trait with the hardening surface

**Goal.** The engine abstraction already exists (`ReplicatedValueStore`, hardening.rs:367; impls
`InMemoryReplicatedValueStore` + `DurableValueStore`). Extend it with the **minimum** methods W2–W3
need — a full scan for scrub/inspect, a direct `remove`, a `compact`, and hoisted budget accessors
— **without** breaking existing callers and **without** any behaviour change for current methods.

**Files.** `crates/hydracache/src/grid/hardening.rs` (extend the trait + `InMemoryReplicatedValueStore`
impl), `crates/hydracache/src/grid/durable_store.rs` (extend the `DurableValueStore` impl), audit
callers of `ReplicatedValueStore` (durability.rs, recovery.rs, elasticity.rs) for compile impact.

**Steps.**
1. **Add methods to `ReplicatedValueStore`** (keep it a generic bound — it is **not** object-safe
   because `upsert(impl Into<String>)`; do not try to make it `dyn`):
   ```rust
   pub trait ReplicatedValueStore: Send + Sync {
       // ...existing: upsert / get / tombstone / scan_owned...

       /// Iterate every stored record (unfiltered) — for scrub/inspect/GC. Distinct from
       /// scan_owned, which filters by the effective replication map.
       fn scan_all(&self) -> Result<Vec<(String, ReplicatedValueRecord)>, ValueStoreError>;

       /// Remove one record outright (used by tombstone GC after the repair gate).
       fn remove(&mut self, key: &str) -> Result<(), ValueStoreError>;

       /// Reclaim space / trigger backend compaction; return reclaimed bytes.
       /// Default: no-op returning 0 (the in-memory store overrides trivially).
       fn compact(&mut self) -> Result<u64, ValueStoreError> { Ok(0) }

       /// Total budgeted bytes retained (hoisted from the inherent impls for uniformity).
       fn total_bytes(&self) -> Result<u64, ValueStoreError>;

       /// Rejected-upsert count (hoisted).
       fn rejected_total(&self) -> u64;
   }
   ```
2. **`InMemoryReplicatedValueStore`** (hardening.rs:397): implement `scan_all` (clone
   `records` — it already exposes `snapshot()`), `remove` (`records.remove`), `compact` (no-op,
   returns 0), and move its inherent `total_bytes`/`rejected_total` (hardening.rs:414/422) to satisfy
   the trait (keep the inherent ones too, or delegate).
3. **`DurableValueStore`** (durable_store.rs:116): `scan_all` = the existing `scan_prefix(RECORD_PREFIX)`
   walk (mirror `scan_owned`, durable_store.rs:160-174, but without the map filter); `remove` =
   `self.db.remove(record_key(key))` + flush; `compact` = sled compaction (see W3); the inherent
   `total_bytes` (durable_store.rs:58) / `rejected_total` (:70) satisfy the trait. Format version,
   checksum, budget, and the **loud unknown-format reject** (`validate_or_initialize_format`,
   durable_store.rs:178) are unchanged.
4. **Do not change** `upsert`/`get`/`tombstone`/`scan_owned` semantics; `compact` has a default so
   any other impl compiles unchanged. Confirm every `ReplicatedValueStore` caller still builds.

**DoD.** `crates/hydracache/tests/replicated_store_ext.rs`
- `scan_all_returns_every_record_both_impls` (in-memory + durable parity; includes tombstones).
- `remove_deletes_only_the_targeted_key`.
- `compact_returns_reclaimed_bytes_or_zero` (durable > 0 after churn; in-memory = 0).
- `total_bytes_and_rejected_total_match_inherent` (hoisting is behaviour-preserving).
- `existing_upsert_get_tombstone_scan_owned_unchanged` (golden vs pre-change on both impls).
- `unknown_future_format_still_refuses_to_open` (durable, loud reject preserved — R-3).
- Run: `cargo test -p hydracache --features durable-value-store --locked replicated_store_ext`.

**Risk & rollback.** Additive trait methods (one with a default) — low blast radius, but every
existing impl of `ReplicatedValueStore` must gain `scan_all`/`remove`/`total_bytes`/`rejected_total`
(no default for those, to force a real impl). If an out-of-tree/mock impl exists in tests, update it.
Revert removes the new methods; W2/W3 depend on them so they revert together.

## W2. Inspect tool + background scrubber (corruption fail-loud)

**Goal.** Give operators a **domain-aware inspector** (dump/scan records, show version/epoch/
tombstone/checksum status) and a **background scrubber** that proactively verifies checksums and
fails loud on corruption — the operability sled does not provide.

**Files.** `crates/hydracache/src/grid/durable_inspect.rs` (inspect/dump over the W1 trait's
`scan_all`), `crates/hydracache/src/grid/durable_scrub.rs` (a `Scrubber` verifying `ReplicatedValueRecord`
checksums, `grid/hardening.rs`), a `hydracache-server`/`xtask` subcommand to run the inspector
offline (leaning on the `0.48` server surface).

**Steps.**
1. Inspector: use W1's `scan_all` for the healthy dump — report `{key, version, epoch, tombstone,
   approx_bytes}` per record. (For the durable store this decodes via `decode_record`,
   durable_store.rs:~204/258, so a corrupt record surfaces as an error, mirroring the existing
   `corrupt_record_is_detected_not_served` `0.51` guarantee — never served.)
2. Scrubber (durable-specific — it needs raw bytes): walk the raw sled tree
   `self.db.scan_prefix(RECORD_PREFIX)` and **decode each record independently** — a decode/checksum
   failure is **one** corruption counted (`durable_scrub_corruption_total`), it does **not** abort
   the scan (contrast `scan_all`, which fails fast). Run on a schedule (interval from config),
   **bounded** to a slice per tick via a persisted cursor (never a full-store stall). On corruption:
   **fail loud + count**, never auto-"repair" silently (repair, if any, is explicit via the `0.45`
   Merkle path).
3. Bounded-label metrics (R-6): `durable_scrub_records_total`, `durable_scrub_corruption_total`,
   `durable_scrub_cycle_seconds`, `durable_scrub_cursor_gauge`.

Corner cases to cover: empty store (scrub is a no-op, no false corruption); a store that is **all
tombstones** (each is a valid record, not corruption); the store **grows mid-scan** (the bounded
cursor advances deterministically, no missed/duplicated verification); a **truncated/torn** record
(decode fails → counted, not panicked); a record with a valid format marker but a **bad checksum**
(counted).

**DoD.** `crates/hydracache/tests/durable_scrub.rs` + `durable_inspect.rs`
- `inspect_dumps_records_with_status`.
- `scrubber_detects_injected_corruption_and_fails_loud` (**falsifiable**: without the scrub the
  corruption is latent; with it, it fires — inject via `put_raw_record_for_test`, durable_store.rs:88,
  or the `0.44` fault-injecting storage).
- `scrub_does_not_abort_on_one_corrupt_record` (independent per-record decode — counts, continues).
- `scrubber_is_bounded_per_cycle_via_cursor_not_a_full_store_stall`.
- `scrub_over_empty_store_and_all_tombstones_reports_zero_corruption` (corner cases).
- `scrub_cursor_is_deterministic_when_store_grows_mid_scan` (no missed/duplicated verification).
- `torn_or_bad_checksum_record_is_counted_not_panicked`.
- `corrupt_record_is_never_served` (regression of the `0.51` guarantee through the new path).
- Run: `cargo test -p hydracache --features durable-value-store --locked durable_scrub durable_inspect`.

**Risk & rollback.** Scrubber must stay bounded per cycle (no stall). Revert removes the inspector +
scrubber module; the store is unaffected.

## W3. Maintenance — tombstone GC + compaction controls + budget hardening

**Goal.** Reclaim space and keep the store lean over time — **tombstone/expired GC** (never
resurrect deleted data), **compaction controls** driving sled's compaction, and hardened byte-budget
accounting — instead of a custom file-set rotation (sled owns files).

**Files.** extend `durable_store.rs` (`compact()`, tombstone/expiry GC honoring the repair-gated
tombstone discipline, `grid/mod.rs` `TombstoneTracker`/`TombstoneBudget`), config knobs in
`persistence_config.rs`.

**Steps.**
1. Tombstone/expired GC: walk `scan_all` (W1), select tombstoned (`ReplicatedValueRecord::is_tombstone`,
   grid/mod.rs:430) / TTL-expired records, and `remove` (W1) them **only after** the repair gate is
   satisfied — a tombstone is GC-able only when `TombstoneTracker` shows no repair debt for it
   (`confirm_repair(key, epoch)` recorded, `repair_debt()` false, grid/mod.rs:531/538). This is what
   stops GC from resurrecting deleted data across replicas (R-3). Count `durable_gc_reclaimed_total`
   / `durable_gc_skipped_repair_pending_total`.
2. `compact()` (the W1 trait method): for the durable store trigger sled compaction (sled reclaims
   space from its own files — HydraCache does **not** rotate a custom file set) and report reclaimed
   bytes; expose a scheduled cadence (config) + an on-demand admin call. Must not block writes
   unboundedly (compact off the write path).
3. Budget hardening: keep `total_bytes()`/`rejected_total()` exact while GC/compaction run
   concurrently with upserts; over-budget writes stay rejected + counted (`rejected_total`,
   durable_store.rs:70/124), and reclaimed bytes feed back so a post-GC upsert that now fits is
   accepted.

Corner cases: a tombstone with **pending repair debt** is **not** GC'd (no premature reclaim); a
**live** record is never GC'd; GC of the last record leaves a consistent empty store; `compact`
on an empty store returns 0; a concurrent upsert during GC keeps the budget exact (no under/over
count); a TTL-expired **but un-repaired** tombstone stays until the gate clears.

**DoD.** `crates/hydracache/tests/durable_maintenance.rs`
- `tombstone_gc_reclaims_only_after_repair_gate` (no premature GC).
- `gc_never_resurrects_deleted_data` (R-3, cross-replica in the `0.44` sim).
- `compaction_reclaims_bytes_and_reports`.
- `budget_is_exact_under_concurrent_gc` (no under/over-count).
- Run: `cargo test -p hydracache --features durable-value-store --locked durable_maintenance`.

**Risk & rollback.** GC correctness (no resurrection) is the load-bearing property — gate it on the
existing repair machinery. Revert removes GC/compaction controls; the store still works (just grows).

## W4. Cluster-wide consistent checkpoint + rescale-with-checkpoint

**Goal.** Beyond `0.51`'s per-namespace scheduled snapshot, add a **coordinated cluster-wide
consistent cut** (barrier-aligned, arroyo controller §4.5) and a **rescale-with-checkpoint** flow
(stop → checkpoint → redistribute → resume) for `0.43` online reshard.

**Files.** `crates/hydracache/src/grid/checkpoint.rs` (new: a `CheckpointCoordinator` producing a
cluster-wide manifest over the per-node `DurabilitySnapshotManifest`), integrate with
`grid/elasticity.rs` (reshard) and `recovery.rs` (restore).

**Steps.**
1. Coordinate a consistent point: the coordinator asks each node to snapshot at a covering
   `(epoch, version)` watermark (reuse the `DurabilitySnapshotManifest`), collects them into a
   **cluster checkpoint manifest** (format registered in `docs/COMPAT.md`, R-4), and marks it valid
   only when every partition's watermark is covered — a torn/partial cut is **rejected loud**, never
   stored (R-3).
2. Rescale-with-checkpoint: on reshard, take a checkpoint, redistribute partitions (0.43 path), and
   resume — losing **no committed write** across the move; validate in the `0.44` DST.
3. Restore a cluster checkpoint through `recovery.rs` against the epoch/version authority (R-1); a
   checkpoint referencing an unknown future format refuses to restore (R-4).

Corner cases: a node crashes **mid-checkpoint** (the partial manifest is rejected loud, the prior
valid checkpoint stays authoritative); a partition's watermark is **missing/stale** (cut invalid,
loud); concurrent **writes during the barrier** land after the covering watermark (not in this cut,
appear in the next — no torn value); a **reshard interrupted by a crash** resumes from the last valid
checkpoint with no lost committed write; restoring a checkpoint whose epoch is **older** than the
current authority is fenced (R-1).

**DoD.** `crates/hydracache/tests/cluster_checkpoint.rs` + `hydracache-sim/tests/checkpoint_sim.rs`
- `cluster_checkpoint_is_a_consistent_cut` (no torn cross-node state; partial cut rejected loud).
- `rescale_with_checkpoint_loses_no_committed_write` (sim, seeded, over the reshard path).
- `checkpoint_restore_reconciles_with_epoch_version_authority`.
- `unknown_future_checkpoint_format_refuses_to_restore` (R-4).
- Run: `cargo test -p hydracache --locked cluster_checkpoint` + `cargo test -p hydracache-sim --locked checkpoint_sim`.

**Risk & rollback.** Coordination correctness under concurrent writes/reshard is the hard part;
the DST run is the proof. Authority stays epoch/version (R-1) — this adds coordination, not a new
consistency mode. Revert keeps the `0.51` per-namespace snapshot as the only checkpoint.

## W5. Poison-load circuit-breaker over the single-flight loader

**Goal.** A key whose single-flight loader repeatedly fails is **quarantined + backed off + counted**
instead of hammering the DB — the blazingmq poison-pill analog for the load path.

**Files.** extend the single-flight loader seam (`crates/hydracache/src/refresh.rs` /
`cache.rs`, ADR 0013): a per-key `LoadBreaker { failures, opened_at, half_open }`.

**Steps.**
1. On repeated load failures for a key (threshold from config), **open the breaker**: subsequent
   loads for that key fail fast (or serve stale-if-allowed) with backoff, so a broken loader does
   not stampede the DB. Count `load_breaker_open_total`.
2. **Half-open probe** after backoff: one trial load; success closes the breaker, failure re-opens
   with longer backoff. Never serve resurrected/stale-as-fresh data (R-3) — a served stale value is
   explicitly marked stale, honoring the existing freshness contract.
3. Bounded-label metrics: `load_breaker_open_total`, `load_breaker_half_open_total`,
   `load_breaker_recovered_total`, `load_breaker_rejected_total`.

Corner cases: a **transient** single failure does **not** trip the breaker (threshold > 1); a key
recovers on the **first half-open probe** (breaker closes cleanly, no lingering backoff); a key that
**keeps failing** re-opens with longer backoff (no probe stampede); the breaker is **per-key** (a
poison key does not block healthy keys); a healthy key with a closed breaker pays **zero** extra
overhead (fast-path unchanged); concurrent misses on an open-breaker key **all** fail fast (the
loader is called at most once for the probe, not per caller — reuse single-flight coordination).

**DoD.** `crates/hydracache/tests/load_breaker.rs`
- `repeated_load_failure_trips_breaker_and_counts`.
- `open_breaker_fails_fast_and_does_not_stampede_the_loader` (assert the loader is not called while
  open).
- `half_open_probe_recovers_or_reopens`.
- `breaker_never_serves_stale_as_fresh` (R-3).
- Run: `cargo test -p hydracache --locked load_breaker`.

**Risk & rollback.** Must not change the healthy-key fast path (breaker is closed = no overhead).
Revert removes the breaker; loaders behave as today (retry-on-every-miss).

## W6. DST validation + COMPAT + operability gates (cross-cutting)

**Goal.** Prove the durability/checkpoint/GC behavior under crash/restart/reshard in the `0.44`
simulator, and register every new durable/wire format.

**Files.** `crates/hydracache-sim/tests/durable_hardening_sim.rs` (new), `docs/COMPAT.md` (cluster
checkpoint manifest format + any record-format change), `FEATURE_MATRIX.md`.

**Steps.**
1. DST: crash/restart across a cluster checkpoint, torn-write + corruption injection (0.44 storage
   fault model), reshard-with-checkpoint, GC-under-fault — assert no lost committed write, no
   resurrection, no torn cut served; replay/shrink on failure (R-5).
2. COMPAT: register the cluster **checkpoint manifest** format with reader window + loud reject;
   if any record/backend format changes, register it too (R-4).
3. Operability: `FEATURE_MATRIX.md` documents the inspector/scrubber/compaction surface;
   `hydracache-server`/`xtask` inspect subcommand documented.

**DoD.** `crates/hydracache-sim/tests/durable_hardening_sim.rs`
- `checkpoint_survives_crash_restart_deterministically`.
- `torn_write_and_corruption_are_detected_not_served`.
- `reshard_with_checkpoint_loses_no_committed_write`.
- Each logs+replays its seed (R-5).
- Run: `cargo test -p hydracache-sim --locked durable_hardening_sim` + `cargo xtask verify`.

**Risk & rollback.** Test/doc only; if the 0.44 harness lacks a fault (e.g. torn write on the sled
path), add it to the shared fault injector in the same PR.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green (fmt, clippy, tests, doc-check, COMPAT, deny).
- Backend behind a trait with **zero behaviour change** for sled (W1 golden test green).
- Inspector + **bounded** scrubber; corruption is proactively detected and **fail-loud** (W2).
- Tombstone GC **never resurrects** deleted data; budget exact under GC/compaction (W3).
- Cluster-wide checkpoint is a **consistent cut** (torn cut rejected loud); rescale-with-checkpoint
  loses no committed write; restore reconciles with epoch/version (W4).
- Poison-load breaker protects the DB without changing the healthy-key fast path (W5).
- New formats (cluster checkpoint manifest, any record/backend change) registered in
  `docs/COMPAT.md` with reader window + loud unknown-future reject (R-4).
- RAM-only default byte-for-byte unchanged (R-10); no new consistency level (R-1); not a database
  (R-9). Metrics bounded-label (R-6). No numeric self-score (R-7).
- `V0_DRAFT_DURABLE_STORE_HARDENING_PLAN.md` marked superseded-by-`0.55`; `releases.toml` + `INDEX.md`
  updated.
