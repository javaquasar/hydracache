# Q stateful acceptance boundary

Continue exact source c91c3f973c9dddd3e01b2ae9ece12ea908276aaf, without a new
optimization candidate. The immutable Q allocation packet is
853f8691d7e9e1f0d2c1143908fddaefdb36e0fd24a507cbf7ac4424a3c61a71.
Restore B, P and Q from its bytes; do not change product methods, old tests,
allocator/features, dependencies or Root. New instrumentation is test-only.

Freeze nine stream scenarios before execution: immediate finish after exact-tail
admission; continuation with roomy budget; cheap refusal then continuation;
effectful refusal then continuation; two tight continuations; three cheap
refusals then continuation; accepted multi-block input; small raw tail; and
effectful refusal without a cache. Thresholds 16/64/1000, seed 0x0741028, explicit
payload recipes, steps and budgets are fixed in stateful.rs. Original 239-case
checks remain controls, not a rerun of the previous measurement cohort.

Use one B/P/Q/Q/P/B sequence, two observations per scenario/process. Actual P/Q
decisions and complete wire bytes must match the independent immutable oracle.
B uses its original cheap bound; known unsafe B admissions are recorded and
cannot be compared as equivalent work. Require exact same-image and inner
repetition. Compare Q against B only for identical decisions/wire, and against
the safe P control for every scenario. B/P are diagnostic controls, neither is
eligible for adoption.

Record logical Rust, physical tagged-Rust and native C allocation/reallocation/
requested/peak fields with the unchanged qualified C object, and lengths/capacity
of all three writer Vecs at every step. Compression calls and cache length have
allocation-free TLS/stack storage; full wire bytes are copied into a fixed stack
trace before in-region destruction. No diagnostic Vec/JSON allocation occurs
inside the measurement. Constructor through final drop is one region; live,
external, failed and error counters must be zero. Capacity is observed separately
from allocator retention/RSS/PSS; the latter are not inferred or qualified.

Predeclare no increase in any of the twelve allocation-dominance fields or summed
retained Vec capacities at a corresponding checkpoint for comparable work.
An individual refusal ends Q integration; do not trade lower C work for greater
Rust retention. Preserve Q's old positive subgate and this first result. No CPU,
MSRV, Windows or rescue cohort is dispatched after a resource refusal. No new
hypothesis, Root adoption, server/product/service/clock work is authorized here.

Keep original local envelopes 1200/native 1100 seconds, physical/native four-GiB
floors, five-second polling/retirement and all existing product checks/windows.
Commands have exclusive output labels and fresh source/vendor/object/image
guards. Run prepare, format, stage, c-control, build/control B/P/Q, cohort-1..6,
then the three check/strict-Clippy pairs. Analysis is offline and never executes
archived code. All first refusals and instrumentation-only fixes remain distinct.

## Applicability boundary

These are generic writer/component scenarios, deliberately including changing
budgets and continuation after a refusal. The real DeltaSerializer uses one
private MTU and documents stop-on-false. A cached exact tail implies E+L+4 > M,
where E is committed output and L pending bytes. For any subsequent positive
item n under the same M, the old floor is E+L+n+4 or E+L+n+7, both greater than M.
It therefore returns before cache invalidation/backup. The variable-budget
cache-continuation counterexample cannot by itself establish production cost.

Keep the component gate negative if its fixed rules refuse; do not relabel it a
production regression or qualify the real caller by inference. A separate exact
fixed-MTU, stop-on-false real-caller gate remains necessary. Its question is
applicability and actual caller work, not a CPU rescue or a replacement candidate.
