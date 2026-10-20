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
