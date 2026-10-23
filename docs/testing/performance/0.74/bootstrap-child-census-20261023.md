# Complete child descriptor census remains an assertion check

The separate --systemd-bootstrap-census-v1 mode checks all fifteen inherited
roles and refuses extra descriptors. Even a complete consistent forged role set
exits 2 with original authentication closed. This closes the entry-reader's
missing role-consistency checks, not original start, actual process credentials,
manager/root provenance, launcher execution or all-writer cessation.

## Why entry bytes were insufficient

Baseline 851763a0 could read and retain HCBT0741 but inspected only FD14. Correct
entry bytes could therefore describe unrelated images, sockets or tasks, and
unexpected inherited FDs could remain outside the named set. The
[plan](bootstrap-child-census-plan-20261023.md) adds an explicit new mode rather
than changing the preceding entry-only protocol. Its exact argument is checked
before reusing the old pure activation-metadata validator; no execution retry or
environment repair occurs. Earlier entry-mode controls remain unchanged.

## Complete role consistency

| Slots | Claimed role | Added checks |
| --- | --- | --- |
| 0 | stdin | Empty nonblocking FIFO reader with EOF; exact stream identity. |
| 1–2 | stdout/stderr | Distinct nonblocking FIFO writers, exact identities and no error/hangup. Backpressure is not drained or repaired. |
| 3 | release | Empty nonblocking Unix seqpacket, PASSCRED and kernel PEERCRED matching asserted owner; peek only, no traffic consumed. |
| 4, 11–13 | worker, host, launcher, bootstrap image | Exact device/inode/size and bounded positional content hashes with ELF header shape. Exact seals except the existing root-owned/nonwritable executable alternative for worker. |
| 5, 10, 14 | config, legacy bootstrap, entry | Readonly sealed material, exact identity/content and retained entry observations; legacy fields agree with the entry, without exporting its token. |
| 6–8 | creator and guardians | Live pidfds, fixed procfs fdinfo PID and asserted guardian identities. Distinct PIDs/FDs do not require distinct anon inodes on older kernels. |
| 9 | alarm | Nonblocking, unreadable BOOTTIME timerfd, absolute-set flag, zero ticks, nonperiodic remaining time bracketed by fresh BOOTTIME reads against the same absolute deadline. |

All inherited descriptor flags are zero, with no O_PATH; regular offsets are
zero and remain unchanged. Regular material inodes must be disjoint. Before and
after capture, fixed procfs enumeration requires exactly 0..14 plus its own
enumerator. A matching role-name count alone cannot establish this closed set.
The enumerator accepts at most eighteen directory entries and nineteen reads,
using a 4096-byte buffer and bounded dirent parsing. fdinfo permits 4096 bytes;
images retain the old 128 MiB bound, config 64 KiB, legacy 256 and entry 1024.

Entry revalidation brackets each capture; a second complete role observation
must match. Fixed-size retained metadata includes access/flags/offset and inode
timestamps. Queries never reopen a role, change flags, consume release data,
arm a runtime timer, start/send or exec. The original parent validation frames,
check order and 300/60/10 origins/windows remain byte-identical. New read-side
BOOTTIME checks compare existing deadlines, never issue a new admission window.

## Limits that the census cannot prove

The HCBT0741 fields remain untrusted assertions. The current executable, actual
credential/namespace policy, original manager/root/cgroup, privileged guardian
role and exact clean-source signed bootstrap are not authenticated here. The
parent fingerprint is opaque: child remapped numbers cannot recompute parent
descriptor authority. No File/FD/owner/permit extraction is exported.

Observation is sequential, not atomic or continuous. A closed descriptor set in
this process does not revoke copies in other processes. The owned fixture
guardians are ordinary sleepers, not authenticated privileged writer guardians.
Original owner transfer, one-shot launcher handoff and all-writer quiescence
remain separate gates before completion, later cells or server admission.

