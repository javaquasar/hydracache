# Original process termination and cell completion boundary

Baseline is `34b3cb37a2774347cf3865af031ad3bd0e2d04c3`. This P1 package
separates a necessary kernel exit observation from sufficient cell completion.
It follows the approved [intermediate plan](diagnostic-server-readiness-intermediate-plan-20261012.md).
No server, installed root, workload or qualification is activated.

## Completion prerequisites and dependency

A completed cell must bind the acknowledged owned Starting record and its exact
signed intent/request/nonce to an authenticated original start, executable,
environment, streams, process generation and owned cgroup. A successful outcome
additionally requires original exit status, validated retained output, revocation
of every possible writer and authenticated emptiness of the original cgroup.
Only an original in-memory owner may acknowledge the durable completion
publication; a reopened record is observation data. All evidence is necessary:
no supplied boolean, signature over unproven assertions, PID disappearance,
empty tree snapshot or stale receipt substitutes for the missing producer.

Current FixedStartingIntentRead has no authenticated original-start producer.
LiveExecutionRead is sequential read-only consistency and TreeSnapshot.empty_at_read
is not writer revocation. The original intent guard remains anchored to revision
one/cell one and cannot be rebound for later cells. Therefore P1 authenticated
completion depends on the P3 original-start/writer-revocation producer and P2
trusted admission. Publishing Running/completed state now would fabricate
authority. Keep later-cell publication closed while implementing the missing
kernel observation below. This prerequisite ordering is within P1 through P3,
not a new execution policy or permission.

## Strict original pidfd observation

Add a read-only ProcessRead.observe_terminal method using only its retained
zero-flag process pidfd, original boot and procfs root. Do not accept a PID,
descriptor, asserted terminal status or replacement generation. Require exact
procfs origin and boot before and after a single nonblocking poll. On no event,
revalidate the existing original live generation. Only exact POLLIN, POLLHUP or
their union with a single ready descriptor report terminal observation; errors,
unexpected bits, inconsistent ready counts and drift refuse. Terminal results
contain the original generation and Exited/Reaped readiness data, not an exit
code, successful outcome, completion guard or cleanup permission. Do not read
a replacement /proc/PID after exit, wait/reap, signal, reopen or write anything.

The [Linux pidfd manual](https://man7.org/linux/man-pages/man2/pidfd_open.2.html)
distinguishes readable exit from hangup after reaping; a zero-flag pidfd observes
the process thread group, not descendant processes. A surviving descendant can
still write after the leader exits. The API must not turn that observation into
release permission.

## Preregistered tests and local gate

Capture the existing process-reader baseline before adding tests; retain a
missing-method red test before implementation. Fast tests cover all 65536 poll
masks, inconsistent counts, sequential origin checks, every read/poll error,
late boot drift, live-generation refusal, actual owned graceful exit/reaping,
actual owned SIGKILL and replacement exclusion. Compile-fail examples reject
constructing an observation or converting it into completion/cleanup authority.

Add a separate ignored manual nonroot Linux proof: retain pidfds for a Python
leader and its exact owned descendant before killing only the leader. Require
signal-nine exit, terminal original pidfd, still-live descendant and an actual
post-exit stream write. Close only the owned descendant's control pipe and prove
its retained pidfd becomes terminal; RAII cleanup signals only retained owned
pidfds if normal cleanup fails. Three repetitions retain the same source.
No cgroup ownership or global writer cessation is inferred from this fixture.

Run affected process/credential/namespace and diagnostic integrations, current
and MSRV checks, warnings-denied Clippy/Rustdoc, scoped format, root contracts,
documentation/book and byte/hash evidence checks. Register the manual gate
with a fixed 180-second budget; bounded readiness uses 30 seconds only to
diagnose harness stalls. There is no new persistent write boundary in this
read-only slice, so it cannot supply a new cell-publication SIGKILL matrix.

## Remaining four requested steps

This package fixes the completion contract and missing termination observation,
and tests false completion through exit, replacement and continued writing.
It does not finish authenticated outcome or cells two through four. Subsequent
work must connect original-start and writer-revocation authority, implement the
owned completion/next-intent protocol, then exercise every actual new publication
boundary with SIGKILL and lost acknowledgement. Unknown/partial attempts remain
retained and cannot be retried. Real reboot, external keys/pins, installation and
the nonproduct host rehearsal remain separate gates. No frozen numerical input
or threshold changes, and no performance claim follows.
