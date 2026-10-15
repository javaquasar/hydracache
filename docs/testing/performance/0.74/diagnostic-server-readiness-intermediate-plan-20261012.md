# Intermediate plan for the diagnostic server pilot

This is the W11 completion sequence approved on 2026-10-12, subordinate to the
[0.74 release plan](../../../plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md)
and [repository rules](../../../RULES.md). It is not a replacement release plan,
qualification amendment or product activation. Starting source is
`ceee4bc7b5b45a7dcb7cc0f018a8e74bb9fbbea8`. Initial Reserved publication and
cell-one consumption are locally implemented; later lifecycle and execution
remain closed. The [readiness audit](production-readiness-and-server-launch-20261011.md)
describes the existing primitives and host prerequisites.

## P0 Actual process interruption and lost acknowledgement

Run bounded local Linux nonroot subprocess fixtures. Parent owns every temporary
directory and exact child handle; kill only that child with SIGKILL at logical
write/sync checkpoints. Cover all eight Reserved-publication stages and all
14 consumption stages, plus death after each successful API return. Include
uninterrupted controls. Confirm actual signal death, original lock contention
before death and lock release after death, unchanged retained bytes/metadata,
and refusal of existing context in a fresh read-only process. No crash attempt
may start a replacement writer, repair data, append a refusal after the owner
died, or reconstruct an acknowledged runtime guard. A before-create crash has
no durable mutation; empty-fence eligibility there is not global nonce replay
protection. Track that earlier lifecycle boundary in P1.

Use seed `0x074d`, three repeats and separately registered ignored/manual proof
gates. Bounded readiness timeouts diagnose a stuck test harness; they are not
correctness or performance thresholds. Actual process loss does not prove
power-loss durability, host reboot recovery or installed-root enrollment.
Exit gate: complete named crash/control matrix, no surviving owned helper,
fresh-process observations and an independently verified exact-source packet.
P0 starts immediately; it opens no service route or server workload.

## P1 Fixed lifecycle and remaining cells

Preregister transitions for cells two through four using the original authority,
fence and retained history. Define legal Reserved/Starting/running/terminal
transitions and exact state revision, completed-cell and previous-record rules.
Durable intent precedes any future execution. Refuse duplicate cell/request/nonce,
partial pairs, uncertain acknowledgement, stale boot/epoch, unknown context and
changed original objects. Reopening history is inspection, never permission to
replay. Specify crash before the first durable intent and reboot handling without
claiming durable revocation from an in-memory latch.

Exit gate: transition/property/concurrency/regression tests, fault matrix at
each new write boundary, no overwrite/adoption/retry, and compatibility review
for any durable change. P0 success alone does not close P1.

## P2 Trusted peer time and external authority

Implementation `f2dbbe64c0fbf0a0eeca3bf6e8e2177188bc7f67` adds the separate
[kernel input path](diagnostic-kernel-admission-design.md): one recvmsg binds
payload to kernel credentials and original sender pidfd, with proc/namespace
and kernel-clock checks. Portable model and campaign APIs remain unchanged.
P2 stays open until independent production issuer/digest/epoch and host trust
are loaded and joined to original authority/fence admission. See the
[pre-server checklist](pre-server-closure-checklist-20261014.md).

Connect peer credentials from the original kernel IPC context rather than caller
assertions. Bind controller provenance, admitted host/boot/namespace/account
context, bounded freshness checks and monotonic lease/progress authority. Never
use wall-clock time as epoch/version authority. Load independently selected
production issuer keys, digest and epoch pins from reviewed trusted ownership;
never enroll fixture keys, synthetic identities or envelope-supplied trust.

Exit gate: valid original-context admission and wrong-peer/signature/epoch/boot,
clock-change, replay and substitution refusals, with no secrets in evidence.
Selecting real operator keys/pins is a required operator input before enrollment.
Whole-host snapshot rollback remains outside the selected local authority model;
adding an external monotonic authority would require a separate decision.

## P3 Production output start watchdog and cleanup

The operator-approved 2026-10-14 [collector amendment](diagnostic-collector-design.md)
replaces future worker-owned regular-file output with worker pipes and a sole
root-owned evidence-file writer. Old signed projections remain unchanged. Local
collector drain is a prerequisite, not authenticated completion or cgroup
revocation; original start and trusted admission must still join the result.

Implementation `186d30bab68ce541050c8c638c047bdc49021086` adds the bounded
pipe-to-file writer with a nonroot fixture entry; see its
[EOF and process-loss evidence](local-runs/diagnostic-collector-20261014/README.md).
It retains partial attempts and returns local drain data only after both kernel
EOFs, retained byte/hash reconciliation, sync and explicit descriptor close.
The separate [signed collector requirements](diagnostic-collector-intent-design.md)
now bind the new pipe-to-collector output contract without accepting legacy
regular-file signatures. They are byte-only requirements, not a fulfilled launch
or a descriptor transfer. Root enrollment, trusted admission, original start and
watchdog/cgroup revocation remain open. No fixture result advances P1.

