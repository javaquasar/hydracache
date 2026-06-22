# HydraCache 0.42.0 Production Grid Hardening Plan

Status: implemented in `0.42.0`. Release notes:
[`docs/releases/0.42.0.md`](../releases/0.42.0.md).

`0.42.0` is the release that **earns the production distributed-data-grid claim**
that `0.41.0` deliberately refused to make. Where `0.41.0` shipped a correctness
*skeleton* (the `RaftLogStore` trait with an in-memory fake plus one
feature-flagged engine example, a `ClusterReplicationStrategy` /
`EffectiveReplicationMap`, rebalance plan-as-data, versioned tombstones, and an
opt-in value-replication *prototype*), `0.42.0` turns those prototypes into
supported, durable, restart-survivable features and makes the project pass the
"Test Matrix For Any Future Production-Grid Claim" that `0.41.0` defined.

The release follows the same authority/dissemination resolution rule established
in `0.41`:

> **Authority** (who owns a key, which topology is valid, which version is newer)
> is the ScyllaDB model: Raft + monotonic epoch. **Dissemination** (how staleness
> is detected and propagated) is the Hazelcast model: sequence/UUID stamps. When
> the two disagree, the epoch (authority) wins; the stamp only triggers a
> conservative refresh/invalidate.

Readiness is described in prose and asserted as boolean release gates. There is
no numeric self-score. Unlike `0.41`, after `0.42.0` the project **may** claim
production distributed-data-grid readiness for the supported topology — but only
if every boolean gate below holds and the full Test Matrix passes on real
multi-node persistence, failover, and chaos suites.

## Release Theme

Make the 0.41 prototypes real: a durable multi-node control plane, durable
replicated values that survive restart, production-grade replication/failover
under load, split-brain detection with a documented merge policy, grid-wide
read-your-writes, enforced node identity/authorization, and an operator surface
good enough to run the grid in production.

The work is organized as seven items (W1–W7) plus explicit deferrals. Each item
builds directly on a named `0.41` artifact and replaces "prototype / trait /
example" with "supported, durable, tested under fault injection".

## Non-Goals

- **No distributed transactions.** This remains a hard non-goal (it has been one
  since `0.37`). Cross-key/cross-node atomic commit is out of scope; docs must
  keep the prominent "still not distributed transactions" warning.
- **No multi-region / zone-aware placement.** Topology-aware replica placement
  (the ScyllaDB `NetworkTopologyStrategy` analogue, an extension of A3) is
  deferred to `0.43`.
- **No new storage engine invention.** `0.42` selects and productionizes an
  existing embedded engine behind the `0.41` `RaftLogStore` trait; it does not
  write a novel storage engine.
- **No remote code execution.** No remote SQL/expression evaluation, no remote
  load closures over the wire (unchanged from prior releases).
- **No silent consistency claims.** Every consistency limitation that survives
  into `0.42` is documented with the scenario that exposes it.
- **No KMS / secret-store integration.** Crypto key material and node-identity
  material stay operator-supplied via provider traits (continuation of the
  `0.41` `ReplicationKeyProvider` posture).

## Inherited Boundary From 0.41

`0.42` only hardens what `0.41` introduced; it must not re-litigate `0.41`'s
design. Concretely:

- The `RaftLogStore` **trait + in-memory fake + one feature-flagged engine
  example** were `0.41`. The **production engine selection, multi-node raft
  transport, and snapshot/compaction** are `0.42` (W1).
- The **value-replication prototype** (opt-in, `max_replicated_entry_bytes`,
  backpressure counters, `Replication { Eligible, LocalOnly }`,
  `ReplicationKeyProvider`) was `0.41`. **Durable replicated values that survive
  restart, adaptive flow control, and hardened B4/B6 under load** are `0.42`
  (W2, W3).
- **Epoch fencing (A1, `TopologyFence` / `CommitTopology`)** was `0.41`.
  **Split-brain *detection* and a *merge policy*** — only possible once the
  control plane is durable and multi-node — are `0.42` (W4).
- The **quorum barrier** matured in `0.40` and `ReplicationConfig`
  (`read_quorum` / `write_quorum`) landed in `0.41` A3. **Grid-wide
  read-your-writes that combines the quorum barrier with durable replication** is
  `0.42` (W5).
- The **`transport_posture` red flags (`AUTH MISSING`)** were `0.40` and the
  **`REPLICATED VALUES PLAINTEXT`** flag was `0.41`. **Enforced node identity and
  authorization on every cluster route** is `0.42` (W6).

## Dependency Graph

```
0.41 RaftLogStore trait ─────────────► W1 durable multi-node control plane
0.41 value-replication prototype ────► W2 durable replicated value store
W1 + W2 ─────────────────────────────► W3 production replication & failover
W1 (durable, multi-node) ────────────► W4 split-brain detection + merge
W1 + 0.40 quorum barrier + W2 ───────► W5 grid-wide read-your-writes
0.40 transport_posture ──────────────► W6 production security (identity + authz)
W1..W6 ──────────────────────────────► W7 operational surface
```

