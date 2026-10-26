# Offline attribution of native owner and decode leaves

The saved native packet locates decode wall in the read/entry bracket and
coordinator CPU outside the two owner-observation brackets. Per-worker and
signed block analysis does not isolate a removable cost. Keep the existing
runtime, live observations and deadlines unchanged; this result supports no
optimization candidate or package pass.

## Fixed input and replay

Analyzer source is `10f160bef32c02171d42f0f8138acbbbc4d8695d`. Its sole
offline attempt reads the [first native leaf packet](capture-leaves-native-result-20261026.md),
index SHA256 `926f30f0eca6ae708b3706976f256a9ca52a3a5d7e9a9c447a8f45cec59a0855`,
native source `ef3d0cadd70e436b75be94e579a605447aaf7e2b`. All 164 objects
are size/hash checked; raw worker logs join the sixteen individual ledgers.
No process, clock, source Git read, retained-code execution or new Git workload
is invoked by the analyzer. The check wrapper's own source postflight is separate.

The result retains sixteen workers, four original alternating blocks, all
thirty-two profile-only worker call records, CPU owners, log sizes, outer
brackets, leaf brackets and arithmetic complements. Raw integer, schedule,
domain, monotonicity, parent-containment and original operational checks
remain. Each block's leaf CPU deltas plus complement CPU delta close to its
original spawn/decode parent. Missing reference worker phases stay null.
Nested worker and coordinator durations are not added.

## Signed block observations

Values below are profile minus reference sums for each original block, in ms.
Wall bounds come from bracket subtraction, not confidence intervals. Both arms
enable the same outer and leaf tracing, so these are not telemetry on/off costs.

| Block | Read wall lower to upper delta | Spawn complement CPU delta | Spawn complement wall lower to upper delta |
| --- | ---: | ---: | ---: |
| 0 | -1.5335 to -1.4695 | +375.000 | +277.1193 to +277.2457 |
| 1 | +19.4109 to +19.4594 | -31.250 | +66.9942 to +67.0756 |
| 2 | +33.9214 to +33.9822 | -781.250 | -969.9863 to -969.8699 |
| 3 | +34.1907 to +34.2376 | -343.750 | -369.3007 to -369.2334 |

Read wall totals remain 380.6253–380.8455 ms and decode complement wall
1.5567–2.1920 ms across all sixteen workers. Every recorded leaf CPU delta is
zero, not evidence of zero work. Owner-leaf subtraction leaves 2375 ms measured
CPU in the spawn complement; sign changes across blocks prevent treating a
single positive or negative block as an instrumentation cost estimate.

All reference logs are exactly 1264 bytes, yet their read brackets range from
3.7491–3.7550 ms to 32.6422–32.6642 ms. Profile logs range from 2641 to
2662 bytes. The largest read bracket, 54.0593–54.0723 ms, belongs to a
2658-byte log, not the largest log. In block 0 the larger profile logs even
have a negative summed read-wall delta. Log size alone does not establish a
causal latency model for this input. No sample is dropped or relabeled.

The largest spawn parent is reference block 2/slot 3, with
591.6810–591.6860 ms wall and 468.750 ms coordinator CPU. Large spawn
observations occur in both arms. The original spawn body still combines
argument/handle preparation, CreateProcessW, retained-child publication,
membership verification and resume. The two owner leaves cover only identity
and initial observation after retention. Their complement cannot identify
which remaining operation caused the recorded CPU or wall duration.

Read still includes open/read/close and the rows-entry boundary, with possible
scheduling and recorder costs. Worker `wait_with_output` remains opaque and
unchanged. Neither a source cache, manual pipe drain, native bypass nor removed
validation is justified by these observations.

## Checks and evidence

Ten new fixed-input/custody controls and all 105 old analyzer, collector and
accounting regressions pass: 115 combined. Controls include integer/boolean
drift, raw corruption/missing objects, refusal/incomplete input, schedule and
clock drift, shifted-but-internally-consistent groups escaping their parent,
CPU-owner regressions, log bounds and exclusive first-result retention. Syntax,
xtask all-target check and strict all-target/all-feature Clippy pass.

The first whole-workspace formatting check fails before formatting with Windows
`os error 206`, command line too long. Its original stdout/stderr and exit 1
are retained, with source and HEAD unchanged. It is not a formatting pass and
was not retried. This stage changes Python diagnostics, not Rust product code.

The [73-member offline evidence packet](local-runs/leaf-attribution-20261026/README.md)
has index SHA256
`70ba30460b8e4c995c23d3f37090b0ef849bc2915e6507e62dfc5b23a7837417`.
It retains first output/intent, seven check receipts including the formatting
refusal, exact analyzer dependencies and raw native inputs pinned to the original
index. Six independent integrity controls recompute leaf/parent/block arithmetic
without importing the analyzer or collector. Later integrity/document/contract
receipts are sealed separately, leaving the first packet unchanged.

The [final three-receipt ledger](local-runs/leaf-attribution-20261026/final-checks/README.md)
retains the passing six-case integrity gate, doc-check and local non-promotable
performance-contract check. Its index SHA256 is
`1295bed7516ff198b4cf4eb85cc435acef99e203ed0d57700624fbd2056eeebb`.

## Decision and next boundary

Offline attribution of this packet is complete. A causal creation/read or
recorder-overhead claim requires a separately bounded method that preserves
original ownership, cleanup, predicates and time origins. In particular, adding
a probe between successful CreateProcessW and retained-child publication risks
leaving a live child unowned on telemetry failure; this is not a harmless
timing hook. A method review must resolve that boundary before another control,
not infer the answer from parent-minus-leaf arithmetic.

No live cohort, V6 replay, service/clock change, server action, dependency change
or product candidate runs here. The full package remains red at its first
780-second V6 guard. Release scope, dependency/MSRV authority, compatible manager,
original delivery/all-writer retirement and subsequent lifecycle/trust gates
remain listed in the [closure status](release-closure-status-20261025.md).
Qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
