# HydraCache 0.44.0 — Deterministic Simulation Testing (DST) & Storage Fault Model — Codex Execution Plan

> **At a glance**
> - **Kind:** foundation release / Codex execution plan. Release slot **0.44.0** (the feature line was renumbered down by one to make room — see "Release slotting").
> - **What:** a TigerBeetle-style deterministic, seed-driven, whole-cluster simulator (`hydracache-sim`) that drives the cluster, a simulated network, and a fault-injecting simulated storage under one logical clock, checking safety invariants every step and reproducing any failure from its seed.
> - **Why:** turn the correctness wedge into a *provable* one — find consensus/storage/consistency bugs that integration tests and even Jepsen miss, with millions of reproducible cluster-steps in CI (RULES R-5).
> - **After (depends on):** 0.43 debt-closure (real networked control plane) — DST needs a sans-IO seam over that runtime.
> - **Unblocks:** 0.45 active-active, 0.46 resilience, 0.47 causal+ — all developed *against* the simulator.
> - **Status:** planned.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · source analysis: [`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md) §6 (tigerbeetle).

This plan is written for an autonomous coding agent (Codex). Read [`CLAUDE.md`](../../CLAUDE.md),
[`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md) first. Execute work
items in order; one work item = one commit/PR; after each, run its Definition of Done
and `cargo xtask verify`; never push red.

## Release slotting (applied)

DST is **testing infrastructure that must precede the features it validates**, so it
took the `0.44.0` slot and the feature line was renumbered down by one:

- `0.44.0` = this release (Deterministic Simulation Testing & Storage Fault Model).
- active-active `0.44`→**`0.45`**, resilience `0.45`→**`0.46`**, causal+ `0.46`→**`0.47`**,
  ecosystem `0.47+`→**`0.48+`** (DRAFT).
- Rationale: building active-active/causal+ *without* a simulator means re-validating
  them later; with DST first, each subsequent feature is developed against it and ships
  with simulation coverage. DST is also the highest-leverage prod-readiness investment.

The renumber is recorded in `docs/plans/releases.toml` and `docs/plans/INDEX.md`. Every
new cluster feature from `0.45` on must add simulation coverage here.

## Release Theme

Build a deterministic, reproducible, whole-system simulator that is the single source
of correctness confidence for the distributed layer — and the storage fault model it
needs — modeled on TigerBeetle's VOPR (`tigerbeetle/src/vopr.zig`,
`tigerbeetle/src/testing/{cluster,packet_simulator,storage,time}.zig`).

## Non-Goals

- **No real time, real threads, or real sockets inside the simulator.** Determinism is
  the whole point; anything non-deterministic (wall-clock, OS scheduling, real I/O,
  `HashMap` iteration order, unseeded RNG) is banned from the simulated path (R-5).
- **No new consensus engine.** DST validates the existing `raft-rs`-based runtime; it
  does not replace it (VSR remains a documented alternative only).
- **No production code behavior change.** The sans-IO seam (W1) must be a *refactor*
  with zero behavior change in the non-simulated path (R-10); the simulator is
  test/dev-only.
- **No distributed transactions** and no other non-goal relaxation (R-2).

## Inherited Boundary (what this builds on)

- The `0.43` debt-closure runtime (durable raft, networked transport, value plane) is
  the system under test; DST needs a **sans-IO seam** over it (W1).
- The existing seeded harness `crates/hydracache/tests/support/fault_injector.rs` and
  the `DurableControlPlaneCluster` in-process model are the starting points — DST
  generalizes them into a full simulator rather than per-test injection.
- The fault enumeration from `0.41`–`0.45` (crash, partition sym/asym, loss/dup/
  reorder, slow node, clock skew, whole-zone/region loss) is the network/fault menu.

## New crate layout

