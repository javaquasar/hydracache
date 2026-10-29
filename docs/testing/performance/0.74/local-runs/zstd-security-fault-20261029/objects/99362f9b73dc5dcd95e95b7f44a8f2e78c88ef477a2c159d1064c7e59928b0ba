# Opt in coordinator phase clock brackets

This diagnostic fills a boundary identified by the saved
[offline variance analysis](../../docs/testing/performance/0.74/git-cpu-variance-result-20261026.md):
process-wide coordinator CPU was recorded, but not its stage endpoints. This
stage implements and model-checks telemetry only. No native control, Git workload,
V6, server, service, clock adjustment or dependency action is executed.

## Clock and phase contract

An explicit `--coordinator-frames` on the existing fixed Git attribution CLI
selects a separate output root and the traced worker frame. Without it, the
original frame is called, no Tape is constructed and the old intent/result
shape remains. The accepted request's exclusive intent precedes the lazy import
and all preflights; import/schema failure is retained before any worker starts.
The new [protocol](coordinator-frames-protocol.toml) preserves the original
four-block schedule and 120/5-second run/cleanup origins, not a new allowance.

The six phases, in order, are:

| Phase | Original operations bracketed |
| --- | --- |
| setup | Existing entry deadline, Job construction/configuration, zero-charge check and exclusive log open |
| spawn | Original atomic Job-list worker creation, pre-resume owner observation, membership and resume |
| wait | Original bounded child wait, log close and exit/deadline check |
| terminal_reads | Original worker PID/counters/validation and two Job terminal queries/checks |
| decode | Original log-size check/read, protocol decode, result join and optional descendant subtraction |
| close | Original child/Job handle close and owner-variable release |

Each begin/end records wall-before, process-CPU and wall-after using
`time.monotonic_ns` / `time.process_time_ns` in the current coordinator. CPU is
process-wide across all threads, not native Git or a single API's CPU. Raw
endpoints, domains, units and zero values remain. At most twelve probes and
thirty-six clock reads occur; no resolution or precision is assumed from `ns`.

For two probes, the reported lower/upper wall bounds describe elapsed time
between their bracketed CPU-read instants, not an effect-only stage duration.
The original effects lie between the start probe's wall-after and end probe's
wall-before. Bookend recorder costs and possible scheduling are not isolated.
CPU deltas also include clock bookends and any other coordinator threads. Gaps,
Tape construction/export, sample-file appends and error cleanup are not a
complete frame partition. The existing whole-frame metric endpoints and
estimator stay untouched; recorder overhead remains unaccepted.

## Faults and original custody

The first ordinary clock/type/regression/order fault freezes telemetry without
further probes or fallback clocks. The original operations and owner wait/close
continue, preserving their checks and deadlines. Raw partial probes stay marked
invalid, not converted to zero or retried. A diagnostic failure cannot become
a passing cohort or authorize another attempt.

The original operational verdict and cleanup errors are saved separately. An
operational first refusal is never replaced by telemetry; a healthy operation
with refused telemetry has a refused diagnostic verdict without requesting
termination. Fatal interrupts still use the original exception/owned-cleanup
path. Interrupted phases have no fabricated end/delta; later phases are labelled
not observed, not claimed unexecuted. Exported copies are not mutated by Tape
reuse. Error cleanup is deliberately not probed or given extra time.

The traced frame has the same operational AST as the current and sealed original
frame after removing only literal begin/end calls and final metadata attachment.
Differential models also compare calls, four original deadline reads, command
argv, healthy close and first-refusal custody. This preserves existing
observation frequency but adds diagnostic clock observations: it is not an
invisible or accepted low-overhead change.

## Remaining gate

Native clock behavior, real original-process/stdio compatibility and overhead
of the enabled mode are not established by model tests. A later bounded native
control needs a separately recorded exact source/protocol/build, existing
environment/pin preflights and first-result retention before any numerical use.
Do not apply new phase fields to old packets or repeat V6. No product cost
reduction or production admission follows from installing this recorder.
