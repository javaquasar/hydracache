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

- **Invariant verdicts are real.** The verdict panel is produced by the actual
  invariant checker (consensus prefix, durability, no-tombstone-resurrection,
  convergence, read-your-writes) — the same checkers the `0.44` deterministic
  simulation testing (DST) gates use. A reported violation is a real violation of
  real state.
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
  purely presentational; positions carry no engine meaning.
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
persistence, configurable consistency levels, and the real external client wire
protocol. `hydracache-sim` is a **sans-IO seam over the core logic**, not the
production runtime.

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

The default engine is the local WASM package. To drive the same UI through the
sandbox HTTP API instead, run the sandbox and add `engine=server` plus the API
base:

```powershell
cargo run -p hydracache-sandbox -- --backend memory
```

```text
http://127.0.0.1:5173/demo/?engine=server&api=http://127.0.0.1:3000
```

The page URL carries `seed`, `steps`, and `scenario`, so a shared URL can be
replayed locally. The copy-reproducer button emits the matching `hydracache-sim`
CLI command.

## DevRel CI

`.github/workflows/demo.yml` runs outside the fast PR gate. It builds the WASM
package, runs the headless UI and seed-share smoke tests, and publishes the
static `demo/` artifact to GitHub Pages on nightly/manual/tag runs.

The local C7 smoke gate uses the same tooling:

```powershell
npx --prefix demo playwright test
```

The Playwright config runs both the desktop `1440x900` and mobile `390x844`
viewports and checks the liquid-glass accessibility fallbacks.
