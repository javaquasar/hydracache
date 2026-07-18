# HydraCache 0.67.0 Performance Characterization & Capacity Baselines - Codex Execution Plan

> **At a glance**
> - **What:** a **measurement-only** release that characterizes only surfaces that exist at the
>   `0.66.0` boundary: the embedded local cache; the `/client/v1/*` Axum client surface in-process
>   (**not** a daemon wire); the real single-daemon RESP listener whose value/lock state is
>   node-local; the real daemon control-plane/admin wire; and selected library/model grid
>   primitives. Every artifact names its execution mode, state scope, and network boundary. There
>   is deliberately no native-daemon or distributed-value-plane capacity claim.
> - **Why:** today the repo has criterion micro-benches + a `bench-budget` gate (`0.37`) and
>   endurance/overload *correctness* proof (`0.58` - "does not fall over, recovers"), but **zero**
>   capacity characterization: no open-loop load generator, no coordinated-omission handling
>   (0 mentions), no surface-scoped saturation knee, no runner-qualified macro budget, and no
>   measured brownout cost on the surfaces that actually exist. "How much does it hold?" is
>   currently unanswerable; any unscoped p99 or cluster-capacity number is methodologically suspect.
> - **After (depends on):** `0.66.0` (real-process/operational tier and its DaemonCluster/kind
>   lanes supply the daemon control-plane substrate and, critically, the executable surface
>   reconciliation this plan must preserve).
> - **Unblocks:** honest capacity/sizing guidance for adopters, a defensible comparative statement
>   versus Redis on the RESP surface, and regression protection so `1.0` performance cannot silently
>   erode. A native-daemon or distributed-value-plane capacity claim remains blocked until those
>   product surfaces are implemented in a separate release.
> - **Status:** in-progress (W10 Phase A governance scaffold).
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md) -
> governance: release-scoped `0.65`/`0.66` evidence, canaries, receipts, and registries - prior perf
> surface: `0.37` bench-budget, `0.58` soak/overload, `0.64` W22 trace simulator and W37 resource
> budgets.

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
2. **Capacity = sustainable throughput-at-SLO, not peak.** The reported number is the maximum offered
   rate for which p99 (and p999 only above a declared minimum sample count) stays under the SLO,
   achieved/offered throughput stays above the scenario threshold, errors/timeouts/rejections stay
   within the declared budget, and the backlog drains within the declared bound. Passing latency
   while silently dropping or queueing work is not a knee. Peak burst numbers may be recorded but
   are never the headline.
3. **Hardware profile or it did not happen.** Every artifact records CPU model, core count, RAM,
   OS/kernel, toolchain, build flags, CPU/cgroup affinity and quota, governor/turbo posture, runner
   class/fingerprint, and whether the run shared hardware. A caller-supplied `--profile` name is not
   proof that the observed host matches that profile.
4. **Reset, preload, warm-up, steady window, repeats.** Every repeat starts from the scenario's named
   state (fresh daemon/cache/data dir unless the scenario says otherwise), performs deterministic
   preload and application warm-up, opens a fixed steady measurement window, then tears down. Run
   >= 3 repeats and report median with min/max and robust spread; warm-up samples never enter the
   histogram, and a spread above tolerance makes the run non-evidence rather than averaging it away.
5. **Falsifiability.** Each measurement suite has a canary proving the instrument discriminates: a
   deliberately degraded build/fixture must fail its budget, and a synthetic stall must appear in the
   corrected histogram (and would be hidden by a naive closed-loop measure).
6. **Surface identity is mandatory.** Every report records `surface_kind`, `execution_mode`,
   `state_scope`, `network_boundary`, and `claim_scope`. Cross-surface results may be shown side by
   side but never divided into a protocol-overhead or capacity ratio unless the same request path,
   state semantics, load model, and timing boundary make that ratio valid.
7. **Build time is outside measurement.** Dedicated lanes visibly prebuild the exact release binaries
   and bind their hashes to a prebuild manifest before any receipt-bound run. Measurements execute
   those binaries directly; dependency compilation, image pulls, and Cargo rebuilds cannot enter the
   warm-up or steady window. Application/OS-cache warm-up remains a separate recorded phase.
8. **Governance.** Every suite uses the release-scoped `0.65`/`0.66` machinery: exact W-item rows,
   gated/fast registry entries with tier/timeout/owner/artifacts, a dynamic guard/canary pair with an
   `HC-CANARY-RED:W<n>` signature, execution through `evidence-run`, and exact-clean-candidate
   receipts consumed by `release-evidence --release 0.67 --require-ship`.

### Runner-noise and baseline policy

- `ci-shared` runs on a declared GitHub-hosted runner class and is a broad-tolerance regression
  tripwire only. It compares with an eligible rolling baseline from the same runner class,
  fingerprint family, toolchain/build/scenario digest, and its numerical output never produces a
  capacity claim or satisfies a performance ship gate. Hosted structural/unit-test receipts remain
  valid for their non-performance contracts.
- `reference-v1` is the enforcing scheduled/manual lane on a serialized dedicated runner whose
  observed fingerprint matches a committed profile. Missing/mismatched affinity, quota, governor,
  calibration, dedicated label, or `shared_hardware = false` makes the run non-evidence.
- The enforcing decision is dual: the candidate must pass a reviewed immutable release anchor
  (prevents gradual ratcheting) and an eligible rolling `main` baseline (detects recent regressions).
  Candidate, failed, quarantined, mixed-fingerprint, stale, unstable, or current-commit reports are
  ineligible. An insufficient baseline window blocks evidence; it never auto-rebaselines.
- Calibration detects an invalid/noisy environment but never rescales product measurements. Budget
  and baseline changes are explicit reviewed data changes under the W7 no-silent-rebaseline rule.

### Planning-state audit (2026-07-18)

