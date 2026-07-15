# HydraCache 0.67.0 Performance Characterization & Capacity Baselines - Codex Execution Plan

> **At a glance**
> - **What:** a **measurement-only** release that answers, with recorded evidence, the question no
>   current test answers: **how much load does the local cache, a bare daemon, and the cluster
>   actually hold** at a defined SLO. It builds an open-loop, coordinated-omission-safe load
>   foundation, then characterizes all three tiers (saturation knee = max sustainable rate at
>   p99 <= SLO, scaling curves, consistency-level cost, operational brownout profiles, overload
>   goodput/recovery), freezes the numbers as **macro perf budgets** with hardware profiles, and
>   wires everything into the `0.64` governance/receipt machinery.
> - **Why:** today the repo has criterion micro-benches + a `bench-budget` gate (`0.37`) and
>   endurance/overload *correctness* proof (`0.58` - "does not fall over, recovers"), but **zero**
>   capacity characterization: no open-loop load generator, no coordinated-omission handling
>   (0 mentions), no YCSB/memtier-style macro run, no saturation knee, no scaling-efficiency or
>   consistency-cost numbers, no brownout cost of failover/rebalance. "How much does it hold?" is
>   currently unanswerable; any p99 someone quotes is methodologically suspect.
> - **After (depends on):** `0.66.0` (real-process/operational tier and its DaemonCluster/kind
>   lanes are the substrate the cluster-tier measurements run on).
> - **Unblocks:** honest capacity/sizing guidance for adopters, a defensible comparative statement
>   versus Redis on the RESP surface, and regression protection so `1.0` performance cannot silently
>   erode.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md) -
> governance: `0.64` W33 (release evidence, receipts, registries) - prior perf surface:
> `0.37` bench-budget, `0.58` soak/overload, `0.64` W37 resource budgets.

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md)
first. This is a **characterization** release: it measures, freezes, and gates - it does **not**
optimize. A product change is allowed only when a measurement exposes a real defect (an unbounded
degradation, a leak, a pathological curve); such a fix is narrow, has its own commit and regression
measurement. Per `R-7`, no self-scored marketing claims: every number ships with its hardware
profile, methodology, and artifact, and comparative statements are measured same-box runs, never
extrapolations.

## Measurement Discipline (applies to every W-item)

1. **Open loop, not closed loop.** A closed-loop client that waits for each response before sending
   the next *stops offering load exactly when the server stalls*, silently hiding tail latency
   (coordinated omission - Gil Tene; the `wrk2` correction). All capacity numbers come from a
   fixed-rate open-loop generator; latency is measured from the **scheduled** send time, recorded in
   an HdrHistogram-style structure.
2. **Capacity = throughput-at-SLO, not peak.** The reported number is the maximum sustained rate at
   which p99 (and p999 where stated) stays under the declared SLO across a stated window - found by
   stepped rate escalation until the knee. Peak burst numbers may be recorded but are never the
   headline.
3. **Hardware profile or it did not happen.** Every artifact records CPU model, core count, RAM,
   OS, toolchain, build flags, and whether the run shared hardware. Budgets are keyed by profile;
   CI-profile budgets protect regressions, not absolute claims.
4. **Warm-up, steady window, repeats.** Discard warm-up, measure a fixed steady window, run >= 3
   repeats, report median-of-runs with min/max spread; a spread above a stated tolerance marks the
   run unstable rather than averaging it away.
5. **Falsifiability.** Each measurement suite has a canary proving the instrument discriminates: a
   deliberately degraded build/fixture must fail its budget, and a synthetic stall must appear in the
   corrected histogram (and would be hidden by a naive closed-loop measure).
6. **Governance.** Every suite registers in the `0.64` machinery: gated-test registry entries with
   tier/timeout/owner, canary-registry pairs, receipts into `docs/testing/release-evidence/0.67.toml`,
   `PERF_REPORT` artifacts uploaded from CI, quarantine rules apply unchanged.

## Non-Goals

- **No optimization work.** Findings become named backlog items or narrow fixes with their own
  regression measurement; this release does not chase numbers.
