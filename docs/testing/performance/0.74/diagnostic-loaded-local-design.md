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

## Implemented observation boundary

`diagnostic-loaded-worker` uses the existing manager client's exact owned-child
limits and cleanup ownership. Its request is the unchanged canonical scope;
its new schema-1 response wraps the existing manager snapshot and 53 typed
settings. The old worker response stays unchanged. `diagnostic-loaded-inspect`
is an operator read, not a signed request or diagnostic reservation.

`LoadedSnapshot::pin_for` derives expected settings from validated startable
state. It refuses absent, non-transient or zero-invocation units and mismatched
scope/settings. `InvocationGuard` retains the original unique manager owner,
boot/scope, object, invocation and cgroup path. Active state, PID and result may
change within that invocation; the guard does not certify process identity or
terminal success. `ManagerClient::inspect_original` latches any observation
failure and preserves bounded failure output. Neither a subsequent matching
read nor a new helper erases that runtime refusal. A new guard after restart
would be a new observation, not continuity with the original execution.

All exposed fixed settings are compared exactly, including extended ExecStart
flags, hard and soft file limits and CPU affinity. Extra lifecycle commands,
environment files, passed/unset environment, dynamic user and alternate root
directory/image refuse. Output mode is checked as `append`; destination paths
remain explicitly unproved. Unknown *wire* fields refuse; unrelated systemd
GetAll properties are outside this fixed projection and are not full hardening
proof. Unsupported property names/types fail rather than downgrade.

## Local checks and remaining binding

The [retained packet](local-runs/diagnostic-loaded-20261010/manifest.json)
contains seven new Linux tests within 189 focused passes and one pre-existing
ignored test. Windows contributes 28 portable passes, not loaded-manager
coverage. Root checks pass 79 performance contracts, 13 evidence tests and 23
governance tests; scoped formatting, all-target checks, strict all-feature
clippy, doc-check, local non-promotable performance contract, links and mdbook
also pass. Full workspace verify was not rerun for this isolated slice.

Four real local WSL reads authenticated the manager and returned absent units
with no settings. Wrong boot refused with helper cleanup confirmed. Positive
loaded settings and original-invocation checks use synthetic properties only.
The packet preserves compiler refusals before APIs/contracts existed and an
invalid Reserved-state fixture that the existing validator correctly rejected.
The fixture was corrected without changing state validation. Its byte/count
verifier is offline retention QA, not an authenticated manager or launch check.

Actual executable/output descriptors, effective process environment, original
process/tree references and controlled start/stop remain before live enrollment.
Durable first failure, writer revocation, autonomous watchdog and pending-request
reconciliation remain separate obligations. The frozen qualification digest is
unchanged; no timing, allocation or release-admission result is produced here.