Prepare root-owned fenced production output for the exact reserved cell, with
exclusive creation and retained ancestry/stream identities. Bind the reviewed
loaded executable, actual environment, original streams, process generation,
unit and cgroup to the signed intent. Wire only the narrow typed diagnostic
backend; expose no arbitrary command/path/descriptor or fixture conversion.
Implement autonomous lease/progress watchdog, writer revocation and uncertain
terminal retention. Release requires original writer cessation and proof that
the exact owned cgroup is empty, not PID-only liveness or a replacement process.

Exit gate: original-start and bounded-write tests; cancellation, supervisor/
controller loss, hang, lease expiry, replacement and cleanup-failure canaries;
no automatic restart or incomplete attempt promoted to a successful receipt.

## P4 Reviewed installation and dedicated-host preflight

After P1 through P3 pass, freeze the exact tooling source/build/config identities
and signed bundle. Verify installed source receipt and signatures, artifact
hashes, trusted directory ownership, actual nonroot UID/GID/group set, systemd/
cgroup-v2 capabilities, filesystem/space limits and admitted host/boot context.
The deployed worker-owned campaign layout requires a reviewed quiescent ownership
and coordinated binary rollout; do not silently chmod/chown it into eligibility.

Host inspection starts read-only. An active campaign, diagnostic/pending marker,
lifecycle fixture or uncertainty blocks maintenance. Any separately approved
maintenance stop targets only `hydracache-performance-supervisor-074.service`,
with explicit restoration, receipt and health verification. Do not interrupt
0.73 or unrelated campaigns. Exit gate: exact-host positive/refusal preflight and
installation/restoration evidence; local modeled roots cannot satisfy it.

## P5 Nonproduct host lifecycle rehearsal

Run a bounded separately reviewed nonproduct positive lifecycle followed by fault
cases: signatures/pins, revoked epoch, task/namespace/credential churn, original
object substitution, incomplete/overflow output, manager/controller/supervisor
loss, stalled progress, lease expiry, uncertain journal and incomplete cleanup.
Host reboot testing needs its own quiescent maintenance approval. Retain every
attempt, prove zero duplicate executors, writer cessation and original empty
cgroup, then restore and verify the stable supervisor.

Exit gate: independently verified host packet and acceptable controller overhead.
Only then define and separately admit a finite numerical pilot with frozen
workload, payload/keys, seed, concurrency, duration, estimator and thresholds.
P5 is not a product candidate run, six-hour qualification or 24-hour confirmation.

## Product release remains a separate boundary

[Composition](composition-ledger.toml) currently has zero accepted product
candidates and C74 is unresolved. [Release admission](release-admission-contract.toml)
remains closed. A product release still requires an admitted measured candidate,
exact I74/C74/Redis/runner identities, native nonregression and compatibility,
focused pairs, independently approved six-hour/24-hour stages, exact packages,
supported targets, dependency/SBOM receipts and immutable nested-hash archive.
The inherited plan/qualification digest mismatch needs explicit reviewed
reconciliation before qualification. No frozen input or threshold is changed by
this intermediate plan. Passing P0 through P5 does not itself close release 0.74.

## Evidence and execution cadence

For every package retain baseline, red test, implementation, targeted repeats,
checks/lint/format and proportionate regression. Use small logical commits and
separate evidence; update this checklist's status with the exact packet, never
with a readiness score. Full verification is required at milestones and before
merge/tag. Keep historical negative results immutable and distinguish local
functional, real process-loss, exact-host and numerical qualification evidence.

## Status after the local process interruption package

P0 local process-loss proof is complete for implementation
`921050e3ab8be41b136ada1b3a6e6bf30a950693`; see the
[exact-source packet](local-runs/diagnostic-process-crash-20261012/README.md).
All 24 SIGKILL cases and two uninterrupted controls pass in each of three
repeats, with two fresh read-only process inspections per case. Original lock
release, helper termination and retained bytes/metadata are confirmed locally.
Power loss, reboot and positive installed-root enrollment are not established.

P1 through P5 remain open. Next preregister fixed state/cell progression and
crash before the first durable intent, preserving the no-replay rule rather
than treating empty-before-create as durable nonce protection. Real production
issuer/pins and trusted host enrollment remain external prerequisites. This
status update changes no qualification input, product candidate or ship gate.

## First P1 transition slice

