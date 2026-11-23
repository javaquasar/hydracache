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
- **Cluster / node FSM, client routing, and heartbeat traffic are approximations.**
  `connected_node` (which node a client/subscriber attaches to) is a deterministic
  hash, not the real smart-routing logic; follower→leader heartbeat acks exist for
  legibility.
- **Replication factor = the whole cluster.** The simulator is a single
  fully-replicated Raft group: every committed value lives on every node. The
  product has a configurable `ReplicationConfig` (replication factor, read/write
  quorums, sync/async backups; default is local-first `replication_factor = 1`).
  The demo does **not** model partition placement, backup counts, or quorum
  reads/writes.

### Visualization only — no engine meaning

- **The force-directed graph layout and physics** (drag, pan, zoom, spacing) are
  purely presentational; positions carry no engine meaning. The view fits the
  whole cluster automatically until you pan or zoom (Ctrl/Cmd + wheel, pinch,
  or the +/− buttons); the ⤢ button returns to the fitted view. A plain wheel
  scrolls the page.
- **Node badges, link labels, the legend, timeline ticks, and the light/dark
  theme** restate engine state (role, link state, recorded actions, leader and
  verdict changes) for readability; they add no information of their own. The
  theme follows the OS setting until you pick one with the ◐ button.
- **Packet and data-flow animations.** `in_flight` messages are real, but their
  on-screen travel timing is cosmetic. The client → node → replication →
  subscriber **pulse** fired on a push is a choreographed overlay; its "fan out to
  every node" matches the simulator's full-replication model, not the product's
  configurable RF.
- **Auto-stepping after an intervention** (`settleAfterIntervention`) is a UI
  convenience so a paused cluster visibly reacts; the engine has no such
  self-advancing clock.

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
  not produce.
- **Timeline.** The slider under the graph rebuilds any earlier step of the
  current run by replaying its script prefix (interventions up to that step,
  resumed exactly there), so a rewound view is the real engine state at that
  step. Events list recorded interventions plus the leader changes and verdict
  transitions this page observed. Step, Play, or any other action while rewound
  continues from the visible step and drops the later recording.
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
- **Keys panel.** Per-node stored checksums for each sampled key; live replicas
  that differ from the majority are highlighted, down nodes are struck through.

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