W1 is the long pole: durable, multi-node, recoverable Raft is the precondition
for W4 (you cannot detect split-brain without a real committed history) and W5
(read-your-writes needs a durable commit index). W2 is the precondition for
durable failover in W3.

---

## W1. Durable Multi-Node Raft Control Plane

**Problem / motivation.** `0.41` shipped the `RaftLogStore` *trait*, an in-memory
fake, and exactly one feature-flagged engine *example*; it deliberately did **not**
pick an engine, run multi-node raft transport, or implement production snapshots.
`RaftMetadataRuntime` (`crates/hydracache-cluster-raft/src/lib.rs`) still ran a
single-node in-memory metadata runtime in practice. A production grid needs a
control plane whose committed topology, ownership, and tombstone versions survive
process restart and a node loss, and that replicates the raft log across members
so a minority cannot lose committed history.

**Design / contract.** Select one embedded persistence engine (candidates: `sled`
or `rocksdb` via the existing feature-flagged example seam from A2) and provide
the *supported* `RaftLogStore` implementation behind a default feature. Implement
a real multi-node raft transport over the existing
`hydracache-cluster-transport-axum` crate so log entries, votes, and snapshots
replicate between members. Implement snapshotting and log compaction with the
A2 durability contract: write order is snapshot → entries → HardState; `must_sync`
fsync policy is honored; compaction never discards past the applied/snapshot
index. A node refuses to start against an unknown future on-disk log-format
version (the fail-loud guard from `0.37` §5a / `0.41` rolling-upgrade pointer).

**Rust sketch.**

```rust
// crates/hydracache-cluster-raft/src/store.rs
/// Supported durable implementation of the 0.41 RaftLogStore trait.
/// Engine chosen in 0.42; the trait and an in-memory fake already exist.
pub struct DurableRaftLogStore<E: LogEngine> {
    engine: E,
    log_format_version: u32,
}

pub trait LogEngine: Send + Sync {
    fn append(&self, entries: &[raft::eraftpb::Entry]) -> Result<(), RaftStoreError>;
    fn put_hard_state(&self, hs: &raft::eraftpb::HardState) -> Result<(), RaftStoreError>;
    fn install_snapshot(&self, snap: &raft::eraftpb::Snapshot) -> Result<(), RaftStoreError>;
    fn compact(&self, up_to_index: u64) -> Result<(), RaftStoreError>;
    fn fsync(&self) -> Result<(), RaftStoreError>;
}

impl<E: LogEngine> RaftLogStore for DurableRaftLogStore<E> {
    fn persist_ready(&self, ready: &PersistBatch) -> Result<(), RaftStoreError> {
        // contract order: snapshot -> entries -> hard_state, then fsync if must_sync
        if let Some(snap) = &ready.snapshot { self.engine.install_snapshot(snap)?; }
        self.engine.append(&ready.entries)?;
        if let Some(hs) = &ready.hard_state { self.engine.put_hard_state(hs)?; }
        if ready.must_sync { self.engine.fsync()?; }
        Ok(())
    }
}

// crates/hydracache-cluster-raft/src/transport.rs
/// Sends raft messages (append/vote/snapshot) to peers over HTTP.
pub struct RaftPeerTransport { /* peer endpoints from EffectiveReplicationMap */ }

#[async_trait::async_trait]
pub trait RaftMessageSink: Send + Sync {
    async fn send(&self, to: ClusterNodeId, msg: raft::eraftpb::Message) -> Result<(), TransportError>;
}
```

**Step-by-step implementation.**

1. Promote the A2 feature-flagged engine example to a supported
   `DurableRaftLogStore<E>` behind a default `durable-log` feature; keep the
   in-memory fake for tests behind `--no-default-features`.
2. Implement the `LogEngine` for the chosen engine (sled or rocksdb) honoring the
   write order and `must_sync` contract; record the on-disk `log_format_version`
   and refuse unknown future versions on startup.
3. Add `RaftPeerTransport` over `hydracache-cluster-transport-axum`: routes for
   `append_entries`, `request_vote`, and `install_snapshot`; drive the `raft-rs`
   `RawNode` tick/step/ready loop with real peers.
4. Implement snapshotting + compaction: materialize the metadata snapshot
   (members, epoch, ownership, tombstone versions), persist atomically, then
   compact the log up to the snapshot index.
5. Register the raft log on-disk format in `docs/COMPAT.md` with its version and
   compatibility window.
6. Expose `raft_commit_index`, `raft_applied_index`, `raft_snapshot_index`,
   `raft_leader_id`, and `raft_term` as bounded-label metrics.

**Testing.** `crates/hydracache-cluster-raft/tests/durable_control_plane.rs`

- `append_then_replay_recovers_committed_log` (integration): append N commands,
  drop the store, reopen against the same dir; assert the materialized snapshot
  and committed index match.
- `snapshot_then_compact_preserves_applied_state` (integration): snapshot, compact
  past stale entries, reopen; assert applied state is intact and the log does not
  regress past the snapshot index.
