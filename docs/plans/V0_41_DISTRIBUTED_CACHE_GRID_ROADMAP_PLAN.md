# HydraCache 0.41.0 Distributed Cache Grid Roadmap Plan

> **At a glance**
> - **What:** ADRs, epoch fence, `RaftLogStore` trait, `ClusterReplicationStrategy`, rebalance-as-data, versioned tombstones, opt-in value-replication prototype, B-items.
> - **Why:** lay the correctness **skeleton** for a distributed grid without claiming production-grid yet.
> - **After (depends on):** 0.40.
> - **Unblocks:** 0.42 (turns these prototypes into supported durable features).
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md)

Status: implemented in `0.41.0`. Release notes:
[`docs/releases/0.41.0.md`](../releases/0.41.0.md).

`0.41.0` defines the roadmap and the first *safe* implementation slice toward a
production distributed cache grid. It is deliberately a roadmap release: a true
production data grid spans multiple releases and changes the product class.
HydraCache today is an embedded local-first cache with DB query-result caching
adapters and cluster coordination (Local/Client/Member roles, chitchat gossip
discovery, a single-node in-memory raft metadata runtime, HTTP peer-fetch /
owner-load, an in-memory invalidation bus, and deterministic rendezvous
ownership with no value replication or backup owners). This release lays the
correctness skeleton (durable control plane, replication strategy, rebalance
plan-as-data, versioned tombstones) plus the dissemination/runtime mechanisms
(near-cache repair, backup promotion, anti-entropy) needed before any future
release may claim production-grid readiness.

Readiness is described in prose and asserted as boolean release gates. There is
no numeric self-score. After 0.41.0, production distributed-data-grid readiness
is explicitly **not** complete: this release delivers the roadmap and the first
safe slice only.

## Release Theme

Define the path to a production data grid without accidentally claiming it too
early, and ship the first correctness-preserving slice.

The release follows one resolution rule taken from the architecture review:

> **Authority** (who owns a key, which topology is valid, which tombstone
> version is newer) is the ScyllaDB model: Raft + monotonic epoch. **Dissemination**
> (how staleness is detected and propagated to clients) is the Hazelcast model:
> sequence/UUID stamps. When the two disagree, the epoch (authority) wins; the
> stamp is only a hint that triggers a conservative refresh/invalidate.

Concretely this release:

- finalizes the architecture decision records started in 0.37 and references
  them from cluster readiness docs (release gates assert they exist);
- enforces epoch fencing so gossip proposes liveness and Raft commits topology
  (A1);
- replaces the in-memory raft log (`raft::storage::MemStorage`) with a durable
  `RaftLogStore` trait plus an in-memory fake and one feature-flagged engine
  example (A2);
- generalizes `RendezvousClusterOwnership` from top-1 to top-N to produce a
  `ClusterReplicationStrategy` with primary + backups and an
  `EffectiveReplicationMap` (A3);
- makes rebalance a plan-as-data computed by a single coordinator and executed
  through Raft, never via gossip-driven moves (A4);
- introduces versioned tombstones whose GC is gated on repair confirmation (A5);
- ships an opt-in, narrow value-replication prototype with mandatory byte caps
  and backpressure counters;
- adds the Hazelcast runtime mechanisms: full near-cache `RepairingTask` (B1),
  three-phase backup promotion (B4), tunable sync/async backup count (B5), and
  per-replica anti-entropy (B6);
- splits three-part counters (owner-load / remote-fetch / hot-cache-hit) and
  makes hot-cache invalidation authoritative over TTL;
- defines the test matrix required before any future production-grid claim.

## Non-Goals

- **No production data-grid claim** in `0.41.0`. Docs and release notes must
  state production-grid readiness is incomplete.
- **No arbitrary remote code execution**: no remote SQL/expression evaluation,
  no remote load closures over the wire.
- **Replication is not mandatory for local users**: `replicate_values(false)`
  (the default) keeps current local-first behavior byte-for-byte.
- **No distributed transactions** in this slice.
- **No universal topology**: one supported topology (rendezvous over admitted
  members with partition indirection); not every layout at once.
- **No split-brain auto-merge**: minority fencing via epoch (A1), not merge.
- **No silent consistency claims**: every consistency limitation is documented.

## Architecture Decision Records (Early, Finalized Here)

ADRs are inputs to the code, not afterthoughts. The skeleton was started in 0.37
and populated across 0.37–0.40; 0.41 **finalizes** the following set and links
them from `docs/cluster/readiness.md`. Release gates assert each ADR file exists
and is referenced.

ADR files live under `docs/adr/`:

- `docs/adr/0001-gossip-liveness-vs-raft-topology.md` — **ADR-1**: gossip carries
  liveness only; Raft is the authoritative source of topology, with version/epoch
  fencing. Gossip may mark a node `suspect`; ownership changes only after a Raft
  `CommitTopology`. Backs A1.
- `docs/adr/0002-raft-log-store-durability-contract.md` — **ADR-2**: the
  durability contract for `RaftLogStore`: what persists (HardState, log entries,
  snapshot), the write order (snapshot → entries → HardState), the fsync policy
  (`must_sync`), and atomic snapshot/compaction that never compacts past the
  applied/snapshot index. Backs A2.
- `docs/adr/0003-replication-strategy-and-effective-map.md` — **ADR-3**: the
  `ClusterReplicationStrategy` trait and the `EffectiveReplicationMap`
  (`natural` / `reading` / `pending`) that separates "how many copies" from "who
  currently owns". Backs A3.
- `docs/adr/0004-rebalance-plan-as-data.md` — **ADR-4**: rebalance is a plan
  materialized as data and executed by a single coordinator through Raft; no
  competing gossip-driven movement path. Backs A4.
- `docs/adr/0005-tombstone-gc-vs-repair-boundary.md` — **ADR-5**: versioned
  tombstones, and the rule that tombstone GC is permitted only after repair
  confirms the deletion on all backups. Backs A5.

### Pros

- Prevents accidental architecture drift; production claims become reviewable.
- Separates embedded-cache behavior from distributed-storage behavior.

### Testing

ADRs are docs; they need no runtime tests. But a gate test in
`crates/hydracache/tests/adr_presence.rs` asserts the five files exist and that
`docs/cluster/readiness.md` links each one.

- Test: `adr_files_exist_and_are_linked` (integration).
- Assertions: each `docs/adr/000{1..5}-*.md` path exists; `readiness.md`
  contains each filename.
- Run: `cargo test -p hydracache --locked adr_presence`.

---

## A-Items (Correctness Skeleton, from ScyllaDB)

### A1. Epoch Fence: Gossip = Liveness, Raft = Authoritative Topology (full)