Implementation has not started: `crates/hydracache-loadgen`,
`docs/testing/release-evidence/0.67.toml`, and
`docs/testing/canary-registry-0.67.json` are absent; the `0.67` release catalog row has no
`work_items`, and the current INDEX summaries still contain the stale native-daemon/cluster claims.
Consequently `release-governance-check`, `canary-check`, and `release-evidence` for `0.67` correctly
cannot establish a release contract yet. W10 Phase A below is the mandatory first implementation
change; this audit changes the plan, not the product.

## Wire-Surface Reality Corrections (`0.66` reconciliation carried forward)

The pre-implementation audit confirmed that the original W2-W5 tier map was stale. These corrections
are release invariants, not optional implementation advice:

| Original assumption | `0.66` / current-code fact | Allowed `0.67` measurement | Forbidden claim |
| --- | --- | --- | --- |
| `/client/v1/*` is a native daemon listener | `AxumClientSurface::routes()` owns the router, but `hydracache-server` mounts only RESP and admin listeners | In-process Axum router/dispatch tier, explicitly labeled no socket and no daemon | Native-daemon knee, connection scaling, or native-vs-RESP protocol ratio |
| RESP across 3/5/7 daemons is one value cluster | Redis values, locks, TTL, scripts, and tags are selected-endpoint/node-local | One selected endpoint over the real RESP socket; independent endpoints may be reported separately | Summed RESP throughput as cluster scaling, failover visibility, or replicated-value capacity |
| Consistency/session helpers are an end-to-end grid data path | `HydraCache::get/put` use the local store; current consistency/session/replication types are library/model/helper paths and are not the daemon RESP backend | Algorithmic/library cost with exact callable primitive and execution mode named | End-to-end embedded-grid, daemon-grid, network replication, or 3/5/7 data capacity |
| One generic cluster load can price failover/rebalance | Real daemon faults affect consensus-backed control-plane metadata; a killed RESP endpoint owns independent state | Separate control-plane event profile, killed-endpoint availability profile, and library/model fault-cost profile | One blended cluster brownout or `no lost committed value` claim across node-local RESP |

Adding or mounting a `/client/v1/*` daemon listener, wiring RESP to a distributed backend, or turning
model primitives into an end-to-end value grid is product work and therefore outside this release.
If one of those surfaces lands independently before implementation, the plan, release catalog,
registries, canaries, and claim boundary must be reviewed atomically before measuring it.

## Release Governance Bootstrap (must land before feature W-items)

Before W0 implementation, the first governance change must make `0.67` release-scoped and fail
closed: `docs/plans/releases.toml` declares exact `work_items = ["W0", ..., "W10"]`; `INDEX.md`
carries the matching generated marker and corrected surface summary;
`docs/testing/release-evidence/0.67.toml` creates exact ordered Planned rows and a real W10 governance
row; and `docs/testing/canary-registry-0.67.json` starts with the real W10 guard/canary entry. Each
feature W-item then fills its manifest row and dynamic canary entry atomically before it can become
Implemented. Shared registries are extended using their actual schemas; no parallel evidence
mechanism is invented. Regression tests must reject missing work items, borrowed older registries,
raw commands standing in for `evidence-run`, stale digests, dirty/wrong commits, missing declared
artifacts, and a performance lane that can issue a ship receipt on a shared or mismatched runner.

## Non-Goals

- **No optimization work.** Findings become named backlog items or narrow fixes with their own
  regression measurement; this release does not chase numbers.
- **No marketing benchmark claims (`R-7`).** Artifacts and budgets, not prose superiority claims;
  the comparative row (W8) is a same-box measured statement with methodology attached.
- **No new product surface, consistency level, or protocol change.** A tiny test-only seam (e.g., a
  latency-injection failpoint for canaries) must stay behind `test-failpoints`/dev-deps.
- **No synthetic daemon client wire or distributed RESP backend.** W2 stays in-process; W3 stays
  selected-endpoint/node-local. Test-only loopback may price harness overhead only and may not be
  relabeled as a shipped listener.
- **No aggregate "cluster capacity" number.** Real control-plane event cost and library/model
  primitive cost are separate evidence classes; neither substitutes for an end-to-end distributed
  value-plane benchmark.
- **No cloud/multi-region benchmarking.** Single-box and single-LAN kind/loopback tiers only; WAN
  characterization is future work once `0.45` geo paths need sizing.
- **No load-generator product.** `hydracache-loadgen` is a dev tool crate (`publish = false`).

## Preflight

Re-grep before implementing; do not trust this plan's anchors blindly:

- existing micro benches: `crates/hydracache/benches/`, `crates/hydracache-db/benches/`, xtask
  `bench-budget` (`crates/xtask/src/` - budget file + criterion baseline comparison).
- endurance/overload: `0.58` soak driver (`hydracache-sim/src/bin/vopr.rs` extensions), admission/
  backpressure tests, RSS/fd sampler; `0.64` W37 `daemon_resource_budget.rs` artifact schema.
- workload inputs to reuse rather than fork: `hydracache-cache-sim::{TraceEvent, parse_trace}` and
  the committed `standard`, `skewed_zipfian`, and `recency_ttl` trace catalog from `0.64` W22. Those
  tiny traces are deterministic smoke/quality fixtures, not sustained capacity workloads; W1 may
  extend the shared crate with a versioned seeded Uniform/Zipfian key schedule.
- harnesses to reuse: `DaemonCluster` (real processes), `0.61`/`0.66` kind lanes, real RESP listener,
  `redis-benchmark` compatibility (`hydracache-redis-compat`), the in-process
  `AxumClientSurface`/`ClientRequestEnvelope` seam, and the actual admin/metrics endpoints. Re-grep
  `hydracache-server/src/main.rs` and route owners before assigning any network claim.
- cold-build precedents: `ce7f846` separates a 300s cold-run timeout from a 60s execution budget;
  `5882109` adds an explicit same-toolchain prebuild before receipt execution. Performance lanes
  must use the stronger explicit-prebuild pattern.
- governance seams: `release-evidence`, `gated-test-registry.toml`, canary-registry, fast-suite
  budgets, quarantine (`crates/xtask/src/`).
