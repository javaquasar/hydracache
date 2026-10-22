# Local held manager transport precondition

Status: preregistered negative compatibility control, before the single manager
mutation. Inherits R-3/R-5/R-7/R-8/R-10 and the W11 original-delivery boundary.
Starting source is c237f580. Temporary systemd units/slices are explicitly allowed
only in local WSL; existing services, clocks, server and product remain excluded.

The observed local manager is systemd 255.4-1ubuntu8.18. The held material owns
eleven launcher role slots, including release/worker/config/pidfds/alarm/bootstrap.
A native transient-unit ExtraFileDescriptors bridge cannot be presumed available
on this manager. The [v255 service setter](https://github.com/systemd/systemd/blob/v255/src/core/dbus-service.c)
has no handler, whereas the [v257 setter](https://github.com/systemd/systemd/blob/v257/src/core/dbus-service.c)
accepts that property. A version string alone is not a working descriptor proof.

## One finite local control

Commit the tool, tests and plan, then run its explicit root WSL entry once with
a fresh attempt name. Require exact workspace/kernel/root identity, original
PID 1 systemd, unchanged boot/namespaces/qualification, clean source and tool
hashes. Nine offline controls pass on Windows and Linux first. Every command
receipt, source, first failure and exact unit name must be retained.

Resolve systemd's unique bus name and check its bus-reported PID 1 and UID 0.
Query Version on that unique destination. Require the observed 255 family for
this particular negative control; a changed or unknown manager refuses before
mutation, rather than being declared supported. These separate busctl clients
are explicitly not the retained original manager channel or full live frame.

Generate one 128-bit random hc074-fdcompat-<hex>.service name and prove it absent.
Submit exactly one StartTransientUnit, mode fail, six fixed properties and no
auxiliary units: Description bound to the name, Type oneshot, Restart no,
NoNewPrivileges true, an empty ExtraFileDescriptors array, and /usr/bin/false as
the only possible helper/argv. It receives zero writer/config/executable rights.
No caller command, path, property or target name is accepted.

Require the exact v255 unsupported-property rejection, then the same unit's
absence and unchanged original manager/context. Access denial, malformed
signature, timeout, disconnect, unknown reply, manager churn or a still-present
unit is a first failure, not a compatibility pass. Never resend or blindly stop,
reset, retry, upgrade or substitute a manager. Retain any uncertain unit for
exact scoped inspection; no other unit may be touched.

The per-call bus timeout is 250 ms and subprocess guard at most two seconds,
inside one ten-second compatibility-test guard. These are only harness bounds.
The original 300/60/10 admission timers, origins, full predicate count/order,
first Causal refusals and qualification remain unchanged; this test constructs
no original reservation. No measurement cohort, worker or production load runs.

## Result and stopping boundary

Expected disposition is blocked-unsupported-manager-fd-api, with both original
delivery and writers cessation false. A successful negative control proves only
the scoped rejection. Empty rights and /usr/bin/false cannot prove held mapping,
executable/stream/process/cgroup binding, active drain, terminal writers, durable
completion or a fault matrix. A newer manager alone would not close these gaps.

If the property is unsupported, stop this lane and retain the result. Do not
silently add an FD broker/bootstrap protocol or upgrade the running manager.
Those change the architecture or existing services and need a separate decision.
Implementation of original one-shot uncertainty, held delivery and all-writer
revocation remains required after selecting a supported transport/environment.
