# Owned attempt transfer to initial Reserved publication

This P1 slice connects the runtime-acknowledged early attempt to the initial
Reserved publisher. Baseline is `89fdbd9540c9aed8a3cacd9af9defc52c9ba3403`. The governing
[intermediate plan](diagnostic-server-readiness-intermediate-plan-20261012.md)
still leaves later cells, trusted inputs and execution closed.

## Preregistered ownership transfer

The new fixed-only API consumes `FixedEarlyAttemptRead` by value. It first
checks the complete original revision-one Reserved snapshot with zero completed
cells and revalidates the original attempt/authority/intent. A mismatching
unbound assertion retains the supplied guard and lock in the failure without
revoking healthy originals. Any bound error remains sticky and retains the
original lock, first failure and refusal uncertainty.

The transfer moves the original fence and original sealed attempt document into
the existing publication storage. It never releases/reacquires the lock or opens
a retained file to reconstruct acknowledgement. Subsequent pathname opens are
identity checks against that owned descriptor, not adoption. The runtime
acknowledgement, original descriptor/stamp/bytes and exclusive authority borrow
are conserved through the move.

The new publisher checks exactly the owned attempt before initial state creation,
then exactly attempt plus original created state. It uses the same eight
Reserved publication stages, canonical schema-one state, exclusive creation,
0600 mode and sync/acknowledgement barriers. Attempt bytes, inode and metadata
cannot change. A state that appears late is never overwritten/adopted.
Failure retains both any created state and the original attempt.

Success returns the existing opaque Reserved-publication guard, carrying the
original attempt. The existing cell-one consumer must therefore retain and
revalidate this owned attempt at every namespace boundary. Its cell-one schema,
previous-record-null rule and 0400 record/commit files are unchanged. An attempt
filename is allowed only when its original document is owned: no general
allowlist, recovered-history conversion or legacy publisher adoption is added.

## Preregistered proof

Baseline the early-attempt and existing publication/consumption tests. Retain a
missing-entry red test before implementation. Cover healthy publication and
cell-one continuation, all eight logical publication interruptions, full-state
mismatch, refused input, late target/context, original attempt and state drift,
expired/changed original intent, unsafe/advanced authority and partial refusal.
Test independent concurrent chains and keep legacy attempt context refusing.
Compile-fail tests must prohibit duplicate ownership and non-attempt/fixture
entry.

Extend the exact owned-process harness with `attemptreserved` and
`attemptconsumption` phases. The first kills at all eight Reserved stages
and after return, plus one uninterrupted control. BeforeCreate now retains
the already acknowledged attempt and must refuse fresh reservation.
The second kills at all fourteen cell-one consumption stages and after return,
plus one control. Every case requires original lock contention/release, helper
termination, unchanged retained bytes/metadata and two fresh read-only processes.
Seed `0x074d` permutes the full matrix; use three identical-source repeats.
Register both ignored/manual gates separately. Rerun earlier process matrices
as regression because the private harness and shared namespace checks change.

Run format, current/MSRV all-target checks, warnings-denied Clippy/Rustdoc,
proportionate functional/concurrency regressions and documentation checks.
Retain exact source, raw commands/logs, negative observations and a read-only
packet verifier. This is software process-loss proof, not power-loss or reboot.

## Compatibility and remaining boundaries

No new filename, durable schema, wire version or state advancement is added.
The early-attempt file remains 0400 throughout; state remains initial Reserved.
Legacy empty-fence readers/campaign admission reject retained context. Old
publishers cannot adopt the new runtime chain. No migration, deletion, repair,
restart, output, CLI/service start or host enrollment is enabled.

This connects only early attempt, initial Reserved and existing cell-one
retention. Fixed cells two through four, new signed state/history progression,
incomplete/reboot handling, authentic outcomes and cgroup cleanup remain open.
P2 through P5 and product qualification/ship gates are unchanged.
