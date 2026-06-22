# HydraCache 0.43.0 Geo-Distribution & Elasticity Plan

> **At a glance**
> - **What:** zone/region-aware placement, online resharding, locality + hedged reads, tiered value storage, narrow atomic-invalidation slice, operational self-healing.
> - **Why:** survive a zone loss and reshard online without a maintenance window.
> - **After (depends on):** 0.42.
> - **Unblocks:** 0.44 (active-active multi-region).
> - **Status:** shipped — Phase F gates validate multi-node/zone behavior over a real networked transport; see [`V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md`](V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md).
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md)

Status: implemented in `0.43.0`. Release notes:
[`docs/releases/0.43.0.md`](../releases/0.43.0.md).

`0.43.0` builds on the production-grid claim that `0.42.0` earned. Where `0.42`
proved production readiness for a **single, flat topology** (durable multi-node
Raft control plane, durable replicated values, hardened failover, split-brain
detection + merge, grid-wide read-your-writes, enforced identity/authz, and an
operator surface), `0.43` extends the grid along the two axes `0.42` explicitly
deferred — **geography** (multi-region / zone-aware placement) and **elasticity**
(online resharding without a downtime window) — and adds a deliberately narrow,
honest transaction slice plus tiered value storage and operational
self-healing.

The release keeps the same authority/dissemination resolution rule from `0.41`/`0.42`:

> **Authority** (who owns a key, which topology is valid, which version is newer)
> is the ScyllaDB model: Raft + monotonic epoch. **Dissemination** (how staleness
> is detected and propagated) is the Hazelcast model: sequence/UUID stamps. When
> the two disagree, the epoch (authority) wins; the stamp only triggers a
> conservative refresh/invalidate.

Readiness is described in prose and asserted as boolean release gates. There is
no numeric self-score. `0.43` does **not** weaken any `0.42` guarantee: every new
capability is opt-in and the flat-topology, single-region behavior of `0.42`
remains byte-for-byte the default.

## Release Theme

Take the production grid from one flat region to **zone/region-aware placement
that survives a zone loss** and to **elastic membership that reshards online**,
while adding a narrow single-partition atomic-invalidation slice (not full
distributed transactions), tiered value storage, and operational self-healing.

The work is six items (W1–W6) plus explicit deferrals. Each builds on a named
`0.42`/`0.41` artifact and turns "single topology / fixed membership" into
"topology-aware / elastic", without re-litigating earlier design.

## Non-Goals

- **No full distributed transactions.** Cross-partition / cross-node atomic
  multi-key commit (2PC, Calvin, deterministic transactions) remains a hard
  non-goal. `0.43` ships only single-partition multi-key atomic invalidation and a
  best-effort cross-partition saga over the `0.37` outbox — both clearly scoped as
  *not* serializable cross-node transactions. The prominent "still not distributed
  transactions" warning stays.
- **No automatic cross-region write conflict resolution beyond the 0.42 merge
  policy.** Geo-replication uses the existing `(version, epoch)` authority rule and
  `MergePolicy`; `0.43` does not introduce CRDTs or vector clocks.
- **No global clock / true-time dependency.** Authority stays epoch/version, never
  wall-clock; clock skew across regions is a fault to tolerate, not a correctness
  source.
- **No KMS / secret-store.** Identity and crypto material stay operator-supplied
  via the `0.41`/`0.42` provider traits.
- **No novel storage-engine invention.** Tiered storage (W4) uses the embedded
  engine selected in `0.42`; it does not write a new engine.

## Inherited Boundary From 0.42

`0.43` only extends `0.42`; it must not redesign it.

- **`ClusterReplicationStrategy` + `EffectiveReplicationMap` (0.41 A3)** placed
  replicas in a flat set. **Zone/region-aware placement** (the ScyllaDB
  `NetworkTopologyStrategy` analogue) is `0.43` W1.
- **Rebalance plan-as-data (0.41 A4)** computed a plan executed through Raft, but
  membership changes assumed a controlled window. **Online resharding under live
  load** is `0.43` W2.