- reference blueprints in the workspace root: `redis/src/redis-benchmark.c` (RESP loadgen),
  `caffeine/caffeine/src/jmh` (concurrent cache micro/meso benches), `tigerbeetle` benchmark +
  budget philosophy, YCSB workload taxonomy (A-F mixes), `wrk2`/HdrHistogram methodology,
  `hikaricp` pool-behavior measurement style.

Audit question:

```text
For each existing surface, does a recorded artifact say exactly what executed, where state lived,
which network boundary was crossed, and what claim follows; and, only for capacity-bearing paths,
does it state a sustainable open-loop rate at SLO on an eligible runner with a discriminating canary
and a dual regression budget? Does every unavailable end-to-end path remain explicitly deferred?
```

Anywhere the answer is "no" is the gap this release closes.

## Implementation Map For Audits

Populate as W-items land (same discipline as `0.64`): item -> where implemented -> required command
-> boundary/gate.

| Item | Implemented where | Required command | Boundary |
| --- | --- | --- | --- |
| _(populate during implementation; W0-W10 below define the targets)_ | | | |

Direct commands inside W0-W9 are developer reproduction commands. A ship-eligible invocation runs
the registered command only through
`cargo run -p xtask --locked -- evidence-run --release 0.67 --gate <id>` so its exact clean commit,
command/registry/input digests, runner identity, and declared artifact hashes are receipt-bound.

Dependency order is explicit: W10 Phase A -> W0; W0 -> W1/W2/W3/W4A/W4B; W3 -> W5B/W8/W9;
W4A -> W5A/W9; W4B -> W5C; valid W1/W2/W3 (and, conditionally, W4A read-only) knees -> W6;
W1-W6 -> W7; and all W0-W9 evidence -> W10 Phase B. W3 has no W2 dependency.

## W0. Open-Loop Load Foundation (blueprint: `wrk2`/HdrHistogram, Gil Tene; principle: fixed-rate open loop + latency from scheduled send time)

**Goal.** One reusable measurement core every other W-item consumes: a fixed-rate open-loop driver,
an HdrHistogram-style recorder (bounded memory, configurable precision), stepped-rate knee search,
reset/preload/warm-up/steady-window/repeat orchestration, and a canonical `PERF_REPORT` JSON schema.
Each scenario TOML declares numeric SLO, steady window, minimum sample counts, achieved/offered
threshold, error/timeout/rejection budgets, backlog-drain bound, and spread tolerance.

**Files to change.** New dev crate `crates/hydracache-loadgen` (`publish = false`): `rate.rs`
(open-loop scheduler; a missed tick is *recorded as latency*, never skipped), `histogram.rs`,
`knee.rs` (full sustainability predicate), `scenario.rs`, `profile.rs`, `report.rs`, and a pluggable
`Target` trait. Concrete targets are only those named by W1-W4; there is no `node-native` target.
The report includes offered/started/completed/achieved rates, backlog and drain result,
p50/p90/p99/p999 or a loud insufficient-sample marker, error/timeout/rejection counts, phase
durations/op counts, repeat/state/scenario/workload digests, surface identity fields, runner profile
and observed fingerprint, affinity/quota/governor/calibration facts, prebuilt binary hashes, seed,
toolchain/build flags, stable `prebuild_contract_digest`, per-run `prebuild_manifest_sha`, git commit,
and spread/stability verdict.

**Required tests (fast, deterministic - the instrument itself is under test):**
- `open_loop_scheduler_accounts_missed_ticks_as_latency_not_skips`;
- `histogram_percentiles_match_reference_values_on_known_distributions`;
- `knee_search_finds_the_stated_knee_on_a_synthetic_latency_model`;
- `knee_rejects_rate_when_latency_passes_but_achieved_rate_lags`;
- `knee_rejects_timeout_rejection_budget_or_undrained_backlog`;
- `p999_is_unreportable_below_the_declared_sample_count`;
- `warmup_samples_never_enter_the_steady_histogram`;
- `repeat_reset_reproduces_the_initial_state_digest`;
- `reference_profile_rejects_a_spoofed_or_shared_runner`;
- `perf_report_schema_records_surface_profile_commit_workload_and_prebuild_digests`.

**Canary.** `canary_closed_loop_measurement_hides_a_synthetic_stall` - drive a target with an
injected 1s stall through (a) the open-loop recorder and (b) a naive closed-loop recorder; the stall
must dominate p99 in (a) and be invisible in (b). Proves the whole release's methodology
discriminates; it is the load-bearing canary and must fail with `HC-CANARY-RED:W0` under its
registered defect fixture.

**DoD.**
```powershell
cargo test -p hydracache-loadgen --locked -j 2
```
**CI.** Fast `rust` job; the loadgen crate never enters a release dependency graph
(`verify-no-test-features` discipline).

## W1. Local Cache Tier Characterization (blueprint: Caffeine `caffeine/src/jmh` GetPutBenchmark; principle: scaling curve + contention worst case, not single-thread averages)

**Goal.** Answer "how much does the **embedded** cache hold": multi-thread scaling curve, hot-key
contention floor, eviction-pressure cost, hit/miss/loader path cost, allocation profile.

**Files to change.** `crates/hydracache-loadgen/src/targets/local.rs` plus versioned scenario TOMLs.
Reuse the W22 trace parser/catalog for deterministic replay and extend the shared
`hydracache-cache-sim` input layer, if needed, with a versioned seeded Uniform/Zipfian key schedule;
the loadgen owns the YCSB-style operation/payload mix around that key stream. Do not copy the trace
parser, benchmark `PolicyKind::Hydra` instead of the real cache, or treat 24-34-event fixtures as a
sustained workload.

