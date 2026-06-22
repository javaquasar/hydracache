# HydraCache 0.45.0 Cluster Resilience & Coordination Plan

`0.45.0` deepens the **cluster itself**. Releases `0.42`–`0.44` built the shape of a
production grid — durable multi-node Raft (`0.42`), zone/region-aware placement and
online resharding (`0.43`), and active-active multi-region with CRDTs and a WAN
transport (`0.44`). What those releases did *not* yet adopt are several
battle-proven cluster primitives from Hazelcast and ScyllaDB/Cassandra that make a
cluster *resilient* and *coordinated* under the messy middle ground between "healthy"
and "node lost": brief outages, flapping liveness, lost invalidations, and the need
for a single-key linearizable decision. `0.45` adds exactly those primitives —
tunable consistency levels, hinted handoff, incremental Merkle repair, an adaptive
failure detector, single-key conditional writes plus a fenced lock, and a durable
replayable invalidation stream — **without** crossing into distributed transactions.

The release keeps the same authority/dissemination resolution rule from
`0.41`–`0.44`:

> **Authority** (who owns a key, which topology is valid, which version is newer)
> is the ScyllaDB model: Raft + monotonic epoch. **Dissemination** (how staleness
> is detected and propagated) is the Hazelcast model: sequence/UUID stamps. When
> the two disagree, the epoch (authority) wins; the stamp only triggers a
> conservative refresh/invalidate.

Readiness is described in prose and asserted as boolean release gates. There is no
numeric self-score. `0.45` does **not** weaken any `0.44` guarantee: every new
mechanism is opt-in or strictly strengthens an existing default, and embedded /
single-region deployments keep `0.44` behavior byte-for-byte.

## Release Theme

Make the existing cluster *resilient and coordinated* by adopting the proven
Hazelcast/ScyllaDB primitives the roadmap skipped — so brief outages self-heal
cheaply, liveness is judged adaptively, lost invalidations are replayable, and a
single key can be decided linearizably — all while distributed transactions stay a
hard non-goal.

Each work item names its source primitive (Hazelcast and/or ScyllaDB/Cassandra) and
the `0.41`–`0.44` artifact it strengthens. The work is six items (W1–W6) plus
explicit deferrals.

## Non-Goals

- **No full distributed transactions.** Serializable cross-node/cross-region
  multi-key atomic commit remains a hard non-goal. W5's conditional writes are
  **single-key linearizable** (ScyllaDB LWT / Hazelcast CP scope), explicitly **not**
  multi-key transactions; the `0.43` W5 atomic-invalidation slice stays the ceiling
  for multi-key. The prominent "still not distributed transactions" warning stays.
- **No remote code execution / compute-near-data.** Hazelcast `EntryProcessor`-style
  server-side execution and any remote SQL/expression/closure evaluation are out of
  scope, as in every prior release. W5 conditional writes compare/set *values the
  client supplies*, never run client code on the server.
- **No new consistency claim beyond what each level delivers.** Tunable levels (W1)
  expose existing trade-offs explicitly; they never promise a guarantee the
  replication factor and quorum math cannot back. Cross-region levels inherit the
  `0.44` bounded-staleness reality.
- **No silent behavior change.** Hinted handoff, repair, the failure detector, and
  the replayable stream are bounded, observable, and fail-closed; none silently
  drops, resurrects, or over-invalidates without a counter and a documented policy.
- **No KMS / secret-store, no ecosystem/external-consumer surface, no causal+
  cross-region session guarantees, no auto home-placement, no provider-specific
  autoscaler controllers.** These stay deferred (see Deferred To 0.46+).

## Inherited Boundary From 0.44

`0.45` only strengthens `0.41`–`0.44`; it must not redesign them.

- **`ReplicationConfig` read/write quorum (`0.41` A3) + grid RYOW (`0.42` W5) +
  LOCAL/EACH quorum intuition (`0.44`)** are generalized into **per-operation tunable
  consistency levels** in W1 — the config becomes a default, the level becomes a
  per-call choice.
- **Anti-entropy / repair-gated GC (`0.41` A5, `0.42` W3/W6) and the WAN digest
  exchange (`0.44` W3)** are the baseline that **hinted handoff (W2)** complements
  (cheap short-outage path) and **incremental Merkle repair (W3)** formalizes
  (repair sessions, foreground read-repair).