- **Grid-wide quorum read-your-writes (0.42 W5)** assumed flat replicas.
  **Locality-aware and hedged reads** that keep the W5 contract while preferring
  local-zone replicas are `0.43` W3.
- **The transactional outbox (0.37)** and **single-partition value store (0.42
  W2)** are the substrate for the **narrow atomic-invalidation slice** in W5.
- **Durable replicated value store (0.42 W2)** held everything in the chosen
  engine. **Tiered hot/cold value spill** is `0.43` W4.
- **Operator surface + repair-debt degraded mode (0.42 W7)** warned and throttled.
  **Operational self-healing (auto-repair, snapshot backup/restore, upgrade
  orchestration)** is `0.43` W6.

## Dependency Graph

```
0.41 A3 ClusterReplicationStrategy ──► W1 zone/region-aware placement
0.41 A4 rebalance plan-as-data ──────► W2 online resharding (elastic membership)
0.42 W5 quorum read-your-writes ─────► W3 locality-aware + hedged reads
0.42 W2 durable value store ─────────► W4 tiered hot/cold value spill
0.37 outbox + 0.42 W2 ───────────────► W5 narrow atomic-invalidation slice
0.42 W7 operator surface ────────────► W6 operational self-healing
W1 (zones) ──────────────────────────► W3, W6   (zone loss is the headline fault)
```

W1 is the long pole: zone-aware placement changes the failure model that W3
(locality reads), W2 (resharding must respect zones), and W6 (zone-loss recovery)
all depend on.

---

## W1. Zone / Region-Aware Replica Placement

**Problem / motivation.** `0.41` A3 placed primary + backups over a flat admitted
member set via rendezvous hashing; `0.42` proved that flat topology in production.
But a flat strategy can put every replica of a key in one availability zone, so a
single zone loss takes the key offline even at RF=3. Production deployments span
zones/regions and need replicas spread so that losing a zone never loses a quorum.

**Design / contract.** Each member declares a `topology` (region, zone) at join,
carried in the gossip payload and committed through Raft `CommitTopology` (A1) so
placement is authoritative, not gossip-derived. Add a
`ZoneAwareReplicationStrategy` that wraps the `0.41` rendezvous ownership: it picks
the primary by rendezvous, then selects backups so that no two replicas share a
zone until zones are exhausted (the ScyllaDB `NetworkTopologyStrategy` rule). The
`EffectiveReplicationMap` gains a per-replica zone tag. Quorum math (0.42 W5)
becomes zone-aware: a configurable `min_zones_for_quorum` guarantees a write
quorum survives a single-zone loss. Flat behavior is preserved when only one zone
is declared.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/topology.rs
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NodeTopology { pub region: RegionId, pub zone: ZoneId }

// crates/hydracache/src/cluster/placement.rs
pub struct ZoneAwareReplicationStrategy {
    inner: RendezvousClusterOwnership, // 0.41
    rf: usize,
    min_zones: usize,                  // zones a key must span
}