- **No marketing benchmark claims (`R-7`).** Artifacts and budgets, not prose superiority claims;
  the comparative row (W8) is a same-box measured statement with methodology attached.
- **No new product surface, consistency level, or protocol change.** A tiny test-only seam (e.g., a
  latency-injection failpoint for canaries) must stay behind `test-failpoints`/dev-deps.
- **No cloud/multi-region benchmarking.** Single-box and single-LAN kind/loopback tiers only; WAN
  characterization is future work once `0.45` geo paths need sizing.
- **No load-generator product.** `hydracache-loadgen` is a dev tool crate (`publish = false`).

## Preflight

Re-grep before implementing; do not trust this plan's anchors blindly:

- existing micro benches: `crates/hydracache/benches/`, `crates/hydracache-db/benches/`, xtask
  `bench-budget` (`crates/xtask/src/` - budget file + criterion baseline comparison).
- endurance/overload: `0.58` soak driver (`hydracache-sim/src/bin/vopr.rs` extensions), admission/
  backpressure tests, RSS/fd sampler; `0.64` W37 `daemon_resource_budget.rs` artifact schema.
- harnesses to reuse: `DaemonCluster` (real processes), `0.61`/`0.66` kind lanes, RESP listener +
  `redis-benchmark` compatibility (`hydracache-redis-compat`), client-protocol loadable surface
  (`ClientRequestEnvelope`), metrics endpoint (`0.57` exporter) for W9 cross-checks.
- governance seams: `release-evidence`, `gated-test-registry.toml`, canary-registry, fast-suite
  budgets, quarantine (`crates/xtask/src/`).
- reference blueprints in the workspace root: `redis/src/redis-benchmark.c` (RESP loadgen),
  `caffeine/caffeine/src/jmh` (concurrent cache micro/meso benches), `tigerbeetle` benchmark +
  budget philosophy, YCSB workload taxonomy (A-F mixes), `wrk2`/HdrHistogram methodology,
  `hikaricp` pool-behavior measurement style.

Audit question:

```text
For each tier (embedded cache, single daemon over the wire, multi-daemon cluster), does a recorded
artifact state the maximum sustained rate at p99 <= SLO on a named hardware profile, with an
open-loop coordinated-omission-safe measurement, a discriminating canary, and a regression budget?
```

Anywhere the answer is "no" is the gap this release closes.

## Implementation Map For Audits

Populate as W-items land (same discipline as `0.64`): item -> where implemented -> required command
-> boundary/gate.

| Item | Implemented where | Required command | Boundary |
| --- | --- | --- | --- |
| _(populate during implementation; W0-W10 below define the targets)_ | | | |

## W0. Open-Loop Load Foundation (blueprint: `wrk2`/HdrHistogram, Gil Tene; principle: fixed-rate open loop + latency from scheduled send time)

**Goal.** One reusable measurement core every other W-item consumes: a fixed-rate open-loop driver,
an HdrHistogram-style recorder (bounded memory, configurable precision), stepped-rate knee search,
warm-up/steady-window/repeat orchestration, and a canonical `PERF_REPORT` JSON schema (rate offered,
rate achieved, p50/p90/p99/p999, error/timeout counts, hardware profile, seed, git commit).

**Files to change.** New dev crate `crates/hydracache-loadgen` (`publish = false`): `rate.rs`
(open-loop scheduler; a missed tick is *recorded as latency*, never skipped), `histogram.rs`,
`knee.rs` (stepped escalation with SLO predicate), `report.rs` (schema + writer), plus a pluggable
`Target` trait (in-process cache / native protocol / RESP socket).

**Required tests (fast, deterministic - the instrument itself is under test):**
- `open_loop_scheduler_accounts_missed_ticks_as_latency_not_skips`;
- `histogram_percentiles_match_reference_values_on_known_distributions`;
- `knee_search_finds_the_stated_knee_on_a_synthetic_latency_model`;
- `perf_report_schema_records_profile_commit_seed_and_spread`.

**Canary.** `canary_closed_loop_measurement_hides_a_synthetic_stall` - drive a target with an
injected 1s stall through (a) the open-loop recorder and (b) a naive closed-loop recorder; the stall
must dominate p99 in (a) and be invisible in (b). Proves the whole release's methodology
discriminates; it is the load-bearing canary.

