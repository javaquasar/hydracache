# HydraCache 0.53.1 Real Raft Election in the Lab — Codex Execution Plan

> **At a glance**
> - **What:** raise the interactive cluster lab from a **modeled** election
>   (`election_source: "sim-model"`) to the **real `raft-rs` consensus** driven
>   deterministically over the simulator's seeded network — executing the "first
>   attempt: drive `raft-rs` deterministically" path that `0.53` W1b deferred to its
>   labelled sim-model fallback. Real terms, vote splits, and re-elections happen under
>   the lab's injected partitions/crashes, and the snapshot reports
>   `election_source: "raft"`.
> - **Why:** the lab's honest weakness (documented in `demo/README.md`) is that leader
>   election is a teaching model, not the product consensus code. The infrastructure to
>   close this already exists: `hydracache-cluster-raft::RaftMetadataRuntime` wraps real
>   `raft-rs` (`RawNode`, `Ready`, campaign/propose/commit, in-memory store) with
>   **serializable messages** (`RaftWireMessage::encode/decode`) and an outbound
>   **`RaftMessageSink`**; the simulator already has a deterministic `SimNetwork`
>   (partition/delay/drop) and `SimClock`. So this is **integration, not new consensus**.
> - **After (depends on):** `0.53.0` (the lab) and the shipped raft runtime (`0.42`/`0.43`
>   `hydracache-cluster-raft`). A point release over `0.53`.
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> lab: [`V0_53_INTERACTIVE_CLUSTER_LAB_PLAN.md`](V0_53_INTERACTIVE_CLUSTER_LAB_PLAN.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its
Definition of Done **and** `cargo xtask verify`; never push red. Determinism/replay (R-5)
is a non-negotiable gate.

## Justification (why this, why now)

`0.53` W1b set an explicit decision gate: *first attempt to drive `raft-rs`
deterministically; if its randomized election-timeout RNG cannot be seeded, fall back to a
labelled sim-model with no product claim.* The implementation took the fallback, so the lab
ships `election_source: "sim-model"` with the disclosure *"not a product consensus claim"*.

Verified against the code, the "first attempt" is now low-risk because the pieces exist —
**but at the right layer**:

- **Drive `raft::RawNode` directly and synchronously — do NOT use `RaftMetadataRuntime`.**
  `RaftMetadataRuntime` and `RaftMessageSink` are **`async` / `tokio`** (`async fn send`,
  `async fn join_member`), which does not fit the synchronous, deterministic, wasm-targetable
  simulator. Instead reuse the **low-level** pieces that `RaftMetadataRuntime` itself uses
  (`crates/hydracache-cluster-raft/src/lib.rs`): the `RawNode::new(&Config, InMemoryRaftLogStore,
  &Logger)` construction pattern (lib.rs ~511/564-566), the synchronous `Ready` loop
  (`has_ready` → `ready()` → `advance()` → `advance_apply()`, mirror `drain_ready` lib.rs
  ~977-1006), `InMemoryRaftLogStore` (implements `raft::storage::Storage`), and
  `RaftWireMessage::encode/decode` for message bytes. The `slog` logger is `Logger::root(slog::
  Discard, o!())` (no logging).
- `crates/hydracache-sim/src/network.rs` (`SimNetwork`) models a deterministic fault-injecting
  network, but `send`/`deliverable` carry `ClusterNodeMessage`, **not** raft bytes. So raft
  traffic rides a **dedicated in-flight queue inside `SimRaftCluster`** that reuses the network's
  **partition decisions** (`SimNetwork::can_deliver(from, to)` and the `partition` set) plus a
  deterministic per-hop latency. Full per-link delay/drop/reorder parity is a stretch goal;
  partition/isolation parity (the main demo behaviour: isolate → real re-election) is in scope.
- `SimClock`/`SimRng` provide seeded logical time; `SimWorld::drive_election` (world.rs ~780-783)
  is the single call-site where the election backend is invoked, and `SimWorld::snapshot`
  (~599) is where election state is read.

So real raft "votes" can be driven through the exact partitions the lab already visualizes.
The **one real seam** is `raft-rs`'s randomized election timeout: override it deterministically
via `raw_node.raft.set_randomized_election_timeout(seeded_value)` (value in `[election_tick,
2*election_tick)`), re-asserting it each tick because `raft-rs` resets it internally on every
term change / role transition.

## Release Theme

Drive **real `raft-rs`** deterministically over the simulator's seeded, fault-injecting
network so the lab shows genuine consensus voting — defaulting on the native **server
(sandbox) path** (no wasm constraint), validating the existing sim-model against real raft,
and resolving the wasm-compat question explicitly. No change to the product runtime, no new
consensus algorithm, and the lab stays a teaching asset (not a release gate).

## Non-Goals

- **No new consensus.** Reuse `raft-rs` via `hydracache-cluster-raft`; do not fork or
  re-implement Raft.
- **No production-runtime change.** This is the *lab/simulator* election; the product
  `hydracache-cluster-raft` / `hydracache-server` paths are untouched.
- **No non-determinism.** Every randomization (election timeout especially) is seeded; the
  same seed reproduces the same leadership history (R-5). A path that cannot be made
  deterministic does not ship as "raft".
- **Not a correctness gate.** The lab remains teaching/DevRel; DST + `cargo xtask verify`
  stay the authoritative gates.
- **No silent fidelity claim.** Wherever the model is still used (e.g. wasm, if raft cannot
  compile there), the snapshot/UI keep saying `sim-model` — never present a model as raft.

## Inherited Boundary (assumes 0.53 + shipped raft runtime)

- **`hydracache-cluster-raft`**: the real raft seam — `RaftMetadataRuntime`,
  `RaftWireMessage`, `RaftMessageSink`/`InMemoryRaftMessageSink`, `InMemoryRaftLogStore`,
  `Config::ticks(election_tick, heartbeat_tick)`. Reuse; do not modify the product runtime.
- **`hydracache-sim` `SimNetwork`/`SimClock`/`SimRng`**: the deterministic transport + clock
  the raft messages ride; the fault model is already there.
- **`0.53` `ElectionDriver`** (`election.rs`): kept as the labelled `sim-model` fallback; not
  deleted. The snapshot's `election_source` / `election_disclosure` already distinguish them.
- **Snapshot schema** (`snapshot.rs`): `NodeView { term, vote_state, voted_for,
  votes_received }` and `election_source` already exist — real raft populates them; bump
  `SIM_SNAPSHOT_SCHEMA_VERSION` only if a new field is added (R-4).
- **Demo engine modes** (`demo/app.js`): `engine=wasm` (default) vs `engine=server`
  (sandbox `/sim/*`). The sandbox is native Rust — the natural home for real raft first.

## Dependency Graph

```
0.53 lab + hydracache-cluster-raft (real raft-rs)
        │
        ▼
W1 SimRaftCluster: drive real raft-rs deterministically over SimNetwork (seeded timeout)  ◄ foundation
        │
        ├──────────► W2 validate sim-model vs real raft (same seeds/faults)
        ▼
W3 surface real raft in the snapshot (election_source:"raft") + use it on the server path
        │
        ▼
W4 wasm-compat spike for raft-rs + explicit decision (raft in wasm, or server=raft/wasm=model)
        │
        ▼
W5 docs + UI election-source disclosure
```

W1 is the foundation (the deterministic raft-over-sim harness); W2 validates the existing
model with it; W3 puts it on screen (server path); W4 resolves wasm; W5 tells the truth.

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests) / Risk & rollback.**

