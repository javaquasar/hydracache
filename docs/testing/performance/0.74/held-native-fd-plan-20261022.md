# Native descriptor mechanics control — 2026-10-22

Run once after the separate local `HydraCache074Native` first-boot check. This
is a synthetic compatibility/mechanics control, **not** original delivery or an
all-writer gate. Existing Ubuntu/default/global config remain untouched. No
server, installed release, existing service, clock change or product workload.

Use one random `hc074-nativefd-<128-bit-hex>.service` and uniquely corresponding
`system-hc074nativefd<hex>.slice`. Resolve the manager's unique bus name and
require its bus-reported PID 1/UID 0; address all manager methods to that name.
One StartTransientUnit only, mode fail, no retry/fallback. Refuse existing unit.
Each call has a 250 ms bus bound and immutable command/output/exit receipts.
Only fixed read-only `/usr/bin/true` and `/dev/null` descriptors are extra-FDs;
the fixed nonproduct shell helper checks LISTEN_PID/FDS/FDNAMES, slot 3/4
device/inode/type, role hash and its service cgroup. Two distinct report FDs are
manager-supplied stdout/stderr. No product executable or original writer is used.

The helper runs under oneshot, restart=no, NoNewPrivileges, control-group kill,
five-second start and one-second stop bounds. Expect exited/success/status 0,
no remaining MainPID, fixed output and empty stderr. Cleanup may stop **only**
the random service with the exact expected Description on the same pinned
manager; otherwise retain uncertainty and report failure. Do not reset/retry.
Host wrapper refuses existing evidence and dirty source; retains source digests,
command, intent and first outcome. No original reservation or absolute deadline
is created/reset. The production 300/60/10 windows and live predicates stand.

This lane deliberately uses separate busctl connections and a path-loaded shell
helper. It cannot prove retained original channel/full frame, held executable
binding, sealed config, release barrier, watchdog or closure of hidden descriptor
aliases. A service ACK, exited state, cgroup or these file reports must not be
used to publish original completion, release a fence or open server admission.
