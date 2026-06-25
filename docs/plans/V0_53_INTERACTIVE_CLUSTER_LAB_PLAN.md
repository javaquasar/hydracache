# HydraCache 0.53.0 Interactive Cluster Lab — Codex Execution Plan

> **At a glance**
> - **What:** evolve the `0.50` browser simulator into a full **interactive cluster lab** with
>   a **liquid-glass (glassmorphism)** UI and **multiple modes**. **Manual mode:** the user
>   pushes client events through the UI and *watches* the write diverge across replicas,
>   replicate, converge, and a **listener receive** it; can **isolate / disable / rejoin** a
>   node and *see* it drop out, trigger **re-election**, rejoin, and **catch up/sync**. On
>   **scenario start the nodes begin disconnected** and the viewer watches the **leader-election
>   vote + connection-formation** happen. **Scripted/mixed modes:** prescribed scenarios cycle
>   through the cluster×client combinations on a loop, yet **every element stays clickable** so
>   the viewer can intervene in the topology mid-run.
> - **Why:** the demo is the "**correctness as a visible product feature**" (`POSITIONING.md`)
>   and the operational-confidence asset for the **Hazelcast-migration pitch**. Today's demo
>   does crash/restart + per-link faults + a static verdict, but it **cannot show the two
>   things that convince an operator**: live **leader election / quorum voting** and a node
>   **rejoining and re-syncing**. Critically, `hydracache-sim` **"has no leader election yet"**
>   (`crates/hydracache-sim/src/snapshot.rs` `NodeView.term` doc) — so this release first makes
>   election **real and deterministic in the simulator**, then makes it visible, rather than
>   animating fiction (R-3).
> - **After (depends on):** `0.50` (browser demo) and `0.52` (entry listeners / event-kind
>   contract, so manual-mode "listener received it" mirrors real bus semantics). Builds on
>   `0.44` (`hydracache-sim`). **Absorbs** the supporting
>   [`V0_50_DEMO_ENHANCEMENTS_PLAN.md`](V0_50_DEMO_ENHANCEMENTS_PLAN.md) (in-flight animation,
>   isolate/overload, add-node, client/subscriber views) — that plan is superseded by this one.
> - **Status:** planned.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> demo release: [`V0_50_INTERACTIVE_SIMULATOR_DEMO_PLAN.md`](V0_50_INTERACTIVE_SIMULATOR_DEMO_PLAN.md)