## Execution order & preflight (read first)

1. **Preflight (no behaviour change).** Add `hydracache-cluster-raft`, `raft`, `slog` to
   `crates/hydracache-sim/Cargo.toml` and prove the native build is green
   (`cargo build -p hydracache-sim --locked`) **before** writing the harness. Confirm the exact
   public API of `InMemoryRaftLogStore` (its `Storage` impl + the write seam used by
   `RaftMetadataRuntime::drain_ready`) and that `RawNode`/`Config`/`StateRole`/
   `set_randomized_election_timeout` are reachable. If `InMemoryRaftLogStore`'s constructor or
   write API is not `pub`, the first commit makes the minimal additive `pub` change in
   `hydracache-cluster-raft` (no logic change) and records it in the PR.
2. **W1 → (W2 ∥ W3) → W4 → W5.** W1 is the foundation. The
   `election_determinism_holds_over_1000_seeds` gate in W1 is the hard prerequisite: **W3 must
   not expose `election_source:"raft"` until that gate is green** (R-5). W2 can proceed in
   parallel with W3 once W1 lands.
3. **Never push red; one work item = one commit/PR; run `cargo xtask verify` after each.** On
   Windows, `cargo test` over many integration binaries can fail to link with `LNK1104` (a
   Defender lock, not a logic failure) — retry, or use `cargo test -p hydracache-sim --lib` for
   the unit layer; the failure text is `linking with link.exe ... LNK1104`, distinct from a real
   test failure.