```
crates/hydracache-sim/
  Cargo.toml            # dev/sim-only; not a runtime dependency of hydracache
  src/lib.rs            # SimWorld, re-exports
  src/rng.rs            # SimRng (seeded, deterministic)
  src/clock.rs          # SimClock (logical time)
  src/storage.rs        # SimStorage (in-mem + fault injection)
  src/network.rs        # SimNetwork (delay/loss/dup/reorder/partition)
  src/node.rs           # SimNode: wraps the sans-IO ClusterNode
  src/workload.rs       # seeded client workload + recorded History
  src/invariants.rs     # InvariantChecker (per-step + end-of-run)
  src/linearizability.rs# per-key linearizability checker (Porcupine-style)
  src/schedule.rs       # FaultSchedule + shrinker (minimization)
  src/bin/vopr.rs       # `cargo run -p hydracache-sim --bin vopr -- --seed S --steps N`
  tests/                # the simulator's own self-tests (meta-tests)
```

Dependencies: a deterministic RNG only (`rand_chacha` or `rand_pcg`, pinned, added to
`deny.toml` allow-list). Everything else is std + the workspace crates. **No `madsim`**
unless W1 proves the sans-IO seam infeasible (then madsim becomes a fallback — see W1
Risk).

---

## W1. Sans-IO seam for the cluster node

**Goal.** Make a cluster node a deterministic state machine that does no I/O itself, so
the simulator can drive it step-by-step. This is the enabling refactor.

**Design / contract.** Extract a `ClusterNode` interface that the production runtime and
the simulator both use:

```rust
// crates/hydracache/src/cluster/node.rs (refactor; behavior-preserving)
pub trait Clock { fn now(&self) -> LogicalTime; }
pub trait Storage { /* append/read/snapshot/fsync, may fault */ }

pub struct ClusterNode { /* raft runtime + value plane, IO-free */ }

impl ClusterNode {
    pub fn tick(&mut self, now: LogicalTime);                 // timers/heartbeats
    pub fn handle_message(&mut self, from: NodeId, msg: Message);
    pub fn handle_client(&mut self, op: ClientOp) -> ClientAck;
    pub fn take_outbound(&mut self) -> Vec<(NodeId, Message)>; // messages to send
    pub fn storage_requests(&mut self) -> Vec<StorageOp>;      // IO to perform
    pub fn apply_storage_result(&mut self, r: StorageResult);
}
```

Production wiring (tokio/transport) becomes a thin driver: real timer → `tick`, inbound
HTTP → `handle_message`, `take_outbound` → real sender, `storage_requests` → real disk.
The simulator provides the same calls deterministically.

**Steps.**
1. Identify all direct I/O / `tokio::time` / `Instant::now` / real-socket / disk calls
   in the runtime; route them through `Clock` / `Storage` / outbound-queue seams.
2. Keep the production driver (`crates/hydracache-cluster-transport-axum`,
   `hydracache-cluster-raft`) as a thin adapter calling the new interface.
3. Ban `Instant::now`/`SystemTime::now`/unseeded `rand`/`HashMap` iteration in the node
   logic; add a clippy/`deny.toml` or a `doc-check`-style lint to enforce it.

**Testing.** `crates/hydracache/tests/sans_io_seam.rs`
- `node_is_deterministic_under_fixed_inputs` (property): same inputs → identical
  outbound + storage requests across runs.
- `production_path_behaviour_unchanged` (integration): existing cluster tests stay green
  (no behavior change).
- Run: `cargo test -p hydracache --locked sans_io_seam` + full suite.

**Pros.** Determinism becomes possible; also clarifies the architecture.
**Risks.** Largest refactor. **Fallback:** if a full sans-IO seam is too invasive, adopt
`madsim` (deterministic tokio shim under `--cfg madsim`) for the simulated build instead
— document the decision in an ADR. Prefer sans-IO.

---

## W2. Deterministic primitives: RNG, clock, storage

**Goal.** The deterministic substrate everything else is built on.

