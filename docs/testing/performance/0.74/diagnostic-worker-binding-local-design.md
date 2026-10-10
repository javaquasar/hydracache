# Account and original process credential binding

## Local scope and sequence

Baseline `0f3c6e674b82e12d365813c307ca22830680a714`. Add a runtime-only
read-only composition of the fixed account guard and an already pinned original
process credential guard. Record the current Linux library/artifact/manager/account
baseline, add missing-API tests, implement and retain repeated checks. No new PID,
numeric policy, account selector, helper mode, descriptor, start or IPC route is
accepted by the composition. This is asserted mapping consistency, not trusted
host account enrollment or production execution.

## Mapping and original guards

The credential guard already owns an explicit numeric policy and original retained
status. Require its UID/GID to equal the account guard's original snapshot. Require
the union of its explicitly asserted supplementary groups and primary GID to
equal the original NSS membership set. Primary inclusion in the kernel list is
not inferred: both an explicit inclusion and an explicit omission can agree with
the union, but the existing status reader always checks the kernel list exactly.
Reject zero/sentinel supplementary IDs, duplicates, noncanonical order and more
than 32 membership IDs in this narrower composition. The existing numeric-only
reader and NSS snapshot semantics are unchanged.

The [systemd supplementary group contract](https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml)
extends groups from the user database; an empty property is not a reason to assume
an empty kernel group list. This composition does not prescribe a production group
policy or authenticate that database. Its original assertions cannot be replaced
by values read from a later process or later NSS observation.

Check pre-existing refusal and mapping before any new observation. Then perform
`account -> original credentials -> account`. Existing account helper deadlines,
capture and cleanup rules apply to each lookup; existing process-generation and
two-status brackets apply to the credential step. Any failure preserves its typed
first error and latches both original inputs, including constructor failure and
after wrapper drop. A refused wrapper performs no further observation and cannot
refresh/reset or export an input. No atomic/continuous snapshot, all-thread proof,
whole-composition deadline, durable journal or writer revocation is promised.

## Test boundary and remaining work

Cover exact UID/GID/group union, primary inclusion/omission, empty/full group bounds,
zero/sentinel/duplicate/missing/foreign groups, every bracket failure, first-error
details, prior refusal, restoration after drift, wrapper-drop propagation and
independent concurrent guards. Seed identity mutations and validate changed
snapshots before asserting refusal. Owned non-product NoNewPrivs cat helpers can
exercise real original kernel credentials with synthetic fixed-account snapshots;
these are not real NSS enrollment positives. A public-constructor check must
retain actual NSS refusal, rather than create the account or substitute the caller.

Trusted provider/configuration and namespace policy still precede production use.
Root-owned output preparation, authenticated signed start, watchdog/uncertain-intent
reconciliation and a controlled positive lifecycle rehearsal remain separate.
No host/account/unit mutation, product workload, performance measurement or
qualification is authorized. Frozen qualification inputs and thresholds stay exact.