---

## W1. `SimRaftCluster` — drive real `raft::RawNode` deterministically over `SimNetwork`

**Goal.** A synchronous harness that runs N real `raft-rs` nodes (bare `RawNode`, not the async
runtime) whose messages ride a deterministic queue gated by `SimNetwork`'s partition state and
ticked on the world's logical step counter, with a **seeded election timeout** so the same seed
reproduces the same leadership history.

**Files.**
- New `crates/hydracache-sim/src/sim_raft.rs` (`SimRaftCluster`, `SimRaftNode`).
- `crates/hydracache-sim/Cargo.toml`: add `hydracache-cluster-raft`, `raft`, `slog` deps.
  (Check feature flags so it builds with the workspace's pinned `raft` 0.7 / `protobuf 2.x`,
  TD-0002.) Re-export `SimRaftCluster` from `crates/hydracache-sim/src/lib.rs`.

**Data model.**
```rust
use hydracache::ClusterNodeId;
use hydracache_cluster_raft::{InMemoryRaftLogStore, RaftWireMessage};
use raft::{Config, RawNode, StateRole};
use slog::{o, Discard, Logger};

struct SimRaftNode {
    raw: RawNode<InMemoryRaftLogStore>,
}

pub struct SimRaftCluster {
    seed: u64,
    election_tick: usize,                          // e.g. 10 (matches RaftMetadataRuntimeConfig)
    heartbeat_tick: usize,                         // e.g. 3
    nodes: BTreeMap<u64, SimRaftNode>,             // raft id -> node
    names: BTreeMap<u64, ClusterNodeId>,           // raft id -> "node-N"
    ids: BTreeMap<ClusterNodeId, u64>,             // "node-N" -> raft id
    // (deliver_at_step, from_id, to_id, wire); kept sorted by deliver_at then (from,to,seq)
    inflight: VecDeque<(u64, u64, u64, u64, RaftWireMessage)>,
    next_seq: u64,
    logger: Logger,                                // Logger::root(Discard, o!())
}
```
Raft ids are assigned deterministically: `node-0 -> 1`, `node-1 -> 2`, … (raft requires ids ≥ 1).

**Steps.**
1. **Construct nodes** (mirror lib.rs ~511/557-566). For the initial member set `peers: Vec<u64>`:
   ```rust
   let storage = InMemoryRaftLogStore::new_with_conf_state((peers.clone(), vec![]));
   let cfg = Config { id, election_tick, heartbeat_tick, ..Default::default() };
   cfg.validate().map_err(..)?;                      // raft-rs validates the config
   let mut raw = RawNode::new(&cfg, storage, &logger).map_err(..)?;
   seed_election_timeout(&mut raw, seed, id);
   ```
   If `InMemoryRaftLogStore::new_with_conf_state` is private, expose a constructor or use the
   path `RaftMetadataRuntime` uses (line ~511).