**DoD.**
```powershell
cargo test -p hydracache-loadgen --locked -j 2
```
**CI.** Fast `rust` job; the loadgen crate never enters a release dependency graph
(`verify-no-test-features` discipline).

## W1. Local Cache Tier Characterization (blueprint: Caffeine `caffeine/src/jmh` GetPutBenchmark; principle: scaling curve + contention worst case, not single-thread averages)

**Goal.** Answer "how much does the **embedded** cache hold": multi-thread scaling curve, hot-key
contention floor, eviction-pressure cost, hit/miss/loader path cost, allocation profile.

**Files to change.** New `crates/hydracache-loadgen/benches/local_cache_tier.rs` (or a bin target)
using the in-process `Target`; small committed workload definitions (thread counts x read/write
mixes 100/0, 95/5, 50/50 x uniform/zipfian keys).

**Required measurements:**
- `local_cache_scaling_curve_1_to_n_threads` (knee per mix; report scaling efficiency vs 1 thread);
- `hot_key_contention_throughput_floor` (all threads on one key - single-flight/lock worst case);
- `throughput_at_full_capacity_vs_half_capacity` (every insert evicts, zipfian and uniform);
- `hit_miss_and_loader_path_cost_breakdown`;
- `bytes_allocated_per_operation_by_feature` (baseline vs TTL vs tags, counting allocator).

**Canary.** `canary_injected_slow_eviction_breaches_the_local_budget` - a test-only latency
failpoint in the eviction path must push the knee below budget.

**DoD.**
```powershell
cargo run -p hydracache-loadgen --release -- tier local --profile ci --report target/perf/local.json
```
**CI.** Nightly perf lane (release build, pinned profile); a short smoke variant compiles+runs in
the fast job with trivially low rates (schema/plumbing check, not a measurement).

## W2. Bare Node Tier - Native Protocol (blueprint: YCSB workload taxonomy; principle: the wire, the connection count, and the payload size are all axes)

**Goal.** Characterize one daemon over the **native client protocol**: knee at SLO, connection-count
scaling, payload-size sweep, and the native-vs-RESP overhead comparison base.

**Files to change.** `hydracache-loadgen` `Target` impl for the client protocol (reuse
`ClientRequestEnvelope` framing); scenario definitions (YCSB-style mixes A(50/50), B(95/5),
C(read-only) mapped to get/put/mget/batch).

**Required measurements:**
- `single_node_native_knee_at_slo_for_ycsb_a_b_c`;
- `connection_scaling_curve_1_10_100_1000` (where accept/per-conn overhead bends the curve);
- `payload_size_sweep_100b_1kb_64kb_1mb` (to the documented value limits, R-3 loud beyond);
- `admission_rejection_overhead_when_saturated` (cost of rejecting vs serving).

**Canary.** `canary_injected_dispatch_delay_breaches_the_native_node_budget`.

**DoD.**
```powershell
cargo run -p hydracache-loadgen --release -- tier node-native --profile ci --report target/perf/node_native.json
```
**CI.** Nightly perf lane against a `DaemonCluster` single daemon; fast-job smoke variant.

## W3. Bare Node Tier - RESP Surface Via Real Redis Tooling (blueprint: `redis/src/redis-benchmark.c`, memtier; principle: measure the compatibility surface with the ecosystem's own instruments)

**Goal.** The RESP facade means the ecosystem's standard load tools apply **unmodified**: use
`redis-benchmark` (and optionally `memtier_benchmark`) against the HydraCache RESP listener for
GET/SET/MSET, pipeline-depth and connection sweeps. This both characterizes the RESP edge and
validates that third-party tooling runs against the facade at load (an interop proof `0.63` never
claimed).

**Files to change.** `crates/hydracache-loadgen/src/resp_external.rs` - a wrapper that launches the
daemon + `redis-benchmark` (Docker or local binary, gated), parses its CSV output into the
`PERF_REPORT` schema, and records the pipeline/connection matrix. Registry rows for the Docker gate.

