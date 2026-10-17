# Original fence custody prerequisite

Baseline 09b437435f6b2e2fac0ed9df475480c7e7c7910c has a private autonomous
guardian, but its generic descriptors do not authenticate original ownership.
The original Fence explicitly unlocks its open-file description on drop. A
duplicate inherited by a guardian therefore cannot currently protect against
ordinary original-owner destruction.

## Scope and acceptance

Preserve the default explicit unlock, including short observation and failed
construction behavior. Add a private one-shot custody transition on the original
Fence only. It brackets a duplicate of the already locked original description
with the unchanged fence inspector. Only after both checks succeed does the
original owner irrevocably select close-only destruction. The private custody
owner also closes without explicit unlock. No caller FD, pathname, constructor,
raw descriptor export, start, send, timer or recovery API is added. Repeated
custody acquisition refuses. Before this transition ordinary drop stays exact.

Keep all original live inspectors and 300/60/10-second origins unchanged. This
step does not move sinks or connect guardian/start dispatch. It isolates the
fence ownership prerequisite so the existing pre-execution guards can retain
their original descriptor while a future guardian retains the same description.
Any later enrollment must additionally bind that owner to the original full
validation frame, sinks, controller, remaining absolute bound and complete
cgroup revocation. Private custody alone grants none of those authorities.

## Tests before implementation

Add tests on the existing original Reserved fixture, not a replacement lock:
ordinary drop still unlocks even with a retained alias; custody survives original
owner drop; custody-first drop leaves original revalidation healthy; repeated
acquisition refuses; a changed named lock refuses without adoption. First retain
the failing custody survival result against the old destructor, then implement
only the conditional destruction transition.

A separate ignored nonroot process matrix transfers the private custody
description into an isolated raw-fork holder. Block signals before fork and
restore the exact parent mask; the child performs only raw bounded syscalls,
isolates inherited descriptors and never runs Rust destructors. Exercise explicit
parent close, omitted and disconnected readiness ACK, and actual holder SIGKILL.
Use independent flock acquisition and pidfd/wait status, never a PID-only
liveness claim. The test holder does not represent the controller, collector or
authenticated production guardian. Killing the last holder must release the
kernel flock; durable-history exclusion remains a separate obligation.

Run focused original fence tests and static contracts, both Linux compilers'
all-target check/strict all-feature Clippy, formatting and local documentation /
performance-contract checks. If practical repeat only this synthetic matrix in
an isolated nonroot dedicated-host scratch directory, retaining pre/post guards.
Do not install a release, mutate host clocks, stop the supervisor, execute a
product workload or admit a pilot. Commit code separately from evidence and
article conclusions. Existing evidence stays immutable; new static exceptions
must match exact declared edits and independently reject unrelated mutations.
