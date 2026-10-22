# Local implementation and offline checks of the v2 timing runner

The fixed 64-process runner is implemented and passes offline control-flow,
receipt and refusal tests. No measured process was launched. This closes the
runner implementation step, not native execution validation, a new timing
comparison or server admission. The inconclusive CPU/elapsed result and the
confirmed allocation/retention tradeoff remain separate unchanged findings.

## Schedule and unchanged measured code

The runner implements the [corrected protocol](authority-runtime-variance-method-v2-20261022.md)
with its inherited conditions. Eight four-process blocks per compiler produce
64 serial processes, 16 observations per role/compiler, eight block ratios per
compiler, 768 cell rows and 24,576 timed warm inspections. Schedule SHA-256 is
1566ab8cbfa4cbdea2897737be07ddf533142ede5a6c6dbbc4f5bf36f6f638c1.
The block orientations and compiler order are fixed; missing, extra, reordered
or mistyped positions refuse. There is no sample-count adjustment, added warmup,
case shuffle, selection or retry-to-pass.

Baseline remains 33c54b0cda8c330b115009cdf092ad002077cedf; installed runtime remains
66d5f371680d085a24de8f2558e1f049c71b9e7b. The old CPU probe, source/build helper,
historical readers and package bodies are not edited. The signed seed-74 corpus,
12 ordered cells, 32 inspections, full semantics, object identity, all refusal
cleanup and publication/terminal work stay in the original probe. Input guards
verify the complete first CPU packet and qualification before a future attempt.

## Runner and review boundaries

[balanced.py](../../../../tools/authority-runtime-balanced-074/balanced.py)
owns schedule validation, source pins and the block estimator. [runner.py](../../../../tools/authority-runtime-balanced-074/runner.py)
has separate `--plan` and explicit `--execute` modes; there is no default execute
or skip-validation option. Execution requires the exact nonroot local WSL
workspace. It prepares exact overlays/includes, uses independent compiler/role
targets, inherits fresh:false and source/image guards, and builds/checks/lints
four release images with the original clock/semantic controls before profiling.

The unchanged method freeze is 600 seconds. Frozen inputs and the schedule must
match the committed clean source before any measured launch. The cohort has a
3,600-second budget, including metadata and validation; each child gets at most
300 seconds or the smaller remaining budget. A returned over-budget operation
refuses. Boot, namespaces, kernel, UID/GID, configurations, qualification,
placement, clean source and image identity are checked around each child.
These experiment budgets do not change production 300/60/10 admission windows.

Parent context has read-only collection spans, explicit unavailable optional
fields and process PID/start/spawn/exit boundaries. It remains outside the
unchanged inner probe; no fsync, inspection or durability operation disappears
from that probe. Context cannot filter/reweight a measurement or identify a
scheduler/syscall cause by itself. Child-only taskset placement does not tune
host affinity, frequency, IRQs, clocks or services.

Every attempted step receives a journal entry, including its first error and
the completed prefix. Timeout or interruption retains partial raw output and
stops; only the owned new-session process group is eligible for cleanup. A
partial directory cannot be resumed or replaced. External hard termination can
leave an incomplete journal; the reader rejects it rather than treating it as
a complete cohort or allowing repeated execution in the same directory.

[evaluate.py](../../../../tools/authority-runtime-balanced-074/evaluate.py)
requires 22 successful build/control command receipts, all 64 profile receipts,
complete ordered membership, exact work/corpus, clock/context/source/image
identity and bounded parent timelines. Failed, partial or synthetic mode refuses
numeric review. Supporting phase/counter observations remain separate; zero
counters retain differences without inventing log ratios. Geometric block ratios
and the per-compiler median/min/max estimator follow the protocol. Strict
0.95/1.05 decisions compare exact squared rational products, algebraically the
same boundary, so floating-point rounding cannot promote a boundary tie. There
is no epsilon, changed threshold, pooling or release-admission claim.

## Offline tests and their limits

Twenty new [controls](../../../../tools/authority-runtime-balanced-074/test_runner.py)
pass on Windows. The complete 64-position engine and the public preparation /
build / freeze / cohort / postflight graph run with synthetic or mocked
dependencies. Tests cover schedule/digest/count/position balance, safe CLI,
unsafe workspace/attempt refusal, incomplete-directory refusal, fixed budgets,
freeze membership/source/digest/deadline, image reuse, optional context, exact
probe overlays, geometric estimation and boundary ties.

Nine deterministic first-failure controls cover timeout, interruption, corpus,
semantic cleanup, parent timeline, image replacement, stable context, budget
expiry before launch and overrun after return. They verify that no later position
runs and that the completed prefix and first error survive. Subprocess-adapter
tests replace Popen, clock reads and killpg with mocks: success, spawn failure,
timeout and interruption check exact argv, new-session ownership, receipts,
partial output and the sole cleanup target. No real process is killed.

Reader tests reject synthetic mode and use explicitly fabricated native-shaped
receipts only inside temporary directories to exercise the positive reader path.
They test raw-log tampering, changed commands, missing inputs, bad order/budgets,
incomplete membership and extra files. These temporary receipts are not retained
as native measurement evidence and support no speed or host-behavior claim.

The initial test invocation had two assertion failures: image/deadline injection
triggered at the preceding child's closing guard instead of the next opening
guard. The test injector now distinguishes those boundaries; the runner's closing
checks were retained. The first log remains in the new packet, not overwritten
or presented as a runtime measurement failure. Seven old CPU offline controls
and eleven old variability controls also pass. Syntax/plan checks, doc-check,
registration of 186 gates and the local non-promotable performance contract pass.
No native Rust/runtime correctness or full release gate is rerun here.

## Evidence and next boundary

Implementation commit is db0ab08f; evidence commit is becfd3f9.
The [offline packet](local-runs/authority-runtime-balanced-runner-20261022/)
retains source, fixed plan, all first/final test logs, gate logs, source digests
and an inventory of 21 members plus inventory.json, SHA-256
b34d1780e2c7229fc005cfa2819973c0b5176bd25068ff5496b7ac6ae0e628ca.
All 22 files match staged raw Git objects. It records no native runtime build/profile, no WSL
command and no new performance result. The previous CPU/variance and allocation/
runtime packets verify read-only; runtime sources and qualification have no diff.

Native Linux builds, current dependency-seed availability, actual optional
context and actual kernel timeout/cleanup behavior remain untested for this
runner. A separately authorized bounded native validation should precede any
measurement; only a later separately authorized complete 64-process cohort can
evaluate whether the revised design resolves the old timing uncertainty.
No automatic measurement follows from passing these offline controls.

All live checks, original deadlines, first Causal refusals 10042390247/10010835739 ns
and qualification SHA-256 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc
remain unchanged. No clock/service change, server action, installation, supervisor
operation or product load occurred. Original delivery, all-writer cessation,
later completion cells, real SIGKILL/lost-ACK, reconnect and production trust /
server admission remain independent gates.
