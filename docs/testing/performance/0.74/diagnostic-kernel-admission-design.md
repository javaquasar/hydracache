# Kernel diagnostic sender and time admission

Baseline is `5483ae1c0d37dc925c464662c1f88c8db83e150a`. This P2 implementation
replaces caller peer/time assertions on a new diagnostic input path, without
changing the campaign transport or enrolling production issuer keys. It follows
the approved [collector architecture](diagnostic-collector-design.md).

## Per message origin

Consume an original AF_UNIX SOCK_SEQPACKET connection into a dedicated receiver.
Require SO_PASSCRED and SO_PASSPIDFD, with no fallback to connection-time
SO_PEERCRED. One recvmsg joins bounded payload, kernel-validated SCM_CREDENTIALS
and an SCM_PIDFD referencing that message's sender. Unknown ancillary data,
duplicate credentials/pidfds, SCM_RIGHTS, truncation, empty/oversized packets or
unsupported socket options refuse. Every received descriptor is owned immediately
and closed on refusal, including unexpected rights. Do not expose raw descriptors
or accept caller-supplied ones. A receive with no packet is nonblocking and
performs no mutation; a malformed packet permanently refuses that receiver.

SO_PEERCRED describes connection establishment, not the process subsequently
using an inherited socket. The [Unix socket manual](https://man7.org/linux/man-pages/man7/unix.7.html)
distinguishes that from per-message credentials. The
[Linux v6.6 SCM implementation](https://raw.githubusercontent.com/torvalds/linux/v6.6/include/net/scm.h)
creates the pidfd from the retained message PID. A host must support these
options before diagnostic admission; the historical rented-host inventory names
kernel 6.8, but current capability must still be checked before installation.
Privileged senders can supply credentials allowed by kernel capability checks;
this is not an alternative to independent signed principal/issuer admission.

Keep the original message pidfd, never reopen a numeric PID after exit. Require
live pidfd readiness and fdinfo PID agreement before/after pinning the sender's
proc directory and reading bounded PID/Tgid/UID/GID/supplementary-group fields.
All four UID/GID slots must match the message credentials. Compare the original
projection on later use; replacement, exit or credential/group churn refuses.
Signatures still use independently supplied DiagnosticPolicy, but the new
authentication method obtains peer/groups and freshness time internally.
Successful signature verification alone is not durable request admission.

## Kernel clock and receiver scope

Retain the original procfs root and its boot ID plus namespace handles for the
calling thread's user, mount, PID and time namespaces. Verify procfs/nsfs origin
and retained/named identities around receipt and authentication. Obtain lease
elapsed time from CLOCK_BOOTTIME, including suspend, and signature freshness
only from CLOCK_REALTIME. Wall time never supplies epoch authority. Boot or
namespace drift, invalid timespec, boottime reversal, wall-time reversal or a
packet retained for 60 seconds refuses. No JSON timestamp, clock callback or
unverified boot string can replace this sampling.

These are kernel inputs in the original receiver namespace, not independently
enrolled host identity. Production must join the receiver namespace/time origin
with root-owned operator pins and existing authority/fence checks. Fixture keys
must never initialize a production trust anchor. The old portable model APIs
remain explicitly modeled; no compatibility projection is reinterpreted.

## Preregistered checks

Capture Unix transport and portable diagnostic-IPC baseline and a missing-method
red test first. Exercise actual parent/child senders through an inherited socket,
sender death before use, pidfd correlation, exact/overflow payload bounds,
unexpected descriptor passing with no leaks, disconnect, option substitution,
strict ancillary layouts, supplementary-group parsing, scope/clock drift,
signature/principal/freshness failure and receiver refusal. Use seed `0x0751`
for parser mutations and three same-source repeats. All owned helpers are local
nonproduct processes with a 30-second harness deadline and RAII cleanup.
Run current/MSRV checks, strict lint, package regression, format, documentation
and non-promotable performance contracts. No host service or workload is started.

After kernel admission, production trust loading and enrollment still require
operator-selected issuer public keys, exact policy digest and authority epoch.
Original start, root collector construction, autonomous watchdog, retained-cgroup
revocation, authenticated completion and later cells remain subsequent P1/P3
work. No synthetic positive result closes these dependencies.
