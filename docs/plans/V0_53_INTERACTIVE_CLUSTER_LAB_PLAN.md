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
> - **Status:** shipped.
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

## Contracts, Invariants & Proof Obligations (read before any W)

This section turns the "nice goals" into **hard, testable contracts**. Every contract below
states **What / Why / How tested / Proof of truthfulness** (how we know the lab shows reality,
not animation). Work items reference these by id (C1…C8). A claim that lacks its proof obligation
is not done (R-7).

> **Authoring note (UTF-8, not mojibake).** These plan files are valid UTF-8 and use `—`, `→`,
> `·`, `◄`, `✅`. If a tool renders them as `â€"`/`â†'`, the **reading pipe is latin-1** — fix the
> pipe, do **not** "repair" the file. New machine-consumed snippets (enum names, JSON keys) stay
> **ASCII** so no toolchain mangles identifiers.

### C1. `ReplayScriptV1` + `ControlActionV1` — the reproducibility contract

- **What.** A first-class, versioned reproducer artifact:
  `ReplayScriptV1 { version: u16, seed: u64, mode: Mode, scenario: Option<String>, actions:
  Vec<ControlActionV1> }`, where `ControlActionV1` is a closed enum:
  `Step{n}`, `Isolate{node}`, `Rejoin{node}`, `Disable{node}`, `Enable{node}`, `AddNode`,
  `PushEvent{client, ns, key, value}`, `Subscribe{client, ns}`, `ModeChange{mode}`. Each action
  carries the **logical step** at which it applies.
- **Why.** A seed + step count reproduces a *scripted* run, but **manual/mixed mode adds
  user-ordered actions** — so the reproducer must be `seed + mode + ordered actions`, or "copy
  reproducer" silently loses the very interventions the lab is about. This is the difference
  between a demo and a provable artifact.
- **How tested.** `crates/hydracache-sim/tests/replay_script.rs`:
  `replay_script_roundtrips_json` (encode→decode identity), `unknown_future_replay_version_rejected`
  (loud, R-4), `share_url_roundtrips_replay_script` (the demo URL encodes/decodes it).
- **Proof of truthfulness.** `full_mixed_run_replays_identically_from_script` (W7 master test):
  executing a `ReplayScriptV1` produces a **byte-identical snapshot-history hash** to the original
  run. If it diverges, replay is a lie and the build is red. **`ReplayScriptV1` is a replay-visible/
  web-visible format → registered in `docs/COMPAT.md` (R-4)** with a reader window + loud reject.

### C2. Snapshot schema version matrix (explicit, not "bump schema")

`SIM_SNAPSHOT_SCHEMA_VERSION` advances on a **named, per-work-item** schedule; each bump has a
reader window and a loud unknown-future reject (`SimSnapshot::from_json`). No work item may add a
field without advancing to its assigned version and registering it in `docs/COMPAT.md`.

| Version | Adds | Owner WI |
| --- | --- | --- |
| `v1` | current (`nodes/links/keys/verdict/progress`) | shipped |
| `v2` | election/formation (`vote_state`, `voted_for`, `votes_received`, `formation_phase`) | W1 |
| `v3` | typed in-flight messages (`in_flight: Vec<MessageView>`) | W2 |
| `v4` | clients/subscribers/manual-push (`clients`, `subscribers`, `sync_progress`) | W3, W4 |
| `v5` | modes + replay metadata + topology progress (`mode`, `active_scenario`, `rebalance`) | W4, W5 |

- **Why.** "Bump schema" without a target invites COMPAT drift and two PRs claiming the same
  version. The matrix makes each reader window auditable.
- **How tested / proof.** `schema_version_matches_contract_for_each_field_set` (a test asserting
  the field set present implies the documented version), and the existing
  `unknown_future_schema_version_refuses_to_load` after every bump. `cargo xtask doc-check`
  enforces the COMPAT entries.

### C3. Election & topology invariants — the **truthfulness** contract

These are real checkers in `crates/hydracache-sim/src/invariants.rs`, evaluated every step and
surfaced in the verdict. They are what make the lab *correctness made visible* rather than a
cartoon. Each invariant names the fault that would violate it (so W7 can inject it):