Implementation `4085e4e9b587fb5d99e0ed271e7ec2ad251bf0f5` adds adjacent-state
checks to the local model publisher, with a
[preregistered graph](diagnostic-transition-local-design.md) and
[exact-source evidence](local-runs/diagnostic-transition-20261013/README.md).
Revision, identity/origin, clock assertions, cell count, original start/inode
and stop reason are checked before mutation. Terminal replay refuses.

P1 is not complete. The fixed runtime publisher still pins immutable initial
state/intent; cells two through four need an explicit owned progression protocol.
Early attempt/nonce admission, signed previous-record history, new-boundary
process-loss cases and reboot handling remain. This validator proves neither
authentic completion nor empty cgroup and does not open P2 through P5.

## P1 Early attempt retention

Implementation `cdeb2c8c8f308161644b9fc9bf224282da2c2dad` adds the
[original early-attempt primitive](diagnostic-early-attempt-local-design.md),
with [local evidence](local-runs/diagnostic-early-attempt-20261013/README.md).
It retains the signed initial attempt before Reserved publication, acknowledges
only after owned-object checks and sync, and blocks fresh reservation for any
retained partial or complete context. BeforeCreate remains unadmitted, not
global nonce proof. Surviving bytes cannot recreate a runtime acknowledgement.

P1 remains open: connect this owned guard to Reserved publication and later
cells without reopening/adoption or weaker original bindings. Signed history,
incomplete-attempt handling and reboot semantics remain before P2 through P5.
No fixed execution route, server operation, product candidate or qualification
input is opened or changed.

## P1 Owned attempt transfer to initial state

Implementation `6363ab7c80efd2af6524a153f56b4ab8db9d12b7` connects the original
acknowledged attempt to initial Reserved and existing cell-one retention.
See the [transfer contract](diagnostic-attempt-publication-local-design.md) and
[exact-source packet](local-runs/diagnostic-attempt-publication-20261013/README.md).
Original attempt descriptor/stamp/bytes, authority borrow and fence survive the
consuming transfer; no lock gap, recovered-file adoption or new schema is added.
Cell-one namespace checks retain and revalidate the inherited original.

P1 remains open. Fixed cells two through four need signed next-state/history
rules and owned progression, including incomplete/reboot refusal. This package
does not authenticate caller outcomes or time, establish writer cessation/empty
cgroup, or open execution, installed-host rehearsal or qualification.

## P1 Signed supplied history inspection

The [history slice](diagnostic-signed-history-local-design.md) verifies bounded
supplied cells one through four against the exact initial-attempt anchor,
original signature pins, unique request/nonce and previous raw hashes.
Reserved projections require at least three revisions per completed cell,
allowing additional heartbeats. These are necessary consistency checks, not a
retained transition trace or proof of completion.

P1 remains open. Observation cannot become an append/start guard or recovered
acknowledgement. The outer previous-record hash is not covered by the intent
signature. Next define the owned state progression and its completion boundary,
preserving the original fence, authority and attempt rather than reopening or
rebinding them. Incomplete/reboot refusal, P2 through P5 and ship remain closed.

## P1 Owned initial departure record

Implementation `07008eb43eb6606c7726d7d27799e995d2cddf6b` retains an owned
revision-two Starting intent after the original attempt, initial state and
cell-one record/commit have been acknowledged. The
[Starting contract](diagnostic-starting-intent-local-design.md) consumes that
original runtime chain by value and preserves its common fence and object
identities. A new exclusive sealed record binds the original state, cell-one
record and signed-envelope hashes to the validated adjacent candidate.
The initial active state is not overwritten or treated as executing.

P1 remains open. No Starting-to-Running conversion, worker start, authenticated
completion, next-cell intent or recovered acknowledgement is added. Later
owned progression, expiry/incomplete/reboot refusal and actual completion/
cleanup boundaries remain before P2 through P5 and server admission.

The [final-source packet](local-runs/diagnostic-starting-intent-20261013/README.md)
uses `099bb14da0d17069913287a528ca2efc5a05caee`, whose only change after
the runtime implementation is the existing transition fixtures' original
fence retention. It includes three complete functional/process repeats and
the seven-phase SIGKILL regression, preserving all development/scheduling
negatives. P1 is still not complete; no execution or host boundary is opened.

## P1 Owned incomplete attempt quarantine

The [quarantine slice](diagnostic-starting-quarantine-local-design.md) consumes
an acknowledged original Starting guard into permanent refusal while holding
the same original chain and fence. It attempts only the existing authority
refusal, using captured pins. It introduces no new durable format or attempt
mutation. Its explicit disposition cannot authorize execution, completed-cell
publication, cleanup or re-admission, including when authority persistence is
uncertain or the old launch envelope would now be expired.