- `unknown_future_log_format_version_refuses_start` (unit): write a header with
  `log_format_version + 1`; assert startup fails loud, not silently.
- `must_sync_persists_before_ack` (property, seeded): inject a crash after append
  but before the next command; assert no acknowledged command is lost on replay.
- `three_member_log_replicates_and_elects` (integration): in-memory transport,
  3 members; kill the leader; assert a new leader is elected and the committed
  log is identical on survivors.
- `minority_cannot_commit` (integration): partition 1 of 3; assert the minority
  side accepts no new commits.
- `leader_crash_under_load_loses_no_committed_command` (**chaos**, `#[ignore]`):
  seeded fault injection via the `0.41`
  `crates/hydracache/tests/support/fault_injector.rs` harness; replayable by seed.
- Run: `cargo test -p hydracache-cluster-raft --locked durable_control_plane`
  and chaos with `-- --ignored`.

**Pros.** Removes the single biggest gap between "roadmap slice" and "production
grid": the control plane now survives restart and node loss with no committed
history loss.

**Risks.** Real raft transport + persistence is the highest-complexity item;
correctness bugs here are catastrophic. Mitigation: the in-memory fake stays the
default in CI fast tier, the durable engine runs in integration/nightly, and
every durability claim is a replayable seeded test.

---

## W2. Durable Replicated Value Store (Values Survive Restart)

**Problem / motivation.** `0.41`'s value-replication was an in-memory *prototype*:
opt-in, capped by `max_replicated_entry_bytes`, with backpressure counters, but
replicated values lived only in the backup's memory and did not survive a backup
restart. A production grid claim requires that a backup which restarts can rejoin
and serve (or correctly re-replicate) the values it owned, and that the
replicated bytes are protected per the `0.41` `ReplicationKeyProvider` /
`Replication { Eligible, LocalOnly }` contract.

**Design / contract.** Add an optional durable value store for replicated entries
keyed by `(PartitionId, key)` carrying the versioned-tombstone metadata from A5
(value version, generation/epoch, tombstone marker). On restart, a backup loads
its durable replicated set, then runs anti-entropy (B6) against the current
primary to converge before serving. Durability is opt-in
(`durable_replicated_values(true)`); the default keeps `0.41` in-memory behavior.
Sealed bytes (via `ReplicationKeyProvider::seal`) are what gets persisted —
plaintext is never written to the value store when encryption is configured.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/value_store.rs
pub struct ReplicatedValueRecord {
    pub partition: PartitionId,
    pub version: ValueVersion,      // from A5 versioned tombstones
    pub epoch: ClusterEpoch,
    pub state: ReplicatedSlot<SealedBytes>, // ReplicatedSlot<V> enum from A5
}

pub trait ReplicatedValueStore: Send + Sync {
    fn upsert(&self, key: &CacheKey, rec: &ReplicatedValueRecord) -> Result<(), ValueStoreError>;
    fn get(&self, key: &CacheKey) -> Result<Option<ReplicatedValueRecord>, ValueStoreError>;
    fn tombstone(&self, key: &CacheKey, version: ValueVersion) -> Result<(), ValueStoreError>;
    /// All records this node currently owns as primary or backup, for rejoin.
    fn scan_owned(&self, map: &EffectiveReplicationMap) -> Result<Vec<(CacheKey, ReplicatedValueRecord)>, ValueStoreError>;
}
```

**Step-by-step implementation.**

1. Add `ReplicatedValueStore` with an in-memory impl (default) and a durable impl
   behind a `durable-values` feature, reusing the W1 engine where practical.
2. Persist on the replication receive path: store the sealed payload + A5 version
   metadata; never persist plaintext when a `ReplicationKeyProvider` is set.
3. On startup, `scan_owned` the durable set and feed it into B6 anti-entropy so
   the node converges with the primary before serving.
4. Enforce `max_replicated_entry_bytes` and a new `max_replicated_total_bytes`
   budget at persist time; over budget → reject + counter (never silent drop),
   mirroring the A5 tombstone-budget posture.
5. Register the on-disk value-record format in `docs/COMPAT.md`.

**Testing.** `crates/hydracache/tests/durable_replicated_values.rs`

- `replicated_value_survives_backup_restart` (integration): replicate a value to a
  backup, restart the backup, assert it is present (sealed) and serves after
  anti-entropy.
- `restart_then_anti_entropy_converges_with_primary` (integration): mutate on the
  primary while the backup is down; on rejoin assert the backup converges to the
  primary's version, not its stale one.
- `sealed_bytes_only_are_persisted` (unit): with a `ReplicationKeyProvider`,
  inspect the store; assert no plaintext, and `open(stored) == value`.
- `total_bytes_budget_rejects_over_limit_not_silently` (integration): assert
  rejection + `replicated_value_rejected_total` increments.
- `tombstone_persisted_blocks_resurrection_after_restart` (**property**): a
  persisted tombstone (A5) beats a stale replicated value after restart.
- `durable_value_format_version_round_trips` (unit): old↔new record format pairing
  against `docs/COMPAT.md`.
- Run: `cargo test -p hydracache --locked durable_replicated_values`.

**Pros.** Replicated values now have real availability semantics across restart;
the encryption posture from `0.41` extends to data at rest without new scope.

**Risks.** Durable values multiply on-disk footprint and add a convergence window
on rejoin during which the node must not serve stale data. Mitigation:
serve-after-converge gating + the A5 version check on every read.

---

## W3. Production Replication & Failover Under Load

**Problem / motivation.** `0.41` shipped B4 three-phase backup promotion, B5
tunable sync/async backup count, and B6 per-replica anti-entropy, but proven only
under in-memory integration tests — not under sustained write load, partitions,
or slow nodes. Production failover must hold the A5 tombstone invariant and the
write-freeze window must stay bounded under load. Flow control in `0.41` was a
backpressure *counter*; production needs *adaptive* backpressure so a slow backup
cannot unboundedly buffer or stall the primary.

**Design / contract.** Add adaptive flow control on the replication path: a
bounded per-backup in-flight window that shrinks on observed lag/slowness and
grows on healthy acks (additive-increase/multiplicative-decrease), surfaced as a
gauge. Harden B4 promotion so `BeforePromotion → CommitPromotion →
FinalizePromotion` runs entirely as a topology operation through W1's durable
Raft (never on the hot path), with the write-freeze window measured and bounded.
Harden B6 anti-entropy to run continuously per replica with the W2 durable store,
converging on `(version, epoch)` and never resurrecting a tombstoned key.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/flow_control.rs
pub struct AdaptiveWindow {
    in_flight: usize,
    max_in_flight: usize,     // AIMD-controlled
    floor: usize,
    ceil: usize,
}

impl AdaptiveWindow {
    pub fn on_ack(&mut self, rtt_ok: bool) {
        if rtt_ok { self.max_in_flight = (self.max_in_flight + 1).min(self.ceil); }
        else { self.max_in_flight = (self.max_in_flight / 2).max(self.floor); }
    }
    pub fn admit(&self) -> bool { self.in_flight < self.max_in_flight }
}

// reuse from 0.41:
//   enum PromotionPhase { Before, Commit, Finalize }
//   struct BackupPromotion { partition, departing_primary, new_primary }
```