impl ClusterReplicationStrategy for ZoneAwareReplicationStrategy {
    fn replicas_for_key(&self, key: &CacheKey, members: &[Member]) -> ReplicaSet {
        let ranked = self.inner.rank(key, members); // rendezvous order
        // primary = ranked[0]; pick backups skipping zones already used
        // until rf reached or zones exhausted; tag each with its zone.
        select_zone_spread(ranked, self.rf, self.min_zones)
    }
}
```

**Step-by-step implementation.**

1. Add `NodeTopology { region, zone }` to the member descriptor; carry it in
   gossip and commit it via `CommitTopology` (authoritative, not gossip-derived).
2. Add `ZoneAwareReplicationStrategy` wrapping `RendezvousClusterOwnership`; tag
   each entry in `EffectiveReplicationMap` with its zone.
3. Make quorum validation (0.42 W5) zone-aware: enforce `min_zones_for_quorum` at
   startup; refuse configs where a single-zone loss breaks write quorum (loud,
   like `0.42` AUTH/plaintext flags).
4. Degrade gracefully when zones are under-supplied (fewer zones than RF): place
   what is possible and surface a `placement_zone_underspread` gauge + readiness
   flag rather than silently co-locating.
5. Keep one-zone deployments byte-for-byte identical to `0.42`.

**Testing.** `crates/hydracache/tests/zone_placement.rs`

- `replicas_spread_across_zones_when_available` (unit): RF=3 over 3 zones → one
  replica per zone, deterministic.
- `single_zone_loss_keeps_write_quorum` (integration): drop a zone's members;
  assert write quorum still reachable for all keys when `min_zones` honored.
- `underspread_zones_are_flagged_not_silently_colocated` (unit): RF=3, 2 zones →
  `placement_zone_underspread` set + readiness flag.
- `one_zone_deployment_matches_042_placement` (property): with a single declared
  zone, placement equals the flat `0.41`/`0.42` rendezvous result.
- `zone_topology_is_authoritative_not_gossip` (integration): a gossip-only zone
  change does not move replicas until `CommitTopology` commits it.
- Run: `cargo test -p hydracache --locked zone_placement`.

**Pros.** Survives a zone outage at RF≥3; aligns the placement model with how
production clusters are actually deployed; reuses rendezvous + the A1 fence.

**Risks.** Cross-zone replication raises write latency and egress cost.
Mitigation: async backups (0.41 B5) can be cross-zone while a sync backup stays
local; cross-zone byte counters feed the operator surface.

---

## W2. Online Resharding (Elastic Membership)

**Problem / motivation.** `0.41` A4 made rebalance a plan-as-data executed through
Raft, but adding/removing members in `0.42` still implied a controlled, low-traffic
window: large partition moves under live load could stall the hot path or violate
the W5 read-your-writes contract mid-move. Production needs to add or drain nodes
**online**, under load, without a maintenance window.

**Design / contract.** Generalize the A4 rebalance plan into a multi-step,
throttled, resumable migration: each partition move is `PrepareMove` (start
shadowing writes to the new owner) → `BackfillMove` (stream existing values +
delta-catchup via B6 anti-entropy) → `CommitMove` (flip ownership in
`EffectiveReplicationMap` through Raft) → `CleanupMove` (drop the source copy).
During a move the partition is in a `reading`/`pending` overlap state (the
`EffectiveReplicationMap`'s existing `natural`/`reading`/`pending` separation), so
reads/writes are served correctly throughout and the W5 contract holds: an
acknowledged write is shadowed to both old and new owner before `CommitMove`. The
migration is rate-limited (reusing the W3-0.42 adaptive flow control) and resumable
across coordinator failover (the plan and its progress live in Raft).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/resharding.rs
pub enum MovePhase { Prepare, Backfill, Commit, Cleanup }

pub struct PartitionMove {
    pub partition: PartitionId,
    pub from: ClusterNodeId,
    pub to: ClusterNodeId,
    pub phase: MovePhase,
    pub backfilled_bytes: u64, // progress, persisted in Raft
}

pub struct ReshardPlan { pub moves: Vec<PartitionMove>, pub max_concurrent: usize }
```

**Step-by-step implementation.**

1. Extend the A4 plan into `ReshardPlan` with per-move phase + progress; persist
   progress through Raft so a coordinator crash resumes, not restarts.
2. Implement `PrepareMove` write-shadowing: writes to a moving partition go to both
   current and target owner; reads stay on the current owner until `CommitMove`.
3. Implement `BackfillMove`: stream the durable value store (0.42 W2) to the
   target, then delta-catchup via B6 anti-entropy until lag is within a threshold.
4. `CommitMove`: flip ownership atomically in `EffectiveReplicationMap` via Raft;
   `CleanupMove`: drop the source after confirmation (respecting the A5 tombstone
   invariant — never resurrect a deleted key during backfill).
5. Rate-limit with the `0.42` W3 adaptive window; expose `reshard_moves_inflight`,
   `reshard_backfill_lag`, and `reshard_progress_ratio` gauges.
6. Zone-aware (W1): a move must not violate zone-spread; the planner rejects moves
   that would co-locate a quorum.