**Problem / motivation.** Gossip membership (chitchat) and the raft metadata
runtime are not yet bound by the contract "Raft decides, gossip only hints
liveness". Without it, a gossip flap can flip ownership, causing an avalanche of
re-replication. The 0.40 pilot's restart/rejoin risk needs a cheap minimal fence;
0.41 finalizes the full Raft-committed topology fence.

**Design / contract.** Extend `RaftMetadataCommand` with a `CommitTopology`
variant carrying the new epoch and the committed member set. Introduce a
`TopologyFence` that admits a message only if its epoch is at least the committed
epoch. Gossip may propose `suspect` for a vanished node, but a node is removed
from the owner set only after `CommitTopology` commits through Raft.

`ClusterEpoch` (cluster.rs:103) and `ClusterGeneration` (cluster.rs:79) already
exist; this wires them into the authority path.

**Rust sketch.**

```rust
// crates/hydracache-cluster-raft/src/lib.rs
pub enum RaftMetadataCommand {
    MemberUpsert { node_id: ClusterNodeId, generation: ClusterGeneration, epoch: ClusterEpoch },
    ClientUpsert { node_id: ClusterNodeId, generation: ClusterGeneration, epoch: ClusterEpoch },
    NodeLeft { node_id: ClusterNodeId, role: ClusterRole, epoch: ClusterEpoch },
    /// New in 0.41: the authoritative committed topology.
    CommitTopology {
        epoch: ClusterEpoch,
        members: Vec<ClusterNodeId>,
    },
}

// crates/hydracache/src/cluster.rs
/// Drops decisions/messages stamped with a stale (pre-committed) epoch.
#[derive(Debug, Clone, Copy)]
pub struct TopologyFence {
    pub committed_epoch: ClusterEpoch,
}

impl TopologyFence {
    /// Admit only messages at or after the committed epoch.
    pub fn admit(&self, message_epoch: ClusterEpoch) -> bool {
        message_epoch.value() >= self.committed_epoch.value()
    }
}
```

**Step-by-step implementation.**

1. Add `CommitTopology` to `RaftMetadataCommand` and apply it in the raft state
   machine so the materialized snapshot advances `ClusterEpoch` and replaces the
   admitted member set atomically.
2. Add `TopologyFence` to `cluster.rs`, constructed from the latest committed
   epoch exposed by the raft runtime export.
3. Route gossip membership events through a `suspect` set that does **not** feed
   `owner_for_key` (cluster.rs:2500) until `CommitTopology` commits.
4. Filter inbound invalidation/replication frames through `TopologyFence::admit`
   using the frame's `source_generation`/epoch context; drop stale-epoch frames.
5. Increment a `topology_fence_rejected_total` counter on drop.

**Testing.** `crates/hydracache/tests/topology_fence.rs`

- `stale_epoch_message_is_dropped` (unit): build a `TopologyFence { committed_epoch: epoch(5) }`; assert `!fence.admit(epoch(4))` and `fence.admit(epoch(5))`.
- `gossip_suspect_does_not_change_owner` (integration): mark a node suspect via
  gossip; assert `owner_for_key("user:42")` is unchanged until a `CommitTopology`
  is applied; then assert it changes deterministically.
- `late_packet_from_old_leader_does_not_resurrect_topology` (integration): apply
  `CommitTopology{epoch:6}`, then replay a stale `CommitTopology{epoch:5}`; assert
  the committed member set stays at epoch 6.
- `committed_topology_owner_set_is_deterministic` (property): for any committed
  member set, `owner_for_key` is stable across repeated resolution.
- Run: `cargo test -p hydracache --locked topology_fence`.

**Pros.** Removes the gossip-flap → ownership-flap → re-replication avalanche;
makes the consistency claim checkable.

**Risks.** Couples fast gossip to slow Raft. Mitigation: the fence applies only
to authority decisions (ownership/replication), never to liveness detection
itself.

---

### A2. `RaftLogStore` Replacing `MemStorage` (durable control plane)

**Problem / motivation.** `RaftMetadataRuntime` (hydracache-cluster-raft/src/lib.rs:355)
uses `raft::storage::MemStorage` — the log lives in memory, the durability gap.
`RaftMetadataStore` (line ~270) stores a *materialized snapshot*, not the raft
log. Production grid readiness needs a durable, recoverable control plane.
We do **not** pick a storage engine in 0.41: we ship a trait, an in-memory fake,
and exactly one feature-flagged engine example.

**Design / contract (ADR-2).** A single type implements both the read side
(`raft::Storage`) and the persist side (`RaftLogStore`). Persist HardState, log
entries (append overwrites from `entries[0].index`), and snapshots. In the Ready
loop the write order is **snapshot → entries → HardState**, and outbound messages
are sent only after fsync when `must_sync()` is true. Snapshot and compaction are
atomic; never compact past the applied/snapshot index; `SnapshotTemporarilyUnavailable`
is allowed.

**Rust sketch.**

```rust
// crates/hydracache-cluster-raft/src/log_store.rs
use raft::eraftpb::{Entry, HardState, Snapshot};

pub trait RaftLogStore: raft::Storage + Send + Sync {
    /// Persist term/vote/commit.
    fn save_hard_state(&self, hs: &HardState) -> RaftStoreResult<()>;
    /// Append, overwriting any existing entries from `entries[0].index`.
    fn append(&self, entries: &[Entry]) -> RaftStoreResult<()>;
    /// Drop the conflicting suffix at and after `from_index`.
    fn truncate_suffix(&self, from_index: u64) -> RaftStoreResult<()>;
    /// Atomically install a snapshot, preserving `preserve_log_entries` trailing entries.
    fn save_snapshot(&self, snap: &Snapshot, preserve_log_entries: usize) -> RaftStoreResult<()>;
    /// Compact (drop the prefix) up to `index`; never past applied/snapshot index.
    fn compact_to(&self, index: u64) -> RaftStoreResult<()>;
}

/// Deterministic in-memory fake for tests; not durable.
#[derive(Debug, Default)]
pub struct InMemoryRaftLogStore { /* entries, hard_state, snapshot */ }

#[cfg(feature = "sled-log-store")]
pub struct SledRaftLogStore { /* one durable example behind a feature flag */ }
```

**Step-by-step implementation.**

1. Add `crates/hydracache-cluster-raft/src/log_store.rs` defining
   `RaftLogStore` and `RaftStoreResult`.
2. Implement `InMemoryRaftLogStore` implementing both `raft::Storage` and
   `RaftLogStore`. Enforce append overwrite-from-index and compaction guards.
3. Rewire `RaftMetadataRuntime` (line 355) to take a `RaftLogStore` instead of
   `MemStorage`; default to `InMemoryRaftLogStore`.
