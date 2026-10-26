# Offline transport and process capture attribution

The saved native packet now has independently replayable coordinator-phase and
CPU-owner attribution. The largest block wall increase sits mainly in the
coordinator wait bracket, while higher Job CPU belongs to worker and descendants,
not coordinator CPU. The data does not isolate pipe IO, process creation or
validation as a causal leaf cost. No runtime optimization is justified yet.

## Fixed input and arithmetic boundary

The analyzer reads the sole native packet from source
`1030acb263e96d1e023f852dceb021de458ce6f4`, index SHA256
`55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f`.
Analyzer source is `fa9b22330289ad0dacd05faf97310fa0b3b96336`.
All 159 input members are size/hash checked; individual ledgers and raw worker
records join before analysis. The first offline result succeeds with sixteen
workers, sixty-four input Git calls and zero new workload calls.

[`capture_attribution.py`](../../../../tools/xtask-package-attribution-074/capture_attribution.py)
uses only fixed bytes and the current pure CPU-owner arithmetic. The
[design](../../../../tools/xtask-package-attribution-074/capture-attribution-design.md)
keeps coordinator and nested worker timings separate. It neither imports a
retained collector nor invokes Git/Cargo, process APIs or clocks. Both reference
and profile arms already trace the coordinator: no on/off overhead comparison
is inferred. Original first outcomes, checks and 120/5, 780 and 300/60/10 origins
remain unchanged; no native rerun, V6, server, service or clock action occurs.

## Where the recorded variation appears

Block 2 has the largest original profile-minus-reference worker-frame wall
delta, +1140.3888 ms. Its coordinator wait delta is bounded by
+1051.1538 to +1051.3037 ms and decode by +96.4829 to +96.5284 ms. These are
differences of recorded clock brackets, not confidence intervals or pure
operation cost. Wait includes the child's execution and possible capture and
scheduling; decode includes file operations, parsing and validation.

The CPU-owner partition in that same block closes exactly:

| Recorded owner | Profile minus reference CPU ms |
| --- | ---: |
| Job including worker and descendants | +859.375 |
| Original worker | +375 |
| All descendants excluding worker | +484.375 |
| External coordinator | -15.625 |

The negative coordinator delta comes from spawn brackets. It neither offsets
the child's higher CPU nor proves negative observer overhead. In block 0, Job
CPU is instead 46.875 ms lower, with worker CPU 437.5 ms lower but descendant
CPU 390.625 ms higher. An aggregate alone hides this reversal of owner mix.

Across all frames, the arithmetic complement of the original whole-frame wall
metric and phase-bracket sums is bounded by 1.8843 to 3.2214 ms. The CPU
complement is zero. It is not a seventh measured phase or proof of no work
between probes: endpoints, clock bookends, gaps and unmeasured leaves remain
distinct. All original zero deltas and signed block differences are retained.

The nested worker has thirty-two profile-only per-call wall observations:

| Worker phase | Sum ms | Maximum ms |
| --- | ---: | ---: |
| Spawn | 1778.4263 | 478.8766 |
| Handle observation | 0.5385 | 0.0555 |
| Wait and collect | 4107.1967 | 564.5524 |
| Terminal observation | 0.0906 | 0.0054 |

The two observation spans together total 0.6291 ms, with 0.0122 to 0.0584 ms
per call. They include possible scheduling and are not total instrumentation
CPU. Do not add these nested durations to coordinator wait or compare missing
reference phases against zero. Worker phase CPU and absolute monotonic
endpoints were not captured; FILETIME cannot supply the missing alignment.

## Checks and remaining gate

Fourteen new controls and seventeen unchanged CPU-owner regressions pass.
They cover complete/failed input, preserved zeros, signed partitions, probe
domain/order/types/regressions, malformed spans, complements, raw object faults
and exclusive first-result custody. xtask all-target check and strict
all-target/all-feature Clippy pass. Five independent integrity controls pass,
including mutations to derived phase/owner data and missing input.

The [73-member packet](local-runs/capture-attribution-20261026/README.md) retains
the first analysis, exact sources, six source-preserving check/run receipts and
the input index plus selected raw native members. Index SHA256:
`055c7f306ad9dafb55b85a578ec788231a11f504e2117830991cdc0c5436a5b8`.
Independent replay recomputes the phase sums, complements, distributions,
per-call joins and signed CPU/wall partitions without executing the analyzer.

Verifier syntax, doc-check and the local non-promotable performance contract
also pass. The separate [final-checks ledger](local-runs/capture-attribution-20261026/final-checks/README.md)
retains both five-case integrity checks and their exact reader revisions without
changing the first analysis. Its index SHA256 is
`ae6f7b3dc03c8df53f4f60a669829173a5d0bc4c1806eda4c331c02acfb3cbbb`.

The next attribution boundary is inside the existing coordinator spawn and
decode operations and the worker's combined wait/collection path. A separate
telemetry-only design must preserve their call graph, predicates, observation
frequency and origins; these results authorize no cache, check removal or
runtime cost candidate. Recorder overhead, causal API cost, full package
coverage, C74 and production admission remain unresolved. Qualification manifest
SHA256 is unchanged:
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