**Design / contract.**
```rust
// crates/hydracache-sim/src/rng.rs
pub struct SimRng(rand_chacha::ChaCha8Rng); // seedable, reproducible
// crates/hydracache-sim/src/clock.rs
pub struct SimClock { now: LogicalTime }     // advanced only by the scheduler
// crates/hydracache-sim/src/storage.rs
pub struct SimStorage { /* per-node in-mem zones + fault config */ }
pub enum StorageFault { LatentReadError, Corruption, TornWrite, Slow(LogicalDuration), LostOnCrash }
```
`SimStorage` models what real disks do: checksummable zones, injected corruption/torn
writes/latent errors, and "lost on crash" for un-fsynced data — exactly the
`tigerbeetle/src/testing/storage.zig` model ("faults a connected cluster can recover
from").

**Steps.**
1. Implement `SimRng`/`SimClock`.
2. Implement `SimStorage` honoring the `Storage` trait from W1, with a per-zone fault
   policy drawn from `SimRng`.
3. Make crash drop un-fsynced writes; make fsynced writes survive.

**Testing.** `crates/hydracache-sim/tests/primitives.rs`
- `sim_rng_is_reproducible_from_seed` (unit).
- `fsynced_survives_crash_unsynced_lost` (unit).
- `injected_corruption_is_detected_by_checksum` (unit) — ties W9.
- Run: `cargo test -p hydracache-sim --locked primitives`.

**Pros.** Reusable substrate; honest disk model.
**Risks.** Over-faulting makes recovery impossible; bound fault rates and document
"recoverable fault" classes per zone.

---

## W3. SimNetwork (the packet simulator)

**Goal.** A deterministic network with the full fault menu.

**Design / contract.**
```rust
// crates/hydracache-sim/src/network.rs
pub struct SimNetwork { in_flight: BinaryHeap<TimedMessage>, links: LinkMatrix, rng: SimRng }
pub enum LinkFault { Delay(LogicalDuration), Drop, Duplicate, Reorder, PartitionSym, PartitionAsym }
impl SimNetwork {
    pub fn send(&mut self, from: NodeId, to: NodeId, msg: Message, now: LogicalTime);
    pub fn deliverable(&mut self, now: LogicalTime) -> Vec<(NodeId, NodeId, Message)>;
    pub fn partition(&mut self, sides: (&[NodeId], &[NodeId]), mode: PartitionSymmetry);
    pub fn heal(&mut self);
}
```
Generalizes `crates/hydracache/tests/support/fault_injector.rs` (which already has
`lose_zone`) to a full delay/loss/dup/reorder/partition model, deterministic from seed.

**Testing.** `crates/hydracache-sim/tests/network.rs`
- `same_seed_same_delivery_order` (property).
- `symmetric_and_asymmetric_partition` (unit).
- `heal_drains_in_flight` (unit).
- Run: `cargo test -p hydracache-sim --locked network`.

---

## W4. SimWorld driver + scheduler

**Goal.** The turn-based engine that owns everything and runs the loop.

**Design / contract.**
```rust
// crates/hydracache-sim/src/lib.rs
pub struct SimWorld {
    rng: SimRng, clock: SimClock,
    nodes: Vec<SimNode>, network: SimNetwork, storage: Vec<SimStorage>,
    schedule: FaultSchedule, history: History, checker: InvariantChecker,
}
impl SimWorld {
    pub fn new(seed: u64, cfg: SimConfig) -> Self;
    pub fn run(&mut self, steps: u64) -> SimOutcome; // Ok | InvariantViolated{seed, step, trace}
}
```
Each step (deterministic order, all choices from `rng`): advance clock → apply scheduled
faults (crash/restart/partition/storage) → deliver due messages → `tick` + drain each
node's outbound & storage IO → issue workload ops → **run the invariant checker**.

**Testing.** `crates/hydracache-sim/tests/world.rs`
- `run_is_reproducible_from_seed` (property): two runs, same seed → identical
  `SimOutcome` + identical history hash.
- `healthy_cluster_makes_progress` (integration): no faults → commits advance.
- Run: `cargo test -p hydracache-sim --locked world`.

---

## W5. Workload generator + history

**Goal.** Seeded client operations and a recorded history for checking.

**Design / contract.** Generate `get/put/invalidate/compare_and_set/session reads` with
random keys/values/consistency-levels from `rng`; record each as
`Event { client, op, invoked_at, returned_at, result }` in a `History` (the input to the
linearizability + session checkers).

**Testing.** `crates/hydracache-sim/tests/workload.rs`
- `workload_is_reproducible` (property).
- `history_records_invocation_and_response_ordering` (unit).
- Run: `cargo test -p hydracache-sim --locked workload`.

---

## W6. Invariant checkers

**Goal.** Catch any safety violation, every step and at end-of-run.

**Design / contract.** A composable `InvariantChecker` with checks (each ties to a RULES
invariant / release contract):
- **Consensus safety:** committed log prefix-agrees across all nodes; no committed entry
  is ever lost or reordered (A1/A2).
- **Durability:** after crash+recovery, committed state is intact (0.42 W1).
- **No tombstone resurrection:** a deleted key never reappears (A5).
- **Convergence:** absent new writes, replicas converge to one value (0.44 W2).
- **Read-your-writes / monotonic / writes-follow-reads:** per-session (0.46).
- **No panic / no failed assertion / no deadlock** (progress within a step budget).
- **Resource bounds:** queues/hints/tombstones stay within budgets (R-6, bounded-alloc).

**Testing (meta-tests — prove the checker catches bugs).**
`crates/hydracache-sim/tests/invariants.rs`
- `checker_flags_injected_lost_commit` (unit): feed a deliberately broken history →
  violation reported (a checker that never fails is useless).
- `checker_flags_tombstone_resurrection` (unit).
- `checker_passes_a_known_good_history` (unit).
- Run: `cargo test -p hydracache-sim --locked invariants`.

---

## W7. Linearizability checker

**Goal.** Verify per-key operation histories are linearizable (the gold-standard
consistency check), Porcupine/Knossos-style.

**Design / contract.** A bounded per-key checker: model each key as a register; search
for a valid linearization of concurrent ops consistent with real-time order; bound the
search (window) to stay tractable. Used by W6 for `Quorum`/strong ops; weaker
consistency levels use the appropriate (session/causal) checker instead.

**Testing.** `crates/hydracache-sim/tests/linearizability.rs`
- `linearizable_history_accepted` / `non_linearizable_history_rejected` (unit).
- `checker_is_deterministic` (property).
- Run: `cargo test -p hydracache-sim --locked linearizability`.

**Risks.** Linearizability checking is NP-hard in general; mitigate with per-key scope +
bounded concurrency window + timeouts, and fall back to weaker invariants beyond the
window.

---

## W8. Failure reporting, replay & shrinking

**Goal.** Make every failure reproducible and minimal.

**Design / contract.** On violation, emit `seed`, `step`, the fault schedule, and a
human-readable trace; `vopr --seed S --steps N` reproduces exactly. A **shrinker**
(`schedule.rs`) minimizes the fault schedule (binary-search/delta-debugging on injected
faults) to the smallest reproducer.

**Testing.** `crates/hydracache-sim/tests/replay.rs`
- `seed_reproduces_identical_violation` (property).
- `shrinker_preserves_the_violation` (unit): shrunk schedule still fails.
- Run: `cargo test -p hydracache-sim --locked replay`.

---

## W9. Storage scrubber + artifact checksums (TigerBeetle §6.2)

**Goal.** The production-side partner of the storage fault model: detect and repair
corruption.

**Design / contract.** Checksum every durable artifact (raft log entries, value
records, tombstones); a background `Scrubber` periodically re-reads, verifies checksums,
and repairs corrupt blocks from replicas (ties the 0.45 Merkle repair). The simulator
(W2 `Corruption`) exercises it.

**Steps.**
1. Add checksums to durable formats; register the change in `docs/COMPAT.md` (R-4).
2. Add a `Scrubber` to the runtime (rate-limited, opt-in default-on).
3. Drive corruption via `SimStorage` and assert repair.

**Testing.** `crates/hydracache/tests/scrubber.rs` + sim scenario
- `corrupt_block_is_detected_and_repaired_from_peer` (integration, via sim).
- `unrepairable_corruption_is_reported_not_served` (unit) — fail loud (R-3).
- Run: `cargo test -p hydracache --locked scrubber`.

---

## W10. `vopr` binary + CI wiring

**Goal.** Run DST in CI (fast budget on PR, long soak nightly) and expose the
reproduction tool.

**Design / contract.**
- `cargo run -p hydracache-sim --bin vopr -- --seed <S> --steps <N>` (random seed if
  omitted; prints the seed it chose).
- **Fast budget (PR):** a `cargo test -p hydracache-sim` target that runs K seeds × M
  bounded steps deterministically — added to `cargo xtask verify` and CI.
- **Nightly soak:** a scheduled CI job running many seeds / long horizons (and the
  shrinker on any failure), reporting the failing seed.
- Update `docs/GATES.md` with the new "DST (fast budget)" gate and the nightly job.

**Testing.** `crates/hydracache-sim/tests/cli.rs`
- `vopr_seed_flag_is_deterministic` (integration): same `--seed` → same result.
- Run: `cargo test -p hydracache-sim --locked cli` + `cargo xtask verify`.

---

## Fault Model and Test Tiering

DST *is* the fault model's home; it composes the full enumerated menu (crash/kill,
restart+higher-generation, partition sym/asym, loss/dup/reorder/delay, slow disk/node,
clock skew, whole-zone/region loss, storage corruption/torn-write/latent-error) from one
seed (RULES R-5). Tiers:

| Tier | Scope | When | Command |
| --- | --- | --- | --- |
| fast | sim self-tests + small seed budget (K×M bounded) | every PR | `cargo test -p hydracache-sim --locked` (in `cargo xtask verify`) |
| nightly soak | many seeds / long horizons + shrinker | nightly | scheduled CI job running `vopr` |
| reproduce | a specific failing seed | on demand | `cargo run -p hydracache-sim --bin vopr -- --seed <S>` |

## Release Gates

Focused:

```powershell
cargo test -p hydracache --locked sans_io_seam
cargo test -p hydracache-sim --locked primitives
cargo test -p hydracache-sim --locked network
cargo test -p hydracache-sim --locked world
cargo test -p hydracache-sim --locked workload
cargo test -p hydracache-sim --locked invariants
cargo test -p hydracache-sim --locked linearizability
cargo test -p hydracache-sim --locked replay
cargo test -p hydracache --locked scrubber
cargo test -p hydracache-sim --locked cli
```

Full:

```powershell
cargo xtask verify   # includes the DST fast budget + doc-check
cargo run -p hydracache-sim --bin vopr -- --steps 100000   # local soak sample
```

## Final Release Decision

This release (recommended `0.44.0`) may claim a **deterministic simulation testing
foundation** only if **all** hold:

- W1: a sans-IO node seam exists; the production path is behavior-unchanged; node logic
  has no `Instant::now`/unseeded RNG/real I/O.
- W2–W4: a seeded `SimWorld` deterministically drives nodes + network + fault-injecting
  storage under a logical clock; `run(seed, steps)` is bit-for-bit reproducible.
- W5–W7: workload + history + invariant checkers (incl. a real linearizability checker)
  exist, and the **meta-tests prove the checkers catch deliberately-broken histories**.
- W8: any violation is reproducible from its seed and shrinkable to a minimal schedule.
- W9: durable artifacts are checksummed and a scrubber detects+repairs corruption (or
  reports it loud); registered in `docs/COMPAT.md`.
- W10: the fast DST budget runs in `cargo xtask verify` + CI, a nightly soak job exists,
  and `docs/GATES.md` documents both.

If any condition fails, the release ships **without** the DST claim, documents what
landed, and the rest moves to a follow-up.