This plan is written for an autonomous coding agent (Codex). Read [`CLAUDE.md`](../../CLAUDE.md),
[`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md) first. One work item = one
commit/PR; after each, run its Definition of Done **and** `cargo xtask verify`; never push red.
Every new control and the election model must be **seed-deterministic and replayable** (R-5).

## Justification (why this, why now)

Verified against the code, the `0.50` demo over the `0.44` engine already has: per-node
crash/restart, per-link delay/drop, directional partitions (`SimNetwork`), seed reproduce, and
a **real invariant verdict**. The snapshot already carries useful fields — `NodeView { role,
term, commit_index, applied_index, up, crashed }` and `LinkView { state, in_flight }`.

But two gaps block the user-facing vision:

1. **No leader election in the simulator.** `NodeView.term`'s own doc says *"The 0.44 simulator
   has no leader election yet."* The sim drives a raft **log** (`propose`/`committed_payloads_on`
   in `hydracache-cluster-raft`) but does not run **elections** over the seeded network. So a
   "watch the vote / cold-start formation / re-election on leader loss" visualization has **no
   real behavior to show** today. Showing it without modeling it would be theater (R-3).
2. **In-flight signals are only a count.** `LinkView.in_flight: u32` exists, and the engine has
   `TimedMessage { from, to, message, deliver_at }` ("Packet currently in flight",
   `network.rs`), but the snapshot does not surface **individual typed messages**, so the UI
   cannot animate votes/appends/replication/invalidation moving between nodes.

This release closes both at the engine level first, then builds the manual/scripted/mixed UI
and the liquid-glass theme on top. It explicitly stays a **teaching/DevRel asset, not a
correctness gate** (RULES): the authoritative correctness signal remains the DST gates and the
invariant checker, which the lab surfaces but does not replace.

## Release Theme

A seed-deterministic **interactive cluster lab**: real modeled leader election + cold-start
formation, typed in-flight signal animation, a manual client-push → diverge → replicate →
converge → listener-receipt flow, one-click isolate/disable/rejoin with visible re-election and
re-sync, runtime add-node scaling, and scripted/mixed scenario loops — all clickable for live
topology intervention, dressed in a liquid-glass UI — **without** breaking determinism/replay
and **without** becoming a correctness gate.

## Non-Goals

- **Not a correctness gate.** The lab visualizes the seeded engine + the real invariant
  checker; it does **not** add or replace any release gate (DST `0.44`/`0.52` W7 remain
  authoritative). No control is presented as a new guarantee.
- **No non-determinism.** No wall-clock, no real sockets, no `Math.random()` in any control or
  election path. **Election timeouts are logical + seeded** (election is timing-sensitive — this
  is the single biggest determinism risk; it must use `SimClock`/`SimRng`, never real time).
- **No fabricated semantics.** Election/vote/sync visuals reflect **modeled** state only;
  listener/subscriber visuals show only the event kinds the bus carries (coordinate with `0.52`
  W6); unknown message kinds collapse to a generic kind, never invented (R-3).
- **No new product/consensus algorithm.** W1 brings election **into the simulator** by driving
  the existing raft mechanism deterministically; it does not invent a new consensus protocol.
- **No accessibility regression.** Liquid-glass styling must keep WCAG-AA contrast and remain
  readable/operable; "glass" is a skin over an accessible structure, not blurred-unreadable.
- **No always-on cost.** All work lives in the demo + the WASM/sandbox `/sim/*` seam; the
  embedded library and `0.50` defaults are unchanged unless the lab is used.

## Inherited Boundary (assumes 0.44 + 0.50 + 0.52)

- **`SimWorld`** (`crates/hydracache-sim/src/world.rs`): the deterministic driver; election,
  cold-start formation, and the new control verbs extend it using `SimRng`/`SimClock`.
- **`SimSnapshot`** (`crates/hydracache-sim/src/snapshot.rs`): `schema_version` + loud
  unknown-future reject. Every new field (election/vote state, typed in-flight messages,
  clients, subscribers, mode, formation phase) bumps `SIM_SNAPSHOT_SCHEMA_VERSION` and is
  registered in `docs/COMPAT.md` (R-4).
- **`SimNetwork` + `TimedMessage`** (`network.rs`): the typed in-flight source (W2) and the
  partition/isolate primitive (W4).
- **`hydracache-cluster-raft`**: the election mechanism to drive deterministically inside the
  sim (W1) — route its timeouts through `SimClock`; do not fork a second raft.
- **`0.43` online reshard** (`grid/elasticity.rs`): the add-node path (W4).
- **`0.52` W6 entry listeners**: the event-kind contract the manual-mode listener (W3) mirrors.
- **WASM + sandbox parity**: every verb in both `crates/hydracache-sim-wasm/src/lib.rs` and the
  sandbox `/sim/*` POST path (the `ServerHandle` in `demo/app.js`).
- **`demo/` assets** (`app.js`, `index.html`, `style.css`, `scenarios.js`,
  `tests/ui_smoke.spec.js`): the UI surface; the liquid-glass theme (W6) restyles them.

## Dependency Graph

```
0.44 hydracache-sim ── 0.50 demo ── 0.52 entry listeners
        │
        ▼
W1 deterministic leader election + cold-start formation in the simulator   ◄ FOUNDATION / long pole
        │
        ├──────────► W2 typed in-flight signal animation (votes/append/replication/invalidation)
        │                    │
        │                    ├──────► W3 manual mode (client push → diverge → converge → listener receipt)
        │                    └──────► W4 topology intervention (isolate/disable/rejoin + re-election/re-sync; add-node)
        ▼
W5 mode system (manual / scripted-loop / mixed; clickable intervention mid-run)
        │
        ▼
W6 liquid-glass (glassmorphism) UI redesign (accessible)
        │
        ▼
W7 determinism/replay + schema/COMPAT + UI-smoke gates (cross-cutting, incl. seeded election)
```

**W1 is the long pole** — without modeled election there is nothing truthful to animate. W2 is
the shared visualization substrate for W3/W4. W5/W6 are UX/theming on top. W7 guards the whole.

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests) / Risk & rollback.**

---

## W1. Deterministic leader election + cold-start cluster formation (foundation)

**Goal.** Make leader election **real and deterministic** in the simulator and let a cluster
**form from a cold (disconnected) start**: nodes begin unconnected, exchange votes over the
seeded network, elect a leader, and connect — all replayable from the seed. Surface the
election state so the UI can show it.

**Files.** `crates/hydracache-sim/src/world.rs` (drive raft election timeouts via `SimClock`;
a `formation_phase` lifecycle: `Disconnected → Campaigning → Elected → Connected`),
`crates/hydracache-sim/src/election.rs` (new, or extend the node seam) wiring
`hydracache-cluster-raft` election deterministically. `snapshot.rs`: extend `NodeView` with
`vote_state` (`follower|candidate|leader`), `voted_for`, `votes_received`, and add
`ElectionView`/`formation_phase` to `SimSnapshot` (schema bump + COMPAT). `invariants.rs`: add
an **election-safety invariant** (≤ one leader per term).

**Steps.**
1. Replace the "no election" gap: drive raft randomized election timeouts from `SimRng` and
   `SimClock` (logical, seeded) so a leaderless cluster elects deterministically. A
   cold-start run begins with all inter-node links **down** (`formation_phase = Disconnected`);
   links come up per the scenario, votes flow, a leader emerges (`Campaigning → Elected`), then
   replication links establish (`Connected`).
2. Surface per-node `vote_state`, `term`, `voted_for`, `votes_received`, and a cluster-level
   `formation_phase`. Election messages (RequestVote/AppendEntries) become typed `TimedMessage`s
   for W2 to animate.
3. Add the **election-safety invariant** (no two leaders in the same term) to the real checker,
   so the lab's verdict covers election too; a violation is loud (R-3).

**DoD.** `crates/hydracache-sim/tests/election.rs`
- `cold_start_elects_single_leader_deterministically` (same seed ⇒ same leader + term).
- `leader_loss_triggers_reelection` (crash/isolate the leader → a new term + leader).
- `election_safety_at_most_one_leader_per_term` (invariant).
- `election_run_is_replayable_from_seed` (R-5).
- `unknown_future_schema_version_refuses_to_load` (schema bump keeps the loud reject).
- Run: `cargo test -p hydracache-sim --locked election` + `cargo xtask verify`.

**Risk & rollback.** Highest-risk item: election is timing-sensitive; **all** timeouts must be
logical+seeded or replay breaks. Keep the raft mechanism reused (not re-implemented). If full
election is too large for one PR, split: (a) seeded election + leader/term in snapshot, (b)
cold-start formation phases. Revert restores fixed-role behavior.

### W1.R — Reinforcement: model election + formation as an explicit FSM-as-table (blazingmq §7.1)

**Why (cross-project).** BlazingMQ runs its cluster and partition lifecycles as **explicit
finite state machines** — `mqbc_clusterfsm` / `mqbc_partitionfsm` with `clusterstatetable` /
`partitionstatetable` (see `COMPETITIVE_ANALYSIS_AND_EVOLUTION.md` §7.1). Transitions are a
reviewable, testable **table**, not ad-hoc branching. This is the single most directly
transferable pattern for W1: it makes the seeded election (W1's riskiest determinism surface)
**auditable and exhaustively testable**, and gives the UI (W2) honest, named states to render.

**Goal.** Implement W1's `formation_phase` + per-node `vote_state` as an **explicit
`ClusterFsm` / `NodeFsm` with a declared transition table** (state × event → state + action),
rather than scattered `if`/`match` over raft callbacks. The seeded scheduler drives events into
the FSM; the FSM is the single source of the election/formation state surfaced in the snapshot.

**Files.** `crates/hydracache-sim/src/election.rs` (or `cluster_fsm.rs`): a `ClusterPhase` /
`NodeRole` enum pair + a pure `transition(state, event) -> (state, Vec<Action>)` table; wire it
from `world.rs::step`. Reuse `hydracache-cluster-raft` for the actual vote mechanism — the FSM
**orchestrates**, it does not re-implement consensus.

**Steps.**
1. Declare the events (`Tick`, `LinkUp`, `LinkDown`, `VoteRequest`, `VoteGranted`,
   `LeaderHeartbeat`, `Crash`, `Rejoin`) and the phase/role states; implement `transition` as a
   **total** pure function (every state×event has a defined outcome — undefined transitions fail
   loud, R-3, never silently ignored).
2. Drive it from the seeded scheduler so the *same* transition sequence replays identically
   (R-5); the snapshot's `formation_phase`/`vote_state` read straight off the FSM.
3. Keep the FSM **independent of wall-clock and of the UI** — it is a pure state machine the
   sim, the tests, and the lab all share (the `0.39` health-state enum can later reference it).

**DoD.** `crates/hydracache-sim/tests/cluster_fsm.rs`
- `transition_table_is_total` (every state×event defined; no panics, no silent no-ops).
- `cold_start_drives_disconnected_to_connected_via_fsm` (the W1 path, asserted as FSM states).
- `crash_then_rejoin_returns_through_defined_states`.
- `fsm_transition_sequence_is_seed_reproducible` (R-5).
- Run: `cargo test -p hydracache-sim --locked cluster_fsm` + `cargo xtask verify`.

**Risk & rollback.** Pure-logic layer over the raft mechanism; low risk, high testability. If
W1 is split (per its rollback note), land the FSM **first** so both halves build on it. Revert
collapses to inline state handling.

---

## W2. Typed in-flight signal animation

**Goal.** Show **individual** signals (votes, append/heartbeat, replication, invalidation,
client req/ack) **traveling between nodes**, not just a per-link count.

**Files.** `snapshot.rs` (`SimSnapshot.in_flight: Vec<MessageView>` = `{ from, to, kind,
deliver_at }`; bounded; schema bump), `world.rs::snapshot` (read `SimNetwork`'s `TimedMessage`
queue). `demo/app.js` (animate packets along link paths interpolating on
`logical_time_millis`), `demo/style.css` (per-kind packet styling), `demo/index.html` (legend).

**Steps.**
1. Map `ClusterNodeMessage` → a **bounded** kind enum (vote, append, heartbeat, replication,
   invalidation, client-req, client-ack, rebalance); unknown → `Other` (no fabrication).
2. Populate `in_flight` (bounded cap) from the network queue; dropped/delayed packets (existing
   faults) render distinctly so a partition is visibly biting.
3. Animate in the UI; election messages from W1 now visibly flow during a vote.

**DoD.** `crates/hydracache-sim/tests/in_flight.rs`
- `snapshot_exposes_typed_in_flight_messages`, `in_flight_is_bounded`,
  `vote_messages_are_visible_during_election`.
- `demo/tests/ui_smoke.spec.js`: `animates_typed_messages_between_nodes`.
- Run: `cargo test -p hydracache-sim --locked in_flight` + demo smoke.

**Risk & rollback.** Additive snapshot field behind a schema bump; cap rendered count for perf.
Revert removes the field + UI layer.

---

## W3. Manual mode — client push → diverge → converge → listener receipt

**Goal.** Let the user **push a client event** from the UI and watch the full life: the write
lands on a node, **diverges** across replicas, **replicates**, **converges**, and a
**listener/subscriber receives** the change — the core "I did X and the cluster did Y" loop.

**Files.** `world.rs` (named client actors issuing UI-driven ops tagged with `client_id`; a
subscriber registry on the existing invalidation/event path). `snapshot.rs` (`clients:
Vec<ClientView>`, `subscribers: Vec<SubscriberView>` with `last_event/lag/dropped`; schema
bump). `crates/hydracache-sim-wasm/src/lib.rs` + sandbox `/sim/*` (`push_event(client, ns, key,
value)`, `subscribe(client, ns)`). `demo/app.js`/`index.html` (a client lane with a "Push"
control + a subscriber lane).

**Steps.**
1. Add a `push_event` verb: a chosen client issues a write; it appears as a client-req
   `TimedMessage` (W2), lands on the receiving replica, and replicates — divergence (different
   `commit_index`/version per replica in `KeyView`) then convergence is **visible** in the
   key panel.
2. A subscriber registered on the namespace receives the resulting invalidation/entry event
   (mirroring `0.52` W6 kinds; else `Invalidated`/`Upserted`) — shown arriving with **bounded
   buffer + lag/drop counters** so it reads as a cache signal, **not** a business event log.
3. Keep determinism: a manually pushed event is recorded as a seeded control action so the run
   replays identically (W7).

**DoD.** `crates/hydracache-sim/tests/manual_push.rs`
- `pushed_event_replicates_and_converges`, `subscriber_receives_event_after_push`,
  `divergence_then_convergence_is_observable_in_keys`,
  `subscriber_only_sees_bus_carried_event_kinds`.
- `demo/tests/ui_smoke.spec.js`: `manual_push_shows_diverge_converge_and_listener_receipt`.
- Run: `cargo test -p hydracache-sim --locked manual_push` + demo smoke.

**Risk & rollback.** Couples to the `0.52` event-kind contract; if `0.52` lands later, show the
conservative kinds and upgrade when W6 ships. Revert removes the push/subscribe verbs.

---

## W4. Topology intervention — isolate / disable / rejoin (+ re-election & re-sync), add-node

**Goal.** One-click **isolate**, **disable**, and **rejoin** a node, with a visible **UI
reaction**: if the isolated/disabled node was the leader, a **re-election** runs (W1) and the
viewer sees it; on **rejoin** the node re-participates in voting and **catches up/re-syncs**.
Also **add a node** at runtime to show horizontal scaling + online reshard.

**Files.** `world.rs` (`isolate_node`/`rejoin_node` composing existing link faults;
`disable_node` = stop stepping; `add_node` deterministic join + `0.43` reshard; surface a
per-node `sync_progress` = applied vs leader commit). `snapshot.rs` (`sync_progress`,
`rebalance` progress; schema bump). WASM + sandbox verbs. `demo/app.js` (per-node
isolate/disable/rejoin/add buttons + a catch-up bar).

**Steps.**
1. `isolate_node` drops all incident links symmetrically; `rejoin_node` clears them;
   `disable_node`/`enable_node` stop/resume the node. Each is seeded + replayable. If the
   target was leader, W1's election fires → visible new term/leader.
2. On rejoin, render **catch-up**: the node's `applied_index` trails the leader's `commit_index`
   and closes via replication messages (W2); a `sync_progress` bar shows it. No silent
   half-sync — a stuck catch-up surfaces in the verdict (R-3).
3. `add_node` assigns the next id from the seeded sequence, joins membership, and triggers the
   `0.43` online reshard (rebalance moves render as W2 messages); invariants hold across the
   membership change.

**DoD.** `crates/hydracache-sim/tests/topology.rs`
- `isolating_leader_triggers_reelection`, `rejoin_node_catches_up_to_leader_commit`,
  `add_node_grows_membership_deterministically`, `reshard_moves_partitions_to_new_node`,
  `invariants_hold_across_isolate_rejoin_and_scale_out`, `topology_actions_replay_from_seed`.
- `demo/tests/ui_smoke.spec.js`: `node_controls_show_reelection_resync_and_scale_out`.
- Run: `cargo test -p hydracache-sim --locked topology` + demo smoke + `cargo xtask verify`.

**Risk & rollback.** add-node + re-election are the heavy parts; both must preserve
determinism/replay and the invariant checker. Split add-node into join then reshard if needed.
Revert removes the verbs; fixed-membership runs unaffected.

---

## W5. Mode system — manual / scripted-loop / mixed (clickable intervention)

**Goal.** A **mode selector**: **Manual** (only user actions), **Scripted** (prescribed
scenarios cycling through cluster×client combinations on a loop to showcase behaviors), and
**Mixed** (a scripted loop the user can interrupt). In **every** mode the topology stays
**clickable** so the viewer can intervene mid-run.

**Files.** `crates/hydracache-sim/src/scenarios.rs` (a scripted scenario catalog: cold-start
formation, leader loss + re-election, partition + heal, scale-out, manual-push convergence,
subscriber lag), `world.rs` (a scenario runner that interleaves scripted control actions with
user actions on the same seeded timeline). `demo/app.js`/`index.html` (mode selector;
scripted-loop play/pause; "you intervened" indicator).

**Steps.**
1. Define a scenario as a **seeded control script** (a sequence of the W1–W4 verbs at logical
   timestamps). Scripted mode loops the catalog; each loop is reproducible from its seed.
2. Allow **user actions to interleave** with a running script deterministically (a user click
   becomes a control action inserted at the current logical step); the resulting run is still
   replayable (record the merged action log).
3. The mode + active scenario + intervention state are in the snapshot (schema bump) so the UI
   reflects them and the reproducer captures them.

**DoD.** `crates/hydracache-sim/tests/modes.rs`
- `scripted_mode_loops_catalog_deterministically`,
  `user_intervention_merges_into_scripted_run_and_replays`,
  `mode_and_scenario_are_in_snapshot`.
- `demo/tests/ui_smoke.spec.js`: `modes_switch_and_topology_is_clickable_in_each`.
- Run: `cargo test -p hydracache-sim --locked modes` + demo smoke.

**Risk & rollback.** The interleave-and-still-replay property is subtle; W7's master replay test
guards it. Revert collapses to a single manual mode.

---

## W6. Liquid-glass (glassmorphism) UI redesign — accessible

**Goal.** Restyle the lab in a **liquid-glass / glassmorphism** theme (translucent layered
panels, blur, depth, soft light) for nodes, links, animated signals, client/subscriber lanes,
the verdict panel, and controls — modern and legible.

**Files.** `demo/style.css` (glass tokens: translucency, backdrop-blur, layered shadows,
gradient light; node/link/packet/lane/verdict styling), `demo/index.html` (structure for
layered panels + legend), minor `demo/app.js` (class hooks; respect `prefers-reduced-motion`
and `prefers-reduced-transparency`).

**Steps.**
1. Introduce a small CSS design-token layer (surfaces, blur radii, elevation, accent gradients)
   and apply it across the lab; animated signals (W2) get a glass-trail treatment.
2. **Accessibility:** maintain **WCAG-AA contrast** on text/controls over translucent surfaces
   (provide a solid fallback layer behind text); honor `prefers-reduced-motion` (freeze packet
   animation) and `prefers-reduced-transparency` (fall back to opaque panels). Glass is a skin,
   not a barrier.
3. Keep the structure unchanged so all W1–W5 testids/controls still resolve (no smoke breakage).

**DoD.**
- `demo/tests/ui_smoke.spec.js`: `glass_theme_renders_and_controls_remain_operable`,
  `reduced_motion_and_transparency_fallbacks_apply`, plus a contrast assertion on key text.
- Manual visual check documented in the PR (before/after).
- Run: demo smoke + `cargo xtask verify` (no Rust change beyond snapshot fields already landed).

**Risk & rollback.** Pure presentation; risk is readability/perf of blur. Keep tokens isolated;
revert restores the prior `style.css`. Must not regress any testid.

---

## W7. Determinism / replay + schema / COMPAT + UI-smoke gates (cross-cutting)

**Goal.** Prove the whole lab preserves **seed reproducibility** — including election, manual
pushes, topology intervention, and scripted/mixed runs — and that snapshot schema discipline
holds.

**Files.** `crates/hydracache-sim/tests/replay_lab.rs` (new), `docs/COMPAT.md` (all snapshot
schema bumps), `demo/tests/ui_smoke.spec.js` (consolidated).

**Steps.**
1. Master property test: a recorded mixed run (cold-start election + scripted scenario +
   interleaved manual pushes + isolate/rejoin + add-node), replayed from the **same seed**,
   yields a byte-identical snapshot history hash (R-5). This is the gate for election timing,
   the riskiest determinism surface.
2. Verify `SIM_SNAPSHOT_SCHEMA_VERSION` is bumped once per new field set and every bump is in
   `docs/COMPAT.md` with reader window + loud unknown-future reject (R-4).
3. The "copy reproducer" must round-trip the full action log (seed + mode + control script).

**DoD.** `crates/hydracache-sim/tests/replay_lab.rs`
- `full_mixed_run_replays_identically_from_seed`, `every_new_snapshot_field_bumped_schema`,
  `reproducer_roundtrips_seed_mode_and_actions`.
- `demo/tests/ui_smoke.spec.js`: full control surface green.
- Run: `cargo xtask verify` + the demo smoke suite.

**Risk & rollback.** Non-negotiable safety net: any control (especially election) that cannot be
made deterministic does not ship (R-5). No production surface.

---

## Gates (Definition of Done for the release)

- `cargo xtask verify` green; demo Playwright smoke green.
- Leader election is **modeled** (W1), covered by the election-safety invariant, and **visible**
  (W2) — no animated fiction (R-3).
- Determinism/replay proven for every control + election + scripted/mixed runs (W7); the
  reproducer reproduces exactly (R-5).
- Every new `SimSnapshot` field registered in `docs/COMPAT.md` with a schema bump (R-4).
- WASM and sandbox `/sim/*` paths stay in parity.
- Liquid-glass theme keeps WCAG-AA contrast + reduced-motion/transparency fallbacks (W6).
- The lab remains a teaching/DevRel asset; no control presented as a correctness guarantee
  (RULES). No numeric self-score (R-7).
- `releases.toml` + `INDEX.md` updated to `0.53.0`; the absorbed
  `V0_50_DEMO_ENHANCEMENTS_PLAN.md` marked superseded.
