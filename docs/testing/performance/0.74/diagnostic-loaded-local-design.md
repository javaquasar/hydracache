# Loaded diagnostic settings and original invocation

Baseline is clean `eec34ffa924f90496721c5469ce1665384844990`: fourteen
Linux manager unit/helper tests pass. This slice adds a separate bounded,
read-only worker operation without changing the existing scope/snapshot wire.
It will observe the fixed diagnostic namespace, authenticate the root/PID-1
unique manager and reply senders, compare two uncached complete observations,
and retain the original manager, object path, nonzero invocation and cgroup.
Changing any of those identities or the checked settings permanently refuses
the runtime guard. An absent unit cannot be pinned. No automatic adoption of
a replacement invocation is allowed.

The settings projection covers fixed spec properties exposed by systemd 255,
including hardening, resources, exact command/argv, extended execution flags,
environment and working directory. Extra lifecycle commands and environment
files refuse. Missing, wrongly typed, unknown wire fields, malformed, duplicate,
noncanonical and oversized responses fail closed. Parent timeout, memory/CPU,
combined output and helper cleanup budgets remain unchanged. Synthetic loaded
properties and actual absent-unit reads are separate evidence categories.

Systemd 255 exposes output mode but not the configured append filename in its
[execution property table](https://github.com/systemd/systemd/blob/v255/src/core/dbus-execute.c).
The [service table](https://github.com/systemd/systemd/blob/v255/src/core/dbus-service.c)
exposes extended commands with flags and runtime status. Normalize only command
path, argv and flags; timestamps/status belong to later process evidence, not
configured policy. No fabricated append-path getter or silent fallback is
allowed. This projection does not prove append destinations, effective process
environment, executable inode, loopback restriction, tree continuity, writer
revocation, durable failure, watchdog or uncertain-intent reconciliation.

The guard is an observation consistency check, not a capability to start, stop,
clean up or release a host. Caller-supplied state is not an authenticated durable
intent; a runtime latch does not survive restart. No units, services, host
installation, product workloads, numerical pilot or qualification are in scope.
Lockfiles, P0 configs and qualification identity stay frozen.