**Step-by-step implementation.**

1. Add `AdaptiveWindow` per backup on the replication enqueue path; gate sends on
   `admit()`; export `replication_window_size` and `replication_backpressure_total`.
2. Route B4 promotion through W1 durable Raft `CommitTopology`; measure the
   write-freeze duration and export `promotion_freeze_window_ms` (histogram is
   fine; not per-partition labels — cardinality discipline from `0.41` Ops).
3. Make B6 anti-entropy continuous against the W2 durable store; converge on
   `(version, epoch)`; assert the A5 tombstone-beats-value rule on every merge.
4. Add a degraded `repair-debt` escalation: when anti-entropy cannot keep up
   under load, surface the `0.41` `tombstone_repair_debt` gauge and a
   `replication_lag` gauge rather than silently dropping.

**Testing.** `crates/hydracache/tests/replication_under_load.rs`

- `slow_backup_does_not_stall_primary` (integration): inject latency on one backup
  via the fault harness; assert the primary's window shrinks and writes keep
  acking from healthy backups within the sync-backup contract.
- `promotion_freeze_window_is_bounded_under_load` (integration): promote during
  sustained writes; assert the freeze window stays under the documented bound.
- `anti_entropy_converges_after_partition_heals` (**chaos**, `#[ignore]`): seeded
  asymmetric partition; on heal, assert all replicas converge on `(version, epoch)`
  with no tombstone resurrection.
- `failover_preserves_tombstone_invariant_under_churn` (**property**): random
  delete/promote interleavings; a deleted key is never resurrected.
- `aimd_window_recovers_after_transient_slowness` (unit): drive `on_ack`; assert
  multiplicative decrease then additive recovery.
- Run: `cargo test -p hydracache --locked replication_under_load` and chaos with
  `-- --ignored`.

**Pros.** Failover and replication now have measured, bounded behavior under the
exact conditions production hits; backpressure is adaptive, not just observed.

**Risks.** AIMD tuning interacts with the sync/async backup contract (B5); a too-
aggressive floor can stall. Mitigation: floor/ceil are configurable and the
window is a gauge so operators can see it.

---

## W4. Split-Brain Detection + Merge Policy

**Problem / motivation.** `0.41` deliberately had **no split-brain auto-merge**:
minorities were fenced by epoch (A1) and that was the whole story, because without
a durable multi-node control plane a real merge could not be reasoned about. Once
W1 makes the control plane durable and multi-node, two sides of a healed partition
may each hold committed-looking state, and the grid needs *detection* plus a
*documented merge policy* — not silent data divergence.

