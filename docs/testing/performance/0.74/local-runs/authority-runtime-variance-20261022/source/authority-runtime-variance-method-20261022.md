# Future local authority timing variability protocol

Status: prepared, not implemented or executed. This is a new cohort protocol,
not permission to repeat the first cohort or replace its inconclusive result.
Inherits R-3, R-5, R-7, R-8 and R-10. The first CPU cohort and its thresholds,
allocation evidence, first Causal refusals and qualification remain immutable.

## Purpose and fixed sources

Distinguish runtime CPU cost from execution-order sensitivity and elapsed-only
variation on the same local WSL host. Baseline remains
33c54b0cda8c330b115009cdf092ad002077cedf; installed runtime remains
66d5f371680d085a24de8f2558e1f049c71b9e7b. A changed source needs a new protocol.
No optimization candidate is combined with this measurement experiment.

Before execution, freeze and review an independent runner/probe and exact
schedule digest. Verify complete overlays, eight permitted role differences,
root Cargo manifests/lockfile, includes, default features and unwrapped System
allocation. Build four fresh distinct release images in separate role/compiler
targets with offline/locked Rust 1.94.0 and 1.88.0. Require fresh:false build
receipts, source/image hashes, check, strict clippy and clock/semantic controls.
No old allocation/model executable or live entry point may execute.

## Balanced finite schedule

Use eight four-process blocks per compiler, 32 observations per role/compiler,
128 measured processes total. Each process runs the unchanged 12-cell probe,
32 inspections per warm cell, seed 74 and the exact signed corpus/semantics.
Four blocks use B I I B, four I B B I. Fix block order to
BIIB, IBBI, IBBI, BIIB, IBBI, BIIB, BIIB, IBBI. Alternate compiler block order:
1.94 then 1.88 for even block indices, 1.88 then 1.94 for odd indices.
Execution remains serial; builds and other task-owned checks finish first.
No added warmup, adaptive iterations, shuffled live checks or case deletion.

This protocol deliberately preserves the original cell order. It controls role
position and linear drift within a block, not cache/order dependence between
the 12 cells. Testing cell order or process-per-cell would change that experiment
and requires a separate protocol; do not silently fold it into this one.

Pin only owned children to the first CPU in the original allowed mask, recording
mask and selected CPU. Never change host affinity, governor, IRQs, clocks,
services or permissions. One runner session owns metadata through postflight;
record boot, namespace, kernel, UID/GID, configuration and qualification. A boot
or source/identity mismatch refuses the cohort, retaining its completed prefix.

Each child has a fixed 300-second process timeout; a single entire cohort has
a fixed 60-minute measurement budget after builds. Neither changes production
300/60/10 admission windows. On timeout or semantic/provenance failure stop,
retain the first refusal and every completed measurement. No replacement cell,
outlier retry, favorable restart or extension into a new budget. A complete
cohort is mandatory for the planned summary; a partial cohort is diagnostic only.

## Read only context and timing

Retain raw wall/thread CPU/rusage counters and eight uncorrected empty brackets
per process. Parent-side before/after each child record monotonic raw timestamps,
loadavg, PSI CPU, readable cpu.stat/cpu.max/cpuset and per-CPU /proc/stat. Record
observed CPU MHz/scaling frequency if already readable, with collection spans;
these are samples, not a frequency trace. Missing optional context is explicit,
not permission to enable perf, change sysctls, require root or discard data.
Capture parent process start/exit boundaries separately from in-cell brackets.
No file mtime is a substitute for an observation timestamp.

The original inner probe is the unmodified timing control. Context collection
occurs outside child cell brackets but can still perturb caches; retain its
duration and keep collection identical for both roles. Do not call CPU PSI a
per-cell scheduler trace, ru_oublock a syscall count, or elapsed-minus-thread-CPU
pure waiting. Both clocks sample sequentially; preserve signed differences.

If phase attribution inside a continuous cycle is later needed, use a separately
frozen diagnostic probe with explicit inner timestamps and a matched untouched
control cohort. Report observer overhead before interpreting that profile.
It is not part of the 128-process schedule and is not silently authorized here.
The current phased fixture cannot causally decompose another cycle occurrence.

## Estimator and stopping rule

Primary endpoints remain depth-127 warm batch CPU per inspect and depth-127
uninterrupted authority lifecycle elapsed. For each block and endpoint compute
exp(mean(log(installed observations)) - mean(log(baseline observations))).
Use the median of eight block ratios separately per compiler, retain all eight
and their min/max interval. The order-statistic interval has nominal 99.21875%
population-median coverage only under IID continuous block ratios; dependence
on a shared host can invalidate that interpretation. Report descriptive ranges
regardless. No pooling between compilers, endpoints or old/new cohorts.

Keep the original five-percent descriptive boundary: clear local benefit only
if every block ratio is below 0.95 on both compilers for that endpoint; clear
cost only if every ratio exceeds 1.05 on both. Otherwise report mixed/no clear
practical effect, never equivalence. This is not a changed release gate. Always
show cold open, publication, refusal/drop and system/user/switch/I/O counters,
including costs, without promoting a favorable supporting metric to primary.

One complete cohort is the stopping rule, not achieved significance, narrower
intervals or a favorable median. If still noisy, report the method insufficient
on this host and stop; do not collect more until a separately reviewed plan.
Context associations are descriptive and do not remove or reweight observations.

## Admission boundaries

The endpoint is the synthetic authority-registry lifecycle, not complete
production preparation or product throughput. It excludes fixture generation,
signing, reporting, tree unlink and process startup/teardown as before. All live
checks and original deadlines, first refusals 10042390247/10010835739 ns and
qualification SHA-256 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc
stay fixed. No server, installation, supervisor action, product load, real
SIGKILL/lost-ACK, original-start or production enrollment is admitted.

Next authorization must explicitly request implementation/execution of this
future cohort. The current offline-analysis step does neither.
