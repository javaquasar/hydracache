# DRAFT — Durable Value-Store & Snapshot Hardening (idea capture, not sequenced)

> **Status: DRAFT — version TBD, not yet sequenced.** This is an **idea-capture sketch** so the
> cross-project borrows from `COMPETITIVE_ANALYSIS_AND_EVOLUTION.md` §7 (blazingmq) and §4.5
> (arroyo) are not lost. It is **not** a committed release: no work-item DoD is binding yet, and
> it carries no gates until it is promoted to a numbered release (`planned`) with full
> Goal/Files/Steps/DoD per the house format. Registered in `releases.toml` as `status = "draft"`,
> `version = "TBD"` (allowed only for drafts).

> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> source ideas: [`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md) §7.3, §7.5, §4.5

## The idea (one paragraph)

`0.51` shipped the `DurableValueStore` (on-disk per-namespace/region persistence). Two
battle-tested references show how to make it and the snapshot path **production-grade**:
blazingmq's **file-store protocol** (`mqbs_filestore*`, `mqbs_fileset`, `mqbs_filestoreprintutil`
— versioned on-disk format, data-file iterators, multi-file rotation, an inspect/print utility,
8+ years in prod) and arroyo's **barrier-aligned consistent checkpoint** coordination
(`arroyo-controller` + `checkpoints.rs` — a cluster-wide consistent point, not N independent
dumps). Add blazingmq's **poison-pill** notion as a **poison-load circuit-breaker** for keys whose
single-flight loader keeps failing. Theme: *make durability and cluster-wide snapshots provably
recoverable and operable*, without changing the consistency contract (R-1) and without becoming a
database (R-9).

## Why capture it now

These are the only substantial **still-unrealized** borrows from the recent cross-project review
(transports already became `0.54`; election stability folded into `0.46`/`0.53`). They cluster
around one coherent theme (durability/recovery operability) and build directly on `0.51`. Capturing
them as a draft keeps the thread alive without forcing premature sequencing.

## Candidate work items (sketch — to be expanded when promoted)

> Each bullet names its **proof obligation** up front, because (per the `0.53` C-contracts
> discipline) "recoverable" and "consistent" must be *demonstrated*, not asserted.

- **D1 — Versioned file-store format + iterators + inspect util** (blazingmq §7.3).
  Give `DurableValueStore` an explicit, versioned on-disk **format protocol**, a data-file
  iterator, and a `print/inspect` operability tool. *Proof:* `reopen_recovers_exactly`,
  `unknown_future_format_refuses_to_open` (loud, R-4), `corrupt_record_detected_not_served`
  (checksum), `inspect_tool_dumps_records_for_ops`. Register the format in `docs/COMPAT.md`.

- **D2 — File-set rotation + compaction** (blazingmq §7.3).
  Multi-file rotation so the store does not grow unbounded; compaction reclaims tombstoned/expired
  space. *Proof:* `rotation_bounds_file_count`, `compaction_reclaims_tombstoned_space`,
  `recovery_spans_rotated_file_set` (recover across rotation boundaries), all seed-deterministic.

- **D3 — Barrier-aligned cluster-wide consistent snapshot** (arroyo §4.5).
  A coordinated consistent point (controller/barrier) when a *cluster-wide* snapshot is wanted
  (vs per-namespace scheduled snapshot), and a rescale-with-checkpoint flow
  (stop → checkpoint → redistribute → resume) for `0.43` reshard. *Proof:*
  `cluster_snapshot_is_a_consistent_cut` (no torn cross-node state),
  `rescale_with_checkpoint_loses_no_committed_writes`, validated in the `0.44` DST. Authority stays
  epoch/version (R-1) — this adds *coordination*, not a new consistency level.

- **D4 — Poison-load circuit-breaker** (blazingmq §7.5 poison-pill analog).
  A key whose single-flight loader repeatedly fails is **quarantined + backed off + counted**,
  not hammered against the DB. *Proof:* `repeated_load_failure_trips_breaker_and_counts`,
  `breaker_half_opens_and_recovers`, `breaker_never_serves_stale_as_fresh` (R-3). Ties the `0.37`
  loader/single-flight work.

## Non-Goals (carried from the parent rules)

- **Not a database / not a new source of truth** (R-9). Persistence makes a *cache* survive a
  restart; the SQL DB stays authoritative.
- **No new consistency level** (R-1). D3 adds coordination, not a tunable mode.
- **No silent recovery.** Any unrecoverable/corrupt/over-budget condition is **loud + counted**
  (R-3), never a quiet partial.
- **Not the message-queue product surface** (the blazingmq borrows are engineering patterns only).

## Dependencies (when sequenced)

Builds on `0.51` (DurableValueStore + per-namespace/region policy) and `0.43` (tiered store +
online reshard); D3 is validated by `0.44` DST. Independent of `0.52`/`0.53`/`0.54`.

## Promotion checklist (draft → planned)

1. Pick a number (next free, currently `0.55`) and set `status = "planned"`, real `version` in
   `releases.toml`; add the DAG edge + table row in `INDEX.md`.
2. Expand D1–D4 into full **Goal / Files / Steps / DoD(tests) / Risk** items.
3. Add the COMPAT entries (on-disk format version, any new wire/durable artifact).
4. Confirm no overlap with `0.51` scope (this is *hardening*, not re-doing persistence).
