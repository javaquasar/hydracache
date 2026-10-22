# Actual authority runtime CPU attribution

Read the [fixed plan](../../docs/testing/performance/0.74/authority-runtime-cpu-plan-20261022.md)
before running. This is a local nonroot filesystem-fixture experiment, not a
live supervisor, controller, original-start, enrollment or product benchmark.

Baseline and installed runtime sources are pinned to the previous allocation
experiment. Full source overlays preserve existing runtime/tests and add only
one common CPU test module, with a read-only per-role scratch inspector outside
timing brackets. The default System allocator is unwrapped. Allocation counters
and old model/profile executables never run.

`python -B test_method.py` runs offline negative controls, not measured images.
Format the unattached template with
`rustfmt --check --config skip_children=true probe.rs`; generated child modules
are resolved by native clippy inside the overlays.

From the exact local Ubuntu WSL workspace run `python3 -B run.py --attempt <new>`.
The runner refuses overwrite, root identity, changed helper/runtime/profile,
unreviewed environment and incomplete static includes. It creates separate
compiler/role targets, copies verified dependency files only, freshly compiles
all four libtest images and runs strict release lib/tests lint and two native
clock/semantic controls per image. Each owned child alone is pinned by taskset;
host affinity, services, clocks and system tuning are unchanged.

On `ready.json`, finish source controls, commit the method and require a clean
tracked checkout. Within 600 seconds atomically publish method.json containing
`{"source":"<full commit SHA>"}` in that attempt's target output. The runner
verifies compiled probe/driver/helper identity before any measurement. Leave
tracked files unchanged until it finishes all seven BC/CB pairs per compiler
and metadata postflight. Never retry measured cells into success.

Each fresh process emits eight phase cells and four uninterrupted-cycle cells,
using 32 inspect calls per cell. Cycle means open, inspect, optional depth 127
publication to 128, latch, sticky refusal and owner drop. It excludes history
production, policy signing, filesystem-tree unlink, process startup/teardown,
JSON and diagnostic provenance work. It does not model complete production
preparation, manager delivery, writers, watchdog or controller recovery.

Read-only thread CPU and MONOTONIC_RAW elapsed clocks are separate. Rusage CPU,
context switches and block-I/O deltas are supplemental. Eight empty brackets
per process are reported without subtraction. Warm measurements are batch
averages, not p95/p99 operation latency. Volatile host context is retained, not
used to select a favorable cohort.

`review.py <attempt-folder> --output <new-file>` checks all 46 command receipts,
28 measured processes, exact work/corpus, clocks, source/image placement and all
first outcomes. It reports every paired ratio and a preregistered min/max median
interval (98.4375% nominal coverage for 7 IID pairs). This assumption can fail on
a shared WSL host. No pooling, release gate, threshold adjustment or discarded
sample follows from the result.

`collect.ps1 -Attempt <successful-attempt>` keeps all completed CPU attempts,
development logs, generated overlay sources, fixed method/runtime archives and
raw results in a new packet. `seal.py <packet> --seal` first-publishes facts and
inventory. Later `seal.py <packet>` verifies them and all 429 allocation-predecessor
members read-only, without rerunning any image. Never overwrite an existing seal.