4. In the Ready loop persist in order snapshot → entries → HardState, fsync when
   `must_sync()`, then send outbound messages.
5. Track command ids so duplicate commands replayed from the log are applied at
   most once (idempotent apply against the materialized snapshot).
6. Add `SledRaftLogStore` behind `#[cfg(feature = "sled-log-store")]` as the one
   concrete durable example. Do not make it the default.

**Testing.** `crates/hydracache-cluster-raft/tests/persistent_log.rs`

- `append_then_replay_restores_log_exactly` (integration): append N entries,
  drop and rebuild the runtime from the same store, assert log entries and last
  index match 1:1.
- `snapshot_recovery_after_restart` (integration): install a snapshot, restart,
  assert `initial_state` reflects the snapshot's index/term/ConfState and that
  the materialized cluster epoch survives.
- `duplicate_command_id_is_idempotent_after_replay` (integration): apply a
  `MemberUpsert` with a fixed command id, replay it from the log, assert the
  member count increments exactly once.
- `truncate_suffix_drops_conflicting_tail` (unit): append 1..10, `truncate_suffix(7)`,
  assert last index is 6.
- `compact_never_passes_applied_index` (unit): assert `compact_to(applied+1)`
  returns an error.
- `snapshot_temporarily_unavailable_is_allowed` (unit): assert the store may
  return `SnapshotTemporarilyUnavailable` without panicking the loop.
- Feature-flagged example: `cargo test -p hydracache-cluster-raft --locked --features sled-log-store persistent_log`.
- Run: `cargo test -p hydracache-cluster-raft --locked persistent_log`.

**Pros.** Removes the main durability blocker; makes control-plane recovery a
real, testable claim.

**Risks.** Durable Raft is large and dangerous; wrong integration is worse than
none. Storage-engine choice affects portability — hence trait + fake + one
example only.

---

### A3. `ClusterReplicationStrategy` + `EffectiveReplicationMap`

**Problem / motivation.** `ClusterOwnershipResolver` (cluster.rs:557) and
`RendezvousClusterOwnership` (cluster.rs:570) resolve exactly one owner via FNV
`rendezvous_score`. There are no backup owners and no pending map. The grid needs
primary + backups and a separation of natural vs in-flight owners.

**Design / contract (ADR-3).** Generalize the same FNV score ranking from top-1
to top-N — **no algorithm change**, preserving determinism. Produce `Replicas {
primary, backups }` (backups = ranking `[1..replication_factor]`). Freeze the
result into `EffectiveReplicationMap { natural, reading, pending }`: `natural` is
the current committed placement; `pending` is the in-flight placement during a
move; `reading` covers both during the move window. Validate config at startup
(olric invariants): `min_replica = 1`; reject `quorum > replication_factor` and
`quorum <= 0`.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster.rs
pub trait ClusterReplicationStrategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn replicas_for_key(&self, key: &str, members: &[ClusterMember]) -> Replicas;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replicas {
    pub primary: ClusterNodeId,
    pub backups: Vec<ClusterNodeId>, // rendezvous(key)[1..replication_factor]
}

#[derive(Debug, Clone)]
pub struct EffectiveReplicationMap {
    pub natural: Replicas,           // current committed topology
    pub reading: Vec<ClusterNodeId>, // union of natural + pending during a move
    pub pending: Option<Replicas>,   // in-flight ownership during rebalance
}

#[derive(Debug, Clone, Copy)]
pub struct ReplicationConfig {
    pub replication_factor: usize,   // >= 1
    pub read_quorum: usize,          // 1..=replication_factor
    pub write_quorum: usize,         // 1..=replication_factor
}

impl ReplicationConfig {
    pub fn validate(self) -> Result<(), ReplicationConfigError> {
        if self.replication_factor < 1 { return Err(ReplicationConfigError::MinReplica); }
        if self.read_quorum == 0 || self.write_quorum == 0 { return Err(ReplicationConfigError::QuorumZero); }
        if self.read_quorum > self.replication_factor || self.write_quorum > self.replication_factor {
            return Err(ReplicationConfigError::QuorumTooLarge);
        }
        Ok(())
    }
}
```

`RendezvousClusterOwnership` gains a `replicas_for_key` that collects the top-N
distinct members by the existing `rendezvous_score` ranking; `resolve_owner`
stays as the top-1 special case.

**Step-by-step implementation.**

1. Add `ClusterReplicationStrategy`, `Replicas`, `EffectiveReplicationMap`,
   `ReplicationConfig`, `ReplicationConfigError` to `cluster.rs`.
2. Implement `replicas_for_key` on `RendezvousClusterOwnership` by ranking all
   members by `rendezvous_score`, taking the top `replication_factor` distinct
   node ids; degrade to fewer when members < RF.
3. Build `EffectiveReplicationMap` from the committed topology (`natural`) and,
   during a rebalance (A4), set `pending` and compute `reading`.
4. Validate `ReplicationConfig` in the builder `.start()` path; return a clear
   error before any networking starts.
5. Surface placement via `placement_for_key(key) -> EffectiveReplicationMap` in
   diagnostics.

**Testing.** `crates/hydracache/tests/placement.rs`

- `placement_deterministic_for_same_member_set` (unit): equal member sets yield
  equal `Replicas`.
- `no_duplicate_backup_owners` (unit): assert `primary` not in `backups` and
  `backups` has no duplicates.
- `replication_factor_exceeding_members_degrades_clearly` (unit): RF=5, members=2
  → 1 primary + 1 backup, no panic.
- `placement_changes_predictably_on_join_leave` (integration): adding/removing a
  member changes only the affected placements.
- `placement_distribution_is_even` (property): over 10k keys, per-member primary
  share is within a tolerance band.
- `pending_map_reads_both_during_move` (integration): during a simulated move,
  `reading` contains both natural and pending owners.
- `quorum_validation_rejects_bad_config` (unit): `quorum=0` and
  `quorum>replication_factor` return the right `ReplicationConfigError`.
- Run: `cargo test -p hydracache --locked placement`.

**Pros.** Implements the `placement_for_key` contract; separates copy count from
current ownership.

**Risks.** RF increases memory/network; topology churn creates re-replication
traffic (mitigated by A1/A4).

---

### A4. Rebalance as Plan-as-Data + Single Coordinator

**Problem / motivation.** Membership events (`ClusterMembershipEvent`) are not
yet expressed as a plan. With backup replication (A3) this becomes mandatory:
ad-hoc, gossip-driven moves race and are non-deterministic.

**Design / contract (ADR-4).** On `CommitTopology`, the raft leader (the single
coordinator) computes the diff between the old and new `EffectiveReplicationMap`,
materializes a list of move/re-replication tasks into raft state, and publishes
them. Executors ack completion. No moves happen outside the committed plan. Use
**partition indirection** over rendezvous (olric): keys map to partitions, moves
relocate whole partitions, and each partition keeps an append-only owner history
for hand-off.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebalanceTask {
    MovePartition { partition: PartitionId, from: ClusterNodeId, to: ClusterNodeId },
    ReReplicate { partition: PartitionId, target: ClusterNodeId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebalancePlan {
    pub epoch: ClusterEpoch,
    pub tasks: Vec<RebalanceTask>,
}

pub fn diff_effective_maps(
    old: &EffectiveReplicationMap,
    new: &EffectiveReplicationMap,
) -> Vec<RebalanceTask> { /* deterministic diff */ }
```

