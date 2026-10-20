# Local held launcher and pinned worker exec

Implement the approved minimal topology after 2a86c1fd. Inherits docs/RULES.md
and the [purpose-bound build plan](held-build-authority-plan-20261020.md).
The new binary is local mechanics, not an installed original-start route.
Keep all old runtime files/46 brackets and original 300/60/10 origins unchanged.

## Descriptor and protocol boundary

Add diagnostic_held_launcher with a private out-of-line Linux x86-64 kernel
module; unsupported platforms refuse. No public library execution API, caller
command, pathname exec, installation path or old unit/ExecStart change.
stdin/stdout/stderr are distinct inherited pipe ends. Fixed descriptors are
3 control SOCK_SEQPACKET, 4 worker, 5 config, 6 owner pidfd, 7/8 guardian pidfds,
9 existing absolute CLOCK_BOOTTIME timerfd and 10 sealed bootstrap memfd.
Close all higher descriptors and mark private roles CLOEXEC before preparation.
Only config FD 5 and stdio survive successful worker exec.

Bootstrap is 256 fixed little-endian bytes (HCHL0741), pinning deadline, exact
owner PID/UID/GID, both guardian PIDs, nonzero token, SHA256 worker/config bytes,
their device/inode/length and stdio/control device/inode. Reserved bytes are zero.
Require read-only descriptors and four immutable memfd seals for bootstrap/config.
Worker is either sealed immutable ELF bytes or a root-owned non-group/world-
writable executable regular file. Root remains trusted in the latter case;
this is not independent root/file-system custody or a new worker-copy authority.
Only local synthetic sealed ELF is executed by this package's fault tests.

Use HCLR0741 fixed 80-byte Ready/Release packets: kind, zero reserved bytes,
token and bootstrap SHA256. Require connected AF_UNIX SEQPACKET, SO_PEERCRED and
exact per-message SCM_CREDENTIALS for the owner. Reject truncation, unknown
versions/kinds, extra rights and additional packets; close delivered unexpected
rights. Ready is preparation only, never watchdog Armed or exec confirmation.
Reject root/set-ID helper execution; set no_new_privs. No environment or caller
argv is forwarded. Worker argv is the fixed timing-controls --run protocol with
/proc/self/fd/5, expected worker digest and unchanged frozen worker source.

Bracket existing alarm remaining-time/deadline with CLOCK_BOOTTIME reads and
check owner/guardian pidfd identities/liveness before Ready and after Release.
Never consume/reset/rearm the timer. Recheck immutable content, stamps, pipe ends,
credentials, deadline and guards before execveat(4, empty, AT_EMPTY_PATH).
No success ACK exists: channel EOF is uncertain and never enables a resend.
The future original adapter must consume its release latch before the syscall.

## Tests and evidence

Retain baseline build-authority tests/packet verification and a missing-target
compile refusal first. Add pure packet/bootstrap mutation unit cases and an
ignored, registered nonroot local process matrix with exact source/image/recipe.
Use a tiny synthetic ELF worker (not HydraCache) which stops at a kernel signal
boundary after writing one marker. Verify same PID/start ticks and changed exe
identity, only config/stdio retained, no pre-release marker, lost confirmation,
real SIGKILL before/after exec, owner/guardian loss, expiry, foreign same-UID
sender, malformed packets/rights, content/FD substitution and failed exec.
The dummy remains blocked on its inherited stdin so owner loss can be tested
without pretending the helper supplies post-exec all-writer termination.
Use deterministic enumerated fault cuts and log the replay seed; the existing
cluster fault injector models partitions/zones, not pidfd/exec process cuts.

An exact additive adapter removes only the new COMPAT/registry blocks, preserves
all 438 preceding runtime files and allows exactly the two new named sources.
Keep existing worker schemas, locks, qualification digest and sealed packets.
Run GNU 1.94/MSRV 1.88 process/pure checks and strict lint, Windows fallback/build
regressions, source guards, scoped fmt, registry/docs and local contract checks.

Original policy/epoch, role receipts/retained objects, full frame, active collector
handoff, watchdog arming/all-writer cessation, durable completion/later cells and
same-attempt reconnect remain separate joins. This package performs no production
enrollment, server access/install, supervisor operation, original/system-bus send,
product workload or expensive qualification.

## Local implementation and remaining join

25888b65 adds the real target and private kernel module without changing any of
the 438 preceding runtime files. The original complete 26-case cohort passes,
but a subsequent O_PATH stdin regression exposes an access-mode alias: a path
descriptor can look read-only without being a readable pipe end. ab47ffc5
strengthens just that mask. Its added argv oracle initially fails to compile;
68b71834 fixes the test hex formatting and is the selected accepted source.

The final matrix adds that refusal as case 27 and records actual PID/start ticks,
pre/post-exec device/inode, complete argv, empty environment and retained FD set.
Its [evidence packet](local-runs/held-launcher-20261020/README.md) keeps the earlier
cohorts, failed regression and compiler refusals separate from the final runs.
No deadline is widened or refreshed. Bootstrap, Ready/Release and fixture-only
owner-loss carrier are registered independently in COMPAT.

Next bind actual helper/host build production and retained role objects to the
same original policy/epoch, fixed root, manager channel and full frame. Only that
join can admit the held projection/release. Guardian arming, active collector
handoff, post-exec all-writer cessation, durable completion/later cells and
same-attempt reconnect still require their joined fault gates before server use.
