# Offline attribution of authority CPU and lifecycle variability

The saved cohort shows variation in measured thread CPU as well as elapsed-only
cost, and warm ratios are sensitive to pair position. It does not identify one
causal source of the spread. Both original primary conclusions remain mixed;
no practical speedup, equivalence or absence of regression is established.
The next useful step is a separately balanced local study, not repeating or
filtering the existing cohort.

## Inputs and arithmetic boundary

The [offline plan](authority-runtime-variance-analysis-plan-20261022.md) retains
the complete [first CPU cohort](authority-runtime-cpu-20261022.md): 28 processes,
336 ordered rows, seven pairs per compiler, every original phase and counter.
Sources remain baseline 33c54b0cda8c330b115009cdf092ad002077cedf and installed
66d5f371680d085a24de8f2558e1f049c71b9e7b; original method remains
12d158427322d3e58c3e06a79f3038d95dec36ec. Input inventory SHA-256 remains
4b86d85c55d0f2ebe36780547fcd27103de8b5368a06e623f7fb1543030b7137.

The analyzer verifies the input before and after calculation, checks historical
reader content, reproduces all original facts and preserves both primary
dispositions exactly. No measured executable, WSL command or new profile runs.
Raw vectors, unscaled median absolute deviations, ranges, sample variance and
covariance, and Pearson/tie-aware Spearman associations are descriptive. Groups
below are post hoc diagnostics fixed by the offline plan, not new acceptance
criteria. No pair, compiler, unfavorable result or old refusal is removed.

Warm values are full 32-inspection batches unless explicitly divided by 32.
The uninterrupted lifecycle is a different fixture occurrence from the phased
measurement; its cost cannot be reconstructed from marginal phase medians.
Neither family is complete production preparation or a product request path.

## CPU variation is not only elapsed waiting

Depth-127 measured thread CPU ranges and unscaled MAD are milliseconds. Each row
has seven observations; B is baseline and I is installed. These are within-role
spreads, not installed/baseline performance effects.

| Measurement | Compiler and role | Minimum ms | Median ms | Maximum ms | MAD ms | Maximum divided by minimum |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Warm batch CPU | 1.94 B | 32.309 | 49.554 | 54.893 | 4.027 | 1.699 |
| Warm batch CPU | 1.94 I | 38.737 | 44.874 | 51.500 | 3.719 | 1.329 |
| Warm batch CPU | 1.88 B | 31.172 | 42.829 | 54.545 | 10.302 | 1.750 |
| Warm batch CPU | 1.88 I | 28.170 | 37.137 | 49.835 | 2.725 | 1.769 |
| Lifecycle CPU | 1.94 B | 38.907 | 50.707 | 64.606 | 8.726 | 1.661 |
| Lifecycle CPU | 1.94 I | 39.453 | 59.994 | 62.106 | 2.111 | 1.574 |
| Lifecycle CPU | 1.88 B | 41.031 | 59.814 | 70.123 | 4.181 | 1.709 |
| Lifecycle CPU | 1.88 I | 39.188 | 57.323 | 67.695 | 2.284 | 1.727 |

Warm CPU/elapsed Pearson associations are 0.993/0.981 on 1.94 and 0.993/1.000
on 1.88 (B/I, rounded). The variation is visible in CPU charged to the measured
thread, not just time outside that clock. This does not distinguish frequency,
kernel work, cache state, virtualization accounting or another CPU-cost cause.
It rules out treating the entire observed spread as an elapsed-only wait term.

The largest observed empty CPU bracket divided by the smallest depth-127 warm
batch CPU is 0.0044–0.0064% across these four groups. No overhead is subtracted.
That scale comparison is not an upper bound on instrumentation effects or clock
error; the empty operation does not reproduce workload/cache/scheduler behavior.
Tiny sticky/refusal brackets remain unsuitable for relative speed claims.

## Pair order and ordinal sensitivity

