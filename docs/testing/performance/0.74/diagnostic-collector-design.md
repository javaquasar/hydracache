# Diagnostic collector and original writer cessation

The operator approved the root-owned collector architecture on 2026-10-14.
Baseline is `802d1a20ea66d94df219a8c787803e82fe58e950`. This amendment is
subordinate to P2/P3 in the [intermediate plan](diagnostic-server-readiness-intermediate-plan-20261012.md).
It does not admit a host operation or a product measurement.

## Output ownership change

The old diagnostic unit appends regular files in a worker-owned cell directory.
An empty original cgroup cannot exclude an outside holder of a writable file
descriptor. Changing file permissions does not revoke such an already-open
descriptor on the selected local filesystem. The new architecture gives the
worker only stdout/stderr pipe write ends. A root-owned collector is the sole
holder of evidence-file write descriptors in a root-private directory.

Old direct-output unit projections, signatures, readers and evidence remain
unchanged. A future enrolled collector start needs a distinct signed output
contract and unit projection: it must not reinterpret a v1 regular-file stream
binding as a pipe-to-collector binding. Local fixture outputs use new exclusive
names `collector-stdout.bin` and `collector-stderr.bin`; no production reader
accepts them as a completed cell. No migration or overwrite is allowed.

The separate [collector intent](diagnostic-collector-intent-design.md) now signs
those output requirements under distinct body/envelope/unit schemas and domain.
It is byte-only verification, not a start or a proof of actual pipe/sink ownership.
The root constructor and authenticated producer below remain necessary.

## Collector implementation sequence

The [original pipe preparation](local-runs/original-collector-pipes-20261016/README.md)
now consumes acknowledged Starting without caller descriptors and retains two
distinct actual kernel pipes, nonblocking readers, blocking writers and CLOEXEC.
Implementation `a533e6df` passes 19 joined cases/compiler and preserves the
original authority/time/storage checks. Private pipe fields drop before the
Starting/fence owner, with actual EOF checked after healthy drop. This is not the
root collector constructor or worker handoff. That constructor must consume these
same objects, not replace them with the fixture factory's newly created pipes;
it still needs sole-root sinks, production ancestry and enrolled original start.

The first [original sink composition](local-runs/original-collector-sinks-20261017/README.md)
is withdrawn. Its first server lane stops before RootCreated; a separate fresh
original-chain attribution obtains Causal(Time) with no root created. Kernel
creation success does not establish complete preparation inside the anchored
challenge window. The full candidate patch is retained and runtime restored.
A separately preregistered constant-depth join must prove unchanged live guards,
effect ordering and deadlines before repeating a complete new candidate screen.

The approved [J1 join plan](diagnostic-output-join-plan-20261017.md) now implements
one consuming framed-consent-to-sinks producer with no asserted inputs. Its
effect-local working/closing rounds reuse every original frame predicate, while
the old standalone publishers remain exact. Both compilers pass three healthy
25-stage construction controls with private EOF after drop. The corrected
joined cohort also passes 27 sink and 41 other fault/live/expiry cases on each
compiler; its first red oracle and independent attribution remain retained.
The separate full process-loss gate passes all 25 effects plus AfterReturn in
three repetitions/compiler: 156 SIGKILL, six controls and 324 fresh inspections.
This is process loss, not power loss or exported-writer cessation. The
[new packet](local-runs/original-collector-sinks-join-20261017/README.md) does not
admit original start, sole-writer drain or an installed production route.

The next [original material slice](diagnostic-original-start-material-plan-20261017.md)
consumes those same outputs and inspects the fixed installed bundle under the
retained builder policy. It compares the whole build statement/identity, full
Reserved state digest and collector unit digest, retaining bundle descriptors
with the original Starting/pipes/sinks/fence. Material preparation is not a
unit transport: it exports neither a legacy file-output spec nor descriptors,
and it cannot start or complete a process. Its separate
[framed material gate](local-runs/original-collector-start-material-20261017/README.md)
passes three healthy chains and thirteen exact refusals per compiler within
the original windows. The first ordinary-composition time refusal remains
retained. This closes temporary fixture material preparation, not installed
production ancestry, executable/process start or writer cessation.

First implement the actual bounded pipe-to-file writer and test its kernel EOF
boundary locally. Retain both read ends and exclusively created evidence files;
export only pipe write ends through a fixture-only Stdio transfer. Readers are
nonblocking, writers blocking, and all originals close-on-exec. One pump step
reads at most 8192 bytes from each stream, preventing a hot stream from starving
the other. Keep the existing 8 MiB per-stream and 16 MiB combined ceilings.
Overflow fails explicitly and retains only the bounded prefix; it never produces
a successful drain receipt. I/O, metadata substitution and unexpected poll
events latch refusal. No retry, truncation, unlink or automatic repair follows.

Both streams must return an actual zero-length read after all queued bytes drain.
A hangup event, dead leader, empty read queue or caller boolean is insufficient.
Only then may the original collector sync the files, make them read-only, verify
their retained bytes against the incremental stream hashes, sync
the directory and close its writable descriptors before returning byte counts
and hashes. That result proves local stream drain only, not authenticated start,
successful workload output, cgroup cessation or lifecycle completion.

The first local constructor accepts only an owned private temporary fixture root,
refuses privileged execution and production trees, and creates a new exclusive
attempt directory. It cannot enroll or convert into production authority. The
same writer algorithm will need a separate private production constructor fed
by the acknowledged attempt/fence and trusted root ancestry, not a path or FD
provided over IPC.

Next connect per-packet kernel sender credentials and boot/monotonic time to
independently enrolled issuer/digest/epoch pins. Implement the narrow original
start producer with exact executable, environment, pipe objects, collector,
process generation, manager invocation and retained cgroup. Its watchdog must
remain effective after controller/supervisor loss. Stop/revoke only the original
owned cgroup; drain EOF also detects pipe write ends leaked outside that tree.
Missing EOF or uncertain stop/sync/close leaves an incomplete retained attempt.
All creator/manager copies of write ends must close, including those retained
across systemd RemainAfterExit. Never infer cessation merely from unit-name lookup.

Authenticated completion then joins original start, successful output, original
exit outcome, exact revoked and empty cgroup, and the collector's consumed drain
result. Cells two through four and their new SIGKILL/lost-ack boundaries follow
that producer, not signatures over supplied completion assertions. Production
issuer enrollment still needs operator-selected public keys and digest/epoch
pins; fixture keys are never a fallback.

## Preregistered local tests

Capture the existing process-exit and output-preparation baseline, then a missing
collector red test. Cover binary stdout/stderr, fragmentation, empty streams,
two-stream fairness, exact and overflowing byte limits, a leaked duplicate writer
after original child exit, actual owned child SIGKILL, and late bytes before EOF.
Check exclusive concurrent creation, symlink/hard-link/path substitution,
owner/mode/size/content drift, invalid descriptor events, sticky refusal and
retained partial outputs. Inject failures at file creation, writes, sync,
read-only conversion, explicit close and acknowledgement; no fault yields a drain
receipt. Test compile-time prohibition of forging or promoting the receipt.

