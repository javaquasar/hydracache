# HydraCache 0.50.0 Interactive Cluster Simulator Demo — Codex Execution Plan

> **At a glance**
> - **What:** a **seed-reproducible browser demo** built on the real `0.44` `hydracache-sim` engine, where you drop/partition/heal links and crash/restart nodes and watch — live — committed-log agreement, leader/term, per-operation consistency-level outcomes, convergence, and the **real invariant-checker verdict** (green = holds, red = violation + the seed). The TigerBeetle `sim.tigerbeetle.com` analogue.
> - **Why:** make the "correctness as a product feature" wedge (`POSITIONING.md` §2) *visible and persuasive*. Consistency/partition behavior is the hardest thing to sell in prose; a "tear the network, watch it stay correct / converge / fail loud" demo is the strongest possible pitch, onboarding aid, and conference asset.
> - **After (depends on):** `0.44` (the `hydracache-sim` deterministic simulator) — **already shipped**, so this can be pulled forward ahead of `0.46`–`0.49` at any time; it is numbered `0.50` only to avoid renumbering the in-flight cluster/prod line, not because it must wait for them.
> - **Unblocks:** — (DevRel / marketing artifact; nothing depends on it).
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · engine: [`V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md`](V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md) · pitch: [`../POSITIONING.md`](../POSITIONING.md).
>
> This release is a **thin visualization** over the `0.44` simulator — it adds no cluster logic and is not the correctness gate (the gates remain the `0.44` DST suites). It stays small, opt-in, and never blocks the cluster/prod releases.

This plan is written for an autonomous coding agent (Codex). Read [`CLAUDE.md`](../../CLAUDE.md),
[`docs/RULES.md`](../RULES.md), and the `0.44` DST plan first. One work item = one
commit/PR; after each, run its Definition of Done **and** `cargo xtask verify`; never
push red.

> **Addendum scope:** this plan also carries a **Pre-0.50 debt closure** section (below)
> recording the debts found auditing the shipped `0.45`–`0.48` releases. Those items are
> **independent of the demo** — they neither block nor depend on the W1–W7 demo work — but
> they are attached here because `0.50` is the next planned roadmap touch-point and they
> should be closed before/with it. Each is a standalone commit/PR.

## Pre-0.50 debt closure (audit of shipped releases 0.45–0.48)

An audit of the `0.45`–`0.48` plans against the implementation found **no feature drift**:
the work items are implemented and tested (`hydracache-server` + `deploy/{k8s,helm,dashboards}`
+ runbooks exist; tests cover every work item; sources contain no `todo!`/`unimplemented!`/
`FIXME`; `#[ignore]` is confined to the chaos/soak/Docker tier per R-5). The debts are
**documentation, compatibility, and process** gaps that the gates do not currently catch
(`doc-check` validates only `releases.toml`). Close these as `D1`–`D4`.

### D1. COMPAT register is missing the 0.45/0.46/0.47 durable/wire artifacts (R-4) — primary

**Problem.** `docs/COMPAT.md` jumps from `0.44` straight to `0.48`: there are **no sections
or table rows for `0.45`, `0.46`, `0.47`**, yet those releases introduced durable or
wire-visible artifacts that R-4 requires registering (with version, writer, reader-
compatibility window, and failure mode).

**Where / what to add.**
- **0.45 (active-active):** the **CRDT value encoding** (durable + replicated), the **WAN
  `RegionLink` frame** (cross-process/region), and cross-region **anti-entropy exchange
  records**. Files: `grid/crdt.rs`, `grid/region_link.rs`, `grid/active_active.rs`,
  `tests/anti_entropy.rs`.
- **0.46 (resilience):** the **durable replayable invalidation stream** (W6 — explicitly a
  durable artifact), **hinted-handoff records**, and the **Merkle repair exchange** wire
  shape. Files: `grid/invalidation_ring.rs`, `grid/hinted_handoff.rs`, `grid/merkle_repair.rs`.
- **0.47 (causal+):** the **session token / `ClientContext` wire format** (transmitted to
  consumers) and the **session watermark**. Files: `grid/session_context.rs`,
  `grid/session_lifecycle.rs`.

**Fix.** For each artifact add a `Versioned Artifacts` table row (version `1`, writer,
reader-compat, fail-loud failure mode) **and** a `## 0.45 …` / `## 0.46 …` / `## 0.47 …`
narrative section, mirroring the existing `0.42`–`0.44`/`0.48` entries. Readers must refuse
unknown-future versions loud (R-3). Verify the on-disk/wire format version each artifact
actually emits in code before writing the row (do not guess the version number).