**Required measurements:**
- `resp_get_set_knee_at_slo_via_redis_benchmark`;
- `pipeline_depth_sweep_1_10_100`;
- `resp_vs_native_overhead_ratio_same_box_same_ops` (joins W2 output);
- note: `redis-benchmark` is closed-loop per connection; record its numbers as *throughput evidence*
  and rely on the W0 open-loop native driver for tail-latency claims - state this in the artifact so
  the two methodologies are never conflated.

**Canary.** `canary_resp_listener_slowdown_breaches_the_resp_budget` (reuse the RESP-side latency
failpoint).

**DoD.**
```powershell
$env:HYDRACACHE_RUN_PERF_RESP='1'
cargo run -p hydracache-loadgen --release -- tier node-resp --profile ci --report target/perf/node_resp.json
Remove-Item Env:\HYDRACACHE_RUN_PERF_RESP -ErrorAction SilentlyContinue
```
**CI.** Docker-gated nightly perf lane, skip-loud without Docker/redis-benchmark.

## W4. Cluster Tier - Scaling, Consistency Cost, Replication Amplification (blueprint: YCSB-on-cluster practice of TiKV/Scylla; principle: efficiency curves and per-guarantee cost, not one aggregate number)

**Goal.** Answer "how much does the **cluster** hold and what does each guarantee cost": ops/s knee
at 3/5/7 daemons, scaling efficiency, consistency-level deltas, replication write amplification.

**Files to change.** `hydracache-loadgen` cluster runner over `DaemonCluster` (loopback tier;
kind-lane variant reuses `0.66` wiring); network byte accounting via existing transport metrics.

**Required measurements:**
- `cluster_knee_at_slo_for_3_5_7_nodes_ycsb_a_b_c` + derived `scaling_efficiency_report` (USL-style
  fit recorded, no extrapolated claims beyond measured points);
- `consistency_level_cost_matrix` (per-level read/write knee + p99 delta, including session/RYOW);
- `replication_write_amplification_bytes_per_committed_byte`;
- `invalidation_fanout_throughput_cost` (heavy tag/explicit invalidation share in the mix).