Use seed `0x0750` for deterministic fragmentation, with three exact-source
repeats. Process tests own only bounded nonproduct children, with a 30-second
harness deadline and RAII cleanup. These tests do not model power loss, actual
root isolation or production cgroup revocation. A separate ignored owned-child
proof interrupts directory creation/sync, both stream creation/sync, append,
read-only conversion/sync, final directory sync, close, acknowledgement and
after-return lost acknowledgement. Use 14 checkpoint deaths plus after-return,
one uninterrupted control and two fresh refusal inspections for every retained
attempt. At each barrier the parent snapshots retained bytes and metadata before
SIGKILL, then requires unchanged snapshots and original signal-nine termination.
Fresh inspectors cannot recreate the collector or repair its files. Run this
180-second manual gate three times, single-flight without relinking libtest.
Run package tests, current/MSRV
checks, warnings-denied Clippy/Rustdoc, scoped formatting, documentation and
non-promotable performance-contract checks. Retain negative results and raw logs.
Collector overhead must later pass the existing host budget before measurement
admission; no numerical benefit or negligible overhead is assumed.

## Original pipe message preparation

The [separate message preparation](diagnostic-original-pipe-transport-plan-20261017.md)
consumes original material and retains an unsent FD-bearing systemd message.
It checks every fixed typed worker property against the signed collector
projection and uses only the same original stdout/stderr write ends. The two
message-owned duplicate writers close before the original material and fence.
No evidence file descriptor enters that message, and no public extraction,
connection, send or execution method exists.

Fresh full original rounds bracket bundle and message inspection. The immutable
RuntimeMaxUSec requirement is not a refreshed execution lease: the future
authenticated start must derive remaining absolute deadlines and arm an
independent watchdog before sending. Effective executable/environment, manager
invocation, process/cgroup ownership and closure of every manager/creator writer
remain actual-start obligations. Marshalled FD identities and temporary EOF
tests alone cannot fulfill them or publish completion.

## Absolute alarm origins and refused composition

The [alarm experiment](diagnostic-original-safety-alarm-plan-20261017.md)
anchors its controller-loss bound to the original authenticated progress packet's
kernel receive time, not the later signed Starting controller timestamp. Total
and cell limits likewise retain their original reservation and Starting origins.
An absolute one-shot BOOTTIME timer is a candidate deadline primitive, not an
autonomous guardian or writer-revocation mechanism. Original consent challenge
expiry remains a separate live predicate and can refuse before that timer expires.

The separately guarded material, message and alarm sequence failed its first
healthy server screen at original Causal(Time). Its production entry is withdrawn;
only cfg(test) kernel/reference and cost probes remain. The retained
[negative packet](local-runs/original-safety-alarm-20261017/README.md) distinguishes
this failed composition from the previously accepted message prerequisite.
Do not extend deadlines or count a negative-characterization test as admission.

A possible follow-up groups only nonexecuting preparation under one fresh full
opening and closing validation frame. It must retain every predicate and original
deadline, but changes observations between internal substeps. That experiment
requires explicit agreement and an independent healthy/fault gate before adoption.
The current schedule stays unchanged. Independent guardian lifetime, original
authenticated send/start, all-writer cessation, completion and later cells remain
subsequent execution obligations.

The [unchanged-frame leaf diagnostic](local-runs/validation-leaf-cost-20261017/README.md)
passes six fresh controls and identifies ReservedLive as the largest separately
measured opening leaf. This is a profiling target, not authorization to omit
nested live observations. The existing callback leaves the full closing round
in its final bucket; that bucket must not be described as isolated FinalClocks.
No production source, observation frequency, deadline or admission changes.

The [component controls](local-runs/controller-component-cost-20261017/README.md)
further narrow that hypothesis through existing validators on original objects,
without inserting hooks into or replacing any production body. Their separate
data-only and nested live spans are not an exclusive ReservedLive profile and
grant no cached authority. The current evidence favors profiling repeated
worker/kernel/authority observation before proposing an immutable-crypto cache.

The [syscall summary](local-runs/controller-syscall-cost-20261017/README.md)
then observes about 606 thousand calls per complete component-control family
on both existing compiler binaries. Metadata/open/filesystem/close calls have
the largest counts. The family includes setup, all controls and teardown;
tracing perturbs its duration substantially and does not isolate a validator
or justify caching live inputs. The original schedule and execution boundaries
stay unchanged pending the separately agreed grouped-preparation experiment.

## Test only grouped preparation

The [bounded experiment](local-runs/grouped-preparation-20261017/README.md)
now constructs material, unsent message and absolute alarm between one existing
material_round and one unchanged original_round. It is reachable only through
the alarm's cfg(test) tests, never a production endpoint. Closing repeats full
live checks and binds the provisional timer back to original bounds before
material becomes ready; failure retains the original refused owner/fence.

This candidate removes four intermediate complete rounds. It detects persistent
closing drift but does not certify a transient change restored between endpoints.
The initial screen, repeated original/reference controls and persistent refusal
matrix pass with unchanged original deadlines. Production bodies and observation
schedule remain exact. Process-loss/ack-loss, production adaptation, independent
guardian lifetime, authenticated send/start and all-writer cessation remain open.

## Grouped process loss and withheld return acknowledgement

The [private process harness](local-runs/grouped-process-loss-20261017/README.md)
adds selected child stops at opened, loaded, bound, message, armed and successful
return before a private ACK. The parent pins its unreaped child with pidfd before
waiting and uses that descriptor for SIGKILL. Snapshots and host-lock exclusion
bracket termination; two fresh inspector processes must refuse retained history
after every outcome. Ordinary controls emit one ACK; separate zero-exit controls
complete reference validation and owner drop while withholding it.

The ACK is a harness-only return notice, not a systemd result. No manager call,
descriptor delivery, worker start or new durable record exists here. Confirmed
process death releases the synthetic owner's kernel resources, but does not prove
P2/P3 writer-tree revocation before resource release. Missing ACK never authorizes
adoption or retry. Production composition and its execution gates remain closed.

The declared six-cut matrix passes three repeats per compiler: 36 SIGKILLs,
six positive ACK controls and six zero-exit lost-ACK controls across the main
cohorts. The separate two-case screen and matrices total 100 fresh inspector
processes with unchanged retained evidence. This closes the selected grouped
nonexecuting process-loss gate, not arbitrary crash, actual manager ACK recovery
or any production execution obligation above.

## Fixed origin grouped preparation

The [fixed origin adaptation](local-runs/fixed-grouped-preparation-20261017/README.md)
adds a separate nonexecuting production owner below the original pipe transport.
prepare_original_grouped consumes acknowledged original sinks without caller
path, policy, key, time, descriptor or callback inputs. Only inspect_fixed_install
with the original checked builder policy and identity can supply its bundle.
Fixture loading and checkpoints are cfg(test) only and cannot be a production
fallback. Existing separate preparers and the rejected alarm remain unchanged.

The shared construction body preserves the experimentally checked grouped
opening/closing schedule and its endpoint observation tradeoff. It marks private
material ready only after the complete closing original_round, equal original
bounds and final timer inspection. Exact reference guards cover the separate
timer primitive and closing-round copy. Refusal retains the original owner,
message writer duplicates and fence until drop; no extraction, send or start
capability exists. The entry is not connected to a service dispatch route.

The new owner's shared fixture schedule passes one screen and six repeated
healthy chains on both compilers. This is not a successful root-owned fixed
installation or reuse of the previous SIGKILL certificate for a new type. The
separate process-loss packet below covers this new owner. Independent watchdog,
authenticated original send/start and P2/P3 all-writer cessation remain distinct
prerequisites before completion, later cells, installation or measurement admission.

