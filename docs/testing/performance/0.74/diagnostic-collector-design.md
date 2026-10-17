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
