# HydraCache 0.50 Demo Enhancements — Interactive Cluster Lab (Execution Plan)

> **Status: SUPERSEDED by [`V0_53_INTERACTIVE_CLUSTER_LAB_PLAN.md`](V0_53_INTERACTIVE_CLUSTER_LAB_PLAN.md) — historical only. DO NOT IMPLEMENT DIRECTLY.**
>
> This supporting plan is kept for history. Its scope was **absorbed** into the numbered `0.53`
> release (which additionally adds modeled leader election + cold-start formation, a liquid-glass
> UI, and the manual/scripted/mixed mode system). **All of this plan's own gates and DoD are
> void** — they do not apply; only `0.53`'s gates and contracts (C1–C8) are authoritative. **Any
> new requirement, field, or test for this scope goes into `0.53`, never here**, so an agent never
> implements two competing plans.
>
> **Scope mapping (this plan → `0.53` work item):**
>
> | This plan (0.50 supporting) | Implement in `0.53` |
> | --- | --- |
> | W1 in-flight message animation | **W2** (typed in-flight signal animation) |
> | W2 isolate/overload + W3 add-node | **W4** (topology intervention: isolate/disable/rejoin + add-node) |
> | W4 client actors + W5 subscribers | **W3** (manual mode: client push → diverge → converge → listener receipt) |
> | W6 determinism/replay + schema/COMPAT + UI-smoke | **W7** (+ contracts **C1** ReplayScriptV1, **C2** schema matrix) |
>
> The text below is the **original** supporting-plan content, retained unchanged for provenance.