The plan is committed via a new `RaftMetadataCommand::CommitRebalancePlan` and
executors record acks via `RebalanceTaskAck`.

**Step-by-step implementation.**

1. Add partition indirection: `partition_for_key(key) -> PartitionId` (hash mod
   partition count); placement resolves at the partition level.
2. Add `RebalanceTask`, `RebalancePlan`, `diff_effective_maps`.
3. On `CommitTopology`, the leader computes the plan and commits
   `CommitRebalancePlan` through Raft (depends on A2 for durability).
4. Executors apply tasks (re-replicate / hand off via owner history) and commit
   `RebalanceTaskAck`.
5. Report `under_replicated_partitions` until all acks land.

**Testing.** `crates/hydracache/tests/rebalance.rs`

- `diff_produces_expected_move_tasks` (unit): two maps differing by one owner →
  exactly the expected `MovePartition`/`ReReplicate` set.
- `concurrent_membership_changes_yield_single_plan` (integration): two membership
  events between commits produce one coordinator plan, no competing plans.
- `replaying_plan_is_idempotent` (integration): re-applying a committed plan does
  not duplicate tasks or moves.
- `under_replication_reported_until_plan_completes` (integration): before all
  acks, `under_replicated_partitions > 0`; after, it is 0.
- Run: `cargo test -p hydracache --locked rebalance`.

**Pros.** Deterministic, observable rebalance; removes movement races.

**Risks.** Depends on A2/A3; without the durable log a plan cannot survive a
coordinator restart.

---

### A5. Versioned Tombstones with Repair-Gated GC

**Problem / motivation.** This is the central correctness invariant of the
release: **invalidation during repair beats stale value replication**. olric's
gap is exactly the absence of tombstones — a hard delete that misses an offline
replica can be *resurrected* by read-repair. ScyllaDB closes this with
`tombstone_gc_mode::repair`: a tombstone cannot be GC'd until repair confirms all
replicas saw it. `CacheInvalidationFrame` (invalidation_bus.rs:179) already
carries `message_id: Option<u64>` and `source_generation: Option<u64>` — the
basis for versioning.

**Design / contract (ADR-5).** Every replicated slot is either a value or a
tombstone, each carrying a version derived from the generation/message_id.
Invalidation writes a `Tombstone` versioned by `generation`/`message_id`. A
tombstone's GC is permitted only after repair (B6) confirms it on all backups
(`gc_eligible_after`). Tombstones participate in the same version ordering as
live values, so repair propagates the *deletion*, not a stale live copy.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster.rs (or a new replication module)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicatedSlot<V> {
    Value { value: V, version: u64 },
    Tombstone { version: u64, gc_eligible_after: ClusterEpoch },
}

impl<V> ReplicatedSlot<V> {
    pub fn version(&self) -> u64 {
        match self {
            ReplicatedSlot::Value { version, .. } => *version,
            ReplicatedSlot::Tombstone { version, .. } => *version,
        }
    }
    /// Higher version wins; on a tie, a tombstone wins over a value.
    pub fn merge(self, other: Self) -> Self {
        use std::cmp::Ordering::*;
        match self.version().cmp(&other.version()) {
            Greater => self,
            Less => other,
            Equal => match (&self, &other) {
                (ReplicatedSlot::Tombstone { .. }, _) => self,
                _ => other,
            },
        }
    }
}
```

The version is computed as `generation << 32 | message_id_low` (or an equivalent
monotonic composite) from the inbound `CacheInvalidationFrame`.

**Step-by-step implementation.**

1. Add `ReplicatedSlot<V>` and `merge` with the version + tombstone-wins-on-tie
   rule.
2. On invalidation, write a `Tombstone { version, gc_eligible_after: <unset> }`
   to the slot instead of removing it.
3. On value replication, only overwrite if the incoming version is higher (use
   `merge`).
4. Gate GC: a tombstone becomes `gc_eligible_after` an epoch only when the
   anti-entropy task (B6) confirms it on all backups.
5. Hot-cache invalidation broadcasts to all hot-copy holders (full fan-out),
   authoritative over the 30s TTL.

**Testing.** `crates/hydracache/tests/tombstone_replication.rs`

- `tombstone_beats_stale_value_replication` (**property**): for any two frames,
  the higher-versioned tombstone always wins over a lower-versioned value; on a
  version tie the tombstone wins. Use `proptest` over versions and orderings.
- `gc_blocked_until_repair_confirmation` (integration): a tombstone is not GC'd
  while any backup is unconfirmed; once all confirm, it becomes eligible.
- `concurrent_value_and_tombstone_resolve_by_version` (unit): merge picks the
  higher version deterministically.
- `failover_does_not_resurrect_invalidated_value` (**chaos**, `#[ignore]` by
  default for long runs): under random partition + failover, an invalidated key
  is never observed alive after the tombstone version. Cross-references A1/B4.
- Run unit/integration/property: `cargo test -p hydracache --locked tombstone_replication`.
- Run chaos: `cargo test -p hydracache --locked tombstone_replication -- --ignored`.

**Pros.** Closes the resurrection gap; makes the grid's central invariant a
machine-checked property.

**Risks.** Tombstone memory and GC discipline; mitigated by the byte cap and
repair-gated cleanup.

**Tombstone budget and overflow policy.** Repair-gated GC has a failure mode the
base design hides: if repair (B6) lags or a backup stays offline, tombstones are
*never* eligible for GC and accumulate without bound — the repair gate quietly
becomes a memory leak. This release must bound that explicitly.

- A configured budget `max_tombstones` (count) and `max_tombstone_bytes`
  caps tombstone retention per node.
- Eviction order is **oldest-eligible first**; a tombstone that is still
  blocking (unconfirmed by some backup) is **never silently dropped**, because
  dropping it would re-open the resurrection window.
- When the budget is exceeded by still-blocking tombstones, the node enters a
  **degraded "repair-debt" state**: it raises `tombstone_repair_debt` (gauge),
  surfaces it in diagnostics, and — per the no-silent-correctness-loss rule —
  fails loud rather than discarding a blocking tombstone. The operator runbook
  ties this to forcing/repairing the lagging replica or removing it from the
  member set so its tombstones can be released.

