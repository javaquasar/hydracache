# Eighteen role child readiness entry

Implement the reserved --systemd-bootstrap-held-readiness-v1 entry without
activating worker execution. The old installed and sealed-image modes remain
unchanged. FD0..16 retain every old predicate and observation order. A separate
closed eighteen-role inventory adds the readiness socket at FD17; only this new
role transitions from inherited flags to CLOEXEC before ownership is adopted.
The release channel remains empty under all old checks.

The new activation contract accepts exactly canonical LISTEN_PID, LISTEN_FDS=15,
the fixed fifteen FD names, nonzero lowercase INVOCATION_ID and current
SYSTEMD_EXEC_PID. The parent adds a finite UnsetEnvironment list and disables
login-environment synthesis; it does not silently ignore arbitrary extra child
variables. Missing/future/malformed metadata fails closed. This is activation
consistency, not independent manager or production-policy enrollment.

The child derives readiness expectations from its same observed sealed entry
and actual PID, not caller argv. A fresh nonce is learned only from the one-shot
kernel-authenticated packet. Full role observations bracket receipt and ACK;
packet, creator, invocation, digest or deadline drift refuses without retry.
After ACK the process remains nonexecuting and bounded by its original absolute
alarm, creator and guardian pidfds and readiness endpoint. It never releases or
executes the worker and never writes product data. Readiness does not authorize
completion or fence release.

Run deterministic activation/closed-inventory controls and local owned-process
fault cases for missing/extra/swapped roles, wrong entry/executable/environment,
wrong peer/context/invocation, duplicate challenge, lost ACK, disconnect, guardian
loss and expired original alarm. Kill/reap only owned fixture processes. Preserve
first failures in distinct lanes. Keep the frozen qualification digest and all
300/60/10 origins/windows unchanged; no clock or service changes, installation,
namespace/cgroup creation, server run or product workload is permitted.