2. **Seed the election timeout (the W1b seam).**
   ```rust
   fn seed_election_timeout(raw: &mut RawNode<InMemoryRaftLogStore>, seed: u64, id: u64) {
       let base = raw.raft.election_timeout();        // == election_tick
       let span = base.max(1);
       let mut h = seed ^ id.rotate_left(17)
           ^ raw.raft.term.wrapping_mul(0x9e37_79b9_7f4a_7c15);
       h ^= h >> 29; h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
       let t = base + (h as usize % span);            // [election_tick, 2*election_tick)
       raw.raft.set_randomized_election_timeout(t);
   }
   ```
   Re-assert it **every tick** (raft-rs resets the randomized timeout on `reset()` during role
   transitions, so a one-shot set is not enough).
3. **`step(now_step, live: &BTreeSet<ClusterNodeId>, network: &SimNetwork)`** does, in order:
   - **Deliver due messages.** Pop every `inflight` entry with `deliver_at <= now_step`; skip if
     the target isn't `live` or `!network.can_deliver(&from_name, &to_name)` (partitioned —
     counts as a real raft message drop); else `let m = wire.decode()?; node.raw.step(m)?;`.
   - **Tick live nodes.** For each `live` node: `seed_election_timeout(...)` then `raw.tick()`.
   - **Drain `Ready` and route outbound** (mirror lib.rs `drain_ready` ~977-1006):
     ```rust
     while node.raw.has_ready() {
         let mut ready = node.raw.ready();
         route(id, ready.take_messages(), now_step, network, live);  // §route below
         if !ready.snapshot().is_empty() { /* apply snapshot to the store */ }
         if !ready.entries().is_empty() { store.wl().append(ready.entries())?; }
         if let Some(hs) = ready.hs() { store.wl().set_hardstate(hs.clone()); }
         // committed entries: apply (no-op state machine is fine for election fidelity)
         let mut light = node.raw.advance(ready);
         route(id, light.take_messages(), now_step, network, live);
         node.raw.advance_apply();
     }
     ```
     The exact `store.wl()` write API is on the `RaftLogStore` / `InMemoryRaftLogStore` type —
     follow how `RaftMetadataRuntime::drain_ready` persists entries/hardstate; reuse it.
   - **`route(from, msgs, now_step, network, live)`:** for each `raft::eraftpb::Message`:
     `let to = m.to;` resolve names; if `from == to` step it back into the same node immediately
     (raft-rs sends some self-messages); else if `network.can_deliver(&from_name,&to_name)` push
     `(now_step + 1, from, to, next_seq, RaftWireMessage::encode(&m)?)` onto `inflight` (1-step
     deterministic latency), keeping `inflight` sorted. Bound `inflight` length and count drops.
   - **Recompute the leader** from `raw.raft.leader_id`.