**Testing.** `crates/hydracache/tests/online_reshard.rs`

- `write_during_move_is_shadowed_to_both_owners` (integration): write mid-move;
  assert both old and new owner hold it before `CommitMove`.
- `read_your_writes_holds_across_a_move` (**property**): random writes interleaved
  with moves; assert the W5 contract is never violated.
- `coordinator_crash_resumes_move_from_progress` (**chaos**, `#[ignore]`): kill the
  coordinator mid-backfill via the fault harness; assert the move resumes from
  persisted progress, no restart.
- `tombstone_not_resurrected_during_backfill` (**property**): delete a key while
  its partition backfills; assert it stays deleted on the target.
- `move_respecting_zone_spread_is_rejected_if_it_colocates_quorum` (unit): ties to
  W1.
- `drain_node_moves_all_partitions_then_leaves_cleanly` (integration).
- Run: `cargo test -p hydracache --locked online_reshard` and chaos with
  `-- --ignored`.

**Pros.** Add/drain capacity under live load with no maintenance window and no
consistency regression; resumable across failover.

**Risks.** Write-shadowing doubles write cost during a move and backfill competes
with the hot path. Mitigation: rate-limit + lag thresholds + operator-visible
progress; only `max_concurrent` partitions move at once.

---

## W3. Locality-Aware & Hedged Reads

**Problem / motivation.** `0.42` W5 gave grid-wide quorum read-your-writes, but a
quorum read in a multi-zone deployment (W1) may cross zones on every read, adding
latency and egress. Production wants reads to prefer the local zone when the
consistency level allows, and to hedge against a slow replica without waiting for
a timeout.

**Design / contract.** Add a replica-selection policy to the read path: for
`Eventual` reads, prefer the nearest (same-zone) replica; for
`QuorumReadYourWrites` (0.42 W5), still contact `read_quorum` replicas but prefer a
local-zone replica as one of them and order the rest by observed latency. Add
**hedged reads**: if the first replica does not answer within an adaptive
percentile-based delay, send a backup request to the next replica and take the
first valid `(version, epoch)` response — never weakening the W5 quorum count, only
reducing tail latency. Replica health/latency feeds an adaptive scoring like
ScyllaDB's dynamic snitch / Cassandra speculative retry.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/read_path.rs
pub enum ReplicaSelection { NearestZone, LowestLatency, RoundRobin }

pub struct HedgePolicy {
    pub after: HedgeDelay,     // adaptive: p50/p99 of observed RTT, not wall-clock SLO
    pub max_extra: usize,      // cap on concurrent hedge requests
}

pub struct ReplicaScorer { /* EWMA latency + zone distance + health */ }

