# HydraCache 0.44.0 Active-Active Multi-Region Plan

`0.44.0` builds on the zone/region-aware, elastically-resizable grid that
`0.43.0` delivered. Where `0.43` made placement **zone/region-aware** (replicas
spread so a zone loss never loses a quorum), reads **locality-aware**, membership
**elastic** (online resharding), and added tiered storage, a narrow
atomic-invalidation slice, and operational self-healing — all still within a
single write-authority region — `0.44` takes the grid **active-active across
regions**: multiple regions accept writes concurrently with a documented,
monitored bounded-staleness contract, conflict-free convergence for safe value
classes, a WAN-aware replication transport, region failover/DR, and the
autoscaling and geo-observability needed to operate it.

The release keeps the same authority/dissemination resolution rule from
`0.41`–`0.43`:

> **Authority** (who owns a key, which topology is valid, which version is newer)
> is the ScyllaDB model: Raft + monotonic epoch. **Dissemination** (how staleness
> is detected and propagated) is the Hazelcast model: sequence/UUID stamps. When
> the two disagree, the epoch (authority) wins; the stamp only triggers a
> conservative refresh/invalidate.

Readiness is described in prose and asserted as boolean release gates. There is
no numeric self-score. `0.44` does **not** weaken any `0.43` guarantee: every new
capability is opt-in, and single-region (active-passive) deployments keep the
`0.43` behavior byte-for-byte. Active-active is a distinct, explicitly-weaker
consistency mode that is never enabled silently.

## Release Theme

Let multiple regions accept writes at once under a bounded, observable staleness
contract — without ever silently downgrading the consistency a caller thinks they
have, and without claiming distributed transactions.

The work is six items (W1–W6) plus explicit deferrals. Each builds on a named
`0.41`–`0.43` artifact and turns "single write-authority region" into
"coordinated active-active regions".

## Non-Goals

- **No full distributed transactions.** Serializable cross-node / cross-region
  multi-key atomic commit (2PC, Calvin, deterministic transactions) remains a hard
  non-goal across the whole project. `0.44` adds no transaction semantics; the
  `0.43` W5 narrow slice (single-partition atomic invalidation + best-effort saga)
  is the ceiling. The prominent "still not distributed transactions" warning stays.
- **No strong (linearizable) consistency across regions.** Active-active is, by
  construction, a bounded-staleness / eventual contract for cross-region writes.
  Strong read-your-writes (`0.42` W5) holds **within** a region's authority, not
  globally. Every cross-region consistency limit is documented with the scenario
  that exposes it.
- **No automatic conflict resolution for arbitrary value types.** Conflict-free
  convergence (W2) applies only to opt-in CRDT classes (counters, sets,
  LWW-registers). Plain values still use the `0.42` `MergePolicy`
  (`HigherVersionWins` on `(version, epoch)`), which can drop a loser-side write —
  documented, counted, never silent.
- **No global clock / true-time dependency.** Authority stays epoch/version;
  cross-region clock skew is a tolerated fault, never a correctness source. LWW
  registers (W2) use a hybrid logical clock, not wall-clock, for tie-break.
- **No KMS / secret-store.** Identity and crypto material stay operator-supplied
  via the `0.41`/`0.42` provider traits; cross-region links reuse the `0.42` W6
  identity/authz model.

## Inherited Boundary From 0.43

`0.44` only extends `0.43`; it must not redesign it.

- **Zone/region-aware placement (0.43 W1)** placed replicas across zones within one
  write-authority region. **Multiple concurrently-writable regions** are `0.44` W1.
- **The `0.42` `MergePolicy`** resolved split-brain by dropping a loser. **Opt-in
  CRDT convergence** that avoids the drop for safe types is `0.44` W2.
- **Cross-zone async backups (0.41 B5) + locality reads (0.43 W3)** assumed
  intra-region links. **A WAN-aware replication transport + cross-region
  anti-entropy** is `0.44` W3.
- **Operational self-healing + control-plane snapshot/restore (0.43 W6)** recovered
  within a region. **Whole-region failover / DR promotion** is `0.44` W4.
- **Online resharding (0.43 W2)** moved partitions on operator/plan trigger.
  **Capacity signals that drive an autoscaler** to trigger resharding are `0.44` W5.
- **Geo metrics (0.43 W1/W3) + operator surface (0.42 W7)** exposed per-zone data.
  **Per-region staleness and replication-lag SLOs** are `0.44` W6.

