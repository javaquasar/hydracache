# HydraCache 0.51.0 Configurable Per-Namespace / Per-Region Persistence — Codex Execution Plan

> **At a glance**
> - **What:** give HydraCache *selective, opt-in* durability in the Hazelcast style — a real on-disk value-store backend plus a **per-namespace policy** (with wildcard/prefix patterns like `cache.*`, `wallet.*`) and **per-geo-region** selection, so only **important** namespaces/regions persist and survive a full-cluster restart while everything else stays lean RAM-only. Includes a recovery model on restart (validation/data-load timeouts, recovery policy, stale-data fencing) and declarative config mirroring Hazelcast's per-map `data-persistence`.
> - **Why:** today the value plane is **RAM-only** (`InMemoryReplicatedValueStore`; the "durable" names cover only the *format/seam*, not disk). A full simultaneous cluster restart loses **all** cached values. Operators need what Hazelcast gives them: persistence configured *flexibly, per important region*, not all-or-nothing — survive a reboot for the data that matters, pay nothing for the data that doesn't.
> - **After (depends on):** 0.45 (region/active-active model). Also builds on 0.43 `TieredValueStore` (the cold-tier seam the durable backend plugs into) and is validated by 0.44 DST. The durable backend is *foundational*, so — like 0.50 — this **may be pulled forward** ahead of 0.46–0.49; it is numbered 0.51 only to avoid renumbering the in-flight 0.46–0.49 line.
> - **Unblocks:** stronger DR for 0.48 backup/PITR (persistent namespaces have a durable source on each node) and Hibernate L2 region durability in 0.49.
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · storage direction: [`../STORAGE_AND_DATA_PLATFORM_EVOLUTION.md`](../STORAGE_AND_DATA_PLATFORM_EVOLUTION.md) · positioning: [`../POSITIONING.md`](../POSITIONING.md)

