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

The [original collector preflight](diagnostic-original-collector-design.md) now
checks those signatures through the acknowledged request's same original kernel,
worker/authority/controller and common fence, with no caller peer/time inputs.
This read-only guard still requires an original durable lifecycle/root-ancestry
join before construction: supplied signed Reserved state is not a start permit.

The separate [observed reservation owner](diagnostic-original-collector-design.md#observed-initial-state-under-the-same-owner)
now mints initial state from the retained receiver's kernel boot/boottime and
freezes it for collector signing. It consumes the request owner and accepts no
caller state/clock in the distinct collector binding. This closes the asserted
clock gap for that new path, not the durable publication gap. P1/P3 still require
owned Reserved/Starting acknowledgement and trusted root ancestry before output
creation, followed by authenticated original start and all-writer cessation.

The separate [collector Reserved publisher](diagnostic-collector-reserved-design.md)
now implements initial durable retention under that same original owner/fence,
without reusing the legacy regular-file publisher or changing its inventory.
It adds an explicit COMPAT format and returns no start/output capability.
Starting, fresh controller progress, root ancestry and execution/cessation
integration remain open; a healthy retained-record guard is not lease renewal.
The [local process evidence](local-runs/collector-reserved-20261016/README.md)
passes three repetitions: 30 owned SIGKILLs, 3 controls and 66 fresh inspections,
including lost acknowledgement and refusal before new-record creation.

The subsequent [authenticated heartbeat preflight](diagnostic-collector-progress-design.md)
now retains a new signed kernel message with the same original reservation and
fence. It checks actual receive-time freshness and original controller/identity/
revision without caller peer/clock inputs. This is not a committed heartbeat,
renewal of the original admission hold, proof of uninterrupted presence or a
Starting transition. Those lifecycle/deadline and uncertain-outcome semantics
remain part of the consuming Starting protocol before output construction.

The subsequent [causal challenge](diagnostic-collector-causal-design.md) binds
one response to a system-random commitment generated after Reserved acknowledgement,
including original request/record/state hashes. This closes the queued-old-message
gap for that distinct preflight, not durable progress or runtime lease authority.
It does not discharge any admission predicate or prove continuous presence.
Its [local packet](local-runs/collector-causal-20261016/README.md) retains passing
isolated current/MSRV proofs but a still-red ordinary regression (98 passed,
one old heartbeat Authentication(Time) failure). Attribute and address that
validation-cost/window interaction before treating the handoff as ready; do not
widen deadlines or suppress the old test.

The [validation-cost probe](diagnostic-validation-cost-design.md) now attributes
repeated full worker-policy verification in the local debug fixture. A new
ordinary repeat passes 99 tests without reproducing Time, not proving its
historical cause or a fix. Isolate any immutable-proof reuse proposal while
retaining live configuration/epoch/context checks, and separately attribute the
retained old IPC Host(Busy) occurrence before treating robustness as closed.

The first [proof reuse candidate](local-runs/worker-proof-reuse-20261016/README.md)
passes focused differential tests but is withdrawn after the first paired
candidate run fails its preregistered Setup counter: two constructors execute,
not one. The screen stops without retries or a gain estimate, and the isolated
candidate is reverted. The separately preregistered
[second screen](local-runs/worker-proof-reuse-v2-20261016/README.md) now preserves
both constructors and passes the unchanged practical floor in five local debug
pairs. Live checks and signed windows stay unchanged. Both compiler regressions
pass without IPC retries, not proving historical Host(Busy)/Time/Kernel(Invalid)
causes or production readiness. Original-start and writer cessation remain below.

The separately preregistered [short-fence investigation](local-runs/short-fence-20261016/README.md)
now proves and corrects one reachable Host(Busy) mechanism: a pre-exec fork alias
outliving the raw File returned by a completed short transaction. An owned guard
explicitly unlocks only the successfully acquired original description. The raw
close control remains Busy; the three baseline regressions fail, then all four
controls and seven corrupt-ledger variants pass after correction. No old IPC test
is retried or relaxed. Historical occurrence-specific attribution remains open.
The short-helper correction deliberately left campaign construction unchanged;
the separate follow-up below closes that gap. Neither correction is all-writer
cessation, original-start, or requalification of prior timings.

The [campaign-construction follow-up](local-runs/claim-construction-20261016/README.md)
reproduces five failing baseline groups while successful claim ownership already
passes. A private pending owner now releases the original successful flock on
every early constructor return, including recover_active returning None, and
consumes its Option<File> exactly once into the existing successful claim. Six
tests cover 23 cases: early errors, fixed-context conflicts, absent active claim,
late refusal retaining the marker and successful transfer. Predicate ordering,
marker retention, final claim Drop and terminal release stay exact. The next
implementation boundary remains the original authenticated start and autonomous
writer cessation described below, not release admission.

The [explicit start-consent bytes](diagnostic-start-consent-plan-20261016.md)
now separate first-cell launch consent from the existing Heartbeat/challenge
protocol. The signed body binds original attempt/Reserved/state/challenge/response
and policy/unit hashes to the first Starting projection and fixed controller/
source/preset. Signature inspection is deliberately data-only, with no asserted
peer/clock admission API or runtime guard. Next derive/freeze Starting under the
consumed original causal owner, compare every signed claim using its retained
kernel context and current policy/epoch/fence, and retain the durable adjacent
transition/replay intent before any output or process creation. No bootstrap
deadline is discharged or renewed by parsing a signed body.

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

## P3 Signed expected placement does not open original delivery

The [separate held placement contract](held-placement-design.md) binds exact
v2 Enroll bytes and nested launch claims under a new v3 domain, deriving safe
expected slice/child names from the same identity. This closes only the signed
placement-data slice. The old direct-worker manager message remains unsent;
actual root placement, original owner/context/bound join and a complete held
unit must precede authenticated delivery. All live checks and original bounds
are unchanged. Production enrollment, all-writer cessation, completion, cells
two through four, reconnect and server rehearsal remain separate open gates.

## P3 Same-owner placement constraints and incomplete live gate

The [consuming join](owned-placement-design.md) now connects signed expectations
to the original retained controller-policy key, complete consent and exact Armed
guardian root/context/deadline. It owns the whole manager/root chain on success
or refusal and repeats the unchanged full live round. Actual fixed slice ancestry,
approved executable artifacts and a held-unit projection remain unimplemented;
this is not authenticated original delivery or production enrollment.

Ordinary GNU/MSRV regression and all 312 source guards pass. The fresh privileged
GNU cohort passes healthy, revalidation and never-Armed custody/replay refusal,
then fails the old Causal(Time) check during the next root preparation, before
wrong-key binding. MSRV is unexecuted and the complete live gate remains false.
Capture the actual preparation comparison with test-only existing causal/raw-age
observers before proposing an isolated cost improvement, preserving all checks
and original deadlines. P1 through P5 and server admission remain open; no
installation, supervisor operation or product workload was performed.

## P3 Actual preparation expiry and remaining GNU gate

The [expanded preparation trace](placement-preparation-design.md) now copies the
new GNU refusal's actual issued/now/boot: 884474515163 → 894476360325 ns, age
10001845162 ns with identical boot IDs. It is original-window expiry before
placement binding, not a wrong-key negative. This does not recover the missing
historical tuple. The independent MSRV seven-case cohort passes once; GNU and
the complete two-compiler gate remain open. Attribute preparation cost by fixed
phases next, preserving every live check and original origin/bound. Actual slice,
artifact admission, held projection and subsequent P1–P5 work remain unchanged.

## P3 Complete local placement constraints matrix

The [fixed phase source](placement-stage-cost-design.md) retains three GNU cost
probes separately from one subsequent normal two-compiler matrix. All seven
same-owner cases pass once on GNU and once on MSRV with cost capture disabled,
keeping every original timer, EOF/custody/fence/replay assertion and absolute
bound. This closes that local matrix only. The preceding GNU expiry remains a
valid retained negative; runtime code is unchanged and no speedup is claimed.

Outputs is the largest measured phase and repeated kernel/context observations
are the next attribution direction. Do not cache live authority or remove checks
to enlarge its budget. Actual fixed slice ancestry, approved artifact objects and
held-unit projection still precede authenticated original delivery, all-writer
revocation, completion, cells two through four, reconnect and server rehearsal.

## P3 Read only fixed slice identity binding

The [next private holder](fixed-slice-design.md) consumes that same placement
owner and checks fixed-path directory identities before and after its unchanged
full live join. Its original guardian root must be the derived named child of
the observed system.slice; retained and fresh path anchors must agree. Refusal
retains the consumed original chain and offers no replay or descriptor escape.

This implements path observation, not systemd creation/ownership, continuously
exclusive population or a successful actual fixed-root join. Temporary fixture
roots are not promoted to that role. Approved artifact objects, held projection,
authenticated delivery and the later P1–P5 gates still remain. The old direct
worker message remains unsent. There is no server or supervisor operation.

Selected source c59b677f passes 321 source contracts and both compiler scoped
unit/doc/readonly-anchor gates. This does not close actual fixed-root acceptance.
The broader Windows integration suite also exposes the unchanged synthetic
root-lock reconstruction mismatch. Repair that fixture with the exact frozen
lock bytes in a separate change before treating the broader integration gate
as green; preserve the production hash. See the
[selected-source packet](local-runs/fixed-slice-20261020/README.md) for the scoped
successes and retained failures, rather than promoting either to server readiness.

## P3 Ordinary synthetic artifact and receipt gates restored

The [test-only immutable lock repair](frozen-lock-fixture-design.md) replaces two
moving-workspace inputs with the exact already-pinned worker lock. Existing
production pins, runtime sources, signature/content checks, old test assertions
and original bounds remain. On 84fee2cc the complete ordinary Windows monitor
package passes 296 tests and twenty docs; both GNU compilers pass all 31 artifact
and nine receipt cases with strict lint. The broader ordinary Windows fixture
defect is closed on that selected source, not retroactively at c59b677f.

Next still requires an actual positive fixed-root join with manager ownership
and exclusive population, authenticated held launcher/watchdog/collector objects
and a complete held-unit projection. Keep original delivery, all-writer cessation,
durable completion/later cells, reconnect, production enrollment and server
rehearsal behind their distinct gates. No live matrix, server installation,
supervisor operation or product run is authorized by synthetic fixture success.

## P3 Private kernel fixture before manager owned admission

The [isolated namespace fixture](fixed-slice-namespace-design.md) tests actual
cgroup2 directory identity through the unchanged fixed-path observer, not fake
Stamp data. Every mutation requires a preceding successful pin and revalidation.
The separately owned child makes its mount propagation private; the parent
retains its original host anchors and removes only exact empty temporary paths.
The thread-self namespace correction and kernel-supported remove/recreate
mutation preserve every case and expected refusal instead of accepting an
earlier fixture error as a security check.

An accepted private kernel pin does not join a manager-owned slice to the original
attempt. That full join, controlled population, authenticated held role objects,
complete held projection and original delivery remain necessary before writer
cessation, durable completion/later cells, reconnect or server rehearsal. The
old worker builder receipt cannot authenticate three newly declared artifact
roles. Keep its fixed recipe and old direct-worker message unchanged and unsent.

## P3 Held artifact topology and build authority decision

The [boundary audit](held-artifact-boundary-audit-20261020.md) separates logical
roles from actual executables. Current watchdog/collector keepers are private
raw-clone continuations; the collector remains quarantined. There is no production
held-launcher target or new original fixed-placement CLI driver. The existing
worker builder policy pins only the frozen worker recipe, not any of these roles.

Approve the minimal topology and purpose-bound builder contract before changing
that authority: one new held launcher, existing keepers in the admitted host
image, separate role receipt/policy and an explicit versioned original binding.
An active collector requires its own admitted handoff; it is not the current
quarantine renamed. Preserve the old worker contract and every original live
predicate/deadline. No production path, key enrollment, host operation or original
delivery is authorized by the proposal or by unchanged artifact regressions.

## P3 Approved purpose-bound static build authority

The subsequent operator continuation approves the minimal local topology and
[build-authority plan](held-build-authority-plan-20261020.md). At 48854c1e the
separate policy, constrained signer and receipt/content verifier are implemented,
without extending the frozen worker policy. The same controller must authenticate
both independently pinned policies; the three keys differ. Receipt checks bind
the policy/epoch, exact source/recipe, distinct host/helper images and fixed role
map. These are static assertions, not a live authority epoch or admitted files.

Next implement the real held-launcher bootstrap and one-shot pinned worker exec,
then protected role build production and retained objects. Join those objects to
original policy/epoch, fixed root, manager and full frames before held projection
and delivery. Active collector handoff, joined SIGKILL/lost-ACK, all-writer
cessation, durable lifecycle and reconnect remain open. The separate
[evidence packet](local-runs/held-build-authority-20261020/README.md) is explicitly
synthetic and non-promotable; it authorizes no installation, supervisor operation,
system-bus/original send, production enrollment or product/server workload.

## P3 Local held exec mechanics before original admission

The [actual local helper](held-launcher-plan-20261020.md) now separates its own
pre-exec image and pinned worker FD from the post-exec worker image. Inherited
bootstrap, streams, control/owner credentials, pidfds and absolute alarm are
checked without opening a caller executable path or renewing a deadline. The
synthetic ELF gate asserts one effect, stable PID/start ticks and exact surviving
descriptors. An external owned subreaper verifies creator SIGKILL and reaps only
its admitted fixture children. This is not an armed production guardian.

The [packet](local-runs/held-launcher-20261020/README.md) distinguishes the initial
26-case cohort, the failing O_PATH regression, a test-only compiler refusal and
the final 27-case source 68b71834. It preserves all 438 previous runtime files and
old wire bytes. Original validation frames, 46 brackets and 300/60/10 bounds stay.

Next connect protected purpose-bound build production, actual retained role
objects and this helper's bootstrap to the same original policy/epoch, fixed
root, manager and complete frames. Do not send the old direct-worker message.
Before original held release, prove both guardians Armed and collector custody;
before completion/unlock, prove post-exec all-writer cessation and actual EOF.
Durable later cells, reconnect, enrollment and server rehearsal remain gated.

## P3 Retained role content before authority join

The [retained-role implementation](held-role-files-plan-20261020.md) now consumes
the checked receipt and two sealed read-only files. It validates the signed role
map against actual ELF/content and retains kernel stamps, without exporting FDs
or granting execution. Positional hashing preserves shared offsets; refusal is
sticky. The ten new Linux controls use synthetic bytes and keys, not a protected
production role build. The [packet](local-runs/held-role-files-20261020/README.md)
also records a diagnostic-renderer panic that ordinary compile-fail reporting
incorrectly calls successful; final acceptance must exclude that panic.

Next consume this retained content with the same FixedPlacement, admitted held
policy/controller/epoch, original signed claims and unchanged complete frames.
Neither static ExpectedTrust input nor content custody establishes live authority.
Then bind bootstrap/helper/worker process identity before original delivery;
watchdog Armed, active collector, all-writer termination, later durable cells and
reconnect remain required before a separately authorized server rehearsal.

## P3 Bind the role provenance documents without upgrading authority

The [new v4 contract](held-role-binding-plan-20261020.md) binds complete signed
v3 placement bytes to the held policy/receipt digests and matching epoch. Both
controller signatures use the independently checked policy's controller; the
receipt uses the separate held builder. Re-signed mismatching role/epoch/digest
data must refuse. The static inspector cannot start, send or extract an original.

Original policy/epoch observations still come from the same retained active
registry/kernel owner and unchanged complete frame, not this signed-data check.
The reviewed trust module currently selects no held builder/policy pin. Admit
those independently before consuming FixedPlacement and role files in an original
join; do not select trust from the incoming binding or reuse the old worker key.
Protected role build production, hashing attribution and joined live/SIGKILL/
lost-ACK, all-writer retirement, durable lifecycle and reconnect gates remain.

## P3 Independent held source and early controller guard

The [fixed source](held-runtime-trust-plan-20261020.md) now retains protected root
configuration and independently selects held key/pin, with reviewed old selectors
unchanged. No actual production selection/enrollment is performed. Its borrowed
OriginalHeldTrust reobserves the same original controller/kernel/active authority
before and after policy inspection, retaining request, policy/epoch/revision and
trust snapshot. Valid signed v4 data for another original request must refuse and
poison both owners. This adds no start/send capability or new time origin.

Next carry this retained selection into the late consuming join with the same
FixedPlacement, original signed claims, role objects and complete unchanged
frame. This early borrow alone cannot replace that later join. Keep production
build/enrollment, real-image hashing attribution, authenticated delivery,
Armed/custody/all-writer cessation and later lifecycle/reconnect independently
gated. No server installation, supervisor operation or workload is authorized.

## P3 Consuming original placement and retained role join

The [private join](held-consuming-join-plan-20261020.md) consumes FixedPlacement,
PinnedRoles and fixed held trust in one holder; failed custody returns no retry
owner. Complete original frames surround fresh trust/provenance and full role
checks in two constant-depth rounds. Exact signed v3 bytes, full receipt statement
and actual authority/request observations must agree. Old frame bodies, clock
origins, bounds and source selectors remain unchanged; final clocks follow hashing.

Local orchestration models and actual kernel/authority/memfd bridges are separate
from the privileged consuming-holder end-to-end gate. That gate and real cost
attribution remain next, without dropping checks or extending time. Then retain
the authenticated-delivery, guardian Armed/custody/all-writer cessation, durable
completion/later cells/reconnect and joined SIGKILL/lost-ACK gates. Root/cgroup2
availability from read-only WSL preflight does not count as a healthy composition.

The consuming candidate at d9d37a8a/f07dcca3 is not yet qualified. Compiler/model,
capability, strict lint and exact old-source guards pass, but complete local Linux
cohorts refuse original kernel inputs. Some later samples are traced to
clock-order; the first and other untraced failures remain unclassified. Local
timesyncd stop/restore, held WSL and CPU affinity do not close that gate. Preserve
all these refusals; do not use model passes or a coarse observer to admit the
real holder. Resolve original-clock input reproducibility on a stable Linux
context or through a separately scoped investigation before cost/SIGKILL and
server rehearsal. No server or Windows clock configuration was modified.

The separate [isolated Linux comparison](held-consuming-server-plan-20261020.md)
passes both GNU/MSRV rounds of the unchanged 17/26/5/38 selections, without CPU
affinity, service/clock operations, installation or product workload. Stable
Linux reproduction of these models and actual kernel/authority/FD bridges is
therefore established. The prior WSL gate remains red, and its clock actor is
not diagnosed by this environment difference. The actual privileged consuming
holder and its full-hash/four-frame cost remain the next P3 gate; neither this
comparison nor the model passes replace joined SIGKILL/lost-ACK or launch consent.

The [actual holder fixture](held-consuming-live-plan-20261020.md) now composes
the complete local original chain and consumes actual FixedPlacement, sealed
role objects and held trust. MSRV passes all five cases; GNU's final matrix
records an original clock-order refusal. The combined local gate remains red.
Separate synthetic-role CPU/wall observations do not close real-image hashing
cost, and terminal collector/custodian processes do not prove root guardian
exit. Root guardians may still retain the fence. Preserve all failed cohorts
and unchanged original bounds. The next privileged native Linux environment
control needs separate authority; no server install or product permission follows.

That isolated privileged control is now authorized and completed, separately
from any installation or product permission. The same GNU/MSRV images both
refuse the first healthy case during signed-placement binding: original causal
or custody age exceeds the unchanged ten-second window. No FixedPlacement or
new consuming-role join is reached, and no matrix is retried. The
[native refusal packet](local-runs/held-consuming-privileged-server-20261020/README.md)
retains both original reports and exits; earlier WSL failures remain sealed.

The next P3 gate is attribution before/through preparation, enrollment and
placement, not a deadline extension or premature joined SIGKILL. Keep all live
checks and original origins. Independent postflight confirms unchanged installed
artifacts, running supervisor PID/restarts, absent campaign markers and no owned
fixture cgroups/roots. Production delivery/trust, all-writer cessation, durable
lifecycle and reconnect gates remain open; server admission is still closed.

The [local phase attribution](held-preparation-cost-plan-20261020.md) now separates
post-challenge preparation, enrollment and placement, preserving original calls
and clocks. One isolated allocation candidate at 1a328593 borrows five constant
status prefixes instead of formatting them. Six paired GNU/MSRV samples confirm
10 to 5 Rust allocation requests and 120 to 80 requested bytes per successful
decode of the shared two-group fixture. The full frozen decoder/refusal behavior
is compared, and 462 preceding
runtime/support files reconstruct exactly outside named observer wrappers and
literal-prefix changes. This does not qualify whole-attempt latency or cache APIs.

The [new packet](local-runs/held-preparation-cost-20261020/README.md) retains
reviewed cost outcomes GNU 0/0/0 and MSRV 101/0/0, then two refused unprofiled
matrices: GNU healthy and MSRV revalidate setup record original same-boot
clock-order. No timer/service setting changes, retries, server access or workload
follow. All local contract/source tests and compiler checks pass, but full local
and native execution gates remain false. Next isolate the remaining expensive
fresh kernel observations without caching or skipping them; obtain separately
authorized native control before joined SIGKILL/lost-ACK or server admission.

## Fresh kernel cost boundary on 20 October

The [fresh observation controls](local-runs/fresh-kernel-cost-20261020/README.md)
retain actual origin/sender traversal and independently measure pidfd, bounded
status read, metadata and filesystem checks. The origin work remains a major
nested cost. No live observation is cached or omitted, and no deadline or
validation schedule changes.

One isolated candidate, `f3009600`, removes construction of the constant status
pathname at its two runtime sites, using the existing fixed C string opener.
The paired leaf result is one fewer Rust allocation request and seven fewer
requested bytes per explicit status open. This does not establish a latency
gain, fix prior clock-order/native expiry or open a server execution gate.

The next larger cost candidate must have its own unchanged-workload baseline
and confirmed attribution. Full functional composition, authenticated delivery,
joined all-writer SIGKILL/lost-ACK and reconnect still require their independent
gates; a native control requires separate authorization. Installation,
supervisor maintenance and product campaigns remain outside this local step.

## Uncached process key allocation on 20 October

The [PID pathname packet](local-runs/pid-path-cost-20261020/README.md) tests one
independent allocation candidate. The exact measured formatter is adopted in
`b9770cc2`: numeric credential PID is converted anew on the stack for every open,
with no retained pathname or kernel value. Twelve paired controls on GNU/MSRV
confirm two requests becoming zero; the measured three-digit keys require
14/12 reference bytes respectively. Fresh calls and original deadlines stay
fixed, and all 350 source/contract tests and compiler checks pass.

This does not demonstrate a latency improvement or repair the prior native
expiry/clock-order refusal. Full composition, authenticated delivery, joined
all-writer SIGKILL/lost-ACK and reconnect remain independent gates. No server,
installation, supervisor/clock service operation or product workload follows.

## Complete local composition recheck after the allocation changes

The [new single-cohort recheck](local-runs/held-composition-recheck-20261020/README.md)
reruns the actual privileged holder matrix with unchanged normal GNU/MSRV images
and source after all three allocation improvements. MSRV completes all five
cases. GNU completes healthy, revalidate and role-before, then refuses original
clock-order before role-after's mutation. Its original same-boot tuple has
boottime 79347583067 then 80244577195 ns and realtime 1792500654855763271 then
1792500648826346512 ns. No causal/custody expiry is recorded there; the actor is
not established. Trust-after is not executed on GNU. No retries or spliced pass.

Both lanes share one unchanged WSL boot and unchanged active timesyncd; no owned
fixture remains and the original bundle is unchanged. The combined local gate
is still false. The preceding native expiry is not rerun; a new isolated native
control requires separate authorization. Original delivery, all-writer cessation,
durable completion/later cells, reconnect and real enrollment remain independent
prerequisites. Do not start joined SIGKILL/lost-ACK or server execution by treating
the MSRV pass or allocation reductions as a replacement for these gates.

## Native recheck after the allocation changes

The [separately authorized native recheck](local-runs/held-native-recheck-20261020/README.md)
executes the same exact normal images once per compiler, serially and without
attribution or case override. GNU and MSRV both refuse healthy during
`prepare_owned_root_guardians` before placement/join, at original causal ages
10,003,954,927 and 10,068,587,556 ns. Kernel refusal records are null; first
custody observations already consume 9,176,614,214 and 9,237,420,814 ns. No retry,
remaining case or changed deadline. The prior native/WSL failures remain intact;
different refusal frontiers do not establish a comparative performance claim.

All host preflights and independent postflight pass with unchanged supervisor,
installed artifacts, boot and mount tables, and no owned fixture residue.
Only new temporary staging/export and existing private fixture resources are
used. No installation, original start, clock/time-service operation or product
workload. This consumes the approved isolated control; another native run needs
separate authorization. Next local attribution must cover preparation before
first custody and through owned-root closing validation. Native composition,
joined SIGKILL/lost-ACK and all independent execution/lifecycle/trust gates stay
closed, regardless of accepted allocation reductions or evidence integrity.

## Local preparation attribution after native refusal

The [custody preparation control](local-runs/custody-preparation-cost-20261020/README.md)
adds only test observations before first custody and around the unchanged closing
round. Baseline outputs occupy 2.828–3.058 s locally; closing occupies
67.092–73.603 ms. This separates phases but neither explains the native age
through a host conversion nor establishes the actor behind old time refusals.

The measured buffer-only mechanism saves three allocations/196611 requested
bytes per healthy Sender revalidation, repeated in 12 pairs. Every original
fresh kernel observation, full read capacity, error order and 300/60/10 bound
stays. The same temporary storage is overwritten and only newly returned bytes
are parsed; no observation or pathname is cached. Final timing ranges overlap,
so numerical latency/CPU/API and retained-memory gains remain unclaimed.

Both final GNU/MSRV complete local holder matrices pass once without attribution;
actual fixed fresh counts and targeted kernel/join/trust checks pass. Historical
source-list, adapter and intermediate GNU Auth(Time) failures remain unchanged.
This strengthens local evidence only. Native composition is still negative and
no new server run is made. Joined SIGKILL/lost-ACK, authenticated original delivery,
all-writer cessation, later cells, reconnect and real enrollment stay closed.

## Remaining outputs and owned root attribution

The [next local control](local-runs/remaining-preparation-cost-20261020/README.md)
separates Starting/pipes/sinks and the owned-root context/opening/controls/closing
steps. The expensive owned-root spans are complete original-owner rounds, while
binding and root controls together occupy less than one millisecond locally.
Their observation frequency is unchanged; this is not a request to remove or
cache them. Outputs' Starting span still includes both fsync and live rounds.

A bounded scalar projection removes eight temporary allocations/128 requested
bytes per healthy complete Sender validation in all six pre-adoption pairs.
The complete old parser, Sender body, fresh counts and original refusal sites
remain controls. No cache API, deadline or production authority changes. Native
time refusal, joined SIGKILL/lost-ACK, original delivery, all-writer cessation,
later cells, reconnect and production enrollment remain independent closed
gates; no new server run is made.

## Starting effects attribution and local clock refusals

The [Starting control](local-runs/starting-effects-cost-20261020/README.md)
separates three fsync intervals from the surrounding full validation rounds.
Three complete GNU profiles place 98.10–99.51% of Starting in nineteen full
rounds; combined fsync occupies 4.707–22.226 ms of 1263.504–1349.378 ms total.
No runtime candidate is adopted and every original effect/check/deadline stays.

The intended MSRV profile cohort stops on its first unchanged kernel clock-order
refusal. The independent GNU unprofiled matrix also refuses healthy; MSRV's
independent five-case matrix passes. Same-boot realtime reversals and differing
initial/final WSL boot IDs are retained, not repaired by a retry or longer
allowance. Source/contract/lint passes and sealed evidence cannot open the live
gate. Next local work must establish time/environment stability through separate
read-only diagnosis before another recorded cohort, then attribute avoidable
work inside full fresh rounds. No new server, service, installation, supervisor
or product operation occurs; all independent execution and trust gates remain.

## Validation step attribution under unchanged live guards

The [read-only diagnosis and step control](local-runs/validation-round-cost-20261020/README.md)
keeps time/service settings unchanged. One 180-sample observer has no adjacent
clock reversal but shows Linux adjusted clocks advancing about 89.555 s while
Windows QPC and Linux RAW advance about 93.27 s. Earlier boots and repeated
timesyncd initialization remain in the journal; actors behind old changes are
not proved. Clock correspondence and past stability cannot be declared green.

Exact source reconstruction and all 361 source/contract assertions pass. Three
complete GNU profiles locate 85.62–85.82% of the eight-step output round sum in
ReservedLive. Boot/Namespace observations and authority validation are the next
local attribution targets, not permission to skip a live check. MSRV's first
profile and independent matrix refuse clock-order; GNU's independent five-case
matrix passes. Failed invocations are not retried. No runtime optimization,
deadline change, server run or execution admission follows from this result.

## Isolated allocation control before another runtime candidate

The [fresh leaf control](local-runs/validation-leaf-requests-20261020/README.md)
completes all eight cases on both compilers with three serial 1024-operation
samples. Borrowed boot, each namespace opener and complete Origin revalidation
request zero Rust allocations. Their remaining wall cost cannot be explained
away by another Rust allocation removal. The shared authority fixture reader
still requests 68 allocations/11 reallocations per inspect at one generation
and 184/59 at seven; fixed ancestry and the full ActiveAuthority wrapper are
outside this control. Requested bytes and read counts are retained separately
from latency, CPU and retained memory.

Both lanes pass 47 focused tests and check/strict lint; all 363 old/new
source/contract assertions pass. The next local hypothesis is issuer-key
formatting and temporary ownership inside authority inspection, with separate
attribution before any substitution. All original live checks and 300/60/10
deadlines remain. This does not rerun or repair the preceding live refusals,
open authenticated execution or authorize a new server rehearsal.

## Authority issuer key formatting candidate

The [isolated candidate](local-runs/authority-key-hex-20261020/README.md)
replaces only per-byte issuer-key formatting with existing exact-capacity hex
encoding. All live checks, their frequency, canonical bytes and original
300/60/10 deadlines remain. Before/after controls conserve exactly 31 fewer
allocations, 3 fewer reallocations and 304 fewer requested bytes per complete
fixture authority inspect at both one and seven journal generations. Fresh
read/write/kernel counts remain equal. Both compiler lanes pass authority,
kernel/trust and check/lint suites; all 365 source/contract assertions pass.

This is a locally accepted allocation substitution, not proof that native
preparation fits its admission window. Before and After report different boot
IDs; no native live matrix is retried or time refusal removed. Fixed production
ancestry, full ActiveAuthority latency, real original execution and all lifecycle,
writer retirement, trust and reconnect gates remain separate. The next local
hypotheses are temporary context/name/inventory ownership, one at a time.

## Authority binding context ownership candidate

The [paired local control](local-runs/authority-context-20261020/README.md)
accepts only direct ordered field equality inside `Record::bind`, runtime
`b8e02d6a`. Issuer/policy/epoch prefix and every original context comparison
remain; parser/history context construction is not part of this candidate.
Both compiler lanes conserve exactly two fewer allocations and 68 fewer
requested bytes per complete fixture inspect at one and seven generations,
with unchanged reallocations, reads and fresh kernel observations. Prefix
issuer/epoch refusals retain their short circuits and cost. All 367 old/new
contracts, before/after authority suites, checks and strict lint pass.

These bounded requests do not establish full ActiveAuthority time, stable
clocks or native preparation within its unchanged admission window. Distinct
Before/After WSL boots and old live clock-order refusals remain explicit.
No native rerun, server installation, supervisor operation or product workload
occurs. Next local attribution may isolate remaining parse/context, name or
inventory ownership, one hypothesis at a time. Authenticated original delivery,
all-writer cessation, later cells, joined SIGKILL/lost-ACK, reconnect and real
enrollment remain independent closed gates, not consequences of this saving.

## Authority repeated leaf read ownership candidate

The [leaf name control](local-runs/authority-leaf-name-20261020/README.md)
separates cold history parsing from retained active inspection. Parsing context
ownership is not adopted for this repeated owner. Instead runtime `843f6fe9`
borrows only temporary reopen names into the same checked read; initial owned
names, fresh File lifetime, inventory, error order and all 300/60/10 limits stay.
Final measured source is `a3fe9098`, with only a test-hook placement repair.

Both compilers conserve two fewer allocations/44 requested bytes per complete
one-generation fixture inspect and fourteen/308 at seven, without changing
reallocations, reads or kernel observations. Final 191-test authority suites,
checks/strict lint, all 369 contracts and repository checks pass. Failed source
template/hook checks and an unrelated Windows pre-test build lock are retained;
no old assertion or measured cohort is replaced.

Distinct WSL boots and old live time refusals remain. Expected inventory
construction is the next local attribution question; directory scanning cannot
be cached or skipped. Original delivery, all-writer cessation, later lifecycle
cells, SIGKILL/lost-ACK, reconnect and real enrollment remain separately closed.
No server/live-matrix run, clock/service change, supervisor control or product
workload occurs, and tool identity changes cannot inherit campaign admission.

## Expected inventory local outcome

The [expected inventory control](local-runs/authority-inventory-20261021/README.md)
accepts only streaming the expected names against the freshly observed set.
Runtime `7595dba2` and whitespace-only repair `c86330ae` preserve all scans,
UTF-8/cardinality/budget checks, error order and original 300/60/10 windows.
Both compilers conserve 4 fewer allocations/656 requested bytes per complete
one-generation fixture inspect and 8/2544 at seven. Reallocations, reads and
tracked kernel counts stay equal. The full ActiveAuthority wrapper, CPU,
latency, retained memory and cache API are outside this measurement.

Final 193-test authority suites, focused checks/strict lint, all 371 source
contracts and repository checks pass. Original canary, lint and format refusals
stay retained; all repairs precede the first After capture. Separate boots and
the old clock-order refusals leave native admission closed. No new live matrix,
server, installation, supervisor, clock/service or product operation occurs.
Original delivery, all-writer cessation, later cells, SIGKILL/lost-ACK,
reconnect and real enrollment remain independent gates. Remaining name
formatting costs require a separate attribution candidate, not a deadline or
observation-frequency change.

## Name capacity local outcome

The [name capacity control](local-runs/authority-name-capacity-20261021/README.md)
accepts only initial String reservation inside the existing formatter, runtime
`433a7cb1`. Names, canonical records, error order, all live checks and original
300/60/10 windows remain. Both compilers conserve 8 fewer reallocations and
136 fewer requested bytes per complete one-generation fixture inspect, and
56/952 at seven. Initial allocations, reads and tracked kernel counts stay
equal. Full ActiveAuthority timing, stable clocks, retained memory and cache
API throughput are not measured here.

All final 194-test authority suites, focused checks/strict lint, 373 source
assertions and repository checks pass. The new canary selector refusal is
retained; only the new test is repaired before captures. Separate boots and old
native clock-order refusals remain. No new server/live-matrix run, installation,
supervisor, clock/service or product operation occurs. Original delivery,
all-writer cessation, later cells, SIGKILL/lost-ACK, reconnect and real
enrollment remain independent closed gates. Expected-name String ownership is
a possible next local attribution question, not permission to skip observation
or extend an eligibility window.

## Expected-name ownership local outcome

The [isolated expected-name control](local-runs/authority-expected-name-20261021/README.md)
accepts runtime `be357d10` for temporary expected-name comparison only. The
same formatter now writes to a 23-byte stack Cursor. Observed inventory,
opening/publication names, all fresh/live checks, error order and original
300/60/10 windows remain. Complete fixture inspect conserves 4 fewer
allocations/88 fewer requested bytes at depth one and 28/616 at seven,
exactly twice the pure comparator saving. Reads stay 4/28, reallocations
stay zero, and six other cases and tracked kernel counts remain equal.

Both compilers pass 196 authority tests, affected checks/strict lint, all
375 source assertions and repository checks. The first new adapter's refusal
of an exact old restored template stays retained; only that known input is
added and no old assertion or measured invocation is replaced. Before/After
boots differ. Request conservation neither repairs old clock-order refusals
nor measures full ActiveAuthority, timing, retained memory or cache APIs.

No new live matrix, server, installation, supervisor, clock/service, product
or qualification operation occurs. Original delivery, all-writer cessation,
later cells, SIGKILL/lost-ACK, reconnect and real enrollment remain independent
closed gates. Remaining observed-inventory and record-read requests need a
separate attribution baseline, not a cache of live state or longer deadline.

## Remaining request owners

The [actual-phase control](local-runs/authority-breakdown-20261021/README.md),
source `20e6fc12`, changes test instrumentation only. Runtime bytes restore
exactly to `ad5d8ad0` after removing hooks; every live check and original
300/60/10 window remains. Full fixture inspect stays 25/0/1718 at depth one
and 101/0/8500 at seven, in allocations/reallocations/requested bytes, with
reads 4/28. Both compilers and three samples conserve actual phase costs.

Two inventories account for 16/68 allocations; record/marker revalidation
4/28; directory checks and bind 5/5. Expected comparison remains zero, while
fd-path formatting is a measured one-allocation/28-byte subset per inventory.
Its ownership is a next isolated hypothesis, not a new accepted reduction or
permission to cache observed state. Zero read counters do not remove metadata
checks or establish timing readiness.

Both lanes pass 199 authority tests, affected checks/strict lint, 377 source
assertions and repository gates. No capture fails or is replaced. Workload,
qualification and old live refusals remain unchanged; independent delivery,
all-writer cessation, later cells, SIGKILL/lost-ACK, reconnect and real enrollment
gates stay closed. No new live/server, supervisor, clock/service or product run.

## Inventory fd path ownership outcome

The [local paired candidate](local-runs/authority-fd-path-20261021/README.md),
Before `3635e560` and After `86dbd967`, removes only fd-path String ownership.
Its 25-byte stack buffer preserves the decimal formatter and fresh read_dir.
Exact source guards retain the complete runtime apart from that substitution;
all live predicates, refusal order and original 300/60/10 windows remain.

Both compilers and three samples conserve the isolated saving twice in full
fixture inspect: 25/0/1718 becomes 23/0/1662 at depth one, and 101/0/8500 becomes
99/0/8444 at seven, in allocations/reallocations/requested bytes. Reads stay
4/28; all six other cases and tracked kernel counts remain equal. Both lanes
pass 201 authority tests, affected checks/strict lint, 379 source assertions
and repository gates. No measured invocation fails or is replaced.

This is local request reduction, not CPU/latency or live admission. Separate
boots do not repair old clock-order failures or establish continuity; earlier
host receipts do not transfer to the new source. Observed-name/container and
record-read ownership need separate attribution. Delivery, all-writer cessation,
later cells, SIGKILL/lost-ACK, reconnect and real enrollment remain independent
closed gates. Workload and qualification stay frozen; no new server/live,
supervisor, clock/service, product, qualification or push operation occurs.

## Remaining allocation owner attribution

The [expression-level packet](local-runs/authority-owners-20261021/README.md),
source `cd75c7cd`, changes test hooks only. Full fixture inspect remains
23/0/1662 at depth one and 99/0/8444 at seven, in allocations/reallocations/
requested bytes, with 4/28 reads. Actual/traced phase sums, all six other cases
and tracked kernel counts agree on both compilers and all three samples.

At depth seven, initial read buffers own 14 allocations/4734 requested bytes
of the full path. The two inventories' next/file_name own 56/1260, set insertion
6/1872, and leaf open_at 14/322. These locate request ownership, not CPU/latency
or a safe change to observation frequency. Metadata/read checks remain live;
zero extra heap requests do not mean zero kernel work.

Both lanes pass refreshed 206 authority tests, affected checks/strict lint,
382 source assertions and repository gates. Source and kernel-FD controls
retain original iterator/File lifetime, guard/error order and 300/60/10 windows.
Authoring failures are retained before measurement; no capture fails or is
retried. Workload/qualification remain frozen. CPU relevance and each ownership
hypothesis need separate preregistered controls. Old live/time, delivery,
all-writer cessation, later cells, SIGKILL/lost-ACK, reconnect and real enrollment
gates remain independent; no server or product admission is granted.

### CPU attribution before another ownership candidate

The [local CPU packet](local-runs/authority-cpu-20261021/README.md), source
`bcd8aee4071055b3b79376459f065e12c7ec1b50`, measures six actual fixture phases
with thread CPU time, 32 warmups and fourteen counterbalanced 2048-call batches.
Both compilers complete once without measured failure/retry. Exact source
restoration retains all runtime checks, fresh owners and 300/60/10 windows.

Depth-seven record-plus-marker revalidation has per-round median CPU
59.19/56.96 us on GNU/MSRV, against 25.08/23.50 us for twice inventory. This
prioritizes a narrower leaf open/metadata/read-versus-buffer study; it does not
identify an accepted ownership change. Diagnostic allocator/fixture overhead,
unpinned scheduling, full ranges and signed residuals prevent a production
CPU or latency claim. No runtime candidate is accepted.

Full allocation/read controls and payload/epoch identities remain unchanged.
Both lanes pass 208 authority tests, affected checks/strict lint, 383 historical
source assertions and repository gates. Original lint refusal and test-only
repair remain. Old live/time failures, delivery, all-writer cessation, durable
lifecycle, joined SIGKILL/lost-ACK, reconnect and real enrollment remain closed
independent gates. No new live/server/product or clock/service action occurs.

Evidence `497c2c8e10465897f675822de12d472d6ab9ed6f` seals 84 members plus
inventory SHA256
`5169a17fdc585ac15be3447a802be5e8b289346ee59e2eb420fcca8b31828d4e`.
All seventeen preceding packets and 85 raw Git entries verify. Eight structural
JSON and nine scope mutations are refused; qualification remains frozen.

### Selected leaf CPU observations

The [selected leaf packet](local-runs/authority-leaf-cpu-20261021/README.md)
uses source `b99c1c759630cf632c0a13a905c3ddf9d822def5`. Each of seven
source-checked observer sites is timed independently, with actual/traced/empty
controls and twenty balanced 1024-operation samples per cluster. Runtime, old
traces, fresh checks, owner lifetime and 300/60/10 windows remain unchanged.

Depth-seven buffer-window medians are 238–273 ns per leaf against empty
192–215 ns; ranges overlap. Open, metadata and read have larger medians, but
the windows include timer work and exclude other operations. No exact CPU
partition, buffer bottleneck, runtime optimization or speedup is established.
Retain all outliers. A separate short file-name stack-encoding candidate must
preserve directory/fallback/NUL semantics and fresh kernel checks, and pass
cold/warm and fault controls before acceptance.

Both compilers complete once without preflight or measured failure/retry.
Owner/full controls and payload/epoch identities agree. Both lanes pass 210
authority tests, affected checks/strict lint, 385 historical assertions and
repository gates. Old live/time, delivery, all-writer cessation, lifecycle,
SIGKILL/lost-ACK, reconnect and real enrollment remain independent closed
gates. No server, installation, supervisor, clock/service, product or
qualification operation occurs.

Evidence `9a460ee7e0ac2b2f9f829b9fd8c8be4a8b28f27f` seals 59 members plus
inventory SHA256
`4a4c317f7e9cc878b57aa205324d865577cd17772bff3dea0d7e513c01b2d805`.
All eighteen preceding packets and 60 raw Git entries verify; eight structural
JSON and ten scope mutations are refused. The
[short file-name candidate](authority-open-name-plan-20261021.md) was preregistered
at this stage, with no directory optimization or transferred server receipt.

### Short file name request reduction

The [local candidate](local-runs/authority-open-name-20261021/README.md)
now compares baseline `275cd253875dc32eda5c6d2ba3c3b1c2290e9da2` with runtime
`b074f8f9`. Only file names of at most 23 UTF-8 bytes avoid CString allocation;
per-call NUL checks, directory/long-name fallback, syscall/errno/FD ownership,
fresh validation frames and original 300/60/10 windows remain unchanged.

GNU 1.94/MSRV 1.88 each reproduce full fixture requests 23/99 to 21/85
allocations and 1662/8444 to 1616/8122 bytes, with zero reallocations and
unchanged 4/28 reads. Both actual/traced owners and all six other full controls
agree, with the same payloads and epochs. Separate cold/warm leaf controls
confirm one fewer request per leaf rather than a shift into retained setup.
These fixture counts do not measure full fixed ancestry or ActiveAuthority.

CPU helper pairs include both signs and overlapping raw ranges on both
compilers, as does the unchanged baseline. Request reduction is confirmed;
CPU gain and CPU nonregression are not. Source-level stack writes are not
physical-copy or RSS evidence. Both stages pass 213 authority tests per lane,
ancillary controls, affected checks and strict lint. The three failed authoring
preflights remain; no measured invocation fails, is filtered or is retried.

Next, quantify the residual cost in complete validation rounds before choosing
another runtime hypothesis. Retain the original time failures and separate
delivery, all-writer cessation, durable lifecycle/cells 2–4, joined SIGKILL/
lost-ACK, reconnect, trust/enrollment and server gates. This local helper change
does not reopen them. No new live/server workload, installation, supervisor,
clock/service or qualification action is authorized by this comparison.

All 388 historical/source assertions across 23 targets pass. Raw-log review
confirms the exact primary request effect and preserves nineteen prior packets.
Four reviewer mutations are refused; CPU ambiguity and old admission refusals
remain explicit rather than being converted into a healthy release result.

Evidence `de0ed3fee20b110e0c48dae6876e64eca2411527` seals 109 members plus
inventory SHA256
`7d5a88964d1df7498aced1d5ce81fbd8b4ad16fe693645bb5044503717a67ce8`.
Final repository and read-only environment receipts pass the additional
`final-review.ps1` wrapper before the original byte-pinned raw-log review;
all 110 Git files preserve raw bytes. Qualification inputs remain frozen.

### Residual validation profile preparation

The [residual profile packet](local-runs/validation-round-residual-20261021/README.md)
terminates before measurement because verified preflight, readiness and
postflight boot IDs differ. Source `bd758f788ed6620517872bcde9e3019403ca9d34`
keeps all runtime/observer bytes unchanged from `b074f8f9`; GNU/MSRV each pass
155 focused tests, affected checks and strict lint. Default images are ready,
but no profile or new full matrix starts. No residual-cost range or execution
admission is obtained. Bundle, qualification and configuration hashes agree;
the boot-change actor is not proved, and no time/service/idle-policy changes
or WSL restart command is performed.

Tooling failures and the initial root rustup query's auto-installed copies
remain documented; both just-created toolchains are explicitly removed.
Corrected version queries disable auto-install and use artur. The preflight
reviewer preserves all twenty old packets, eight round/binding and four
preflight mutation refusals, and false completeness/native admission flags.

Next, prepare a separate bounded single-session local driver around fresh
context, serial scheduled profiles and postflight. Preserve the original
six-profile schedule, first-refusal stopping, all live checks and 300/60/10
windows. Do not continue the refused packet, assume external boot/time stability,
or change server/supervisor/product state. Old delivery, all-writer cessation,
lifecycle/cells 2–4, joined SIGKILL/lost-ACK, reconnect and trust/enrollment gates
remain independent and closed.

The residual preflight evidence `133803a86b51df62ddcf163b0585cf797e5c7744`
seals 66 members plus inventory SHA256
`15c86f32b3f56c0c86ac526ad72b54b32aa3cf75625543cea3dd3d9aee9c2807`.
All 67 raw Git files, twenty prior packets and final repository checks verify;
zero profile invocations and false completeness/admission remain explicit.

### Single session preparation reaches the original deadline

The [bounded local runner](local-runs/single-session-rounds-bound-20261021/README.md)
is implemented and exercised at `c1aa639990c4d2434a896ab7e90d93a0f8ce4b8a`.
One WSL parent encloses fresh metadata, fixed serial compiler lanes and
postflight. Boot, PID/start ticks, configuration, original bundle, qualification,
images and empty fixture inventory agree at recorded boundaries. No runtime or
observer change, time/service/idle-policy adjustment or server action occurs.

GNU/MSRV both refuse their first healthy preparation with original ten-second
expiration (`Causal(Time)`, exit 101): ages 10,175,057,064/10,006,497,560 ns.
Their later samples stay absent and neither cohort supplies complete round
cost. The wrapper fixes the inter-entry lifetime assumption in this observation,
not time stability, all-writer cessation or native admission. The two earlier
zero-profile tooling failures remain separate. Final local controls pass 29
Linux tests, 22 Windows tests plus seven skips, two PowerShell controls and four
conservation mutations; previous compiler checks remain inherited.

Next, attribute preparation before actual expiration from the retained partial
phase/round evidence. Any subsequent candidate needs its own frozen method and
packet, unchanged original checks/windows and fresh bounded execution; do not
resume this one. Original delivery, all-writer cessation, lifecycle cells 2–4,
joined SIGKILL/lost-ACK, reconnection and production enrollment remain separate
closed gates. Installation/supervisor maintenance and server workload are not
authorized by this local runner step.

### Offline partial preparation attribution

The [derived attribution packet](local-runs/partial-preparation-attribution-20261021/README.md)
reconstructs the two sealed failed prefixes with zero new timing profiles.
Outputs completes 46 rounds in each prefix; ReservedLive owns approximately
86% of its measured step wall. Its frequent Boot/Namespace work and overlapping
authority observations are the next cost-control targets, not permission to
remove checks, change cryptographic policy or extend the ten-second window.

Eighteen controls pass on both Windows and Linux; partial-round/refusal joins,
graph conservation and all twenty-two prior packets verify. No complete cohort,
native admission or new optimization is obtained. Phase CPU excludes children
and report emission, so phase sums and residuals cannot reconstruct admission
age or exclusive CPU ownership. First isolate observer bookkeeping at the same
numeric topology, then investigate per-call kernel/authority costs while
retaining every live predicate and original window. Delivery, all-writer
cessation, lifecycle, SIGKILL/lost-ACK, reconnect, enrollment and server gates
stay closed. No service, clock, server, installation or product action occurs.

### Recorded topology observer control

The [synthetic observer packet](local-runs/observer-topology-cost-20261021/README.md)
isolates kernel capture and round snapshots at the recorded aggregate graph,
not original temporal/cross-observer execution. Original live source and
300/60/10 windows remain byte-identical. Twelve independent cases complete
without privilege, manager, real guard body or product execution.

Kernel activation has positive CPU deltas in all six cases per compiler;
added round snapshot effects change sign and are unresolved. Aggregation and
serialization are measured separately for the kernel/round report subset.
No result is subtracted from live cost or converted to a healthy admission.
Both compilers pass 19 tests/check/strict lint, seven reader controls and nine
real-report mutations pass, and twenty-three prior packets verify. Next
separate fresh kernel/authority work; live/auth observer costs and all original
delivery, writer cessation, lifecycle, SIGKILL/lost-ACK, reconnect, trust and
server gates remain. No clock, service, server or installation changes occur.

### Fresh kernel and authority cost boundaries

The [offline split](local-runs/fresh-kernel-authority-20261021/README.md)
uses actual recorded fresh Boot/Namespace calls and separates Boot's Document
child from its setup/check/close/decode shell. Exact kernel partitions now
include that read once. Authority/authentication remain independent because
their cross-observer intersections were not recorded. No exclusive CPU or
syscall cost is inferred and no existing expired attempt is retried.

Ten new controls and eighteen original reader tests pass on both Windows
and Linux, together with repository checks and all twenty-four previous
packets. All live predicates, source bytes, original 300/60/10 windows and
qualification inputs remain frozen. Next isolate actual fresh primitive
costs before any candidate; this offline attribution does not close original
delivery, writer cessation, lifecycle, SIGKILL/lost-ACK, reconnect, trust or
server gates. No clock, service, server, supervisor or installation action.

### Boot operation attribution and directory request reduction

The [local operation packet](local-runs/boot-authority-operations-20261021/README.md)
separates Boot open/filesystem/read/decode/close windows without a production
Boot change. It accepts one isolated short directory-name encoding change in
the shared authority helper: the existing bounded stack buffer now covers
short directories while all fresh checks, flags, errors, owner lifetimes and
original 300/60/10 windows remain. Before/after controls on both compilers
confirm two fewer allocations/eight requested bytes per synthetic inspect,
with unchanged read/kernel/site counts, payloads and policy epochs.

Candidate authority selection passes 214 tests per compiler, affected checks
and strict lint. Source canaries, thirteen reader/runner controls per OS and
repository/evidence checks pass; old refusals and authoring failures remain.
Different stage boots and mixed CPU pairs preclude a CPU gain or nonregression
claim. This is not fixed-root enrollment, a healthy original attempt, native
performance or server admission. No clock/service/server/supervisor/install/
product action occurs. Delivery, all-writer cessation, lifecycle, fault,
reconnect, enrollment and exact installed-source gates stay separate; next
validate another isolated owner or unchanged full-frame preparation locally.

### Full healthy local preparation completes in unchanged windows

The [directory candidate preparation cohort](local-runs/directory-preparation-impact-20261021/README.md)
completes all six scheduled healthy samples, three per compiler, under the
unchanged original 300/60/10-second admission windows. All ten phases and their
full validation call graphs remain; live age/causal observations, source,
original bundle, default-feature images, boot/configuration and postflight
checks agree. Largest reported live ages remain below 5.612 seconds. Raw child
times include setup/reporting/cleanup and are not the admission age.

The earlier time refusals remain immutable. A distinct WSL lifetime prevents
attributing this cohort's shorter preparation to the isolated allocation
change. Healthy preparation is now demonstrated for this local lifetime only;
the next separate local gate is the remaining original fault scenarios with
the same windows and no retries. Original delivery, all-writer cessation,
lifecycle/SIGKILL/lost-ACK, reconnect, production trust and installed-source
server gates are not closed by a preparation-only success. No clocks,
services, installation, supervisor, product or server actions occurred.

### Remaining original holder refusal cases complete locally

The [local holder fault packet](local-runs/held-original-faults-20261021/README.md)
passes revalidate, role-before, role-after and trust-after on both GNU and MSRV,
once each, with original live predicates and 300/60/10-second windows. Exact
Roles/Trust errors, post-join sticky refusal, live timer, retained fence and
synthetic empty-cgroup/terminal-custodian/EOF cleanup are asserted by the
unchanged original harness. Metadata, default images, original bundle and
postflight remain stable; raw failures would not be retried or relabeled.

These four remaining holder fixture cases are now covered. They are not
original start/delivery, all-production-writer cessation, durable lifecycle,
full SIGKILL/lost-ACK, reconnect or production enrollment proof. Next audit
those original-path prerequisites and remaining local matrix boundaries
before a separate experiment. No clock/service/idle-policy, server,
supervisor, installation, product or qualification action is introduced.

Full-phase count equality is separately negative: two accept-consent pairs
differ by one standalone Sample outside the otherwise equal validation rounds.
The delta matches the unchanged bounded nonblocking relay polling signature;
this is inference, not direct receive-attempt telemetry. The refused strict
comparison and original raw counts remain. No deadline, predicate or acceptance
threshold changes to accommodate this cost-comparability observation.

### Original delivery and writer cessation prerequisite audit

The [current source audit](original-delivery-writers-prerequisites-audit-20261021.md)
keeps the full joined SIGKILL/lost-ACK gate closed. Full frames, original manager
identity/message preparation, root Armed/revocation mechanics and holder
retirement exist. The consumed holder has no delivery/release API; the original
message still uses the legacy worker projection, and reply state remains
preparation-only. A standalone held launcher exists but is not that integration.

The missing proof spans creator/message/queued/manager writer copies outside
the root cgroup, an active collector rather than barred IO, terminal sink/journal
writers, durable completion and the original cells 2–4/recovery bridge. Those
model transitions already exist; they must not be mistaken for original-path
admission. The next bounded implementation is exact held-unit projection from
the consumed owner, followed by one-shot authenticated delivery and terminal
writer protocol before new joined fault cuts.

The audit passes 32 existing local unit/property controls per compiler and
reviews the preceding sealed chain, without new privileged fixtures or manager
operations. No runtime, live check, original 300/60/10 window, qualification,
clock/service/idle policy, server, supervisor, installation or product change.

### Consumed held unit semantics implemented without transport

The [held projection implementation](held-unit-projection-20261021.md)
consumes the existing joined owner and retains its placement, trust, roles,
manager/message identity and Armed root guardians. The closed result exposes
only revalidation, with unchanged full holder validations before and after
projection and exact closing/retained equality. Failure retains and retires
the same original owner. No live predicate or original deadline changes.

The unit and eleven FD slots are an exact semantic projection, not a runnable
systemd specification. Next bind newly materialized dedicated stdin, release
endpoint, sealed original config copy, creator pidfd and sealed bootstrap to
this owner and an expanded exact descriptor inventory. Then run a separate
original live-window gate before adding one-shot transport and durable
uncertainty. Actual process/barrier binding, active collection and complete
writer cessation must still precede completion/cells 2–4 and the full joined
SIGKILL/lost-ACK matrix. Production enrollment and server rehearsal remain
independent gates; this step does not open them.

### Kernel materialization implemented with extended inventory

The [materialized holder](held-unit-material-20261021.md) now consumes the
projected owner, copies and seals the original config, creates EOF stdin,
release socketpair, sealed bootstrap and creator pidfd, and binds twenty
specified FD roles. Fresh inspection includes original context and exact
object/flag/position equality, bounded content and peer/kernel checks, inside
the unchanged complete validation bracket. No deadline is reconstructed or
extended; failure/drop retire the same owner without a send or release method.

The next local gate is whole original-owner projection/materialization in the
original windows, retaining any first refusal. Current real-kernel unit tests
use synthetic role expectations and the bracket controls are models; they do
not close that composition gate. FD inheritance/delivery, durable uncertainty,
actual process/barrier binding, active collection, complete writer cessation,
completion/cells 2–4, SIGKILL/lost-ACK/reconnect and production enrollment still
precede server admission. This step changes no clock, service or server state.

### Original owner construction returns but full revalidation expires

The [first complete materialization gate](held-material-live-20261021.md) retains
one original call per compiler, without retry. Production projection and
materialization return on both lanes; subsequent full revalidation refuses
`Causal(Time)` beyond the unchanged ten-second original origin. Full composition
readiness remains false. Empty postflight residue is not a passing EOF/fence or
all-writer proof because refusal bypasses those final fixture assertions.

Keep this packet closed. Next attribute the retained combined validation/effect
cost offline, then review a separate candidate that preserves every fresh check
and original deadline. No clock, service, server, installation, supervisor,
product or original-send action is authorized by this failed gate. Durable
uncertainty, delivery/process binding, collection, all-writer cessation,
completion/cells 2–4 and joined SIGKILL/lost-ACK/reconnect remain downstream.

### Repeated validation round cost attributed without new execution

The [offline round attribution](held-material-round-attribution-20261021.md)
identifies ReservedLive as the dominant measured step in the new refused phase,
with Boot/Namespace/Origin dominant in the phase's in-step kernel tree. All 22 observed
phase partitions conserve; the original two Causal refusals and failed readiness
remain. No runtime, observer, deadline, check frequency, WSL, clock/service or
server operation changes.

The next candidate should address measured fresh worker/authority inspection
implementation cost under the same call graph. Coalescing full validations or
caching successes is not authorized by this attribution. Unassigned effects,
observer overhead and stage timings remain unresolved. Original construction
success still does not admit full revalidation, completion, all writers or the
server; downstream fault/reconnect/enrollment gates remain independent.

### First fresh check allocation candidate implemented without live replay

The [isolated pidfd pathname candidate](pidfd-info-path-20261021.md) removes heap
formatting of one fresh fdinfo pathname, not the observation itself. Its exact
old origin reconstruction preserves all predicates, their order and the original
windows. GNU/MSRV pure allocator controls and own-process units confirm pathname
equivalence, zero formatter requests, unchanged trace counts and refusal behavior.

No new original-owner profile or timing claim follows. The two first Causal
refusals and failed material gate stay closed, as do EOF/fence, delivery,
all-writer cessation, SIGKILL/lost-ACK/reconnect and server admission. Keep further
cost candidates separate; a future timing comparison needs separate admission
and must never replace the original refused cohort. No server/service action.

### Fresh Groups allocation candidate leaves admission unchanged

The [bounded collection leaf](projection-groups-20261021.md) preserves the final
Vec representation, all fresh reads, limits, field/error order, validation graph
and original windows. GNU/MSRV pure controls confirm exact-size requests without
vector growth; full-file guards and deterministic parser controls remain green.
It is an allocation-only candidate, not a successful material run or timing proof.

The first two Causal refusals and all prior seals remain unchanged. Full material
revalidation, cleanup/EOF/fence, authenticated delivery, all writers and downstream
SIGKILL/lost-ACK/reconnect/server gates stay independent. No live replay,
clock/service change or server run is performed.

### Remaining ReservedLive leaves audited offline

The [offline leaf inventory](reserved-leaf-audit-20261021.md) retains a
ReservedLive-only view of the same refused traces and source witnesses at the
current tree. Old span wall is not promoted into post-candidate performance.
The next bounded work package is a pure baseline for local context document
scratch, preserving callback slice lengths, both metadata observations, single
read/error schedule and original windows before any implementation.

No candidate is implemented in this audit. Trust C-string and peer-growth
experiments stay separate; larger buffers, inventory and FD/representation
changes need additional review. First refusals and cleanup/delivery/all-writer,
SIGKILL/lost-ACK/reconnect and server blockers remain unchanged. No host action.

### Short document baseline retained before replacement

The [pure scratch baseline](context-document-buffer-baseline-20261021.md)
confirms the existing Rust request sizes and freezes metadata/read/error order
with an exact-body synthetic model. The production worker-context file and
all live checks/original windows remain unchanged; only a private opt-in test
module hook is added. First compiler errors are retained, not hidden.

Next consider one short stack-storage candidate with exact slice length,
zeroing and generic compatibility. No implementation or timing improvement is
claimed here. Material, cleanup, delivery/all-writer, fault/reconnect and server
gates remain closed; no original-owner or server run occurs.

### Short document stack candidate kept after pure controls

The [storage-only successor](context-document-stack-20261021.md) implements
private invocation-local scratch through length 37 and unchanged generic Vec
fallback. Exact whole-source reversal and complete-body models retain both
fresh metadata observations, read/error order, call counts and 300/60/10-second
windows. GNU/MSRV allocation controls confirm the short-request removal, not
full preparation timing. The first GNU test-lint refusal is retained separately
from its two test-only fixes and the green focused cohort.

No original-owner attempt or server action is repeated. First Causal refusals,
material/cleanup/EOF/fence, authenticated delivery/all writers, full fault/reconnect
and server gates remain closed. Next audit trust component C strings as a
separate pure baseline/candidate; no timing or readiness promotion is implied.

### Trust component baseline retained without runtime mutation

The [shared CString baseline](trust-component-baseline-20261021.md) retains six
fixed names plus generic and embedded-NUL shapes. GNU/MSRV counters and the
complete-child-body model establish allocation and first-error contracts before
any replacement. Only an opt-in pure test hook changes in old supervisor code;
all trust bodies, observation counts/order and original windows remain intact.

First compile refusals are retained beside the one test-only payload-inspection
repair. Next evaluate one short owned C-string candidate with generic fallback
and exact open/error semantics. Do not equate pure allocation success with a
successful material/cleanup/delivery/all-writer or fault/reconnect/server gate.
No live replay, clock/service change or server work occurs.

### Short owned trust name candidate retained after pure comparison

The [storage successor](trust-component-stack-20261021.md) replaces only the
shared child-name constructor with owned local bytes for short names, retaining
generic CString fallback. Exact whole-file restoration, complete child models
and pointer/error/ownership controls preserve open semantics, fresh-check counts
and order, original windows and both first Causal failures. GNU/MSRV counters
confirm short-name request removal, not full preparation timing.

First missing-import test compiles remain beside the cfg(test) import repair.
No material, cleanup/EOF/fence, authenticated delivery/all-writer, full fault/
reconnect or server gate is promoted. Next isolate collector peer growth with
its own baseline and canonical/error contract. No live or server work occurs.

### Collector peer growth baseline and wrapper contract retained

The [separate pure baseline](collector-peer-baseline-20261021.md) confirms
to_vec allocation followed by push growth for nonempty supplementary lists,
while sort/dedup makes no allocator requests in this corpus. Complete peer and
wrapper source comparisons, BTreeSet union controls and synthetic first-error/
latch/reentry cases preserve ownership, no truncation and validation order.

Only a private opt-in test hook changes in old supervisor code. First test
compile/canary refusals remain alongside the explicit-type/owned-label repairs.
Next consider reserving room for primary in one separate growth candidate;
the current baseline implements none. Original windows, first Causal refusals
and all material/cleanup/delivery/all-writer/fault/reconnect/server gates remain.
No live replay, clock/service change or server operation occurs.

### Peer primary capacity candidate retained after matched pure controls

The [single growth candidate](collector-peer-capacity-20261022.md) reserves
input length+1 before copy and keeps push/sort/dedup, ownership and the complete
live collector wrapper. GNU/MSRV counters confirm no reallocation on the matched
corpus; complete-source reversal and both synthetic first-error/latch arms
preserve validation counts/order and original windows. Full-path timing remains
unmeasured.

The first host expected-text refusal remains beside the exact canary repair;
Linux pure controls are not replayed. All ten prior seals and first Causal
failures remain. Next reconcile retained leaf candidates and unresolved offline
owners before any separately admitted timing comparison. Material/cleanup/
delivery/all-writer/fault/reconnect/server gates stay closed; no host action.

### Retained allocation leaves reconciled with the old refused trace

The [separate offline reconciliation](reserved-reconciliation-20261022.md)
checks eleven raw seals and all 491 members while leaving the older reviewers
frozen. Current source witnesses retain five allocation-only candidates and
eight unresolved owners in that leaf inventory. Different corpora/source pairs
are not summed or extrapolated into current preparation headroom.

The original wall partition, first Causal(Time) ages, live checks and
300/60/10-second windows remain. Next isolate the invocation-local proc document
buffer's construction/reuse and first-error contract in a pure baseline, before
any storage candidate or separately admitted full-path comparison. No runtime,
observer, qualification, clock/service or server change occurs. Material,
cleanup/EOF/fence, authenticated delivery/all writers, full SIGKILL/lost-ACK,
reconnect and server gates remain closed.

### Remaining proc buffer baseline preserves the existing reuse optimization

The [pure constructor/reuse baseline](proc-document-baseline-20261022.md)
confirms one remaining zeroed request per modeled buffer and allocation-free
reuse on its synthetic corpus. It does not repeat the older complete Sender
comparison or add a runtime candidate. Two first Windows compiler lanes, exact
body/source guards and thirteen preceding raw seals preserve the distinction
between an allocation request and live preparation cost.

Keep the current buffer unchanged. A smaller slice, missing zeroing, cached
observation or large nested stack buffer is not justified by these counters.
Next isolate sample boot ownership with the full original surrounding frame
preserved. All live checks, 300/60/10-second windows, first Causal refusals and
material/cleanup/delivery/all-writer/fault/reconnect/server gates remain. No WSL,
service/clock change, installation or server action occurs.

### Sample boot ownership baseline retains the complete original frame

The [separate pure baseline](sample-boot-baseline-20261022.md) confirms the
remaining owned String request after both revalidations and both clock calls.
All fifteen refused synthetic masks return before the clone; seeded ownership,
getter/point-clone and exact decoder controls preserve the model contract.
Two first Windows compiler selections and fourteen prior raw seals do not
substitute for a real original-owner or fault/reconnect gate.

An owned inline/fallback representation is a separate candidate decision,
requiring matched allocation and getter/copy/value-size review before adoption.
Current runtime/storage, live checks, original 300/60/10 windows, first Causal
refusals and qualification stay unchanged. Material/cleanup/delivery/all-writer,
full SIGKILL/lost-ACK, reconnect and server gates remain closed. No WSL, service/
clock change, installation, supervisor operation or server action occurs.

### Inline boot hypothesis rejected without runtime integration

The [owned-inline comparison](sample-boot-inline-20261022.md) retains short-value
allocation savings but rejects adoption under its predeclared getter rule:
higher paired medians on both Windows compilers and a 40-to-56-byte point.
No runtime representation, validation schedule or admission deadline changes.
First raw intervals, the newline-only source-canary refusal/repair and fifteen
older seals/856 members remain available for offline reconstruction.

Next audit remaining leaf costs offline before selecting another hypothesis.
Pure correctness and request counters do not reopen original-owner material,
cleanup/EOF/fence, authenticated delivery/all-writer retirement, full SIGKILL/
lost-ACK, reconnect or server gates. Original 300/60/10 windows, first refusals
and qualification are unchanged. No WSL, clocks/services, installation, supervisor
or server operation is included.

### Fresh authority read storage requires its own reference baseline

The [offline authority ownership audit](authority-read-audit-20261022.md)
retains original Leaf.raw bytes/File/stamp and distinguishes temporary bytes
from an independently opened fresh read. Any storage proposal must retain the
new descriptor through both comparisons, bounded EOF/growth detection, all
metadata predicates and shared-registry sticky refusal. Sixteen prior seals/
928 members are verified; no new request counts or timing are claimed.

Next prepare the separate bounded-read baseline before any transient-storage
candidate. Do not substitute cached/overwritten expected bytes or read_exact of
the old length. Runtime, original 300/60/10 windows, first refusals and
qualification remain unchanged. Material/cleanup, delivery/all-writer, full
SIGKILL/lost-ACK, reconnect and server admission remain closed; no WSL, clocks/
services, installation, supervisor or server/product run occurs.

### Pure bounded read baseline precedes transient storage changes

The [fresh-read reference baseline](authority-read-baseline-20261022.md)
retains complete copied read/revalidate bodies, owned expected bytes, first-error
prefixes and bounded EOF behavior. Two first Windows lanes agree on constructor
requests with no additional generic-read requests in the fixed healthy corpus.
Synthetic token ownership is not real descriptor or kernel fault evidence.

Forty-nine source canaries, fifty-five offline controls and seventeen preceding
seals/955 members pass the focused step gate. Next preregister a separate
transient-storage prototype with its own read/EOF/ownership and copy/size/cost
controls; do not integrate from this allocation baseline alone. Runtime, original
300/60/10 windows, first refusals and qualification remain unchanged. Material,
all-writer, full fault/lost-ACK, reconnect and server gates remain closed; no WSL,
clocks/services, installation, supervisor or server/product run occurs.

### Owner-bound scratch is not original-owner admission

The [temporary storage prototype](authority-scratch-reuse-20261022.md) confirms
allocation amortization only in synthetic full-revalidate controls. It preserves
every original fresh observation and evicts storage on refusal, but adds retained
capacity and an owner wrapper. Paired wall intervals are preliminary; no runtime
integration, Linux/full-path, copy/RSS or all-writer proof follows. First tooling
refusals/repairs and eighteen prior seals/1000 members remain.

Next audit real Leaf/registry owner lifetimes and aggregate retained-memory
budgets offline before considering any integration candidate. Keep original
300/60/10 windows, first refusals and qualification unchanged. Material/cleanup,
authenticated delivery/all-writer termination, full SIGKILL/lost-ACK, reconnect
and server admission remain closed. No WSL, clocks/services, installation,
supervisor or server/product execution is authorized by this prototype.

### Scratch integration requires journal lifetime and eviction coverage

The [offline owner audit](authority-scratch-owner-audit-20261022.md) identifies
borrowed self-reference/entries relocation and journal-wide refusal cleanup as
integration blockers. Successful per-Leaf cache capacity would add up to 270464
bytes per maximum journal, not measured RSS or a process-wide bound. Original
history and descriptors remain independently owned even after sticky refusal.

Next preregister an isolated owned-storage journal model covering append/reopen,
explicit mutable receivers, all original validation brackets and complete cache
eviction on journal errors/external latches. Preserve durable operator transition
versus failed-owner semantics. Nineteen prior seals/1076 members and all first
observations remain; no measurement image or live attempt is rerun. Runtime,
300/60/10 windows, qualification and material/all-writer/full-fault/reconnect/server
gates are unchanged. No WSL, clocks/services or server/product actions occur.
