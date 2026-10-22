# Installed authority runtime CPU and lifecycle timing

The first complete local CPU/elapsed cohort does not confirm a practical speed
benefit from owned scratch storage. Warm CPU point estimates improve at depth
127, but the paired intervals include both benefit and cost; complete-cycle
elapsed point estimates disagree between compilers. The measured allocation
reduction and additional retained memory remain valid separate findings. This
result establishes neither performance equivalence nor a regression-free release.

## Fixed sources and measurement scope

The [preregistered method](authority-runtime-cpu-plan-20261022.md) compares baseline
`33c54b0cda8c330b115009cdf092ad002077cedf` with installed runtime
`66d5f371680d085a24de8f2558e1f049c71b9e7b`. Starting checkout is 32ba3f5a;
method is `12d158427322d3e58c3e06a79f3038d95dec36ec`, evidence
`b79af89fe9eef724334c381b9e4dbbb47bf6b0ef`. No repository runtime/test body changes.
The same eight baseline files, complete package bytes/membership, original
checks, root manifest/lockfile and generated probe are independently verified.
Only a read-only scratch inspector differs by role outside timing brackets.

New default-feature release libtest images use the unwrapped System allocator.
The allocation counter and old model/allocation images do not execute. Separate
compiler/role targets, fresh:false artifact receipts and different executable
paths/SHA-256 prevent the prior shared-target reuse. Dependency copies preserve
bytes/modes without links or original-cache writes and exclude supervisor units.
Rust 1.94.0 and MSRV 1.88.0 each pass strict release lib/tests clippy and two
native clock/semantic controls per role before measurements.

One nonroot WSL runner owns build, method freeze, profiles and postflight. The
build/control phase precedes committing the final instrumentation; it is not
represented as a clean frozen build. Compiled probe/driver/helper identities
are verified against the method SHA before measuring from a clean checkout.
All task-owned builds/source checks finish before profiles. Each child alone
is pinned to CPU 0, the first CPU in the original allowed mask 0–7; host affinity,
governor, IRQs, clocks and services are unchanged. Volatile load/pressure/cgroup
context is retained without filtering data. Stable metadata matches before and
after, including boot `9895e318-cf33-4617-afec-bb6a1e39672e`, kernel
6.6.87.2-microsoft-standard-WSL2, UID/GID 1000, CPU identity, taskset/configuration
hashes, affinity and qualification. This is a new CPU cohort, not the previous
allocation cohort's boot or a continuation of a memory series.

Each compiler executes seven fresh-process pairs in BC, CB, BC, CB, BC, CB, BC
order, serially. All 28 processes pass all 12 cells. Eight phased cells match the
previous seed 74 signed synthetic history: external latch at depths 1/7/31/127,
and validation/binding/transition/RecordCreated failures at depth 7. Each counts
32 full inspections, without added warmup, and checks original FD/stamp/raw
identity, canonical corpus, logical outcome, scratch eviction and sticky refusal.
At 127, successful publication to 128 is separate. Four further cells measure an
uninterrupted owner lifecycle at depths 1/7/31/127: open, 32 inspections, publication
to 128 only at 127, latch, sticky inspection and drop, without inner timers,
hashing, scratch inspectors or JSON. Across the cohort there are 10,752 timed
warm inspect calls; sticky checks and internal publication/reopen validations
remain additional unchanged work.

Lifecycle timing excludes fixture-history production, policy signing, tree
unlink, process startup/teardown and reporting. It is not complete original-owner
preparation, enrollment, authenticated manager delivery, watchdog/writers or a
product request path. No positive production end-to-end claim follows.

## Primary results did not clear the fixed practical boundary

CPU is CLOCK_THREAD_CPUTIME_ID on the test thread; elapsed is
CLOCK_MONOTONIC_RAW. Values below are milliseconds, separately by compiler.
Role columns are marginal medians; the ratio is the median of seven matched
installed/baseline ratios, not the ratio of those marginal medians.

| Depth 127 metric | Compiler | Baseline median ms | Installed median ms | Median paired ratio | Paired interval |
| --- | --- | ---: | ---: | ---: | --- |
| Warm CPU per inspect | 1.94 | 1.549 | 1.402 | 0.888 | 0.764–1.458 |
| Warm CPU per inspect | 1.88 | 1.338 | 1.161 | 0.733 | 0.530–1.193 |
| Uninterrupted lifecycle elapsed | 1.94 | 63.859 | 66.724 | 1.053 | 0.554–1.375 |
| Uninterrupted lifecycle elapsed | 1.88 | 74.092 | 68.714 | 0.925 | 0.520–1.677 |

Intervals are the fixed min/max order-statistic interval for the population
median of seven IID pair ratios: 98.4375% nominal coverage. Shared WSL/host
interference may violate IID assumptions; the full raw ratios remain available.
A local descriptive benefit required the entire interval below 0.95 on both
compilers, and a cost required it above 1.05 on both. Neither primary signal
passes either boundary. These are exploratory attribution boundaries, not
modified D3, native-regression or release thresholds. The intervals also do not
establish equivalence within five percent. Favorable medians cannot replace them.

## Phase attribution preserves cold construction and durability costs

Supporting depth 127 medians are below. These are different phased/cycle cells,
not additive components whose marginal medians reconstruct a cycle median.

