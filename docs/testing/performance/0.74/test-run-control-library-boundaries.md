# Boundaries for a reusable test run controller

The approved direction is a reusable library plus an autonomous supervisor,
with HydraCache-specific policy in an adapter. For release 0.74 this records
boundaries only: no crate extraction, package publication, new stable API or
change to signed formats is authorized by this note. Complete the current
[P2 and P3 lifecycle](diagnostic-server-readiness-intermediate-plan-20261012.md)
and local crash gates before moving implementation across crates.

## General control core

The future platform-independent core owns run/attempt identity, legal state
transitions, bounded progress/checkpoint validation, lease decisions, journal
integrity, uncertain outcomes and offline evidence verification. Metric names
and progress rules must come from a reviewed finite adapter contract. Wall time
may constrain signature freshness; monotonic deadlines and authority epochs
remain distinct. Counters do not authenticate successful completion.

Passive observation is a separate capability from start, cancel, revoke and
release. Reading a journal cannot reconstruct an acknowledged runtime owner or
retry an incomplete attempt. Observation APIs must not expose writable handles,
arbitrary commands, unchecked paths or replacement-object adoption.

## Linux execution backend and autonomous supervisor

Linux-specific code owns kernel credentials, original pidfds and namespace
objects, systemd invocation/cgroup identity, bounded output collection and
whole-writer cessation. It refuses unsupported guarantees explicitly; portable
model tests cannot masquerade as production backend evidence.

The supervisor runs outside the controlled workload's process/cgroup lifetime.
Embedding a library in the worker does not provide an autonomous watchdog after
worker death. The root collector alone owns evidence-file write descriptors;
workers receive pipe writers only. Completion needs original exit outcome,
original revoked/empty cgroup, both actual EOFs and collector sync/close, not
leader death or a successful unit-name lookup. Uncertain cleanup retains failure.

Privileged constructors remain private and accept owned admitted capabilities,
not path/FD/config claims supplied through the control socket. Fixtures cannot
convert into those capabilities. Keys and credentials are not collected as
telemetry; detailed run identifiers belong in evidence, not metric labels.

## HydraCache adapter

The adapter owns I74/C74/B73 and Redis identities, the fixed four-cell diagnostic
preset, workload and payload/key corpus, selected issuer/principal pins, authority
epoch policy, host admission, phase-specific progress and qualification budgets.
It maps validated generic events to release evidence without changing estimator,
duration, thresholds or native nonregression semantics.

Existing repository IDs, fixed service/directory names, signed domains and
schemas remain in this adapter until a separately versioned compatibility design
exists. Do not mechanically rename historical signed bytes or reinterpret old
direct-file output as the newer pipe/collector contract.

## Extraction acceptance

Before extraction, capture the completed lifecycle baseline and freeze the exact
test/evidence inventory. Move one component at a time, retain nonforgeable owned
guards and test failure/drop behavior across each new boundary. Public APIs need
positive and compile-fail lifetime tests, documented platform guarantees and
versioning. Existing receipts stay verifiable against their original formats.

Prove that observation never gains execution rights, controller loss never
creates a second executor, a dead process cannot resume a continuous experiment,
and incomplete output cannot become green. Repeat kernel/EOF/crash and integrated
lifecycle tests after movement. Separately measure CPU, memory, I/O and timing
overhead on the admitted host; low overhead is not assumed from code separation.

Publishing a standalone library or enabling another workload/backend requires
its own explicit scope and admission. These boundaries add no release gate and
do not postpone the current P2/P3 implementation for generalization.
