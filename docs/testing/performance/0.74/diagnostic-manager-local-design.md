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
