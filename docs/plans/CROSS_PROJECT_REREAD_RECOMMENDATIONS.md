# Cross-Project Re-Analysis Recommendations For HydraCache

Date: 2026-07-07.

Purpose: given where HydraCache is now (the `0.59`→`0.62` arc — networked daemon
grid, `ConfChange`, cluster-transport TLS, and the `0.62` correctness test
hardening plan — heading toward `1.0`), pick which reference projects under
`C:\Workspace\prj\jq\cashe` are worth **re-reading now** to harvest ideas, and
which to skip. This complements
[`CROSS_PROJECT_IDEA_BACKLOG.md`](./CROSS_PROJECT_IDEA_BACKLOG.md) (the standing
idea map) by prioritizing against the *current* frontier rather than restating
the full catalog.

The distinction used below:

- **Underexplored** = not in the backlog Source Map, high value now.
- **Second pass** = already in the backlog, but the project has grown into that
  project's exact territory and should be re-read with fresh eyes.
- **Skip for now** = low ROI at the current stage, with the reason.

All items are read for **design ideas and shapes**, re-implemented clean-room in
idiomatic Rust — never copied (license discipline, same as the Redis reread).

## Top recommendations — underexplored projects (not yet in the Source Map)

### 1. TiKV — the #1 target right now

Local source: [tikv](../../../tikv). Already used as the blueprint for the `0.62`
test-hardening plan (`components/test_raftstore/src/transport_simulate.rs`,
`tests/failpoints/`), but there is far more, all directly on the current frontier:

- `components/raftstore` — production raft layout: region split/merge, snapshot
  apply, lease-read, **hibernate** (idle regions stop running heartbeats — directly
  relevant to a future idle-cluster / scale-down-to-quiet story).
- `components/batch-system` — their actor-like polling of many raft groups; maps to
  backlog **#4 (cluster runtime lifecycle)** but with real production load rather
  than a toy.
- `tests/integrations/raftstore/` — the standing catalog of correctness tests
  (`test_conf_change`, `test_transfer_leader`, `test_stale_peer`, `test_prevote`,
  `test_tombstone`) that the `0.62` plan mirrors.

Why now: HydraCache's `0.59`-`0.62` work is essentially building a small TiKV
membership plane. TiKV is the map of where single-raft-group simplifications stop
scaling (multi-raft, region split), and the reference implementation of the
operability details (lease read, hibernate) HydraCache will eventually want.
**Action: add to the backlog Source Map + write a dedicated `TIKV_HYDRACACHE_REREAD.md`.**

### 2. Pingora — profile-matched, entirely outside the backlog

Local source: [pingora](../../../pingora). Cloudflare's production proxy/cache
framework in Rust; the only production-grade Rust cache-proxy in the root. Has a
dedicated cache crate:

- `pingora-cache` — cache model: **cache lock** (cross-process single-flight),
  purge, a storage trait, eviction. Direct idea-competitor to HydraCache's
  `hydracache-server` near-cache and single-flight loader.
- `pingora-core` — **zero-downtime graceful reload/upgrade**, connection pooling.
  HydraCache's `0.48` graceful upgrade + `0.56` rolling upgrade do this; Pingora
  runs it at millions of RPS — worth diffing their seam against ours.

Why now: it is the single most operability-relevant source in the root for the
`hydracache-server` transport, graceful lifecycle, and pooling. Higher ROI for the
daemon than academic sources. **Action: add to the Source Map + reread.**

### 3. TigerBeetle — a deeper second pass on DST discipline

Local source: [tigerbeetle](../../../tigerbeetle). The VOPR/DST idea already
seeded `0.44`/`0.58`, but it was read shallowly. Territory for the `1.0`
correctness push:

- `src/testing/` (`cluster`, `vortex`) — how they build a deterministic
  whole-cluster test over simulated time/network/disk.
- **TigerStyle** discipline: static allocation, "everything has a limit,"
  assertions-as-contracts. This is RULES **R-3/R-6** raised to a methodology.
  Given HydraCache already pays for DST, it should extract the maximum: the
  "no dynamic allocation after startup" and state-machine determinism thinking.

Why now: the `1.0` claim rests on correctness evidence; TigerBeetle is the
canonical bar for "how paranoid a deterministic test suite can be."

### 4. qdrant — the second real-process test blueprint + sharding

Local source: [qdrant](../../../qdrant). Used as the `0.62` W3 blueprint
(`tests/consensus_tests`, `PeerProcess.kill` real SIGKILL) but not in the map:

- `tests/consensus_tests/` — the Python real-process harness; the reference for the
  `0.62` `DaemonCluster` (real child processes, kill/restart, rejoin).
- `lib/collection/` — their sharding / replica-set model over raft; a parallel for
  HydraCache's future ownership-routing (backlog **#6**).

## Second pass — already in the backlog, but the project grew into their territory

### 5. ScyllaDB — reclassify from "reference" to "active model"

Local source: [scylladb](../../../scylladb). In the backlog as "gossip + raft
split." But the entire `0.59`-`0.62` arc *is* an implementation of
**topology-over-raft** (a Raft group for membership/schema, separate from the data
path) — exactly Scylla's design. Re-read `docs/dev` on their raft topology and the
reader-concurrency semaphore **now that a working `ConfChange` exists**, to check
HydraCache isn't reinventing an already-debugged pattern (e.g. their late-node join
bootstrap ≈ the `0.61` W1 join path).

### 6. Arroyo — second pass on checkpoints / controller

Local source: [arroyo](../../../arroyo). In the backlog as "controller/worker
lifecycle." Relevant twice now: their barrier-aligned checkpoint (HydraCache
`0.55`) and controller-reconcile (HydraCache `0.56` operator). Worth re-reading the
phase model if checkpoint-rescale or operator robustness gets hardened toward `1.0`.

## Skip for now (with reasons)

- **tantivy** ([tantivy](../../../tantivy)) — search index; mmap/segment management
  could in theory inform the durable store, but HydraCache already chose sled and
  closed durability (`0.51`/`0.55`). Low ROI.
- **blazingmq** ([blazingmq](../../../blazingmq)) — already cited pointwise
  (FSM-as-table, poison-pill). A full pass on message-queue semantics drifts toward
  an event log, which is a **non-goal (R-9)**. Keep it as a pointwise source.
- **noria / readyset** ([noria](../../../noria), [readyset](../../../readyset)) —
  deliberately anti-scope (transparent materialized-view serving). Useful only as
  guardrails; no re-read needed.

## Priority

If one re-read: **TiKV** — the direct frontier, the production-raft map.

If two: add **Pingora** — profile-matched production cache-proxy, server
operability.

Both are new to the Source Map and both point into where the project is heading
toward `1.0`, not into already-closed topics.

## How to record a re-read

Follow the established convention (as done for Redis,
[`../../../redis/REDIS_HYDRACACHE_REREAD.md`](../../../redis/REDIS_HYDRACACHE_REREAD.md)):
create `<PROJECT>_HYDRACACHE_REREAD.md` in that project's root with a `Steal now` /
`Avoid` split, verified `file:line` references, and explicit cross-links back to the
HydraCache backlog items and release plans it feeds. Then add the project to the
Source Map table in `CROSS_PROJECT_IDEA_BACKLOG.md`.