**Design / contract.** Adopt the Hazelcast `SplitBrainHandler` /
`ClusterMergeTask` shape over the existing epoch fence. Detection: on partition
heal, compare committed epochs/generations across the rejoining sides; the side
with the lower epoch is the loser and must discard topology decisions made while
split. Merge policy is per-entry and pluggable, defaulting to a safe
`HigherVersionWins` (using the A5 `(version, epoch)`), with `PutIfAbsent` and a
`Discard` (loser-side drop) option. Merge runs as a topology operation through W1
Raft, never on the hot path. Anything that cannot be merged deterministically is
**not** silently resolved — it is surfaced as a conflict count + diagnostic and
the loser side's value is dropped (favoring the higher-authority epoch), matching
the "epoch wins; stamp is only a hint" rule.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/split_brain.rs
pub struct SplitBrainReport {
    pub winner_epoch: ClusterEpoch,
    pub loser_epoch: ClusterEpoch,
    pub merged_entries: u64,
    pub discarded_entries: u64,
    pub unresolved_conflicts: u64,
}

pub trait MergePolicy: Send + Sync {
    /// Returns the value to keep, or None to discard the loser-side entry.
    fn merge(&self, winner: Option<&ReplicatedValueRecord>,
             loser: &ReplicatedValueRecord) -> Option<ReplicatedValueRecord>;
}

pub struct HigherVersionWins; // default: keep max (version, epoch)
pub struct PutIfAbsent;        // keep loser only if winner absent
pub struct DiscardLoser;       // always drop loser side
```

**Step-by-step implementation.**

1. Add split-brain detection on partition heal: W1 surfaces each side's committed
   epoch/generation; pick the higher-epoch side as winner via Raft.
2. Run `ClusterMergeTask` as a topology op: for each loser-side entry, apply the
   configured `MergePolicy`; default `HigherVersionWins` on `(version, epoch)`.
3. Loser side discards topology decisions made while split (ownership reverts to
   the winner's committed topology); fence (A1) prevents stale-epoch frames from
   resurrecting them.
4. Export `split_brain_detected_total`, `merge_discarded_entries_total`, and
   `merge_unresolved_conflicts_total`; record a `SplitBrainReport` in the
   diagnostics snapshot.
5. Document the merge policy and its loss semantics in `docs/cluster/`.

**Testing.** `crates/hydracache/tests/split_brain.rs`

- `lower_epoch_side_loses_topology` (integration): two committed sides after a
  simulated split; assert the lower-epoch side reverts to the winner's topology.
- `higher_version_wins_merges_values` (unit): default policy keeps max
  `(version, epoch)`.
- `tombstone_on_winner_beats_value_on_loser` (**property**): a delete on the winner
  is not undone by a loser-side value (ties A5 + the epoch rule).
- `merge_runs_as_topology_op_not_hot_path` (integration): assert no merge work
  executes on the read/write data path.
- `split_then_heal_under_churn_converges` (**chaos**, `#[ignore]`): seeded
  symmetric partition with writes on both sides; on heal, assert a single
  converged state + a `SplitBrainReport` with accurate counts.
- Run: `cargo test -p hydracache --locked split_brain` and chaos with `-- --ignored`.

**Pros.** Closes the one consistency hole that durable multi-node necessarily
opens; divergence becomes detected + policy-resolved + observable instead of
silent.

**Risks.** Any merge policy loses data on the loser side by definition.
Mitigation: the loss is deterministic (epoch authority), counted, reported, and
documented; `PutIfAbsent` exists for caches where loser writes are additive.

---

## W5. Grid-Wide Quorum Read-Your-Writes

**Problem / motivation.** `0.40` matured a quorum *barrier* (read-after-write
within a node / best-effort), and `0.41` A3 added `ReplicationConfig` with
`read_quorum` / `write_quorum`, but there was no end-to-end guarantee that a write
acknowledged by the grid is visible to a subsequent read routed anywhere in the
grid. Production callers expect read-your-writes across the grid for the keys they
just wrote.

**Design / contract.** Combine the `0.40` quorum barrier, the W1 durable commit
index, and W2 durable replication into a read-your-writes contract: a write is
acknowledged only after `write_quorum` replicas confirm the new `(version, epoch)`;
a read at `read_quorum` is guaranteed to observe a version `>=` the last write the
client acknowledged, enforced via a client-carried write watermark
(`(PartitionId, version)`) reconciled against replica versions (the Hazelcast
`MetaDataContainer` watermark mechanism from B1, generalized to values). With
`read_quorum + write_quorum > replication_factor`, the contract is strong; below
that, the contract degrades to read-your-writes-on-the-same-session and is
documented as such.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/consistency.rs
pub struct WriteWatermark {
    pub partition: PartitionId,
    pub version: ValueVersion,
    pub epoch: ClusterEpoch,
}

pub enum ReadConsistency {
    /// read_quorum replicas agree on >= the client's watermark
    QuorumReadYourWrites,
    /// best-effort: nearest replica, may be stale (current default)
    Eventual,
}

