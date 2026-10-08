# Bounded diagnostic manager: connection setup is part of the deadline

This preregistration follows clean source `55066325`. It closes one live-backend
prerequisite, not the permission or proof to run the numerical pilot. Historical
packets, P0 inputs, lockfiles and qualification identity remain unchanged.

The existing campaign manager uses blocking zbus calls. A method timeout does
not bound connection authentication, proxy construction, all property calls or
connection teardown. A detached thread and `recv_timeout` would leave the
operation alive after the caller's timeout. Do not extend that dispatcher to the
diagnostic namespace or use a timeout as evidence that a unit did not start.

The fixed supervisor executable has a separate **read-only** worker mode. A
Linux parent executes only `/proc/self/exe`, clears the environment, closes
stdin after a canonical bounded scope request, and polls nonblocking stdout and
stderr together. The two-second budget covers request delivery, handshake,
observation, output and exit. Output has one combined 64-KiB ceiling; polling
checks the deadline inside draining loops. Failure retains bounded partial
output and initiates termination of only the exact unreaped helper child.
Cleanup has a separate one-second polling budget. An unconfirmed helper remains
owned by the client and blocks a new operation; no unbounded `wait` or retry is
allowed. This is helper lifetime control, not workload-tree cleanup proof.
Scheduler suspension and an uninterruptible kernel task can exceed wall time;
those cases fail closed rather than claim a hard physical deadline.

The helper's Tokio worker count is fixed to one. Before parsing input or opening
DBus, its address space is capped at 512 MiB, CPU time at two seconds and core
dumps disabled. These are helper resource ceilings, not product thresholds.
Unsupported kernel/resource operations refuse rather than silently downgrade.

The worker accepts only a lease/boot/four-surface scope, never a unit name, path,
argv or method. It uses the fixed system-bus socket under root-owned,
non-symlink, non-writable ancestor directories, pins systemd's unique bus owner,
and requires its bus-reported UID 0 and PID 1 before and after observation.
Properties are fetched without a cache. Only the named systemd NoSuchUnit reply
means absent; transport, authorization, malformed properties and owner/boot
drift are errors. Loaded unit ID, invocation ID and cgroup path are bounded and
scope checked. A snapshot is not verified unit hardening, artifact identity,
recursive emptiness, writer revocation or release authority.

Every successful reply and named-error reply also requires the exact pinned
sender header (the bus daemon itself for owner/credential queries). A familiar
error name alone cannot establish absence. Two complete unit observations must
match, including invocation identity and cgroup; observations are not atomic
with a later unit operation. The local socket's root owner and safe ancestors
are the trust anchor, not a claim of protection against malicious host root.

Local checks will cover canonical scope/response binding, foreign principals,
drift, malformed/oversized output, stderr overflow, stalled input/output/exit,
nonzero exit, bounded failure capture and cleanup ownership. Synthetic children
are non-product test helpers. A positive real system-bus read, when available,
must be reported separately from synthetic coverage and must create no units.

The next launch gates remain: authenticated start/stop with intent fencing,
live tree/process/artifact binding, writer revocation before spool publication,
autonomous watchdog and explicit pending-request reconciliation, non-product
loss rehearsals, then reviewed signed installation and a baseline-only pilot.
No candidate, six-hour or 24-hour qualification is opened by this helper.

## Implemented boundary

`diagnostic_manager.rs` implements the fixed worker, canonical scope/snapshot,
bounded child client and read-only manager lookup. The Linux-only operator route
is `diagnostic-manager-inspect LEASE BOOT SURFACE`; the worker route is internal
and accepts its canonical scope only on stdin. Neither is a signed diagnostic
request route, lease reservation, workload start, stop or host release.

The client retains partial stdout/stderr in typed failures. Those runtime bytes
are **not** a durable raw workload packet; the future backend must publish them
with its first-failure record. A retained helper prevents a second operation in
the same client. Parent restart does not preserve that runtime handle or prove
continuity. Scheduler/kernel stalls, hostile root and physical crash durability
are not proved by polling and local tests.

Positive local system-bus lookup is kept separate from typed invented loaded-unit
properties. No diagnostic unit was created. The live backend still needs exact
policy/artifact/start binding, controlled mutation, original tree/process
references, writer revocation, durable failure, autonomous watchdog and explicit
uncertain-ledger reconciliation before deployment or numerical execution.

The clean `097ab6fa` repeat preserved a negative manager fixture result:
`failed_exit_and_unexpected_stderr_are_not_success` observed `Io` rather than
`Exit`. Its child could exit before accepting stdin, so the parent correctly
refused the broken pipe before reaching exit classification. Output/exit/limit
fixtures now consume the fixed request and EOF before their intended action.
Production IO refusal, deadlines, budgets and cleanup behavior are unchanged;
this isolates test causality rather than accepting either failure category.

## Exact-source repeat and retained negatives

Clean `c2fdc3f48878bd931788218fec3573b137def3ba`, tree
`f61e80525d4742b2d2fa3ab1f6f91507ddcdf44d`, passed 62 Windows, 154 Linux/WSL,
88 root contract/evidence and 23 governance tests. All-target check, strict
all-feature clippy, scoped fmt, docs/local contract, 17 governance CLI checks,
links and mdbook passed. Require-ship remains expected-red (exit 1).
[The packet](local-runs/diagnostic-manager-c2fdc3f4/manifest.json) retains all
three preparation repeats and the deterministic red proc-reader test, not only
the final green result. Source was clean before and after the final capture;
packet sealing and its independent hash/count/claim guard follow separately.

Linux includes 14 manager unit/helper tests, two manager integration tests,
15 process tests and six pre-existing abort-backend diagnostic tests selected
by the diagnostic_ substring. Windows runs zero Linux manager/process/tree/spool
tests; it is not Linux coverage. Four actual local system-bus absence reads
authenticated a unique owner with UID 0 / PID 1 under one boot, and the wrong
boot was refused with helper cleanup confirmed. No unit was loaded or created.
This is an authenticated absent-unit observation, not complete loaded-unit
policy, exec identity, writer revocation, durable failure or cleanup proof.