impl ReplicaScorer {
    pub fn order(&self, replicas: &ReplicaSet, local: &NodeTopology) -> Vec<ClusterNodeId>;
}
```

**Step-by-step implementation.**

1. Add `ReplicaScorer` (EWMA latency + zone distance + health) and order replicas
   per read; prefer same-zone for `Eventual`, local-preferred quorum for RYOW.
2. Add hedged reads: adaptive hedge delay from observed RTT percentiles; cap extra
   requests; the W5 quorum count is unchanged — hedging only adds redundancy, never
   reduces required acks.
3. Reconcile hedge winners by max `(version, epoch)`; never serve a lower version
   even if it arrives first.
4. Export `read_local_zone_ratio`, `read_hedged_total`, `read_hedge_win_total`, and
   cross-zone read bytes (bounded labels; cardinality discipline from 0.41).

**Testing.** `crates/hydracache/tests/locality_reads.rs`

- `eventual_read_prefers_local_zone` (integration): assert same-zone replica is
  chosen when healthy.
- `quorum_read_still_contacts_read_quorum` (integration): with hedging on, assert
  the W5 quorum count is unchanged.
- `slow_replica_triggers_hedge_and_returns_fresh` (integration): inject latency on
  the first replica; assert a hedge fires and the freshest `(version, epoch)` wins.
- `hedge_winner_is_max_version_not_first_arrival` (**property**).
- `hedge_delay_adapts_to_rtt_distribution` (unit): drive observed RTTs; assert the
  hedge-after delay tracks the percentile.
- Run: `cargo test -p hydracache --locked locality_reads`.

**Pros.** Cuts cross-zone read latency and tail latency without weakening the W5
consistency contract; egress drops for local-preferred reads.

**Risks.** Hedging adds read amplification. Mitigation: `max_extra` cap +
adaptive delay + amplification counter feeding the operator surface.

---

## W4. Tiered Hot/Cold Value Spill

**Problem / motivation.** `0.42` W2 made replicated values durable, but everything
lived in the chosen embedded engine and hot working sets competed with cold data
for memory. Large grids need a memory tier for hot values and a disk tier for cold
ones, integrated with the existing moka weigher and the `0.37` byte budgets, so
memory stays bounded while capacity scales.

**Design / contract.** Add a two-tier value store behind the `0.42` W2
`ReplicatedValueStore`: a hot in-memory tier (moka, with the existing byte-weigher
and `max_entry_bytes` reject from `0.37`) backed by a cold durable tier (the `0.42`
engine). Admission/eviction between tiers is by recency/frequency (reuse moka's
TinyLFU); a miss in hot loads from cold and promotes; eviction from hot demotes to
cold (or drops if non-durable). The A5 versioned-tombstone invariant holds across
both tiers — a tombstone in either tier wins. Tiering is opt-in
(`tiered_values(true)`); the default is the `0.42` single-tier behavior.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/value_store/tiered.rs
pub struct TieredValueStore {
    hot: moka::sync::Cache<CacheKey, ReplicatedValueRecord>, // byte-weighed, 0.37 weigher
    cold: Box<dyn ReplicatedValueStore>,                     // 0.42 durable store
}

impl ReplicatedValueStore for TieredValueStore {
    fn get(&self, key: &CacheKey) -> Result<Option<ReplicatedValueRecord>, ValueStoreError> {
        if let Some(rec) = self.hot.get(key) { return Ok(Some(rec)); }
        let rec = self.cold.get(key)?;
        if let Some(r) = &rec { self.hot.insert(key.clone(), r.clone()); } // promote
        Ok(rec)
    }
    // upsert writes hot + cold; tombstone tombstones both; eviction demotes hot->cold
}
```

**Step-by-step implementation.**

1. Add `TieredValueStore` wrapping a moka hot tier (0.37 byte-weigher,
   `max_entry_bytes` reject) over the 0.42 durable cold tier.
2. Promote on cold hit, demote on hot eviction; enforce that a tombstone in either
   tier wins (A5 invariant), and that demotion never loses a newer version.
3. Wire the `0.37` memory budget to the hot tier only; cold tier is bounded by disk
   budget with the same "reject, never silently drop" posture.
4. Keep `tiered_values(false)` (default) identical to `0.42` W2.
5. Export `value_tier_hot_ratio`, `value_tier_promotions_total`,
   `value_tier_demotions_total`.

**Testing.** `crates/hydracache/tests/tiered_values.rs`

- `cold_hit_promotes_to_hot` (unit).
- `hot_eviction_demotes_to_cold_without_loss` (integration): evict under memory
  pressure; assert the value is still served from cold at the correct version.
- `tombstone_in_either_tier_wins` (**property**): ties to A5.
- `hot_tier_respects_byte_budget` (integration): assert memory stays within the
  0.37 budget under load.
- `tiering_off_matches_042_behavior` (property).
- Run: `cargo test -p hydracache --locked tiered_values`.

**Pros.** Bounded memory with disk-scale capacity; reuses moka TinyLFU and the
0.37 byte budgets; opt-in so existing deployments are unaffected.

**Risks.** Cold-tier reads add latency on hot misses. Mitigation: hot ratio is a
gauge, sizing guidance is in docs, and W3 hedging masks cold-tier tail latency.

---

## W5. Narrow Atomic-Invalidation Slice (NOT Distributed Transactions)

