# Original custody and barred enrollment in one validation frame

Baseline is 3a12b722b8ec7847892f18bbfd0318dba8975e63. The previous
two-child owner matrix remains withdrawn: its first MSRV healthy screen refused
Challenge(Time), and the separate fixed ABBA diagnostic returned 101/0/0/0.
Neither later green screens nor this experiment replace that stopped cohort.

The approved hypothesis is that nonexecuting custody and barred enrollment can
share the existing opening and closing observations. Today custody creation
performs 17 opening and 17 closing authentication/signature calls, then barred
enrollment performs another complete 34-call frame. The candidate puts both
child enrollments between the first opening and closing observations: 34 calls
for combined construction, followed by an unchanged complete 34-call
revalidation. Removing the intermediate frame is an explicitly approved change
in observation frequency, not caching, a weaker predicate or a renewed lease.
Its runtime benefit is unmeasured until the new cohort completes.

Use a separate private owning type, not a self-referential owner plus borrowed
join. Its collector and custodian must cancel/reap before the owned original
message, pipes, sinks and final custody alias drop. Failure retains this entire
object and poisons the original attempt. Keep the old constructors, validators,
raw child implementations and generic O_WRONLY watchdog unchanged. Reuse the
exact original descriptor inventory and atomic pidfd child implementations.

Preserve every material, message, storage, custody, inventory, child-liveness,
notice, alarm and direct challenge-age check. Capture the original inventory
before enrollment and compare it after the closing observation. Observe both
children before and after that closing observation, including final deadline
checks. Mark readiness only after the closing frame and final observations.
Reserved 300 s, Starting 60 s, progress 10 s and challenge age 10 s retain their
original origins and absolute limits. No timer rearm, clock mutation or deadline
rebasing is allowed. P and B remain isolation notices, never execution permits.

Before acceptance, run fresh healthy screens on Rust 1.94 and MSRV 1.88, then
three healthy controls, reason-specific live refusal cases, actual lost-B
controls and the complete two-cut/two-death-order owner SIGKILL matrix on both
compilers. Reuse the credential-checked external driver and independently
registered parser corpus; do not reconstruct an acknowledgement. Verify exact
child FD inventories, parent mask, immutable first four origins, unchanged
records, sticky refusal and fence retention until both children are dead.
Fail the cohort at its first unexpected result; preserve it without an automatic
retry. A diagnostic attribution run is not a replacement passing gate.

Run fast units, source-freeze and mutation checks, formatting, affected
all-target checks and strict Clippy, registry/docs and the local non-promotable
performance contract. Retain exact source inputs, images, scripts and raw
outcomes before changing a failed candidate. Accept code only on its declared
green gate; otherwise withdraw only this candidate and retain the evidence.

Host tests are nonroot synthetic correctness fixtures in a new private owned
temporary directory with read-only pre/post guards. No release installation,
supervisor operation, product workload, original send/start, completion, pilot
or expensive qualification is authorized by this experiment. Whole-writer
revocation and active lifecycle admission remain later work.