Both compiler cohorts pass 28 persistent-drift/expiry cases in total. A separate
two-line origin-test correction retains the first negative and original binaries;
the corrected controls prove four expected refusals across both compilers:
missing fixed install without fallback and fixture-to-production revalidation.
All 198 static contracts pass. No fixture success enrolls root-owned installation.

## Process loss for the fixed grouped owner

The [new owner matrix](local-runs/fixed-grouped-process-loss-20261017/README.md)
keeps the shared production construction body and all earlier runtime sources
exact. Its private child harness differs from the old candidate harness only in
namespace, constructor and one exact formatting fold. New static guards cover
that equivalence and four negative mutations; all 199 contracts pass.

The first screen and both three-repeat compiler matrices pass: 36 SIGKILLs,
six ordinary controls and six zero-exit omitted-ACK controls in the main cohorts.
Including the screen there are 50 cases and 100 fresh-process refusals, with
checked original records/metadata unchanged and flock reacquirable after terminal
wait. The ACK is a harness-only return notice. No manager delivery occurs.

This closes the selected nonexecuting acquisition/return cuts for the new type,
not fixed-root enrollment, arbitrary syscall/power loss or actual execution.
During future execution a guardian must retain the fence independently of the
controller until all worker/collector writers cease. The private timer and this
process cleanup proof do not fulfill that stronger P2/P3 obligation.

## Private autonomous guardian mechanics

The [kernel packet](local-runs/autonomous-watchdog-20261017/README.md) now separates
the collector from a disk-independent guardian. A private barrier prevents disk
work until the guardian pins its exact unreaped collector and closes its own
sink aliases. The collector alone drains, bounds, syncs and closes output.
The guardian retains the fence, controller/writer/collector pidfds and original
selected absolute alarm; it has no regular-file output writes or fsyncs.

Ordinary controller death stops known pipe writers and allows drain only within
the existing bound. Expiry kills the collector too, emits explicit refusal and
retains the fence. Release requires the private post-close success notice,
actual successful child wait status, terminal known writers and two independent
EOF reads. A hidden writer blocks release even after every known writer exits.
Neither missing ACK nor later EOF can repair a refused run.

All 36 WSL and 72 three-repeat server cases pass, including actual stopped
collector/controller, SIGKILL, lost/disconnected readiness ACK, sink EBADF,
RLIMIT_FSIZE exhaustion and fixed-cap overflow. All 200 static contracts pass.
These private descriptors are not authenticated original objects. The original
common-fence destructor, all validators and 300/60/10-second origins stay exact;
there is no start/service route. Original ownership transfer, complete cgroup
revocation, durable completion and external guardian supervision remain open.
No standalone flock can survive its last guardian being killed or host loss.

## Original fence custody prerequisite

The [custody packet](local-runs/original-fence-custody-20261017/README.md)
isolates the original destructor hazard without changing its default short
observation behavior. A private one-shot transition brackets duplication of the
already locked original description with the unchanged fence inspector. Only a
successful transition selects close-only destruction for the original owner;
the private custody owner also closes without explicit unlock. The existing
ShortHostFence and campaign claim types are unchanged. No production caller,
public descriptor export or send/start capability is added.

The saved old-destructor counterexample fails independent common-lock contention
after original drop. The corrected code passes 14 fast tests, 18 WSL process
cases and 36 two-compiler host cases, including actual holder SIGKILL and omitted
or disconnected private ACK. Original-first and custody-first destruction both
preserve exclusion until the last owner closes. All 201 static contracts pass;
the prior autonomous packet remains independently verifiable at its old source.

This is not the full original-object transfer. The typed full validation frame,
original sinks, parent writer/lock alias closure, authenticated start and complete
cgroup revocation still need composition. Every original live predicate and
300/60/10-second origin stays exact. No installation or supervisor stop was needed.

## Custody joined to original sinks inside one complete frame

The [registered join experiment](diagnostic-original-custody-join-plan-20261018.md)
first composed two complete grouped validation frames around custody acquisition.
Both compiler healthy screens rejected its subsequent revalidation at the original
challenge age. That composition is withdrawn, not repaired by renewing time.
Its full rejected source capsule and both failures are retained separately.

The replacement consumes OriginalCollectorSinks directly. Its opening
material_round, fixed bundle/projection/message and absolute alarm preparation,
and closing original_round are the existing grouped constructor's full frame.
An opaque storage bridge retains only that acknowledged Starting storage's own
original fence description before the closing round. Custody metadata and
CLOEXEC inspections bracket the close. A final observation checks the same
absolute alarm and unchanged causal-age predicate after metadata IO. Only then
are the original material and new owner marked ready. No original validator or
original grouped API body changes, and no caller path, FD, key or time is accepted.

OriginalCustodiedPreparation owns the same original sinks, pipe/message aliases,
bundle and alarm before its private custody field. Revalidation brackets the
unchanged complete grouped validate with custody inspections and final original
deadline observations. Any failure poisons the complete borrowed chain and keeps
the refused owner, including any acquired custody. Destruction closes original
writers before custody. The new construction/revalidation count is the existing
baseline's 34/34 authentications, not a revised release or safety threshold.

This is still nonexecuting ownership, not guardian enrollment. No extraction,
clone, send, start, completion or recovery method exists. The new storage bridge
does not hand descriptors to the autonomous engine. Exclusive collector transfer,
parent alias closure, authenticated original start, complete cgroup revocation,
external guardian supervision and durable completion/later cells remain closed.
The original 300/60/10-second origins and every live predicate remain mandatory.

The [retained join packet](local-runs/original-custody-join-20261018/README.md)
passes both GNU compiler cohorts: six repeated healthy controls, forty persistent
refusals, thirty selected process cases and sixty fresh inspector refusals. The
process cases include eighteen real SIGKILLs, six normal returns and six synthetic
return-ACK omissions. A separate first healthy screen also passes. All 204 static
contracts preserve old bodies and reject undeclared frame/origin changes. Stable
host postflight preserves the supervisor, installed binary and receipt. These
are nonexecuting correctness results, not descriptor enrollment or performance.

## Original descriptor inventory before a barred transfer

The [descriptor plan](diagnostic-original-descriptor-plan-20261018.md) borrows
the exact original custodied owner and captures ten descriptors: custody, its
selected alarm, readers/sinks and four pipe/message writers. The initial guardian
allowlist includes custody, alarm, readers and sinks; the collector list contains
only readers and sinks. All four pipe/message writers are excluded from both.
The future guardian must still shed its sink aliases before the collector barrier.

Capture is bracketed by the unchanged complete custodied validation and exact
post-frame metadata comparison, then the existing final original deadline probe.
The plan cannot export an FD publicly, clone, start, send, release or complete.
It remains exclusively borrowed, not cached authority. No generic guardian
predicate changes: its O_WRONLY requirement does not silently accept the original
O_RDWR sinks. An explicit original-object process join remains necessary.

The first screen exposed an oracle error: four immutable origins were mistakenly
compared together with two advancing observation times. The correction preserves
exact origins and requires monotonic observations. A separate regression then
exposed preexisting nonzero sink offsets that equal snapshots alone would miss.
The new inventory now requires zero position; old sink/guardian bodies and
300/60/10-second origins remain exact. Both negatives have complete source capsules.

A gated test-only raw holder uses these actual descriptors, excludes all inherited
writer aliases and pins its exact child before parent destruction. Actual EOF,
independent common-lock contention and terminal wait status are the ownership
oracles. A hidden writer must refuse and retain custody even after later closure.
This holder is not the production guardian: authenticated message delivery,
barred original transfer, whole-cgroup revocation and external supervision remain
execution gates; no test-only ACK or FD inventory substitutes for them.

