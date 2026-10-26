# Coordinator phase telemetry implemented behind an explicit gate

The coordinator now has opt-in raw clock brackets around setup, spawn, wait,
terminal reads, decode and close. Model/differential checks preserve the original
operational body and deadlines. The enabled mode has not run with a real worker;
native compatibility and overhead remain unqualified. No new Git workload,
measurement cohort, server action or V6 repeat occurs in this stage.

## Implementation and caller boundary

Commit `e6ef8937297042a2cb4f45386af360db6c9c8732` adds
[`coordinator_frames.py`](../../../../tools/xtask-package-attribution-074/coordinator_frames.py),
its fault tests and [protocol/design](../../../../tools/xtask-package-attribution-074/coordinator-frames-design.md).
The existing attribution caller selects it only with the exact
`--coordinator-frames` flag, under a separate local output root. Default mode
still calls the unchanged original `measure`, constructs no Tape and keeps its
original intent/result fields. No environment activation or fallback is added.

The accepted request's exclusive intent precedes lazy import, schema validation
and all original preflights. Import/stale-schema failure therefore leaves a
first result without starting any worker. The old worker, pipe helper,
environment/image/source checks, inherited Git argv/stdio, block schedule and
cohort estimator remain unchanged. The original 120-second shared origin and
five-second owned cleanup are not renewed for telemetry or for each worker.

There are at most twelve probes and thirty-six additional clock reads per
worker frame. Each probe retains monotonic wall-before, process-CPU and
wall-after, labelled with its clock domain and ns representation. CPU is
process-wide across coordinator threads, not native Git or a single API's CPU.
Raw zeros stay; no timer precision or OS resolution is inferred.

Reported wall lower/upper bounds concern elapsed time between the two bracketed
CPU-read instants, not exact effect-only latency. Original phase effects lie
between start wall-after and end wall-before. CPU deltas include clock bookends
and other coordinator threads. Gaps, recorder construction/export, per-sample
file appends and error cleanup are not a complete frame partition. The original
whole-frame metric endpoints and estimator stay in place; tracing adds cost
whose overhead is not accepted.

## First refusals and original process custody

An ordinary telemetry fault freezes capture on its first error and prevents
further clock reads. The already-scheduled operational frame continues its
original checks, wait and close/cleanup. It cannot start another frame after
the refused diagnostic verdict. Raw partial probes stay invalid rather than
being retried, repaired or replaced with zeros.

Operational verdict and cleanup errors are retained separately from telemetry.
A clock fault does not replace an operational first refusal; a healthy operation
with bad telemetry instead has a refused diagnostic verdict without requesting
termination. Fatal interrupts after spawn still enter the original owned-cleanup
path with the original child. Interrupted spans have no invented end or delta;
later spans are labelled not observed. The native owner-loss/all-writer contract
is not certified by these model outcomes.

## Model and differential checks

All twenty new tests pass, along with sixteen unchanged caller and twenty
accounting tests: 56 cases in total. The initial seventeen-case source and first
result are retained. Coverage includes:

- Every one of 36 possible ordinary clock-read fault positions, retaining the
  healthy original wait/close sequence and four original deadline reads.
- Partial raw probes, invalid types, wall/CPU regression, unexpected phase
  ordering, incomplete schedules, capture reuse and zero CPU values.
- Original spawn/PID refusal plus simultaneous telemetry error, cleanup-error
  preservation, fatal interruption after child creation and the original late
  closing deadline.
- Default/opt-in caller separation, one shared origin across sixteen modeled
  workers, strict boolean/CLI flag, import/schema refusal before APIs and stop
  after the first refused frame without a subset summary or retry.

The AST check removes only literal begin/end calls and the final metadata
attachment. What remains matches both current `measure` and its exact sealed
original source from the old 64-call attribution. Differential execution also
compares operational calls, original metric fields, argv, wait budget and close
order on injected API/clock fixtures. This proves the checked model/source
boundary, not invisible instrumentation or native overhead.

Four Tape-only examples use injected deterministic integer clocks: complete,
clock fault, combined operational/clock refusal and operation interruption.
They contain 36, 8, 1 and 9 synthetic clock reads respectively and no process
owner. Their bracket arithmetic is independently replayable; their numbers are
not timing measurements. Python syntax/indentation checks pass. No Rust source
changed or native tests ran in this stage.

## Evidence and remaining native gate

The [40-member packet](local-runs/coordinator-frames-20261026/README.md) retains
all first model/source receipts and synthetic output. Independent verification
parses the operational AST and recomputes bracket arithmetic without executing
retained code. Four integrity controls cover index/missing-object refusal and
mutation of baseline/current source, model receipts and synthetic first errors.
Index SHA256:
`7f768549711cb4cdd5a0e2de8bcac1502951499c5a36e28f05c678dd483d2eb1`.
The separate final-checks index retains passing integrity/syntax, all-target
xtask check, strict all-target/all-feature Clippy, doc-check and local
non-promotable performance-contract receipts. These are not full-package or
native enabled-mode verdicts.

Next is a separately recorded first bounded enabled-mode native control on exact
frozen source/build/protocol, using the existing environment/pin preflights and
first-result retention. Only that can check native clock/process/stdio behavior;
overhead needs its own evidence and cannot be inferred from synthetic spans or
retrofitted into old packets. No product optimization is authorized by this
recorder. V6 and the package gate remain red; C74 and production admission stay
unresolved. All live checks/frequencies, original 780-second guard and 300/60/10
origins remain unchanged. Qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