**Canary.** `canary_quorum_path_delay_breaches_the_cluster_budget`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_PERF_CLUSTER='1'
cargo run -p hydracache-loadgen --release -- tier cluster --nodes 3 --profile ci --report target/perf/cluster_3.json
Remove-Item Env:\HYDRACACHE_RUN_PERF_CLUSTER -ErrorAction SilentlyContinue
```
**CI.** Nightly perf lane (loopback 3-node always; 5/7-node and kind-lane scheduled/manual).

## W5. Operational Brownout Profiles (blueprint: extends `0.62`/`0.66` correctness events with cost; principle: an operational event has a depth and a recovery time, both measurable)

**Goal.** `0.62`/`0.66` prove failover/rebalance/membership change are *correct*; nothing measures
their *cost*. Under a fixed sub-knee load, trigger each event and record the brownout: goodput dip
depth, p99 spike, and time-to-recover to steady state.

**Required measurements (each: depth + recovery time + no lost committed write cross-check):**
- `brownout_profile_leader_failover`;
- `brownout_profile_membership_add_and_drain`;
- `brownout_profile_rebalance_reshard`;
- `brownout_profile_node_kill_and_rejoin`.

**Files to change.** `hydracache-loadgen` event orchestration hooks over `DaemonCluster` admin
surface (kill/drain/scale already exist).

**Canary.** `canary_extended_leader_downtime_breaches_the_brownout_budget` (a failpoint that delays
re-election must blow the recovery-time budget).

**DoD.**
```powershell
$env:HYDRACACHE_RUN_PERF_CLUSTER='1'
cargo run -p hydracache-loadgen --release -- brownout leader-failover --profile ci --report target/perf/brownout_failover.json
Remove-Item Env:\HYDRACACHE_RUN_PERF_CLUSTER -ErrorAction SilentlyContinue
```
**CI.** Nightly perf lane.

## W6. Overload Goodput And Recovery Curves (blueprint: `0.58` admission proof + Netflix/SRE brownout practice; principle: the shape of degradation matters, not only its boundedness)

**Goal.** `0.58` proves overload is bounded and recovers; W6 measures the **curve**: goodput and
p99 at 1.2x/1.5x/2x the knee, rejection ratio, and time-to-baseline after the burst ends - per tier
(node-native, node-resp, cluster).

**Required measurements:**
- `overload_goodput_curve_1_2x_1_5x_2x_knee`;
- `rejection_ratio_and_latency_of_rejects_under_overload`;
- `recovery_time_to_baseline_after_burst`.

**Canary.** `canary_admission_disabled_fixture_shows_goodput_collapse` - with the admission gate
bypassed (test-only), goodput at 2x must collapse instead of plateauing, proving the curve actually
reflects the admission mechanism.

**DoD.**
```powershell
cargo run -p hydracache-loadgen --release -- overload node-native --profile ci --report target/perf/overload_native.json
```
**CI.** Nightly perf lane.

## W7. Macro Perf Budgets And Regression Gate (blueprint: `0.37` bench-budget extended; TigerBeetle budget discipline; principle: a measured floor, keyed by hardware profile, that CI defends)

**Goal.** Freeze the W1-W6 results as **macro budgets** (ops/s floors at SLO, p99 ceilings, brownout
depth/recovery ceilings, overload goodput floors) in a reviewed budget file keyed by hardware
profile (`ci`, `dev-reference`), and extend the xtask gate so nightly perf runs fail on unreviewed
regression - same shape as the criterion `bench-budget` but at the macro tier.

**Files to change.** `docs/testing/perf-budgets/0.67/{ci,dev-reference}.toml`; xtask
`perf-budget-check` (compare `PERF_REPORT` artifacts against the budget with stated tolerance and
spread rules); wire receipts into `release-evidence/0.67.toml`.

**Required tests/gates:**
- `perf_budget_check_fails_on_floor_breach_and_on_unstable_spread`;
- `perf_budget_change_requires_reviewed_budget_file_edit` (no auto-rebaseline - the `0.64` W32
  no-silent-regeneration discipline applied to budgets);
- `every_tier_has_a_budget_row_for_the_ci_profile`.

**Canary.** `canary_perf_budget_accepts_a_silent_rebaseline` must fail.

**DoD.**
```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- perf-budget-check --release 0.67 --profile ci
```
**CI.** Fast job validates schema/coverage; nightly perf lane runs measurements then the check.

## W8. Same-Box Comparative Baseline Versus Redis (blueprint: `redis-benchmark` both ways; principle: comparative honesty - one box, one method, both systems, methodology attached)

**Goal.** One reviewed artifact: same box, same tool (`redis-benchmark`), same ops (GET/SET,
pipeline 1 and 10) against real Redis (pinned image from `0.63` oracle set) and HydraCache RESP.
Output is a measured ratio with methodology, stored as evidence - **not** a marketing claim (`R-7`).
This bounds the honest "cost of the facade + engine" statement for POSITIONING.

**Required measurements:**
- `same_box_redis_vs_hydracache_resp_get_set_ratio` (pipeline 1/10, recorded spread);
- artifact includes both raw outputs, versions, and the divergence note (single-endpoint scope).

**Canary.** none beyond W3's (same instrument); the row is gated and informational-but-recorded.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_PERF_RESP='1'
cargo run -p hydracache-loadgen --release -- compare redis --profile ci --report target/perf/compare_redis.json
Remove-Item Env:\HYDRACACHE_RUN_PERF_RESP -ErrorAction SilentlyContinue
```
**CI.** Docker-gated nightly perf lane.

## W9. Metrics Honesty Cross-Check (blueprint: `0.57` exporter + R-11; principle: the server's own numbers must match an independent observer)

**Goal.** During W2/W4 runs, cross-check server-reported metrics (ops counters, latency summaries,
admission rejects) against the loadgen's independent measurements within stated tolerance - so the
operator-facing numbers used for capacity decisions are proven, not assumed.

**Required tests:**
- `server_reported_ops_and_rejects_match_loadgen_within_tolerance`;
- `server_latency_summary_is_not_understated_versus_open_loop_observer` (the dangerous direction).