P1 is not complete. Next test real process loss during this authority append
and acknowledgement, preserving partial tails and read-only fresh inspection.
Later progression requires authentic completion and original writer/cgroup
evidence; no caller-asserted outcome can promote Starting to Running or Terminal.
Reboot handling, cells two through four and P2 through P5 remain open.

Implementation `9875fa44a58b45f25fde249c9c8038bf946c6425` has a
[local exact-source packet](local-runs/diagnostic-starting-quarantine-20261013/README.md)
with three repeated eight-case matrices and scoped regression. This closes only
the owned quarantine API slice, not P1 or server readiness.

## P1 Local authority refusal process loss

The [quarantine process-loss proof](diagnostic-quarantine-crash-local-design.md)
adds test-only observation at the existing four authority append boundaries,
plus death before refusal and after return. Retained reservation context refuses
new admission even when the original authority remains active. A partial tail
is invalid; a sealed refusal pair is readable data, not recovered acknowledgement.
Two fresh processes per case inspect without append, repair, retry or adoption.

This closes only the specified local SIGKILL boundary. It does not close P1.
Reboot/context semantics, authentic completed
cell evidence and cells two through four remain before trusted production inputs,
execution/watchdog/cleanup and installed-host rehearsal. No server operation or
qualification input is changed.

Implementation `86a52311a4094d657a0ac59ede3310f1731b581d` passes three
six-SIGKILL/control repeats and the eight-proof process regression; see the
[final-source packet](local-runs/diagnostic-quarantine-crash-20261013/README.md).
The normal non-test library has no observer symbols. The local authority
process-loss matrix is complete; P1 and server admission remain open.

## P1 Fresh reader context replay refusal

The [context replay slice](diagnostic-context-replay-local-design.md) checks
modeled boot, machine, namespace and trust changes in fresh processes after
owned SIGKILL. It uses the existing journal Binding refusal and does not mutate
retained files to fit the new claim. Partial tails refuse Invalid before binding;
their rejection is not claimed as an evaluated boot/pin mismatch.

This covers local changed-claim refusal only. Actual reboot, namespace switching,
whole-host rollback resistance and production issuer enrollment remain unproved.
No observation recovers a guard or admits execution/cleanup. Fixed cells two
through four still require authenticated outcome and original writer/cgroup
evidence; P2 through P5 and server admission remain open.

The [exact-source packet](local-runs/diagnostic-context-replay-20261013/README.md)
pins implementation `4302202b12d90f0708c029929b6f7f51c6f38aea`.
Three targeted repeats pass 18 SIGKILL cases, three controls and 336 fresh
inspections. This closes the modeled fresh-reader refusal slice only; it does
not close P1 or open any host operation.

The separate nine-proof regression passes 78 SIGKILL cases, eight controls and
270 fresh inspections, preserving every prior process-loss proof. The next P1
slice must establish authenticated outcome/progression before implementing
cells two through four; a retained reader cannot supply that authority.

## P1 Completion prerequisites and original terminal observation

The [completion boundary](diagnostic-completion-boundary-local-design.md) fixes
the prerequisite ordering: authenticated completion requires P2 trusted
admission and P3 original-start/writer-revocation authority. P1 grammar and
negative tests can precede those producers; durable Running/completion and
next-cell publication cannot. This does not enroll any new execution policy.

ProcessRead.observe_terminal now distinguishes exact retained pidfd exit/hangup
readiness from errors. It does not obtain an exit code or final cgroup context,
reap, signal, start or authorize cleanup. A nonproduct leader/descendant fixture
proves an actual stream write after the leader's SIGKILL. Descendant signal
ownership is acquired only after kernel-parent and original-generation checks.
This closes a necessary observation gap, not P1 completion or cells two through
four. The next implementation must own and authenticate original start and
writer revocation, then journal completion under the retained authority/fence.

## P2 Original controller attempt process loss

The [controller attempt](diagnostic-controller-attempt-design.md) consumes its
original common-fence preflight into one durable Reserve request without
releasing the lock or replacing the original controller/kernel/authority chain.
Its separate [process matrix](diagnostic-controller-attempt-crash-contract.toml)
now checks eight write cuts, AfterReturn lost runtime acknowledgement and an
uninterrupted control with actual owned pidfd SIGKILL. Two fresh readers per
case preserve bytes/metadata and refuse every retained partial or complete
context. BeforeCreate stays empty and unadmitted.

The sender is the same synthetic fixture child. This closes only the local
joined process-loss boundary, not independent production policy/epoch/job
enrollment, global nonce consumption, reboot or rollback. The implementation
still has no lifecycle/start conversion. Keep authentic original-start,
collector/output ownership and autonomous all-writer revocation ahead of
completion and cells two through four; P1 through P5 and server admission
remain open.