impl ReplicationConfig {
    pub fn is_strong_ryow(&self) -> bool {
        self.read_quorum + self.write_quorum > self.replication_factor
    }
}
```

**Step-by-step implementation.**

1. On write ack, return a `WriteWatermark` to the caller after `write_quorum`
   replicas confirm `(version, epoch)`.
2. On a `QuorumReadYourWrites` read, query `read_quorum` replicas and serve the
   max `(version, epoch)`; if all are below the client watermark, conservatively
   read from the primary / trigger anti-entropy rather than serve stale.
3. Validate `read_quorum + write_quorum` vs `replication_factor` at startup;
   surface whether strong RYOW holds in readiness output.
4. Document the degraded mode (session-scoped RYOW) when quorums don't overlap.

**Testing.** `crates/hydracache/tests/read_your_writes.rs`

- `acked_write_is_visible_to_quorum_read` (integration): write at `write_quorum`,
  read at `read_quorum` from a different node; assert visibility when
  `is_strong_ryow()`.
- `read_below_watermark_does_not_serve_stale` (integration): force one replica
  stale; assert the read does not return the stale version.
- `quorum_overlap_validated_at_startup` (unit): non-overlapping quorums →
  readiness reports degraded RYOW, not a false strong claim.
- `ryow_holds_during_single_node_failure` (**chaos**, `#[ignore]`): kill one
  replica mid-test; assert acked writes stay visible while quorum is reachable.
- Run: `cargo test -p hydracache --locked read_your_writes` and chaos with
  `-- --ignored`.

**Pros.** Gives callers a checkable, end-to-end consistency contract for the keys
they wrote, with an explicit strong-vs-degraded boundary.

**Risks.** Quorum reads add latency and read amplification. Mitigation: the
consistency level is per-read opt-in; `Eventual` stays the default and the
read-amplification is exported (ties to the `0.41` three-part counters).

---

## W6. Production Security: Node Identity + Authorization

**Problem / motivation.** `0.40` surfaced `AUTH MISSING` as a red flag and `0.41`
added `REPLICATED VALUES PLAINTEXT`, but cluster routes (peer-fetch, owner-load,
replication, admin, and now raft transport from W1) were not *enforcing* node
identity or authorization — the flags warned, they did not block. A production
grid must reject unauthenticated/unauthorized peers on every cluster route and
support key/token rotation without downtime.

**Design / contract.** Add an operator-supplied `NodeIdentityProvider` (a
continuation of the `ReplicationKeyProvider` posture: provider trait, no KMS) that
issues and verifies a per-node credential (token or mTLS-bound identity).
Enforcement: every cluster route — peer-fetch, owner-load, `/replicate`, raft
append/vote/snapshot (W1), and admin/actuator — verifies the caller's identity and
checks an `Authorizer` before acting. Unauthenticated/unauthorized calls are
rejected (counted, never silently allowed). Rotation: the provider exposes a
current + previous credential window so a rolling restart can rotate keys without
a flag-day. When no identity provider is configured, the `AUTH MISSING` red flag
escalates from "warning" to "refuse to enable replication/admin routes" unless the
operator explicitly acknowledges an insecure trust boundary.

**Rust sketch.**

```rust
// crates/hydracache-cluster-transport-axum/src/auth.rs
pub trait NodeIdentityProvider: Send + Sync {
    fn present(&self) -> NodeCredential;                 // attach to outbound calls
    fn verify(&self, cred: &NodeCredential) -> Result<ClusterNodeId, AuthError>;
    /// current + previous, for rotation windows
    fn accepted(&self) -> SmallVec<[KeyId; 2]>;
}

pub trait Authorizer: Send + Sync {
    fn allow(&self, who: ClusterNodeId, route: ClusterRoute) -> bool;
}

pub enum ClusterRoute { PeerFetch, OwnerLoad, Replicate, RaftAppend, RaftVote, Snapshot, Admin }
```

**Step-by-step implementation.**

1. Add `NodeIdentityProvider` + `Authorizer` traits; thread them through
   `hydracache-cluster-transport-axum` as middleware on every cluster route,
   including the W1 raft routes.
2. Reject unauthenticated/unauthorized calls with a structured error; export
   `cluster_auth_rejected_total` (label: route, bounded).
3. Implement the rotation window (`accepted()` returns current + previous) so a
   rolling restart rotates credentials without downtime.
4. Escalate the `AUTH MISSING` flag: without a provider, refuse to enable
   replication/admin routes unless `acknowledge_insecure_trust_boundary(true)` is
   set (loud, like `0.40`).
5. Document the security checklist in `docs/cluster/` (identity, authz matrix,
   rotation procedure, mTLS/service-mesh recommended deployment).

**Testing.** `crates/hydracache-cluster-transport-axum/tests/cluster_auth.rs`

- `unauthenticated_peer_fetch_is_rejected` (integration): no credential → 401-style
  reject + counter.
- `unauthorized_route_is_denied` (integration): valid identity, `Authorizer`
  denies `Admin`; assert deny.
- `rotation_window_accepts_old_and_new` (integration): rotate keys; assert calls
  signed with previous-window credential still verify during the window, then fail
  after.