**Problem / motivation.** Full cross-node distributed transactions stay a hard
non-goal — they are expensive and would consume the release. But callers
frequently need a *bounded* atomicity guarantee: invalidate a small set of keys
that live in one partition all-or-nothing, or fan out a related-key invalidation
across partitions reliably (eventually) without losing any. `0.43` ships exactly
that narrow slice and names it honestly.

**Design / contract.** Two clearly-scoped mechanisms, both built on existing
substrate:

1. **Single-partition multi-key atomic invalidation.** Because a partition has one
   primary, a set of keys in the same partition can be invalidated atomically via a
   single Raft-committed `InvalidateBatch` (versioned per A5). Serializable only
   within the partition; the contract is documented as such.
2. **Cross-partition best-effort saga over the outbox.** For related keys across
   partitions, enqueue all invalidations as one outbox unit (the `0.37`
   transactional outbox + idempotency key `(commit_position, sha256(target))`); the
   dispatcher guarantees each is eventually applied at-least-once and idempotently.
   This is explicitly **not** atomic or serializable across partitions — it is
   reliable eventual fan-out, documented with the visible interleaving window.

No 2PC, no cross-node locks, no serializable cross-partition transaction.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/atomic_invalidation.rs
pub struct InvalidateBatch {
    pub partition: PartitionId,        // single-partition only
    pub keys: SmallVec<[CacheKey; 8]>,
    pub version: ValueVersion,         // one version stamp for the whole batch (A5)
}

// crates/hydracache-db/src/saga.rs
pub struct InvalidationSaga {
    pub unit_id: OutboxUnitId,         // one outbox unit -> many partition targets
    pub targets: Vec<InvalidationTarget>,
}
// dispatched at-least-once + idempotently via the 0.37 outbox; NOT serializable.
```

**Step-by-step implementation.**

1. Add `InvalidateBatch` committed as one Raft entry on the partition primary;
   apply all keys at one `(version, epoch)` so a reader never sees a half-applied
   batch within the partition.
2. Reject batches that span partitions with a `compile_error!`-style runtime error
   pointing at the saga API — never silently degrade single-partition atomicity to
   best-effort.
3. Add `InvalidationSaga` as one outbox unit fanning out to multiple partition
   targets; reuse the `0.37` idempotency key + dispatcher; guarantee at-least-once
   idempotent application.
4. Document both contracts precisely in `docs/cluster/`: partition-atomic vs
   eventual-fan-out, with the interleaving window for the saga.
5. Export `invalidate_batch_total` and `invalidation_saga_pending` (outbox lag
   reuse from 0.37).

**Testing.** `crates/hydracache/tests/atomic_invalidation.rs`

- `single_partition_batch_is_all_or_nothing` (integration): a reader sees either
  all keys at the new version or all at the old, never a mix.
- `cross_partition_batch_is_rejected_pointing_at_saga` (unit): assert the loud
  rejection, not silent best-effort.
- `saga_fans_out_at_least_once_idempotently` (integration): inject a dispatcher
  retry; assert each target applied exactly-once-effect via the idempotency key.
- `saga_survives_dispatcher_crash` (**chaos**, `#[ignore]`): kill mid-fan-out;
  assert all targets eventually applied on resume.
- `batch_version_beats_concurrent_single_writes` (**property**): ties to A5.
- Run: `cargo test -p hydracache --locked atomic_invalidation` and chaos with
  `-- --ignored`.

**Pros.** Covers the real-world need (related-key invalidation) honestly, on
existing Raft + outbox substrate, without opening the distributed-transaction can.

**Risks.** Callers may mistake the saga for a transaction. Mitigation: the API
names (`InvalidationSaga`, not `Transaction`), the loud cross-partition rejection,
and explicit docs.

---

## W6. Operational Self-Healing

**Problem / motivation.** `0.42` W7 gave a read-only status surface, dashboards,
repair-debt degraded mode, and a runbook — but recovery still needed an operator to
read the runbook and act. At grid scale and across zones (W1), routine faults
(a backup falling behind, a node draining, a zone blip) should self-heal under a
policy, and the control plane needs backup/restore plus upgrade orchestration.