> **At a glance**
> - **What:** turn the shipped `0.50` browser simulator into a hands-on **interactive
>   cluster lab** (TigerBeetle-`sim.tigerbeetle.com` style): **see messages travel between
>   nodes**, **one-click isolate / overload** any node (not just per-link faults), **add a
>   node at runtime** to watch horizontal scaling + online resharding, and **see a couple of
>   client actors** — some generating write traffic, some subscribed to change events —
>   so a viewer can manually drive partitions/crashes/scale and watch the grid (and the live
>   invariant verdict) react.
> - **Why:** the `0.50` demo already does per-node crash/restart, per-link delay/drop,
>   directional partitions, seed-reproduce, and live invariant verdicts — but the *manual,
>   pedagogical* surface is thinner than TigerBeetle's: no in-flight signal animation, no
>   one-click node isolate/overload, no live membership growth, and no visible clients. This
>   is the "**correctness as a visible product feature**" (`POSITIONING.md`) and the
>   operational-confidence asset for the **Hazelcast-migration pitch**: a prospect kills /
>   isolates / overloads a node, adds capacity, and *sees* the grid survive.
> - **After (depends on):** `0.44` (`hydracache-sim` engine) and `0.50` (the browser demo).
>   W5 (event subscribers) coordinates with `0.52` W6 (entry listeners) so the demo does not
>   fabricate event semantics the bus does not carry.
> - **Status:** planned. This is a **supporting/execution plan over `0.50`**, not a new
>   numbered release (avoids renumbering churn); it still ships on its own boolean gates and
>   may be promoted to a point release (e.g. `0.53`) if preferred.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> demo release: [`V0_50_INTERACTIVE_SIMULATOR_DEMO_PLAN.md`](V0_50_INTERACTIVE_SIMULATOR_DEMO_PLAN.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its
Definition of Done **and** `cargo xtask verify`; never push red.

## Justification (why this, why now)

The `0.50` demo (`demo/app.js`, `demo/index.html`, `demo/scenarios.js`) over the `0.44`
`hydracache-sim` engine already exposes, verified against the code:

- **Per-node crash/restart** — a button per node (`crash-${id}` / `restart-${id}`,
  `app.js` → `SimWorld::crash_node` / `restart_node`).
- **Per-link delay/drop** — clicking a link injects a fault (`app.js applyLinkAction` →
  `SimWorld`/`SimNetwork` inject).
- **Directional partitions** in the engine (`PartitionDirection::{Symmetric,LeftToRight,
  RightToLeft}`, `crates/hydracache-sim/src/network.rs`).
- **Seed reproducer**, step/play, and a **live invariant verdict** panel.

The engine is also closer to the new asks than it looks:

- In-flight packets **already exist** in the engine — `TimedMessage { from, to, message,
  deliver_at }` ("Packet currently in flight", `network.rs`) — but `SimSnapshot`
  (`crates/hydracache-sim/src/snapshot.rs`: `nodes/links/keys/verdict/progress`) does **not**
  surface them, so the UI cannot animate signal flow today.
- `SimWorld.nodes` is a `BTreeMap<ClusterNodeId, SimNode>` — structurally able to grow — but
  there is **no `add_node` verb** (only `crash_node`/`restart_node`/`inject`/`step`/
  `set_workload_enabled`), and membership growth must drive the existing `0.43` online-reshard
  path **deterministically**.
- The workload is a **single global `WorkloadGenerator`** toggle, not distinct client actors,
  and there is **no subscriber model** at all.

So the work is: surface what the engine already has (in-flight messages), add a small set of
**deterministic, replayable** control verbs (isolate, overload, add-node, named clients,
subscribers), and render them. The hard constraint is **determinism/replay** (R-5): every new
control is a seeded event so the "copy reproducer" seed still reproduces the exact run.

## Release Theme

A manual, seed-reproducible **interactive cluster lab**: visible inter-node signals, one-click
node isolate/overload, live horizontal scaling (add-node + online reshard), and visible client
actors (writers + event subscribers) — all over the same `0.44` deterministic engine and the
same real invariant checker, **without** turning the demo into a correctness gate and
**without** breaking determinism/replay.

## Non-Goals

- **Not a correctness gate.** The demo remains a teaching/DevRel asset. The authoritative
  correctness signal stays the DST gates (`0.44`/`0.52` W7) and the invariant checker; new
  controls must **not** be presented as new guarantees (RULES: the demo "shows the same seeded
  engine and invariant checker to humans without replacing the release gates").
- **No non-determinism.** No wall-clock, no real network, no `Math.random()` in control paths.
  Every control is a seeded, replayable scheduler event; replay from the seed must be
  byte-identical (R-5).
- **No fabricated event semantics.** Subscriber visualization (W5) shows only the event kinds
  the invalidation bus actually carries; it coordinates with `0.52` W6 and does not invent
  `Added`/`Updated` the stream cannot prove (R-3).
- **No new business surface.** Clients/subscribers are *visualization* of the existing client
  op + invalidation paths, not a new product API.
- **No always-on cost.** Enhancements live in the demo + the WASM/sandbox `/sim/*` seam; the
  embedded library and the `0.50` default behavior are unchanged unless a control is used.

## Inherited Boundary (assumes 0.44 + 0.50)

- **`SimWorld`** (`crates/hydracache-sim/src/world.rs`): the deterministic driver. New verbs
  (`isolate_node`, `overload_node`, `add_node`, named-client workload, subscriber registry)
  extend it; reuse `SimRng`/`SimClock` for seeding — never a new clock.
- **`SimSnapshot`** (`crates/hydracache-sim/src/snapshot.rs`): has a `schema_version` and a
  loud unknown-future reject (`from_json`). Every new field bumps `SIM_SNAPSHOT_SCHEMA_VERSION`
  and is registered in `docs/COMPAT.md` (R-4).
- **`SimNetwork` + `TimedMessage`** (`network.rs`): the in-flight packet source for W1; the
  partition primitives for W2's isolate.
- **WASM + sandbox parity**: every new verb is exposed in **both** `crates/hydracache-sim-wasm/
  src/lib.rs` (WASM default) **and** the sandbox `/sim/*` POST path (the `ServerHandle` in
  `app.js`), so both demo modes stay in sync.
- **`0.43` online reshard** (`grid/elasticity.rs`): W3 (add-node) drives this existing path; it
  does not invent a new rebalance algorithm.
- **`0.52` W6 entry listeners**: W5 (subscribers) mirrors those event kinds; if `0.52` is not
  yet shipped, W5 shows `Invalidated`/`Upserted` only.
- **`demo/tests/ui_smoke.spec.js`**: the existing Playwright smoke harness; every UI control
  gets a smoke assertion here.

## Dependency Graph

```
0.44 hydracache-sim ── 0.50 browser demo
        │
        ▼
W1 in-flight message animation (surface TimedMessage in SimSnapshot + UI)   ◄ foundational viz
        │
        ├───────────► W2 one-click node isolate / overload (per-node faults)
        ├───────────► W4 visible client actors (named writers, in-flight requests)
        ▼
W5 visible event subscribers (coordinate with 0.52 W6)
        │
W3 add-node at runtime → horizontal scaling + online reshard viz   ◄ long pole (own track)
        │
        ▼
W6 determinism/replay + schema/COMPAT + UI-smoke gates (cross-cutting)
```

W1 is foundational (W2/W4/W5 all render on top of visible signals). **W3 (add-node) is the
long pole** — real deterministic membership growth — and runs as its own track; do it after
W1 so scaling is *visible*, but it does not block W2/W4/W5.

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests) / Risk & rollback.**

---

## W1. In-flight message animation (surface what the engine already has)

**Goal.** Show raft/replication/invalidation signals **traveling between nodes** — the headline
"see signals move" effect — by surfacing the engine's existing in-flight packets in the
snapshot and animating them along links.

**Files.** Extend `crates/hydracache-sim/src/snapshot.rs` (`SimSnapshot.in_flight:
Vec<MessageView>` = `{ from, to, kind, deliver_at }`; bump `SIM_SNAPSHOT_SCHEMA_VERSION`),
read from `SimNetwork`'s queued `TimedMessage`s in `world.rs::snapshot`. Register the schema
bump in `docs/COMPAT.md`. UI: `demo/app.js` (render + animate packets along link paths),
`demo/style.css` (packet styling), `demo/index.html` (legend for message kinds).

**Steps.**
1. Add a `MessageView` carrying source, dest, a **bounded** message-kind enum (heartbeat,
   append/vote, replication, invalidation, client-req/ack), and `deliver_at` (logical). Map
   `ClusterNodeMessage` → kind; unknown kinds collapse to a generic `Other` (no fabrication).
2. In `world.rs::snapshot`, read the network's in-flight queue and populate `in_flight`. Keep
   it **bounded** (cap the count rendered; the engine stays the source of truth).
3. UI animates each packet from `from`→`to` interpolating on `logical_time_millis`; dropped/
   delayed packets (from existing faults) render distinctly so a viewer sees a partition bite.

**DoD.**
- `crates/hydracache-sim/tests/snapshot_in_flight.rs`: `snapshot_exposes_in_flight_messages`
  (after a step with traffic, `in_flight` is non-empty and references valid node ids);
  `in_flight_is_bounded`; `unknown_future_schema_version_refuses_to_load` (existing reject
  still holds after the bump).
- `demo/tests/ui_smoke.spec.js`: `animates_messages_between_nodes` (packets appear; a dropped
  link shows no delivery).
- Run: `cargo test -p hydracache-sim --locked snapshot_in_flight` + the demo smoke + `cargo xtask verify`.

**Risk & rollback.** Additive snapshot field behind a schema bump; if rendering is heavy, cap
the in-flight count. Revert removes the field + UI layer; engine untouched.

---

## W2. One-click node isolate / overload (per-node faults)

**Goal.** Let a viewer **isolate** a node from all peers in one click, and **overload/slow** a
node — node-level controls, beyond today's per-link delay/drop.

**Files.** `crates/hydracache-sim/src/world.rs` (`isolate_node(id)` = inject symmetric drop on
**all** links touching the node via the existing `SimNetwork` primitive; `rejoin_node(id)`;
`overload_node(id, factor)` = node-level processing delay / bounded backpressure).
`crates/hydracache-sim-wasm/src/lib.rs` + sandbox `/sim/inject` (new actions `isolate`,
`rejoin`, `overload`). UI: per-node buttons in `app.js` next to crash/restart.

**Steps.**
1. `isolate_node` composes existing link faults (no new fault model) — drop all of the node's
   incident links symmetrically; `rejoin_node` clears them. Deterministic (seeded), replayable.
2. `overload_node` adds a **bounded** per-node service delay (a new node-level fault kind):
   the node still steps, but its outbound processing is slowed by a factor; backpressure is
   bounded and counted, never unbounded (R-3). Document it as "slow", not "infinite".
3. Expose both in WASM + sandbox paths; wire per-node buttons + state badges (`isolated`,
   `overloaded`) in the UI.

**DoD.**
- `crates/hydracache-sim/tests/node_faults.rs`: `isolate_node_drops_all_incident_links`,
  `rejoin_restores_delivery`, `overload_slows_but_does_not_starve`,
  `isolate_then_rejoin_is_seed_reproducible`.
- `demo/tests/ui_smoke.spec.js`: `isolate_and_overload_buttons_change_node_state`.
- Run: `cargo test -p hydracache-sim --locked node_faults` + demo smoke.

**Risk & rollback.** `isolate` is pure composition (low risk). `overload` adds a fault kind —
keep it bounded and seeded; revert removes the verb + buttons.

---

## W3. Add-node at runtime — horizontal scaling + online reshard (long pole)

**Goal.** Let a viewer **add a node** mid-run and watch the cluster grow and **online-reshard**
onto it — visible horizontal scaling.

**Files.** `crates/hydracache-sim/src/world.rs` (`add_node()` — deterministic: instantiate a
`ClusterNode` with config, register in `nodes`, wire into `SimNetwork`, schedule membership
join + trigger the `0.43` reshard path). `crates/hydracache-sim-wasm/src/lib.rs` + sandbox
`/sim/*` (`add_node`). UI: "Add node" button + a reshard/migration indicator reusing the W1
signal animation (rebalance moves as visible traffic).

**Steps.**
1. `add_node` assigns the next `ClusterNodeId` **from the seeded sequence** (determinism: the
   new id and its join timing are scheduler events, not wall-clock). Reuse `SimConfig` defaults
   for heartbeat/step.
2. Drive the existing **online reshard** (`grid/elasticity.rs` / rebalance-as-data): the new
   node receives partitions; render the migration as in-flight "rebalance" messages (W1) and
   surface a `progress.rebalance` field in the snapshot (schema bump + COMPAT).
3. The invariant checker must keep passing across the membership change (no lost/duplicated
   ownership); a failed reshard surfaces a loud verdict, never a silent half-move (R-3).

**DoD.**
- `crates/hydracache-sim/tests/add_node.rs`: `add_node_grows_membership_deterministically`
  (same seed ⇒ same post-add state hash), `reshard_moves_partitions_to_new_node`,
  `invariants_hold_across_scale_out`, `add_node_run_is_replayable_from_seed`.
- `demo/tests/ui_smoke.spec.js`: `add_node_button_grows_cluster_and_shows_reshard`.
- Run: `cargo test -p hydracache-sim --locked add_node` + demo smoke + `cargo xtask verify`.

**Risk & rollback.** Highest-risk item: membership growth must not break determinism/replay or
the invariant checker. If reshard-in-sim is too heavy for one PR, split: (a) join membership +
visualize, (b) move partitions. Revert removes `add_node`; existing fixed-membership runs are
unaffected.

---

## W4. Visible client actors (named writers, in-flight requests)

**Goal.** Show **a couple of client actors** generating traffic — distinct, named clients whose
requests are visible in flight to the nodes they hit (not a single global workload toggle).

**Files.** `crates/hydracache-sim/src/world.rs` + the workload generator (model `N` named
clients each issuing ops on the seeded schedule; tag each `ClientOp` with a `client_id`).
`snapshot.rs` (`clients: Vec<ClientView>` = `{ id, last_op, in_flight }`; schema bump). UI:
client lane in `app.js`/`index.html`; client→node requests reuse the W1 animation.

**Steps.**
1. Split the global `WorkloadGenerator` into a small set of **named** clients (e.g. `client-a`,
   `client-b`) issuing ops deterministically from the seed; each op carries its `client_id`.
2. Surface `clients` in the snapshot with each client's last op and in-flight request; a
   client's request is a W1 in-flight message (kind = client-req/ack), so writers are visible
   talking to nodes.
3. Keep determinism: client scheduling is seeded; the reproducer seed reproduces the same
   per-client op stream.

**DoD.**
- `crates/hydracache-sim/tests/clients.rs`: `named_clients_issue_seeded_ops`,
  `client_requests_appear_in_flight`, `client_streams_are_seed_reproducible`.
- `demo/tests/ui_smoke.spec.js`: `client_actors_are_visible_and_generate_traffic`.
- Run: `cargo test -p hydracache-sim --locked clients` + demo smoke.

**Risk & rollback.** Refactors the workload into named clients; keep the history hash stable or
update the reproducer fixtures in the same PR. Revert collapses back to the global workload.

---

## W5. Visible event subscribers (coordinate with 0.52 W6)

**Goal.** Show **client(s) subscribed to change events** receiving invalidation/entry signals —
the consumer side of the bus made visible.

**Files.** `crates/hydracache-sim/src/world.rs` (a subscriber registry: clients that subscribe
to a namespace and receive delivered events). `snapshot.rs` (`subscribers: Vec<SubscriberView>`
= `{ id, subscribed_ns, last_event, lag, dropped }`; schema bump). UI: subscriber lane showing
events arriving (reuse W1 animation for delivery), with **lag/drop counters** visible.

**Steps.**
1. Model a subscriber client that registers on the existing invalidation/event path; on a
   write that invalidates a subscribed key, the subscriber receives a delivered event (an
   in-flight "invalidation" message in W1 terms).
2. Show **only** the event kinds the bus carries (coordinate with `0.52` W6: if shipped, mirror
   `Added/Updated/Removed/Evicted`; else `Invalidated`/`Upserted`). **Never fabricate** a
   transition the stream cannot prove (R-3).
3. Surface the **bounded-buffer + lag/drop** semantics in the UI so the demo teaches that this
   is a *cache signal, not a business event log* (consistent with the unsupported
   `Ringbuffer`/`ReliableTopic` stance).

**DoD.**
- `crates/hydracache-sim/tests/subscribers.rs`: `subscriber_receives_invalidation_on_write`,
  `subscriber_only_sees_bus_carried_event_kinds`, `slow_subscriber_lag_and_drop_are_counted`.
- `demo/tests/ui_smoke.spec.js`: `subscriber_lane_shows_events_and_counters`.
- Run: `cargo test -p hydracache-sim --locked subscribers` + demo smoke.

**Risk & rollback.** Couples to the event-kind contract; if `0.52` W6 lands later, the demo
shows the conservative kinds and upgrades when W6 ships. Revert removes the subscriber registry.

---

## W6. Determinism / replay + schema / COMPAT + UI-smoke gates (cross-cutting)

**Goal.** Prove every new control preserves the demo's core promise — **seed reproducibility**
— and that snapshot schema discipline and UI smokes hold.

**Files.** `crates/hydracache-sim/tests/replay_with_controls.rs` (new), `docs/COMPAT.md`
(snapshot schema versions), `demo/tests/ui_smoke.spec.js` (consolidated control smokes).

**Steps.**
1. A property test: a recorded control script (isolate/overload/add-node/client/subscriber
   actions interleaved with steps), replayed from the **same seed**, produces a byte-identical
   snapshot history hash (R-5). This is the master guard for the whole plan.
2. Confirm `SIM_SNAPSHOT_SCHEMA_VERSION` is bumped once per new field set and every bump is in
   `docs/COMPAT.md` with the reader window + loud unknown-future reject (R-4).
3. The "copy reproducer" button must still round-trip: copied seed + control script reproduce
   the exact run shown.

**DoD.**
- `crates/hydracache-sim/tests/replay_with_controls.rs`:
  `control_script_replays_identically_from_seed`, `every_new_snapshot_field_bumped_schema`.
- `demo/tests/ui_smoke.spec.js`: full control-surface smoke green.
- Run: `cargo xtask verify` (fmt, clippy, tests, doc-check, COMPAT, deny) + the demo smoke suite.

**Risk & rollback.** This item is the safety net; if a control cannot be made deterministic, it
does not ship (R-5 is non-negotiable). No production surface.

---

## Gates (Definition of Done for the enhancement)

- `cargo xtask verify` green; demo Playwright smoke green.
- Every new `SimSnapshot` field registered in `docs/COMPAT.md` with a schema bump (R-4).
- Determinism/replay proven for every control (W6); the reproducer seed reproduces exactly (R-5).
- WASM and sandbox `/sim/*` paths stay in parity (no control in one mode only).
- The demo remains labeled a teaching/DevRel asset; no new control is presented as a
  correctness guarantee (RULES). No numeric self-score (R-7).
- `INDEX.md` "Execution / supporting plans" lists this plan.