The [completed descriptor packet](local-runs/original-descriptor-plan-20261018/README.md)
passes six repeated healthy controls, twenty refusals and twenty-four holder
cases on GNU Rust 1.94/1.88. Those include twelve actual EOF controls, six explicit
holder SIGKILL cuts and six sticky hidden-writer refusals with separate owned
SIGKILL cleanup. Four new units and the 32-test original storage/fence subtree
pass per compiler; all 207 static contracts pass. Host postflight preserves the
same supervisor PID/restarts and installed digests. Parent alias closure is
proved for this nonexecuting holder, not production original-start enrollment.

## Full original frame behind an inherited process barrier

The [post-fork ordering experiment](diagnostic-original-barrier-validation-20261018.md)
adds no runtime constructor or guardian admission. It reuses the previous raw
holder unchanged: the child is pinned and isolated before the parent runs the
descriptor plan's complete revalidation. The child remains behind its private
command barrier during all live authority checks, inventory comparison and the
final original deadline probe. Parent descriptors remain present throughout.
Only a healthy frame and a still-live pinned child precede parent destruction
and the private EOF command. The four immutable origins stay exact; the two
observation times must advance monotonically. No alarm is replaced or renewed.

Persistent drift after fork must poison the original owner without sending the
private command. Common-lock contention after parent destruction proves the
barred child still retains custody. Selected child SIGKILL boundaries distinguish
the parent's remaining ownership from the last holder's death; retained history
must still forbid replay once kernel locks become available. A previous healthy
frame cannot make a terminal child ready, and empty-sink EOF/fsync is not durable
lifecycle completion.

This orders checks for a test holder, not the independent production engine.
Authenticated delivery must preserve the original unsent message until its send
boundary; destroying that message and reconstructing it later cannot grant start.
The production guardian must additionally bind the original alarm, controller and
complete writer tree, remain autonomous while barred, and shed sink aliases before
its collector runs. External guardian supervision and durable completion remain
required. The test holder's blocked command wait is not that autonomous watchdog.

The [completed barrier packet](local-runs/original-barrier-validation-20261018/README.md)
passes both compiler cohorts: twelve repeated healthy controls, eighteen
post-fork refusals and twenty-four selected SIGKILL cuts. Eighteen separate
refusal-cleanup kills are not successful transfers. Both first screens, 32+4
local fast units per compiler and all 210 static contracts pass. The original
runtime and raw holder bodies, supervisor and installed digests stay exact;
these are ordering and refusal results, not production guardian enrollment.

## Autonomous pending custody on the original alarm

The [barred custody experiment](diagnostic-original-barred-custody-plan-20261018.md)
adds an ordinary-build private custodian without activating a collector. It
inherits only the original custody description, the same selected alarm and one
private socket. No stream reader/sink, writer or message alias survives child
isolation. The parent keeps those original objects and its exclusive owner borrow
through complete validation and child cancellation. No File/PID/time/key input,
start/send, detach, release or completion is exposed in the ordinary build.

clone3 with only CLONE_PIDFD and SIGCHLD creates the exact pidfd atomically, before
any fallible parent restoration/check. There is no numerical-PID signal fallback.
The isolated raw child observes the original absolute alarm autonomously, never
reads/rearms it, and retains the fence after expiry or private peer loss. P is
pending isolation only, not manager acknowledgement; the private vocabulary is
registered separately in COMPAT. Drop is cancellation under a retained parent
borrow, not an active writer-cessation result.

The first wrapper consumed a separately validated descriptor plan. Both compiler
screens created custody, then refused the next full frame with Causal(Time).
Its four-frame composition is withdrawn and its complete source/screens retained.
The replacement captures the same draft Inventory directly from the sealed
original owner and closes with one unchanged full custodied validation, exact
inventory comparison, child observation and original deadline probes. Later
revalidation repeats that same full frame. No old body, predicate or 300/60/10
origin changes. The kernel implementation remains exact across the replacement.

This closes neither original collector enrollment nor authenticated start. The
original unsent message remains owned and cannot be destroyed/reconstructed as
authority. Exact collector handoff, whole-writer-tree binding/revocation, external
guardian supervision, durable completion and later cells still precede execution.
Nonroot host fixtures do not install authority, modify the supervisor or admit a
product/pilot workload.

The [completed pending-custody packet](local-runs/original-barred-custody-20261018/README.md)
passes six repeated healthy controls, sixteen refusals and eighteen selected
process cases across Rust 1.94/1.88, with two new first screens and separately
classified constructor-death/cleanup cases. Both compilers pass 32+5 local fast
units and all 213 static contracts pass. Both rejected four-frame screens and
complete source remain reproducible, with an identical raw kernel source body.
Stable postflight finds no old/new fixture executable and no supervisor change.
This establishes pending autonomous custody, not active writer-tree cessation.

## Pending owner death and ordered notice loss

The [owner loss packet](local-runs/original-custody-loss-20261018/README.md)
adds real SIGKILL of the original owning fixture process before and after the
unchanged closing full frame. Its external subreaper receives only the exact
custodian pidfd, not an original lock or notice-peer alias. The original fence
must stay busy past its bound, and retained history must refuse even after the
last custodian dies and the kernel flock becomes available.

Lost initial P is distinguished from pre-read expiry by requiring the first
remaining ordered notice after refusal to be E while the custodian remains live.
This test-only observation cannot renew a window or authorize release. A traced
attempt refused upstream Time before pending creation and is retained separately;
it does not waive a live check or prove notice loss. Ordinary pending code and
the frozen qualification inputs stay exact. Actual barred collector handoff,
authenticated original send and active-writer cessation remain required.

## Barred original I/O enrollment

The [barred collector packet](local-runs/original-barred-collector-20261018/README.md)
adds a private typed owner of the pending custodian and a second barred child.
Collector cancellation/reaping precedes ending the exclusive original borrow
and custodian lifetime. Failure objects retain that ownership. Both children
are prepared before one unchanged full closing frame; revalidation brackets the
same complete frame with collector pidfd/notice/alarm observations. No original
predicate, time origin or recursive frame is removed from the old validators.

The collector inherits exactly fence/alarm/readers/sinks/private endpoint: seven
FDs, excluding all four pair/message writer aliases. Original O_RDWR sinks are
not adapted into the separate generic O_WRONLY engine. Its B notice denotes
barred isolation only. It accepts no command, performs no stream/sink I/O and
neither reads nor rearms the original alarm. Expiry or peer failure quarantines
all four I/O FDs before a best-effort E/L/F notice and sticky parking. Its fence
alias persists, including after the separate custodian dies. A close error
cannot establish writer cessation, and cancellation is not active completion.

The preparing owner retains the original objects/message; enrollment is not
exclusive transfer or authenticated delivery. Two-child owner death and lost-B
proofs, malformed-notice runtime controls, authenticated activation, all-writer
binding/revocation, external active-guardian supervision and durable completion
remain distinct gates. No installed service or product workload changes here.

## Two-child barred owner loss

The [collector owner-loss packet](local-runs/original-collector-loss-20261018/README.md)
tests actual owner SIGKILL before/after the joint full frame. Only the two atomic
pidfds leave the fixture, over exact kernel-credential-checked datagrams; neither
fence nor private peer can remain alive through the observer. The collector must
quarantine its four I/O FDs after owner disconnect, both children must be adopted
by the temporary test subreaper, and either survivor must hold the fence past
the original bound after its sibling dies. Fresh preparation must still refuse
retained history after both exact children are killed/reaped.