| Invariant | Violated by | Checker |
| --- | --- | --- |
| **≤ one leader per term** | split-brain election | `election_safety` |
| **Leader only with quorum** | minority leader after partition | `leader_requires_quorum` |
| **Isolated old leader is not authoritative** | stale leader accepting writes | `no_stale_leader_writes` |
| **`commit_index` / `applied_index` monotonic per node** | log rollback | `index_monotonicity` |
| **Rejoin catch-up never skips a commit** | gap during re-sync | `catchup_no_skip` |
| **Subscriber event never precedes apply/commit of its write** | event before durability | `event_after_commit` |

- **Why.** A pretty animation that violates these would *mislead* an operator — the opposite of
  the pitch. The invariants are the proof the visuals reflect a correct run.
- **How tested / proof.** Each invariant has (a) a unit test asserting it holds on a good run and
  (b) a **fault test that injects the violating fault and asserts the checker fires loud** — e.g.
  `no_stale_leader_writes_fires_when_isolated_leader_writes`. A checker that cannot be made to fire
  under its fault is not trusted (a checker must be *falsifiable*).

### C4. Manual-mode observable-field semantics (C4)

"Diverge → converge → listener receipt" is defined on **snapshot fields**, not vibes:
- **Divergence** = at least two live replicas report different `(version | commit_index |
  applied_index)` for the pushed key in `KeyView`.
- **Convergence** = all **live** replicas report the **same** `version` and value for that key.
- **Listener receipt** = a `SubscriberView` records an event whose **kind is one the `0.52` W6
  bus actually carries** (`Invalidated`/`Upserted`, or W6's `Added/Updated/Removed/Evicted` if
  shipped) — **never** a business-event payload (R-9).
- **Proof.** `manual_push_diverges_then_converges_on_observable_fields` asserts the exact field
  transitions; `subscriber_receipt_kind_is_bus_carried_only` asserts no fabricated kind.

### C5. Bounded / perf limits + over-budget counters (R-3/R-6)

Explicit caps, each with a **drop/over-budget counter that fails loud**, never silent or unbounded:
- `MAX_IN_FLIGHT_RENDERED` (typed packets in a snapshot) — excess summarized, counted.
- `MAX_REPLAY_ACTIONS` (actions in a `ReplayScriptV1` / share URL) — over-length refuses loud with
  a "script too long" error (URLs have length limits), never truncates silently.
- `MAX_SUBSCRIBER_BUFFER` — slow subscriber dropped-with-counter.
- **Proof.** `over_budget_in_flight_is_summarized_and_counted`,
  `replay_script_over_max_actions_refuses_loud`, `slow_subscriber_drops_with_counter`.

### C6. Native/WASM/sandbox parity via one shared control API (C6)

`ControlActionV1` (C1) is the **single** control surface used by `hydracache-sim` (native),
`hydracache-sim-wasm`, and the sandbox `/sim/*` routes — no mode re-implements verbs.
- **Why.** Three drifting control paths = three behaviors and an unreproducible lab.
- **How tested / proof.** `wasm_control_actions_match_native` (the WASM handle accepts exactly the
  native `ControlActionV1` set) and `sim_routes_accept_same_control_script` (the sandbox `/sim/*`
  path executes an identical `ReplayScriptV1` to the same hash).

### C7. UI gate — concrete, not "demo smoke" (C7)

The demo gate is a named, runnable procedure, not a vibe:
- **Prerequisite — scaffold the demo tooling first (W6).** `demo/` today has
  `tests/ui_smoke.spec.js` but **no `demo/package.json` and no Playwright config** (verified). W6
  must add `demo/package.json` (build + `@playwright/test` devDep + scripts) and a
  `playwright.config.*` with the two viewports below, or the commands here cannot run. This
  scaffolding is part of W6's DoD, gated by W7.
- **Start:** `npm --prefix demo ci && npm --prefix demo run build`; serve via the documented dev
  command; for sandbox mode start `cargo run -p hydracache-sandbox` and point the demo at `/sim/*`.
