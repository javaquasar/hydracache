# Original barred collector enrollment

This packet concerns synthetic correctness fixtures only. The original collector
is barred: no stream read, sink write, activation, original message send, worker
start, completion or production admission occurs. The installed supervisor and
release remain unchanged. It is not performance or qualification evidence.

The implementation adds a typed owner of both a pending custodian and a barred
collector. Construction/revalidation retain the unchanged complete frame with
34 authentication and signature calls each, identical original time origins
and the original alarm. The direct composition adds no recursively validated
descriptor-plan constructor. Its ordinary entry accepts no fixture override.
An opaque failure retains both children and the exclusive original-owner borrow.
Field order cancels/reaps the collector before ending custody/original ownership.

The custodian still inherits exactly three descriptors. The collector inherits
exactly seven: the original fence, same original alarm, two original readers,
two original empty O_RDWR sinks and its private seqpacket endpoint. All four
pair/message writer aliases and every unrelated FD are excluded. The generic
O_WRONLY engine and original pending-custodian kernel are unchanged. Atomic
CLONE_PIDFD, parent signal restoration ownership and exact SIGKILL/wait cleanup
remain mandatory; no PID lookup/signalling fallback is introduced.

The child accepts no command. On expiry, peer activity/loss or observation error
it attempts all four I/O closes before a best-effort E/L/F notice, then parks
retaining its fence alias. Close failure is F, not quiescence. No success notice
or successful close releases custody. Kernel fence availability after child
death still cannot override durable retained-history refusal.

## Selected results and reproduction

Run `./verify.ps1 -SelfTest` in PowerShell from this directory. The verifier
checks the closed packet inventory, per-file digests, exact implementation patch,
ten changed source/registry files, all 320 libtest build inputs bound to the
source commit, binary/script identities, cohort/phase/count oracles, frozen
qualification digest and host postflight. Eight in-memory negative mutations
must fail without editing evidence. `capture.ps1 -SourceCommit <SHA>` generates
bindings only; it does not generate test results.

The source commit is recorded in manifest.json; the base is the preregistration
70e000c5. Rust 1.94/1.88 release-libtest images run in
/tmp/hc074-barred-collector-20261018.ET1hWc as UID/GID 1000 under umask 077.
The signed bundle is copied data only, never an executable workload. run.sh
starts each compiler cohort with fast units and a complete-frame healthy screen
before the repeated controls and faults. Actual commands and results are in
host.log and the retained local logs.

Across both compiler cohorts the selected gates require eight healthy controls
(including both first screens), eighteen reason-specific refusals and eighteen
process scenarios. The fixed refusal seed is 0x0740c118; the process seed is
0x0740c218 with three repetitions of expiry, peer activity and peer loss per
compiler. Healthy controls assert both full frame counts, exact FD inventories,
unchanged records/origins/signal mask and terminal children after cancellation.
Refusals assert their specific reason, not an upstream Time failure, sticky
poisoning and replay refusal. Process cases drop the parent aliases normally,
observe the collector's four I/O FDs gone, kill/reap the exact custodian while
the fence stays busy, then kill/reap the exact collector. The final kernel lock
is available, but retained history still refuses. These eighteen cases use
thirty-six selected child SIGKILLs, separate from refusal/ordinary cleanup.

All selected gates passed on both first-attempt compiler cohorts. Read-only
pre/post guards preserved supervisor PID 8839, zero restarts, boot identity,
installed executable/receipt digests and absent active campaign/authority state.
The final scan found no fixture executable from this packet or the preceding
pending-owner matrix. The frozen qualification manifest raw digest remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

Local checks cover 219 static contracts, registry discovery, Windows and GNU
current/MSRV all-target checks and strict Clippy, full workspace formatting,
two descriptor compile-fail doctests, docs and the local non-promotable
performance contract. Each host cohort also runs 32 original-storage and five
descriptor/clone-argument fast units. Full `cargo xtask verify`, product load,
installation and qualification are not run.

## Remaining boundary

This is enrollment of aliases, not exclusive transfer: the preparing owner still
owns the original readers/sinks/message and unsent writer aliases. These process
cases kill the children but drop the preparing owner normally. Earlier real
owner-SIGKILL evidence covered only the output-incapable pending custodian and
cannot be promoted to this two-child topology. Next prove actual owner SIGKILL
before/after the joint frame, both child-death orders, uncertain B delivery and
malformed notices without exporting fence/peer aliases to an external observer.

Authenticated original send/start, controller and whole-cgroup writer enrollment,
all-writer revocation, independently supervised active guardian, explicit active
cleanup failure handling, durable completion and later cells remain closed.
Neither a B notice nor I/O closure is authenticated readiness or cessation of
every writer. No throughput or native-API improvement is claimed here.