Lost B requires first remaining E rather than merely an Expired result. Parser
units cover malformed notices and alarm/terminal precedence with synthetic
readiness FDs, separately from the real-pidfd process matrix. The ordinary join,
kernel, complete validation frames and four original time origins are frozen;
test seams cannot enter an ordinary build. This covers selected barred loss cuts,
not every instruction boundary, authenticated activation or active writer-tree
revocation. Explicit active-cleanup errors, external active supervision and
durable completion remain required before production execution.

This two-child experiment is withheld: 1.94 completed its twelve owner cases and
three lost-B controls, but the first 1.88 healthy screen refused the direct
challenge-age guard. A separate ABBA healthy diagnostic also refused on the
previous retained image; later green screens do not replace the stopped cohort.
The candidate's thirteen source files are archived and its fault seams/protocol
registration are withdrawn. Only parser-only tests are accepted independently.
The subsequent explicitly approved composition has a separate gate; it does
not replace or accept the withdrawn candidate.

## Custody and barred enrollment inside one frame

The [grouped enrollment packet](local-runs/original-enrollment-frame-20261018/README.md)
owns original preparation and both children without a self-referential borrow.
It enrolls the unchanged custodian/collector after opening observations and
before the unchanged closing round. Failure retains all resources and refuses
the original attempt. Child cancellation/reaping attempts precede original
writers and the final custody alias; Drop is not active cleanup authority.

Construction performs 34 authentication/signature calls rather than 68 across
two sequential complete frames. Revalidation remains a complete 34-call frame.
Custody, original inventory, both child states/notices, alarm and direct
challenge age bracket closing; readiness is last. Existing constructors,
validators and raw child bodies remain exact. The approved frequency change
does not renew Reserved/Starting/progress/challenge origins or weaken a predicate.

Both GNU toolchains complete eight healthy controls, eighteen specific refusals,
six actual lost-B controls and twenty-four owner SIGKILL cases. The observer
holds pidfds only; either surviving child retains the fence past the original
bound after owner death and collector I/O quarantine. Historical refusal stays
separate. These synthetic barred proofs do not establish authenticated original
send/start, whole-writer enrollment/cessation, external active supervision or
durable completion. Installation, product workload and qualification stay closed.

## One shot exchange before authenticated original delivery

The [local message-mechanics packet](local-runs/dispatch-exchange-20261018/README.md)
adds a private owner, not a manager connection or start capability. It retains
the supplied FD-bearing message on preparation failure and uncertain delivery,
arms Uncertain before transport entry and admits only one exactly correlated
reply. Timeout/disconnect closure is sticky: a delayed valid reply cannot heal
it. A canonical nonzero u32 job path is data, never unit/process identity or
completion authority. Manager error bodies are not diagnostics.

The selected manager/client names are checked claims only. Original signed
settings, ancestry and authenticated channel ownership are not established by
this primitive's shape checks or its same-process SCM_RIGHTS fixtures. The old
well-known-addressed original message remains unsent and unchanged; the trusted
channel must construct the uniquely addressed original message inside the full
live join. It must not rewrite the old message or promote barred P/B notices.

Fourteen units per GNU compiler include lost/late acknowledgement and a hidden
Message clone that prevents EOF after job receipt and owner destruction. This
demonstrates why local Drop cannot prove whole-writer cessation. No send/export
API, authority conversion, new admission clock, original fence release or server
operation is added. All old validation, manager/child/watchdog bodies and
Reserved/Starting/progress/challenge origins remain frozen. Fixed trusted
delivery, active supervision, whole-writer revocation and durable completion
remain separate unclosed gates; the frozen qualification contract is unchanged.

## Fixed live manager producer before the original join

The [channel packet](local-runs/manager-channel-20261018/README.md) owns a
current-thread runtime and one fixed system-bus connection. Root-selected
ancestry, actual SO_PEERCRED, atomic SO_PEERPIDFD and retained procfs/nsfs objects
precede trust in namespace-relative UID 0/PID 1 replies. Caller, bus peer and
PID 1 must share PID/user namespaces; the caller's time namespace is retained.
Pidfd liveness brackets proc lookups. Client name, manager owner, GUID, boot and
route are reobserved; closure or context failure latches refusal.

The two-second absolute BOOTTIME operation alarm bounds the entire observation,
including authentication/Hello and queries. It is never an original lease or
permission to cross a challenge/progress deadline. There is no original frame,
host-admission, message, unit/cgroup or writer-cessation capability in this
producer, and no exported connection/name/FD or injected backend constructor.
The consuming original join must enforce all original limits independently.

The first real-bus screen stopped on a reply flag that mock builder defaults
had omitted. Its frozen capsule and separate header attribution remain negative.
The second producer accepts only 0/NO_REPLY_EXPECTED on METHOD_RETURN, retaining
all other header/body/FD/context checks. Fresh current/MSRV root read-only cases,
nonroot access refusals and local controls pass; old dispatch/request predicates
remain exact. Their own interoperability review, unique-owner original-message
construction, Unix-FD delivery proof, active supervision and all-writer revocation
still precede authenticated start, completion or installation.

The [ownership audit](supervisor-boundary-audit-20261018.md) retains process and
cgroup management in systemd and admission/progress/evidence rules in the custom
layer. The [separate dispatch correction](local-runs/exchange-reply-flags-20261018/README.md)
now admits only flags 0/NO_REPLY_EXPECTED on METHOD_RETURN in the old exchange;
requests, ERROR classification and every correlation/FD/refusal check stay exact.
Both compiler cohorts pass nineteen exchange tests. This closes its isolated
reply interoperability prerequisite, not original message delivery or execution.

## Original receiver and live manager under one frame

The [original manager join](local-runs/original-manager-join-20261018/README.md)
consumes OriginalEnrollmentFrame and retains that exact owner, its children,
writers, alarm objects and custody together with the existing fixed live
channel. An opaque binding clones the actual retained kernel Origin from the
original progress packet after sender/hold revalidation. Boot and typed nsfs
user/mount/PID/time objects must match the channel's selected caller view. A
fresh caller namespace capture alone cannot replace original receiver proof.

Construction and subsequent validation surround one unchanged complete frame
with live channel/context checks and original observations. The full frame
still performs 34 authentication and 34 signature checks. All old predicates,
origins, absolute alarms and 300/60/10-second windows stay unchanged. Failure
retains and poisons the original owner; no retry, channel/frame extraction,
caller-supplied channel, serialization or message-send path is provided.

The shared round is tested with a distinct test-only channel; the actual fixed
channel/context component is separately tested read-only. These scopes must not
be conflated with a positive production-enrolled full join. Local Challenge(Time)
remains a stopped negative cohort. Host harness/test corrections and independent
host results are recorded separately. Unique-owner original message construction,
FD negotiation/delivery and authenticated start remain next, ahead of active
watchdog/all-writer cessation, completion, reconnect and real host enrollment.

## Withdrawn original message addressing experiment

The [addressed message slice](local-runs/original-manager-message-20261018/README.md)
candidate consumed that private manager join without accepting a message, identity or
factory argument. It derives both unique names from the held channel, keeps
the legacy message intact and builds a distinct unsent message with its exact
canonical body and serial. Old signed property validation stays unchanged.
New envelope checks and typed original-pipe checks pin two extra writer aliases;
missing/replaced aliases or manager/client drift permanently refuse the owner.