**Design / contract.** Three capabilities:

1. **Policy-driven auto-repair.** When repair-debt or replication-lag (0.42 W3/W7)
   crosses a threshold, an `AutoRepairPolicy` schedules anti-entropy (B6) and, if
   under-replicated past a bound, triggers a bounded re-replication or a W2
   partition move — all as topology ops through Raft, rate-limited, and capped so
   repair never overwhelms the hot path. Operators can run it as advisory
   (suggest-only) or active.
2. **Control-plane snapshot backup/restore.** Periodic durable snapshots of the W1
   metadata (members, epoch, ownership, tombstone versions) to an operator-supplied
   `SnapshotSink`; restore reconstructs the control plane on catastrophic loss.
3. **Upgrade orchestration.** A rolling-upgrade helper that checks the
   `docs/COMPAT.md` window (raft log format, value-record format, wire frame
   versions) before allowing a mixed-version step, refusing incompatible jumps
   loud (the 0.37 §5a / 0.41 / 0.42 discipline).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/self_heal.rs
pub enum RepairMode { Advisory, Active }

pub struct AutoRepairPolicy {
    pub mode: RepairMode,
    pub debt_threshold: u64,
    pub max_concurrent_repairs: usize,
}

pub trait SnapshotSink: Send + Sync {           // operator-supplied (S3, disk, ...)
    fn put(&self, snapshot: &ControlPlaneSnapshot) -> Result<(), SnapshotError>;
    fn latest(&self) -> Result<Option<ControlPlaneSnapshot>, SnapshotError>;
}

pub struct UpgradeGuard;                          // checks docs/COMPAT.md window
```

**Step-by-step implementation.**

1. Add `AutoRepairPolicy`; in `Active` mode, schedule B6 anti-entropy + bounded
   re-replication / W2 moves through Raft when debt/lag crosses thresholds;
   `Advisory` mode only surfaces recommendations in the W7 status.
2. Cap concurrent repairs and rate-limit (reuse 0.42 W3 adaptive window) so repair
   never starves the hot path; export `auto_repair_active_total`,
   `auto_repair_advisory_total`.
3. Add `SnapshotSink` + periodic control-plane snapshot; implement restore that
   rebuilds the W1 control plane from `latest()`.
4. Add `UpgradeGuard` that reads `docs/COMPAT.md` and refuses an incompatible
   mixed-version step loud; expose it in the W7 status.
5. Extend the repair runbook with zone-loss recovery (W1) and restore procedure.

**Testing.** `crates/hydracache/tests/self_heal.rs`

- `debt_over_threshold_triggers_bounded_repair` (integration): assert active mode
  schedules repair, capped at `max_concurrent_repairs`.
- `advisory_mode_suggests_but_does_not_act` (unit).
- `repair_never_starves_hot_path` (integration): under load, assert hot-path
  latency stays within budget while repair runs.
- `control_plane_restore_rebuilds_topology` (integration): snapshot, wipe, restore;
  assert committed topology/ownership/tombstones match.
- `upgrade_guard_refuses_incompatible_step` (unit): a version pair outside the
  COMPAT window is refused loud.
- `zone_loss_self_heals_to_target_rf` (**chaos**, `#[ignore]`): drop a zone (W1);
  assert auto-repair restores RF across surviving zones within bounds.
- Run: `cargo test -p hydracache --locked self_heal` and chaos with `-- --ignored`.

**Pros.** Routine faults heal under policy without paging an operator; the control
plane is recoverable from catastrophic loss; upgrades are guarded by the existing
compatibility register.

**Risks.** Auto-repair acting at the wrong time can amplify load during an
incident. Mitigation: `Advisory` default, hard concurrency cap, rate-limit, and an
operator kill-switch surfaced in the W7 status.

---

## Deferred To 0.44+ (Explicit)

- **Full distributed transactions** (serializable cross-node multi-key commit;
  2PC/Calvin/deterministic). Still a hard non-goal; `0.43` ships only the narrow
  W5 slice.
