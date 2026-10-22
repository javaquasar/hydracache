# Offline CPU and lifecycle variability analysis

Analyze only the sealed first CPU cohort from method
12d158427322d3e58c3e06a79f3038d95dec36ec. This is a post hoc diagnostic analysis,
not a new experiment or a replacement acceptance rule. Inherits R-3, R-5, R-7
and R-8. No runtime, historical reader, evidence, threshold or workload edits.

## Fixed input and questions

Require CPU inventory SHA-256
4b86d85c55d0f2ebe36780547fcd27103de8b5368a06e623f7fb1543030b7137,
all 183 members, frozen reader bytes, all 46 command receipts, 28 processes,
12 ordered cells per process and the allocation predecessor. Recompute and
compare the original facts and both primary dispositions without changing them.
Verify the earlier allocation/runtime chain separately. Never execute any image.

Retain all seven pairs per compiler. For each recorded phase or uninterrupted
cycle report raw seven-element cost vectors, min/median/max, median absolute
deviation (unscaled MAD) and max/min where defined. Keep warm batches intact;
dividing by 32 is for presentation, not individual-operation quantiles.

Separate CPU variation from the signed elapsed-minus-thread-CPU residual within
each bracket. Report sample variances and covariance satisfying the arithmetic
identity Var(elapsed) = Var(CPU) + Var(residual) + 2 Cov(CPU,residual).
This residual is not a measurement of scheduler wait or storage latency: clock
sampling is sequential and covers only the measured thread. Keep negative values.
Report user/system rusage, switches and block counters separately, including
their discrepancy from the thread clock. Do not subtract empty-bracket costs.

For each compiler and role, describe Pearson and tie-aware Spearman associations
between CPU/elapsed, residual/involuntary switches, residual/voluntary switches,
and CPU/pair index. Each has only seven observations. Report null for a constant
vector, not a fabricated zero; no significance, causal or pooled inference.

For the two original primary metrics show every pair and diagnostic strata:
baseline-first indices 0/2/4/6, installed-first 1/3/5, early 0/1/2, middle 3,
late 4/5/6. Membership is fixed here; strata never replace seven-pair acceptance,
exclude extremes or license reruns. Pair ordinal is not a measured timestamp.

Within the same depth-127 phased fixture only, sum its measured CPU brackets and
report per-observation phase shares and covariance contributions to variance of
that sum. The warm term is the full 32-call batch. These are interrupted measured
brackets with untimed gaps, not the independent cycle fixture or its decomposition.

Compare the largest observed empty CPU/elapsed bracket with workload scale as
a diagnostic, not a bound on clock error or a correction. Retain the before/after
pressure context without allocating it to individual cells. Identify what the
saved data cannot distinguish: frequency/virtualization, scheduler interference,
cache/order history, filesystem syscalls or fsync latency. Do not manufacture
per-cell chronology from file timestamps.

## Output and next method

Publish a new immutable derived packet with raw vectors, formulas, frozen input
digests, analyzer/test source and this plan. Verify inputs before and after.
Offline negative controls cover pairing, membership, signed residuals, zero
variance, ties, covariance identities, corpus drift and byte tampering. Retain
the first failures, including analysis-tool failures, separately from final checks.

Prepare a separate future local timing protocol with fixed balanced scheduling,
sample/stop rules, per-cell context and distinct profiling/control paths. Do not
implement or run that experiment in this step. Preserve all live predicates and
their order, 300/60/10 windows, first Causal refusals 10042390247/10010835739 ns and
qualification SHA-256 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
No repeat profiles, live attempts, WSL actions, clocks, services, server,
installation, supervisor or product workload.