Two unchanged complete channel/frame rounds surround payload preparation and
later validation. This adds work (68 authentication/signature calls of each
kind), preserving all old checks, child FD inventories, alarms and original
300/60/10-second windows. Extra aliases close before original custody. The
failed owner retains all resources; no getter, retry, reconstruction, serialization,
send/start, completion or release interface is added.

Test-only original-frame controls use a distinct channel. The same-process
SCM_RIGHTS adapter transfers this exact message and original writer objects
inside the opening/closing full round. It is mechanical socket evidence, not
negotiated system-bus delivery or a positive production-enrolled full join.
Authenticated original start, actual process/executable/cgroup binding, active
watchdog/all-writer cessation and completion remain later gates.

The selected healthy host case passed preparation, then refused Causal(Time)
on the next call. No remaining current/MSRV host matrix or new-path delivery
passed. A separate numeric diagnostic observed original challenge age increase
from 9.301862561 s to 9.976507282 s during the added 68-check round, followed by
refusal at 10.116451429 s. No origin or deadline changed. The runtime candidate
is withdrawn and archived; a proposed nonexecuting grouping under the existing
full frame awaits agreement. The approved original manager join remains intact.

## Grouped original manager message preparation

The separately authorized [grouped source slice](local-runs/original-manager-grouped-message-20261018/README.md)
consumes the original frame directly and initializes the retained receiver/pin
and fixed channel without an extra already-validated join. Its sealed round
observes original context and channel, prepares or inspects the exact canonical
addressed message, validates the complete original frame once, then checks the
same channel/context, payload and original observations again. Both live
channel/context checks and every full-frame predicate remain; only duplicate
full traversal of the rejected prototype disappears. This remains sequential
observation, not atomic kernel attestation.

Only the held connection supplies unique names. The body, serial and original
typed pipe objects are pinned, with two explicitly owned message aliases that
drop before the original custody. Missing/replaced payloads cannot reconstruct;
failure retains and poisons the same owner. The approved old join, old legacy
message, child descriptor inventory, alarm origins and 300/60/10-second windows
stay exact. No send or execution authority is added.

Source `2ed04dc8` passes current/MSRV scoped host fixtures: twelve healthy full
rounds with 34 authentication/signature checks each, six same-process socket
transfers, forty sticky refusals and two real nonroot channel refusals. Tests
use a distinct channel probe, never a production-owner factory. They do not
prove positive production enrollment or negotiated bus delivery. Observed
successful challenge age approaches 10 s with only about 25 ms remaining.
Budget attribution, actual FD negotiation/correlated one-shot start, original
process binding, active all-writer supervision, completion and reconnect remain
required before admission. The failed earlier cohort remains withdrawn.

The [subsequent budget attribution](local-runs/original-preparation-budget-20261018/README.md)
keeps those runtime bodies and all observation boundaries unchanged. An exact
original-helper equivalence guard permits only test-only numeric insertions.
Six current/MSRV scenarios localize 7.185–7.222 seconds to joined outputs with
782 authentications. The source performs nineteen single frame traversals for
Starting publication, six for pipes and twenty-one for sinks: 46 traversals at
seventeen authentications each. These are effect/checkpoint brackets, not
permission to reuse a successful guard across a write or callback. Observed IPC
authentication time is only 25.938–28.319 ms within that bucket; its signature
subspan overlaps and other cryptographic work is outside that timer. Further
attribution must examine the original worker/kernel/authority walks, preserving
all live predicates and original deadlines. No dispatch or admission follows.

The [retained live worker profile](local-runs/original-live-walk-20261018/README.md)
in `bd65514b` adds only six cfg(test) entry spans and a before-reservation
control. All 349 old Rust sources restore exactly after removing the approved
insertions/registrations. Six current/MSRV fixtures pass 192 validations and
36 nested numeric rows with zero IPC authentication/signature calls. Kernel
context contributes about 69 percent of the whole-worker wall span; authority
about 12 percent. Do not sum a policy parent with its source/authority children,
extrapolate an exclusive CPU/IO owner or transfer these results to the complete
Starting/pipe/sink path. Preserve its 46 effect-local brackets. Kernel sender,
origin, namespace/boot and clock observations are the next attribution target;
real bus delivery/start and active all-writer cessation remain separate gates.

The [nested kernel profile](local-runs/original-kernel-walk-20261018/README.md)
in `da663790` retains every old runtime statement while adding ten cfg(test)
entry spans. Six fresh current/MSRV fixtures verify the expected call graph:
sixteen origin validations and forty-eight bounded proc reads per worker
validation, with 174 raw rows and exact parent/child wall accounting. Origin's
inclusive contribution is 82.2–82.5 percent of kernel context. Sampling mostly
contains origin validation rather than clock work. A small fixed boot-read
buffer is a possible allocation candidate; the profile does not measure
allocator calls, deallocation in parent scopes or a product speedup. Preserve
all 46 effect-local brackets and original deadlines. Actual authenticated
start, active all-writer cessation, completion and reconnect remain unadmitted.

The [boot document buffer candidate](local-runs/boot-buffer-20261018/README.md)
is an isolated experimental runtime substitution after that profile. Its
38-byte stack buffer retains one positional read, canonical validation and
closing the document before decoding. Sender status/pidfd `bounded` reads,
fresh origin/namespace/time/authority checks, latches, all 46 effect brackets
and original deadlines remain unchanged. The source adapter must exactly
restore all 353 old Rust sources outside the declared substitution. Allocation
counting is confined to a separate integration image; it establishes only the
temporary-reader reduction. Plain matched original-worker timing and the full
preparation healthy/fault gate were still required at that local-only checkpoint.

The [plain worker and complete preparation gate](local-runs/boot-buffer-plain-20261018/README.md)
now accepts source `8e7e5da1` for nonexecuting preparation. The only active
runtime comparison is the boot-buffer substitution. Five fresh pairs per GNU
compiler pass the frozen local wall criteria; all twelve healthy message rounds,
forty sticky refusals, six owned transfers and two real channel refusals pass.
No original predicate, origin, effect bracket or deadline is removed. Maximum
observed successful challenge age leaves 451 ms, not a guaranteed dispatch
budget. Same-process rights transfers remain distinct from negotiated system-bus
delivery. Authenticated original-start, all-writer cessation, completion/later
cells and new-boundary SIGKILL/lost-ack/reconnect remain unadmitted.

The [private negotiated delivery gate](local-runs/negotiated-dispatch-20261018/README.md)
tests the unchanged exchange against actual D-Bus routing and rights negotiation,
without constructing an original/fixed-channel owner. Both compilers pass nine
matrix cases and three receiver units. Positive fixtures require an explicit
atomic CLOEXEC receiver: the pinned library's bare receive path is a retained
negative control. The private fault policy allows unsolicited replies only to
exercise the unchanged classifier, not to model installed policy. A received
Message keeps writing after exchange and connection closure; no reply, sender
Drop or disconnect proves cessation. Keep original custody/fence ownership and
all 46 live effect brackets until the future all-writer proof exists. Production
original delivery, active execution, completion and reconnect remain closed.