- **Run:** `npx --prefix demo playwright test` (the `demo/tests/ui_smoke.spec.js` suite).
- **Required artifacts:** Playwright screenshots at **desktop (1440×900)** and **mobile
  (390×844)** viewports for: cold-start formation, an election, a manual push, an isolate→rejoin.
- **Glass checks (W6):** `no_overlap_at_both_viewports`, `wcag_aa_contrast_on_glass_text`,
  `prefers_reduced_motion_freezes_packets`, `prefers_reduced_transparency_uses_opaque_panels`.
- **Proof.** The gate is green only if all the above pass; screenshots are attached to the PR.

### C8. Release gate list (consolidated; W7 enforces)

`cargo xtask verify` · focused per-W sim tests · `wasm-pack`/`cargo build -p hydracache-sim-wasm`
· sandbox `/sim/*` route tests · Playwright UI smoke (C7) · `doc-check` (COMPAT incl. C1 + C2) ·
**publish-readiness (corrected — no packaging decision in 0.53):** `0.53` adds **no** new crate
and **changes no crate's publish status**. It does **not** assert anything about
`hydracache-sim` / `hydracache-sim-wasm` publish state. (Those were **already reconciled to
`publish = false`** as a standalone hygiene change ahead of this release — sim is an internal
test harness, sim-wasm is a `wasm-pack` demo cdylib — so the prior metadata-vs-list inconsistency
is gone.) The only `0.53` publish-gate is: *no crate's `publish` field changed by this release's
commits* (a diff check, fail loud on drift). The broader publish-list reconciliation for
`client*`/`server` remains `0.54`'s scope.

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

### W1.S — Sub-item split (W1a–W1d) + the determinism fail-loud criterion

W1 is the long pole; ship it as **four ordered PRs, scope preserved** (not reduced). Each is its
own commit with its own tests; the FSM (W1.R) lands inside W1a.

| Sub-PR | Scope | Proof gate |
| --- | --- | --- |
| **W1a** | explicit `ClusterFsm`/`NodeFsm` transition table (W1.R) | `transition_table_is_total` |
| **W1b** | deterministic election **driver** over the FSM | `cold_start_elects_single_leader_deterministically` + `election_run_is_replayable_from_seed` |
| **W1c** | election/topology **invariants** (C3) wired into the checker | each C3 invariant's hold-test **and** its fault-fires-test |
| **W1d** | cold-start formation phases in snapshot + UI (`v2`, C2) | `cold_start_drives_disconnected_to_connected_via_fsm` + UI smoke |

**The determinism criterion (precise, fail-loud).** The real risk is **not** wall-clock —
`raft-rs` (TiKV 0.7) is **tick-based**: `Raft::tick()` advances *logical* ticks and
`election_timeout` is in tick units, so driving it from `SimClock` is fine. The actual risk is the
**randomized election-timeout RNG inside `raft-rs`**, which is not trivially seedable from outside.

W1b therefore has an explicit decision gate:
1. **First attempt:** drive `raft-rs` by `tick()` on `SimClock` and **seed its randomized
   election timeout deterministically** (inject/seed the RNG seam). If `same seed ⇒ same leader,
   term, and election-message order` holds across 1000 seeded runs → done, this is the product-
   accurate path.
2. **If (1) cannot be made deterministic** (the RNG seam is not reachable without patching
   `raft-rs`): **fail loud in the plan** — fall back to a **simulator election adapter** (a
   deterministic election model over the same FSM) that is **explicitly labelled "model for the
   demo, not the product consensus"** in the snapshot (`election_source: "raft" | "sim-model"`)
   and in the UI. **No product correctness claim** is attached to the sim-model path (R-7); it is
   a teaching device, and the lab says so on screen.

**Proof of truthfulness for W1b.** `election_determinism_holds_over_1000_seeds` (same seed ⇒
identical leader/term/message-order). If the product path is used, also
`election_source_is_raft_not_model`; if the fallback is used,
`sim_model_is_labelled_and_makes_no_product_claim` asserts the snapshot/UI disclosure exists.

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