This plan is written for an autonomous coding agent (Codex). Read [`CLAUDE.md`](../../CLAUDE.md),
[`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md) first. One work item =
one commit/PR; after each, run its Definition of Done **and** `cargo xtask verify`;
never push red. Where behavior is multi-node or crash/restart, add coverage to the
`0.44` `hydracache-sim` deterministic harness.

## Justification (why this, why now)

HydraCache's honest weakness (`POSITIONING.md`) includes "not yet production-deployable"
and its storage today is RAM-only on the value plane: `InMemoryReplicatedValueStore`
is a `BTreeMap` in memory, and `TieredValueStore<S>`'s "cold" tier is parameterized over
the same in-memory store — the only `ReplicatedValueStore` impl. The raft control plane
already has a real durable backend behind the `sled-log-store` feature
(`crates/hydracache-cluster-raft/src/log_store.rs`), proving the durable seam pattern;
the value plane has no equivalent. Consequently a full simultaneous cluster restart
returns every node empty (verified against the code).

The uploaded Hazelcast configuration shows the target operators expect: a cluster that
is mostly in-memory, with persistence enabled **selectively** — `cache.jwt.pem` carries
`data-persistence: enabled: true` while the rest of `cache.*` does not, and policy
(TTL/idle/eviction/backup-count/in-memory-format) is set **per named map or wildcard
pattern** (`cache.*`, `common.*`, `wallet.*`, `starfish.*`). This is exactly the
"flexible persistence only for important regions" capability requested. HydraCache
already has the prerequisites to express it cleanly: `namespace` is first-class
(`cache.rs` typed caches, `consistency.rs`), and `RegionId` + placement
(`grid/elasticity.rs`, `grid/active_active.rs`) give the per-region axis.

This release turns "everything is volatile" into "the data you mark important is durable
and survives a reboot, everything else stays a lean RAM cache" — the single most-asked
storage capability, delivered as an **opt-in** layer that does not regress the embedded
or RAM-only default (R-10) and does not turn HydraCache into a database (R-9; the SQL DB
remains the system of record).

## Release Theme

Selective, declarative durability: a real on-disk value-store backend, a per-namespace
and per-region **persistence policy resolver**, scheduled-snapshot + write-through
durability for persistent namespaces only, and a fail-loud recovery model on
full-cluster restart — without weakening any `0.37`–`0.50` guarantee, without changing
the consistency contract, and without becoming a database (R-1, R-2, R-9).

The work is seven items (W1–W6) plus a DST validation item (W7) and explicit deferrals.

## Non-Goals

- **Not all-or-nothing, and not always-on.** Persistence is **opt-in per namespace/
  region**; the default and every unconfigured namespace stay RAM-only, byte-for-byte
  identical to prior releases (R-10). There is no global "persist everything" default.
- **Not a database / not a new source of truth.** Persistence makes a *cache* survive a
  restart; the SQL DB remains authoritative. No distributed transactions, no cross-node
  atomic multi-key durability (R-2, R-9).
- **No new consistency level.** Recovery must reconcile with the existing epoch/version
  authority (R-1); it does not add a new tunable consistency mode (that is 0.46).
- **No new storage *engine* research.** The backend is abstract behind a trait with one
  reference durable impl (sled, reusing the proven `sled-log-store` pattern); choosing
  redb/RocksDB as the long-term default is a separate decision tracked in
  [`TD-0003`](../technical-debt/TD-0003-dependency-upgrades.md), not this release.
- **No KMS / at-rest key ownership.** If at-rest encryption is wanted it reuses the
  `0.48` `KeyProvider` seam (operator-supplied); absent that, sealing is deferred — never
  invented here.
- **No silent persistence.** Requesting persistence without a configured/writable
  storage directory is a **loud refusal** at startup, not a quiet fallback to RAM (R-3).

## Inherited Boundary (assumes 0.43–0.45 implemented; 0.46–0.50 not required)

- **0.43 `TieredValueStore<S>`** (`grid/elasticity.rs`) is the seam: the new durable
  backend becomes a real **cold tier** implementation, so hot RAM + durable cold compose
  without a rewrite. Do not redesign the hot/promote/demote logic.
- **0.43 `ReplicatedValueRecord` + durable value format version** (`grid/hardening.rs`):
  the persisted record format extends this; register the on-disk format in
  `docs/COMPAT.md` (R-4) and reuse the checksum metadata.
- **0.45 `RegionId` + `RegionPlacement`/active-active** (`grid/active_active.rs`): the
  per-region axis of the policy keys off these; persistence selection must respect home/
  active-active placement (a namespace persists in its configured regions only).
- **0.44 DST harness + scrubber/checksums**: the validation substrate (W7); recovery and
  torn-write faults are added there.
- **raft `sled-log-store`** (`hydracache-cluster-raft/src/log_store.rs`): the reference
  pattern for a feature-gated durable backend (open/append/fsync/format-version/
  fail-loud-on-unknown-future). Mirror it; keep the **KV-engine vs raft-engine split**
  (the value store gets its own engine, never shares the raft log engine).
- **`namespace` (0.38)**: the policy key. No new identifier type is introduced for
  namespaces; wildcard matching is layered on the existing string namespace.

## Dependency Graph

```
0.45 (region/active-active model) ── builds on 0.43 tiered store, validated by 0.44 DST
        │
        ▼
W1 DurableValueStore backend (on-disk cold tier, format+COMPAT, fail-loud)
        │
        ▼
W2 PersistencePolicy resolver (per-namespace wildcard/prefix, opt-in default RAM-only)
        │
        ├────────────► W3 per-region selection (persist only in important regions)
        ▼
W4 durability write path + scheduled snapshots (persistent namespaces only)
        │
        ▼
W5 full-restart recovery model (validation/load timeouts, recovery policy, epoch fencing)
        │
        ▼
W6 declarative config + operator surface (Hazelcast-style blocks, bounded metrics, loud refusal)
        │
        ▼
W7 DST validation: crash/restart, selective recovery, torn writes, stale-data fencing
```

W1 is the long pole: there is no durable value backend today; everything else composes
on top of it.

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests + exact
`cargo`/CI) / Risk & rollback.**

---

## W1. `DurableValueStore` on-disk backend (the missing value-plane durability)

**Goal.** A real disk-backed implementation of the `ReplicatedValueStore` seam so values
can outlive the process — the piece that does not exist today. Mirror the proven
`sled-log-store` pattern; keep it feature-gated so the default build stays RAM-only.

**Files.** `crates/hydracache/src/grid/durable_store.rs` (new:
`DurableValueStore` impl of `ReplicatedValueStore`, behind `feature = "durable-value-store"`),
extend `grid/hardening.rs` (persisted record encode/decode + format version), wire as a
`TieredValueStore` cold tier in `grid/elasticity.rs`.

**Steps.**
1. Define `DURABLE_VALUE_FORMAT_VERSION: u32` and a length-prefixed, checksummed on-disk
   encoding of `ReplicatedValueRecord` (key, value bytes, version, epoch, tombstone
   marker, expiry). Register it in `docs/COMPAT.md` with reader window + failure mode
   (R-4). **Refuse an unknown future format on open** — loud (R-3).
2. Implement `DurableValueStore::open(path)` over the reference engine (sled, mirroring
   `SledRaftLogStore::open`): `upsert`/`get`/`remove`/`iter`/`total_bytes`/`rejected_total`
   honoring the existing byte budget; persist tombstones (never resurrect deleted data —
   R-3). Pair reads with the `0.44` checksum/scrubber path.
3. Compose it as the **cold tier** of `TieredValueStore` (hot RAM unchanged); the engine
   is abstract behind the trait, so redb/RocksDB can be added later without touching
   callers (TD-0003). Keep the value engine **separate from the raft log engine**.

**DoD.** `crates/hydracache/tests/durable_value_store.rs`
- `reopen_recovers_records_and_tombstones` (integration): write, drop, reopen → identical.
- `unknown_future_format_refuses_to_open` (unit) — fail loud.
- `over_budget_upsert_is_rejected_and_counted` (unit).
- `corrupt_record_is_detected_not_served` (unit) — checksum.
- Run: `cargo test -p hydracache --features durable-value-store --locked durable_value_store` + `cargo xtask verify`.

**Risk & rollback.** New feature-gated module; default build and embedded API unchanged
(R-10). Engine choice is isolated behind the trait; revert removes the module.

---

## W2. `PersistencePolicy` resolver (per-namespace, Hazelcast-style)

**Goal.** Resolve, for any namespace, whether it persists and with what durability knobs —
matching Hazelcast's per-map / wildcard `data-persistence` model. Default and unconfigured
namespaces are **RAM-only** (opt-in, R-10).

**Files.** `crates/hydracache/src/grid/persistence_policy.rs` (new:
`PersistencePolicy`, `NamespacePersistenceRule`, `PersistenceMatcher`), referenced from
the builder and the write path.

**Steps.**
1. Model a rule set: an ordered map of pattern → `NamespacePersistenceRule { persist:
   bool, durability: Sync|AsyncBounded{max_lag}, snapshot_interval, eviction, backup_count,
   in_memory_format }`. Patterns support **exact**, **prefix wildcard** (`cache.*`,
   `wallet.*`) and a **`default`** fallback — the Hazelcast shape.
2. Deterministic precedence: **exact match > longest matching prefix > `default` > built-in
   RAM-only**. Resolution is pure and total. **Conflicting/overlapping rules with
   incompatible settings fail loud at construction** with the offending patterns (R-3),
   never "last wins silently".
3. Validate budgets: a rule cannot request persistence without W1's feature/engine
   available — construction returns a loud error (R-3), wired in W6.

**DoD.** `crates/hydracache/tests/persistence_policy.rs`
- `exact_beats_prefix_beats_default` (unit).
- `unconfigured_namespace_is_ram_only` (unit) — R-10 default.
- `conflicting_rules_fail_loud` (unit).
- `wildcard_matches_hazelcast_style_patterns` (unit) — `cache.*`, `wallet.*`.
- Run: `cargo test -p hydracache --locked persistence_policy`.

**Risk & rollback.** Pure logic, no I/O; trivially revertible. Keep matching O(rules) with
a precomputed prefix index to stay off the hot path.

---

## W3. Per-region persistence selection ("important regions")

**Goal.** Let a namespace persist **only in the regions that matter** and stay RAM-only
elsewhere — the per-region flexibility requested — keyed off the `0.45` `RegionId`/
placement model.

**Files.** extend `grid/persistence_policy.rs` (`persist_in_regions:
RegionSelector`), integrate with `grid/active_active.rs` placement.

**Steps.**
1. Add `RegionSelector { All, Only(BTreeSet<RegionId>), HomeRegionOnly }` to a rule. A
   value persists on a node iff the node's region is selected **and** the namespace rule
   persists. Local resolution uses the node's own `RegionId` (no cross-region calls).
2. Respect `0.45` placement: a namespace's persist-regions must be a subset of where the
   namespace is actually replicated; **a persist-region outside the placement fails loud**
   (R-3) rather than silently never persisting.
3. Region downgrade/upgrade is config-driven and observable; changing it is a documented
   migration (R-4) — flipping a region from persist→RAM drops its on-disk data on next
   compaction, flipping RAM→persist starts persisting new writes (no retroactive backfill
   without an explicit rebuild).

**DoD.** `crates/hydracache/tests/region_persistence.rs`
- `namespace_persists_only_in_selected_regions` (integration, 2-region sim).
- `persist_region_outside_placement_fails_loud` (unit).
- `home_region_only_selector_matches_placement` (unit).
- Run: `cargo test -p hydracache --locked region_persistence`.

**Risk & rollback.** Couples to `0.45`; if placement types differ at implementation time,
adapt the selector to the actual `RegionPlacement` API (documented in the PR).

---

## W4. Durability write path + scheduled snapshots (persistent namespaces only)

**Goal.** Actually persist writes for namespaces the policy marks durable, with a
configurable durability level and a scheduled snapshot cadence (Hazelcast
`hotrestart.scheduled.snapshot.interval`), while non-persistent namespaces keep **zero**
added overhead.

**Files.** `crates/hydracache/src/grid/durability.rs` (write-through / bounded
write-behind coordinator + snapshot scheduler), wiring in the value upsert/invalidate path.

**Steps.**
1. On upsert/tombstone for a persistent namespace, route the record to W1's
   `DurableValueStore` per the rule's durability: **`Sync`** (fsync before ack) or
   **`AsyncBounded{max_lag}`** (write-behind with a bounded queue that **fails loud /
   applies backpressure when the lag bound is exceeded**, never unbounded — R-3). For
   RAM-only namespaces, this path is a compile-time/branch no-op.
2. A snapshot scheduler periodically flushes a consistent point (interval from policy);
   record the snapshot's covering epoch/version watermark so recovery (W5) knows what it
   restored. Snapshot writes are checksummed and registered (R-4).
3. Surface durability lag + snapshot age as **bounded-label** gauges (R-6: label by
   namespace **only if** the namespace roster is bounded/registered — see W6; otherwise
   aggregate and put per-namespace detail in the diagnostics snapshot).

**DoD.** `crates/hydracache/tests/durability_write_path.rs`
- `sync_durability_acks_after_fsync` (integration).
- `async_bounded_backpressures_when_lag_exceeded` (integration) — no unbounded queue.
- `ram_only_namespace_has_no_durable_writes` (unit) — overhead guard.
- `scheduled_snapshot_records_epoch_watermark` (integration).
- Run: `cargo test -p hydracache --features durable-value-store --locked durability_write_path`.

**Risk & rollback.** fsync cost on the write path for `Sync` namespaces — that is the
explicit operator trade-off, observable via lag gauges; `AsyncBounded` is the default
durable level. Revert disables routing; data already on disk stays readable.

---

## W5. Full-cluster-restart recovery model (fail-loud, epoch-fenced)

**Goal.** On restart, reload persistent namespaces correctly and safely — the Hazelcast
"Hot Restart" semantics (validation timeout, data-load timeout, recovery policy,
auto-remove-stale-data) — **reconciled with HydraCache's epoch authority** so recovery
never resurrects superseded data.

**Files.** `crates/hydracache/src/grid/recovery.rs` (new: `RecoveryPolicy`,
`recover_namespaces`), invoked at node bootstrap before serving reads.

**Steps.**
1. `RecoveryPolicy { mode: FullRecoveryOnly | PartialAllowed, validation_timeout,
   data_load_timeout, auto_remove_stale_data }`. On open: validate format/checksums
   (R-4), then load records within `data_load_timeout`; exceeding a timeout is a **loud
   failure** in `FullRecoveryOnly`, a counted partial in `PartialAllowed` (R-3).
2. **Epoch fence (R-1):** every recovered record is admitted only if its `(epoch, version)`
   is not behind the authority recovered from the raft control plane; a record from a
   superseded epoch is **dropped as stale, counted, never served** (no resurrection of
   deleted/old data — R-3). `auto_remove_stale_data` controls whether stale on-disk data
   is compacted away or retained for inspection.
3. Non-persistent namespaces deliberately come back **empty** (cache-aside repopulation
   from the DB); document this explicitly. After recovery, the node rejoins via gossip,
   re-derives ownership (rendezvous), and reconciles via `0.46` repair if present.

**DoD.** `crates/hydracache/tests/persistence_recovery.rs`
- `persistent_namespace_survives_full_restart` (integration): write durable ns, restart
  all nodes (sim), values present; non-persistent ns empty.
- `stale_epoch_record_is_fenced_not_served` (integration) — R-1/R-3.
- `full_recovery_only_fails_loud_on_timeout` (unit).
- `corrupt_or_future_format_refuses_recovery` (unit).
- Run: `cargo test -p hydracache --features durable-value-store --locked persistence_recovery`.

**Risk & rollback.** Recovery correctness is the crux; W7 hammers it in the simulator.
Epoch reconciliation reuses existing tombstone/epoch-fence logic — do not invent a new
authority source (R-1).

---

## W6. Declarative config + operator surface

**Goal.** Express all of the above the way the Hazelcast YAML does — per-namespace and
per-region blocks — via config and the builder, with bounded metrics and loud refusals.

**Files.** `crates/hydracache/src/builder.rs` (`with_persistence_policy`,
`with_storage_dir`), `crates/hydracache/src/grid/persistence_config.rs` (serde
config: `PersistenceConfig` → `PersistencePolicy`), docs in
`docs/cluster/persistence.md` (new).

**Steps.**
1. Serde config mirroring the Hazelcast shape: a `persistence` block (storage dir,
   recovery policy, snapshot interval defaults) + a `namespaces` map of pattern → rule
   (persist, durability, regions, eviction, backup-count). Round-trips to
   `PersistencePolicy` with the W2/W3 validation.
2. **Loud refusal (R-3):** persistence requested with no/un-writable `storage_dir`, or
   without the `durable-value-store` feature, refuses to start with the exact reason —
   never a silent RAM fallback.
3. **Metric cardinality (R-6):** the set of persistable namespaces is treated as a
   **bounded, registered roster**; only rostered namespace ids may appear as metric
   labels. Unbounded/ad-hoc namespaces aggregate into an `other` bucket with per-namespace
   detail in the diagnostics snapshot. A drift test asserts no unbounded label escapes.
4. Document a worked example translating the uploaded Hazelcast config (e.g.
   `cache.jwt.pem` persistent, `cache.*` RAM-only) into HydraCache config.

**DoD.** `crates/hydracache/tests/persistence_config.rs`
- `config_roundtrips_to_policy` (unit).
- `persistence_without_storage_dir_refuses_to_start` (unit) — loud.
- `namespace_metric_labels_are_bounded` (unit, R-6 drift guard).
- `hazelcast_example_translates` (unit) — the documented mapping compiles to a valid policy.
- Run: `cargo test -p hydracache --locked persistence_config`.

**Risk & rollback.** Config surface is additive and opt-in; absent a persistence block,
behavior is byte-for-byte prior-release (R-10).

---

## W7. DST validation: crash/restart, selective recovery, stale-data fencing

**Goal.** Prove durability + selective recovery + epoch fencing under seeded faults in the
`0.44` simulator — not just in integration tests.

**Files.** extend `crates/hydracache-sim` (0.44) with persistence fault types and
invariants; a `persistence_recovery` sim test in the fast budget.

**Steps.**
1. Add fault types: **whole-cluster crash + restart**, **crash mid-snapshot**, **torn /
   partial durable write**, **storage corruption** (reuse the 0.44 fault-injecting
   storage), and **recovery with a stale on-disk epoch** vs a newer control-plane epoch.
2. Assert invariants across seeds: (a) every value written to a **persistent** namespace
   and acked under `Sync` is present after a full restart (no committed loss); (b)
   **non-persistent** namespaces are empty after restart (no accidental durability); (c)
   **no stale resurrection** — a fenced record is never served (R-1/R-3); (d) recovery is
   deterministic and replayable from the seed (R-5).
3. Wire a bounded seed matrix into the existing `dst_budget` fast gate.

**DoD.**
- `cargo test -p hydracache-sim --locked persistence_recovery` (fast budget).
- nightly: `cargo run -p hydracache-sim --bin vopr -- --seed <n> --steps 100000` exercising
  crash/restart cycles.
- Run: `cargo xtask verify` (fast budget) + nightly VOPR.

**Risk & rollback.** If the sim's storage seam cannot yet model torn writes, fall back to
integration-tier crash/restart (documented) and file the sim extension as follow-up debt.

---

## Deferred

- **Long-term default durable engine (redb / RocksDB) and a blob tier for large values** —
  strategic storage work tracked in `STORAGE_AND_DATA_PLATFORM_EVOLUTION.md` §1–§2 and
  `TD-0003`; this release ships one reference engine behind the trait.
- **At-rest encryption of persisted values** — reuses the `0.48` `KeyProvider` when that
  release lands; not invented here.
- **Cross-region durable backfill / rebuild** (retroactively persisting a namespace that
  was RAM-only) — explicit operator rebuild only; no automatic historical backfill.
- **SQL/vector persistence** — those are separate optional crates (storage doc §4–§5).
- **Full distributed transactions** — permanent hard non-goal (R-2).

## Fault Model and Test Tiering

Reuses the `0.41`–`0.45` shared model + the `0.44` deterministic simulator and scrubber.
**Adds** (all seeded, replayable — R-5): whole-cluster crash/restart, crash mid-snapshot,
torn/partial durable write, storage corruption on the value engine, and stale-epoch
recovery reconciliation. Tiers: fast (unit/integration + sim fast budget) on PR;
chaos/soak (VOPR crash-restart cycles) nightly.

## Release Gates

Focused:

```powershell
cargo test -p hydracache --features durable-value-store --locked durable_value_store
cargo test -p hydracache --locked persistence_policy
cargo test -p hydracache --locked region_persistence
cargo test -p hydracache --features durable-value-store --locked durability_write_path
cargo test -p hydracache --features durable-value-store --locked persistence_recovery
cargo test -p hydracache --locked persistence_config
cargo test -p hydracache-sim --locked persistence_recovery
```

Full:

```powershell
cargo xtask verify
cargo test --workspace --locked -- --ignored   # crash/restart soak
cargo run -p hydracache-sim --bin vopr -- --seed 51 --steps 100000
```

## Final Release Decision

`0.51.0` may claim **configurable per-namespace / per-region persistence** only if **all**
hold:

- W1: a feature-gated `DurableValueStore` persists records + tombstones, reopens
  identically, rejects unknown-future formats and corrupt records loud; format registered
  in `docs/COMPAT.md`.
- W2: the policy resolver is deterministic (exact > longest-prefix > default), defaults
  unconfigured namespaces to RAM-only, and fails loud on conflicting rules.
- W3: a namespace persists only in its selected regions; a persist-region outside
  placement fails loud.
- W4: `Sync` acks after fsync; `AsyncBounded` backpressures at its lag bound (never
  unbounded); RAM-only namespaces incur no durable writes; snapshots record an epoch
  watermark.
- W5: a persistent namespace survives a full-cluster restart; non-persistent namespaces
  come back empty; stale-epoch records are fenced and never served; `FullRecoveryOnly`
  fails loud on timeout/corruption.
- W6: declarative config round-trips, refuses to start when persistence is requested
  without storage, keeps namespace metric labels bounded (R-6 drift guard), and the
  uploaded Hazelcast example translates to a valid policy.
- W7: crash/restart, torn-write, corruption, and stale-epoch faults are modeled in the
  `0.44` simulator and the durability / emptiness / no-resurrection / determinism
  invariants hold across the seed matrix.
- The default and embedded builds remain byte-for-byte prior-release behavior (R-10), and
  docs keep the **"persistence makes a cache survive a restart; the SQL DB is still the
  source of truth; still not distributed transactions"** framing (R-2/R-9).

If any condition fails, the release ships **without** the corresponding claim, documents
what landed, and the rest moves to a follow-up.
