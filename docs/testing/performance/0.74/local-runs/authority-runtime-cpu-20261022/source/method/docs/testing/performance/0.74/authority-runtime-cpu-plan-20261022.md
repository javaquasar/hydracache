# Installed authority runtime CPU and elapsed attribution

Compare the actual installed owned-storage authority engine against its exact
pre-install baseline after the allocation/retention experiment. Inherits R-3,
R-5, R-7, R-8 and R-10. This local attribution is not product qualification,
original-owner preparation or production end-to-end admission.

## Source and instrumentation boundary

Starting checkout is 32ba3f5a. Baseline source remains
33c54b0cda8c330b115009cdf092ad002077cedf; installed runtime source remains
66d5f371680d085a24de8f2558e1f049c71b9e7b. Verify the same eight changed supervisor
files, identical root manifests/lockfile, full package membership and bodies.
The repository runtime and existing tests are not edited. Add one identical
test-module hook/probe to isolated source overlays, with only a read-only
scratch inspector differing by role outside measured intervals.

Use default System allocation: no allocation-counter wrapper, profiling feature,
old allocation/model executable, live helper, service or manager transport.
Build new release libtest images for Rust 1.94.0 and MSRV 1.88.0, offline/locked,
with -Dwarnings and separate compiler/role targets. Reuse only byte/mode-verified
dependency copies through the frozen allocation harness; exclude supervisor
artifacts/fingerprints and require fresh:false, distinct paths and digests.
Keep first build/control refusals. Build and run clock/semantic controls before
committing the final method; verify compiled inputs against that method before
the first measurement, with a finite freeze wait and no measured-cell retries.

## Fixed paired cohort

One nonroot local WSL runner owns metadata, builds, freeze, profiles and postflight.
After all builds/controls and local source checks finish, execute seven fresh
pairs per compiler in BC, CB, BC, CB, BC, CB, BC order, serially without competing
builds from this task. Pin only each owned test child with taskset to the first
CPU in the runner's original allowed affinity mask; record the full mask and
chosen CPU. Do not change host affinity/governor, IRQs, clocks or services.
Record read-only load/pressure/cgroup context before and after, without using it
to discard data or trigger retries. Stable guards cover method/source, boot,
kernel, affinity, UID/GID, configuration and qualification; volatile load stays
separate and is not required to be identical.

Use synthetic signed policy seed 74 and the previous canonical corpus/history
producer. Preparation and signing are outside measured intervals. Eight phased
cells match the prior experiment: depths 1/7/31/127 with external latch and depth
seven validation/binding/transition/RecordCreated errors. Each reopens actual
files, then performs 32 full inspections with no extra warmup. Preserve every
runtime check and operation order, original object identity, sticky refusal and
scratch cleanup. At depth 127 separately measure successful publication to128.

Add four uninterrupted cycle cells at depths 1/7/31/127: one timing bracket around
open, 32 inspect calls, publication to128 only at127, external latch, sticky
inspection and owner drop. There are no inner timers, JSON, corpus hashing or
scratch inspectors in that bracket. Outcome assertions remain common to both
roles; input corpus is verified during preparation and the phased control path.
This is an authority-registry synthetic lifecycle, not the full original-start,
enrollment, manager/watchdog, controller or product workflow. Every pair must
match corpus, depth, mode, seed, fixed work and logical outcome.

## Clocks and reporting

Use read-only CLOCK_THREAD_CPUTIME_ID for measured thread CPU and
CLOCK_MONOTONIC_RAW for elapsed nanoseconds. Also retain RUSAGE_THREAD user/system
CPU, voluntary/involuntary switches and block-I/O deltas; rusage CPU has coarser
resolution and does not replace the CPU clock. Record clock resolutions and
eight empty-bracket samples per measured process, outside workload intervals.
Do not subtract overhead, rescale timestamps or alter any time source. Clock
controls check valid resolution, monotonic sampling and bounded synthetic work;
correctness outcomes never depend on a performance threshold.

Report absolute per-role ranges, paired ratios and median paired ratios,
separately by compiler/phase/depth. Warm values are batch averages per inspect,
not per-operation latency quantiles. Seven independent-pair ratios yield the
preregistered distribution-free min/max interval for their population median
(98.4375% nominal coverage under independent identically distributed pairs).
WSL/host interference can violate that assumption, so retain all ratios and
describe this as local exploratory evidence, not portable precision.

Primary local signals are depth127 warm CPU/inspect and depth127 uninterrupted
cycle elapsed. A descriptive clear benefit requires the entire ratio interval
below0.95 on both compilers; a clear cost requires the interval above1.05 on both.
Otherwise report no clear practical effect/mixed evidence, including regressions.
The five-percent descriptive boundary is not a changed D3/product/release gate.
Do not pool compilers, phases or cold/warm costs; do not optimize/revert runtime
based on these data. Other metrics are descriptive, without multiple-comparison
or universal-win claims. No timing result inherits the old allocation numbers.

Seal raw receipts, first refusals, all28 fresh measurement processes, sources,
image hashes, metadata, derivation and previous packet digests. Retain existing
300/60/10 windows, live check count/order, first Causal refusals
10042390247/10010835739 ns and qualification raw SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
No clock/service change, live preparation, server, installation, supervisor,
product workload or real SIGKILL/lost-ACK campaign is authorized here.