4. **Membership** (used by W3 add/remove). `add_node(name, now_step)`: assign next raft id, build
   the node, and on the current leader `propose_conf_change(ctx, ConfChange{AddNode, node_id})`;
   apply the resulting committed conf change via `raw.apply_conf_change(&cc)` on each node when it
   commits. `remove_node(name)`: leader proposes `RemoveNode`. (Real raft membership change — this
   is *more* faithful than the model's add/remove.)
5. **Accessors for the snapshot (W3):**
   ```rust
   pub fn leader(&self) -> Option<ClusterNodeId>;         // from raw.raft.leader_id != 0
   pub fn term(&self) -> u64;                             // raw.raft.term of the leader (or max)
   pub fn node_state(&self, name: &ClusterNodeId)
       -> Option<(NodeFsmStateLike, u64, Option<ClusterNodeId>, usize)>;
   // role: raw.raft.state -> StateRole::{Follower,Candidate|PreCandidate,Leader}
   // term: raw.raft.term ; voted_for: raw.raft.vote (0 = none) -> name
   // votes: count of granted votes when candidate (raw.raft.prs() / poll), else 0
   ```
   Map `StateRole` exactly like lib.rs ~405-407.

Keep **all** time logical (the `now_step` counter; no wall-clock), all randomness seeded
(`seed` + id + term only). No `tokio`, no `async`, no `RaftMetadataRuntime`/`RaftMessageSink`.

**DoD.** `crates/hydracache-sim/tests/sim_raft.rs`
- `sim_raft_elects_single_leader_deterministically` (cold start, 3–5 nodes → exactly one leader).
- `election_determinism_holds_over_1000_seeds` — for 1000 seeds, two independent runs produce an
  **identical** `(leader, term)` history and identical `inflight` ordering. **This is the gate
  that authorises `election_source:"raft"` in W3.**
- `leader_loss_triggers_real_reelection` (remove the leader from `live` → a higher term + new
  leader emerges).
- `partition_minority_cannot_elect` (partition 2-of-5 away via `SimNetwork::partition` →
  minority side does not elect; majority keeps/elects a leader — real raft safety).
- `conf_change_add_then_remove_node_is_deterministic` (membership change replays identically).
- Run: `cargo test -p hydracache-sim --locked sim_raft` + `cargo xtask verify`.

**Risk & rollback.** (a) **Determinism completeness** — if any raft-rs randomization point is
missed, the 1000-seed test diverges; the seam is the election timeout, but also confirm no
`HashMap` iteration order leaks into message order (use `BTreeMap`/sorted `inflight`). Until the
1000-seed test is green, W3 must not expose `"raft"`. (b) **`raft`/`protobuf`/`slog` build** —
adding these to `hydracache-sim` must keep the native build green (wasm is W4's concern). Revert
removes `sim_raft.rs` + the deps.

## W2. Validate the sim-model against real raft

**Goal.** Run the `0.53` `ElectionDriver` (sim-model) and `SimRaftCluster` (real raft) on the
**same seeds and the same injected faults**, and assert the model tracks real raft's
leadership decisions — turning the lab's claim into "model **validated** against real raft",
and catching where the model lies.

**Files.** `crates/hydracache-sim/tests/election_model_vs_raft.rs` (new).

**Steps.**
1. For a battery of seeds + scripted fault sequences (partition, crash, rejoin), step both
   drivers in lockstep over an identical `SimNetwork` fault schedule. Reuse the `SimRaftCluster`
   from W1 and a fresh `ElectionDriver` (`election.rs`) seeded the same; advance both with the
   same `(now_step, live_set)` sequence so the only variable is the election logic.
2. Assert agreement on the **safety-relevant** facts: at most one leader per term in both; the
   model never reports a leader when real raft has none under the same faults. Liveness timing
   may differ (the model is coarser) — assert *eventual* agreement on the leader within a
   bounded number of steps, and **document** any seed where they diverge.
3. Emit a short fidelity summary (agreement rate, divergence cases) the docs (W5) can cite.

**DoD.** `crates/hydracache-sim/tests/election_model_vs_raft.rs`
- `model_and_raft_agree_on_single_leader_per_term`.
- `model_never_claims_a_leader_raft_denies` (safety direction).
- `model_converges_to_raft_leader_within_bound` (eventual liveness agreement).
- Run: `cargo test -p hydracache-sim --locked election_model_vs_raft`.

**Risk & rollback.** Test-only. If divergences are large, that is a real finding to document
(and a reason to prefer W3's real-raft path in the demo); it does not block the release.

## W3. Surface real raft in the snapshot; default it on the server path

**Goal.** Add an election-backend switch to `SimWorld` so it can drive `SimRaftCluster` instead
of the `ElectionDriver` model, populate the snapshot's **existing** election fields from real
raft state, set `election_source: "raft"`, and make raft the default on the native **server
(sandbox)** engine. The model stays available and is selected only when raft is unavailable.

**Files.** `crates/hydracache-sim/src/election.rs` (add `ElectionSource::Raft`),
`crates/hydracache-sim/src/world.rs` (backend field + the call-sites below),
`crates/hydracache-sim/src/snapshot.rs` (no new field if possible — reuse `NodeView`),
`crates/hydracache-sandbox/...` (default `/sim/*` to the raft backend).

**Steps.**
1. **`ElectionSource::Raft`** in `election.rs`: `as_str() = "raft"`,
   `disclosure() = "real raft-rs consensus driven deterministically over the seeded simulator
   network (not the full product transport/persistence)"`, and
   `carries_product_consensus_claim() = true` (the *voting* is real; still not the product
   runtime — say so precisely in the disclosure).
2. **Backend field on `SimWorld`.** Add `enum ElectionBackend { Model, Raft }` and
   `election_backend: ElectionBackend`. Keep `election: ElectionDriver` (model) **and** add
   `raft: Option<SimRaftCluster>` constructed when the backend is `Raft`. Default constructor
   keeps `Model`; add `SimWorld::with_raft_election(seed, cfg)` (or a builder flag) used by the
   sandbox.
3. **Switch the single election call-site.** In `drive_election` (world.rs ~780-783) branch:
   ```rust
   match self.election_backend {
       ElectionBackend::Model => self.election.step(self.steps, &live_nodes),
       ElectionBackend::Raft  => self.raft.as_mut().unwrap()
                                     .step(self.steps, &live_nodes, &self.network),
   }
   ```
   Note the raft backend reads `&self.network` for partition decisions — watch the borrow (it
   already borrows `self.network` elsewhere; pass the live set + a `&SimNetwork` explicitly).
4. **Mirror the membership/liveness mutations.** The model is mutated at world.rs ~442/482/493/
   510/530 (`restore_node`/`remove_node`/`add_node`). For the raft backend, route the same
   intents: `add_node` → `raft.add_node(name, self.steps)` (conf-change AddNode); `disable_node`
   → `raft.remove_node(name)` (conf-change RemoveNode, matching the `0.53.1`/`0.53` decommission
   semantics); `enable_node`/`restart`/`rejoin` → `raft.add_node`/re-include in `live` (real raft
   catch-up handles the rest). Crash/isolate need **no** membership change — they just leave the
   `live` set, exactly like the model, and real raft re-elects.
5. **Populate the snapshot from raft.** In `snapshot()` (world.rs ~599), when the backend is
   `Raft`, build the per-node election view from `SimRaftCluster::node_state(name)` instead of
   `self.election.snapshot()`; set `election_source` / `election_disclosure` from
   `ElectionSource::Raft`. Reuse the **existing** `NodeView { term, vote_state, voted_for,
   votes_received }` fields — **no schema bump needed** (assert this in a test; if a field must
   be added, bump `SIM_SNAPSHOT_SCHEMA_VERSION` per R-4). Derive `formation_phase` from real raft
   (no leader → `electing`/`degraded`; leader present → `formed`).
6. **Default the sandbox to raft.** In `hydracache-sandbox`, the `/sim/new` (or equivalent)
   handler constructs the world with `ElectionBackend::Raft`. The WASM default stays `Model`
   until W4 decides otherwise.

**DoD.** `crates/hydracache-sim/tests/raft_snapshot.rs`
- `raft_backend_reports_election_source_raft`.
- `node_views_reflect_real_raft_term_and_leader` (the leader's `vote_state == "leader"`, term
  matches `raft.term()`).
- `c3_invariants_hold_against_real_raft` — the `0.53` C3 election/topology invariants
  (≤1 leader/term, leader-requires-quorum, monotonic indices) pass on **raft** state.
- `raft_snapshot_uses_existing_schema_version` (no unexpected schema bump).
- `crates/hydracache-sandbox` route test: `sim_new_defaults_to_raft_election_source`.
- Run: `cargo test -p hydracache-sim --locked raft_snapshot` + sandbox `/sim/*` route tests +
  `cargo xtask verify`.

**Risk & rollback.** Gated on W1's 1000-seed determinism being green — **do not expose
`"raft"` otherwise**. Borrow-checker friction around `&self.network` in `drive_election` is the
likely snag (split the borrow or pass owned ids). Real raft's liveness *timing* differs from the
model (it is more accurate, not a regression). Revert flips the default backend back to `Model`.

## W4. wasm-compat spike for `raft-rs` + explicit decision

**Goal.** Determine whether `raft-rs` (+ `protobuf`/`slog`, TD-0002) compiles and stays
deterministic under `wasm32-unknown-unknown`, and **decide explicitly** how each engine mode
presents election.

**Files.** spike build of `hydracache-sim-wasm` with the raft backend; a short ADR
`docs/adr/00NN-raft-in-wasm-lab.md` recording the outcome.

**Steps.**
1. Attempt the build with the raft backend reachable from wasm:
   ```powershell
   cargo build -p hydracache-sim --target wasm32-unknown-unknown --locked
   wasm-pack build crates/hydracache-sim-wasm --target web --out-dir ../../demo/pkg --release -- --locked
   ```
   Check: does `raft` + `protobuf 2.x` + `slog` compile to `wasm32-unknown-unknown` (no
   native-only deps, no threads, no `std::time`/IO on the hot path)? Common blockers: `protobuf`
   codegen, `getrandom` needing the `js` feature, `slog` async drains. If `getrandom` is pulled
   in, enable its `js` feature for the wasm target — but note **the sim must not depend on
   `getrandom` for election randomness** (that lives in our seeded timeout seam, not raft-rs's
   RNG, so determinism is independent of `getrandom`).
2. **Decide and record (ADR):**
   - **If it works:** enable `election_source: "raft"` in the default wasm demo too.
   - **If it does not:** keep wasm on the labelled `sim-model` (validated by W2) and document
     that the **server engine is the high-fidelity mode** (`?engine=server`). No silent
     fidelity claim in wasm.
3. Make the chosen behaviour the default and add a CI guard so the wasm build can't silently
   regress the decision.

**DoD.** ADR committed with the build result + decision; `cargo xtask verify`; the demo build
matches the ADR (raft-in-wasm or documented server-only). A test/assert that the wasm snapshot's
`election_source` equals the ADR's decision.

**Risk & rollback.** Pure investigation + a config decision. The fallback (server=raft,
wasm=validated-model) is already honest and shippable, so this item cannot block the release.

## W5. Docs + UI election-source disclosure

**Goal.** Tell the truth on screen and in the README: where election is real raft vs a
validated model, and how to run the high-fidelity mode.

**Files.** `demo/README.md` (update the Fidelity section), `demo/app.js` + `demo/style.css`
(make the `election_source` badge prominent next to the verdict, e.g. a "raft" / "sim-model"
chip with the disclosure on hover).

**Steps.**
1. Update the README Fidelity section: election is **real raft-rs** in the server engine
   (and in wasm iff W4 enabled it); the sim-model is now **validated against real raft** (cite
   the W2 summary).
2. Surface `election_source` + `election_disclosure` as a visible chip in the UI (the data
   already exists in the snapshot; today it is only in the banner text).
3. Keep the "teaching lab, not a release gate" framing intact.

**DoD.** `demo/tests/ui_smoke.spec.js`: `election_source_chip_is_visible_and_labelled`. README
updated. `cargo xtask verify` + demo smoke green.

**Risk & rollback.** Docs/UI only; revert restores prior README/badge.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green; demo smoke green.
- **W1 determinism gate passes** (`election_determinism_holds_over_1000_seeds`) before any
  `election_source: "raft"` is exposed (R-5).
- `0.53` C3 election/topology invariants hold against **real raft** state (W3).
- The sim-model is either used only where raft is unavailable, and **always labelled** as
  such — no model is ever presented as raft (R-3/R-7).
- W4 ADR committed; the wasm demo's election source matches the recorded decision.
- `demo/README.md` Fidelity section updated; UI election-source chip visible.
- `releases.toml` + `INDEX.md` updated to `0.53.1`. No numeric self-score (R-7).