**Required measurements:**
- `local_cache_scaling_curve_1_to_n_threads` (knee per mix; report scaling efficiency vs 1 thread);
- `hot_key_contention_throughput_floor` (all threads on one key - single-flight/lock worst case);
- `throughput_at_full_capacity_vs_half_capacity` (every insert evicts, zipfian and uniform);
- `hit_miss_and_loader_path_cost_breakdown`;
- `bytes_allocated_per_operation_by_feature` (baseline vs TTL vs tags, counting allocator);
- `w22_trace_replay_preserves_order_and_records_trace_digest` and seeded workload reports record
  generator version, seed, theta, key count, operation mix, payload mix, and workload digest.

**Canary.** `canary_injected_slow_eviction_breaches_the_local_budget` - a test-only latency
failpoint in the eviction path must push the knee below budget and emit `HC-CANARY-RED:W1`.

**DoD.**
```powershell
cargo build -p hydracache-loadgen --release --locked
& target\release\hydracache-loadgen.exe tier local --profile reference-v1 --report target/test-evidence/0.67/local.json
```
**CI.** Dedicated reference lane after the W10 prebuild/fingerprint gate. A short smoke variant runs
in the fast job at trivially low rates (schema/plumbing check, explicitly not a measurement).

## W2. Client-Surface Library Tier - In-Process Axum Router, Not Daemon Wire (blueprint: YCSB taxonomy; principle: measure the callable boundary and name what is absent)

**Goal.** Characterize `AxumClientSurface` plus `ClientRequestEnvelope` through its in-process Router:
scheduled-latency knee, concurrent in-flight scaling, payload sweep, codec/dispatch cost, and
admission rejection. This is a client-surface/library result with process-local state. It is not a
socket, daemon listener, accept loop, connection-capacity, or native-wire result.

**Files to change.** `crates/hydracache-loadgen/src/targets/client_surface.rs`, using the real router
and request framing without mounting it in `hydracache-server`; scenario definitions for A(50/50),
B(95/5), and C(read-only) mapped to supported get/put/mget/batch shapes. Every report fixes
`surface_kind = client-surface`, `execution_mode = in-process-axum-router`,
`state_scope = process-local`, and `network_boundary = none`.

**Required measurements:**
- `client_surface_in_process_knee_at_slo_for_a_b_c`;
- `concurrent_inflight_scaling_curve_1_10_100_1000` (not connection scaling);
- `client_surface_payload_sweep_100b_1kb_64kb_1mb` to the documented limits, with loud rejection
  beyond the cap;
- `client_surface_codec_dispatch_and_admission_rejection_cost`.

**Canary.** `canary_injected_client_surface_dispatch_delay_breaches_the_in_process_budget` must fail
with `HC-CANARY-RED:W2`; a governance test rejects any W2 artifact labeled daemon or wire.

**DoD.**
```powershell
& target\release\hydracache-loadgen.exe tier client-surface --profile reference-v1 --report target/test-evidence/0.67/client-surface.json
```
**CI.** Dedicated reference lane plus fast smoke; it does not start `DaemonCluster`. A future
product listener requires a different release and a newly reviewed tier.

## W3. Single-Daemon Node-Local RESP Tier (blueprint: `redis-benchmark`/memtier; principle: open-loop SLO evidence plus unmodified ecosystem-tool interop)

**Goal.** Characterize one selected daemon RESP endpoint. The W0 open-loop RESP socket target owns
all scheduled-latency, SLO-knee, and overload claims. Unmodified `redis-benchmark` (optionally
`memtier_benchmark`) supplies separate closed-loop throughput/interoperability evidence and the W8
same-tool comparison. Values, locks, TTL, scripts, and tags remain endpoint-local.

**Files to change.** `crates/hydracache-loadgen/src/targets/resp.rs` for the open-loop socket path and
`src/resp_external.rs` for process launch/CSV parsing. Reports from the two instruments stay separate
and record pipeline/connection matrix, methodology, selected endpoint, and
`state_scope = node-local`. Add Docker/tool registry rows only for the external-tool leg.

**Required measurements:**
- `resp_open_loop_get_set_knee_at_slo` plus A/B/C mixes supported by the facade;
- `resp_open_loop_connection_and_pipeline_sweeps` with scheduled-send latency;
- `redis_benchmark_get_set_mset_throughput_and_interop` as explicitly closed-loop supplemental data;
- `resp_open_loop_stall_is_visible_in_scheduled_latency`.

There is no W2 ratio: an in-process router divided by a daemon TCP/RESP result is not protocol
overhead. The only allowed product comparison is W8, where both systems use the same tool on the
same box.

