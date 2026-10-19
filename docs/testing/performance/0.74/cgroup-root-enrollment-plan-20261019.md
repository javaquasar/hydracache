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
