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
these are not real NSS enrollment positives. Public-constructor checks cover
pre-refusal, mapping mismatch and pending helper cleanup with those owned readers.
The existing operator integration retains actual NSS refusal independently; no
production helper selector is added merely to obtain a synthetic positive.

Trusted provider/configuration and namespace policy still precede production use.
Root-owned output preparation, authenticated signed start, watchdog/uncertain-intent
reconciliation and a controlled positive lifecycle rehearsal remain separate.
No host/account/unit mutation, product workload, performance measurement or
qualification is authorized. Frozen qualification inputs and thresholds stay exact.

## Local implementation and results

`bind_asserted_worker` borrows both original guards. Mapping comparison uses slices
and a bounded iterator that excludes the separately checked primary GID; it does
not allocate a temporary union. This is a source-level property, not measured
allocations/op or a product performance result. The original numeric credential
policy and NSS snapshot format remain unchanged. Both inputs permanently refuse
on every composition error, including errors during construction. No reset or
original-input extraction API is added.

The [local packet](local-runs/diagnostic-worker-binding-20261010/manifest.json)
retains the 205-pass Linux baseline, missing-API check and first twelve green
tests separately. Review adds the public pre-refused-input propagation test and
replaces the temporary union with iterator comparison. Final coverage passes
218 Linux tests with one pre-existing ignored; all thirteen new binding tests
pass three further repetitions. Seed `0x74b2026` exercises 256 canonically valid
mapping mutations per repetition. Windows passes 28 portable tests, executing
none of the Linux binding cases; contracts/evidence/governance pass 124 checks
before packet registration. Scoped checks, lint and documentation gates pass.
Full workspace milestone verification is not part of this local step.

The first Linux strict lint refused two explicit `drop` calls on the non-Drop
borrow wrapper in tests. Lexical scopes now end those borrows before checking
the original guards; production code is unchanged. Initial lint and reviewed
tests/check/lint/format captures remain separate. An initial root-test invocation
used nonexistent target `governance`; the corrected `release_governance` target
supplies the retained root result.

Owned NoNewPrivs cat processes establish real original credential observation,
exit refusal and concurrent independent state under a test-only process pin.
Positive account snapshots in those tests are synthetic. Public-constructor
tests exercise early refusal and retained pending-child cleanup, not a positive
production composition. The independent real NSS operator integration still
refuses with exit 9, empty stdout and confirmed helper cleanup in WSL. That
observation does not identify which underlying NSS condition caused refusal.

The asserted consistency prerequisite is implemented. Trusted NSS provider and
configuration, namespace/thread policy, root-owned production preparation and
authenticated original start remain closed; this guard cannot supply authority
for any of them. Sequential observation does not freeze credentials or NSS.
The qualification digest and expected-red ship admission remain unchanged.