- `missing_provider_refuses_replication_routes_unless_acked` (unit): no provider,
  no ack → replication routes disabled; with ack → enabled + loud flag.
- `raft_transport_requires_identity` (integration): unauthenticated raft append is
  rejected (W1 routes are covered too).
- Run: `cargo test -p hydracache-cluster-transport-axum --locked cluster_auth`.

**Pros.** Turns the prior advisory flags into real enforcement on every route; key
rotation is downtime-free; the insecure path is an explicit, loud, acknowledged
choice.

**Risks.** Auth on the hot peer-fetch path adds per-call verification cost.
Mitigation: verification is cheap (token/identity check, not per-call crypto
handshake when behind mTLS/mesh), and the cost is benchmarked against the `0.37`
performance budget.

---

## W7. Operational Surface

**Problem / motivation.** A grid you cannot observe and operate is not production
ready. `0.41` defined Ops & SLOs and metric-cardinality discipline; `0.42` must
deliver the concrete operator surface: a read-only status view, dashboards/alerts
as shippable artifacts, repair-debt handling automation, and a repair runbook good
enough that an on-call who has never read the code can act.

**Design / contract.** Provide a read-only `SHOW`-like status surface (a
Hazelcast/SQL `SHOW CLUSTER`-style read-only command over the actuator) that
returns committed topology, per-partition replication health, repair-debt, quorum
posture (W5 strong/degraded), and split-brain reports (W4) — all from the
diagnostics snapshot, honoring the `0.41` cardinality rule (per-entity detail in
the snapshot, not in metrics). Ship dashboards and alert rules as artifacts under
`docs/cluster/dashboards/` (Prometheus alert rules + a Grafana JSON), wired to the
exported metrics. Automate repair-debt handling: when `tombstone_repair_debt` or
`replication_lag` crosses a threshold, the node enters a documented degraded mode
that throttles new replication admission and prioritizes anti-entropy (B6), and
the runbook documents the operator response.

**Rust sketch.**

```rust
// crates/hydracache-observability/src/status.rs
pub struct ClusterStatus {
    pub committed_epoch: ClusterEpoch,
    pub leader: Option<ClusterNodeId>,
    pub members: Vec<MemberStatus>,
    pub partitions_under_replicated: u64, // aggregate gauge, not per-partition label
    pub repair_debt: u64,
    pub quorum_posture: QuorumPosture,    // Strong | DegradedSessionRyow
    pub last_split_brain: Option<SplitBrainReport>,
}

// read-only actuator route: GET /cluster/status -> ClusterStatus (JSON)
```

**Step-by-step implementation.**

1. Add `ClusterStatus` assembled from the diagnostics snapshot; expose a read-only
   `GET /cluster/status` actuator route (no mutation; authz via W6 `Admin` or a
   read-only role).
2. Ship `docs/cluster/dashboards/` artifacts: Prometheus alert rules (leader
   flaps, repair-debt, replication-lag, auth-rejected, split-brain) and a Grafana
   dashboard JSON keyed to the exported metric names.
3. Implement repair-debt degraded mode: threshold-driven throttle of replication
   admission + anti-entropy prioritization; export the mode as a gauge.
4. Write the repair runbook `docs/cluster/runbooks/repair.md`: symptoms → metric →
   action, including split-brain merge review (W4) and rotation (W6).
5. Add a "still not distributed transactions" warning to the status docs.

**Testing.** `crates/hydracache-observability/tests/operational_surface.rs`

- `cluster_status_is_read_only_and_complete` (integration): `GET /cluster/status`
  returns every documented field and performs no mutation.
- `repair_debt_threshold_enters_degraded_mode` (integration): drive repair-debt
  over threshold; assert the throttle engages and the mode gauge flips.
- `status_honors_cardinality_rule` (unit): assert per-partition detail is in the
  snapshot/status JSON, never registered as a labeled metric series.
- `alert_rules_reference_existing_metric_names` (unit): parse the shipped alert
  rules; assert every referenced metric is actually registered.
- Run: `cargo test -p hydracache-observability --locked operational_surface`.

**Pros.** Makes the grid operable by an on-call without code spelunking; debt is
self-throttling and the runbook closes the loop from alert to action.

**Risks.** Dashboards/alerts drift from metric names over time. Mitigation: the
`alert_rules_reference_existing_metric_names` test fails CI when they drift.

---

## Deferred To 0.43 (Explicit)

- **Distributed transactions.** Still a hard non-goal. Cross-node atomic commit
  remains out of scope; the prominent warning stays in all cluster docs.
- **Multi-region / zone-aware placement.** Topology-aware replica placement (the
  ScyllaDB `NetworkTopologyStrategy` analogue) is a natural extension of the A3
  `ClusterReplicationStrategy` but materially expands the placement and failure
  model; it is deferred so `0.42` can prove single-topology production readiness
  first.
- **Tiered/SSD value spill for non-replicated local caches.** Out of scope; `0.42`
  durability is for replicated control-plane and replicated values only.

## Fault Model and Test Tiering