## Dependency Graph

```
0.43 W1 zone/region placement ───────► W1 active-active multi-region writes
0.42 W4 MergePolicy + A5 versions ───► W2 conflict-free (CRDT) value types
0.41 B5 async backups + 0.43 W3 ─────► W3 WAN transport + cross-region anti-entropy
0.43 W6 self-heal + snapshot/restore ► W4 region failover / DR
0.43 W2 online resharding ───────────► W5 capacity signals + autoscaler hooks
0.42 W7 operator surface ────────────► W6 geo observability + staleness SLOs
W1 (regions writable) ───────────────► W2, W3, W4, W6   (cross-region writes are the headline)
```

W1 is the long pole: making more than one region accept writes is what creates
the cross-region conflict surface that W2 (convergence), W3 (propagation), W4
(failover), and W6 (staleness SLOs) all exist to manage.

---

## W1. Active-Active Multi-Region Writes with Bounded Staleness

**Problem / motivation.** Through `0.43`, exactly one region held write authority
for a key (zones within it provided HA). A write originating in a remote region had
to cross the WAN to that authority, paying full inter-region latency on every
write. Globally-distributed callers need each region to accept writes locally and
converge asynchronously, accepting a bounded, *known* staleness window between
regions in exchange.

**Design / contract.** Introduce a per-key **home region** (the Raft authority for
that key's partition) plus **active-active mode** where any region may accept a
local write, stamp it with `(version, epoch)` + a hybrid-logical-clock (HLC)
timestamp, apply it locally, and asynchronously replicate it to the home region and
peer regions. The home region's Raft remains the authority for ordering; remote
writes are reconciled there and the converged result flows back. Within a region,
the `0.42` W5 read-your-writes contract is unchanged; **across** regions the
contract is bounded staleness with a measured, SLO-tracked window (W6).
Active-active is opt-in per cache (`active_active(true)`) and named loudly; the
default stays single-authority `0.43` behavior. Plain values converge via the
`0.42` `MergePolicy`; CRDT classes (W2) converge without loss.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/geo.rs
pub enum WriteAuthority {
    HomeRegionOnly,                 // 0.43 default: one region owns writes
    ActiveActive { home: RegionId },// 0.44: any region writes locally, home reconciles
}

pub struct HybridLogicalClock { wall: u64, logical: u32, node: ClusterNodeId }

pub struct GeoWrite {
    pub key: CacheKey,
    pub version: ValueVersion,      // A5
    pub epoch: ClusterEpoch,        // authority
    pub hlc: HybridLogicalClock,    // cross-region tie-break, NOT wall-clock authority
    pub origin_region: RegionId,
}
```

**Step-by-step implementation.**

1. Add `RegionId` home assignment per partition (committed via Raft); add
   `WriteAuthority::ActiveActive { home }` opt-in mode.
2. On a local active-active write: apply locally, stamp `(version, epoch, hlc)`,
   enqueue cross-region replication (W3) to the home + peers.
3. At the home region, reconcile incoming remote writes through Raft into the
   authoritative order; flow the converged `(version, epoch)` back to all regions.
4. Keep intra-region `0.42` W5 read-your-writes unchanged; document cross-region
   reads as bounded-staleness and surface the window (W6).
5. Refuse to enable active-active without an explicit acknowledgement of the weaker
   cross-region contract (loud, like the `0.42`/`0.43` flags).

**Testing.** `crates/hydracache/tests/active_active.rs`

- `local_write_acks_without_crossing_wan` (integration): an active-active write
  acks at local-region quorum, no WAN round-trip on the ack path.
- `regions_converge_after_propagation` (integration): write in region A and B;
  after propagation assert both converge to the same `(version, epoch)`.
- `intra_region_ryow_unchanged_in_active_active` (**property**): the `0.42` W5
  contract holds within a region while active-active is on.
- `active_active_requires_explicit_ack` (unit): enabling without ack → refused
  loud; with ack → enabled + readiness flag.
- `hlc_tiebreak_is_not_wall_clock_authority` (unit): equal `(version, epoch)`, skewed
  clocks → deterministic HLC tie-break, authority still epoch/version.
- Run: `cargo test -p hydracache --locked active_active`.

**Pros.** Local-latency writes for globally-distributed callers; the staleness
trade is explicit, opt-in, and measured rather than hidden.

**Risks.** Active-active multiplies the conflict surface and is easy to misuse.
Mitigation: opt-in + loud ack, intra-region strong contract preserved, CRDT classes
(W2) for the high-conflict cases, and SLO monitoring (W6).

---

## W2. Conflict-Free Value Types (CRDTs) for Safe Classes

**Problem / motivation.** Under active-active (W1), two regions can write the same
key concurrently. The `0.42` `MergePolicy` resolves this by keeping the higher
`(version, epoch)` and **dropping the loser** — correct and documented, but lossy
for value classes where both writes should survive (a counter incremented in two
regions, a set with adds in two regions). Those classes need conflict-free merge.

**Design / contract.** Add an opt-in `ConflictFreeValue` trait and a small set of
built-in CRDT types layered on top of `(version, epoch)` + HLC: a grow-only and PN
counter, an OR-set (add/remove with tags), and an LWW-register (HLC tie-break).
A cache declares a key class as CRDT; cross-region merge for that class is the
type's `merge` (associative, commutative, idempotent), not the lossy
`MergePolicy`. CRDT merge never resurrects a tombstoned key (the A5 invariant still
dominates: a delete at a higher epoch beats a concurrent CRDT update). Plain
(non-CRDT) values keep the `0.42` `MergePolicy` unchanged.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/crdt.rs
pub trait ConflictFreeValue: Clone + Send + Sync {
    /// associative, commutative, idempotent
    fn merge(&mut self, other: &Self);
}

pub struct GCounter { per_node: BTreeMap<ClusterNodeId, u64> }
pub struct PnCounter { inc: GCounter, dec: GCounter }
pub struct OrSet<T> { adds: HashMap<T, HashSet<Tag>>, removes: HashSet<Tag> }
pub struct LwwRegister<T> { value: T, hlc: HybridLogicalClock }

impl ConflictFreeValue for GCounter {
    fn merge(&mut self, other: &Self) {
        for (n, v) in &other.per_node {
            let e = self.per_node.entry(*n).or_default();
            *e = (*e).max(*v); // element-wise max
        }
    }
}
```

**Step-by-step implementation.**

1. Add the `ConflictFreeValue` trait + `GCounter`/`PnCounter`/`OrSet`/`LwwRegister`.
2. Tag a key class as CRDT at the cache level; route its cross-region merge through
   `ConflictFreeValue::merge` instead of `MergePolicy`.
3. Enforce the A5 dominance rule: a higher-epoch tombstone beats a concurrent CRDT
   merge (delete wins; document the resurrection-prevention test).
4. Property-test each type for associativity, commutativity, idempotence under
   random region interleavings.
5. Export `crdt_merge_total` per type (bounded label set), `crdt_conflict_resolved_total`.

**Testing.** `crates/hydracache/tests/crdt.rs`

- `gcounter_merge_is_associative_commutative_idempotent` (**property**).
- `pn_counter_converges_across_regions` (integration): increment in A, decrement in
  B; assert both regions converge to the correct net value.
- `or_set_add_remove_converges` (**property**): concurrent add/remove → converges,
  no lost add that happens-after a remove tag.
- `lww_register_uses_hlc_not_wall_clock` (unit).
- `tombstone_beats_concurrent_crdt_update` (**property**): A5 dominance holds.
- `non_crdt_value_still_uses_merge_policy` (unit): plain values unchanged from 0.42.
- Run: `cargo test -p hydracache --locked crdt`.

**Pros.** Lossless convergence for the value classes that actually conflict under
active-active; keeps the lossy-but-simple `MergePolicy` for everything else.

**Risks.** CRDT metadata (OR-set tags, per-node counters) grows over time.
Mitigation: tag GC gated on cross-region anti-entropy confirmation (W3), mirroring
the A5 repair-gated tombstone GC; metadata size is a gauge.

---

## W3. WAN Replication Transport & Cross-Region Anti-Entropy

**Problem / motivation.** `0.41` B5 async backups and `0.43` W3 locality reads
assumed intra-region links: low latency, high bandwidth, low loss. Cross-region
links are the opposite — high latency, metered bandwidth, lossy. Replicating
per-write across regions naively wastes bandwidth and falls behind. `0.44` needs a
WAN-aware transport plus periodic cross-region anti-entropy to bound divergence.

**Design / contract.** Add a `RegionLink` transport that batches and compresses
cross-region replication, applies the `0.42` W3 adaptive flow control per link
(WAN links get a smaller window and longer hedge), and dedupes via the `0.37`
idempotency key so a retried batch is safe. Add periodic cross-region anti-entropy:
each region exchanges a compact digest (Merkle-style or version-vector summary) per
partition with peer regions and ships only the diff. Cross-region links reuse the
`0.42` W6 node identity/authz and may be encrypted via the `0.41`
`ReplicationKeyProvider`. Anti-entropy converges on `(version, epoch)` for plain
values and via `ConflictFreeValue::merge` for CRDT classes (W2); it never
resurrects a tombstone (A5).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/region_link.rs
pub struct RegionLink {
    peer: RegionId,
    window: AdaptiveWindow,          // 0.42 W3, WAN-tuned floor/ceil
    codec: BatchCodec,               // batch + compress
}

pub struct GeoBatch { entries: Vec<GeoWrite>, idem_keys: Vec<IdempotencyKey> } // 0.37

pub struct PartitionDigest { partition: PartitionId, summary: VersionSummary } // anti-entropy
```

**Step-by-step implementation.**

1. Add `RegionLink` batching + compression over `hydracache-cluster-transport-axum`,
   authed via `0.42` W6 and optionally sealed via the `0.41` key provider.
2. Apply per-link adaptive flow control (WAN-tuned); export `region_link_lag`,
   `region_link_window`, `region_link_bytes_total`.
3. Dedupe batches via the `0.37` idempotency key; a retried batch applies at-most-
   once-effect.
4. Add periodic cross-region anti-entropy: exchange `PartitionDigest`, ship only the
   diff, converge on `(version, epoch)` / CRDT merge, never resurrect a tombstone.
5. Gate CRDT-metadata GC (W2) on anti-entropy confirmation across all regions
   (repair-gated, like A5).

**Testing.** `crates/hydracache/tests/region_link.rs`

- `batch_is_compressed_and_deduped` (integration): assert batching + that a replayed
  batch does not double-apply.
- `wan_link_backpressure_bounds_inflight` (integration): inject WAN latency/loss;
  assert the per-link window shrinks and lag stays bounded.
- `anti_entropy_ships_only_the_diff` (integration): diverge two regions slightly;
  assert only changed entries cross the link.
- `cross_region_converges_after_partition_heal` (**chaos**, `#[ignore]`): seeded
  region partition; on heal assert convergence with no tombstone resurrection.
- `crdt_metadata_gc_gated_on_all_region_confirmation` (**property**): ties to W2/A5.
- Run: `cargo test -p hydracache --locked region_link` and chaos with `-- --ignored`.

**Pros.** Cross-region replication stays within metered bandwidth and bounded lag;
divergence is repaired by anti-entropy rather than per-write fan-out.

**Risks.** Batching adds latency to cross-region visibility (the staleness window).
Mitigation: window is a tunable SLO input (W6); urgent invalidations can bypass
batching with a high-priority lane.

---

## W4. Region Failover & Disaster Recovery

**Problem / motivation.** `0.43` W6 recovered within a region (auto-repair, control-
plane snapshot/restore) and `0.43` W1 survived a *zone* loss. Losing a whole region
(authority for its home partitions) is a new failure mode that active-active opens:
the home region for some keys can vanish. `0.44` needs to fail those keys over to a
surviving region and recover a lost region from DR snapshots.

**Design / contract.** When a region is declared down (operator or detector), a
surviving region is promoted to home for the affected partitions through Raft
(reusing the `0.42` W4 split-brain detection so a flapping region cannot
double-promote). Promotion is a topology op (like `0.42` W4 backup promotion,
generalized to regions): freeze cross-region authority for the partition, commit the
new home, converge via W3 anti-entropy, unfreeze. DR: a region rebuilds from the
`0.43` W6 control-plane snapshot + W2-0.42 durable values shipped to the operator's
`SnapshotSink`, then rejoins and back-fills via W3. A region that was partitioned
(not dead) and rejoins with a lower epoch loses authority (A1 fence + `0.42` W4
merge), never double-writes.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/region_failover.rs
pub enum RegionState { Up, Suspect, Down }

pub struct RegionPromotion {
    pub partitions: Vec<PartitionId>,
    pub from_home: RegionId,
    pub to_home: RegionId,            // surviving region, via Raft CommitTopology
}

pub struct RegionRestore { from: ControlPlaneSnapshot, values: Box<dyn ReplicatedValueStore> }
```

**Step-by-step implementation.**

1. Add `RegionState` detection (operator-declared + a conservative detector that
   never auto-downs on a transient blip; uses `0.42` W4 to avoid double-promotion).
2. Promote a surviving region to home via Raft `CommitTopology`: freeze → commit →
   converge (W3) → unfreeze; emit a degraded report if no surviving home holds the
   data.
3. Implement DR restore from the `0.43` W6 snapshot + durable values; rejoin via W3
   anti-entropy.
4. Enforce that a rejoining lower-epoch region loses authority (A1 fence + `0.42` W4
   merge) — never resurrects stale homes or double-writes.
5. Export `region_state`, `region_promotion_total`, `region_restore_duration` (HLC-
   free, logical where it gates correctness).

**Testing.** `crates/hydracache/tests/region_failover.rs`

- `region_down_promotes_surviving_home` (integration): down a region; assert its home
  partitions get a new home and stay writable.
- `flapping_region_does_not_double_promote` (**property**): ties to `0.42` W4.
- `rejoining_lower_epoch_region_loses_authority` (integration): partition + rejoin;
  assert A1 fence + merge, no double-write.
- `region_restore_rebuilds_from_snapshot` (integration): wipe a region, restore from
  `SnapshotSink`, rejoin; assert convergence.
- `whole_region_loss_self_heals_to_target_rf` (**chaos**, `#[ignore]`): drop a region
  under load; assert recovery within bounds, no committed loss for surviving-quorum
  keys.
- Run: `cargo test -p hydracache --locked region_failover` and chaos with
  `-- --ignored`.

**Pros.** A region outage becomes a recoverable, bounded event rather than data
loss; rejoining regions cannot corrupt authority.

**Risks.** Aggressive region-down detection can cause spurious promotions (a costly
mistake at region scale). Mitigation: conservative detector + operator confirmation
default + `0.42` W4 anti-double-promote.

---

## W5. Capacity Signals & Autoscaler Integration

**Problem / motivation.** `0.43` W2 made resharding online and safe but
operator/plan-triggered: a human decided *when* to add or drain nodes. At
multi-region scale with shifting load, the grid should emit capacity signals that an
external autoscaler can act on, and accept scale decisions that drive the W2-0.43
online reshard automatically — without HydraCache itself owning cloud APIs.

**Design / contract.** Add a `CapacitySignal` surface: per-region/per-node load
(memory pressure vs the `0.37` budgets and W4-0.43 tier hot-ratio, replication lag,
hot-partition skew, repair-debt) exported both as metrics (W6) and as a structured
recommendation (`scale_out` / `scale_in` / `rebalance`). HydraCache does **not** call
cloud APIs; it exposes the signal and a guarded admission endpoint so an external
autoscaler (or operator) can request a membership change, which is validated
(zone-spread W1, quorum, COMPAT) and executed through the `0.43` W2 online reshard.
Scale-in drains a node via W2 before removal; scale-out backfills before counting
the node toward quorum.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/capacity.rs
pub enum ScaleRecommendation { Hold, ScaleOut { suggested: usize }, ScaleIn { drain: Vec<ClusterNodeId> }, Rebalance }

pub struct CapacitySignal {
    pub region: RegionId,
    pub memory_pressure: f32,        // vs 0.37 budgets + 0.43 W4 hot ratio
    pub replication_lag: u64,
    pub hot_partition_skew: f32,
    pub recommendation: ScaleRecommendation,
}

// guarded admission: external autoscaler POSTs a membership intent ->
// validated (W1 zone-spread, quorum, COMPAT) -> 0.43 W2 reshard.
```

**Step-by-step implementation.**

1. Compute `CapacitySignal` from existing gauges (memory/tier/lag/skew/debt) per
   region/node; expose via the `0.42` W7 status + metrics (W6).
2. Add a guarded membership-intent endpoint (authed via `0.42` W6) that validates
   zone-spread (W1), quorum, and COMPAT before accepting.
3. On accepted scale-out: join + backfill via `0.43` W2, count toward quorum only
   after backfill; on scale-in: drain via W2 then remove cleanly.
4. Never let an autoscaler request violate a `0.44`/`0.43` invariant — reject loud
   with the reason (zone co-location, quorum break, COMPAT mismatch).
5. Export `capacity_recommendation`, `scale_actions_total` (bounded labels).

**Testing.** `crates/hydracache/tests/capacity_autoscale.rs`

- `signal_recommends_scale_out_under_memory_pressure` (unit).
- `scale_out_counts_toward_quorum_only_after_backfill` (integration).
- `scale_in_drains_before_removal` (integration): ties to `0.43` W2.
- `autoscaler_intent_violating_zone_spread_is_rejected` (unit): ties to W1.
- `intent_outside_compat_window_is_refused` (unit): ties to `0.43` W6 upgrade guard.
- Run: `cargo test -p hydracache --locked capacity_autoscale`.

**Pros.** The grid becomes autoscaler-friendly without owning cloud APIs or
violating placement/consistency invariants; scale events reuse the proven `0.43` W2
path.

**Risks.** An autoscaler can thrash (scale out/in rapidly). Mitigation: hysteresis
in the recommendation + a minimum dwell time + the W2 rate-limit so churn never
overruns the hot path.

---

## W6. Geo Observability & Staleness SLOs

**Problem / motivation.** Active-active (W1) trades strong cross-region consistency
for bounded staleness — a trade that is only acceptable if the staleness is
*measured* and alertable. `0.43` W1/W3 exposed per-zone data and `0.42` W7 gave an
operator surface, but there is no per-region staleness SLO, replication-lag SLO, or
region-health view. `0.44` adds them so operators can see and alert on the contract
W1 actually delivers.

**Design / contract.** Extend the `0.42` W7 read-only status with a geo view:
per-region state (W4), per-link replication lag and bytes (W3), per-region
convergence/staleness window (the observed time between a write in region A and its
visibility in region B), CRDT metadata size (W2), and the active-active
acknowledgement state (W1). Define staleness as an SLO with a target window and emit
an alert when the observed window exceeds it. All per-region/per-link series obey the
`0.41` cardinality rule (region and link id are bounded labels at small region
counts; per-partition/per-key detail stays in the snapshot). Ship geo dashboards and
alert rules as artifacts under `docs/cluster/dashboards/geo/`, wired to the exported
metric names and tested for drift (like `0.42` W7).

**Rust sketch.**

```rust
// crates/hydracache-observability/src/geo_status.rs
pub struct GeoStatus {
    pub regions: Vec<RegionHealth>,        // state, lag, staleness window
    pub links: Vec<LinkHealth>,            // bytes, window, backpressure
    pub active_active_acked: bool,         // W1 ack state
    pub worst_staleness_window_ms: u64,    // SLO-tracked aggregate, bounded labels
    pub crdt_metadata_bytes: u64,          // W2 growth watch
}
// GET /cluster/geo -> GeoStatus (read-only); SLO breach -> alert rule.
```

**Step-by-step implementation.**

1. Add `GeoStatus` assembled from W1/W3/W4 signals; expose read-only
   `GET /cluster/geo` (authed via `0.42` W6).
2. Define the staleness SLO (target window) per link/region; export
   `region_staleness_window_ms`, `region_link_lag`, `region_state` as bounded-label
   series; per-key/per-partition detail only in the snapshot.
3. Ship `docs/cluster/dashboards/geo/` Prometheus alert rules (staleness breach,
   link backpressure, region down, CRDT metadata growth) + a Grafana JSON.
4. Add a test asserting every shipped alert rule references a registered metric
   (drift guard, like `0.42` W7).
5. Extend the runbook with active-active operations: reading staleness, responding
   to a breach, region failover (W4), and disabling active-active safely.

**Testing.** `crates/hydracache-observability/tests/geo_observability.rs`

- `geo_status_is_read_only_and_complete` (integration): every documented field
  present, no mutation.
- `staleness_window_is_measured_and_breach_alerts` (integration): force lag past the
  SLO; assert the window reflects it and the breach condition fires.
- `geo_series_honor_cardinality_rule` (unit): no per-partition/per-key label; region/
  link are bounded.
- `geo_alert_rules_reference_existing_metrics` (unit): drift guard.
- Run: `cargo test -p hydracache-observability --locked geo_observability`.

**Pros.** Makes the active-active staleness trade visible and alertable — the trade
is only safe if operators can see it; closes the loop from SLO breach to runbook
action.

**Risks.** Staleness measurement itself adds cross-region probe traffic. Mitigation:
piggy-back on the W3 anti-entropy digests rather than dedicated probes.

---

## Deferred To 0.45+ (Explicit)

- **Full distributed transactions** (serializable cross-node/cross-region multi-key
  commit). Still a hard non-goal; `0.44` adds no transaction semantics.
- **Causal+ / session-guarantee consistency across regions** (read-your-writes,
  monotonic reads spanning regions). `0.44` gives intra-region strong + cross-region
  bounded-staleness; formal cross-region session guarantees are future work.
- **Automatic region placement / latency-based home assignment.** `0.44` assigns
  home regions explicitly; auto-placing homes by observed traffic is deferred.
- **Cloud-provider-native autoscaler controllers.** `0.44` emits capacity signals +
  a guarded admission endpoint; shipping provider-specific controllers is out of
  scope.

## Fault Model and Test Tiering

`0.44` reuses the `0.41`–`0.43` shared fault model and harness verbatim
(`crates/hydracache/tests/support/fault_injector.rs`) and its determinism contract
(seeded, replayable, logical-signal assertions — never wall-clock pass/fail). It
**adds**: whole-region loss, cross-region partition (region A↔B link down while each
stays internally healthy), and metered/lossy WAN link (high latency + bandwidth cap +
loss) to drive W1/W3/W4. Cross-region clock skew is injected to stress HLC tie-break
but is never a correctness source; authority stays epoch/version.

| Tier | Scope | When | Command shape |
| --- | --- | --- | --- |
| fast | unit + deterministic property (CRDT laws, HLC tie-break, capacity recs, cardinality) | every PR | `cargo test --workspace --locked` |
| integration | in-memory multi-region, active-active convergence, region link, failover, autoscale intent | every PR | `cargo test --workspace --locked` |
| chaos/soak | seeded region loss, cross-region partition heal, lossy WAN, active-active churn | nightly / pre-release | `cargo test --workspace --locked -- --ignored` |
| Docker | testcontainers multi-process across simulated regions | nightly / pre-release | Docker gate |

## Release Gates For 0.44

Focused:

```powershell
cargo test -p hydracache --locked active_active
cargo test -p hydracache --locked crdt
cargo test -p hydracache --locked region_link
cargo test -p hydracache --locked region_failover
cargo test -p hydracache --locked capacity_autoscale
cargo test -p hydracache-observability --locked geo_observability
cargo test -p hydracache --locked fault_injector_selftest
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --locked --features durable-log,durable-values,tiered-values,active-active
cargo test --workspace --locked -- --ignored   # region-loss / WAN / active-active chaos suites
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.44.0` may claim **active-active multi-region grid with bounded staleness** (for
the supported topology) only if **all** of the following boolean conditions hold:

- W1: multiple regions accept local writes; intra-region `0.42` W5 read-your-writes
  is unchanged; cross-region is bounded-staleness, opt-in, and refused without an
  explicit ack; regions converge after propagation; `active_active` passes.
- W2: opt-in CRDT classes (counters, OR-set, LWW-register) converge conflict-free
  (associative/commutative/idempotent), a higher-epoch tombstone beats a concurrent
  CRDT update, and plain values still use the `0.42` `MergePolicy`; `crdt` passes.
- W3: the WAN transport batches/compresses/dedupes, applies per-link adaptive
  backpressure, and cross-region anti-entropy ships only the diff and converges with
  no tombstone resurrection; `region_link` passes (incl. chaos).
- W4: a region loss promotes a surviving home via Raft without double-promotion, a
  rejoining lower-epoch region loses authority, and DR restore rebuilds from
  snapshot; `region_failover` passes (incl. region-loss chaos).
- W5: capacity signals are exported, the guarded admission endpoint validates
  zone-spread/quorum/COMPAT before any scale action, and scale-out/in reuse the
  `0.43` W2 reshard path; `capacity_autoscale` passes.
- W6: the read-only geo status + staleness SLO + shipped alerts exist, alert rules
  reference only registered metrics, and series honor the cardinality rule;
  `geo_observability` passes.
- The fault model adds region loss, cross-region partition, and lossy WAN, and all
  those chaos suites pass.
- Docs keep the prominent **"still not distributed transactions"** warning, document
  the cross-region bounded-staleness contract and where it differs from intra-region
  strong RYOW, and list causal+ cross-region session guarantees / auto home placement
  as deferred.

If any condition fails, `0.44.0` ships **without** the corresponding claim,
documents exactly which work item(s) did not land, and the claim moves to a later
release.