- **Gossip `suspect` + the A1 epoch fence (`0.41`)** are fed by the **phi-accrual
  failure detector (W4)** instead of a fixed timeout — the fence is unchanged;
  only the *suspicion signal* gets smarter.
- **Single-partition Raft authority (`0.42` W1) + the `0.43` W5 atomic-invalidation
  slice** are the substrate for **single-key conditional writes + a fenced lock
  (W5)**.
- **Near-cache watermark reconciliation (`0.41` B1) + invalidation bus
  (`CacheInvalidationFrame`)** are upgraded by the **durable replayable invalidation
  stream (W6)** so a lagging subscriber replays exactly instead of conservatively
  flushing.

## Dependency Graph

```
0.41 A3 + 0.42 W5 quorum ────────────► W1 tunable consistency levels (Scylla CL)
0.41 A5 + 0.42 W3/W6 anti-entropy ───► W2 hinted handoff (Cassandra/Scylla/Hazelcast)
W2 + 0.44 W3 digest exchange ────────► W3 incremental Merkle repair (Scylla/Cassandra)
0.41 A1 epoch fence (gossip suspect) ► W4 phi-accrual failure detector (Hazelcast/Cassandra)
0.42 W1 single-partition Raft ───────► W5 single-key conditional writes + fenced lock (LWT / CP)
0.41 B1 near-cache + invalidation bus► W6 durable replayable invalidation stream (Hazelcast ringbuffer)
W4 (accurate liveness) ──────────────► W2, W3   (repair/handoff need trustworthy down/up signals)
```

W4 is a quiet long pole: hinted handoff (W2) and repair (W3) both make wrong, costly
decisions if liveness is judged by a flapping fixed-timeout signal, so the adaptive
detector underpins them.

---

## W1. Tunable Per-Operation Consistency Levels (ScyllaDB CL)

**Problem / motivation.** Consistency is currently a *deployment-wide* setting:
`ReplicationConfig.read_quorum`/`write_quorum` (`0.41` A3) plus the `0.42` W5 grid
read-your-writes contract. ScyllaDB/Cassandra learned that callers need *per-operation*
control — a cheap `ONE` read for a dashboard, a `QUORUM` write for money, a
`LOCAL_QUORUM` in active-active to stay in-region, an `EACH_QUORUM` when a write must
be durable in every region. HydraCache has the quorum machinery but not the per-call
knob.