**Acceptance.** `docs/COMPAT.md` has `0.45`/`0.46`/`0.47` sections; every durable/wire
artifact those releases added is registered; a reviewer can map each table row to the
emitting code path.

### D2. Dangling plan cross-references after the +1 renumber (R-11)

**Problem.** After the DST-insertion renumber, the `Deferred` sections still point to
pre-renumber filenames that no longer exist:
- `docs/plans/V0_45_ACTIVE_ACTIVE_MULTIREGION_PLAN.md` (~lines 537, 539) →
  old pre-renumber cluster-resilience and cross-region-session filenames.
- `docs/plans/V0_46_CLUSTER_RESILIENCE_AND_COORDINATION_PLAN.md` (~line 565) →
  the old pre-renumber cross-region-session filename.

**Fix.** Repoint to the real files (`V0_46_CLUSTER_RESILIENCE_AND_COORDINATION_PLAN.md`,
`V0_47_CROSS_REGION_SESSION_CONSISTENCY_PLAN.md`). **Recurrence guard:** extend
`crates/xtask/src/doc_check.rs` to validate in-prose `V0_*.md` plan links resolve to files
on disk (today it only checks `releases.toml` `file` entries), with a test in
`crates/xtask/tests/doc_check.rs`.

**Acceptance.** No `V0_4x_*.md` reference in any plan points to a non-existent file;
`doc-check` fails on a dangling in-prose plan link.

### D3. ADR numbering collisions

**Problem.** `docs/adr/` has duplicate numbers across two schemes
(`0001-gossip-…` vs `0001-ownership`, `0002-…` ×2, `0003-…` ×2, `0004-…` ×2, `0005-…` ×2)
plus a separate `ADR-00xx` scheme — ADRs are not uniquely addressable, which weakens the
cross-references plans rely on (e.g. `0.49` W2 expects `0006-why-not-clone-hibernate…`,
patch P7 expects `0007-client-wire-framing`).

**Fix.** Renumber to a single monotonic scheme, add/refresh `docs/adr/README.md` as the
index, and fix any plan references to the renamed ADRs. Reserve `0006`/`0007` for the
`0.49` ADRs (create them when `0.49` starts, not here).

**Acceptance.** ADR numbers are unique; `docs/adr/README.md` lists them; no plan references
a missing/duplicate ADR number.

### D4. Perpetually-deferred items have no committed home (record as TD)

**Problem.** **Automatic home-region placement / latency-based home assignment** and
**provider-specific autoscaler controllers** have been carried as "deferred" across
`0.45` → `0.46` → `0.47` with no target release — an accumulating deferral rather than a
tracked decision.

**Fix.** Create `docs/technical-debt/TD-0004-deferred-placement-and-autoscaling.md`
capturing both, with current status, why-deferred, and a candidate target release; replace
the loose "deferred" prose in the three plans' `Deferred` sections with a reference to
`TD-0004`.

**Acceptance.** `TD-0004` exists; the three plans reference it instead of re-stating the
deferral; the item has an owner/target, not an open-ended "later".

### Debt-closure gates

```powershell
cargo xtask doc-check          # after D1/D2: COMPAT + in-prose plan links resolve
cargo test -p xtask --locked   # after D2: the new dangling-plan-link guard
```

These are doc/process items: each is its own commit, and none of them is part of the demo's
W1–W7 scope or Final Decision below.

## The non-negotiable principle