**Canary.** The registered W3 defect command covers both
`canary_resp_listener_slowdown_breaches_the_open_loop_resp_budget` and rejection of truncated/
swallowed external-tool output; it must fail with `HC-CANARY-RED:W3`. A closed-loop tool result
cannot satisfy the open-loop guard.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_PERF_RESP='1'
& target\release\hydracache-loadgen.exe tier node-resp --profile reference-v1 --report target/test-evidence/0.67/node-resp-open-loop.json
Remove-Item Env:\HYDRACACHE_RUN_PERF_RESP -ErrorAction SilentlyContinue
```
**CI.** Open-loop leg on the dedicated reference runner. A local unclaimed external-tool run skips
loud when the tool is absent; the scheduled/manual mandatory evidence gate fails closed on a missing
tool, image, or capability. W3 depends on W0, not W2.

## W4. Split Cluster-Adjacent Characterization - Real Daemon Control Plane And Library/Model Primitives

**Goal.** Produce two non-combinable evidence classes. W4A measures the real 3/5/7-daemon
control-plane/admin boundary. W4B prices selected consistency/session/replication primitives in an
explicitly constructed in-process model. Neither is distributed-cache value capacity, and no YCSB
data knee or summed RESP throughput is reported for the daemon group.

**W4A - real-process control plane.** Reuse `DaemonCluster` and the actual admin/cluster endpoints:

- `admin_status_and_overview_knee_at_slo_for_3_5_7_daemons` for the repeatable read-only wire;
- `membership_add_drain_commit_and_convergence_latency_3_5_7` as event latency, not throughput;
- control-plane CPU/network/commit/convergence counters scoped to the exact operation;
- no live reshard cost: `/admin/reshard` is request acceptance and the networked grid currently
  reports an idle phase, so rebalance/reshard performance remains explicitly deferred.

Each W4A steady run targets one exact admin endpoint and records `target_node_id`,
`target_node_role`, the complete endpoint set, and role changes. Leader and follower observations are
separate scenarios; a role change invalidates an ordinary steady window and belongs to W5. Results
for endpoints or node counts are never summed into cluster capacity.

**W4B - library/model primitives.** Use the real exported types but label the constructed harness:

- `consistency_ack_requirement_cost_by_level_and_replica_shape` for `ConsistencyLevel` math, not
  read/write capacity;
- `session_ryw_and_staleness_decision_cost` for the exact helper functions exercised;
- `replication_peer_and_store_model_primitive_curve` plus
  `modeled_replica_copy_bytes_per_input_byte` (never "committed-byte amplification");
- `in_process_invalidation_bus_fanout_cost`, separate from value replication.

**Files to change.** `src/targets/control_plane.rs`, `src/targets/grid_model.rs`, and distinct scenario
and report paths. Schema validation rejects a merged W4 report or any W4B report with
`daemon_processes = true`, `product_data_plane = true`, or an end-to-end cluster-capacity claim.

**Canary.** One registered composite W4 defect command proves both instruments discriminate:
`canary_control_plane_delay_breaches_the_w4a_event_budget` and
`canary_grid_model_short_circuit_is_rejected`; it must fail with `HC-CANARY-RED:W4`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_PERF_CONTROL_PLANE='1'
& target\release\hydracache-loadgen.exe tier control-plane --nodes 3 --target-roles leader,follower --profile reference-v1 --report target/test-evidence/0.67/control-plane-3.json
& target\release\hydracache-loadgen.exe tier control-plane --nodes 5 --target-roles leader,follower --profile reference-v1 --report target/test-evidence/0.67/control-plane-5.json
& target\release\hydracache-loadgen.exe tier control-plane --nodes 7 --target-roles leader,follower --profile reference-v1 --report target/test-evidence/0.67/control-plane-7.json
& target\release\hydracache-loadgen.exe tier grid-model --profile reference-v1 --report target/test-evidence/0.67/grid-model.json
Remove-Item Env:\HYDRACACHE_RUN_PERF_CONTROL_PLANE -ErrorAction SilentlyContinue
```
**CI.** W4B fast smoke plus dedicated reference run; W4A 3-node is scheduled and required 5/7-node
points run manual on the same eligible profile, all with mandatory receipts. The kind variant is an
optional informational observation unless governance is atomically widened to make it mandatory.

## W5. Surface-Specific Operational Brownout And Availability Profiles (blueprint: `0.62`/`0.66`; principle: event depth and recovery must follow the affected authority)

**Goal.** Measure operational events without blending unrelated state planes. Each experiment runs
at a fixed sub-knee load from its own valid predecessor and records dip depth, comparable latency,
availability/error shape, and time-to-recover.

**W5A - control-plane brownout (depends on W4A):**
- leader failover, member add/drain, and node kill/rejoin under admin observation load;
- cross-check `no lost committed metadata transition` and convergence of public membership views;
- no generic client-write or distributed-value invariant.

**W5B - node-local RESP availability (depends on W3):**
- kill/restart the selected endpoint under open-loop RESP load and record its outage/recovery;
- record surviving endpoints only as independent controls with `automatic_failover = false`;
- recovery means socket availability and steady throughput from the explicitly recorded post-restart
  state, not data recovery; never claim neighbor visibility, value survival, or cross-node failover.

**W5C - library/model fault cost (depends on W4B):** inject a slow/unavailable modeled replica into
the explicitly constructed primitive harness and record decision/backpressure/recovery cost. It is
not daemon brownout evidence. Live rebalance/reshard remains deferred until an end-to-end
implementation exists.

**Files to change.** Surface-specific orchestration over `DaemonCluster`, the RESP target, and the
model harness; reports cannot share a headline or aggregate goodput value.

