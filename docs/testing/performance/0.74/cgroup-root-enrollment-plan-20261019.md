# Attempt cgroup root enrollment

The operator approved the separate attempt cgroup contour and a new enrollment
protocol on 2026-10-19. Baseline is `7fb289c5e2b80f4f0ea2f68da12f61a8afb44125`.
This supersedes the pending architecture decision in the
[boundary audit](original-arming-boundary-audit-20261019.md), not its findings or
the original live predicates. Inherits R-3, R-4, R-5, R-7, R-8 and R-10.

## Approved local implementation sequence

1. Define a distinct fixed-size root enrollment packet. Bind attempt and full
   original-context digests, actual root device/inode and the unchanged selected
   absolute BOOTTIME bound. A data-only inspector grants no runtime authority.
   Reject every unknown kind/version/reserved byte, malformed field, old packet,
   size change or substituted binding. Keep sender uncertainty before its first
   send; lost ACK cannot permit a retry. Register the new format in COMPAT.
2. Add private retained cgroup2 root mechanics. Open only one child component of
   a retained parent with no symlink traversal; pin root/type/events/procs/kill
   identities and root ownership. Require an empty unfrozen domain before
   enrollment. Do not accept regular-file fixtures as kernel roots. Never
   reopen a replacement kill target after name drift. Kill submission and an
   empty observation remain distinct from all-writer cessation or completion.
3. Exercise owned local WSL cgroups only in an explicit root/manual fixture.
   Create a fresh test-prefixed cgroup and controlled descendant children;
   never migrate the creator, collector or guardian. Cover empty root, descendant
   kill, replacement/removal, permission drift, hidden external pipe copies,
   SIGKILL immediately before/after kill submission and lost acknowledgement.
   Exact owned child cleanup precedes nonrecursive removal of these roots.
4. Add authenticated FD transport and independent guardians retaining the
   original fence/timer. Join original policy/epoch/artifacts/manager/full frame,
   then the separately projected held unit. Armed precedes first held delivery;
   actual executable/process/streams/cgroup binding and a full frame precede
   release. No supplied packet, FD or successful low-level root test can mint
   original admission. Keep creator and collector outside the revocation root.
5. Extend the SIGKILL/lost-ack matrix to the joined new boundaries, then durable
   completion, later cells and same-attempt reconnect before server rehearsal.

Step two may expose only private kernel mechanics before step four provides an
original-owner constructor. The new enrollment wire format is not the old
direct-child guardian protocol; leave its implementation, predicates, tests
and signatures intact. Freeze all 381 previous supervisor Rust sources through
exact additive registrations. Preserve every old frame, all 46 effect brackets
and the original 300/60/10-second origins. No new relative lease or deadline
renewal is permitted.

Retain the prior closed regression packet as baseline, a missing-module red
receipt, ordinary/property controls, actual local-kernel process receipts and
current/MSRV/Windows checks. Run strict lint, formatting, source contracts,
registry/docs and local performance-contract checks. Keep code, selected-source
evidence and article conclusions separate where practical. All new inventories
hash exact bytes or use explicit ordinal BOM handling, never culture-sensitive
prefix tests.

The Linux kernel's [cgroup2 interface](https://docs.kernel.org/admin-guide/cgroup-v2.html)
defines recursive `cgroup.kill` and hierarchical `populated` observations.
These primitives do not prevent later admission or close writer FDs outside the
root. The future systemd slice/held-unit projection must separately establish
exclusive population and manager identity; this package installs no unit.

No server access, release installation, supervisor operation, original/system-bus
send or product workload is authorized. Local WSL root privileges are limited
to isolated temporary cgroup fixtures and their exact owned subprocesses.

## First implementation result

Source `593e51481d78cfd3a917b81d22f46b153ae1c57a` completes steps one through
three as separate packet claims and private pinned-root mechanics, not an
original runtime constructor. GNU 1.94 and MSRV 1.88 each pass seven new ordinary
controls and three eleven-case actual cgroup matrices. Across selected cohorts
there are 66 scenario executions and 24 actual SIGKILL cuts of the private kill
caller. Old two-phase, exchange and seventeen-case guardian regressions remain
green, including their common-mode and hidden-writer negatives.

The before-write caller death leaves its target alive; the harness performs
exact owned cleanup. Empty-at-read also does not prevent late population or
close an outside pipe alias. These are retained limits, not failures to remove
or proofs of autonomous revocation. Cuts after write/API return/empty observation
establish only the private kill operation's uncertain-confirmation behavior.
New authenticated enrollment ACK loss and joined durable boundaries remain.

Windows passes 54 ordinary and eighteen compile-fail controls. Both GNU
compilers pass all-target check and strict all-feature Clippy. The complete
source suite passes 277 controls, preserving every previous Rust body, old
protocol and original bound through exact additive registrations. The
[evidence packet](local-runs/cgroup-root-enrollment-20261019/README.md) binds
source, images and recipe and retains the initial test-visibility compile error
and an ineffective zero-test filter separately from accepted receipts.

Next is step four: authenticated three-FD transport, independent root guardians
under the original fence/timer, original trust/epoch/artifact/channel/full-frame
binding and the separately projected held unit. No additional topology approval
is needed for this approved local sequence, but production installation, server
operations, original send and product workloads remain closed.

## Descriptor transfer within step four

Source `65b02f3b` adds the separately scoped
[local creator request transport](cgroup-root-transport-plan-20261019.md).
Actual socket credentials, three received FD roles, root identity/access and
empty-domain checks precede retention. Sender and receiver consume once before
their syscall. Rejection closes installed rights; missing confirmation never
permits replay. This ordinary-process reader emits no ACK or Armed and is not
safe to execute inside a raw post-clone child.

The [transport contract](cgroup-root-transport-contract.toml) keeps guardian
reply, original-owner construction, autonomous revocation, held projection,
completion and server admission false. The first packet/root contract remains
unchanged as its earlier scoped result. Next, independent stack-safe guardians
must retain original process-pidfd/fence/timer custody, validate this same root
under the unchanged frame and authenticate their replies before held delivery.
Use the [transport evidence](local-runs/cgroup-root-transport-20261019/README.md)
for its selected-source controls, not as proof of that future integration.

## Independent local guardians within step four

Source `72f53806` adds the [local active pair](root-guardian-plan-20261019.md).
Two stack-only children inherit the same existing fence/timer OFDs and monitor
actual creator process pidfds. They authenticate and retain the three root
roles before rights-free v2 Armed replies; the parent authenticates exact clone
children and consumes both sends before waiting for either reply. Each child
uses a separate read-only events watch OFD on the same inode. Notification
acknowledgement precedes kill so late population cannot be cleared and slept
through after a previous kill.

The [evidence scope](local-runs/root-guardian-20261019/README.md) separates local
expiry/channel-loss and single-guardian faults from actual creator SIGKILL,
which still needs its own subprocess matrix. Common-mode failures and an
external writer remain negative controls. The
[contract](root-guardian-contract.toml) keeps original owner/Armed, held unit,
whole-writer cessation, completion and server admission false. Next is actual
creator process loss and the original policy/epoch/artifact/channel/full-frame
constructor, not a reinterpretation of local Armed as original permission.