```rust
pub struct TombstoneBudget {
    pub max_tombstones: usize,
    pub max_tombstone_bytes: u64,
}

pub enum TombstoneAdmission {
    Stored,
    EvictedEligible { freed: usize },
    RepairDebt,   // budget exceeded by still-blocking tombstones -> degraded
}
```

Testing additions to `tombstone_replication.rs`:

- `eligible_tombstones_evicted_oldest_first_under_budget` (unit) — once GC-
  eligible, tombstones are reclaimed in age order when the budget is hit.
- `blocking_tombstone_never_silently_dropped` (unit) — a still-unconfirmed
  tombstone is retained even over budget; admission returns `RepairDebt`.
- `repair_debt_state_is_observable` (integration) — `tombstone_repair_debt`
  gauge increments and appears in the diagnostics snapshot.

This makes the repair gate safe under sustained repair failure instead of
trading a resurrection bug for a memory-exhaustion bug.

---

### Value Replication Prototype (opt-in, narrow)

**Problem / motivation.** The first real data-grid behavior: replicate encoded
value bytes from primary to backups. It must be opt-in and bounded — without a
byte cap and backpressure counter from the start, the first large `fetch_all`
floods the network.

**Design / contract.** Replication is off by default. When enabled, the primary
replicates the encoded value to backups after a local write/load, and replicates
invalidations (tombstones, A5) to primary and backups. A mandatory
`max_replicated_entry_bytes` (tied to the byte-weigher) rejects oversized entries
pre-send, and a backpressure counter exists from day one. B5's sync/async backup
count controls whether the write waits for backup acks.

**Rust sketch.**

```rust
// builder API on HydraCache
let cache = HydraCache::member()
    .replication_factor(2)
    .replicate_values(true)
    .sync_backups(1)
    .async_backups(1)
    .max_replicated_entry_bytes(256 * 1024) // mandatory cap
    .start()
    .await?;
```

```rust
// crates/hydracache-observability counters
pub struct ReplicationMetrics {
    pub replication_success_total: Counter,
    pub replication_failure_total: Counter,
    pub bytes_replicated_total: Counter,
    pub replication_backpressure_total: Counter,
    pub replication_oversized_rejected_total: Counter,
    pub under_replicated_keys: Gauge,
}
```

**Step-by-step implementation.**

1. Add builder options `replicate_values`, `replication_factor`,
   `sync_backups`, `async_backups`, `max_replicated_entry_bytes` (the cap is
   required when replication is on).
2. After a local write/load on the primary, enqueue replication to backups via
   the existing HTTP transport (`hydracache-cluster-transport-axum`); add a
   `/replicate` route.
3. Reject entries exceeding `max_replicated_entry_bytes` before send; bump
   `replication_oversized_rejected_total`.
4. Apply backpressure (bounded queue); bump `replication_backpressure_total`
   when the queue is full.
5. Apply A1 epoch fencing and wire-version checks to inbound replication frames.

**Testing.** `crates/hydracache/tests/replication.rs` and
`crates/hydracache-cluster-transport-axum/tests/replication.rs`

- `value_loaded_on_primary_replicates_to_backup` (integration): backup holds the
  encoded bytes after replication.
- `invalidation_removes_primary_and_backup_copies` (integration): tombstone
  fan-out clears both.
- `replication_disabled_keeps_local_behavior` (integration): with
  `replicate_values(false)`, no replication traffic and identical local results.
- `stale_generation_replication_is_rejected` (integration): a frame with an old
  epoch is dropped (A1).
- `oversized_value_rejected_and_counted` (unit): entry over cap is rejected and
  the counter increments.
- `replication_failure_increments_counter_and_reports_degraded` (integration):
  backup down → failure counter up, degraded report.
- `wire_version_mismatch_rejects_replication_safely`
  (`crates/hydracache-cluster-transport-axum/tests/replication.rs`, integration):
  mismatched `version` header → safe reject.
- Run: `cargo test -p hydracache --locked replication` and
  `cargo test -p hydracache-cluster-transport-axum --locked replication`.

**Pros.** First step toward data-grid behavior; cost is visible from the start.

**Risks.** Replication correctness is harder than invalidation; large values and
serialization compatibility become production concerns. Mitigated by opt-in
scope, byte cap, and tombstone ordering.

---

### Replicated-Value Data Protection (confidentiality & redaction)

**Problem / motivation.** Until `0.41`, cached values never left the process that
loaded them. Value replication changes the threat model: encoded query results —
which may contain PII or otherwise sensitive columns — now travel between nodes
and sit in the memory of backup owners that never issued the query. The transport
ADR (mTLS / service mesh) protects the *channel*, but says nothing about the
*data*. This release must take an explicit position on replicated-value
confidentiality rather than leaking it implicitly.

**Design / contract (extends ADR-6, transport/security).**

- **Opt-in, per-cache classification.** A value type declares whether it is
  replication-eligible. The default for a replicated cache is "eligible"; a type
  can opt out (`Replication::LocalOnly`) so sensitive results are cached locally
  but never shipped to backups.
- **Optional payload encryption at the replication boundary.** When
  `encrypt_replicated_values` is set, the encoded bytes are sealed with an
  operator-supplied key provider (AEAD, e.g. AES-GCM/ChaCha20-Poly1305) before
  leaving the primary and opened on the backup. HydraCache does **not** manage or
  rotate keys itself — it consumes a `ReplicationKeyProvider` trait (same
  posture as transport: we make the unsafe default loud, we do not become a KMS).
- **Redaction hook.** A `RedactReplicatedValue` hook can strip/transform fields
  before replication for cases where the cached projection is broader than what
  may be shared with backups.
- **Loud default.** If replication is on but neither encryption nor an explicit
  "plaintext acceptable on this trust boundary" acknowledgement is configured,
  `cluster_pilot_readiness()` / actuator surfaces `REPLICATED VALUES PLAINTEXT`
  (same red-flag treatment as `AUTH MISSING` in `0.40`).

```rust
pub enum Replication { Eligible, LocalOnly }

pub trait ReplicationKeyProvider: Send + Sync {
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, ReplicationCryptoError>;
    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, ReplicationCryptoError>;
}

// builder additions
let cache = HydraCache::member()
    .replicate_values(true)
    .encrypt_replicated_values(provider)      // optional; else must ack plaintext
    .redact_replicated_values(redactor)       // optional
    .start().await?;
```

**Step-by-step implementation.**

1. Add `Replication { Eligible, LocalOnly }` and honor `LocalOnly` in the
   replication enqueue path (step 2 of the prototype) — `LocalOnly` values are
   cached but never sent.
