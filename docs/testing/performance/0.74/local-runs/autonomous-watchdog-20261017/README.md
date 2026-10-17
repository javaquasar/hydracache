# Autonomous guardian and writer fence mechanics

The private Linux implementation passes 36 local WSL debug cases and 72 isolated
server cases: twelve cases, three repeats per normal GNU release compiler,
Rust 1.94 and MSRV 1.88. This is a synthetic kernel correctness packet, not a
product workload, authenticated original execution or server admission.

The main cohort uses final source 9751a95c, which blocks signals in the caller
before fork and restores its exact previous mask, including deterministic
fork-failure refusal. Each process fixture also asserts successful-path mask
restoration. The separate initial-* files retain the earlier positive cohort
at 1f9e0e96; source review, not a measured process failure, found that draft's
pre-blocking handler window. Both complete matrices were rerun after correction.
The injected fork failure does not change any host process limit.

The collector and guardian are different processes. The collector starts behind
a one-shot barrier; the guardian pins its child with pidfd and closes its own
regular-file writer aliases before releasing that barrier. Only the collector
writes/fsyncs sinks. The guardian retains the flock independently and observes
the already selected absolute BOOTTIME deadline, with no rearm API.

## Cases and release oracle

Each repeat covers normal writer termination, controller SIGKILL, stopped
controller through expiry, hidden writer alias, stopped writer, omitted readiness
ACK, receiver disconnected before fork, actual sink-write EBADF, actual fsync
EBADF, kernel RLIMIT_FSIZE exhaustion, fixed-ceiling overflow, and a collector
which actually SIGSTOPs before disk work. SIGKILL is sent through retained
pidfds, except the direct still-unreaped barred-child cancellation if pidfd
creation itself fails. The tested healthy path never takes that fallback.

Ordinary controller death kills known pipe writers and permits collector drain
inside the existing bound. Absolute expiry kills controller, writer and
collector; it is an explicit failure retaining the fence, not successful drain.
The stopped-collector cases check actual procfs T state, terminal pidfd, zombie
SIGKILL wait encoding and elapsed original BOOTTIME bound while the guardian
remains alive with the flock busy. This proves process separation under SIGSTOP,
not every possible filesystem or kernel failure.

Successful release requires the collector's post-EOF/fsync/close notice, actual
successful child wait status, terminal known writers and the guardian's own two
EOF reads. Independent open-file descriptions test flock contention/reacquisition.
Exact binary stdout/stderr bytes are compared in successful cases. A hidden
writer prevents release despite known-process death. Write/sync/size/overflow
and expiry cases retain the fence until explicit fixture teardown; later EOF
does not repair the refusal. Error notices distinguish these failures. Lost ACK
does not grant retry, completion or recovered authority.

## Scope and retained checks

The original validators, grouped preparer, fence destructor and 300/60/10-second
origins are unchanged. These private fixtures select a ten-second absolute
deadline, not a substitute original kernel/controller admission chain. Production
must still transfer the original common fence and sole sinks, preserve every
live authority/process/cgroup check, enroll authenticated original start, and
revoke the complete writer tree. Private descriptor-role checks do not prove
production ownership. Guardian SIGKILL/host loss still needs durable-history
exclusion and external supervision; a flock cannot survive its last owner.

The first combined collector/guardian draft was rejected by source review before
any process run because disk I/O could block the watchdog. The separate-process
amendment was preregistered before this matrix. No measured failure is claimed
for that draft. The first static check retained one failure for the undeclared
watchdog module registration; the correction strips only that exact registration
and independently proves the old body unchanged. The final 200 contracts pass.

Four new fast units, eight original/fixed alarm units and seven selected grouped
compile-fail tests pass on both GNU compilers. All-target check and strict
all-feature Clippy pass on both; Windows check/Clippy, gated registry, workspace
format, doc-check and local non-promotable performance contract pass. No full
release verify or ship qualification is claimed.

The authorized server root is /tmp/hc074-autonomous-20261017.6GsLW5. Five pre/post
guards preserve boot, supervisor PID 8839/start/restarts 0, installed binary and
receipt digests, absent campaign/lifecycle/authority context and absent diagnostic
install. Final executable inspection finds no remaining fixture process. There
was no install, supervisor stop/restart, clock/root mutation or product workload.

Run verify.ps1 and verify.ps1 -SelfTest for the exact cohort/source/hash envelope
and ten rejection controls. The retained first verifier negative is a detector
bug: case-insensitive FAILED matched the ordinary text "0 failed". The corrected
case-sensitive detector accepts that positive and rejects an actual FAILED row;
no runtime, case outcome, deadline or raw process log changed.
verify-prior.ps1 verifies the unchanged preceding
packet in its exact archived source view without modifying the active index.
