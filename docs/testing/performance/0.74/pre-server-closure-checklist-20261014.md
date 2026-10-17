# Diagnostic prerequisites before server execution

Server execution is not admitted yet. This checklist follows the approved
[P0 through P5 sequence](diagnostic-server-readiness-intermediate-plan-20261012.md)
and [collector amendment](diagnostic-collector-design.md). Local primitives and
kernel facts are distinct from a complete installed production path.

## Implemented local prerequisites

P0 has retained original-process SIGKILL/lost-ack proof. P1 has original attempt,
Reserved and first Starting retention with signed history inspection and no
recovered acknowledgement. The collector writer requires actual EOF on both
pipes, exact bounded retained bytes/hashes, sync and explicit descriptor close.

P2 implementation `f2dbbe64c0fbf0a0eeca3bf6e8e2177188bc7f67` obtains kernel
per-message credentials and pidfd, retains the live original sender credential
projection, and samples boot/namespace-bound CLOCK_BOOTTIME/CLOCK_REALTIME.
Independent supplied-key signature checks no longer require caller peer/time
arguments on this new path. This does not load or enroll production trust.

## Required operator input for production trust

The reviewed controller and distinct builder public keys already exist in the
[public builder policy](diagnostic-builder-public-policy.json). Its raw SHA-256
is `dfe81cf770088029e03f04faa4da429ef163583c0ba1b7002b8ec5606e766e22`, matching
the [October 9 enrollment](local-runs/diagnostic-builder-enrollment-20261009/manifest.json)
and separately verified hosted-build evidence. The builder seed was retained in
a protected GitHub environment secret, not a local key file. SSH access keys are
separate from these signing authorities; regenerating them is not a prerequisite.

The [reviewed-trust entry points](diagnostic-reviewed-trust-design.md) reuse these
fixed public pins for builder-policy loading and opt-in kernel controller-key
selection. They do not establish current revocation status or installed trust.

The operator-authorized separate worker issuer key has now been generated locally;
see the [public selection](diagnostic-worker-issuer-public.json) and
[custody/use instructions](diagnostic-worker-issuer-key.md). Its fixed non-policy
selfcheck verifies with OpenSSL and Rust strict Ed25519. The private key remains
outside Git in local WSL storage, not on the performance host or in a new CI secret.

Still select the signed production worker policy with its exact canonical body
SHA-256 and current nonzero authority epoch. Supply an
independently reviewed public-config file or approved public values; no private
key or token belongs in chat/evidence.
An envelope cannot choose its own verifier, digest or epoch. Fixture seeds, keys,
namespaces and modeled host facts cannot fill these slots.

The historical worker-policy packet uses synthetic issuer keys. Creating the
separate real issuer now does not retroactively enroll that packet or a real host.
Production trust loading/enrollment cannot pass until the
remaining operator input exists and is joined to trusted host/root ownership.
The fixed controller/build/worker verification primitives are not themselves
proof of independent production selection. Choosing how keys are generated,
stored and controlled is a deployment decision, not an implicit test default.

## Code and local gates still open

Original anonymous pipe preparation `a533e6df` now retains the same acknowledged
Starting and fence without FD export or worker start. Its
[packet](local-runs/original-collector-pipes-20261016/README.md) passes 19 joined
cases/compiler, original field-drop ordering and private EOF after drop. This
closes only ownership of unexported pipe objects. Production trust/epoch,
sole-root sinks, executable/manager/process/cgroup start, autonomous watchdog and
revocation of every exported writer remain open before completion/cells2–4.

The first [sink preparation screen](local-runs/original-collector-sinks-20261017/README.md)
stops before RootCreated and is withdrawn. Separate attribution reaches the
original causal expiry without creating output. That candidate never reaches
its complete SIGKILL/lost-ack matrix; unchanged deadlines and guards are not
waived. A joined validation-cost/ordering proposal precedes a new candidate.