**The demo runs the real `hydracache-sim` engine — never a scripted animation.** Its
credibility (and TigerBeetle's) comes entirely from showing the *actual* deterministic
simulator and its *actual* invariant-checker verdicts, including the honest failure and
"can't make progress" cases. A demo that fakes success is worse than no demo — it is
overclaiming and violates the project's correctness ethos (R-3, R-7). Every state the UI
shows must be a serialization of a real `SimWorld` step.

## Why this is cheap (reuse, don't rebuild)

It is a **thin front-end over assets that already exist after `0.44`**:
`hydracache-sim` (`SimWorld`, `SimNetwork` with partition/loss/delay, `SimStorage`,
`InvariantChecker`, seed/replay). The demo adds (a) a WASM binding, (b) a stable JSON
state schema, and (c) a static web UI. No cluster logic is reimplemented. The existing
`hydracache-sandbox` crate is the fallback host for a server-driven variant.

## Non-Goals

- **Not a correctness gate.** The gate is the `0.44` DST fast budget + nightly soak
  (`cargo xtask verify` already runs `hydracache-sim --test dst_budget`). This demo is
  communication, not verification.
- **Not a production tool, not a real cluster.** It is a single-process simulation; it
  does not connect to or operate real nodes.
- **No new cluster behavior.** It only visualizes `hydracache-sim`; if the demo needs a
  capability the sim lacks, that capability is added in the `0.44` plan, not here.
- **No heavy front-end stack.** Minimal/no JS framework, no bundler sprawl; one static
  page + the WASM module. Keep the dependency surface tiny (R-10 spirit).

## Architecture decision

Primary: **WASM, client-side, zero-backend.** Compile `hydracache-sim` to
`wasm32-unknown-unknown` via `wasm-bindgen`; the simulator runs entirely in the browser,
deterministically, from a seed in the URL. This gives a static site (GitHub Pages),
viral "share a seed" reproducibility, and zero hosting cost. The sim is already sans-IO
+ logical-clock + seeded-RNG (`0.44` W1/W2), so it is wasm-friendly (no real time,
threads, or fs).

Fallback: **server-driven** over `hydracache-sandbox` (axum) exposing the same JSON
schema — only if a WASM constraint bites. Both share the W2 schema.

## Dependency Graph

```
0.44 hydracache-sim (SimWorld / SimNetwork / SimStorage / InvariantChecker / seed-replay)
        │
        ▼
W1 hydracache-sim-wasm (wasm-bindgen bindings)
        │
        ▼
W2 SimSnapshot JSON schema (stable, versioned)
        │
        ▼
W3 static web UI ──► W4 seed reproducibility & sharing ──► W5 curated scenario presets
        │
        ▼
W6 honesty guardrails + CI build + static-site packaging
        │
        ▼
W7 (optional) server-driven variant over hydracache-sandbox
```

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests + exact
commands) / Risk & rollback.**

---

## W1. `hydracache-sim-wasm` — browser bindings over the real engine

**Goal.** Expose the `0.44` `hydracache-sim` `SimWorld` to JavaScript via WASM, with no
loss of determinism.

**Files.** new crate `crates/hydracache-sim-wasm/` (`Cargo.toml` with
`crate-type = ["cdylib"]`, `wasm-bindgen` dep), `src/lib.rs`.

**Rust sketch.**
```rust
// crates/hydracache-sim-wasm/src/lib.rs
use wasm_bindgen::prelude::*;
use hydracache_sim::{SimWorld, SimConfig, Fault};

#[wasm_bindgen]
pub struct SimHandle { world: SimWorld }

#[wasm_bindgen]
impl SimHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64, config_json: &str) -> SimHandle { /* SimConfig from JSON */ }
    pub fn step(&mut self, n: u32);                 // advance n logical steps
    pub fn inject(&mut self, fault_json: &str);     // partition/drop/delay/crash/heal
    pub fn snapshot_json(&self) -> String;          // W2 SimSnapshot
    pub fn verdict_json(&self) -> String;           // real InvariantChecker output
    pub fn seed(&self) -> u64;
}
```

**Steps.**
1. Add the crate (cdylib); depend on `hydracache-sim`.
2. Confirm `hydracache-sim` compiles to `wasm32-unknown-unknown` (no `std::time`,
   threads, or fs in the simulated path — it is sans-IO; fix any stray non-wasm use in
   the `0.44` crate, behind a `#[cfg]` if needed, with no behavior change).
3. Expose `new/step/inject/snapshot_json/verdict_json/seed`.

**DoD.** `crates/hydracache-sim-wasm/tests/wasm_parity.rs`
- `same_seed_native_and_wasm_match` (run under `wasm-pack test --node`): identical
  snapshot/verdict hashes for a seed in native vs wasm builds — **determinism is
  preserved across the wasm boundary**.
- Build check: `cargo build -p hydracache-sim-wasm --target wasm32-unknown-unknown --locked`.
- Run: `wasm-pack test --node crates/hydracache-sim-wasm` (nightly/dev tier) + the build
  check in CI.

**Risk & rollback.** If the sim has a non-wasm dependency, that is a `0.44` fix, not a
demo hack. Revert removes the crate; the engine and its CI gate are unaffected.

---

## W2. `SimSnapshot` JSON schema (stable, versioned)

**Goal.** One serde schema the UI renders, decoupling UI from engine internals.

**Files.** `crates/hydracache-sim/src/snapshot.rs` (the canonical `SimSnapshot` +
`schema_version`), re-exported through W1.