The [FD-free reply-state prerequisite](local-runs/reply-state-owner-20261018/README.md)
retains exactly one request owner while separating names/serial/phase/latch data.
The existing exchange delegates to the original unchanged state-machine methods.
A standalone state can classify data after request destruction, so it is not
custody or admission proof. The original message/frame/channel implementation
remains untouched; a later binding must keep this state under that same complete
validation round, without cloning or extracting its Message. Private-bus and
seeded model checks preserve the old behavior but open no execution/lifecycle gate.

The [original reply binding](local-runs/original-reply-binding-20261018/README.md)
now retains opaque preparation-only correlation in the original payload. It
borrows the same Message and checks original serial, names and Prepared/closed
state together with unchanged request predicates. The fixed channel's sealed
names comparison runs at both boundaries of the existing complete round; its
live authentication and receiver checks remain. Canonical body/FD identity,
sticky original refusal, custody, all 46 effect brackets and original admission
origins/deadlines remain intact. No new Message/FD owner, full walk, sender or
arm/observe capability is introduced. Complete-frame fixture evidence is a
separate gate from the ordinary correlation controls, never production enrollment
or whole-writer cessation proof.

## Two phase launch boundary

The approved [two-phase launch plan](two-phase-launch-plan-20261019.md) replaces
direct worker execution with a held launcher only under a separate signed v2
requirements schema and domain. Existing v1 consent and collector-unit bytes
retain their meaning. The new document binds historical consent claims and
launcher/watchdog/collector artifact roles; it is not an executable unit or an
original-owner permit. The original live join must still authenticate and open
the exact artifacts, pre-arm an independent guardian, negotiate original FD
delivery, pin the complete cgroup, validate the complete frame and release the
barrier once. All original 300/60/10-second origins and live predicates remain.

The private barrier mechanics intentionally accept no executable, evidence sink,
cgroup or fence. Their owned kernel fixture can establish packet handling,
pidfd loss, absolute expiry and at-most-once delivery, but live surrogate pidfds
cannot prove watchdog readiness or whole-writer revocation. The guardian must
also cover death after the barrier's last observation; a final liveness check
alone cannot close that race. Both pipe EOFs, external writer closure and
collector finalization remain necessary before any future fence release.

## Private prearmed guardian boundary

The [prearmed watchdog plan](prearmed-watchdog-plan-20261019.md) and
[local contract](prearmed-watchdog-contract.toml) add private mechanics, not an
original execution owner. A guardian is atomically pinned through CLONE_PIDFD,
isolates inherited descriptors and retains a real fence, creating-owner pidfd
and the same absolute BOOTTIME timer before Pending. Separate one-shot enrollment
requires actual creator credentials and one live direct-child pidfd. That pidfd
is retained before Armed, so owner loss before a lost ACK still triggers target
revocation. Sender uncertainty is latched before send; no retry or renewal is
available. Actual SIGKILL and EOF must be independently observed, not inferred
from the guardian's notice. No completion or unlock operation exists here.

The owned fixture intentionally stops target-side monitoring after its single
constant pipe effect. Its owner-loss result therefore tests active guardian
behavior, not a cooperative child exiting on owner death. Hidden writer aliases,
foreign senders, malformed rights, repeat messages, expiry and lost ACK have
separate controls. Guardian death before release refuses the barrier; guardian
death after release instead exposes a still-live writer and a released last
flock holder. A SIGSTOPed guardian cannot act at the original deadline until
resumed. These last two are retained expected-negative results, not readiness
proofs or reasons to change deadlines.

The receiver's ambient procfs/direct-child check cannot authenticate a
systemd-managed target or historical original admission. Future integration must
retain the original trusted kernel/cgroup context and original collector and
custodian owners, authenticate all artifact and stream identities inside the
complete validation frame, and join independent revocation without discarding
those existing fence holders. It must cover every writer, not just one pidfd.
Original delivery, release, durable completion, later cells, reconnect and
production policy/enrollment remain closed. The
[evidence packet](local-runs/prearmed-watchdog-20261019/README.md) is local-only;
it does not authorize installation, supervisor operations or a server workload.

## Redundant local guardian experiment

The [redundant watchdog plan](redundant-watchdog-plan-20261019.md) composes two
unchanged private guardians. Actual Pending from both precedes target creation;
both enroll the same direct-child pidfd and retain aliases of the same fence
OFD, with exactly the same selected absolute deadline. A global latch precedes
the first enrollment send. Partial enrollment, lost ACK, retry or a failed
closing live check poisons the pair; the fixture cannot release until both Armed
and that closing check succeed. No completion, unlock or replacement is added.

One dead/stopped guardian no longer removes all local revocation capacity: the
survivor handles creating-owner loss or the unchanged original bound and keeps
the fence. This is not immediate sibling-failure detection or a guarantee under
common-mode failure. Both-dead and both-stopped controls still expose a live
writer; the last flock is available after both holders die. A hidden external
write alias remains outside the pair's revocation authority and prevents EOF.
The [local contract](redundant-watchdog-contract.toml) explicitly keeps original
admission, whole-cgroup/all-writer proof, production construction and server
qualification closed.

Retain the original collector/custodian holders and bind trusted original host,
manager, signed artifacts, process/cgroup and streams inside the full frame
before delivery. Do not remove the prototype's direct-child check to admit a
systemd-managed process. The [evidence](local-runs/redundant-watchdog-20261019/README.md)
tests local independent custody only; durable exclusion/completion, later cells,
new-boundary crash coverage and reconnect remain separate gates. Original live
predicates, 46 effect brackets and 300/60/10-second origins remain unchanged.

## Borrowed original watchdog context

The [original-context plan](original-watchdog-context-plan-20261019.md) adds an
opaque preparation view from an exclusive borrow of the same ready
OriginalManagerMessage. It retains the original frame/collector/custodian,
receiver and fixed channel. Derived state/identity, four origins, three bounds
and existing selected timer deadline, descriptor/pidfd roles, serial/names and
addressed aliases are compared around one unchanged complete addressed round.
Actual current process/thread and real/effective credentials are also retained.
No caller supplies those inputs or obtains their raw descriptors.

The healthy fixture still performs 34 authentication/signature calls and two
channel checks; comparison is not cached admission. Opening/closing mismatch or
any original-frame failure poisons the payload and original chain, so retry
cannot enter another round. The view retains no new FD owner and creates no
timer or child, has no send/release/completion API, and cannot be constructed by a
test-channel probe. Dropping it only returns the original borrow; it produces no
transferable or durable capability.
The contract's new_fd_owners=false concerns retained aliases in the view, not
temporary kernel-document descriptors opened by the required live observations.

The [contract](original-watchdog-context-contract.toml) keeps guardian spawn,
v2 artifact binding, original send, all-cgroup/writer cessation and production
admission closed. The retained message is still original v1. Future enrollment
must bind the reviewed v2 roles and retained original kernel, timer/fence and
process/cgroup ownership instead of promoting asserted metadata or loosening
the local prototype's direct-child check. The
[evidence](local-runs/original-watchdog-context-20261019/README.md) separates the
unchanged-baseline refusal from isolated healthy/drift controls. All original
predicates, effect brackets and admission origins remain unchanged; server and
supervisor operations are not part of this package.

Both GNU 1.94 and MSRV 1.88 pass fourteen explicit full-frame context scenarios,
four ordinary controls and retained exchange/guardian regressions. The complete
271-control source suite preserves all 372 old Rust bodies through exact additive
registrations. Evidence binds each executed image to the same committed source
and nine unchanged inputs. The context does not enroll the local guardian pair;
that pair's separate regression harness must not be mistaken for original
watchdog admission or a proof that every writer has stopped.