The [approved J1 join](diagnostic-output-join-plan-20261017.md) now has repeated
baseline attribution and three healthy complete output chains per compiler.
It keeps every original predicate and anchored deadline, with a preregistered
working/closing round schedule rather than recursive duplicate traversals.
The separately retained corrected cohort passes 27 sink and 41 other joined
fault/live/expiry cases per compiler. Its separate three-repeat/compiler
process-loss gate passes 156 SIGKILL, six controls and 324 fresh inspections,
including AfterReturn/lost acknowledgement. This does not test power loss or
exported-writer cessation and does not establish production trust enrollment,
original start or server admission.

The [original controller attempt](diagnostic-controller-attempt-design.md) now
consumes the original common-fence preflight into a durable Reserve request,
retaining its original OIDC/kernel/authority chain and uninterrupted lock.
The standalone bytes are unsigned consistency data, not recovered admission.
The separate [joined SIGKILL/lost-ack matrix](diagnostic-controller-attempt-crash-contract.toml)
now covers the eight write cuts and AfterReturn with owned nonroot processes.
It preserves retained bytes and refusal in two fresh readers per case; its
kernel sender is the fixture child itself, not an enrolled external controller.
Owned lifecycle integration, actual production trust and original-start/
all-writer cessation remain before server execution.

The [process packet](local-runs/controller-attempt-crash-20261016/README.md)
also retains one unexplained Kernel(Invalid) while constructing the original
guard before a write checkpoint. Same-source isolated and full-suite successes
do not identify its cause. Attribute that refusal before treating the integrated
admission path as stable or enrolling it for server execution; do not relax the
kernel/time guards to remove the failure.

The [scoped classifier](diagnostic-kernel-refusal-attribution-design.md) now
records the first Invalid site and fixture phase in test builds, with existing
numeric clock observations for ordering/hold refusals. It preserves original
ordinary-build constructors and checks guard/source equivalence against the
pre-instrumentation baseline. Model classifications and clean repeats are not
a historical root-cause finding or an enrolled production diagnostic route.

The [fixed trust input](diagnostic-worker-trust-input-design.md) now retains the
selected issuer and explicitly configured body pin/epoch from a bounded original
root-owned file, with an owned config/journal/policy join. Its positive filesystem
and signed-policy cases are private nonroot models with synthetic issuers. No
production configuration or worker policy is created, and current host context
still needs independent kernel admission. This does not close P2.

The separate [worker/kernel join](diagnostic-worker-kernel-binding-design.md)
now compares verified policy with actual retained local machine/boot/user/mount
context and the original kernel packet, without a caller host argument. It
retains config/journal/policy, Probe and packet ownership through cross-refusal.
Real production policy/epoch, source-opener/physical-host enrollment and
controller provenance/replay/fence remain open; no execution route follows.

The [controller principal input](diagnostic-controller-trust-input-design.md)
adds the fixed root-owned configuration reader and an original signed-request
guard borrowing the same worker/kernel chain. It verifies the selected signer's
statement and actual peer/freshness, not independent GitHub/build provenance or
nonce consumption. Real allowlists/configuration, worker policy/epoch and source
enrollment remain unselected; durable replay/fence and P3 are still required.

The [controller and builder receipt binding](diagnostic-controller-build-input-design.md)
retains the independently pinned builder policy and signed statement against the
original request identity, with whole-chain refusal. The receipt has no controller
run/actor/workflow identity: independent controller provenance is still required.
Installed executable contents, durable nonce consumption and common fence are
not supplied by this read-only guard.
The [approved local OIDC decision](diagnostic-controller-provenance-decision.md)
now has a [configured original-input verifier](diagnostic-oidc-input-design.md).
The existing campaign signing workflow cannot produce a diagnostic packet.
The separate [job-side producer](diagnostic-controller-producer-operator.md)
prepares/signs the existing diagnostic body and derives its request-bound audience.
Its [retained signing API](diagnostic-retained-producer-design.md) now avoids
public-file rereads and requires the earlier audience before key IO. This is
not actual token coordination, signer/bootstrap custody or receiver admission.
Local synthetic signatures and offline public discovery/JWKS inspection do not
authenticate acquisition, select actual workflow/key policy or mint a real token.
The [separate acquisition tool](diagnostic-oidc-acquisition-design.md) now bounds
fixed HTTPS in an isolated child and records public transport/source metadata.
Its initial synthetic loopback TLS and timeout packet does not choose actual
issuer keys or establish reviewed platform-CA/source custody.
The subsequent [actual public observation](local-runs/diagnostic-oidc-public-source-20261014/README.md)
received the fixed discovery/JWKS once with matching runtime/CA inventory and
four unselected keys. It adds no selected kid, snapshot expiry, workflow subject,
production custody or token. The [dedicated job preparation](diagnostic-controller-job-preparation.md)
separates the local [bounded-roster implementation](diagnostic-oidc-roster-design.md)
from actual key/workflow selection and the prebuilt-signer/bootstrap custody gap.
V1 remains readable; v2 adds bounded selected membership and no key refresh.
No active job or configuration follows.
Independently authenticated GitHub key selection/custody/rotation, actual workflow
policy selection and a protected diagnostic signing-job receipt remain required
before real provenance enrollment. P2 and replay/start admission remain open.

