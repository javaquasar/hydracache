# HydraCache Cluster Simulator Demo

This directory contains the static browser demo for the real deterministic
`hydracache-sim` engine. The UI does not script fake green animations: every
visible state is serialized from a real `SimWorld` snapshot, and every verdict is
produced by the actual invariant checker.

The published DevRel build is intended for explanation and reproduction, not as
the release correctness gate. Correctness remains covered by the simulator test
suite and `cargo xtask verify`.

## Fidelity — what the demo reflects vs. models vs. visualizes

The demo runs the real deterministic `hydracache-sim` engine, but it is a
**teaching lab, not the production runtime**. Different parts of the screen have
very different fidelity. Read this before treating anything here as a product
guarantee.

### Faithful — real engine code and real checkers

- **Invariant verdicts are real, and exactly as broad as listed.** The verdict
  panel is produced by the actual invariant checker. The live world checks the
  workload-history invariants (`read-your-writes`, `invalidate-read`), the
  election/topology invariants (`election_safety`, `leader_requires_quorum`,
  `no_stale_leader_writes`, `index_monotonicity`, `catchup_no_skip`,
  `event_after_commit`) and `resource_bounded_growth`; the page lists them from
  the snapshot's `checked_invariants`. The replica-snapshot invariants
  (`consensus-prefix`, `durability`, `tombstone-resurrection`, `convergence`)
  run in the DST suites but are **not** part of the live verdict. A reported
  violation is a real violation of real state, and its first violating step is
  kept.
- **Deterministic and replayable.** Execution is driven by a seeded `SimRng` /
  `SimClock`; the same `seed` reproduces the same run. The shared URL and the
  copy-reproducer button reproduce the exact run.
- **The fault model is real.** Partition / crash / disable / delay / drop change
  real simulator state, not just the picture.
- **Value semantics are real.** Committed log, checksums, tombstones, and
  single-key conditional records run on the actual `hydracache` core types.
- **Native/server leader election can be real `raft-rs`.** When the UI is backed
  by the native sandbox/server engine, snapshots report `election_source: "raft"`.
  That path drives real `raft-rs` `RawNode`s deterministically over the seeded
  simulator network. It is still the lab harness, not the product transport or
  durable runtime.

### Modeled — simulator-specific, and explicitly labelled

- **Browser-only leader election remains a labelled model.** The wasm build
  reports `election_source: "sim-model"` with the disclosure *"deterministic
  simulator election model for the lab; not a product consensus claim"*. That FSM
  is validated against the native raft harness for safety, quorum denial, and
  bounded convergence, but it is **not a product consensus claim**.
  Since 0.74 it honours cut links the way the raft path does: no election signal
  crosses a partitioned link, votes and heartbeat rounds count only peers
  connected both ways, a leader that no longer reaches a quorum steps down, and a
  node that cannot reach a quorum does not campaign.
- **Cluster / node FSM, client routing, and heartbeat traffic are approximations.**
  A client connects to one member, picked by a deterministic hash of its name,
  and stays there until that member goes down (snapshot schema 10). That is
  not the real smart-routing logic. A new client is admitted only while there
  is a leader (a simulated `ClientUpsert`). Follower→leader heartbeat acks exist
  for legibility.
- **The write path is a choice** (Clients tab, *Write path*; snapshot schema 11):
  - *full broadcast* is the simulator's original model and the default: the
    writer's member sends every write to every node. The built-in workload
    and its invariants run only in this model.
  - *local-first* is the product default: the writer's member keeps the
    value and the other members drop their copy.
  - *replicated* models the product's opt-in placement and consistency
    primitives: rendezvous owners (`rank_replicas`), a forward to the primary,
    copies to the backups, and acknowledgements counted with
    `ConsistencyLevel`. A write the live owners cannot meet is refused with the
    product's reason.

  The product ships those primitives but no runtime replicated write path, so
  *replicated* is a model, not shipped behaviour. Hinted handoff and repair are
  not modeled yet: a copy lost in flight leaves its write pending.

### Visualization only — no engine meaning

