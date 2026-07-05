# HydraCache 0.58.0 Endurance — Soak & Overload Hardening — Codex Execution Plan

> **At a glance**
> - **Status:** shipped.
> - **What:** close the single most honest remaining weakness — *"no multi-year soak … the remaining
>   weakness is longer-running soak/chaos history and proven behavior under sustained production
>   overload."* Concretely, with implementation tasks for the parts that **do not exist yet**:
>   (**W1**) turn the single-shot **VOPR** (`hydracache-sim/src/bin/vopr.rs`, one seed / N steps /
>   exit-0-or-2) into a **continuous, wall-clock-budgeted, multi-seed soak driver** that inspects
>   `SimOutcome.invariant_violations` itself (the existing `ReplayRunner` only trips on a synthetic
>   fault — see audit), records the first failing seed, and **minimizes** it via the existing
>   `ReplayRunner::shrink_failure`; (**W2**) **add** the missing resource accounting
>   (`SimStorage::footprint()`) and a **`BoundedGrowthChecker`** invariant so leaks-over-time are
>   caught, plus a real-server RSS/fd sampler; (**W3**) a **sustained-overload / backpressure** proof
>   over the shipped `AdmissionController` (`admission.rs`) — rejects counted, `in_flight` /
>   `memory_bytes` bounded, recovers-after — fixing any unbounded path found; (**W4**) a **real
>   multi-node chaos soak** on the `0.56` operator/kind harness; (**W5**) a **bounded, deterministic CI
>   soak gate** (a new `xtask verify` gate beside the existing "DST fast budget"), an **extended
>   nightly**, and a structured **`SOAK_REPORT`** (honest status, **no** numeric self-score, R-7).
> - **Why (verified against the code):** `0.44` gave a Jepsen-class simulator (`hydracache-sim`) with
>   fault injection (`FaultSchedule`/`ScheduledFault`), invariant + linearizability checkers, a VOPR
>   binary, and a bounded seed-matrix test (`dst_budget.rs`) — but everything is **short and
>   single-shot**. There is **no** continuous soak, **no** resource-leak-over-time assertion, and
>   **no** sustained-overload proof. The algorithms are validated; **endurance is not.** Pure
>   **develop-downward** (operate-in-prod / soak mileage): no new algorithms, no new core.
> - **After (depends on):** `0.44` (DST/sim/VOPR, `SimWorld`, `FaultSchedule`, `InvariantChecker`,
>   `dst_budget.rs`), `0.46` (admission/capacity + `hydracache_admission_*` metrics), `0.56`
>   (operator + kind harness). Independent of `0.54`/`0.55`/`0.57`.
> - **Blueprint:** TigerBeetle **VOPR** ("the VOPR runs forever"), FoundationDB simulation-soak
>   discipline, `COMPETITIVE_ANALYSIS_AND_EVOLUTION.md` soak thread.
> - **Status:** planned.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> positioning: [`../POSITIONING.md`](../POSITIONING.md) ·
> DST plan: [`V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md`](V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its Definition of
Done **and** `cargo xtask verify`; never push red. Everything deterministic is seeded and replayable
(R-5); nothing here claims a throughput number (R-7).

## Preflight Audit (Codex, 0.58 start — read before touching code)

Verified by reading the code; each is a load-bearing fact this plan is built on:

1. **VOPR is single-shot.** `crates/hydracache-sim/src/bin/vopr.rs`: parses `--seed`/`--steps`
   (default 1000), builds one `SimWorld::new(seed, SimConfig::default())`, `world.run(steps)`, prints
   `history_hash` + `invariant_violations`, exits `0`/`2`. No wall-clock budget, no seed fleet, no
   failing-seed persistence. **W1 wraps this; it does not re-derive the simulator.**
2. **The run surface is right there.** `crates/hydracache-sim/src/world.rs`: `SimWorld::run(steps)
   -> SimOutcome` (world.rs:230), `step()` (world.rs:238, which calls `refresh_invariant_report()`),
   `outcome()` (world.rs:257), `invariant_report() -> &InvariantReport` (world.rs:279). `SimOutcome {
   seed, steps, accepted_ops, delivered_messages, history_hash, invariant_violations }`
   (world.rs:53-66). `SimConfig { node_count, heartbeat_interval, step_duration, key_count }`
   (world.rs:21-30). **The oracle the soak driver checks each seed is `outcome.invariant_violations`.**
3. **A replay + delta-debug harness exists — but does not check real invariants.**
   `crates/hydracache-sim/src/schedule.rs`: `ReplayRunner::run(seed, steps, schedule) ->
   ReplayOutcome { seed, steps, sim, failure }` (schedule.rs:115) sets `failure` **only** for
   `ScheduledFaultKind::SyntheticViolation` (schedule.rs:122-134) — it ignores
   `sim.invariant_violations`. `shrink_failure(seed, steps, schedule)` (schedule.rs:171) and
   `shrink_with(schedule, predicate)` (schedule.rs:147) do generic delta-debugging. **Consequence:**
   W1 must (a) inspect `invariant_violations` itself, and (b) minimize a failing schedule via
   `shrink_with(|s| run(seed, steps, s).sim.invariant_violations > 0)` — a small, honest extension,
   not a rewrite.
4. **Fault primitives are shipped; only duration/continuity is missing.** `FaultSchedule` /
   `ScheduledFault { step, kind }` / `ScheduledFaultKind::{NetworkDrop, NetworkDelay,
   StorageCorruption, Crash, Restart, SyntheticViolation }` (schedule.rs:47-81); `SimStorage` fault
   classes `StorageFault::{LatentReadError, Corruption, TornWrite, Slow, LostOnCrash}`
   (`storage.rs:275-281`).
5. **The invariant model is append-only and composable.** `InvariantReport { checked, violations:
   Vec<InvariantViolation { name, message }> }` (invariants.rs:268-274) with
   `record_violation(name, msg)` / `record_check()` (invariants.rs:287-298) and a composable
   `InvariantChecker` (invariants.rs:308). **W2's `BoundedGrowthChecker` appends to this report — it
   fits the existing shape.**
6. **The backpressure surface is shipped and bounded.** `crates/hydracache/src/admission.rs`:
   `AdmissionController` with `limits.max_in_flight` / `limits.max_memory_bytes`, a FIFO queue,
   `rejected_total`, `reject()` → `AdmissionError::Backpressure { reason, retry_after_ms }`, and
   `snapshot() -> AdmissionSnapshot { in_flight, memory_bytes, queue_depth, rejected_total }`
   (admission.rs:100-150). **W3 drives this under sustained load and asserts boundedness + counted
   rejects — the metrics already exist.**
7. **`SimStorage` has no total-footprint accessor.** `crates/hydracache-sim/src/storage.rs:254`
   `SimStorage { zones: BTreeMap<...> }` exposes `apply_checked`, faults, per-entry `bytes()`
   (storage.rs:87) — but **no** `footprint()/total_bytes()`. **This is a missing part W2 adds.**
8. **A bounded seed-matrix test already exists.** `crates/hydracache-sim/tests/dst_budget.rs`:
   seeds `44..49`, 32 steps, asserts `invariant_violations == 0`. It is already a `verify` gate
   ("DST fast budget", `crates/xtask/src/verify.rs:51-62`, guarded by
   `verify_includes_dst_fast_budget_gate`, verify.rs:221). **W5's bounded soak gate is a sibling
   `Gate` in the same list.**

## Release Theme

Prove the shipped grid **endures**: a continuous multi-seed soak over the `0.44` simulator with
bounded-resource and leak-over-time invariants, a sustained-overload/backpressure hardening proof over
the shipped admission path, a real multi-node chaos soak on the `0.56` kind harness, and a
bounded-CI + extended-nightly gate — **no** new algorithms, **no** throughput claim, **no** new
consistency level.

## Non-Goals

- **Not a benchmark / no throughput claim (R-7).** Soak proves *endurance and boundedness*, not
  ops/sec. No "N million ops" number; the `SOAK_REPORT` records seeds, wall-clock, invariant status,
  and resource-bound verdicts — never a self-score.
- **Not new algorithms / not wider.** No new consensus, consistency level (R-1), or storage engine —
  this is **downward** (operate/soak), hardening what ships.
- **Not on the fast path (R-10).** The soak driver, checker, and samplers are test/tooling; embedded
  caching is byte-for-byte unchanged. Any hardening fix must not regress the healthy fast path.
- **Not a chaos-engineering platform.** We drive the existing `FaultSchedule`/kind harness for a
  window; no general fault-injection product.
- **Not a substitute for real production mileage.** Soak *raises confidence and finds bugs*;
  positioning stays honest that multi-year field history is still accruing (R-11).

## Inherited Boundary (assumes 0.44 + 0.46 + 0.56)

- **From `0.44`:** `SimWorld` (`run`/`step`/`outcome`/`invariant_report`), `SimOutcome`, `SimConfig`;
  VOPR (`bin/vopr.rs`); `FaultSchedule`/`ScheduledFault`/`ReplayRunner`/`shrink_*` (schedule.rs);
  `InvariantChecker`/`InvariantReport` (invariants.rs); `SimStorage` (storage.rs); `dst_budget.rs`.
  **Determinism is the contract** — every failure is a seed + step count (R-5).
- **From `0.46`:** `AdmissionController` + `AdmissionSnapshot` (admission.rs:38/122) — a **standalone**
  backpressure component (no cache-hot-path call site, G1), driven **directly** by W3.
- **From `0.56` + `0.57.1`:** the operator kind infra **and** the `0.57.1` driven-E2E harness
  (`e2e.rs`, TD-0007, skip-gracefully) reused by W4 (G4).

## Gap Analysis (post-audit — holes found and closed)

A second code-grounded pass found four holes at the **simulator-vs-real boundary**. Each is verified
and folded into the work items below:

- **G1 — the simulator does NOT wire the real `AdmissionController`.** `crates/hydracache-sim/src/
  world.rs` and `workload.rs` contain **zero** admission/capacity references (the earlier match was
  the sim model's own `in_flight` field, world.rs:79). And `AdmissionController` (admission.rs:38) has
  **no call sites on the cache write path** (grep finds only the struct + exports at lib.rs:510) — it
  is a **standalone component tested in isolation**, not on the live hot path. **Fix:** W3's overload
  proof is a **focused component test** driving `AdmissionController` **directly** (honest: it proves
  the component's boundedness/recovery, not an end-to-end server overload). The sim does **not** get a
  fake "admission workload"; the sim's leak role is W2 over its **own** tracked resources.
- **G2 — W2's sample sources partly don't exist in the sim.** "in-flight/queue via the admission
  snapshot in the sim" and "tombstone debt (`TombstoneTracker`)" are **not** sim-tracked
  (`TombstoneTracker` is grid-side, grid/mod.rs; there is no admission in the sim). **Fix:** W2 samples
  only what `SimWorld::snapshot() -> SimSnapshot` (world.rs:658) genuinely exposes — network in-flight
  messages (:749), per-client in-flight (:759), subscriber pending/lag (:771, capped by
  `MAX_SUBSCRIBER_BUFFER` :1041) — plus the new `SimStorage::footprint()`. Tombstone-debt boundedness
  moves to the **real/grid** side (or is dropped), not the sim checker.
- **G3 — W1 conflates two minimization mechanisms.** `ReplayRunner::shrink_with` (schedule.rs:147)
  shrinks a **`FaultSchedule`** — it only applies to **schedule-driven** failures. A plain-seed VOPR
  failure (no injected schedule) has nothing to shrink; it minimizes over **step count** (bisect
  `--steps` until the violation disappears). **Fix:** W1 defines **two** paths — schedule-shrink for
  fault-driven runs, step-bisection for plain-seed runs — not one.
- **G4 — W4 predates 0.57.1.** The driven operator kind harness now **exists**
  (`crates/hydracache-operator/tests/e2e.rs`, TD-0007 resolved:
  `full_lifecycle_drives_install_scale_upgrade_rotate_backup_restore`, `e2e_skips_gracefully_without_a_cluster`).
  **Fix:** W4 **reuses that harness/provisioning**, not "the 0.56 kind harness" generically.

## Technical-debt scope (what this release includes vs leaves for later)

`0.58` is **develop-downward endurance**, not a ledger-closing maintenance pass (that was `0.57.1`).
Its relationship to `docs/technical-debt/` is explicit:

| TD | In `0.58`? | Why |
| --- | --- | --- |
| **TD-0008** networked daemon grid | **Partially blocked → stays for `0.59`** | W4's "real multi-node soak" runs against the `0.56`/`0.57.1` operator/kind fixture, whose pods run the **in-process** member grid (`grid_host.rs` W6a), **not** a true multi-daemon raft cluster. The **full** daemon-cluster soak lands only after `0.59` wires the networked grid. W4 is honest-partial until then. |
| **New: `TD-0009` soak/overload findings** | **Created here if needed** | W2/W3 may surface a real leak or an admission-boundedness bug. Fixable in-window → fixed + gated (no TD). Too large → **spawn `TD-0009`** with the reproducing seed, rather than half-fix. |
| **New: real-server RSS/fd sampler portability** | **Tracked if platform-bound** | The `#[ignore]` real-process sampler (W2) is nightly/Linux-first; if Windows/macOS sampling needs work, note it as a small follow-up, don't block the release. |
| TD-0002 raft/protobuf, TD-0003 bucket C, TD-0004 placement, TD-0005 Java artifact | **Out of scope** | Untouched — unrelated to soak/endurance. |

**What stays for later (explicit):** the **true daemon-cluster** endurance soak (needs `0.59`/TD-0008);
**production soak mileage** (`0.58` builds the *harness + evidence*, not "battle-tested" history — that
accrues into `0.60`/`1.0`, R-11); any **large hardening fix** W3 surfaces (→ `TD-0009`).

## Dependency Graph

```
0.44 SimWorld/VOPR/FaultSchedule ─► W1 continuous soak driver ─► W2 footprint + BoundedGrowthChecker (sim's OWN resources) ─┐
0.46 AdmissionController (standalone) ─────────────► W3 component-level overload proof (drive it directly) ─────────────────┼─► W5 CI soak gate + nightly + SOAK_REPORT + docs
0.56 operator/kind + 0.57.1 driven e2e.rs harness ─► W4 real multi-node chaos soak (reuses that harness) ──────────────────┘
```

---

## W1. Continuous, wall-clock-budgeted, multi-seed soak driver

**Goal.** A soak driver that runs a **rolling fleet of seeds** against `SimWorld` until an invariant
violation or a **wall-clock budget** elapses, records the **first failing seed + step** for exact
replay (R-5), **minimizes** the failing schedule via the existing shrink harness, keeps memory
**bounded** (rolling summaries, not per-run traces), and exits **loud** on any violation.

**Files.**
- new `crates/hydracache-sim/src/soak.rs` (driver types),
- new/extended `crates/hydracache-sim/src/bin/vopr.rs` (add a `soak` subcommand) or sibling
  `bin/soak.rs`,
- reuses `SimWorld` (world.rs:230/257), `SimRng` (rng.rs), `ReplayRunner::shrink_with`
  (schedule.rs:147).

**Code sketch (grounded in the real API).**
```rust
// crates/hydracache-sim/src/soak.rs  (new)
use std::time::{Duration, Instant};
use crate::{FaultSchedule, ReplayRunner, SimConfig, SimRng, SimWorld};

#[derive(Debug, Clone)]
pub struct SoakConfig {
    pub master_seed: u64,       // makes the *fleet* reproducible
    pub budget: Duration,       // wall-clock, not a step count
    pub steps_per_seed: u64,
    pub sim: SimConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakFailure { pub seed: u64, pub step: u64, pub violations: Vec<String> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakOutcome {
    pub master_seed: u64,
    pub seeds_run: u64,
    pub total_steps: u64,
    pub first_failure: Option<SoakFailure>,   // None => clean over the whole budget
}

pub fn run_soak(cfg: &SoakConfig) -> SoakOutcome {
    let mut fleet = SimRng::new(cfg.master_seed);   // deterministic seed sequence
    let start = Instant::now();
    let (mut seeds_run, mut total_steps) = (0u64, 0u64);
    loop {
        let seed = fleet.next_u64();
        let mut world = SimWorld::new(seed, cfg.sim.clone());
        let outcome = world.run(cfg.steps_per_seed);   // world.rs:230
        seeds_run += 1;
        total_steps += outcome.steps;
        if outcome.invariant_violations > 0 {          // the oracle (audit item 2)
            let violations = world.invariant_report()  // world.rs:279
                .violations.iter().map(|v| v.to_string()).collect();
            return SoakOutcome {
                master_seed: cfg.master_seed, seeds_run, total_steps,
                first_failure: Some(SoakFailure { seed, step: outcome.steps, violations }),
            };
        }
        if start.elapsed() >= cfg.budget || cfg.budget.is_zero() && seeds_run >= 1 {
            return SoakOutcome { master_seed: cfg.master_seed, seeds_run, total_steps, first_failure: None };
        }
    }
}
```
CLI: extend the existing single-shot binary (`bin/vopr.rs`, args `--seed`/`--steps`, exit `0`/`2`) with
a `soak` subcommand — a thin wrapper over `run_soak`, exit `2` loud on the first failure:
```rust
// crates/hydracache-sim/src/bin/vopr.rs — add `soak --budget-secs S --steps-per-seed K --master-seed M`.
Some("soak") => {
    let out = hydracache_sim::run_soak(&cfg_from_args(args)?);
    println!("{}", serde_json::to_string(&SoakReport::from(&out))?);   // W5 SoakReport
    if out.first_failure.is_some() { return ExitCode::from(2); }        // loud (R-3)
    ExitCode::SUCCESS
}
```

Minimization has **two distinct paths** (G3 — do not conflate):
```rust
// (a) SCHEDULE-DRIVEN failure — shrink the FaultSchedule via the existing delta-debugger.
let minimal_schedule = ReplayRunner::default().shrink_with(schedule, |s| {   // schedule.rs:147
    ReplayRunner::default().run(seed, steps, s.clone()).sim.invariant_violations > 0
});

// (b) PLAIN-SEED failure (no injected schedule) — there is nothing to shrink; bisect STEP COUNT.
fn minimal_failing_steps(seed: u64, cfg: &SimConfig, failing_steps: u64) -> u64 {
    let (mut lo, mut hi) = (1u64, failing_steps);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if SimWorld::new(seed, cfg.clone()).run(mid).invariant_violations > 0 { hi = mid; }
        else { lo = mid + 1; }
    }
    lo   // fewest steps that still reproduces the violation
}
```

**Steps.**
1. `soak.rs` with `SoakConfig`/`SoakOutcome`/`SoakFailure`; loop deriving seeds from a **seeded**
   master RNG so the fleet is reproducible; run each seed via `SimWorld::run`; inspect
   `invariant_violations`.
2. Budget by **wall clock**; report `seeds_run`/`total_steps`; on failure capture `{seed, step,
   violations}` and **stop loud** (exit `2`), printing the exact `--seed`/`--steps` to reproduce.
3. Add minimization on **two** paths (G3): a fault-**schedule** failure → `shrink_with`
   (schedule.rs:147) with an `invariant_violations > 0` predicate (extends the harness that today only
   trips on `SyntheticViolation`); a **plain-seed** failure → **step-count bisection** (fewest
   `--steps` that still reproduces). The report says which mechanism was used.
4. Bound memory: retain only summaries + the single first failure (feeds W2's driver-footprint test).

**Corner-case scenarios (each an explicit test).**
- **Exact replay:** a failing seed reproduces byte-identically via `--seed N --steps K`
  (the core R-5 property).
- **Fleet reproducibility:** same `master_seed` ⇒ identical seed sequence ⇒ identical first failure.
- **Loud stop:** first `invariant_violations > 0` stops the fleet (exit `2`), no silent continue (R-3).
- **Minimization (schedule):** a schedule of N faults that fails shrinks to the minimal failing
  subset and the minimal set still reproduces.
- **Minimization (plain seed):** a seed failing at step K bisects to the fewest steps that still
  reproduce (G3 — step-bisection, not schedule-shrink).
- **Zero-budget:** runs at least one seed.
- **Bounded memory:** a long fleet does not accumulate per-run traces.

**DoD.** `crates/hydracache-sim/tests/soak_driver.rs`
- `soak_fleet_is_reproducible_from_master_seed`.
- `first_failing_seed_reproduces_the_violation_exactly` (R-5).
- `soak_stops_loud_on_first_invariant_violation` (exit-2 semantics, R-3).
- `failing_schedule_shrinks_to_minimal_reproducing_subset` (schedule path).
- `plain_seed_failure_bisects_to_minimal_step_count` (G3, step-bisection path).
- `soak_driver_memory_is_bounded_over_a_long_fleet`.
- Run: `cargo test -p hydracache-sim --locked soak_driver`.

**Risk & rollback.** Load-bearing property: **exact replay + minimization of a failing seed** —
gated. Revert leaves VOPR single-shot.

## W2. Resource accounting (`SimStorage::footprint`) + `BoundedGrowthChecker` + real RSS/fd sampler

**Goal.** Make leaks-over-time detectable. **Add the missing** `SimStorage::footprint()` accessor;
add a **`BoundedGrowthChecker`** that asserts the sim's **own tracked** resources (storage footprint,
network + client in-flight, subscriber pending/lag — the fields `SimWorld::snapshot()` already exposes,
G2) **do not grow unboundedly** over a long run (falsifiable); and a real-server RSS/fd sampler that
fails loud on monotonic, unbounded climb. (Admission-queue and tombstone-debt are **not** sim-tracked —
G1/G2 — so they are **not** sampled here; tombstone-debt boundedness lives on the grid/real side.)

**Files.**
- `crates/hydracache-sim/src/storage.rs` (**add** `pub fn footprint(&self) -> StorageFootprint`
  summing per-zone entry bytes — the missing accessor, audit item 7),
- `crates/hydracache-sim/src/invariants.rs` (**add** `BoundedGrowthChecker` producing
  `InvariantViolation { name: "resource_bounded_growth", .. }` via `record_violation`, audit item 5),
- wired into `SimWorld::refresh_invariant_report` (world.rs:253) so it runs each step,
- real sampler `crates/hydracache-server/tests/soak_resource.rs` (ignored-by-default).

**Code sketch.**
```rust
// storage.rs — the missing footprint accessor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StorageFootprint { pub live_bytes: u64, pub tombstone_bytes: u64, pub entries: u64 }

impl SimStorage {
    pub fn footprint(&self) -> StorageFootprint {
        self.zones.values().fold(StorageFootprint::default(), |mut f, zone| {
            for entry in zone.entries() { f.entries += 1; f.live_bytes += entry.bytes().len() as u64; }
            f
        })
    }
}
```
```rust
// invariants.rs — bounded-growth check appended to the existing report shape.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceSample {
    pub storage_bytes: u64,     // sum of SimStorage::footprint().live_bytes
    pub in_flight: u64,         // network + per-client in-flight
    pub subscriber_pending: u64,// sum of subscriber.pending.len()
}

#[derive(Debug, Clone, Default)]
pub struct BoundedGrowthChecker { budget: ResourceBudget, samples: Vec<ResourceSample> }

impl BoundedGrowthChecker {
    /// Record one sample and flag unbounded monotonic climb past budget.
    pub fn observe(&mut self, sample: ResourceSample, report: &mut InvariantReport) {
        report.record_check();
        self.samples.push(sample);
        if self.exceeds_budget(&sample) && self.is_monotonic_climb() {
            report.record_violation(
                "resource_bounded_growth",
                format!("unbounded growth: {sample:?} over budget {:?}", self.budget),
            );
        }
    }
    // bounded oscillation within budget => OK; a monotonic climb past ceiling => leak.
}
```
```rust
// world.rs — wire the checker into the existing per-step hook. SimWorld already holds
// nodes/clients/subscribers (world.rs:133-135) and calls refresh_invariant_report() each
// step (world.rs:882); sample ONLY those sim-tracked fields (G2).
pub struct SimWorld {
    // …existing fields (invariant_report world.rs:132, nodes :133, clients :134, subscribers :135)…
    growth: BoundedGrowthChecker,   // NEW
}

fn refresh_invariant_report(&mut self) {   // world.rs:882 (existing)
    // …existing invariant checks…
    let sample = ResourceSample {
        storage_bytes: self.nodes.values().map(|n| n.storage.footprint().live_bytes).sum(),
        in_flight: self.network.in_flight_messages().len() as u64
            + self.clients.values().map(|c| u64::from(c.in_flight)).sum::<u64>(),
        subscriber_pending: self.subscribers.values().map(|s| s.pending.len() as u64).sum(),
    };
    self.growth.observe(sample, &mut self.invariant_report);   // appends resource_bounded_growth on leak
}
```

**Steps.**
1. Add `SimStorage::footprint()` (+ `StorageFootprint`); no behaviour change (read-only).
2. Add `BoundedGrowthChecker` + `ResourceBudget`/`ResourceSample`; sample **only sim-tracked**
   resources each step (G2): `SimStorage::footprint()` (new), network in-flight (`SimWorld::snapshot()`
   → SimSnapshot, world.rs:749), per-client in-flight (:759), subscriber pending/lag (:771, cap
   `MAX_SUBSCRIBER_BUFFER` :1041). **Monotonic climb past budget = loud violation** (R-3), reported
   like any invariant with a reproducing seed. (No admission-queue / tombstone-debt sampling here —
   G1/G2.)
3. Distinguish **bounded oscillation** (OK) from **unbounded growth** (leak) — the check is
   **falsifiable**: a deliberately-leaky fixture must fail it.
4. Real process: an `#[ignore]` long test samples RSS + open fds around a sustained
   `hydracache-server` workload; fail loud on a climb past budget with no plateau; skip where the
   platform can't sample (documented; Windows `LNK1104` is a linker lock, not a soak failure).

**Corner-case scenarios.**
- **Leaky fixture caught:** a fixture that never releases bytes trips `resource_bounded_growth`
  (proves the checker is not vacuous).
- **Oscillation passes:** steady rise-and-fall within budget does not trip.
- **One-time step-up plateau:** a new partition raises footprint once then plateaus → not a leak.
- **Subscriber lag bounded:** a slow subscriber's pending queue stays under `MAX_SUBSCRIBER_BUFFER`,
  not unbounded (a sim-tracked resource, world.rs:771/1041).

**DoD.** `crates/hydracache-sim/tests/soak_resource.rs` (sim) +
`crates/hydracache-server/tests/soak_resource.rs` (process, ignored-by-default)
- `bounded_growth_invariant_flags_a_leaky_fixture` (falsifiable).
- `steady_state_oscillation_within_budget_passes`.
- `one_time_stepup_then_plateau_is_not_a_leak`.
- `subscriber_pending_stays_bounded_under_slow_consumer` (sim-tracked, G2).
- `real_server_rss_and_fds_plateau_under_sustained_load` (ignored-by-default; nightly).
- Run: `cargo test -p hydracache-sim --locked soak_resource` (+ nightly `-- --ignored`).

**Risk & rollback.** Leak-vs-oscillation discrimination is the hard part — the falsifiable-fixture
test proves the checker isn't vacuous. Revert removes the checker + accessor; existing invariants stay.

## W3. Sustained-overload / backpressure hardening proof

**Goal.** Prove that holding offered load **above capacity for a long window** yields **fail-loud,
bounded** behaviour on the shipped `AdmissionController`: rejects **counted**, `in_flight` /
`memory_bytes` / `queue_depth` **bounded** by the configured limits (never OOM / unbounded growth),
and **recovery** once load falls. **Honest scope (G1):** `AdmissionController` (admission.rs:38) is a
**standalone component** with **no call site on the cache hot path**, so this is a **component-level**
proof (drive the controller directly), not an end-to-end server overload; any unbounded/silent-drop
path found is **fixed** in the component (the "hardening" half).

**Files.** real test `crates/hydracache/tests/sustained_overload.rs` driving
`crates/hydracache/src/admission.rs` **directly** (`AdmissionController::try_admit`/`snapshot`,
admission.rs:122). A **separate** sim test (`hydracache-sim/tests/overload_sim.rs`) composes a
**high-rate workload + a `FaultSchedule` partition** and asserts the sim's **own** resources stay
bounded (no deadlock, W2 invariants) — it does **not** exercise `AdmissionController` (G1: no admission
in the sim). Any fix lands in `admission.rs`, never the fast path (R-10).

**Code sketch (asserting the shipped snapshot stays bounded).**
```rust
// crates/hydracache/tests/sustained_overload.rs
let mut controller = AdmissionController::new(limits);           // max_in_flight, max_memory_bytes
for _ in 0..SUSTAINED_WINDOW {                                   // hold load above capacity
    let _ = controller.try_admit(ticket("req", big_bytes));      // some Ok, many Backpressure
}
let s = controller.snapshot();                                  // admission.rs:122
assert!(s.in_flight    <= limits.max_in_flight,    "in_flight bounded");
assert!(s.memory_bytes <= limits.max_memory_bytes, "memory bounded (no unbounded queue)");
assert!(s.rejected_total > 0, "rejects are COUNTED, never silently dropped (R-3)");
// recovery: drain in-flight, offer below capacity, assert healthy again
drain(&mut controller);
assert!(controller.try_admit(ticket("ok", small_bytes)).is_ok(), "recovers after overload");
```

**Steps.**
1. Component (real, `hydracache/tests/sustained_overload.rs`): drive `AdmissionController` above
   capacity for a long window; assert `rejected_total` climbs while `in_flight`/`queue_depth`/
   `memory_bytes` stay **bounded** by limits, **no OOM**, and **recovery to healthy** after load drops.
   Fail loud on any silent drop (R-3) or unbounded queue.
2. Sim (separate, `hydracache-sim/tests/overload_sim.rs`, G1): a **high-rate** workload composed with a
   `FaultSchedule` partition; assert the sim's **own** resources stay bounded (W2 checker) and there is
   **no deadlock** — this is a *sim resilience* check, not an admission check.
3. If a real unbounded-growth or silent-drop path is found in the controller, **fix it** in
   `admission.rs` and gate the fix.

**Corner-case scenarios.**
- **At capacity:** load exactly at `max_in_flight` — steady, no thrash.
- **Burst above→below:** rejects during, healthy after (recovery proven).
- **High-rate + partition (sim, G1):** a high-rate workload composed with a `FaultSchedule` partition
  → no deadlock, sim resources bounded (W2) — a *sim resilience* check, not an admission assertion.
- **Slow downstream:** a slow loader/DB does not grow the in-flight set unbounded (ties `0.55`
  poison-load breaker).
- **Oversized single request:** rejected via `ensure_fits` (`AdmissionRejectionReason::MemoryLimit`),
  counted, not queued forever.

**DoD.** `crates/hydracache/tests/sustained_overload.rs` (component) +
`hydracache-sim/tests/overload_sim.rs` (sim resilience, G1)
- `sustained_overload_rejects_are_counted_and_queue_is_bounded` (component).
- `node_recovers_to_healthy_after_overload_subsides` (component).
- `oversized_request_is_rejected_and_counted` (component).
- `slow_downstream_does_not_grow_in_flight_unbounded` (component).
- `high_rate_workload_with_partition_does_not_deadlock` (sim, seeded — resources bounded, no admission).
- Run: `cargo test -p hydracache --locked sustained_overload` +
  `cargo test -p hydracache-sim --locked overload_sim`.

**Risk & rollback.** May surface a real hardening bug — that is the point; fix + gate. The
authority/consistency contract is untouched (R-1); only boundedness under overload is hardened.

## W4. Real multi-node chaos soak (kind, skip-gracefully)

**Goal.** A sustained **multi-node** soak on the `0.56` operator/kind harness: a small cluster under a
rolling chaos schedule (partition / crash / slow-disk) over a wall-clock window, asserting the shipped
invariants — **no lost committed write**, quorum preserved, recovery after each fault — reusing the
operator's kind infrastructure and its **skip-without-a-cluster** pattern.

**Files.** `crates/hydracache-operator/tests/soak_kind.rs` (new, `#[ignore]`/kind-gated), **reusing the
`0.57.1` driven-E2E harness/provisioning** (`crates/hydracache-operator/tests/e2e.rs`, TD-0007 —
`full_lifecycle_drives_…`, `e2e_skips_gracefully_without_a_cluster`) rather than re-deriving a kind
harness (G4); applies the `0.42`/`0.44` invariant oracles against the real cluster; a chaos driver over
operator actions (drain, kill-pod, partition).

**Code sketch (chaos loop over the reused harness, G4).**
```rust
// crates/hydracache-operator/tests/soak_kind.rs — reuse the 0.57.1 e2e harness (TD-0007).
#[tokio::test]
async fn multi_node_chaos_soak_loses_no_committed_write() {
    let Some(kind) = KindHarness::try_start() else { return log_skip(); } // e2e.rs skip-graceful (G4)
    let cr = apply_cluster(&kind, sample_spec()).await;
    kind.wait_ready(&cr, quorum_for(3)).await;                          // scale.rs:297
    let writer = spawn_committed_write_probe(&kind, &cr);               // records committed writes
    for fault in rolling_chaos_schedule(WINDOW) {                       // crash|partition|slow-disk, spaced
        kind.inject(fault).await;
        assert!(kind.has_leader(&cr).await, "leader re-established after {fault:?}");
        kind.heal(fault).await;
        kind.wait_ready(&cr, quorum_for(3)).await;                      // /readyz recovers
    }
    assert_eq!(writer.committed(), kind.durable_committed(&cr).await, "no lost committed write");
}
```

**Steps.**
1. Stand up a small `HydraCacheCluster` via the operator (reusing the `0.57.1` `apply_cluster`/
   `wait_ready` harness, G4); run a steady committed-write probe.
2. Apply a **rolling** chaos schedule over a window (crash a pod, heal; partition, heal; slow a disk,
   heal), spaced so quorum is preserved (drain-before-remove, `0.56` semantics).
3. Continuously assert: no lost committed write, a leader always re-established, `/readyz` recovers
   after each fault; collect a `SOAK_REPORT` (W5). Skip **gracefully** if no cluster (kind/Docker
   absent), exactly like the `0.56` kind rows.

**Corner-case scenarios.**
- **Leader pod crash** → re-election + catch-up, no lost write.
- **Two faults close in time** but never below quorum.
- **Pod fails to return** → halt loud (not silent degradation).
- **Slow-disk node** stays a member but is backpressured (ties W3).
- **No cluster** → the whole suite skips cleanly (no false green, no hard failure).

**DoD.** `crates/hydracache-operator/tests/soak_kind.rs` (kind-gated, nightly)
- `multi_node_chaos_soak_loses_no_committed_write` (kind).
- `leader_is_always_reestablished_after_pod_crash` (kind).
- `soak_skips_gracefully_without_a_cluster` (unit-level guard, always runs).
- Run (nightly/manual): the `0.57.1` kind command in `docs/GATES.md`
  (`HYDRACACHE_OPERATOR_KIND=1 … -- --ignored`, G4); skipped in the fast PR gate.

**Risk & rollback.** Real-cluster soak is nightly/manual, off the fast PR gate (kind is heavy). Revert
leaves the sim soak (W1–W3) as the endurance proof.

## W5. Bounded CI soak gate + extended nightly + `SOAK_REPORT` + docs

**Goal.** A **short, bounded, deterministic** soak in `cargo xtask verify` (a new `Gate` beside "DST
fast budget"), an **extended nightly** soak, a structured **`SOAK_REPORT`** (seeds, wall-clock,
invariant status, resource verdicts — **no** numeric self-score, R-7), and the endurance docs/runbook.

**Files.** `crates/xtask/src/verify.rs` (**add** a `Gate` — mirror the existing "DST fast budget"
gate at verify.rs:51-62 and its guard test at verify.rs:221), a new bounded test
`crates/hydracache-sim/tests/soak_budget.rs` (generalize `dst_budget.rs`), a nightly CI workflow, and
`docs/soak.md` (runbook: run it, triage a failing seed, read the report).

**Code sketch (the new bounded gate + its guard, matching the existing pattern).**
```rust
// verify.rs — add beside the "DST fast budget" gate.
gate("soak fast budget",
     ["test", "-p", "hydracache-sim", "--test", "soak_budget", "--locked"],
     None),
```
```rust
// crates/hydracache-sim/tests/soak_budget.rs — deterministic, seconds-long, PR-safe.
use hydracache_sim::{run_soak, SoakConfig, SimConfig};
use std::time::Duration;

#[test]
fn bounded_ci_soak_is_deterministic_and_fast() {
    let cfg = SoakConfig {
        master_seed: 0x5040,           // fixed => deterministic fleet
        budget: Duration::from_millis(500),
        steps_per_seed: 64,
        sim: SimConfig::default(),
    };
    let a = run_soak(&cfg);
    let b = run_soak(&cfg);
    assert_eq!(a, b, "fixed master seed => identical outcome (no flake)");
    assert!(a.first_failure.is_none(), "clean seeds must stay clean: {a:?}");
}
```
```rust
// verify.rs test — mirror verify_includes_dst_fast_budget_gate (verify.rs:221).
#[test]
fn verify_includes_soak_fast_budget_gate() {
    assert!(gates_for_platform(false).iter().any(|g| g.label == "soak fast budget"));
}
```

**Steps.**
1. Bounded CI: add the `soak_budget.rs` test (fixed master seed, ~sub-second budget) and the new
   `Gate`; add `verify_includes_soak_fast_budget_gate`. Deterministic ⇒ never flakes the PR gate.
2. Extended nightly: a long wall-clock soak (W1) + the ignored resource/overload/kind tests
   (`-- --ignored`); publishes the `SOAK_REPORT`.
3. `SOAK_REPORT`: a structured, **score-free** (R-7) summary emitted by the driver:
   ```rust
   // crates/hydracache-sim/src/soak.rs — serialized to JSON for the nightly artifact.
   #[derive(Debug, Clone, Serialize)]
   pub struct SoakReport {
       pub master_seed: u64,
       pub seeds_run: u64,
       pub total_steps: u64,
       pub wall_clock_secs: u64,
       pub resource_bounds_ok: bool,          // W2 BoundedGrowthChecker verdict
       pub outcome: SoakReportOutcome,        // Clean | Failed { .. }
   }
   #[derive(Debug, Clone, Serialize)]
   #[serde(rename_all = "snake_case", tag = "status")]
   pub enum SoakReportOutcome {
       Clean,                                  // no violation over the whole budget
       Failed {
           seed: u64,
           reproduce: String,                  // e.g. "vopr --seed N --steps K"
           minimization: Minimization,         // Schedule { faults } | Steps { minimal }  (G3)
           violations: Vec<String>,
       },
   }
   ```
   No health percentage, no numeric self-score (R-7); a failure carries the **exact reproducing seed**
   and which **minimization mechanism** applied (schedule-shrink vs step-bisect, G3).
4. Docs: `docs/soak.md` runbook + a `POSITIONING.md` honesty note that soak is accruing evidence, not
   a "battle-tested" claim (R-11).

**Corner-case scenarios.**
- **No Node / no Docker:** the PR gate stays green — nightly-only and kind tests are excluded/skipped
  (the `verify` header already excludes chaos/soak/Docker: verify.rs:1-5).
- **Determinism:** two runs of the bounded soak with the same master seed are byte-identical (the
  no-flake guard).
- **Nightly failure:** carries the exact reproducing seed + minimal schedule.
- **No fabricated number:** the report has no health % (R-7).

**DoD.**
- `crates/hydracache-sim/tests/soak_budget.rs`: `bounded_ci_soak_is_deterministic_and_fast`.
- `crates/xtask/src/verify.rs`: `verify_includes_soak_fast_budget_gate` (mirrors verify.rs:221).
- `docs/soak.md` present; `SOAK_REPORT` shape documented; nightly config committed.
- Run: `cargo xtask verify`.

**Risk & rollback.** The bounded CI soak must not become flaky — determinism (fixed master seed +
small budget) is the safeguard. Revert removes the gate; the soak driver stays runnable manually.

## Test coverage matrix (every new artifact has a named test)

| New code | Source file | Covering test(s) | Tier |
| --- | --- | --- | --- |
| `run_soak`, `SoakConfig/Outcome/Failure` | `hydracache-sim/src/soak.rs` (new) | `soak_fleet_is_reproducible_from_master_seed`, `soak_stops_loud_on_first_invariant_violation`, `soak_driver_memory_is_bounded_over_a_long_fleet` | PR (bounded) + nightly |
| exact-replay + minimization (both paths, G3) | `soak.rs` (`minimal_failing_steps`) | `first_failing_seed_reproduces_the_violation_exactly`, `failing_schedule_shrinks_to_minimal_reproducing_subset`, `plain_seed_failure_bisects_to_minimal_step_count` | PR |
| `soak` subcommand | `hydracache-sim/src/bin/vopr.rs` | `vopr_soak_subcommand_exits_2_on_failure` (tests/cli.rs) | PR |
| `SoakReport`/`SoakReportOutcome` | `soak.rs` | `soak_report_serializes_without_a_self_score` (R-7) | PR |
| `SimStorage::footprint()` + `StorageFootprint` | `hydracache-sim/src/storage.rs` | `footprint_sums_live_and_tombstone_bytes` | PR |
| `BoundedGrowthChecker`/`ResourceSample`/`ResourceBudget` + world wiring | `hydracache-sim/src/invariants.rs`, `world.rs:882` | `bounded_growth_invariant_flags_a_leaky_fixture` (falsifiable), `steady_state_oscillation_within_budget_passes`, `one_time_stepup_then_plateau_is_not_a_leak`, `subscriber_pending_stays_bounded_under_slow_consumer` | PR |
| overload component (drive `AdmissionController`, G1) | `hydracache/tests/sustained_overload.rs` (new) | `sustained_overload_rejects_are_counted_and_queue_is_bounded`, `node_recovers_to_healthy_after_overload_subsides`, `oversized_request_is_rejected_and_counted`, `slow_downstream_does_not_grow_in_flight_unbounded` | PR |
| sim overload resilience (no admission, G1) | `hydracache-sim/tests/overload_sim.rs` (new) | `high_rate_workload_with_partition_does_not_deadlock` | PR |
| real-server RSS/fd sampler | `hydracache-server/tests/soak_resource.rs` (new) | `real_server_rss_and_fds_plateau_under_sustained_load` (`#[ignore]`) | nightly |
| bounded CI soak gate + guard | `xtask/src/verify.rs`, `hydracache-sim/tests/soak_budget.rs` (new) | `bounded_ci_soak_is_deterministic_and_fast`, `verify_includes_soak_fast_budget_gate` | PR |
| multi-node chaos soak (reuses 0.57.1 harness, G4) | `hydracache-operator/tests/soak_kind.rs` (new) | `multi_node_chaos_soak_loses_no_committed_write`, `leader_is_always_reestablished_after_pod_crash`, `soak_skips_gracefully_without_a_cluster` | kind / nightly |

**Coverage rule (DoD):** no new public type or file lands without a row here; PR-tier tests are
deterministic and run under `cargo xtask verify`; nightly/kind rows are `#[ignore]`/env-gated and
**skip-graceful** so the fast gate stays green without Node/Docker/kind.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green (fmt, clippy, tests, doc-check, COMPAT, deny), including a **bounded,
  deterministic** "soak fast budget" gate beside "DST fast budget" (W5).
- A failing soak seed **reproduces the violation exactly** and **minimizes** — schedule-shrink for
  fault-driven runs, **step-bisection** for plain-seed runs (R-5, W1, G3); the fleet is reproducible
  from one master seed.
- Soak **stops loud** on the first invariant violation (R-3) — never a silent continue (W1).
- **Bounded-growth / leak-over-time** invariants are **falsifiable** (a leaky fixture is caught) over
  the sim's **own** tracked resources (footprint + in-flight + subscriber lag, G1/G2); admission-queue
  and tombstone-debt are **not** sampled in the sim; `SimStorage::footprint()` added (W2).
- Sustained overload is **fail-loud + bounded + recovers-after** on the **`AdmissionController`
  component driven directly** (G1): rejects counted, `in_flight`/`memory_bytes`/`queue_depth` bounded,
  no OOM, healthy after (W3); any fix stays off the fast path (R-10).
- Real multi-node chaos soak loses **no committed write** and always re-establishes a leader, reusing
  the `0.57.1` driven-E2E harness (G4), and **skips gracefully without a cluster** (W4).
- No throughput/ops number and **no numeric self-score** anywhere (R-7); the `SOAK_REPORT` states
  honest status only; positioning stays honest that field mileage is still accruing (R-11).
- No new algorithm / consistency level (R-1); embedded fast path byte-for-byte unchanged (R-10).
- `releases.toml` + `INDEX.md` updated; `docs/soak.md` added.