**Contracts.** Divergence/convergence/receipt are defined on snapshot fields per **C4** (assert
field transitions, not visuals); the listener kind is bus-carried only (R-9). `PushEvent` /
`Subscribe` are `ControlActionV1` cases (**C1/C6**) and so are part of the replayable script.
Subscriber buffering is bounded per **C5**.

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

**Contracts.** A scenario **is** a `ReplayScriptV1` (**C1**); a user click in mixed mode is a
`ControlActionV1` appended at the current logical step. Scripted/mixed/manual all run the **same
`ControlActionV1` surface across native/WASM/sandbox** (**C6**), and the merged action log is
itself a `ReplayScriptV1` that round-trips (proof in W7). Action count is bounded per **C5**.

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

**Contracts.** The accessibility + viewport gate is **C7** (exact start/run commands, required
desktop 1440×900 + mobile 390×844 screenshots, and the four glass checks: no-overlap, WCAG-AA
contrast, reduced-motion freeze, reduced-transparency opaque fallback). W6 is green only when C7
passes; do not weaken C7 to make the theme land.

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
- `full_mixed_run_replays_identically_from_script` (the **C1** master proof: execute a
  `ReplayScriptV1` → byte-identical snapshot-history hash; the single gate that makes the whole
  lab provably reproducible),
- `every_new_snapshot_field_bumped_schema` (the **C2** matrix is honored),
- `reproducer_roundtrips_seed_mode_and_actions` (share-URL ↔ `ReplayScriptV1`).
- `demo/tests/ui_smoke.spec.js`: full control surface green at both **C7** viewports.
- **Run the full C8 gate list:** `cargo xtask verify` · per-W focused sim tests ·
  `cargo build -p hydracache-sim-wasm` · sandbox `/sim/*` route tests · Playwright UI smoke (C7) ·
  `doc-check` (COMPAT incl. C1 + C2) · publish-readiness: **no crate's `publish` field changed in
  `0.53`** (diff check, fail loud). The sim/sim-wasm publish-list reconciliation is `0.54`'s
  scope, not asserted here (C8).

**Risk & rollback.** Non-negotiable safety net: any control (especially election) that cannot be
made deterministic does not ship (R-5). The **C3 invariant checkers must be falsifiable** — each
must fire under its injected fault, or it is not trusted. No production surface.

---

## Gates (Definition of Done for the release)

- Full **C8** gate list green (`cargo xtask verify` · per-W sim tests · `hydracache-sim-wasm`
  build · sandbox `/sim/*` routes · Playwright smoke at C7 viewports · `doc-check`).
- **C1:** `ReplayScriptV1` + `ControlActionV1` shipped, share-URL round-trips, registered in
  `docs/COMPAT.md`; the master test `full_mixed_run_replays_identically_from_script` is green.
- **C2:** snapshot schema followed the `v2…v5` matrix; every field set advanced to its assigned
  version with a reader window + loud unknown-future reject.
- **C3:** all six election/topology invariants are real checkers **and each is falsifiable**
  (fires under its injected fault) — this is the proof the visuals reflect a correct run, not
  animation (R-3).
- **C4:** manual-mode diverge/converge/receipt asserted on snapshot fields; listener kinds are
  bus-carried only (R-9).
- **C5:** all caps (in-flight, replay actions, subscriber buffer) enforced with over-budget
  counters that fail loud (R-3/R-6).
- **C6:** one `ControlActionV1` surface across native/WASM/sandbox; parity tests green.
- **C7:** liquid-glass passes WCAG-AA contrast + reduced-motion/transparency fallbacks at both
  viewports; screenshots attached to the PR.
- Election uses `raft-rs` deterministically **or** the labelled sim-model fallback (W1.S) — if
  the fallback, the snapshot/UI disclose `election_source: "sim-model"` and make **no** product
  claim (R-7).
- The lab remains a teaching/DevRel asset; no control presented as a correctness guarantee
  (RULES). No numeric self-score (R-7).
- `releases.toml` + `INDEX.md` updated to `0.53.0`; the absorbed
  `V0_50_DEMO_ENHANCEMENTS_PLAN.md` marked superseded.