## Verification and retained failures

The [first results](bootstrap-child-census-first-result-20261023.json) preserve
the missing-census source failure and a GNU 1.94 compiler ICE while rendering
an unused fixture-method warning. A short-format diagnostic check identified
the warning; using the fixture's actual fds() array fixes its unused code without
suppression or runtime/deadline changes. These are captured-tool summaries,
distinct from raw fixed-runner receipts.

Unit positives use actual private memfds, pipes, sockets, timerfds and owned
helper pidfds with a modeled child identity. Process controls remap only a fresh
synthetic child and use its actual PID and closed 0..14 layout. A complete forged
role set reaches explicit authentication-closed; an extra FD, wrong image digest
or dead guardian refuses earlier. No helper is a product process and no original
live fixture or measured attempt is repeated.

The initial fixed GNU/MSRV lanes passed runtime controls but refused Clippy
because the fixture helper was loaded twice. The first metadata lane refused an
unregistered Linux cfg-target. Sharing the helper at the binary root and adding
the explicit additive registry entry corrected those issues. The first revised
Windows lanes also preserve Access denied failures from rebuilding a running
xtask.exe; host and metadata lanes are now serialized. An earlier source-view
reconstruction failure came from import ordering and is retained in the captured
summary. None of these failures is an original live attempt or a performance
sample, and none is erased by the later green correctness checks.

Final fixed lanes host-final-v2, 194-final-v2, 188-final-v2 and metadata-final-v2
all completed without failure:

| Scope | Result |
| --- | --- |
| Windows | 42 source contracts and 13 bootstrap/unit/process controls; check, strict all-feature Clippy and format passed. |
| GNU 1.94 | 57 selected unit/process/regression controls; all-target check, strict all-feature Clippy and selected unsigned build passed. |
| GNU MSRV 1.88 | The same 57 selected controls; all-target check and strict all-feature Clippy passed. |
| Metadata | Three runner controls, doc-check, 187 registered gates, local nonpromotable performance-contract and diff checks passed. |

Source commit is f3098487ba930ca101f2209dad211f4214aa218c. The
[exclusive packet](local-runs/bootstrap-child-census-20261023/inventory.json),
committed separately as ff7b7800, retains 581 inventoried members plus the
inventory itself; all 582 staged Git blobs were checked against their exact
working bytes and inventory hashes before commit. Its inventory SHA-256 is
dbc23996b37702d9be94f718598f92705fc296483f1a0ce442e9fcb3b707e7bd.
Initial/revision source snapshots and raw refusals remain alongside final logs.

After packet retention, documentation postflight repeated doc-check and the
nonpromotable performance contract successfully. An additional Windows
whole-workspace cargo fmt --all invocation failed with OS error 206 (command
line too long); its captured-tool summary is retained separately, not inserted
into the sealed packet. Formatting coverage here is the two affected packages,
as selected in the successful fixed lane, not a whole-workspace formatting claim.

The selected unsigned GNU image was 1596552 bytes with SHA-256
81b59b35ea8e95f2fdb391552adb49e2d76139ed381a020d39e4dd07d45c6ec8.
This dirty-worktree build observation is not a clean-source signed receipt or
enrollment; that requires a new trusted build from the selected clean source.
The frozen qualification manifest remains unchanged at SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

New metadata, hashing, procfs and clock-query costs are unmeasured; unit elapsed
time does not establish that full original preparation fits its admission
deadline. No clock/service change, systemd request, installation, server action
or product workload occurs. No performance or server-readiness claim follows.

## Next original authentication boundary

Bind actual executable/credentials/namespaces and privileged root/guardians to
the independently selected protected policy and original manager/owner context.
Enroll a clean signed bootstrap build, then permit only the original sealed
launcher handoff. Prove writer cessation and the full SIGKILL/lost-ACK lifecycle
matrix before publishing completion or opening server rehearsal admission.