2. Add the `ReplicationKeyProvider` trait + seal-on-send / open-on-receive around
   the `/replicate` payload; on `open` failure, reject and count
   `replication_decrypt_failure_total` (never serve undecryptable bytes).
3. Add the `RedactReplicatedValue` hook applied before seal.
4. Add the readiness flag + actuator red-flag for the plaintext-without-ack case.

**Testing.** `crates/hydracache/tests/replication_data_protection.rs`

- `local_only_value_is_cached_but_never_replicated` (integration) — no
  `/replicate` traffic for a `LocalOnly` type; local reads still work.
- `encrypted_roundtrip_seals_and_opens` (unit) — sealed bytes differ from
  plaintext and `open(seal(x)) == x`.
- `undecryptable_payload_is_rejected_not_served` (integration) — wrong key →
  reject + `replication_decrypt_failure_total` increments, backup serves nothing
  stale.
- `redaction_strips_fields_before_send` (unit) — redactor output is what crosses
  the wire, not the full value.
- `plaintext_without_ack_is_flagged_in_readiness` (integration) — readiness
  surfaces `REPLICATED VALUES PLAINTEXT`.
- Run: `cargo test -p hydracache --locked replication_data_protection`.

**Pros.** Makes the new cross-node data exposure a deliberate, observable choice;
keys/redaction stay the operator's responsibility (no KMS scope creep).