**Canary.** One registered W5 command combines
`canary_extended_leader_downtime_breaches_the_control_plane_brownout_budget`, a fixture that falsely
claims RESP neighbor failover, and a no-op model fault; the validator rejects all three and emits
`HC-CANARY-RED:W5`. Guards also reject the old `no lost committed write` and reshard claims.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_PERF_CONTROL_PLANE='1'
& target\release\hydracache-loadgen.exe brownout control-plane-leader --profile reference-v1 --report target/test-evidence/0.67/brownout-control-plane.json
& target\release\hydracache-loadgen.exe brownout resp-endpoint-kill --profile reference-v1 --report target/test-evidence/0.67/brownout-resp-endpoint.json
& target\release\hydracache-loadgen.exe brownout grid-model-replica --profile reference-v1 --report target/test-evidence/0.67/brownout-grid-model.json
Remove-Item Env:\HYDRACACHE_RUN_PERF_CONTROL_PLANE -ErrorAction SilentlyContinue
```
**CI.** Dedicated scheduled/manual lane with separate artifacts and gate IDs per surface.

## W6. Overload Goodput And Recovery Curves (blueprint: `0.58` admission proof + Netflix/SRE brownout practice; principle: the shape of degradation matters, not only its boundedness)

**Goal.** Measure goodput, scheduled p99, rejection/error ratio, backlog, and time-to-baseline at
1.2x/1.5x/2x a valid W1/W2/W3 capacity knee. W4A read-only admin may participate only if it has the
same complete sustainability predicate; one-shot control-plane events and W4B model costs do not
inherit a synthetic "cluster knee". Any model overload curve stays labeled library/model.

**Required measurements:**
- `overload_goodput_curve_1_2x_1_5x_2x_knee_per_eligible_surface`;
- `rejection_ratio_latency_and_backlog_under_overload`;
- `recovery_time_to_baseline_after_burst`;
- coverage rejects the removed `node-native` and generic `cluster` tier names.

**Canary.** `canary_admission_disabled_fixture_shows_goodput_collapse` - with the admission gate
bypassed (test-only), goodput at 2x must collapse instead of plateauing; the registry expects
`HC-CANARY-RED:W6`.

**DoD.**
```powershell
& target\release\hydracache-loadgen.exe overload local --profile reference-v1 --report target/test-evidence/0.67/overload-local.json
& target\release\hydracache-loadgen.exe overload client-surface --profile reference-v1 --report target/test-evidence/0.67/overload-client-surface.json
& target\release\hydracache-loadgen.exe overload node-resp --profile reference-v1 --report target/test-evidence/0.67/overload-node-resp.json
```
**CI.** Dedicated reference lane; shared-hosted output is a non-enforcing tripwire only.

## W7. Macro Perf Budgets And Regression Gate (blueprint: `0.37` bench-budget extended; TigerBeetle budget discipline; principle: a measured floor, keyed by hardware profile, that CI defends)

**Goal.** Freeze the W1-W6 results as **macro budgets** (ops/s floors at SLO, p99 ceilings, brownout
depth/recovery ceilings, overload goodput floors) without trusting shared-runner noise or allowing a
slow rolling ratchet. `ci-shared` is a wide-tolerance tripwire; `reference-v1` is the enforcing
dedicated profile and must pass both its reviewed release anchor and an eligible rolling `main`
baseline. Budget rows preserve `claim_scope`: capacity, operational event, and library/model
primitive costs are different types and cannot satisfy one another.

**Files to change.** `docs/testing/perf-profiles/{ci-shared,reference-v1}.toml`,
`docs/testing/perf-budgets/0.67/{ci-shared,reference-v1}.toml`, a versioned rolling-baseline contract,
and xtask `perf-budget-check`. The checker consumes the exact expected report set, rejects
missing/extra/mixed reports, validates commit/profile/fingerprint/SLO/methodology/spread/prebuild and
budget digests, and writes `target/test-evidence/0.67/perf-budget-verdict.json` with hashes of every
input report, baseline member, profile, and budget. Runtime reports are gate artifacts, not committed
`required_artifacts` in the evidence manifest.

**Required tests/gates:**
- `perf_budget_check_fails_on_floor_breach_and_on_unstable_spread`;
- `perf_budget_change_requires_reviewed_budget_file_edit` (no auto-rebaseline - the `0.64` W32
  no-silent-regeneration discipline applied to budgets);
- `rolling_baseline_uses_only_eligible_same_fingerprint_main_reports`;
- `rolling_baseline_rejects_mixed_stale_insufficient_or_unstable_window`;
- `candidate_cannot_baseline_itself`;
- `release_anchor_prevents_slow_rolling_ratcheting`;
- `baseline_manifest_and_budget_verdict_are_receipt_digest_bound`;
- `every_capacity_bearing_surface_has_a_reference_v1_anchor`.

The default rolling selection is the ten most recent eligible successful `main` run medians, with a
minimum of five and maximum age of 30 days; the committed contract may tighten these values only by
review. Every member must match runner fingerprint, toolchain identity, `prebuild_contract_digest`,
and scenario/workload digests and pass its own calibration/spread verdict. The stable prebuild
`prebuild_contract_digest` is computed over toolchain, target set, features, profile, flags, and
build recipe.
Per-run source commit, Cargo.lock hash, binary hashes, and `prebuild_manifest_sha` are expected to
differ; they are recorded and receipt-bound but are not baseline-eligibility equality keys. The
baseline manifest records run ids, commits, report hashes, selection reason, median/MAD, and the
configured tolerance.

**First-contract bootstrap.** After loadgen, scenarios, profile, and report schema are frozen, that
exact contract must land on `main` before the release candidate is frozen. Run at least five eligible
`reference-v1` measurements on clean pre-candidate `main` commits with the same digests; review their
median/spread into the immutable anchor and rolling-baseline manifest, then commit the anchor/budgets
and rerun the final candidate. Bootstrap observations are provenance, not final-candidate receipts.
If the eligible `main` window cannot be assembled, 0.67 waits; candidate/self-baselining and an
"anchor-only for now" exception are forbidden.

**Canary.** `canary_perf_budget_accepts_a_silent_rebaseline_or_candidate_self_baseline` must fail
with `HC-CANARY-RED:W7`.

**DoD.**
```powershell
cargo run -p xtask --locked -- perf-budget-check --release 0.67 --profile reference-v1
```
**CI.** Fast job validates schema/coverage. Hosted CI produces only `ci-shared` tripwires. The
receipt-bound `tool.perf-budget-check-067` verdict is created on the dedicated profile after all
mandatory reports are restored at their exact declared paths.

## W8. Same-Box Comparative Baseline Versus Redis (blueprint: `redis-benchmark` both ways; principle: comparative honesty - one box, one method, both systems, methodology attached)

**Goal.** One reviewed artifact: same box, same tool (`redis-benchmark`), same ops (GET/SET,
pipeline 1 and 10) against real Redis (pinned image from `0.63` oracle set) and HydraCache RESP.
Output is a measured ratio with methodology, stored as evidence - **not** a marketing claim (`R-7`).
This bounds the honest "cost of the facade + engine" statement for POSITIONING.

**Required measurements:**
- `same_box_redis_vs_hydracache_resp_get_set_ratio` (pipeline 1/10, recorded spread), alternating
  execution order across repeats to expose thermal/order bias;
- artifact includes both raw outputs, image/binary digests, exact tool versions, host fingerprint,
  and the selected-endpoint/node-local divergence note.

**Canary.** `canary_same_box_comparison_accepts_a_mismatched_host_or_unpinned_redis` must fail with
`HC-CANARY-RED:W8`; W3's instrument canary cannot substitute for this comparison-boundary proof.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_PERF_RESP='1'
& target\release\hydracache-loadgen.exe compare redis --profile reference-v1 --report target/test-evidence/0.67/compare-redis.json
Remove-Item Env:\HYDRACACHE_RUN_PERF_RESP -ErrorAction SilentlyContinue
```
**CI.** Mandatory Docker/pinned-Redis reference gate. A local unclaimed run may skip loud; the
claimed CI gate fails closed if Docker, the exact tool, image digest, or reference runner is absent.