`0.42` reuses the `0.41` shared fault model and harness verbatim
(`crates/hydracache/tests/support/fault_injector.rs`) and its determinism
contract: fault injection is seeded, every chaos test logs and replays its seed,
and correctness assertions use logical signals (epoch, version, applied/commit
index) — never wall-clock thresholds. The W1/W3/W4/W5 chaos suites compose the
enumerated faults (crash, kill, rejoin-with-higher-generation, symmetric and
asymmetric partition, loss/dup/reorder, slow disk/node, bounded clock skew,
permanently-offline backup) rather than re-implementing partition logic.

| Tier | Scope | When | Command shape |
| --- | --- | --- | --- |
| fast | unit + deterministic property (merge policy, AIMD, quorum overlap, cardinality) | every PR | `cargo test --workspace --locked` |
| integration | in-memory multi-node, durable-engine restart, HTTP + raft routes, auth | every PR (durable engine: integration job) | `cargo test --workspace --locked` |
| chaos/soak | seeded fault injection, partition, split-brain heal, leader crash under load, membership churn | nightly / pre-release | `cargo test --workspace --locked -- --ignored` |
| Docker | testcontainers-backed durable-engine + multi-process raft | nightly / pre-release | Docker gate |

## Test Matrix Closure (the 0.41 list, now required to PASS)

`0.41` enumerated the test matrix "required before any future production-grid
claim". `0.42` is that claim, so each line must now pass on real durable,
multi-node, fault-injected suites — not just in-memory placeholders:

- deterministic placement (A3) — passes (carried from 0.41).
- in-memory multi-node replication — passes; **plus** durable-store replication
  (W2).
- HTTP replication route tests + **raft transport route tests** (W1, W6).
- auth and wire-compatibility tests — now **enforced** (W6) + COMPAT registers
  (W1/W2).
- failover/repair property and chaos tests (W3) under load.
- chaos/partition simulation **including split-brain heal** (W4).
- long-running soak with membership churn.
- large-value and memory-pressure tests (byte caps W2).
- rolling-upgrade compatibility tests (raft log + value-record formats).
- persistence/restart tests for metadata (W1) — **required**, not optional.
- persistence/restart tests for values (W2) — **required** in 0.42.
- external-consumer tests against published crates.

## Release Gates For 0.42

Focused:

```powershell
cargo test -p hydracache-cluster-raft --locked durable_control_plane
cargo test -p hydracache-cluster-raft --locked --features durable-log durable_control_plane
cargo test -p hydracache --locked durable_replicated_values
cargo test -p hydracache --locked replication_under_load
cargo test -p hydracache --locked split_brain
cargo test -p hydracache --locked read_your_writes
cargo test -p hydracache-cluster-transport-axum --locked cluster_auth
cargo test -p hydracache-observability --locked operational_surface
cargo test -p hydracache --locked fault_injector_selftest
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --locked --features durable-log,durable-values
cargo test --workspace --locked -- --ignored   # chaos / soak / split-brain suites
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.42.0` may claim **production distributed-data-grid readiness** (for the single
supported topology) only if **all** of the following boolean conditions hold:

- W1: a durable `RaftLogStore` engine is the default; the control plane replicates
  across members, elects under leader loss, lets no committed command be lost on
  crash, refuses unknown future log-format versions, and `durable_control_plane`
  passes (including the durable-engine and chaos runs).
- W2: replicated values are durable and survive backup restart; sealed bytes only
  are persisted under a `ReplicationKeyProvider`; total-bytes budget rejects (never
  silently drops); `durable_replicated_values` passes.
- W3: replication has adaptive backpressure; B4 promotion runs as a topology op
  with a bounded, measured freeze window; B6 anti-entropy converges after partition
  heal without tombstone resurrection; `replication_under_load` passes (incl.
  chaos).
- W4: split-brain is detected on heal, resolved by a documented `MergePolicy`
  (default `HigherVersionWins` on `(version, epoch)`), runs as a topology op, never
  resurrects a tombstone, and is reported/counted; `split_brain` passes (incl.
  chaos).
- W5: acknowledged writes are visible to quorum reads when
  `read_quorum + write_quorum > replication_factor`; quorum overlap is validated at
  startup and the strong-vs-degraded posture is reported; `read_your_writes`
  passes.
- W6: node identity is enforced on every cluster route (incl. W1 raft routes);
  unauthorized calls are rejected and counted; key rotation is downtime-free; the
  insecure path requires an explicit acknowledgement; `cluster_auth` passes.
- W7: the read-only `GET /cluster/status` surface, shipped dashboards/alerts, and
  repair-debt degraded mode exist; alert rules reference only registered metrics;
  `operational_surface` passes.
- The full `0.41` Test Matrix passes on durable, multi-node, fault-injected suites
  (not in-memory placeholders).
- Docs keep the prominent **"still not distributed transactions"** warning and
  document multi-region/zone-aware placement as deferred to `0.43`.

If any condition fails, `0.42.0` ships **without** the production-grid claim,
documents exactly which work item(s) did not land, and the claim moves to a later
release.