**Design / contract.** Add a `ConsistencyLevel` chosen per `get`/`put`/`invalidate`,
mapping onto the existing quorum math: `One`, `LocalQuorum` (quorum within the
caller's region, ties `0.43`/`0.44`), `Quorum` (grid-wide majority), `EachQuorum`
(quorum in every region), `All`. The level is validated against the live
`EffectiveReplicationMap` and `ReplicationConfig`: a level that cannot be satisfied
(e.g., `EachQuorum` with a region down) **fails the operation explicitly** rather
than silently degrading. The deployment `ReplicationConfig` becomes the *default*
level; per-call overrides it. Strong read-your-writes (`0.42` W5) is the combination
`write>=Quorum` + `read>=Quorum` with overlap; weaker levels are documented as
weaker. Cross-region levels inherit the `0.44` bounded-staleness reality (no level
makes active-active linearizable across regions).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/consistency_level.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsistencyLevel { One, LocalQuorum, Quorum, EachQuorum, All }

pub struct ReadOptions { pub level: ConsistencyLevel /* default from ReplicationConfig */ }
pub struct WriteOptions { pub level: ConsistencyLevel }

impl ConsistencyLevel {
    /// Required acks for this level given the effective replicas (per region for EACH).
    pub fn required_acks(&self, map: &EffectiveReplicationMap, local: RegionId) -> AckRequirement;
    /// True if satisfiable against the current live map; else the op fails loud.
    pub fn is_satisfiable(&self, map: &EffectiveReplicationMap) -> bool;
}
```

**Step-by-step implementation.**

1. Add `ConsistencyLevel` + `ReadOptions`/`WriteOptions`; default from
   `ReplicationConfig`.
2. Compute `required_acks` per level (per-region counts for `LocalQuorum`/`EachQuorum`)
   from the `EffectiveReplicationMap`; reuse the `0.42` W5 quorum read/write paths.
3. Fail an unsatisfiable level loud (`ConsistencyUnsatisfiable { level, reason }`),
   never silently downgrade.
4. Surface in readiness which strong combinations hold (`write_quorum+read_quorum`
   overlap), reusing the `0.42` W5 reporting.
5. Export `op_consistency_level_total` and `consistency_unsatisfiable_total` (bounded
   labels: the small level enum).

**Testing.** `crates/hydracache/tests/consistency_levels.rs`

- `level_required_acks_matches_replica_math` (unit): `One`/`Quorum`/`All` over a
  known map.
- `local_quorum_counts_only_local_region` (unit): ties `0.44` regions.
- `each_quorum_requires_every_region` (integration): one region short → operation
  fails loud, not a silent partial.
- `quorum_read_after_quorum_write_is_read_your_writes` (**property**): ties `0.42` W5.
- `unsatisfiable_level_fails_not_degrades` (unit).
- Run: `cargo test -p hydracache --locked consistency_levels`.

**Pros.** Callers pick the latency/durability trade per operation, exactly as Scylla
users expect; the contract is explicit and fail-loud.

**Risks.** More levels = more ways to misconfigure. Mitigation: the deployment default
stays, levels are a small enum, and unsatisfiable levels fail with a clear reason.

---

## W2. Hinted Handoff (Cassandra / ScyllaDB / Hazelcast)

**Problem / motivation.** When a replica is briefly down (restart, GC pause, transient
partition), today's only recovery is anti-entropy/repair (`0.41` A5, `0.42` W3/W6) —
correct but heavy, and it leaves a window where the key is under-replicated. Cassandra/
Scylla solve the *short* outage cheaply with **hinted handoff**: the coordinator stores
the missed write as a "hint" and replays it when the replica returns, restoring RF fast
without a full repair.

**Design / contract.** When a write cannot reach a replica that the consistency level
(W1) does not strictly require, the coordinator records a bounded, durable **hint**
`(target, key, version, epoch, sealed-value)` in a per-target hint store. On the
target's return (W4 says it's up), hints replay in order, gated by the A5 version rule
(a hint never resurrects a key tombstoned at a higher epoch, never downgrades a newer
version). Hints are **bounded** (max count/bytes and a max age = "hint window", like
Cassandra `max_hint_window`); past the window, the hint is dropped and the key is left
to repair (W3) — dropped hints are **counted**, never silent, and trigger a repair
mark. Hinted handoff is on by default for brief outages but capped so it never becomes
unbounded backlog.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/hinted_handoff.rs
pub struct Hint {
    pub target: ClusterNodeId,
    pub key: CacheKey,
    pub version: ValueVersion, // A5
    pub epoch: ClusterEpoch,
    pub sealed: SealedBytes,   // 0.41 ReplicationKeyProvider when configured
}

pub struct HintBudget { pub max_hints: u64, pub max_bytes: u64, pub max_age: Duration }

pub enum HintOutcome { Stored, Replayed, DroppedOverBudget, DroppedExpired /* -> repair mark */ }

pub trait HintStore: Send + Sync {
    fn store(&self, hint: &Hint) -> Result<HintOutcome, HintError>;
    fn drain_for(&self, target: ClusterNodeId) -> Result<Vec<Hint>, HintError>;
}
```

**Step-by-step implementation.**

1. Add `Hint` + `HintBudget` + `HintStore` (in-memory default; durable behind the
   `0.42` engine feature for restart-survivable hints).
2. On a write that misses a non-required replica, store a hint (sealed if a key
   provider is set); enforce the budget — over-budget/expired → drop + count +
   mark the key for repair (W3).
3. On target return (W4 `Up` transition), `drain_for` and replay in order, each gated
   by the A5 `(version, epoch)` rule (no resurrection, no downgrade).
4. Never let hints substitute for a *required* ack: if the consistency level (W1)
   required that replica, the write fails — hints only cover the non-required gap.
5. Export `hints_stored_total`, `hints_replayed_total`, `hints_dropped_total`
   (labels: reason — over_budget/expired), `hint_store_bytes` (gauge).

**Testing.** `crates/hydracache/tests/hinted_handoff.rs`

- `brief_outage_hint_replays_on_return` (integration): replica down then up within the
  window → key restored to RF without a full repair.
- `hint_never_resurrects_tombstone` (**property**): ties A5.
- `over_budget_hint_dropped_and_marked_for_repair` (integration): not silent.
- `expired_hint_dropped_after_window` (unit).
- `required_replica_miss_still_fails_the_write` (unit): ties W1 — hints don't fake an
  ack.
- `replica_recovers_after_hint_window_falls_back_to_repair` (**chaos**, `#[ignore]`):
  outage longer than the window → W3 repair converges.
- Run: `cargo test -p hydracache --locked hinted_handoff` and chaos with `-- --ignored`.

**Pros.** Short outages restore RF cheaply and fast; the heavy repair path is reserved
for real divergence; bounded + observable so it can't become a memory bomb.

**Risks.** Hints add write-path bookkeeping and storage. Mitigation: strict budget,
fail-closed to repair past the window, and the budget is a gauge.

---

## W3. Incremental Merkle-Tree Repair (ScyllaDB / Cassandra)

**Problem / motivation.** `0.41`/`0.42` anti-entropy and the `0.44` W3 WAN digest
exchange detect and ship diffs, but there is no *formal repair* with bounded,
resumable sessions and a foreground read-repair on the hot path. Cassandra/Scylla use
**Merkle trees** to compare replicas efficiently and **incremental repair** to avoid
re-checking already-repaired data. HydraCache needs that to keep divergence bounded at
scale without re-scanning everything.

**Design / contract.** Build a `RepairSession` over per-partition **Merkle trees**:
replicas exchange tree hashes top-down, descend only into mismatching subtrees, and
exchange only the differing ranges (generalizing the `0.44` W3 digest). Two modes:
(a) **foreground read-repair** — a `QUORUM`+ read (W1) that sees divergent replicas
repairs them inline before returning the freshest `(version, epoch)`; (b) **scheduled
incremental repair** — background sessions that track a "repaired watermark" so already-
repaired ranges are skipped next time. Repair is rate-limited (the `0.42` W3 adaptive
window), respects the A5 tombstone/version rules, and gates tombstone GC on repair
confirmation (unchanged A5 contract). Repair sessions are resumable across coordinator
failover (state lives in the partition, not in one process).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/repair.rs
pub struct MerkleTree { /* per-partition, leaf = hash(key, version, epoch) */ }

pub struct RepairSession {
    pub partition: PartitionId,
    pub peers: SmallVec<[ClusterNodeId; 4]>,
    pub repaired_watermark: RepairToken, // incremental: skip already-repaired ranges
}

pub enum RepairKind { ForegroundReadRepair, ScheduledIncremental }

impl MerkleTree {
    pub fn diff(&self, other: &MerkleTree) -> Vec<KeyRange>; // descend only mismatches
}
```

**Step-by-step implementation.**

1. Add per-partition `MerkleTree` (leaf hashes over `(key, version, epoch)`); build
   incrementally so it is cheap to maintain.
2. Implement `RepairSession::run` using `diff` to exchange only mismatching ranges
   (supersedes the `0.44` W3 ad-hoc digest with a tree).
3. Foreground read-repair: on a `QUORUM`+ read (W1) with divergent replicas, repair
   inline, then serve the max `(version, epoch)`.
4. Scheduled incremental repair: track a `repaired_watermark`; skip repaired ranges;
   rate-limit via the `0.42` W3 window; resumable across failover.
5. Keep A5: never resurrect a tombstone; GC stays gated on repair confirmation.
6. Export `repair_sessions_total`, `repair_ranges_exchanged_total`,
   `read_repair_total`, `repair_progress_ratio` (bounded labels).

**Testing.** `crates/hydracache/tests/merkle_repair.rs`

- `merkle_diff_descends_only_mismatches` (unit): identical trees → no ranges; one
  divergent key → one range.
- `foreground_read_repair_fixes_divergence_inline` (integration): divergent replicas on
  a `Quorum` read → repaired + freshest served.
- `incremental_repair_skips_repaired_ranges` (integration): second session does far
  less work.
- `repair_preserves_tombstone_invariant` (**property**): ties A5.
- `repair_session_resumes_after_coordinator_crash` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked merkle_repair` and chaos with `-- --ignored`.

**Pros.** Bounded, resumable, efficient convergence at scale; read-repair fixes
divergence on the path callers already pay for; incremental tracking avoids
re-scanning.

**Risks.** Merkle maintenance adds memory/CPU. Mitigation: incremental tree updates,
rate-limited sessions, and progress as a gauge so operators see cost.

---

## W4. Phi-Accrual Adaptive Failure Detector (Hazelcast / Cassandra / Akka)

**Problem / motivation.** Liveness today is gossip `suspect` (`0.41` A1) on what is
effectively a fixed timeout. Fixed timeouts are wrong in both directions: too short →
false suspicions (flap → wasted hint/repair/promotion churn), too long → slow real
detection. Hazelcast and Cassandra use a **phi-accrual** detector that adapts to the
observed heartbeat distribution and outputs a *suspicion level* (phi), not a boolean.

**Design / contract.** Add a `PhiAccrualDetector` per peer that tracks recent
heartbeat inter-arrival times and computes `phi`; a tunable `phi_threshold` converts it
to `suspect`/`up`. The detector's output **replaces the fixed-timeout suspect signal
feeding the `0.41` A1 fence and gossip** — the fence itself is unchanged (Raft
`CommitTopology` still decides authority; the detector only proposes liveness). The
detector underpins W2 (replay hints only on a trustworthy `Up`) and W3 (don't repair
against a node about to be marked down). Thresholds and the heartbeat window are
config; the current phi per peer is observable.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/failure_detector.rs
pub struct PhiAccrualDetector {
    window: SlidingWindow<Duration>, // recent heartbeat intervals
    phi_threshold: f64,              // e.g. 8.0; higher = more conservative
}

impl PhiAccrualDetector {
    pub fn heartbeat(&mut self, now: Instant);
    pub fn phi(&self, now: Instant) -> f64;          // suspicion level
    pub fn is_available(&self, now: Instant) -> bool { self.phi(now) < self.phi_threshold }
}

pub enum Liveness { Up, Suspect { phi: f64 } } // feeds gossip suspect + A1 fence input
```

**Step-by-step implementation.**

1. Add `PhiAccrualDetector` fed by the existing gossip/transport heartbeats; maintain a
   sliding window of inter-arrival times.
2. Replace the fixed-timeout suspect input to gossip / the A1 fence with the detector's
   `is_available` (authority decision path unchanged — Raft still commits topology).
3. Expose `peer_phi` (gauge per peer; node id bounded by cluster size) and
   `false_suspect_total` (measured against later `Up` without a real outage).
4. Wire W2/W3 to consult the detector before replaying hints / starting repair against a
   peer.
5. Keep the fence semantics intact: suspect never changes ownership before
   `CommitTopology` (A1).

**Testing.** `crates/hydracache/tests/failure_detector.rs`

- `steady_heartbeats_keep_phi_low` (unit).
- `missed_heartbeats_raise_phi_past_threshold` (unit).
- `adapts_to_slower_but_regular_links` (**property**): a consistently slow link does not
  trip suspicion the way a fixed timeout would.
- `flapping_does_not_change_ownership_before_commit_topology` (integration): ties A1 —
  suspicion never bypasses the fence.
- `detector_gates_hint_replay_and_repair` (integration): ties W2/W3.
- Run: `cargo test -p hydracache --locked failure_detector`.

**Pros.** Fewer false-positive churns and faster true detection; repair/handoff act on
a trustworthy signal; the fence stays the authority.

**Risks.** Phi tuning is a knob to get wrong. Mitigation: sane default threshold,
per-peer phi is observable, and false-suspect rate is a metric.

---

## W5. Single-Key Conditional Writes + Fenced Lock (ScyllaDB LWT / Hazelcast CP)

**Problem / motivation.** Coordination sometimes needs a *linearizable single-key
decision*: compare-and-set, put-if-absent, or a lock. ScyllaDB offers this as LWT
(single-key Paxos); Hazelcast offers a CP `FencedLock`. HydraCache has the Raft
authority per partition (`0.42` W1) but exposes no conditional write and no lock — so
callers can't safely coordinate (e.g., leader election for a cache-warming job, dedup of
an expensive load). This is **not** a distributed transaction: it is one key, decided by
that key's existing Raft authority.

**Design / contract.** Add `compare_and_set` / `put_if_absent` on a single key,
executed through the key's partition-home Raft so the decision is linearizable for that
key. Build a `FencedLock` on top: acquiring returns a **monotonic fence token**; a
holder that pauses and resumes presents a stale token and is rejected (the Hazelcast
fenced-lock guarantee — protects against the classic "lock holder GC-paused while
another acquires" hazard). Strictly single-key: any attempt to span keys/partitions is
rejected loud and pointed at the `0.43` W5 atomic-invalidation slice / documented as a
non-goal. Conditional writes honor A5 versions and the W1 consistency level
(`Quorum`/`All` only — a conditional write at `One` is refused, since it cannot be
linearizable).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/conditional.rs
pub enum CasResult { Applied { new_version: ValueVersion }, Mismatch { current: Option<Bytes> } }

impl HydraCache {
    pub async fn compare_and_set(&self, key: &CacheKey, expected: Option<&Bytes>, new: Bytes)
        -> Result<CasResult, ConditionalError>; // single-key, via partition Raft
    pub async fn put_if_absent(&self, key: &CacheKey, value: Bytes) -> Result<CasResult, ConditionalError>;
}

// crates/hydracache/src/cluster/fenced_lock.rs
pub struct FenceToken(u64); // monotonic; stale token is rejected

pub struct FencedLock { key: CacheKey }
impl FencedLock {
    pub async fn try_acquire(&self) -> Result<Option<FenceToken>, LockError>;
    pub async fn release(&self, token: FenceToken) -> Result<(), LockError>;
}
```

**Step-by-step implementation.**

1. Add `compare_and_set`/`put_if_absent` routed through the key's partition-home Raft
   (`0.42` W1); require consistency level `Quorum`+ (refuse `One` loud).
2. Reject any multi-key/cross-partition conditional attempt loud, pointing at the
   `0.43` W5 slice — keep distributed transactions a non-goal.
3. Build `FencedLock` on `compare_and_set` with a monotonic `FenceToken`; reject a
   stale token on release/use (fencing guarantee).
4. Honor A5: a conditional write produces a new `(version, epoch)`; never bypass the
   tombstone rule.
5. Export `cas_applied_total`, `cas_mismatch_total`, `lock_acquired_total`,
   `lock_stale_token_rejected_total` (bounded labels).

**Testing.** `crates/hydracache/tests/conditional_writes.rs`

- `compare_and_set_applies_only_on_match` (integration).
- `put_if_absent_is_linearizable_under_contention` (**property**): concurrent
  contenders → exactly one wins.
- `cas_at_consistency_one_is_refused` (unit): linearizability can't be faked.
- `multi_key_conditional_is_rejected_loud` (unit): non-goal guard.
- `fenced_lock_rejects_stale_token` (integration): paused holder's old token fails after
  another acquires.
- `cas_respects_tombstone_version` (**property**): ties A5.
- `lock_survives_partition_via_raft_authority` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked conditional_writes` and chaos with
  `-- --ignored`.

**Pros.** Gives callers safe single-key coordination (CAS, put-if-absent, fenced lock)
on the existing Raft authority — covering real needs (leader election, load dedup)
without opening distributed transactions.

**Risks.** Conditional writes pay Raft latency and callers may overuse them. Mitigation:
they are explicitly heavier-path APIs, refused below `Quorum`, and single-key only.

---

## W6. Durable Replayable Invalidation Stream (Hazelcast Ringbuffer / Reliable Topic)

**Problem / motivation.** Invalidations today are best-effort frames on the bus; a
near-cache or client that misses one falls back to the `0.41` B1 conservative
"invalidate-on-gap" — correct but it over-invalidates and hurts hit-rate, and a
restarted subscriber can't recover precisely. Hazelcast's **Ringbuffer / Reliable
Topic** model gives a bounded, sequence-numbered, **replayable** event log: a subscriber
that fell behind replays from its last sequence instead of flushing everything.

**Design / contract.** Back the invalidation bus with a per-partition bounded ring
buffer of sequence-numbered invalidation events (`message_id` already exists in
`CacheInvalidationFrame` / B1 watermark). A subscriber (in-process near-cache, remote
client, repair task) tracks its consumed sequence and **replays the exact missed range**
from the ring on reconnect/lag, as long as the range is still within the buffer
(capacity = retention window). If the subscriber fell **beyond** the buffer, it falls
back to the B1 `ClearPartition` (the only safe action) — that fallback is **counted** so
operators can size the buffer. The ring is bounded (count/bytes); it never grows
unboundedly and never blocks the write path (a full ring advances its tail and bumps a
counter, the slow consumer then takes the clear-partition fallback). Optionally durable
behind the `0.42` engine feature so a restarted owner keeps the recent window.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/invalidation_ring.rs
pub struct InvalidationRing {
    capacity: usize,
    head_seq: u64,                 // oldest retained sequence
    events: VecDeque<InvalidationEvent>, // (key, generation=last_uuid, message_id=last_seq)
}

pub enum ReplayResult {
    Range(Vec<InvalidationEvent>),  // subscriber within retention -> exact replay
    FellBehind,                     // beyond retention -> B1 ClearPartition fallback (counted)
}

impl InvalidationRing {
    pub fn publish(&mut self, ev: InvalidationEvent) -> u64;       // returns assigned seq
    pub fn replay_from(&self, last_seen: u64) -> ReplayResult;
}
```

**Step-by-step implementation.**

1. Add `InvalidationRing` per partition behind the invalidation bus; assign monotonic
   `message_id`s (reuse the existing field).
2. On subscriber reconnect/lag, `replay_from(last_seen)`: within retention → exact
   replay; beyond → B1 `ClearPartition` fallback + `invalidation_fell_behind_total`.
3. Keep it bounded and non-blocking: a full ring advances its tail (bumps
   `invalidation_ring_overrun_total`); never block the write path.
4. Make remote clients (the bus already carries B1 fields) replay over the wire the same
   way as in-process near-caches.
5. Optionally persist the recent window behind the `0.42` engine feature for
   restart-survivable replay.
6. Export `invalidation_ring_depth` (gauge), `invalidation_replayed_total`,
   `invalidation_fell_behind_total` (bounded labels).

**Testing.** `crates/hydracache/tests/invalidation_ring.rs`

- `subscriber_within_retention_replays_exact_range` (integration): no over-invalidation.
- `subscriber_beyond_retention_falls_back_to_clear_partition` (integration): ties B1,
  counted.
- `full_ring_advances_tail_without_blocking_writes` (**property**): write path never
  stalls.
- `restart_keeps_recent_window_when_durable` (integration, `durable` feature).
- `remote_client_replays_like_embedded` (**property**): ties B1 / `0.44`.
- Run: `cargo test -p hydracache --locked invalidation_ring`.

**Pros.** Lost invalidations are recovered precisely instead of by flushing the
partition — better hit-rate after blips — while staying bounded and non-blocking; the
fallback is observable so the buffer can be sized.

**Risks.** The ring adds memory per partition. Mitigation: bounded capacity, depth is a
gauge, and the fell-behind counter tells operators when to grow it.

---

## Deferred To 0.46+ (Explicit)

- **Full distributed transactions** (serializable cross-node/cross-region multi-key
  commit). Still a hard non-goal; W5 is single-key only.
- **Ecosystem / external consumers** (stable client protocol, Hibernate L2 provider,
  SDKs, multi-tenancy, residency). Drafted in
  `DRAFT_ECOSYSTEM_AND_EXTERNAL_CONSUMERS_PLAN.md`; slotted after the cluster-resilience
  work.
- **Causal+ / cross-region session guarantees** (read-your-writes / monotonic reads
  spanning regions for a session). Deferred from `0.44`.
- **Automatic home-region placement / latency-based home assignment** and
  **provider-specific autoscaler controllers.** Deferred from `0.44`.
- **Compute-near-data / entry processors.** Out of scope (RCE non-goal).

## Fault Model and Test Tiering

`0.45` reuses the `0.41`–`0.44` shared fault model and harness verbatim
(`crates/hydracache/tests/support/fault_injector.rs`) and its determinism contract
(seeded, replayable, logical-signal assertions — never wall-clock pass/fail). The
inherited model already includes whole-region loss, cross-region partition, and
lossy/metered WAN (`0.44`).

`0.45` **adds** faults that exercise the new resilience primitives:

- **liveness flapping** (rapid up/down/up heartbeats) — drives W4 false-suspect
  resistance and must not churn ownership (ties A1);
- **brief outage then recovery within the hint window** vs **outage beyond the window**
  — drives W2 replay-vs-repair fallback;
- **subscriber far behind beyond ring retention** — drives W6 exact-replay-vs-clear
  fallback;
- **lock holder pause/resume** (stop-the-world then continue) — drives W5 fenced-token
  rejection.

Clock skew is injected only to stress logic (e.g., phi inter-arrival math), never as a
correctness source; authority stays epoch/version.

| Tier | Scope | When | Command shape |
| --- | --- | --- | --- |
| fast | unit + deterministic property (CL math, phi, merkle diff, CAS, ring replay) | every PR | `cargo test --workspace --locked` |
| integration | in-memory multi-node, hinted handoff, read-repair, fenced lock, ring replay | every PR | `cargo test --workspace --locked` |
| chaos/soak | seeded flapping, outage-window boundaries, lock pause/resume, repair resume | nightly / pre-release | `cargo test --workspace --locked -- --ignored` |
| Docker | testcontainers multi-process: durable hints/ring + repair under churn | nightly / pre-release | Docker gate |

## Release Gates For 0.45

Focused:

```powershell
cargo test -p hydracache --locked consistency_levels
cargo test -p hydracache --locked hinted_handoff
cargo test -p hydracache --locked merkle_repair
cargo test -p hydracache --locked failure_detector
cargo test -p hydracache --locked conditional_writes
cargo test -p hydracache --locked invalidation_ring
cargo test -p hydracache --locked fault_injector_selftest
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --locked --features durable-log,durable-values
cargo test --workspace --locked -- --ignored   # flapping / outage-window / lock-pause chaos suites
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.45.0` may claim **resilient, coordinated cluster** (Hazelcast/ScyllaDB primitives
adopted) only if **all** of the following boolean conditions hold:

- W1: per-operation `ConsistencyLevel` (`One`/`LocalQuorum`/`Quorum`/`EachQuorum`/`All`)
  maps onto the quorum math, an unsatisfiable level fails loud (never silently
  degrades), and the deployment default is preserved; `consistency_levels` passes.
- W2: hinted handoff restores RF for brief outages, is bounded (count/bytes/age),
  never resurrects a tombstone, never fakes a required ack, drops over-budget/expired
  hints with a counter + repair mark; `hinted_handoff` passes (incl. chaos).
- W3: Merkle-tree repair descends only mismatches, foreground read-repair fixes
  divergence inline on `Quorum`+ reads, incremental repair skips repaired ranges,
  preserves the A5 invariant, and resumes after coordinator crash; `merkle_repair`
  passes (incl. chaos).
- W4: the phi-accrual detector adapts to heartbeat distributions, resists flapping,
  never changes ownership before `CommitTopology`, and gates W2/W3; `failure_detector`
  passes.
- W5: single-key `compare_and_set`/`put_if_absent` are linearizable via partition Raft,
  refused below `Quorum`, multi-key attempts rejected loud (non-goal guard), and the
  fenced lock rejects stale tokens; `conditional_writes` passes (incl. chaos).
- W6: the invalidation ring replays exact missed ranges within retention, falls back to
  B1 `ClearPartition` (counted) beyond it, never blocks the write path, and remote
  clients replay like embedded ones; `invalidation_ring` passes.
- The fault model adds liveness flapping, outage-window boundaries, far-behind
  subscriber, and lock pause/resume, and all those suites pass.
- Docs keep the prominent **"still not distributed transactions"** warning (W5 is
  single-key only) and list ecosystem/external consumers, causal+ session guarantees,
  auto home-placement, and provider-specific autoscaler controllers as deferred to
  0.46+.

If any condition fails, `0.45.0` ships **without** the corresponding claim, documents
exactly which work item(s) did not land, and the claim moves to a later release.
