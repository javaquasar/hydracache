# Refusal of changed context during fresh journal inspection

Baseline is `bd72707189db21fe5ac67111462c3c91b71f049e`. This P1 slice
tests the existing boot/context and authority-pin refusal behavior in fresh
processes after actual owned-process loss. Production APIs, storage formats,
original authority bindings and cleanup permissions remain unchanged.

## Test inputs and observation

Add eight explicit test-only inspection profiles: same, boot, machine,
user-namespace, mount-namespace, digest, epoch and issuer. Each changed profile
changes exactly one selected host identity or trust component; it does not
modify retained bytes. Unknown profile names refuse rather than defaulting.
The unchanged profile preserves the existing three journal observations.

Use the existing owned Starting quarantine and six SIGKILL checkpoints, plus
one uninterrupted AfterReturn control. At each retained state, two fresh
processes per profile inspect reservation eligibility and the authority journal.
Complete journals under changed profiles must return AuthorityError::Binding;
partial tails must return Invalid before binding. Do not call the latter a
successful boot/pin check. Same-profile complete journals remain active before
refusal or refused after durable commit. All retained contexts prohibit a new
reservation. No inspection may append, repair, retry, delete or recover a guard.

The parent still owns the exact child and helper pidfd, and verifies common
lock release, helper termination and unchanged retained bytes/metadata after
each inspection. Add profile-specific observation lines without changing the
existing same-profile proof semantics. Test-parent fixture pins are not
independent production enrollment. Boot and namespace mutations are modeled
claims; no actual reboot, namespace switch, key enrollment or server operation
is performed. Missing PID or inode is not cleanup evidence.

## Preregistered proof

Capture baseline authority fixture tests and a missing-profile red test.
Fast tests cover explicit name parsing, the exact observation taxonomy and
seeded one-component mutations with seed 0x074c: 32 unchanged and 224 changed
contexts per repeat. Register a separate ignored/manual Linux proof using
unchanged seed 0x074d and 600-second gate budget. Each of three same-source
repeats must pass six SIGKILL cases, one control, 56 profile groups and 112
fresh inspections. Rerun all eight prior process proofs as regression.

Run scoped Linux/Windows regression, current/MSRV all-target checks,
warnings-denied lint/Rustdoc, format and docs/book/performance-contract checks.
Retain exact-source capture hashes and all negatives in a read-only verified
packet. Frozen qualification inputs and thresholds remain unchanged.

## Remaining lifecycle boundary

This establishes only local refusal of modeled changed contexts in fresh
readers. It is not host reboot handling, whole-host rollback resistance,
authenticated peer/time, production issuer enrollment, cell completion or
writer/cgroup cleanup. P1 cells two through four and P2 through P5 remain open.