The original seven paired ratios are preserved below, installed divided by
baseline. Pair indices are the fixed runner order, not measured timestamps.

| Pair | First role | Warm CPU 1.94 | Warm CPU 1.88 | Lifecycle elapsed 1.94 | Lifecycle elapsed 1.88 |
| --- | --- | ---: | ---: | ---: | ---: |
| 0 | B | 0.817 | 0.688 | 0.929 | 0.930 |
| 1 | I | 0.764 | 0.733 | 0.554 | 0.925 |
| 2 | B | 0.888 | 1.163 | 1.375 | 0.934 |
| 3 | I | 0.799 | 0.716 | 1.222 | 0.918 |
| 4 | B | 1.245 | 1.164 | 0.773 | 1.677 |
| 5 | I | 1.458 | 0.530 | 1.053 | 0.520 |
| 6 | B | 1.039 | 1.193 | 1.162 | 0.831 |

On 1.88 the median warm ratio is 1.163 in the four baseline-first pairs, versus
0.716 in the three installed-first pairs. On 1.94 those medians are 0.964/0.799.
Early indices 0/1/2 versus late 4/5/6 yield warm medians 0.817/1.245 on 1.94
and 0.733/1.164 on 1.88; middle index 3 remains in the full summary. These small,
unequal diagnostic groups show sensitivity to grouping. They do not isolate
order from time, host state or runtime effects, and cannot replace the original
seven-pair estimate. In particular, the favorable installed-first group is not
a valid reason to drop baseline-first observations.

The lifecycle elapsed strata do not reproduce a uniform order signal: their
baseline-first/installed-first medians are 1.046/1.053 on 1.94 and 0.932/0.918 on
1.88. Role order is therefore a design concern for the next cohort, not a single
proven explanation for both endpoints. Compiler lanes also ran sequentially;
compiler identity and cohort time cannot be independently attributed here.

## Elapsed residual and durable publication

For every bracket define R = elapsed minus measured thread CPU, keeping its
sign. The clocks are read sequentially and rusage covers the current thread;
R is not pure scheduler waiting, fsync latency or a whole-process CPU deficit.
Negative values in small brackets remain intact.

Depth-127 publication R ranges are 5.806–23.677/6.303–23.630 ms on 1.94 and
5.948–70.537/6.555–30.301 ms on 1.88 (B/I). Lifecycle R medians are
10.977/8.770 ms and 15.523/11.258 ms, respectively. Publication keeps ru_oublock=16
and five or six voluntary switches; these do not count write/flush syscalls or
identify which sync waited. A large publication residual is consistent with
waiting around durable work, without authorizing any durability relaxation.

For the same seven lifecycle observations, the variance identity
Var(elapsed) = Var(CPU) + Var(R) + 2 Cov(CPU,R) holds. Dividing each term by
Var(elapsed) gives the following dimensionless arithmetic contributions:

| Compiler and role | CPU variance | Residual variance | Twice covariance |
| --- | ---: | ---: | ---: |
| 1.94 B | 0.236 | 0.346 | 0.418 |
| 1.94 I | 0.545 | 0.181 | 0.274 |
| 1.88 B | 1.095 | 1.431 | -1.525 |
| 1.88 I | 0.491 | 0.596 | -0.087 |

Terms above one and negative covariance are valid. They prevent an independent
percentage-of-cause interpretation; for example, baseline 1.88 has partially
cancelling CPU/residual movements. Residual/involuntary-switch Spearman values
for those four lifecycle groups are 0.546, 0.767, -0.394 and 0.668. The sign is
not uniform. Seven observations and no per-cell scheduler trace cannot establish
scheduling as the cause, even where an association is positive.

## Phased CPU sum has a different variance balance

Within each actual phased depth-127 fixture, sum only its measured CPU brackets:
open, full warm batch, publication, refusal, sticky and drop. This excludes the
untimed digest/identity checks and preparation gaps. Median per-observation warm
shares of that sum are 85.6/81.8% on 1.94 and 83.4/80.9% on 1.88 (B/I). Median
shares need not add to one, whereas each individual fixture's shares do.