**Risks.** Crypto adds latency and a key-management burden — mitigated by keeping
it opt-in and provider-supplied. **Fallback:** if encryption does not land in
time, ship `Replication::LocalOnly` + the loud plaintext readiness flag + an ADR;
that still gives operators a safe path (don't replicate sensitive caches) and
satisfies the work item.

---

## B-Items (Runtime & Client, from Hazelcast)

### B1. Near-Cache `RepairingTask` (full)

**Problem / motivation.** 0.40 shipped the early near-cache repair (uuid-reset +
sequence-gap detection over the existing `CacheInvalidationFrame` fields). 0.41
adds the periodic `RepairingTask` that reconciles the client watermark against
the owner, catching invalidations lost by a dropped push.

**Design / contract.** The client keeps a `MetaDataContainer` per partition with
`last_uuid` (= owner `source_generation`) and `last_seq` (= last applied
`message_id`). On each frame: a changed UUID means the owner restarted → clear
the partition's near-cache; a sequence gap means a possibly-lost invalidation →
invalidate conservatively. A periodic `RepairingTask` polls the owner's current
watermark and reconciles any drift.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster.rs (client side)
struct MetaDataContainer {
    last_uuid: ClusterGeneration, // owner source_generation
    last_seq: u64,                // last applied message_id
}

enum RepairAction { Apply, InvalidateConservatively, ClearPartition }

impl MetaDataContainer {
    fn on_frame(&mut self, generation: ClusterGeneration, message_id: u64) -> RepairAction {
        if generation != self.last_uuid {
            self.last_uuid = generation;
            self.last_seq = message_id;
            return RepairAction::ClearPartition;
        }
        if message_id > self.last_seq + 1 {
            self.last_seq = message_id;
            return RepairAction::InvalidateConservatively;
        }
        self.last_seq = message_id.max(self.last_seq);
        RepairAction::Apply
    }
}
```

A `RepairingTask` runs on an interval, fetching the owner watermark and applying
`on_frame`-equivalent reconciliation for any missed range.

**Step-by-step implementation.**

1. Add `MetaDataContainer` and `RepairAction` on the client path; feed it the
   frame `source_generation` and `message_id`.
2. Wire `ClearPartition`/`InvalidateConservatively` into the near-cache.
3. Add a periodic `RepairingTask` (configurable interval) that polls owner
   watermarks and reconciles drift.
4. Add metrics for conservative invalidations and repair reconciliations.

**Testing.** `crates/hydracache/tests/near_cache_repair.rs`

- `sequence_gap_triggers_conservative_invalidation` (unit).
- `generation_change_clears_partition` (unit).
- `duplicate_or_reordered_frame_does_not_break_watermark` (unit).
- `lost_frame_is_recovered_by_periodic_task` (integration).
- `reorder_and_restart_simultaneously_resolves_to_clear` (integration).
- Run: `cargo test -p hydracache --locked near_cache_repair`.

**Pros.** Cheapest way to make near-caches resilient to lost invalidations
without strong consistency; reuses existing frame fields.

**Risks.** False gaps over-invalidate and hurt hit-rate; tune threshold + expose
metrics.

---

### B4. Three-Phase Backup Promotion (implementation)

**Problem / motivation.** No failover semantics exist yet — only miss/reload on
owner departure. Promotion must repair the partition table, not run on the hot
data path.

**Design / contract.** When `CommitTopology` records a primary's departure, the
coordinator (A4) runs `BeforePromotion` (freeze writes to the partition) →
`CommitPromotion` (backup → primary in `EffectiveReplicationMap`) →
`FinalizePromotion` (unfreeze, re-replicate to RF). Promotion is a topology
operation, not a data op. A5's tombstone invariant holds throughout.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster.rs
enum PromotionPhase { Before, Commit, Finalize }

struct BackupPromotion {
    partition: PartitionId,
    departing_primary: ClusterNodeId,
    new_primary: ClusterNodeId, // promoted backup
}
```

**Step-by-step implementation.**

1. Detect primary departure in the committed topology.
2. `BeforePromotion`: freeze partition writes.
3. `CommitPromotion`: promote a backup in the effective map via Raft.
4. `FinalizePromotion`: unfreeze and trigger re-replication (A4) back to RF.
5. Emit a degraded report when no backup exists.

**Testing.** `crates/hydracache/tests/failover.rs`

- `primary_leaves_backup_serves_value` (**chaos**, `#[ignore]`): controlled
  in-memory partition/failover; backup serves the value.
- `writes_frozen_during_promotion` (integration).
- `replication_factor_restored_after_finalize` (integration).
- `invalidation_during_promotion_beats_stale_value` (**property**): ties to A5.
- `no_backup_owner_reports_degraded` (unit).
- Run: `cargo test -p hydracache --locked failover` and chaos with `-- --ignored`.

**Pros.** Deterministic failover decoupled from the hot path.

**Risks.** Depends on A3/A4; the write freeze adds latency on the promotion
window.

---

### B5. Tunable Sync/Async Backup Count

**Problem / motivation.** Make the latency/durability trade-off explicit, like
Hazelcast `backup-count` / `async-backup-count` and `sendBackups0`.

**Design / contract.** `ReplicationConfig` (A3) gains `sync_backups` and
`async_backups`. A write is acknowledged after the sync backups confirm; async
backups are best-effort. `sync_backups + async_backups <= replication_factor - 1`.

**Rust sketch.**

```rust
pub struct ReplicationConfig {
    pub replication_factor: usize,
    pub read_quorum: usize,
    pub write_quorum: usize,
    pub sync_backups: usize,  // acked before client response
    pub async_backups: usize, // best-effort
}
```

**Step-by-step implementation.**

1. Extend `ReplicationConfig` and its `validate()`.
2. In the replication path, wait for `sync_backups` acks before responding;
   fire-and-forget the async backups.
3. Emit a degraded report if a sync backup is unavailable.

**Testing.** `crates/hydracache/tests/replication.rs` (shared file, new fns)

- `sync_backup_acked_before_client_response` (integration).
- `async_backup_does_not_block_write` (integration).
- `sync_backup_unavailable_reports_degraded` (integration).
- `sync_async_counts_validated` (unit).
- Run: `cargo test -p hydracache --locked replication`.

**Pros.** Explicit, configurable durability/latency trade-off.

**Risks.** Sync backups raise write latency — chosen deliberately.

---

### B6. Per-Replica Anti-Entropy (executor of A5's GC gate)

**Problem / motivation.** Per-op replication can lose updates. Hazelcast's
`PartitionReplicaVersions` + `CheckPartitionReplicaVersionTask` lazily resync
drifted backups. This task is also the executor that confirms a tombstone on all
backups, unblocking A5's GC.

**Design / contract.** Maintain a per-`(partition, replica)` version. A periodic
task compares each backup's version to the primary's and re-replicates laggards.
When a tombstone is confirmed on all backups, it sets `gc_eligible_after`.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster.rs
struct PartitionReplicaVersions {
    versions: HashMap<(PartitionId, ClusterNodeId), u64>,
}

struct AntiEntropyTask { interval: Duration }
```

**Step-by-step implementation.**

1. Track per-(partition, replica) versions.
2. Run a throttled periodic task comparing backup vs primary versions.
3. Re-replicate laggards; report `under_replicated_keys`.
4. On full confirmation of a tombstone, set its `gc_eligible_after` (A5).

**Testing.** `crates/hydracache/tests/anti_entropy.rs`

- `lagging_backup_is_caught_up_by_task` (integration).
- `repair_confirmation_unblocks_tombstone_gc` (integration): ties to A5.
- `under_replicated_key_is_reported` (integration).
- Run: `cargo test -p hydracache --locked anti_entropy`.

**Pros.** Insures against per-op replication loss; provides the safe-GC signal.

**Risks.** Background traffic; configurable interval/throttle.

---

### Three-Part Counters & Authoritative Hot-Cache Invalidation

**Problem / motivation.** groupcache splits `main_cache` (owned keys) vs
`hot_cache` (foreign keys, 30s TTL), and crucially does **not** invalidate hot
copies on other nodes — their only freshness boundary is the TTL. HydraCache must
distinguish owner-load, remote-fetch, and hot-cache-hit as **separate** counters,
and hot-cache invalidation must be authoritative over TTL (broadcast to hot-copy
holders, full fan-out).

**Design / contract.** Add three distinct counters and make invalidation
fan-out to all hot-copy holders, superseding TTL.

**Rust sketch.**

```rust
// crates/hydracache-observability
pub struct CacheHitMetrics {
    pub owner_load_total: Counter,    // this node owns the key, loaded it
    pub remote_fetch_total: Counter,  // fetched from the owning peer
    pub hot_cache_hit_total: Counter, // served from the foreign hot copy
}
```

**Step-by-step implementation.**

1. Split the existing hit counter into the three above.
2. On invalidation, broadcast to all hot-copy holders (full fan-out via the
   invalidation bus / transport), not relying on TTL.
3. Expose the three counters in the actuator/diagnostics.

**Testing.** `crates/hydracache/tests/hot_cache_invalidation.rs`

- `three_counters_increment_independently` (unit).
- `hot_copy_invalidated_before_ttl` (integration): invalidation reaches hot-copy
  holders and clears them before the 30s TTL would.
- `full_fanout_reaches_all_holders` (integration).
- Run: `cargo test -p hydracache --locked hot_cache_invalidation`.

**Pros.** Closes the groupcache hot-copy staleness gap; makes cache cost visible.

**Risks.** Fan-out traffic on invalidation bursts; bounded by the invalidation
bus backpressure.

---

## Ops & SLOs

Required observability (all exported via `hydracache-observability` and surfaced
in the actuator without leaking secrets):

- replication success / failure counters;
- bytes replicated;
- replication lag;
- under-replicated key count;
- failover count;
- repair task count and repair failures;
- placement churn (rebalance plans, moves, acks);
- transport auth failures;
- wire-compatibility (version mismatch) failures;
- control-plane quorum health (and degraded diagnostics when quorum is lost);
- replication backpressure and oversized-rejected counters;
- the three-part hit counters (owner-load / remote-fetch / hot-cache-hit);
- topology fence rejections;
- replicated-value confidentiality posture (encrypted vs plaintext-acked) and
  `replication_decrypt_failure_total`.

### Metric cardinality discipline

This release introduces several naturally **per-entity** signals — per-partition
sequences/UUIDs (B1 near-cache), per-replica versions (B6 anti-entropy),
per-key under-replication, per-move placement churn (A4). Exported naively as
labeled time series, these explode metric cardinality and can overwhelm a
Prometheus/OTLP backend — turning observability into an outage source. The plan
must bound this explicitly:

- **No unbounded label is exported as a metric.** Per-partition / per-replica /
  per-key state is exposed only through the **diagnostics snapshot** (a bounded,
  on-demand JSON structure), never as one time series per entity.
- **Metrics are aggregates.** What crosses to the metrics backend is counts,
  rates, gauges, and histograms — e.g. `under_replicated_keys` is a single gauge,
  not one series per key; per-partition repair is a histogram of lag, not a
  series per partition.
- **Bounded label sets only.** Allowed labels are low-cardinality and enumerable
  (role, result-kind: owner-load/remote-fetch/hot-cache-hit, outcome:
  success/failure). Node id is acceptable (bounded by cluster size, 2–5 in the
  pilot); partition id, key, and replica index are **not** label material.
- **Exposition format.** Counters/gauges/histograms follow a standard exposition
  (Prometheus text / OTLP) via `hydracache-observability`; the actuator JSON
  remains the human/debug surface and the home for high-cardinality detail.

TESTING (`crates/hydracache-observability/tests/cardinality.rs`):

- `fn exported_metrics_have_only_bounded_labels()` — enumerate every registered
  metric; assert no label is `partition_id` / `key` / `replica_index`.
- `fn under_replication_is_a_single_gauge_not_per_key()` — registering N
  under-replicated keys yields one series, value N.
- `fn per_partition_detail_only_in_snapshot()` — per-partition data appears in
  the diagnostics snapshot but not in the metrics registry.
- Run: `cargo test -p hydracache-observability --locked cardinality`.

Required docs (under `docs/cluster/`): deployment topology; memory sizing
(including the tombstone budget and repair-debt behavior from A5);
replication-factor selection; failure scenarios; rolling upgrade and wire
compatibility; backup-owner behavior; repair runbook; security checklist (mTLS /
service-mesh recommended deployment, node identity, token-provider rotation,
authorization for peer-fetch/owner-load/replication/admin routes); and a
prominent "still not distributed transactions" warning.

**Rolling upgrade builds on the cross-release compatibility discipline.** The
raft log format (`RaftLogStore`, A2) and the value-replication wire format added
in this release are new durable/wire artifacts and MUST be registered in
`docs/COMPAT.md` with their version and compatibility window, following the
discipline started in `0.37` (see `V0_37_...` §5a, "Compatibility and Migration
Discipline"). Concretely: the replication frame reuses
`CacheInvalidationFrame::version` negotiation; the raft log carries its own
on-disk format version, and a node refuses to start against an unknown future
log-format version (fail loud, mirroring the outbox `schema_version` guard). The
rolling-upgrade compatibility tests below assert old↔new reader/writer pairings
against that register.

## Fault Model and Test Tiering

The chaos and partition tests referenced throughout the A/B items (A5, B4, B6)
are under-specified if "chaos" is left to each test's imagination. This release
pins a shared **fault model** and a **test tier** so the suites are reproducible
and another agent can implement them against one contract.

**Enumerated faults** (the injectable set; tests compose these):

- node crash (clean process exit) and node kill (no graceful stop);
- node restart with a higher generation (rejoin);
- network partition (symmetric and asymmetric / one-way drop);
- message loss, duplication, and reordering on the invalidation/replication bus;
- slow node / slow disk (latency injection on persist and transport);
- clock skew between nodes (bounded, to stress epoch/version logic — never used
  as a correctness source; authority is epoch/version, not wall-clock);
- backup permanently offline (drives A5 repair-debt and B6 anti-entropy).

**Determinism contract.** Fault injection is driven by a seeded RNG; every chaos
test logs its seed and is replayable from it. Correctness assertions use logical
signals (epoch, version, applied counters), not wall-clock thresholds — wall-clock
appears only in soak latency reporting, never in pass/fail (the same rule the
`0.39` gate established).

**Test tiers** (which gate runs what):

| Tier | Scope | When | Command shape |
| --- | --- | --- | --- |
| fast | unit + deterministic property (placement, merge, fence, cardinality) | every PR | `cargo test --workspace --locked` |
| integration | in-memory multi-node, HTTP routes, restart/rejoin | every PR | `cargo test --workspace --locked` |
| chaos/soak | seeded fault injection, partition, long churn | nightly / pre-release | `cargo test --workspace --locked -- --ignored` |
| Docker | testcontainers-backed (where applicable) | nightly / pre-release | Docker gate |

A shared harness `crates/hydracache/tests/support/fault_injector.rs` exposes the
enumerated faults behind one API so A5/B4/B6 chaos tests compose them rather than
re-implementing partition logic per test.

TESTING of the harness itself (`crates/hydracache/tests/fault_injector_selftest.rs`):

- `fn seed_replays_identical_fault_schedule()` — same seed → same fault sequence.
- `fn partition_is_symmetric_and_asymmetric()` — both directions injectable.
- `fn injected_latency_is_observed_by_target_only()` — slow-node scoping works.
- Run: `cargo test -p hydracache --locked fault_injector_selftest`.

## Test Matrix For Any Future Production-Grid Claim

Before any future release claims production data-grid readiness, the project
needs all of:

- deterministic unit tests for placement (A3);
- in-memory multi-node replication tests;
- HTTP replication route tests (`hydracache-cluster-transport-axum`);
- auth and wire-compatibility tests;
- failover/repair property and chaos tests (A5/B4/B6);
- chaos/partition simulation;
- long-running soak with membership churn;
- large-value and memory-pressure tests (byte cap);
- rolling-upgrade compatibility tests;
- persistence/restart tests for metadata (A2);
- optional persistence/restart tests for values if durable values are added;
- external-consumer tests against published crates.

## Release Gates For 0.41

Focused:

```powershell
cargo test -p hydracache --locked adr_presence
cargo test -p hydracache --locked topology_fence
cargo test -p hydracache --locked placement
cargo test -p hydracache --locked rebalance
cargo test -p hydracache --locked tombstone_replication
cargo test -p hydracache --locked replication
cargo test -p hydracache --locked near_cache_repair
cargo test -p hydracache --locked failover
cargo test -p hydracache --locked anti_entropy
cargo test -p hydracache --locked hot_cache_invalidation
cargo test -p hydracache-cluster-transport-axum --locked replication
cargo test -p hydracache-cluster-raft --locked persistent_log
cargo test -p hydracache-cluster-raft --locked --features sled-log-store persistent_log
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --workspace --locked -- --ignored   # chaos/long-running suites
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.41.0` may claim "distributed cache grid roadmap and first safe slice" only if
**all** of the following boolean conditions hold:

- The five ADR files (`docs/adr/0001`–`0005`) exist and are linked from
  `docs/cluster/readiness.md`, and `adr_presence` passes.
- A1: epoch fencing is implemented; gossip suspects never change ownership before
  a Raft `CommitTopology`; `topology_fence` passes.
- A2: `RaftLogStore` replaces `MemStorage`; append→replay, snapshot-recovery, and
  duplicate-command-id idempotency tests pass, including the feature-flagged
  engine example.
- A3: `ClusterReplicationStrategy` + `EffectiveReplicationMap` expose primary +
  backups; quorum/replication config is validated at startup; `placement` passes.
- A4: rebalance is plan-as-data executed by a single coordinator; `rebalance`
  passes.
- A5: versioned tombstones with repair-gated GC exist; the
  tombstone-beats-stale-replication **property** test and the failover **chaos**
  test pass.
- The value-replication prototype is opt-in, off by default, has a mandatory
  `max_replicated_entry_bytes` and a backpressure counter; `replication` passes —
  or replication is explicitly deferred and documented as such.
- B1/B4/B5/B6 are implemented and their suites pass.
- The three-part counters exist and hot-cache invalidation is authoritative over
  TTL.
- All required Ops & SLO observability is exported.
- Docs explicitly state that **production distributed-data-grid readiness is not
  complete**: this release delivers the roadmap and the first safe slice only.

If any condition fails, the release ships without the data-grid slice claim and
documents what was deferred.