## Original Pending guardian custody

The [Pending plan](original-pending-watchdog-plan-20261019.md) consumes the same
OriginalWatchdogContext into an original Pending owner. Its private mechanics
adapter borrows the real fence and existing timer from original storage/alarm
objects and checks their retained roles. Two independent children inherit the
same fence OFD and aliases of the same timer OFD. No timer creation/read/rearm,
flock operation, target enrollment, original send or barrier release is added.
The old credentialed Pending wire/body and direct-child guard remain unchanged.

The complete original context round executes before and after kernel setup:
68 authentication calls, 68 signature calls and four channel checks in the
healthy fixture. Revalidation keeps one full round and both Pending observations.
The production wrapper retains the original frame, collector/custodian,
receiver/channel and payload aliases. Its failure owner retains the refused
original borrow; the distinct test probe cannot construct it.

Parent cleanup is owned before signal restoration can fail. Cancellation
signals and waits through the atomic clone3 pidfd of only a newly created,
targetless Pending child. Partial setup cancels those new children while
original custody remains live; cancellation is not completion or fence release.
New parent-side timer aliases and process/channel handles are retained, not
hidden behind a zero-FD-cost claim. Required original live checks and all
300/60/10-second origins stay unchanged.

The [contract](original-pending-watchdog-contract.toml) keeps Armed/target
enrollment, responsiveness, common-mode guarantees, v2 artifact/cgroup/all-writer
proof, original delivery and server admission closed. The
[evidence](local-runs/original-pending-watchdog-20261019/README.md) covers ordinary
kernel mechanics and full-frame refusal cases, not the future complete send,
release and durable-write fault matrix. Actual target admission must bind trusted
original artifacts/process/streams/cgroup rather than deleting the old local
prototype's direct-child predicate or passing an asserted pidfd as authority.

Both GNU compilers pass ten new Pending and fourteen retained context frame
cases, with kernel/type/exchange/guardian regressions. The full source suite
passes 274 controls and preserves all 376 old Rust bodies, protocols and harnesses.
Earlier clock-preflight refusals remain refused; bounded environment settling
precedes the unchanged strict probe and creates no admission. No source predicate
or original bound is waived. Best-effort Drop is not a successful cleanup receipt:
the future completion path must independently prove cessation before unlock.

## Original Armed topology boundary

The [arming audit](original-arming-boundary-audit-20261019.md) requires an
explicit production topology decision before target enrollment. The private
guardian requires a creator's direct child, while the systemd-created held
target only appears after delivery. The signed v2 order requires Armed before
that delivery. Current read-only process/tree/manager checks do not provide a
pre-delivery revocation root, and original material still binds the v1 worker
projection.

An attempt-specific empty cgroup root, potentially a dedicated systemd slice,
is the recommended design for review. It requires its own enrolled kernel
identity, guardian protocol, held-unit projection and host setup; none is granted
by Pending or by this audit. Preserve old protocols, direct-child predicates,
the full validation frame and original deadlines. Even an armed cgroup revoker
cannot establish closure of external writer aliases or exclusive finalization.
The operator approved the attempt contour and new protocol on 2026-10-19. Follow
the [local implementation plan](cgroup-root-enrollment-plan-20261019.md); approval
allows isolated local fixtures, not production enrollment, installation or sends.

The first [root enrollment package](local-runs/cgroup-root-enrollment-20261019/README.md),
source `593e5148`, adds fixed 104-byte claims and a private pinned cgroup2 root
with root-owned type/events/procs/kill descriptors. Actual name replacement
cannot redirect kill submission. GNU 1.94/MSRV 1.88 each pass three eleven-case
local kernel matrices plus ordinary and retained guardian/exchange controls.
Before-write kill-caller death, late population and outside writer aliases
remain explicit limits. This does not construct an autonomous root guardian,
authenticate enrollment FD transfer or bind a production held unit. The
[contract](cgroup-root-enrollment-contract.toml) keeps Armed, original constructor,
completion and server admission false pending that next integration.

The next [descriptor transport package](local-runs/cgroup-root-transport-20261019/README.md),
source `65b02f3b`, authenticates per-message creator credentials on a private
seqpacket pair and validates exactly three atomic-CLOEXEC directory/kill/events
rights against actual root-owned cgroup2 objects. Both operation objects consume
before their first syscall, including a failed send or empty receive. Rejected
installed rights close, and no retry follows missing confirmation. The private
ordinary-process result has no kill or ACK method; it cannot replace a
stack-safe guardian or original-owner constructor. That integration must retain
the original process pidfd, fence and existing timer, preserve every full-frame
predicate and original bound, and authenticate the actual guardian reply. Held
delivery, cessation/completion and server installation remain closed.

The [local root guardian pair](local-runs/root-guardian-20261019/README.md),
source `72f53806`, adds stack-only v2 child handling instead of calling the
allocator-based reader after clone. Both children keep the existing fence/timer
open descriptions and actual creator process pidfds. Each retains three
authenticated root roles before replying, then opens an independent event watch
on the same inode. Shared event notification state would invalidate the claimed
single-guardian independence. Revocation acknowledges its watch before writing
the original kill FD and continues across empty roots and late population.

No timer counter is read or timer rewritten. The parent reads timer fdinfo under
a BOOTTIME consistency bracket; this does not reconstruct original expiry
identity or supply an original owner. Initial local ACKs are checked against
actual clone3 PID/pidfd/credentials. Closing a possibly enrolled parent's
channels requests revocation without cancelling independent holders. Partial
creation before any send retains exact-child cleanup. Both-dead and both-stopped
failures, outside aliases and untested creator SIGKILL remain explicit limits.
Original frame/trust/manager/held projection, durable completion and server
operation are still closed; this is not a new original runtime constructor.

### Creator death and the original root frame join

The [creator and frame package](local-runs/root-creator-20261019/README.md)
adds an external subreaper and actual pidfd SIGKILL of the exec-created creator.
Cut notices and exported guardian pidfds are test-only. A channel shutdown or
normal Rust Drop is not substituted for creator death. Writers must be alive
before the cut; late population and outside pipe aliases remain separate checks.

The crate-private `OriginalRootGuardians` consumes the opaque original context
and borrows a pinned root. It derives its packet binding from the original
owner/state/origins/bounds/FD roles, fixed manager names, serial and addressed
aliases, rather than accepting caller digests or deadlines. Preparation and
enrollment each retain two full rounds; revalidation retains one. A closing
round failure refuses both original owner and kernel controls. Possibly enrolled
guardians keep their independent revocation obligation and original fence.

The dynamic original-frame gate still uses a distinct simulated kernel; the
real root adapter has its own root-only fault gate. Neither gate alone proves
the combined privileged original manager path. Root pinning verifies kernel
objects, not protected host admission. Held-unit projection, whole-writer
cessation, durable completion, fence release and server operations stay closed.

The [retirement follow-up](local-runs/root-retirement-20261019/README.md)
also refuses the original owner before kernel controls in wrapper Drop. Channel
closure alone cannot exclude constructing a fresh pair once the exclusive
borrow returns. Both pending and consumed retirement are fail-closed; active
guardians remain independent and keep the fence. The original-frame regression
uses fresh simulated controls after retirement, so the old controls' refusal
cannot conceal a still-usable owner. No clock renewal, completion or unlock is
added, and the combined privileged constructor remains a separate gate.
