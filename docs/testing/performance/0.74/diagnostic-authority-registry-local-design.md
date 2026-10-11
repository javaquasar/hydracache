# Local worker authority journal

## Selected rollback boundary

The continuation after the original signed-task slice selects the recommended
local journal model: independently operator-supplied issuer/policy/epoch pins,
with crash-durable storage as the implementation target. Whole-host snapshot
rollback and owner/root tampering are excluded; an external monotonic authority
would be required to detect them. This does not select or install a real issuer.

Baseline: `27e5f0bd7c7596a8e54d9720cdeebae06ea1d268`. This slice implements
a bounded canonical record/transition core and a distinct Linux temporary-fixture
append-only storage backend. It does not implement production-root enrollment or
automatically connect the journal to a signed worker/start guard.

## Preregistered storage and transitions

Version-1 records bind the original verified policy's external issuer key, body
digest, epoch, machine/boot and user/mount assertions. The first record is active;
a refusal keeps exactly that binding. A later active generation requires a strictly
higher epoch after a refused generation, the same issuer and machine, and a new
policy digest. Every generation has a contiguous revision and predecessor digest.
Repeated same-epoch activation, key rotation, history gaps, changed refusal
bindings and overflow refuse. No wall clock determines authority.

Bounds: 128 generations, 2048 bytes per record, 65 bytes per commit marker and
256 named entries. Records and markers have fixed revision-derived names.
Opening an empty registry never bootstraps it; explicit initialization requires
an empty fixture directory. No file is overwritten, truncated, renamed, deleted,
evicted or automatically repaired. The writer owns a nonblocking exclusive
directory flock and retains path/object bindings; drift refuses.

Publication creates a record exclusively, syncs its directory entry, writes the
canonical record, makes it readonly and fsyncs it. A separate exclusive marker
contains the record digest, is made readonly and fsynced; directory fsync completes
publication. Missing, partial or mismatched pairs refuse on reopening. Failure
latches the original writer with no retry; all files remain. A complete marker
may be visible after an unacknowledged late failure: inspect, never assume rollback
or replay. Success proves the local fsync protocol, not physical power-loss behavior.

Only nonroot-owned 0700 fixture directories directly under /tmp with the
hydracache-authority- prefix are writable by this backend. Root-owned production
paths and the production account are not substituted with fixture ownership.
Active journal state is inspection data, never a start/admission capability.

## Planned checks and remaining work

Retain unchanged policy baseline and an absent-API compile red. Test canonical
grammar, all transitions, seeded history mutation, exact external pins/context,
real local close/reopen, refusal/restart, higher-epoch activation, bounded history,
exclusive writers, linked/special/unsafe files, original path/FD replacement,
partial IO and faults at logical publication stages without automatic repair.
Run affected-package tests/check/clippy, MSRV, root contracts/evidence/governance,
formatting/doc/book and offline packet integrity checks.

The next layers must add the reviewed root-owned fixed backend, issuer provisioning,
original signed guard composition and durable failure propagation, plus lifecycle
reservation/reboot/uncertain-intent/watchdog/cleanup behavior. No production host,
workflow, helper/IPC/start mode, product workload, numerical claim or qualification
input is changed by this local slice.