## W9. Metrics Honesty Cross-Check (blueprint: `0.57` exporter + R-11; principle: the server's own numbers must match an independent observer)

**Goal.** During W3 RESP and W4A admin/control-plane runs, cross-check real server-reported counters
and comparable latency fields against the independent observer. W2 and W4B are in-process and may
validate their own instrumentation, but cannot be called daemon/exporter metrics evidence. Internal
service time is compared only with the matching observer interval; it is never expected to equal or
replace queue-inclusive scheduled-send latency.

Only fields already exported by the daemon are eligible. The artifact contains a metric-coverage
table; an absent operation counter or latency summary is recorded `not_available` and receives no
agreement claim. W9 does not add an exporter metric merely to make its own comparison green.

**Required tests:**
- `server_reported_ops_and_rejects_match_loadgen_within_tolerance`;
- `server_latency_and_open_loop_scheduled_latency_have_explicit_non_conflated_boundaries`;
- `metrics_cross_check_rejects_in_process_reports_labeled_as_daemon_exporter_evidence`.

**Canary.** `canary_metrics_undercount_fixture_is_detected` (a fixture dropping every Nth counter
increment must fail the cross-check with `HC-CANARY-RED:W9`).

**DoD.**
```powershell
cargo test -p hydracache-loadgen metrics_cross_check --locked -j 2
```
**CI.** Runs inside the mandatory RESP and control-plane reference gates (same processes, separate
validator artifact; no hosted-runner ship claim).

## W10. Governance, CI Lanes, And Docs

**Goal.** Build the exact-candidate release contract first, then freeze and aggregate every mandatory
measurement on the final clean candidate. W10 has two phases; **Phase A is the first implementation
work in the release**, before W0 feature code, while Phase B closes the release after W0-W9.

**Phase A - fail-closed scaffold (lands first).**

- Add exact `work_items = ["W0", ..., "W10"]` plus the `INDEX.md` marker and create
  `release-evidence/0.67.toml` with W0-W10 rows in that order and `ship_required = true` on each row.
  In Phase A, W10 has real governance sources/tests while W0-W9 rows remain intentionally incomplete
  and therefore `Planned`; fake placeholder tests may not mark them implemented. Start
  `dynamic_canary_work_items = ["W10"]` and a release-scoped canary registry with exact JSON
  `"version": 2`, one W10 guard/canary entry, generated sweep-receipt path, and
  `HC-CANARY-RED:W10`.
- Before or with each W0-W9 implementation, atomically populate its non-empty committed
  `required_sources`, real Rust `#[test]` validators, fast/gated IDs and artifacts, add its id to the
  dynamic list, and add its real guard/canary entry. CLI measurements alone do not satisfy the AST
  contract. Phase B requires the full ordered W0-W10 dynamic list and exact-commit sweep receipts;
  `--require-ship` intentionally remains red until then.
- Register `fast.performance-contract-067` with the actual fast-suite schema: `work_items`,
  timeout/budget, deterministic flag, artifacts/logical digest, baseline, and command. Its mandatory
  receipt comes from every W-item referencing it in `fast_gate_ids`; fast rows do not invent gated
  fields such as `owner_release` or `ship_mandatory`.
- Register gated rows `tool.perf-prebuild-067`, `env.hydracache-run-067-perf-core`,
  `env.hydracache-run-067-perf-resp`, `env.hydracache-run-067-perf-control-plane`, and
  `tool.perf-budget-check-067` with exact command, tier/timeout/platform, required tools/env,
  CI workflow/job/step, artifact paths, `owner_release = "0.67.0"`, and
  `ship_mandatory = true`. An optional kind row is `ship_mandatory = false` and cannot support a
  release claim; promoting it to a claim requires a mandatory receipt. Every measurement W-item
  references `tool.perf-prebuild-067` and its relevant execution/verdict gates in `gated_gate_ids`.
- Put committed schemas/profiles/budgets in manifest `required_artifacts`; put every generated
  `PERF_REPORT`, raw tool output, prebuild/baseline/budget verdict, and process log at an exact
  `target/test-evidence/0.67/...` path in its gate `artifacts`. Globs, uploads without hashes, stale
  files, and post-command artifact creation are non-evidence.
- Extend requested-release governance and CI binding to `0.67`; reject older-registry fallback,
  missing work items/marker, raw commands in claimed CI steps, optional/missing mandatory gates,
  shared/mismatched reference runners, wrong/dirty commits, and stale command/registry/input or
  artifact digests. The real `v0.66.0` compatibility baseline must be an ancestor of the candidate.

**Phase B - execution and candidate freeze.**

- The dedicated job uses `runs-on: [self-hosted, linux, x64, hydracache-perf-v1]` (or an atomically
  reviewed equivalent) with serialized `concurrency`. First run the mandatory
  `tool.perf-prebuild-067` through `evidence-run`; it builds the exact release server/loadgen/test
  targets and creates `target/test-evidence/0.67/prebuild-manifest.json` with commit, Cargo.lock,
  toolchain/flags, stable build-contract digest, and binary hashes. Consumer perf gates do **not**
  redeclare that file as their own artifact (which `evidence-run` would clear); they validate it,
  execute binaries directly, and embed its per-run SHA, build-contract digest, and binary SHAs in
  their own receipt-hashed reports. The final budget verdict cross-checks those hashes against the
  prebuild-gate receipt while comparing baseline eligibility only on the stable contract digest.
  Missing/mismatched manifests, changed binaries, or a Cargo rebuild during a measurement gate is red.