- **The graph layout** (drag, pan, zoom, spacing) is purely presentational;
  positions carry no engine meaning. It is tidy and deterministic: cluster
  nodes on a ring in id order, writers and subscribers just outside it beside
  their node, so the same run draws the same picture. A dragged vertex stays
  where it is put until the Tidy button puts everything back. The presentation
  button (or P) lets the graph fill the screen with only the run controls and
  the timeline around it; Esc leaves. A busy edge folds its routine heartbeats
  and acks into a "+N" badge (votes, data, delayed and lost messages always
  show), and nodes flash only for role, term and membership changes or a write
  a push carried there. The view fits the
  whole cluster automatically until you pan or zoom (Ctrl/Cmd + wheel, pinch,
  or the +/− buttons); the ⤢ button returns to the fitted view. A plain wheel
  scrolls the page.
- **Node badges, link labels, the legend, timeline ticks, and the light/dark
  theme** restate engine state (role, link state, recorded actions, leader and
  verdict changes) for readability; they add no information of their own. Node
  badges draw a power icon for a disabled node and a slashed circle for an
  isolated one (the engine reports it as `disconnected`). The bar under each
  node is the writes that node applied (snapshot `applied_writes`) against
  the most up-to-date live node; writes already on their way to it are drawn
  lighter, so a node one replication step behind reads as arriving, and it
  turns amber only when writes are missing (crashed, isolated, behind a cut).
  A red dashed track adds the second signal: its stored values differ from
  the live majority once replication has settled. Link labels are pills; a delayed link
  shows the longest pending delay on the pair, counting down, and a cut link
  how many messages it has refused (snapshot link `drops`, cumulative for the
  run). When a pair's drop counters rise, a loss marker bursts at the cut, or
  at the node that received messages while it was down; the link's tooltip
  and accessible name give the breakdown by cause. The legend folds
  into a "Legend" button (open on wide screens, folded on narrow ones, the
  viewer's choice remembered), and the fit keeps the cluster clear of it. The
  theme follows the OS setting until you pick one with the ◐ button.
- **Packet and data-flow animations.** `in_flight` messages are real, and their
  travel follows the engine's delivery schedule: a message's flight lasts its
  `remaining_millis` (converted to steps) plus one step, so an undelayed message
  crosses its edge in one step and a delayed one advances by its share per
  step. A message the engine will drop on delivery (its directed link is cut,
  or its target is crashed, disabled or isolated) is drawn dashed and fades
  before arrival ("lost" in the legend). The wall-clock speed of that travel
  follows the play speed and is cosmetic. Shapes tell message families apart without
  colour: votes are diamonds, replicated writes squares (labelled with their
  key), invalidations triangles, and control traffic (heartbeats, acks) small
  faint dots.
- **Within-step staging and role overlays.** The engine changes state once per
  step; the graph orders what it shows inside the step. A role change that a
  message causes is shown when that message lands (a voter turns when the vote
  request reaches it, a follower when the heartbeat does); local causes (a
  timeout, a crash, a user action) show at once. The leader's star travelling
  from the previous leader, the "term N" label, the ring around the node
  running the election and its "votes" tally (its own vote plus vote responses
  as they land, against the quorum) restate in-flight messages and snapshot
  state. Reduced motion shows every change at once, without overlays.
- **Event effects.** A node going down greys out with a short shudder; one
  coming back shows a rotating ring while its replicas lag the live majority
  and flashes once they agree. A delivered vote, write or invalidation
  ripples at its target; a message the engine drops on delivery bursts where
  it vanishes. A partitioned link is drawn broken (a gap and two bars), a
  delayed one crawls, and a pair with held-back messages (not deliverable on
  the next step) swells with that backlog. Graphics meet 3:1 contrast against
  the canvas in both themes (WCAG 1.4.11). Reduced motion turns the effects
  off. The client → node → replication →
  subscriber **pulse** fired on a push replays what the engine did: it is
  built from the snapshot right after the push (the push step and the entry
  node that applied it: the member the writer is connected to) and the settled
  snapshot (each replica's `applied_at_step`, copies still in flight, copies
  that will never arrive, each subscriber's delivery step). Two parts are
  drawn, not engine facts: its wall-clock pace (a fixed time per engine
  step), and the shape of the writer's link. Since snapshot schema 10 the
  write enters on the writer's own member, as in the product, so the request
  travels only along that link; the Raft leader is not on the path. Its "fan out to
  every node" matches the simulator's full-replication model, not the
  product's configurable RF.
- **Auto-stepping after an intervention** (`settleAfterIntervention`) is a UI
  convenience so a paused cluster visibly reacts; the engine has no such
  self-advancing clock.

Three earlier gaps here (per-node progress, send-time drops, push timing)
were closed as
[TD-0015](../docs/technical-debt/TD-0015-demo-fidelity-gaps.md).

### Not represented at all — the production stack

Real networked Raft transport, the `hydracache-server` daemon, the durable
storage engine and recovery (`0.51`), mTLS / encryption-at-rest (`0.48`),
persistence, per-level consistency reads and writes, and the real external
client wire protocol. `hydracache-sim` is a **sans-IO seam over the core logic**, not the
production runtime.

This layer is also where the currently open deployment debt lives, so a perfect
demo run says nothing about it: the `0.59` daemon cluster transport has no TLS
termination or peer auth yet
([TD-0010](../docs/technical-debt/TD-0010-cluster-transport-tls-and-peer-auth.md)),
and its raft voter set is static — no runtime join/leave of voters, identity
coupled to the listen address
([TD-0011](../docs/technical-debt/TD-0011-dynamic-raft-membership-and-node-identity.md)).
Both are scheduled for `0.60`
([plan](../docs/plans/V0_60_NETWORKED_GRID_HARDENING_PLAN.md)). The lab's
"kill leader → re-elect" and the real daemon E2E
(`crates/hydracache-server/tests/grid_host.rs`) are different machinery: a
seeded simulator network here, real `ServerRuntime` processes over loopback
there.

### Bottom line

Trust the **invariant verdicts, how state reacts to faults, and native/server
leader election when the source chip says `raft`**. Treat **wasm `sim-model`
election, replication, transport, deployment, and every animation/layout** as
illustration, not a production guarantee. The lab "shows the same seeded engine
and invariant checker to humans **without replacing the release gates**."

## Local Build

```powershell
rustup target add wasm32-unknown-unknown
cargo build -p hydracache-sim-wasm --target wasm32-unknown-unknown --locked
wasm-pack build crates/hydracache-sim-wasm --target web --out-dir ../../demo/pkg --release -- --locked
npm --prefix demo ci
npm --prefix demo run build
npm --prefix demo run serve
```

Then open:

```text
http://127.0.0.1:5173/demo/
```

The dev server serves only `demo/`. It also mounts the same files at
`http://127.0.0.1:5173/hydracache/demo/`, the path GitHub Pages uses, so relative
asset URLs can be checked locally exactly as they are published.

The default engine is the local WASM package. To drive the same UI through the
sandbox HTTP API instead, run the sandbox and add `engine=server` plus the API
base:

```powershell
cargo run -p hydracache-sandbox -- --backend memory
```

```text
http://127.0.0.1:5173/demo/?engine=server&api=http://127.0.0.1:3000
```

`?api=` accepts only a same-origin path or a loopback API (`localhost`,
`127.0.0.1`, `[::1]`); any other host is refused and the lab stays disabled, so a
shared link cannot make the page render another server's verdict.

## Using the lab

- **Guide.** Every scenario (and the default run) has a short guided tour.
  `Do it` runs the step's actions through the same engine calls as the buttons;
  the text describes what to look at and never claims an outcome the engine did
  not produce. After `Do it` the guide reads the result back from the snapshot
  (leader and term, delivery and replicas holding the value, live nodes
  against the quorum, unavailable consistency levels, agreement) and marks the
  step done; when the step's claim did not happen it says so (✗) and does not
  move on. Each step opens the side-panel tab it is about and spotlights where
  its result shows. The default tour crosses the quorum boundary: a second
  crash leaves one live node and no leader, then a restart recovers. Some steps
  ask for a prediction first; the right answer is whatever the engine then did.
  Step dots jump between steps, "Play the tour" runs the rest with pauses,
  "Copy link to this step" shares the run and the step (`?tourStep=N`), and the
  last step offers the next tour and the reproducer.
- **History strip.** Under the timeline, a lane shows which node led at each
  step the page observed (red: no leader) and a line shows committed entries.
  Steps the page did not render (a run-until jump) stay blank rather than
  guessed; a text summary is available to screen readers.
- **Timeline.** The slider under the graph rebuilds any earlier step of the
  current run by replaying its script prefix (interventions up to that step,
  resumed exactly there), so a rewound view is the real engine state at that
  step. Events list recorded interventions plus the leader changes and verdict
  transitions this page observed. Step, Play, or any other action while rewound
  continues from the visible step and drops the later recording.
  Events are grouped by step with an icon per kind (▸ action, ★ leader,
  ✕ / ✓ verdict); clicking a group shows that step.
- **Keyboard.** `Space` plays or pauses, `.` steps, `←` / `→` move one step on
  the timeline (`Shift` for ten), `F` fits the graph, `?` lists the shortcuts.
  They are ignored while typing in a field or when a focused control owns the
  key. Leader changes are also announced to screen readers.
- **Phones.** Once the controls scroll away, a dock at the bottom keeps Play,
  Step, the current step, and the verdict in reach.
- **Run to leader / replica agreement / violation.** Steps the engine until the
  condition holds, and stops loudly after 500 steps if it never does.
- **Verdict panel.** Lists the snapshot's `checked_invariants` (the exact set
  behind the verdict), the violation detail, and the first violating step.
- **Consistency panel.** For each product level (`one`, `local_quorum`,
  `quorum`, `each_quorum`, `all`) it shows whether an operation could be
  acknowledged right now, how many acknowledgements it needs, and the product's
  reason when it cannot (snapshot `consistency`). The rules are the product's
  `ConsistencyLevel` rules, mirrored in the simulator for the browser build and
  parity-tested against the product. It reports availability on the current
  topology; the simulator does not execute per-level reads or writes.
- **Panel tabs.** The side panels are grouped into Cluster (link, nodes,
  progress, signals), Clients, Data (keys, consistency), and Verdict. Tab badges
  restate what a hidden tab would show: nodes down, client count, replicas
  differing, and the verdict.
- **Writers and subscribers.** The Clients tab keeps them apart, as the engine
  does since 0.74. "+ Add writer" registers a writer for a namespace without
  writing (`add_client`); each writer row has its own Push and Remove
  (`remove_client`; the writer's recorded history stays). "+ Subscribe"
  creates a reader only (`subscribe` no longer creates a writer twin); each
  subscriber row shows its last event, lag and drops, and Unsubscribe
  (`unsubscribe`). Names are pre-filled with the next free one, namespace
  fields suggest the namespaces in use, and an existing writer or a missing
  subscription fails loud. On the graph a writer (↑) pushes when clicked, a
  subscriber (↓) opens its row, and a dotted line joins writers to the
  subscribers of their namespace. A push lights each graph vertex up when its copy
  arrives, timed by the engine: the entry node when the request arrives,
  each replica after as many steps as the engine took to deliver it, each
  subscriber at its delivery step. A replica whose copy is still in flight
  does not flash; one that will never get it (down, or behind a cut) shows
  the copy fading with a loss marker. Reduced motion flashes at once.
- **Keys heatmap.** A replica that takes a new value fades from its old colour to the new one; hovering a cell gives the whole checksum and the step the page saw it written. One row per sampled key, one column per node. A cell's
  colour is derived from its stored checksum, so equal checksums share a colour;
  live replicas that differ from the majority are outlined, down nodes are
  striped, and `—` means no replica.
- **What changed.** After every step or action the line under the graph lists
  the engine fields that changed since the previous view (node status, term,
  commit/applied index, link states, leader, committed entries, key checksums),
  and the matching nodes, rows, and key cells flash briefly.
- **Node details.** Click a node (or focus it and press Enter) for its engine
  fields — term, vote, commit and applied index, how far it trails the leader —
  and the same Crash / Isolate / Rejoin / Disable actions as the Nodes panel.
  Escape closes it.
- **When a violation appears.** The run pauses, the view rewinds to the first
  violating step the engine reported (`first_violation_step`), and a card shows
  the invariant, the checker's detail, and the reproducer buttons. Nodes are
  highlighted only when the detail text names them; the engine attaches no other
  node attribution to a violation.
- **First visit.** A short three-point introduction is shown once; the `?`
  button in the header brings it back.

## Sharing and reproducing a run

The page URL carries `seed`, `steps`, and `scenario`. Once the run has
interventions (node, link, workload, client actions, a mode change, or a loaded
scenario), it also carries `script=`: a `ReplayScriptV1` with every recorded
action and the step to resume at, which rebuilds the identical snapshot. The
copy-reproducer button emits that replay link, or the matching `hydracache-sim`
CLI command (`vopr --seed … --steps …`) for a plain seeded run.

`Download script` saves the same replay script as JSON. Replay it outside the
browser, including every intervention and the curated scenario it started from:

```powershell
cargo run -p hydracache-sim --bin vopr -- --script hydracache-replay-seed-80-step-42.json
```

`vopr --script` prints the run summary, `first_violation_step`, and the verdict,
and exits `2` on a violation, so a violation found in the browser can become a
regression test. A violation shown in the header always names the reproducer
for that run: the plain `--seed/--steps` command only when no intervention or
scenario shaped it.

## CI and publishing

Two workflows cover the demo; neither is part of the fast PR gate.

- `.github/workflows/docs-site.yml` owns publishing. It runs on pull requests
  that touch `demo/`, the simulator crates, or the docs, on pushes to `main`, and
  on manual dispatch. It builds the WASM package, copies the page modules
  (`index.html`, `style.css`, `app.js`, `scenarios.js`, `share.js`) and the
  `pkg/` output into `docs-site/book/demo/`, and runs `npm --prefix demo run build`
  and the full Playwright suite against that bundle served at
  `/hydracache/demo/`. On `main` pushes and manual runs it deploys the docs site
  and simulator to GitHub Pages
  (`https://javaquasar.github.io/hydracache/demo/`) and then verifies that every
  published simulator file matches the SHA-256 of the tested bundle.
- `.github/workflows/demo.yml` is a nightly, `main`-push, and manual smoke. It
  builds the WASM package and runs the UI and seed-share Playwright specs against
  `demo/` directly. It does not deploy Pages.

Tests: `npm --prefix demo test` runs the `node:test` unit suite (`demo/unit`,
URL/share and view-model helpers) and then Playwright on desktop and mobile
viewports. `demo/tests/server_engine.spec.js` drives the same UI through the
sandbox (`engine=server`, raft election); it is skipped unless
`HYDRACACHE_SANDBOX_URL` is set, and `demo.yml` runs it against a freshly built
sandbox:

```powershell
cargo run -p hydracache-sandbox -- --backend memory
```

```powershell
$env:HYDRACACHE_SANDBOX_URL = "http://127.0.0.1:3000"; npx --prefix demo playwright test server_engine --workers=1
```

`npm --prefix demo run build` (`scripts/check-static.mjs`) fails when a file the
page needs is missing (including `pkg/`, so build WASM first), when a module the
page loads is not in the `docs-site.yml` bundle list, when
`demo/package.json` does not carry the workspace version from `Cargo.toml`, or
when the dev server would serve a file outside `demo/`. Bump
`demo/package.json` (and its lockfile) together with the workspace version.

The local C7 smoke gate uses the same tooling:

```powershell
npx --prefix demo playwright test
```

The Playwright config runs both the desktop `1440x900` and mobile `390x844`
viewports and checks the liquid-glass accessibility fallbacks.

`tests/a11y.spec.js` runs axe-core (WCAG 2.2 A/AA rules) over each side-panel
tab, the dark theme, the node popover, the shortcuts dialog and the violation
card; any violation fails the test with the rule and the selectors it hit.

`tests/visual.spec.js` compares the graph-free panels against screenshot
baselines in `tests/visual.spec.js-snapshots/`. Baselines are per OS and per
viewport (`<name>-<viewport>-<platform>.png`); on a platform with no committed
baseline the tests are skipped, not failed. Record or refresh them after an
intended visual change:

```powershell
npx --prefix demo playwright test demo/tests/visual.spec.js --update-snapshots
```

`og-image.png` is the 1200x630 link-preview image named by the Open Graph and
Twitter card tags in `index.html`; `npm run build` checks that it exists, has
that size, and is bundled and checksummed by `docs-site.yml`. After a visible
UI change, re-render it from the running page (`npm --prefix demo run serve`
in another terminal):

```powershell
node demo/scripts/og-image.mjs
```