| Phase metric | Compiler | Baseline ms | Installed ms | Median paired ratio | Paired interval |
| --- | --- | ---: | ---: | ---: | --- |
| Reopen CPU | 1.94 | 2.783 | 3.961 | 1.365 | 0.946–2.400 |
| Reopen CPU | 1.88 | 2.643 | 2.423 | 0.931 | 0.712–1.069 |
| Publication CPU | 1.94 | 5.192 | 6.046 | 1.212 | 0.607–2.474 |
| Publication CPU | 1.88 | 4.840 | 5.067 | 1.033 | 0.853–2.511 |
| Publication elapsed | 1.94 | 19.488 | 23.500 | 1.112 | 0.515–1.455 |
| Publication elapsed | 1.88 | 12.967 | 17.634 | 1.228 | 0.430–2.481 |
| Complete lifecycle CPU | 1.94 | 50.707 | 59.994 | 1.089 | 0.757–1.431 |
| Complete lifecycle CPU | 1.88 | 59.814 | 57.323 | 0.909 | 0.655–1.597 |

Repeated inspections are the largest measured CPU phase: 32 times the per-inspect
warm values substantially exceeds reopen or publication CPU in this workload.
Publication has a larger elapsed than CPU median, ru_oublock=16 and generally
six voluntary switches in both roles. This supplemental counter is not a count
of write/flush syscalls. The observation is consistent
with waiting around the unchanged durable path; these counters do not isolate
fsync/device latency from all other waiting. They do not authorize removing a
sync, reducing validations or altering acknowledgement order.

Scratch eviction also has a visible terminal cost. After publication to 128,
external-latch CPU medians are 1.246 versus 6.862 microseconds on 1.94 and 0.975
versus 5.372 on 1.88. These include measurement overhead and are not independent
percentage claims. Drop CPU medians are about 0.147/0.148 ms and 0.140/0.133 ms.
The uninterrupted lifecycle includes latch and drop, so reuse is not evaluated
by simply moving final cleanup outside its interval. All five refusal paths
still clear scratch; all phased original object/corpus assertions pass.

The remaining depths do not provide a consistent cross-compiler warm signal.
Median warm CPU ratios at depths 1/7/31 are 0.981/0.990/1.039 on 1.94 and
1.021/1.038/0.870 on 1.88. These supporting cells are descriptive, not alternative
primary metrics selected after seeing the depth 127 result. No pooled number
hides cold, publication, cleanup or compiler-specific negative observations.

## Clock overhead and local variability

All measured processes report 1 ns resolution for both clocks; this is not a
claim of nanosecond accuracy. The 112 empty
brackets per compiler have CPU ranges 366–1,818 ns on 1.94 and 359–1,911 ns on 1.88;
elapsed ranges are 368–3,250 and 357–3,276 ns. No overhead is subtracted. Warm
values are averages of 32 calls, not individual-operation p50/p95/p99. Tiny sticky
refusal timings are close to empty-bracket scale and support no speedup claim.
RUSAGE_THREAD user/system CPU, switches and block-I/O remain supplemental; they
have different granularity and scope from the thread clock and host CPU usage.

Depth 127 warm batches record zero voluntary switches and input/output blocks,
but involuntary switches range 2–8/2–6 on 1.94 and 1–5/1–4 on 1.88 (baseline/installed).
Zero block counters do not prove zero filesystem cost: fresh opens, reads,
metadata and parsing still execute. Wide paired ratios remain even with fixed
child affinity. The before/after context alone cannot identify their cause.
Do not attribute the spread exclusively to scheduling, frequency, virtualization,
the clocks or a product regression without another separately planned study.

## Evidence and preserved boundaries

All first builds/controls and all 28 first measured processes pass. No cell,
outlier or compiler is discarded and none is retried into success. Seven offline
controls pass on Windows and Linux; all 23 current/historical source controls,
Rust formatting, four native release builds/lints and eight native clock/semantic
controls pass. This is not a full workspace or release qualification run.

The [packet](local-runs/authority-runtime-cpu-20261022/) seals 183 members plus
inventory.json, SHA-256
`4b86d85c55d0f2ebe36780547fcd27103de8b5368a06e623f7fb1543030b7137`.
All 184 files match staged raw Git objects. Sources, dependency-copy inventories,
image hashes, compiled inputs, all 46 command receipts, raw timings, calibration,
metadata and derivation are retained. All 429 allocation-predecessor members
and its inventory remain unchanged.

Offline verification does not execute a measured image:

```text
python -B tools/authority-runtime-cpu-074/seal.py docs/testing/performance/0.74/local-runs/authority-runtime-cpu-20261022
```

The [allocation/retention finding](authority-runtime-allocation-20261022.md)
still establishes fewer warm allocation requests at a larger retained-layout
cost. The present CPU/elapsed data do not establish its practical speed value.
No runtime change, rollback or new performance acceptance is made from these
observations. A more discriminating local method must be preregistered separately;
do not tune clocks, thresholds or the existing validation graph to obtain a win.

Original live check count/order, 300/60/10 windows, first Causal refusals
10042390247/10010835739 ns and qualification raw SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`
are unchanged. No live preparation, clock/service change, server, installation,
supervisor or product workload occurred. Authenticated original delivery,
all-writer termination, later lifecycle cells, real SIGKILL/lost-ACK, reconnect
and production enrollment/server admission remain separate gates.

## Documentation follow-up

Doc-check, the 186-gate registration check, the local non-promotable performance
contract, seven offline controls and read-only verification of this packet pass.
The predecessor verifier additionally confirms the earlier allocation packet and
all 364 runtime-predecessor members. [Follow-up receipts](local-runs/authority-runtime-cpu-doc-followup-20261022/)
are separate from the immutable measurement packet. No native/live image is
rerun for documentation; no full workspace/release qualification is implied.