**Design / contract.** A versioned snapshot capturing exactly what the UI shows:
```rust
pub struct SimSnapshot {
    pub schema_version: u16,
    pub seed: u64, pub step: u64,
    pub nodes: Vec<NodeView>,   // id, zone/region, role, term, commit_index, applied_index, up/crashed
    pub links: Vec<LinkView>,   // (a,b), state: Up|Partitioned|Delayed(ms), in_flight: u32
    pub keys: Vec<KeyView>,     // sampled key -> per-replica (version, epoch)
    pub verdict: VerdictView,   // Holding | Violated{ invariant, detail } (from InvariantChecker)
    pub progress: ProgressView, // committed entries, last leader change, convergence: Converged|Diverged
}
```

**Steps.** Define the structs in `hydracache-sim`; `snapshot_json()`/`verdict_json()`
(W1) serialize them; bump `schema_version` on any change.

**DoD.** `crates/hydracache-sim/tests/snapshot_schema.rs`
- `snapshot_roundtrips_and_is_versioned` (unit).
- `verdict_reflects_real_checker` (unit): a deliberately-broken history yields
  `Violated` in the snapshot (ties `0.44` W6 meta-tests — no fake green).
- Run: `cargo test -p hydracache-sim --locked snapshot_schema`.

---

## W3. Static web UI

**Goal.** The interactive front-end: see the cluster, break the network, watch the
verdict.

**Files.** `demo/` (static): `demo/index.html`, `demo/app.js`, `demo/style.css`,
`demo/pkg/` (wasm-pack output, git-ignored / built in CI).

**Steps.**
1. Render nodes as a graph (zone/region grouped); each link is clickable to
   **drop / partition / delay / heal**; per-node **crash / restart** buttons.