- **CRDT / conflict-free geo-replication.** `0.43` geo-replication uses the
  `(version, epoch)` authority + `MergePolicy`; richer convergence types are a
  later, separate design.
- **Active-active multi-region writes with bounded staleness SLAs.** `0.43` does
  zone/region-aware placement and locality reads; a formal cross-region write SLA
  is future work.
- **Automatic capacity planning / autoscaling.** W2 makes resharding online and
  safe; deciding *when* to scale (autoscaler integration) is deferred.

## Fault Model and Test Tiering

`0.43` reuses the `0.41`/`0.42` shared fault model and harness verbatim
(`crates/hydracache/tests/support/fault_injector.rs`) and its determinism contract
(seeded, replayable, logical-signal assertions — never wall-clock pass/fail). It
**adds one enumerated fault**: whole-zone loss (all members of a `ZoneId` partition
away at once), which drives W1/W3/W6 zone-loss suites. Cross-region latency and
bounded clock skew are injected, never used as a correctness source.

| Tier | Scope | When | Command shape |
| --- | --- | --- | --- |
| fast | unit + deterministic property (zone spread, hedge scoring, tier promotion, batch atomicity) | every PR | `cargo test --workspace --locked` |
| integration | in-memory multi-zone, online move, locality reads, tiering, saga | every PR | `cargo test --workspace --locked` |
| chaos/soak | seeded zone loss, coordinator-crash-mid-reshard, saga dispatcher crash, self-heal under load | nightly / pre-release | `cargo test --workspace --locked -- --ignored` |
| Docker | testcontainers multi-process across simulated zones | nightly / pre-release | Docker gate |

## Release Gates For 0.43

Focused:

```powershell
cargo test -p hydracache --locked zone_placement
cargo test -p hydracache --locked online_reshard
cargo test -p hydracache --locked locality_reads
cargo test -p hydracache --locked tiered_values
cargo test -p hydracache --locked atomic_invalidation
cargo test -p hydracache --locked self_heal
cargo test -p hydracache --locked fault_injector_selftest
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --locked --features durable-log,durable-values,tiered-values
cargo test --workspace --locked -- --ignored   # zone-loss / reshard / saga chaos suites
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.43.0` may claim **zone/region-aware, elastically-resizable production grid**
(for the supported topology) only if **all** of the following boolean conditions
hold:

- W1: replicas spread across declared zones; a single-zone loss keeps write quorum;
  under-supplied zones are flagged, not silently co-located; one-zone deployments
  match `0.42` placement; `zone_placement` passes.
- W2: nodes add/drain online under load with no maintenance window; read-your-writes
  (0.42 W5) holds across a move; moves resume after coordinator crash; no tombstone
  resurrection during backfill; `online_reshard` passes (incl. chaos).
- W3: reads prefer the local zone for `Eventual` and keep the W5 quorum count for
  RYOW; hedging reduces tail latency and returns the max `(version, epoch)`;
  `locality_reads` passes.
- W4: tiered storage keeps the hot tier within the `0.37` byte budget, demotes
  without loss, honors the A5 tombstone invariant across tiers, and is off-by-default
  identical to `0.42`; `tiered_values` passes.
- W5: single-partition multi-key invalidation is all-or-nothing; cross-partition is
  rejected loud and offered as an at-least-once idempotent saga over the `0.37`
  outbox; neither is presented as a distributed transaction; `atomic_invalidation`
  passes.
- W6: auto-repair acts under policy without starving the hot path; the control plane
  is snapshot-backed and restorable; the upgrade guard refuses incompatible
  mixed-version steps; `self_heal` passes (incl. zone-loss chaos).
- The fault model adds whole-zone loss and all zone-loss chaos suites pass.
- Docs keep the prominent **"still not distributed transactions"** warning, document
  the W5 saga's eventual (non-serializable) semantics, and list active-active
  multi-region writes / CRDTs as deferred.

If any condition fails, `0.43.0` ships **without** the corresponding claim,
documents exactly which work item(s) did not land, and the claim moves to a later
release.