The [original controller/common-fence preflight](diagnostic-controller-fence-preflight-design.md)
now brackets the retained OIDC/controller/kernel/authority chain under an already
owned common empty fence. Errors refuse both chains and retain the original lock
until the result/failure is dropped. This read-only connection creates no durable
attempt, consumes no nonce and has no start conversion. The separate consuming
request-retention API and its process matrix do not make that constructor writable
or provide lifecycle/start conversion; real trust selection is unchanged.

The [original collector preflight](diagnostic-original-collector-design.md) now
binds the signed collector requirements to that acknowledged request owner,
original kernel peer/time and configured worker/controller authority without
creating a second worker guard. It retains the original fence and cross-refuses
on drift. The Reserved state is still a supplied signed projection; original
durable lifecycle/root-ancestry integration is needed before output creation.

The distinct [observed reservation path](diagnostic-original-collector-design.md#observed-initial-state-under-the-same-owner)
now creates and freezes initial state from the retained kernel boot/boottime.
Collector binding accepts only signed bytes, not a replacement state or clock;
the original request owner and fence are consumed into this in-memory owner.
This observed guard alone does not publish durable state. Starting publication,
root ancestry and execution remain open.

The subsequent [collector Reserved publication](diagnostic-collector-reserved-design.md)
now retains initial state and signed requirements with the original request
digest under the same authority/fence. Its exact two-document engine does not
relax the legacy one-file reader. This closes initial retention only: Starting,
authenticated progress, root output/start, writer cessation and host enrollment
remain pending. Readable bytes do not recover acknowledgement after a crash.
Its [local process packet](local-runs/collector-reserved-20261016/README.md)
passes three repetitions, totaling 30 owned SIGKILLs, 3 controls and 66 fresh
inspections. This closes the new initial-retention process boundary only,
not reboot/power-loss or authenticated execution.

The [authenticated heartbeat preflight](diagnostic-collector-progress-design.md)
adds one new signed message from the original receiver under the same authority
and fence. Request/controller/revision and actual receive freshness are checked,
but no progress entry or Starting state is committed. It cannot renew original
admission, prove continuous presence or revive an expired execution lease.
Durable Starting/replay/deadline integration remains open before root output.

The distinct [causal challenge preflight](diagnostic-collector-causal-design.md)
now requires a signed response to a post-ack random request/Reserved/state
commitment. A queued old heartbeat cannot supply that response. This changes no
durable state or existing validity window; authority handoff, Starting and replay
commit remain open. The costly four-fixture proof moves to an explicit manual
gate with unchanged assertions and deadlines. Old ordinary tests remain enabled;
this test-tier change supplies no production scheduling guarantee.
The [local packet](local-runs/collector-causal-20261016/README.md) keeps the
ordinary regression explicitly red: 98 passed, one unchanged old heartbeat
Authentication(Time) failure. The isolated current/MSRV proof passes, but that
does not close the validation-cost/window blocker or waive launch checks.

The subsequent [validation-cost packet](local-runs/validation-cost-20261016/README.md)
records a green 99-test ordinary repeat and a measured repeated worker-verifier
owner, with no product optimization or historical Time-cause proof. It also
retains one old IPC Host(Busy) failure despite an unchanged successful repeat.
Neither observation closes production lifecycle or server admission.

The first [worker proof reuse screen](local-runs/worker-proof-reuse-20261016/README.md)
stops on its first candidate's wrong Setup call expectation (two versus one).
The candidate is reverted, with no retry, completed five-pair estimate or
performance claim. A separate corrected preregistration and
[five-pair second screen](local-runs/worker-proof-reuse-v2-20261016/README.md)
now pass the unchanged local debug floor, preserving both constructors and all
live checks. The first candidate's 23-pass/one-Host(Busy) IPC result remains
retained without retry; new current/MSRV 24-pass IPC runs do not establish its
cause or fix. Local proof reuse does not supply lifecycle or server admission.

The [short-fence packet](local-runs/short-fence-20261016/README.md) subsequently
reproduces and fixes a completed short transaction's flock surviving in a
pre-exec fork alias. Explicit unlock is owned only after successful acquisition,
also on admission error; a failed contender or old child's late unlock cannot
release the next independently opened owner. Both Linux compiler regressions
pass all four causal controls and seven corrupt/pending ledger variants without
execution or file mutation. This is a proven local defect class, not attribution
of each old Host(Busy). The separately preregistered
[campaign-construction correction](local-runs/claim-construction-20261016/README.md)
now covers early errors, Ok(None), late marker-preserving refusal and transfer
into the original successful claim. Its six tests span 23 cases and retain the
original admission predicates and terminal release. This closes that local
construction gap, not all-writer cessation or server admission.

The distinct [start-consent byte contract](diagnostic-start-consent-plan-20261016.md)
now exists without changing Heartbeat semantics. It signs first-cell Starting and
the original request/reservation/challenge/response/policy/unit bindings. Its
inspector's supplied key and signed timestamps are not enrolled authority or
kernel observations. The consuming original-owner join, exact live checks and
durable replay/Starting acknowledgement remain before output or execution.

The separate [original consent consumer](local-runs/original-start-consent-20261016/README.md)
now freezes kernel-derived Starting and checks all body bindings against the
retained original chain. It consumes a later same-receiver packet, observes
credentials/time and preserves sticky refusal/fence ownership. This closes only
the local admission join. The default-debug deadline failure remains retained;
the successful optimized-crypto correctness lane is not installed budget proof.
Durable Starting/replay, root output/original start and all-writer cessation
remain before server admission.

The subsequent [budget experiment](local-runs/consent-publication-budget-20261016/README.md)
refuses all twelve local publication-free chains, including normal release
profiles on both compilers. Durable Starting therefore remains blocked before
its first write, and the new crash matrix, original-start/writer closure,
signed installation and host rehearsal cannot be marked complete. The
[validation frame design](diagnostic-validation-frame-design-20261016.md)
has now received operator approval. Its local candidate preserves the original
live checks and deadlines, but its complete fault gate is blocked by observed
WSL realtime reversals. Release-profile publication-free results are mixed on
Rust 1.94 and complete on 1.88; no actual Starting write or server admission
follows from them. Finish the complete mutation/reference/concurrent gate in a
declared stable-clock environment before treating this packet as closed.
The separately approved timesyncd-stop control refused its own preflight at
sample seven, before tests. The service was restored; stopping it alone did
not solve the clock problem. Further host environment changes need separate
agreement, and no server preparation or runtime acceptance was substituted.
The subsequently approved isolated Linux matrix now passes all 48 cases per
compiler using the unchanged binaries, with stable supervisor and installation
identities. The isolated frame is committed as 7b799270; WSL negatives remain.
This closes the frame fault gate, not new Starting publication/crash proof or
the production execution and host-admission prerequisites below.

The separate [Starting byte-record slice](local-runs/collector-starting-record-20261016/README.md)
is now committed as `8d1b5e2a`. Its encoder/inspector preserves prior history and
state bindings, but verifies neither signatures nor live peer/time authority.
Its passing clock-free tests do not implement the private three-file storage
phase, durable acknowledgement, SIGKILL/lost-ack proof or owner recovery. The
original default two-file reader and all runtime leaf predicates remain intact.

The [durable Starting candidate](local-runs/collector-starting-publication-20261016/README.md)
subsequently reaches actual sealed/fsynced bytes but refuses before runtime
acknowledgement in both server fault lanes. Four fresh attribution chains agree
on Original(Causal(Time)); no extension, re-anchoring or guard waiver is applied.
The full Acknowledging/AfterReturn/lost-ack and uninterrupted control gate remains
blocked. Preserve the candidate as evidence, not accepted runtime, and review
[pre-consent composition](diagnostic-pre-consent-frame-review-20261016.md) separately.
The distinct partial diagnostic passes sixteen actual SIGKILL and thirty-two
fresh inspections across both compilers, not the missing acknowledgement/control gate.

That historical rejection is now followed by an independently approved
[earlier-composition semantic proof](local-runs/preconsent-frame-20261016/README.md)
and [complete Starting publication proof](local-runs/collector-starting-framed-20261016/README.md).
Implementations `77b3ad15`/`a3f2734e` preserve all live checks and anchored
deadlines. Both compilers pass all nine publication cuts and AfterReturn,
three times: sixty SIGKILL, six controls and 132 fresh inspections total.
No retained history recovers a runtime acknowledgement. Only this first-cell
intent/process-loss prerequisite is closed; external ACK transport, device
failure/reboot, production enrollment and the five execution items below are not.
The exact stable supervisor and installed binary/receipt stayed unchanged.

1. Connect independently selected root-owned trust inputs to kernel admission
   and original authority/fence ownership, including receiver time/PID namespace
   identity, verified controller provenance, replay, epoch revocation and trusted
   host enrollment.
2. Join the [separately signed collector requirements](diagnostic-collector-intent-design.md)
   to actual original start. Private output preparation and
   [retained original bundle material](local-runs/original-collector-start-material-20261017/README.md)
   now have complete temporary fixture gates; installed root ancestry remains
   unproved. Preserve legacy signatures; fulfill executable, environment, pipe FD
   transport, collector, process and manager invocation/cgroup requirements.
3. Implement autonomous watchdog and exact original-cgroup revocation/emptiness.
   Missing EOF, uncertain stop, collector loss and leaked writers retain failure;
   a stopped leader or replacement unit name grants no release.
4. Join authenticated successful output/outcome and writer cessation to owned
   completion. Implement cells two through four without reopening/rebinding the
   immutable first intent, then SIGKILL every new publication boundary and lost
   acknowledgement. Unknown attempts cannot retry or become successful.
5. Pass the complete integrated local lifecycle/fault gate, full milestone
   verification and signed installation/preflight preparation for exact tooling.

The kernel-input package does not close these items. No green local receipt may
stand in for an enrolled start, whole-writer revocation or later-cell protocol.

## Actual enrollment inventory and pending selection

The operator-approved October 15 read-only inventory now retains an
[exact proposal and observations](diagnostic-enrollment-review-20261015.md).
Worker UID/GID are 986/986; local runner UID is 1001 and its explicit client
group is 987. No worker/controller/OIDC trust file or authority/intent root was
present. The actual supervisor remains on the old installed source and has a
different mount namespace from SSH/PID 1. Do not use SSH namespace facts to
finalize a receiver policy or weaken service confinement to make them match.

The dedicated controller environment/workflow are absent in GitHub; the old
reference environment is unprotected. Public acquisition reproduces four RSA
keys without selecting or installing them. The new packet proposes explicit
new-authority initialization and a short-lived v2 roster, but current authority,
actual receiver/workflow/runtime pins, signed policy/receipt and protected job
enrollment remain unresolved. No service, epoch, configuration or workflow was
changed. Read-only approval is not enrollment or execution approval.

## Host gates after local completion

P4 needs actual signed bundle/receipt, ownership, UID/GID, current kernel options,
systemd/cgroup and a quiescent host check. Do not chown live campaign layouts or
stop another campaign. Any approved maintenance stop targets only the exact
stable supervisor, with restoration and health verification.

P5 then needs a separately reviewed bounded nonproduct lifecycle/fault rehearsal
and controller overhead observation on that exact host. Reboot needs separate
maintenance approval. Numerical pilot, product candidates, six-hour/24-hour
qualification and release admission remain separate gates. Frozen numerical
inputs and the inherited plan-digest mismatch are not changed here.