2. Controls: workload on/off, **step / play / pause / speed**, reset.
3. Panels (all from W2 snapshot, refreshed each step): committed-log agreement across
   nodes, current leader/term, per-operation **consistency-level outcomes** (ONE /
   LOCAL_QUORUM / QUORUM / EACH_QUORUM / ALL — ties `0.46` W1), convergence status, and a
   **prominent verdict banner** (green "invariants hold" / red "violation: <invariant>
   @ seed <S>").
4. No backend: `app.js` loads the WASM and drives `SimHandle`.

**DoD.** `demo/tests/ui_smoke.{spec}` (headless, nightly tier)
- `loads_steps_and_renders_verdict` (headless browser / wasm-pack-in-node smoke): the
  page loads the WASM, runs steps, and shows a verdict element.
- `clicking_partition_updates_link_state` (smoke).
- Run: nightly DevRel job (headless browser); not in the fast PR gate.

**Risk & rollback.** UI is isolated under `demo/`; revert removes it without touching the
library.

---

## W4. Seed reproducibility & sharing

**Goal.** Anyone can reproduce exactly what they see — parity with TigerBeetle's seed
sharing.

**Files.** `demo/app.js` (URL state), `demo/share.js`.

**Steps.**
1. Encode `?seed=<S>&steps=<N>&scenario=<name>` in the URL; on load, restore exactly.
2. A **"copy reproducer"** button emits the CLI line
   `cargo run -p hydracache-sim --bin vopr -- --seed <S> --steps <N>` so a viewer can
   replay the *same* run in the test harness.
3. If the verdict turns red, the banner shows the seed and the reproducer command.

**DoD.** `demo/tests/seed_share.spec`
- `url_seed_reproduces_identical_run` (smoke): two loads of the same URL → identical
  snapshot hash.
- Run: nightly DevRel job.

---

## W5. Curated scenario presets (incl. honest failures)

**Goal.** One-click stories that teach the real behavior — including the cases where the
grid correctly *refuses* or *cannot* make progress.

**Files.** `crates/hydracache-sim/src/scenarios.rs` (named seeds/scripts),
`demo/scenarios.js`.

**Steps.** Curate presets, each a deterministic seed/script demonstrating a documented
behavior:
- **Minority partition → cannot commit** (safety over availability).
- **Leader crash → failover, no committed loss.**
- **Symmetric partition + heal → convergence** (anti-entropy/repair, `0.46`).
- **`EachQuorum` under region loss → fails loud** (not a silent partial — `0.46` W1 /
  R-3).
- **Delete vs concurrent write → tombstone wins, no resurrection** (A5).

Each preset must show the *real* verdict (some are "progress halted, invariants still
hold" — that is the honest, correct outcome and should be presented as a feature).

**DoD.** `crates/hydracache-sim/tests/scenarios.rs`
- `each_preset_seed_is_deterministic_and_matches_expected_verdict` (unit): every preset's
  seed yields its documented outcome (e.g. minority-partition → `Holding` + no progress;
  not a spurious violation).
- Run: `cargo test -p hydracache-sim --locked scenarios`.

---

## W6. Honesty guardrails, CI build & static-site packaging

**Goal.** Ship it, keep it truthful, keep it from rotting.

**Files.** `.github/workflows/demo.yml` (or a job in CI), `demo/README.md`, a banner in
`demo/index.html`, a link from the repo `README.md` and `POSITIONING.md`.

**Steps.**
1. The UI shows a persistent banner: *"This runs the real `hydracache-sim` engine,
   seed&nbsp;`<S>` — verdicts are produced by the actual invariant checker."*
2. CI job: `cargo build -p hydracache-sim-wasm --target wasm32-unknown-unknown` +
   `wasm-pack build` + the headless smoke (W3/W4) + publish the static `demo/` to GitHub
   Pages (nightly / on tag).
3. A guard test asserts the demo's preset list (W5) matches the scenarios defined in
   `hydracache-sim` (no demo-only scenario without an engine seed) — prevents drift /
   fake scenarios.
4. Link the live demo from `README.md` and `POSITIONING.md` (the pitch surface).

**DoD.** `crates/hydracache-sim/tests/demo_scenarios_match.rs`
- `demo_presets_have_engine_seeds` (unit): every UI preset name resolves to a real
  `scenarios.rs` entry.
- CI: the demo build + smoke job is green on the nightly/DevRel tier.
- Run: `cargo test -p hydracache-sim --locked demo_scenarios_match` + the CI job.

**Risk & rollback.** Demo drifting from engine behavior is the key risk; the W6.3 guard
test + "real engine" banner mitigate it. The demo is never on the fast PR gate, so it
cannot block development.

---

## W7. (Optional) server-driven variant over `hydracache-sandbox`

**Goal.** A non-WASM host for environments that prefer a server, reusing the existing
sandbox.

**Files.** `crates/hydracache-sandbox/src/sim_routes.rs` (axum routes:
`POST /sim/new`, `/sim/step`, `/sim/inject`, `GET /sim/snapshot`) emitting the **same W2
schema**.

**Steps.** Expose `SimWorld` over the sandbox's axum app with the W2 schema; the same
`demo/app.js` can target either the WASM module or this API via a config flag.

**DoD.** `crates/hydracache-sandbox/tests/sim_routes.rs`
- `sim_routes_emit_w2_schema_and_step_deterministically` (integration).
- Run: `cargo test -p hydracache-sandbox --locked sim_routes`.

**Risk & rollback.** Optional; skip if WASM (W1) is sufficient.

---

## Fault Model and Test Tiering

The demo reuses the `0.44` fault model (the `SimNetwork`/`SimStorage` faults) verbatim —
it does not invent faults. Tiers: the engine's correctness tests stay where they are
(`hydracache-sim` fast budget in `cargo xtask verify`, nightly soak); the demo's own
tests (WASM parity, UI/seed smoke, scenario-match) run on a **separate nightly DevRel
job** and never gate PRs (R-5 spirit: time-/browser-heavy out of the fast path).

## Release Gates (DevRel artifact)

Focused (engine-side, run on PR via the normal gates):

```powershell
cargo test -p hydracache-sim --locked snapshot_schema
cargo test -p hydracache-sim --locked scenarios
cargo test -p hydracache-sim --locked demo_scenarios_match
cargo build -p hydracache-sim-wasm --target wasm32-unknown-unknown --locked
```

DevRel nightly (browser/WASM tier, separate job — not in `cargo xtask verify`):

```bash
wasm-pack test --node crates/hydracache-sim-wasm
# headless UI + seed-share smoke; build + publish demo/ to GitHub Pages
```

## Final Decision

The demo may be linked from the README/site as "the HydraCache cluster simulator" only
if **all** hold:

- W1: `hydracache-sim-wasm` builds for `wasm32`; native↔wasm runs match bit-for-bit by
  seed.
- W2: the `SimSnapshot`/verdict schema is stable, versioned, and reflects the **real**
  `InvariantChecker` (a broken history shows `Violated`).
- W3–W5: the UI lets a visitor partition/crash/heal and shows committed-log, leader,
  consistency-level outcomes, convergence, and the live verdict; presets cover the honest
  failure/halt cases; seeds reproduce.
- W6: the "real engine, seed X" banner is present; a guard test forbids demo-only
  scenarios; the build+smoke CI job is green and publishes the static site.

If any fails, the demo stays internal/unlinked and the gap is documented — it must never
present a behavior the engine does not actually produce.
