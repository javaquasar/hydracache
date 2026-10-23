# Complete inherited role assertions with authentication closed

Baseline 851763a0 reads only entry FD14. Add an explicit separate
--systemd-bootstrap-census-v1 mode; preserve the preceding entry-only mode and
all original owner/live/launcher code, check order and absolute windows.

Check all fifteen semantic slots against the same read entry: distinct FD
numbers; exact inherited flags, type/access/metadata; empty EOF stdin and
distinct output pipes; empty credentialled seqpacket release endpoint; bounded
readonly image/config/legacy contents and identities; live creator/guardian
pidfds through fixed fdinfo; original absolute BOOTTIME alarm; disjoint regular
inodes. Older kernels may share pidfd anon inodes, so PID/FD/liveness rather
than inode inequality identifies those roles. Repeat the complete observation
and bracket it with unchanged entry revalidation. Never recreate/repair inputs,
consume release traffic, arm a new runtime timer, send, start or exec.

The census establishes consistency with untrusted assertions, NOT the actual
bootstrap executable, manager, root/cgroup, privileged guardian, worker
credentials, signed documents or protected enrollment. Even a complete forged
fixture must end at authentication-closed. The parent's fingerprint is opaque,
not recomputable from remapped child numbers or a transferable permit.

Tests use private pipes/sockets/memfds/timers and owned helper pidfds only. Keep
first failures, old entry-mode controls, and frozen qualification bytes. Local
Windows/GNU/MSRV tests/check/strict Clippy, format/docs/contracts and an unsigned
selected build are separate from original live or performance evidence. No
clock/service changes, server, installation or product workload.

Next authenticate real credentials/namespaces/root/guardians and original
manager/owner provenance, enroll the clean signed bootstrap and only then
permit one-shot launcher handoff and all-writer/lifecycle fault gates.