Warm CPU is the largest cost, but need not dominate variance equally. The exact
Cov(phase,sum)/Var(sum) warm/publication contributions are 0.944/0.055 for 1.94 B,
0.430/0.455 for 1.94 I, 0.928/0.065 for 1.88 B and 0.723/0.275 for 1.88 I.
Installed 1.94 also has an open contribution of 0.114. All phases, including
small negative contributions, remain in the facts. These are arithmetic
covariance allocations of an interrupted phased sum, not causal explanations
of the separately measured lifecycle or a reason to omit publication/cleanup.

## Separate future method

The [corrected future protocol](authority-runtime-variance-method-v2-20261022.md) is prepared,
not implemented or run. It fixes eight four-process BIIB/IBBI blocks per compiler,
counterbalances compiler order by block and retains the unchanged 12-cell probe,
32 inspections, corpus and original checks. This is 64 measured processes in
one finite cohort, with no adaptive sample count or retry-to-pass. Block ratios
use geometric within-block role means and a separate per-compiler median; all
eight ratios and the fixed five-percent descriptive boundary remain visible.

The first prepared draft mislabeled eight four-process blocks per compiler as
128 processes and 32 observations per role/compiler. The actual schedule is
8 × 4 × 2 = 64 processes, 16 observations per role/compiler; the corrected
protocol preserves the schedule and all other conditions. The first draft and
sealed snapshot remain as preparation history. Neither version was executed.

Parent-side read-only context gains actual collection spans and process start/
exit boundaries instead of relying on cohort-edge snapshots or file mtimes.
It still cannot trace individual syscalls or identify all WSL interference.
The untouched continuous-cycle control remains separate from any future inner
profiling probe. Neither the current analysis nor the prepared protocol changes
the production validation graph, admission deadlines or release gates.

## Evidence and checks

Plan commit is 59f07cb2; analyzer/tests are e6e7126a; derived evidence is 44e7a7e8.
The [new packet](local-runs/authority-runtime-variance-20261022/) contains 13 sealed
members plus inventory.json, SHA-256
14347e695e151ccdf88866f4d3b7014c648df649f67fe827c05b4d00f4c8aa4d.
All 14 files match staged raw Git objects. The original CPU inventory and its
183 members, allocation predecessor's 429 members and earlier runtime packet's
364 members pass read-only verification.

Eleven offline controls pass: signed residuals, variance/covariance identities,
constant vectors/ties, full pairing/strata, invalid costs and nonfinite values,
historical corpus/order guards, fixed inference, byte tampering, extra members
and overwrite refusal. Two early analyzer source-guard refusals remain in the
packet: the archived reader is CRLF and the working reader LF. Only exact bytes
or a complete LF/CRLF checkout representation are accepted after the correction;
mixed line endings or any content change still refuse. Neither refusal is a
runtime result. The first raw-index check also refused log normalization; scoped
binary Git attributes preserve every sealed byte, without changing the packet.

Read-only reproduction:

```text
python -B tools/authority-runtime-variance-074/test_analysis.py
python -B tools/authority-runtime-variance-074/analyze.py --verify docs/testing/performance/0.74/local-runs/authority-runtime-variance-20261022
```

Documentation, 186 registered gates and the local non-promotable performance
contract pass; their [follow-up receipts](local-runs/authority-runtime-variance-doc-followup-20261022/)
remain separate from the sealed analysis packet. This is not native correctness,
full workspace or release qualification rerun. No measured/live/WSL/server
attempt, clock/service change, installation, supervisor action or product load.
All checks, 300/60/10 windows, first Causal refusals 10042390247/10010835739 ns and
qualification SHA-256 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc
remain unchanged. Original delivery, all-writer cessation, later lifecycle cells,
SIGKILL/lost-ACK, reconnect and production enrollment/server admission still have
their own gates.