- `ci-shared` hosted runs are non-ship tripwires. Dedicated core, RESP/Redis, and control-plane gates
  fail closed on missing capabilities and upload exact artifacts plus receipts. Fast canary sweep
  runs on PRs; `canary-sweep --release 0.67 --tier all` is mandatory scheduled/dispatch evidence.
- Finalize budgets and docs **before** the frozen run, then rerun every perf and canary gate on that
  exact clean commit. The final aggregator checks out the same full-history SHA, restores receipts,
  canary receipts, and declared artifacts to their exact paths, runs the receipt-bound budget gate,
  and finally runs `release-evidence ... --receipts-dir ... --require-ship`. Any later budget, docs,
  registry, or command commit invalidates earlier receipts and requires a complete rerun.
- Document PowerShell and Bash reproduction in `docs/TESTING.md`; add `docs/PERFORMANCE.md` and
  `docs/releases/0.67.0.md`; reconcile `GATES.md`, `releases.toml`, the INDEX graph/table/marker, plan
  status, `POSITIONING.md`, and the README release-note link. Numbers without surface/profile/
  methodology are not quotable.

**Required governance tests.** `release_067_registered_performance_gates_are_mandatory_and_fail_closed`,
`performance_lane_requires_dedicated_label_and_serial_concurrency`,
`measurement_refuses_missing_or_mismatched_prebuild_manifest`,
`prebuilt_binary_digest_is_bound_to_report`, `compile_time_is_excluded_from_measurement_window`,
`prebuild_receipt_hash_matches_every_performance_report`,
`consumer_gate_does_not_delete_the_prebuild_manifest`,
`runtime_reports_are_gate_artifacts_not_committed_manifest_artifacts`, and
`final_aggregator_requires_exact_candidate_receipts_and_artifact_hashes`.

**Canary.** `canary_release_governance_accepts_a_missing_mandatory_performance_gate` must fail with
`HC-CANARY-RED:W10`.

**DoD.**
```powershell
cargo run -p xtask --locked -- doc-check
cargo run -p xtask --locked -- canary-check --release 0.67
cargo run -p xtask --locked -- gated-test-check
cargo run -p xtask --locked -- fast-suite-check --release 0.67
cargo run -p xtask --locked -- release-governance-check --release 0.67
cargo run -p xtask --locked -- canary-sweep --release 0.67 --tier all
cargo run -p xtask --locked -- quarantine-check --release 0.67
cargo run -p xtask --locked -- verify-no-test-features
cargo run -p xtask --locked -- evidence-run --release 0.67 --gate fast.performance-contract-067
cargo run -p xtask --locked -- evidence-run --release 0.67 --gate tool.perf-prebuild-067
cargo run -p xtask --locked -- evidence-run --release 0.67 --gate env.hydracache-run-067-perf-core
cargo run -p xtask --locked -- evidence-run --release 0.67 --gate env.hydracache-run-067-perf-resp
cargo run -p xtask --locked -- evidence-run --release 0.67 --gate env.hydracache-run-067-perf-control-plane
cargo run -p xtask --locked -- evidence-run --release 0.67 --gate tool.perf-budget-check-067
cargo run -p xtask --locked -- release-evidence --release 0.67 --receipts-dir target/release-evidence/receipts --require-ship
```

## Gates (Definition of Done for the release)

- The W0 instrument is itself proven: missed ticks count as latency, percentiles match reference
  distributions, the full sustainability predicate rejects dropped/queued work, and the closed-loop
  canary demonstrably hides a stall the open-loop recorder reports.
- Valid `reference-v1` knees exist only for the embedded cache, in-process client router, real
  selected-endpoint RESP wire, and (if its complete predicate holds) the read-only admin surface.
  W3's SLO result comes from the open-loop RESP target; `redis-benchmark` remains supplemental.
- W4A real control-plane event/read results and W4B library/model primitive costs are separate
  artifacts with separate claim scopes. No native-daemon, distributed value-grid, summed RESP,
  consistency-level data capacity, or live-reshard performance claim appears anywhere.
- Brownout evidence is split into committed-metadata control-plane recovery, selected node-local RESP
  availability with `automatic_failover = false`, and labeled model fault cost. No-lost-value and
  live-reshard claims are absent.
- Overload curves at 1.2x/1.5x/2x a valid knee are recorded per eligible surface; the canary shows
  collapse, proving the curve reflects the mechanism.
- Dedicated `reference-v1` budgets pass both immutable anchors and eligible rolling-main baselines;
  shared hosted runs are tripwires only. Floor/ceiling breach, unstable environment/spread,
  insufficient baseline, profile mismatch, or silent rebaseline is red.
- The same-box Redis comparative artifact exists with methodology and versions; no prose claim
  exceeds the artifact (`R-7`).
- Real RESP/control-plane server metrics match comparable independent observations; in-process
  metrics are not relabeled as exporter evidence.
- All W0-W10 guards and exact `HC-CANARY-RED` defects are receipt-green; every mandatory performance
  gate ran via `evidence-run`, and exact clean-candidate artifacts, canary receipts, and gate receipts
  make `release-evidence --release 0.67 --receipts-dir ... --require-ship` green. Missing mandatory
  tools/capabilities fail closed rather than skip.
- No optimization work, no product surface change, loadgen never in a release graph; any fix driven
  by a finding is narrow, separate, and regression-measured.

## Final Release Decision

Ship `0.67.0` only when artifacts answer the narrower questions the product can honestly support:
embedded-cache capacity, in-process client-surface cost, single-endpoint node-local RESP capacity,
real control-plane read/event cost, and explicitly labeled library/model primitive cost. Every
capacity number must pass the open-loop sustainability predicate on the dedicated reference profile;
every operational profile must name the authority it perturbs; overload must be tied to a valid knee;
the same-box Redis comparison must preserve one tool/host/method; and dual budgets plus exact-candidate
receipts must defend the results. Native-daemon and distributed-value-cluster capacity remain named
deferrals, not zeroes and not estimates. The release adds measurement and evidence only - the fastest
thing it is allowed to make faster is the feedback loop that tells the truth about performance.