**Canary.** `canary_metrics_undercount_fixture_is_detected` (a fixture dropping every Nth counter
increment must fail the cross-check).

**DoD.**
```powershell
cargo test -p hydracache-loadgen metrics_cross_check --locked -j 2
```
**CI.** Runs inside the nightly perf lane (same processes, no extra cost).

## W10. Governance, CI Lanes, And Docs

**Goal.** Wire everything into the `0.64` machinery and both execution environments.

**Design.**
- `docs/testing/release-evidence/0.67.toml` work items for W0-W9; receipts required for ship;
  `release-evidence --release 0.67 --require-ship` is the ship gate.
- Registry rows (gated-test registry) for every env/Docker-gated perf lane with tier/timeout/owner;
  canary-registry pairs for every canary above; quarantine rules unchanged.
- CI: fast `rust` job runs loadgen unit tests + smoke plumbing + budget schema checks; a new
  scheduled/`workflow_dispatch` **`Performance Nightly`** job runs tiers W1-W6, W8, W9 on the pinned
  `ci` profile, uploads all `PERF_REPORT` artifacts, then runs `perf-budget-check`. Kind-lane cluster
  variant is manual. Local runbook (PowerShell + bash) in `docs/TESTING.md` per tier.
- Docs: `docs/PERFORMANCE.md` - methodology (open loop, knee, profiles), how to read `PERF_REPORT`,
  and the standing rule that numbers without profile+methodology are not quotable; reconcile
  `GATES.md`, `TESTING.md`, `releases.toml`, `INDEX.md`, plan header, `docs/releases/0.67.0.md`.

**DoD.**
```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- release-governance-check --release 0.67
cargo run --manifest-path crates\xtask\Cargo.toml -- release-evidence --release 0.67
cargo run --manifest-path crates\xtask\Cargo.toml -- doc-check
```

## Gates (Definition of Done for the release)

- The W0 instrument is itself proven: missed ticks count as latency, percentiles match reference
  distributions, and the closed-loop canary demonstrably hides a stall the open-loop recorder
  reports.
- Each tier has a recorded knee at the declared SLO on the `ci` profile: local cache (scaling,
  hot-key, eviction pressure, allocation), bare node native (YCSB mixes, connections, payloads),
  bare node RESP via real `redis-benchmark`, cluster 3/5/7 with consistency-cost and write
  amplification. Every artifact carries hardware profile, commit, seed, spread.
- Brownout profiles exist for leader failover, membership add/drain, rebalance, and kill/rejoin -
  each with depth, recovery time, and a no-lost-committed-write cross-check.
- Overload curves at 1.2x/1.5x/2x knee are recorded per tier; the admission-bypass canary shows
  collapse, proving the curve reflects the mechanism.
- Macro budgets exist for the `ci` profile, `perf-budget-check` fails on floor breach, unstable
  spread, or silent rebaseline; all its canaries are proven red.
- The same-box Redis comparative artifact exists with methodology and versions; no prose claim
  exceeds the artifact (`R-7`).
- Server metrics match the independent observer within tolerance; the undercount canary is caught.
- All suites registered (gated-test registry, canary registry, evidence manifest); a green
  `release-evidence --release 0.67 --require-ship` on the candidate commit is the ship gate; every
  lane runs locally and in GitHub CI with skip-loud discipline.
- No optimization work, no product surface change, loadgen never in a release graph; any fix driven
  by a finding is narrow, separate, and regression-measured.

## Final Release Decision

Ship `0.67.0` only when the three questions are answered by artifacts, not estimates: the embedded
cache, the bare daemon (native and RESP), and the cluster each have a recorded saturation knee at a
declared SLO on a named hardware profile, measured open-loop with coordinated-omission-safe
methodology whose discriminating canaries are proven red; the operational events have measured
brownout depth and recovery time; overload has a measured goodput curve tied to the admission
mechanism; the numbers are frozen as reviewed macro budgets that CI defends against silent
regression; and the server's own metrics are proven against an independent observer. The release
adds measurement and evidence only - the fastest thing it is allowed to make faster is the feedback
loop that tells the truth about performance.
