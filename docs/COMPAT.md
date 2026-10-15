# HydraCache Compatibility Register

The local audit record `diagnostic-signer-issuer-selection-074-v1` records only
operator selection of a separate signer-build public key. It is not the earlier
generation selfcheck record, a signed receipt or a production trust loader.
Code pins select the issuer independently of incoming records. Artifact/runtime/
job/signing/server fields remain false; no new wire reader or negotiation exists.
Old receipt readers and key roles remain unchanged. Rotation requires a new
reviewed code pin/selection, not changing the old generation event. See the
[selected-issuer design](testing/performance/0.74/diagnostic-signer-pin-design.md).

The local public inventory `diagnostic-signer-build-issuer-selection-074-v1`
has a separate purpose, public key/fingerprint and fixed selfcheck domain. It
contains no private material and keeps pin approval, production enrollment,
receipt signing, workflow installation and admission false. Unknown/duplicate
fields, noncanonical encoding, role confusion or changed flags refuse. It is
not a signed build receipt or a production trust configuration; old worker and
observer readers do not consume it. Rotation requires a new destination and
explicit public selection, not overwriting an old key. See the
[custody design](testing/performance/0.74/diagnostic-signer-custody-design.md).

The private Linux libtest report `diagnostic-signer-original-thread-exit-074-v1`
uses distinct 32-byte magic/stage/mode/original-TID/zero-reserved fields. A fixed
prefix transfers one retained creator task FD, followed by five original child
and root FDs. The unchanged receiver checks kernel peer credentials and at most
five FDs per packet; at most ten packets are received, including closed progress
or terminal records. Unknown schema, fields, lengths, FD set or peer refuse.
Old binaries have no consumer. No normal CLI, persisted/signed schema, public API,
dependency, pin or qualification changes; removing the private fixture disables
it. See the [thread-exit boundary](testing/performance/0.74/diagnostic-signer-thread-exit-design.md).

The private Linux libtest report `diagnostic-signer-original-creator-loss-074-v1`
has distinct 32-byte magic/cut/control/healthy/TID/reserved fields. After one
descriptor-free setup byte, a fixed prefix transfers gate/task FDs, then five
original child/root FDs. The unchanged credential-checked receiver accepts at
most five FDs per packet and 128 ancillary bytes; at most ten packets total.
Closed progress/terminal records and one-byte commands do not grant authority.
Unknown shape/peer/ancestry/sequence refuses. Old binaries have no consumer;
no persisted/signed schema, public API, normal CLI, pin, dependency or frozen
qualification changes. Removing the test profile disables it. See the
[creator-loss boundary](testing/performance/0.74/diagnostic-signer-creator-loss-design.md).

The private Linux libtest report
`diagnostic-signer-original-parent-constructor-loss-074-v1` uses distinct magic,
32 bytes with cut/fault/original-TID/zero reserved fields, exactly five original
FDs and kernel credentials, after one descriptor-free progress byte. Its private
child gate admits one fixed cut token and one release byte; EOF exposes actual
parent loss. Unknown fields, length, descriptor set, peer or ancestry refuse.
Older binaries have no consumer. It is neither persisted nor connected to a
production receiver; no durable/signed schema, public API, dependency, pin or
qualification changes. Removing the private fixture disables this test profile.
See the [constructor boundary](testing/performance/0.74/diagnostic-signer-constructor-loss-design.md).

The private Linux libtest report `diagnostic-signer-tracer-loss-report-074-v1`
uses 32 bytes, fixed stage/send-count/TID/reserved fields and exactly five original
SCM_RIGHTS descriptors with kernel credentials, plus a closed one-byte response.
A setup prefix has exactly stage+2 credential-checked one-byte ordinals0..6 with
no FDs, at most eight received packets total. Those hints never grant authority.
Unknown fields, length, descriptor set, peer, ancestry or sequence refuse before
fault admission. Older binaries have no consumer; it is not persisted, negotiated
or accepted by the production ancillary receiver. No durable/signed schema,
normal-library API, dependency, pin, production policy or qualification input
changes. Removing the private test module disables it. See the
[tracer-loss boundary](testing/performance/0.74/diagnostic-signer-tracer-loss-design.md).

The private Linux x86_64 libtest handoff protocol
`diagnostic-signer-original-synthetic-handoff-074-v1` uses three exact 16-byte
READY/DUMMY/ACK records on an original-owner-created unnamed channel. It accepts
no negotiation, future version, caller FD/PID or variable credential. Unknown
bytes/length/sequence refuse and reap; old binaries have no protocol consumer.
It is test wire only, not persisted or connected to any CLI/server/issuer, with
no migration or production rollback artifact. Removing the test profile disables
it; normal-library behavior and frozen inputs are unchanged. See the
[handoff boundary](testing/performance/0.74/diagnostic-signer-handoff-design.md).

The post-install namespace policy is a private child of the existing Linux x86_64
rooted libtest module. Root materialization and original-owner observation plumbing
are reused with a closed internal selector; old helper and 34-instruction policy
remain exact. New helper/64-instruction bytes and retained proc sources are local
fixture mechanics, not a normal-library API, public profile or issuer authority.
No durable/wire/schema/dependency/pin/qualification input changes. See the
[namespace boundary](testing/performance/0.74/diagnostic-signer-namespace-design.md).

The selected-root original-exec composition is a private child of the existing
Linux x86_64 libtest root module. It owns its own root/child and observes stopped
kernel file mappings and exact unchanged filter bytes. No caller adoption, public
API, normal-library consumer, durable/wire schema, dependency, production profile,
reviewed pin or qualification input changes. Post-install root/namespace policy,
custody, handoff and server admission remain closed. See the
[rooted exec boundary](testing/performance/0.74/diagnostic-signer-rooted-exec-design.md).

The child root probe exists only in Linux x86_64 libtest builds. Its temporary
tree and private scalar report are not a public API or durable/wire artifact.
It changes only its owned child's user namespace/root/capabilities with unchanged
nonzero UID/GID; no host account/mount/service configuration changes. Existing
GNU helper launch/filter and normal-library consumers remain unchanged. No signer
exec, handoff, custody or server admission follows. See the
[child root boundary](testing/performance/0.74/diagnostic-signer-root-design.md).

The runtime inventory/parser exist only in Linux x86_64 libtest builds. They
retain original readonly inputs and bounded declared ELF metadata without a
loader/launch consumer or public acceptance/FD API. No dependency, normal-library
surface, CLI, schema, reviewed pin, server or qualification input changes.
Only the joined helper compiler gains test-module visibility. No custody,
runtime closure, key, handoff or worker start is admitted. See the
[runtime boundary](testing/performance/0.74/diagnostic-signer-runtime-design.md).

The joined signer bootstrap fixture and embedded helper exist only in Linux
x86_64 library test builds. Shared policy/info/file helpers gain pub(super)
test-module visibility, not a normal-library/public ABI. No durable/wire/signed
schema, configuration, migration, dependency or production kernel/build profile
changes. Rust 1.94/GNU compiler and exact local copy utility are fixture setup
requirements and explicitly refuse absence; they do not select production
runtime custody. The bounded post-exec input proof has its own narrower trust
scope rather than changing the standalone fork-only contract. See the
[joined boundary](testing/performance/0.74/diagnostic-signer-joined-design.md).

The owned seccomp installation fixture is a private Linux x86_64 test-build
module, with no public launch/address/filter API, signed/durable/wire schema or
migration. It requires kernel syscall-info, pidfd and close_range support and
refuses unavailable or unknown forms; supported production kernel policy is
unchanged. The 34-instruction policy and non-dumpable hardening are local
fixture mechanics, not a production signer runtime profile. Existing first-exec
source/contract, producer/process semantics, dependencies and reviewed pins
remain unchanged. See the [seccomp boundary](testing/performance/0.74/diagnostic-signer-seccomp-design.md).

The owned signer first-exec prototype is a private Linux x86_64 test-build
module only. Normal libraries/CLIs have no new fixture launch symbols or API, and no durable,
wire, signed-receipt, configuration or migration artifact changes. The fixture
requires close_range for immediate inherited-FD closure and explicitly refuses
when unavailable; this does not change supported production kernel policy.
Original process scope, producer semantics and reviewed pins remain unchanged.
See the [first-exec boundary](testing/performance/0.74/diagnostic-signer-first-exec-design.md).

The Linux signer executable observation is a runtime-only borrowed guard, with
no serialized/wire/durable schema or migration. It joins original asserted
bundle files to an existing retained ProcessRead through a crate-private
executable-only observer. Old bundle/receipt formats, signed domains, process
scope and IO predicates remain unchanged. Unknown/mismatched objects and
observed loss refuse the wrapper/bundle; same-inode re-exec remains an explicit
observational limit, not original-start authority. Older tools have no such
method and require no artifact conversion. See the
[executable boundary](testing/performance/0.74/diagnostic-signer-executable-design.md).

The Linux local signer-bundle input layout v1 has six fixed public fixture names:
`diagnostic_controller`, `signer-build-receipt-v1.json`, `Cargo.lock.root`,
`build-log.jsonl`, `runtime-manifest.json` and `bootstrap-context.json`. Its new
opt-in asserted reader refuses extra/missing entries, unsafe owner/type/mode,
name/inode/byte drift and the old fixed observer install root. It consumes the
unchanged `diagnostic-signer-build-074-v1` receipt/domain and streams bounded
hashes through a private shared verifier. The returned runtime-only guard has
no serialized schema, installer, production path, authority export or migration.
Older tools have no signer-bundle reader; observer bundles remain separate and
cannot be automatically upgraded or treated as signer custody. See the
[original file boundary](testing/performance/0.74/diagnostic-signer-bundle-design.md).

The separate local `diagnostic-signer-build-074-v1` receipt reader accepts at most
8192 canonical LF bytes with strict fields and an Ed25519 signature under the
new `hydracache-diagnostic-signer-build-074-v1` domain. Expected source/profile
and all supplied binary/lock/log/runtime/context hashes and lengths must match
an explicitly asserted selection. Weak and known controller/observer-builder/
worker-issuer keys refuse this new role. Old observer readers reject this shape;
the new reader rejects old receipts and unknown future schemas without fallback.
No migration, production issuer/pin, receipt producer, production file reader or bootstrap/
execution consumer is introduced. Its opaque result exposes inventory only, not
the existing verified observer build. See the
[signer model](testing/performance/0.74/diagnostic-signer-build-design.md).

The additional opt-in `diagnostic-oidc-trust-input-074-v2` reader accepts one
through eight explicitly configured RSA/RS256 keys, strictly sorted by kid with
duplicate modulus/exponent refusal, in at most 8192 canonical LF bytes. It uses
the existing root-only OIDC path and required principal/snapshot fields, with
`keys` instead of v1 `key`. Current readers preserve v1's 2048-byte document and
single-key semantics; prior v1 readers reject v2 and unknown future versions.
Only the original OIDC source observation uses the larger read budget; worker
and controller configurations remain bounded to 2048 bytes. Mixed shapes,
unknown keys, expired selection, unsafe metadata or source drift refuse without
network refresh, key fallback or downgrade. No migration/writer installs trust;
rotation and old-binary rollback require explicit reviewed configuration and new
original enrollment. Signed domains, durable records, frozen qualification and
server routes are unchanged. See the
[roster design](testing/performance/0.74/diagnostic-oidc-roster-design.md).

The opt-in `diagnostic-oidc-public-acquisition-074-v1` tool report is unsigned
observation data, not a runtime trust configuration. It retains bounded raw public
discovery/JWKS bodies as base64, source/CA-roster/peer certificate hashes, TLS
metadata and job receive timestamps. The owned-child reader refuses unknown
fields/schema, bad bounds/framing/duplicates, inconsistent inspector data or any
selected-key/admission flag. No runtime config reader consumes this report;
older tools have no report reader or migration. Existing offline report,
diagnostic domains and frozen qualification are unchanged. See the
[acquisition boundary](testing/performance/0.74/diagnostic-oidc-acquisition-design.md).

The separate `diagnostic_controller` job CLI emits the existing diagnostic signed
request/domain unchanged; it is not an installed supervisor command or a migration.
Its portable preparation requires canonical LF requests and the fixed reviewed
builder receipt. Controller selection is fixed; no public key/time override exists.
The opt-in retained signing API consumes the same opaque preparation and checks
its earlier audience without rereading public input files. This is runtime-only
job-body consistency: no signed/wire/durable schema, CLI command, migration,
token trust or receiver route changes. Older tools keep their existing separate
file commands. See the [retained boundary](testing/performance/0.74/diagnostic-retained-producer-design.md).
The offline `diagnostic-oidc-source-inspection-074-v1` report contains only bounded
public-source metadata/digests and explicit false acquisition/enrollment/admission
flags. No runtime reader consumes that report as trust. Unknown source key fields,
duplicates, wrong fixed issuer/endpoint or algorithms refuse; discovery extension
fields are ignored and confer no authority. Older tools have no inspection reader.
See the [producer boundary](testing/performance/0.74/diagnostic-controller-producer-design.md).

Separate opt-in `diagnostic-oidc-trust-input-074-v1` configuration has a fixed
root-only original reader at `/etc/hydracache-perf/oidc-trust-074.json`, bounded
to 2048 canonical LF bytes. It selects one explicit RSA/RS256 signing key, fixed
GitHub issuer/repository/dedicated job profile, explicit workflow SHA/subject and
snapshot validity window. Missing/null/duplicate/unknown/future/noncanonical or
unsafe input, source drift and expired selection refuse without repair/refresh.
Older binaries have no reader; no writer or migration installs configuration.
The additional compact-JWS profile verifies original signing bytes and strict
bounded claims under configured trust, binding audience and origin to the same
original diagnostic request. Additional bounded scalar claims confer no authority;
unknown JOSE headers and reusable-job claims refuse in this initial profile.
Actual independently authenticated key acquisition, protected-job enrollment,
durable replay, execution and server routing remain separate. Existing signed
domains, request/ledger/receipt schemas and frozen qualification are unchanged.
See the [OIDC design](testing/performance/0.74/diagnostic-oidc-input-design.md).

Separate operator configuration `diagnostic-controller-trust-input-074-v1`
has a new opt-in reader at `/etc/hydracache-perf/controller-trust-074.json`.
It accepts only schema/repository/controller key/actor IDs/client UIDs/client GID
in canonical LF JSON, at most 2048 bytes and 32 strictly sorted unique principals
per list. Root-owned original no-follow files retain metadata and exact bytes;
missing/null/duplicate/unknown/future/noncanonical, wrong pinned key/repository,
unsafe principals or drift refuse without repair, reload or fallback. No writer,
migration or actual configuration installation is added; older binaries have no
reader for this new configuration. Existing signed request/domain and durable
formats remain unchanged. Its borrowed request guard verifies signature/peer/
freshness only, not GitHub provenance, nonce consumption or start. See the
[design](testing/performance/0.74/diagnostic-controller-trust-input-design.md).

Worker policy/kernel binding adds an opt-in read-only runtime guard, not a new
wire, configuration or durable schema. It compares verified policy context with
retained machine/boot files, typed user/mount namespaces and the original kernel
packet, without a caller host argument. Existing assertion APIs remain unchanged;
controller/replay/fence and initial-host/source-opener enrollment remain separate.
See the [binding design](testing/performance/0.74/diagnostic-worker-kernel-binding-design.md).

The fixed worker trust input introduces a separate root-owned configuration
schema and read-only runtime guard; see its
[contract](testing/performance/0.74/diagnostic-worker-trust-input-design.md).
No actual trust file is created here. Original config/journal/policy ownership
is retained, but supplied host context is not kernel enrollment, runtime refusal
is not durable revocation and no listener/start/completion route is enabled.

Reviewed diagnostic trust adds opt-in runtime-only verification entry points,
using the existing October 9 public controller/builder pins and raw policy digest.
No signed domain, JSON schema, durable record, installed configuration or existing
generic API changes. The new kernel entry latches refusal on key/repository drift;
it cannot enroll worker trust, current epoch/revocation, host authority or a start.
See the [reviewed-trust scope](testing/performance/0.74/diagnostic-reviewed-trust-design.md).

Diagnostic kernel admission adds an opt-in AF_UNIX SOCK_SEQPACKET receiver with
SO_PASSCRED/SO_PASSPIDFD. It requires one per-message credentials record and one
kernel pidfd, denies received SCM_RIGHTS/unknown/truncated control data, and keeps
the diagnostic 16 KiB payload ceiling. Unsupported kernel options refuse rather
than falling back to connection-time peer credentials. The existing campaign
transport, portable diagnostic signed JSON/domain and durable formats are
unchanged. Kernel sender/clock inputs do not enroll an issuer or authorize start;
see the [kernel contract](testing/performance/0.74/diagnostic-kernel-admission-design.md).

The operator-approved diagnostic collector amendment introduces a separate local
fixture namespace `collector-attempt-v1/{collector-stdout.bin,collector-stderr.bin}`.
Only the original nonroot fixture writer can return a nonserialized drain result;
reopening the files cannot recreate its acknowledgement or grant completion.
Old signed unit projections and regular-file FD1/FD2 bindings remain unchanged.
Future root enrollment needs a distinct signed pipe-to-collector projection;
there is no automatic migration, fixture promotion or installed writer route.
See the [collector contract](testing/performance/0.74/diagnostic-collector-design.md).

Original pidfd termination adds a Linux read-only runtime API only. The result
contains retained generation and strict exit/hangup readiness, never an exit
code, authenticated outcome or cleanup authority. Existing live admission and
all durable/wire formats are unchanged. See the
[completion boundary](testing/performance/0.74/diagnostic-completion-boundary-local-design.md).

Fresh-reader context replay extends only the private test harness. Seven
single-component mutations exercise the existing authority Binding refusal,
while partial journal tails continue to fail Invalid before binding. No
production schema, namespace, protocol, migration or reader API changes.
Retained context still denies new reservation; no recovered view becomes an
acknowledgement, retry or cleanup capability. Boot/namespace changes are modeled,
not an installed-host reboot. See the
[context design](testing/performance/0.74/diagnostic-context-replay-local-design.md).

Owned quarantine process-loss testing adds only cfg(test) instrumentation at
the four existing authority append checkpoints. The production refusal API,
record/commit encoding, filenames, ownership rules, sync order and reader
compatibility are unchanged. Fresh fixture inspection observes partial journal
refusal or a complete authority refusal without recovering acknowledgement,
retrying an append or admitting another reservation. No migration or cleanup is
added. See the
[process-loss design](testing/performance/0.74/diagnostic-quarantine-crash-local-design.md).

Owned Starting quarantine introduces only a consuming runtime type. It reuses
the existing bounded authority refusal record/commit schema and Policy reason;
no filename, encoding, migration or reader compatibility changes. Acknowledged
means original authority refusal only. Uncertain retains the persistence error,
and PreviouslyRefused performs no new I/O or claim of durable acknowledgement.
The original attempt objects remain retained and fresh admission still refuses.
No terminal receipt, cleanup, replay or healthy capability conversion is added.
See the [quarantine design](testing/performance/0.74/diagnostic-starting-quarantine-local-design.md).

Owned first Starting departure adds only `diagnostic-starting-intent-074.json`
inside the original fixed leaf. Canonical LF-terminated
`diagnostic-starting-intent-074-v1` is bounded by 4096 bytes and includes the
full revision-two Starting state, original encoded-state hash, exact cell-one
record hash and original signed-envelope hash. These links add no controller
signature. The initial schema-one active state remains immutable; the new
record is not a live executing state. Exclusive 0600 creation, owned descriptor
and prefix checks, sealing to 0400 and file/directory sync precede runtime
acknowledgement. Namespace permission requires the actual owned document.
Readers accept only this version; unknown versions/fields/noncanonical bytes
are not adopted. Old readers/fresh admission reject retained context. No
migration, downgrade deletion, retry, restart or execution conversion is added.
Partial or unacknowledged records remain retained. See the
[Starting intent design](testing/performance/0.74/diagnostic-starting-intent-local-design.md).

Signed diagnostic history inspection adds a data-only supplied-byte API using
the existing retained-intent schema, canonical LF encoding, record budget and
commit digest. It changes no publisher, reader, filename, state or migration.
Cells one through four are checked against the exact initial-attempt anchor,
original signature pins and previous raw hashes. Revision gaps are necessary
bounds, not proof of omitted transitions or completion. Inspection returns no
storage acknowledgement, append or execution capability. See the
[history design](testing/performance/0.74/diagnostic-signed-history-local-design.md).

Owned early-attempt transfer now connects the original runtime acknowledgement
to initial schema-1 Reserved publication and existing cell-one retention.
It moves the original descriptor/stamp/bytes and common fence without unlock
or recovered-file adoption. Namespace checks permit the attempt name only while
that original document is owned; every state/cell-one boundary revalidates it.
State creation remains exclusive 0600 with original sync barriers; attempt stays
0400 and record/commit bytes retain their existing schema and limits.
No filename, durable schema, migration, recovered acknowledgement or execution
permission is added. Old empty-fence/campaign readers refuse retained context;
legacy publishers do not adopt an attempt. Partial/uncertain context is retained
and never repaired or deleted. See
[transfer design](testing/performance/0.74/diagnostic-attempt-publication-local-design.md).

Early diagnostic attempt retention adds only `diagnostic-attempt-074.json` to
the bound fixed intent leaf. It reuses canonical LF-terminated
`diagnostic-retained-intent-074-v1`, its 16384-byte limit and initial Reserved
record fields; it is not cell-one consumption. The original exclusively created
file is sealed 0400 and acknowledged only after file/directory sync and final
original authority/object checks. No new schema, commit marker or persisted
acknowledgement is introduced. Old empty-fence and fixed campaign-admission
readers refuse any retained leaf context. Partial, uncertain and complete
attempts block fresh reservation and cannot be adopted, repaired or replayed.
No downgrade or deletion with unresolved context; a reviewed owned progression
protocol is still required. Existing state/cell publishers and readers are
unchanged. See [early attempt design](testing/performance/0.74/diagnostic-early-attempt-local-design.md).

The local diagnostic schema-1 publisher now checks adjacent state transitions
under its existing transaction fence before writing. Revision must advance by
exactly one; lease/boot/build/origin, original active-cell start/inode and stop
reason cannot drift. Only Running may complete one cell; Terminal has no next
publication. Existing snapshot bytes, readers, filenames and pending/terminal
recovery are unchanged; no migration or new wire version is introduced. Invalid
adjacent candidates refuse before mutation. This strengthens the local model,
not fixed-root lifecycle or production execution. See
[transition contract](testing/performance/0.74/diagnostic-transition-local-design.md).

Initial fixed intent consumption now consumes the runtime-acknowledged Reserved
publisher under its uninterrupted original lock. It creates only cell-one JSON
and digest-commit files, reusing `diagnostic-retained-intent-074-v1`, canonical
newline bytes and bounded record/marker sizes. New owned files are sealed 0400;
state remains unchanged. No schema migration, fixture promotion, recovered-history
adoption, server route or execution permission is added. Retained partial files
block fresh reservation and cannot become a runtime acknowledgement after restart.
See [initial consumption design](testing/performance/0.74/diagnostic-initial-consumption-local-design.md).

The fixed-only initial Reserved publisher now creates `active-diagnostic.json`
under the retained original common fence using descriptor-relative O_EXCL
creation. It reuses the existing canonical schema-one envelope, private 0600
mode and bounded state reader; no schema or migration changes. Existing records
are never overwritten or adopted for publication. Empty, partial and unacknowledged
records are retained as admission-blocking context; schema-one readers refuse
torn/noncanonical state and unknown versions. A complete existing record can
be inspected read-only but is not a new publication acknowledgement.

Success follows file and directory sync plus exact original-inode/bytes and
signed-intent checks. Failure retains the original lock until drop and latches
bound storage/authority, preserving refusal uncertainty without cleanup.
No fixture-authority conversion, installer, ownership migration, authenticated
peer/clock, service route or worker start is added. See
[design](testing/performance/0.74/diagnostic-reservation-publication-local-design.md).

Original signed intent now also has distinct fixed/fixture read-only compositions
with the original empty reservation fence. They retain the caller's existing
lock, match full asserted state before I/O and cross-latch both original chains
on bound failure. Policy refusal uses only the independently exact-safe original
authority journal; intent failures do not publish twice. No fixed intent state,
schema, installer, ownership, authentication or start route changes. Empty
observations are not durable reservation. See
[design](testing/performance/0.74/diagnostic-empty-fenced-intent-local-design.md).

This file tracks durable and wire-visible artifacts whose versions matter during
rolling upgrades. Runtime-only Rust types are intentionally out of scope unless
they are persisted or transmitted across processes.

Initial diagnostic reservation now has a read-only original empty-fence guard.
It borrows existing fixed intent storage and reuses the same trusted campaign
lock engine before any Reserved file exists. Any fixed-leaf or legacy/parent
context refuses; initial Busy does not revoke healthy storage, while bound
failure is sticky. No state, lock, permissions, schema, installer or execution
route is created or changed. The empty observation ends with the guard and is
not a durable reservation or write capability. See
[design](testing/performance/0.74/diagnostic-empty-reservation-fence-local-design.md).

The original authority/signed-intent reader now has distinct fixed and fixture
read-only compositions with the original fenced Reserved-state reader. Full
state snapshots must match before composition I/O. Bound failures latch both
chains; fence failures publish Policy refusal only through the independently
exact-safe original authority journal, retaining typed uncertainty on failure.
This reuses existing state/intent/refusal schemas and changes no installer,
ownership, publisher, peer/clock authority or start API. Healthy composition
drop leaves the caller's original fence held. See
[design](testing/performance/0.74/diagnostic-fenced-intent-local-design.md).

Linux campaign admission for the canonical production campaigns root now
checks existing root-owned fixed intent storage under the original common lock.
Every fixed-leaf entry, including unknown, pending, terminal or symlink context,
blocks admission and recovery; missing/unsafe storage explicitly refuses.
Original storage/campaign/lock objects survive the pre/post marker checks, and
a late refusal retains any created marker. Unlocked absence is not admission.
This strengthens the 0.74 supervisor read-side contract without a new persisted
format, ownership migration, publisher or launch API. Older supervisor binaries
do not know this namespace; a future publisher requires coordinated reviewed
deployment and trusted ownership enrollment first. Empty safe storage preserves
the legacy route but does not turn its worker-owned lock into root authority.
See [design](testing/performance/0.74/diagnostic-fixed-admission-local-design.md).

The original campaign-fence binding is runtime-only. It borrows the existing
fixed Reserved-state reader and locks only the existing campaigns
`.host-execution.lock`, with original-object checks and conservative legacy
context refusal. It requires a private trusted-owner directory and lock; the
deployed worker-owned campaigns layout is not silently enrolled or repaired.
That initial binding changed no durable format, installer, campaign admission,
publisher or start route. Its nonproduction legacy fixture still ignores fixed
state after guard drop; the production read-side check is registered above.
The binding alone does not establish durable campaign exclusion.
See [design](testing/performance/0.74/diagnostic-original-fence-local-design.md).

The fixed Reserved state inspector reuses the exact schema-1 diagnostic state
envelope and canonical bytes. It reads only the prospective existing
`active-diagnostic.json` name under root-owned `diagnostic-intents-074` storage,
requiring root:root 0600 and the original full snapshot. It does not migrate,
publish or copy legacy worker-owned campaign state into that namespace. Pending
or unhandled same-leaf context refuses. Bound drift latches the original storage
guard; no host fence, authority/intent join, installer/CLI or start is added.
See [design](testing/performance/0.74/diagnostic-reserved-state-local-design.md).

The original-authority signed-intent composition is runtime-only and preserves
all registered request, state, signature and journal formats. It exclusively
borrows the original authority guard; fixed and fixture types remain distinct.
Invalid unbound intent does not revoke a healthy worker. After binding, drift
refuses the original chain and attempts only an exact-bound safe Policy refusal
record in the existing authority format. Publication uncertainty retains the
projection error, never allowing a retry or execution. No fixed intent publisher,
live fence/state proof, migration or start route is introduced.
See [design](testing/performance/0.74/diagnostic-authority-intent-local-design.md).

The Linux read-only `diagnostic-storage-preflight` CLI emits the bounded JSON
schema `diagnostic-storage-preflight-074-v1`. Success means storage ancestry only;
journal verification, authority binding, enrollment, execution, qualification and
ship fields remain false. There is no input schema, path selector, key enrollment
or migration. Missing/unsafe paths refuse without journal initialization.
The associated ownership amendment accepts the original GID of the root-owned,
nonwritable common performance parent; all other ancestors and private leaves
remain root:root, with leaves exactly 0700. Original GID changes still refuse.
The authority journal record schema and epoch/pin rules are unchanged.
See [design](testing/performance/0.74/diagnostic-storage-preflight-design.md).

The opt-in Linux temporary-fixture intent journal registers
`diagnostic-retained-intent-074-v1`. Four fixed `diagnostic-intent-cell-N.json`
and `.commit` pairs (N=1..4) are append-only canonical sorted-key JSON plus LF,
at most 16384 bytes per record and exactly 65 bytes per lowercase digest marker.
A record contains full Reserved state, exact signed intent UTF-8 bytes, asserted
peer UID/groups and previous-record digest. Readers reject unknown/duplicate
fields, future schemas, gaps, partial pairs, unsafe metadata and hash/signature
or original-binding drift. There is no overwrite, cleanup, migration or replay;
a complete late-unacknowledged pair remains consumed. Signature-window grammar
for historical records is checked at their signed issue instant, not renewed.
This deduplication scope is one retained fixture journal, not multiple roots or
a production once-only executor. The existing request/state formats are unchanged.
See [design](testing/performance/0.74/diagnostic-intent-journal-local-design.md).

The opt-in Linux-only 0.74 signed diagnostic intent projection registers
`diagnostic-intent-projection-074-v1` and
`diagnostic-signed-intent-projection-074-v1`. These are canonical sorted-key JSON
plus LF, bounded at 4096/8192 bytes. The controller signature covers
`hydracache-diagnostic-intent-projection-074-v1`, NUL and the canonical body.
Unknown/duplicate fields, future schemas and noncanonical envelopes refuse.
The body binds request/nonce, controller IDs/window, full Reserved state digest,
original worker policy digest/epoch, boot and a fixed typed ordered unit digest.
Unit projection is `{unit_name, properties}` with ordered `[name, {type, value}]`
pairs and tags text/boolean/unsigned/bytes/strings/commands; command objects
retain path/argv/ignore_failure. Both projections hash canonical JSON plus LF.
This byte contract does not change the existing diagnostic request protocol,
persist an intent, prevent replay, authenticate live state or authorize execution.
See [design](testing/performance/0.74/diagnostic-intent-projection-local-design.md).

The opt-in 0.74 local diagnostic authority fixture journal registers
`diagnostic-authority-record-074-v1`: canonical JSON plus LF, at most 2048 bytes,
128 contiguous generations, with a 65-byte SHA-256-plus-LF marker for each record.
The predecessor digest chains exact bytes. Refusal preserves the exact original
issuer/policy/epoch/context; later activation requires a refused predecessor,
a strictly higher epoch, the same issuer/machine and a new policy digest.
Unknown schemas, extra fields, gaps, changed refusal bindings, incomplete pairs
and exhausted bounds fail closed. No automatic migration, tail repair, eviction
or key rotation exists in v1. Independent current pins remain mandatory.
The first backend is a distinct nonroot Linux temporary fixture. A subsequent
opt-in fixed-path backend uses the same v1 bytes and append-only engine, retaining
root-owned nonwritable ancestry (with the common-parent GID amendment above)
and an exact root:root 0700 leaf. Its constructors accept no
path/owner/descriptor selector and cannot promote a fixture. Neither backend is
historical signature attestation, whole-host snapshot rollback detection, signed
worker lifecycle or a start/admission capability. The fixed backend is implemented
locally, but real root-path enrollment and issuer selection remain unproved.
The subsequent runtime-only signed-task bridge exclusively borrows an original
guard and matching typed journal. A safe exact-bound journal receives an
append-only refusal after signed failure; unsafe storage is refused without
repair. The v1 record bytes and epoch rules are unchanged. Unacknowledged writes
do not authorize restart; crash-before-publication and lifecycle intent remain
separate, unimplemented boundaries.

The subsequent fixture-only recovery refusal composition reuses the existing
schema-1 diagnostic state and IPC ledger decoders and v1 Policy refusal records.
Pending publication, an unacknowledged IPC intent, interrupted execution stages,
or unsafe lifecycle documents latch the original signed fixture chain; only a
safe exact-bound authority journal receives a refusal. Quiet Reserved/Terminal
observations are data, not start/adoption/cleanup permission. It changes no
durable bytes, production dispatch or epoch transition. Authenticated production
intent, fixed-root enrollment and crash-before-any-intent remain open boundaries.

The separate 0.74 asserted task reader is opt-in Linux runtime-only. It retains
at most 128 original tasks and their generation/status/user/mount objects under
bounded membership scans, generation brackets and sticky refusal. It adds no
wire/persisted schema, helper/IPC mode or launch operation; existing leader and
signed APIs are unchanged. Numeric assertions and sequential stable inventories
are not signed enrollment, atomic/continuous confinement, trusted host authority
or durable revocation. The additional opt-in signed-task wrapper exclusively
borrows the original typed signed mount reader, privately derives task numeric
assertions from its original policy and checks both observed namespace pins against
that policy. Fixed/fixture types
stay distinct. An error refuses the original chain even after wrapper drop;
healthy drop preserves it. This is runtime-only, not a durable/wire format change.
Controlled fixed-host enrollment and production lifecycle remain prerequisites.

The 0.74 same-user-namespace guard is opt-in Linux runtime-only. It compares
retained nsfs user namespace objects for an original process and the reading
thread, with generation brackets and sticky refusal. It adds no helper mode,
IPC or serialized artifact, accepts no replacement PID/namespace/FD and cannot
join namespaces. It does not authenticate an initial host namespace, bind an
already-open credential document's view or alter the numeric-only credential API.

The separate 0.74 namespace-checked credential reader is opt-in Linux runtime-only.
It owns original namespace and credential guards, opens status inside a
namespace/open/namespace/credentials/namespace bracket, and later brackets
credential revalidation with namespace checks. Every failure refuses both guards.
It cannot enroll an already-open reader or export/refresh either guard. No wire,
helper or IPC mode changes; the numeric-only API and account binder are unchanged.
Sequential opening-context checks are not kernel-attested file credentials,
atomic/continuous namespace proof, trusted host/NSS enrollment or admission.

The separate 0.74 namespace-checked worker binder borrows the original fixed
account and namespace-checked credential guards, reusing the existing numeric
mapping rule. Account observations bracket the unchanged guarded reader; every
error refuses account, namespace and credentials through constructor failure
and binding drop. A successful drop leaves healthy inputs usable. It adds no
serialized artifact or helper/IPC mode and accepts no numeric-only reader
conversion or replacement inputs. The old numeric API and binder remain intact.
Mapping agreement does not attest NSS provider/configuration or helper namespace,
trusted host/initial namespace, all threads, authenticated start or admission.

The 0.74 asserted worker binding is Linux runtime-only. It borrows the original
fixed-account and retained-process credential guards, compares the existing
numeric policy and brackets credential observation with account checks. It adds
no serialized artifact or helper/IPC mode, and exports no PID/policy replacement
or descriptors. Refusal propagates to both original inputs, including constructor
failure and after wrapper drop. Mapping consistency is not trusted provider or
namespace enrollment, authenticated original start, durable refusal or admission.

The 0.74 original-process credential guard is Linux runtime-only. It reads the
retained original status under bounded generation brackets and sticky refusal,
checking explicit numeric assertions, not an enrolled account name. No raw FD,
wire/persisted schema, helper mode, credential mutation or launch route is added.
It proves neither all-thread credentials nor user namespace enrollment,
continuous privilege confinement, authenticated original start or admission.

The 0.74 fenced output preparer is Linux runtime-only and fixture-only. It adds
no wire operation or persistent receipt schema. It creates fixed empty streams
under a temporary non-root owner and the existing cooperative host lock, without
changing diagnostic state. Existing or partial cells refuse rather than replay,
overwrite or repair. This is not production provisioning or worker enrollment;
it adds no process, host-install or qualification authority.

The 0.74 original execution composition is Linux runtime-only and reuses the
existing helper and descriptor readers. It adds no wire/persisted schema or launch
API. Only the original identity's process can bind fixed production outputs and
signed material; a failure latches all borrowed guards, even after wrapper drop.
Sequential observations remain neither original-start authentication nor durable
refusal, writer revocation or qualification authority.

The 0.74 fixed production output inspector is Linux runtime-only. It adds no
wire or persisted artifact, output creation or launch authority. Its numeric
worker UID/GID inputs are assertions, not authenticated account enrollment;
read-only ancestry observations are neither durable refusal nor writer revocation.

The 0.74 original manager/process/tree identity join is Linux runtime-only and
reuses the existing bounded loaded-manager helper. No new wire, persisted schema
or launch operation is added. Its refusal latch is not a durable journal, and
stable sequential observations do not authenticate original start or cleanup.

The 0.74 fixed named-output fixture guard and its composed process IO guard are
Linux runtime-only. They add no serialized artifact, helper mode or launch API;
explicit temporary ownership assertions do not certify production ancestry.

The 0.74 original executable/output descriptor binding is Linux runtime-only:
no process/helper wire or persisted schema changes. Its sticky refusal is not a
durable journal; caller-asserted output objects are not certified pathnames or
authenticated original start. Existing versioned manager projections are unchanged.

## Versioned Artifacts

| Artifact | Current Version | Writer | Reader Compatibility | Failure Mode |
| --- | --- | --- | --- | --- |
| Separate 0.74 local worker policy and signed envelope | `diagnostic-worker-policy-074-v1` / `diagnostic-signed-worker-policy-074-v1` | Future separately reviewed operator enrollment; local test signer only in this slice | Canonical LF-terminated sorted JSON, 4096-byte body and 8192-byte envelope; external issuer key/body digest/epoch, distinct domain and strict Ed25519 verification. Fixed diagnostic account, exact nonroot IDs/groups and account-document hashes; host/boot/user/mount context values remain caller assertions. Existing receipt v1 and NSS/numeric APIs are unchanged. | Unknown/duplicate/future/noncanonical/oversized documents, wrong pins/epochs/signatures/context or invalid identity refuse without fallback. Original guard cannot refresh; any revalidation failure latches refusal. No old-format migration, real issuer/host enrollment, durable epoch registry, file observation, install or start route. |
| 0.74 read-only fixed worker account observation | `1; separate local Linux helper mode` | Matched parent and `diagnostic-worker-account-worker`, fixed canonical request only | Bounded canonical snapshot projects the fixed user/group, UID/GID and at most 32 sorted membership IDs after two matching forward/reverse rounds. Private snapshot decoding; original guard is runtime-only. | Unknown/duplicate/noncanonical/future fields, missing account, NSS error, exhaustion, invalid mapping or drift refuse without fallback or retry. Older binaries reject the new helper mode; deploy matched parent/worker. NSS observation is not trusted provider/namespace enrollment, actual process group proof, account creation, launch or admission. |
| 0.74 read-only loaded diagnostic settings projection | `1; separate local Linux helper mode` | matched supervisor parent and `diagnostic-loaded-worker`; existing manager scope and snapshot unchanged | New canonical bounded wrapper binds the existing manager snapshot to 53 typed settings or absent unit with no settings. Private decoding only; the original invocation guard is runtime-only. | Unknown/duplicate/noncanonical/type/version/scope drift refuses. Missing settings and zero/replaced invocation cannot bind. Observation failure latches the guard. Older workers reject the new mode; deploy matched parent/worker. No durable migration, unit mutation, full policy, append filename, effective environment, executable, tree or cleanup authority. |
| Separate 0.74 diagnostic builder public policy | `diagnostic-builder-policy-074-v1` | reviewed operator configuration; standalone protected signing lane | Canonical newline-terminated JSON, independently pinned SHA-256 and controller public key; fixed repository/builder identity and distinct builder key. Existing schema-1 build receipt and signature domain are unchanged. | Unknown/duplicate/noncanonical fields, weak/shared/wrong keys, digest/repository/builder drift refuse without fallback. Local tests enroll no real key or host execution. |
| Separate 0.74 local worker issuer public selection | `diagnostic-worker-issuer-key-selection-074-v1` | operator-only local key custody and public selfcheck; not installed trust | Canonical LF JSON with raw Ed25519 public key, SHA-256 of its 32 public bytes, local custody modes/UID and a fixed signature in the separate `hydracache-diagnostic-worker-key-selfcheck-074-v1` domain. Worker-policy pin/epoch stay null; no private material is included. | Local inspect requires the exact recomputed document and rejects unknown/version/noncanonical/drifted/incomplete data. No automatic rotation, migration, policy signature, host enrollment or launch authority. Existing worker-policy schema/domain and old model APIs are unchanged. |
| Separate 0.74 fixed worker trust input | `diagnostic-worker-trust-input-074-v1` | root-owned operator configuration at `/etc/hydracache-perf/worker-trust-074.json`; no writer or production enrollment in this slice | Exactly schema/repository/issuer/policy-body SHA-256/epoch in canonical LF JSON, 2048-byte bound. Selected real issuer, explicit body pin/nonzero epoch, original no-follow root/file identities and bytes; owned policy guard borrows original config and active journal. | Missing/null/unknown/duplicate/future/noncanonical data, wrong issuer, metadata/content drift, policy/journal/refusal mismatch fail permanently without refresh, repair or fallback. Host remains asserted; no fixture promotion, policy/trust/FD export, durable revocation append or execution. Old policy/journal/receipt schemas remain unchanged. |
| Separate 0.74 diagnostic build-job observation | `diagnostic-builder-observation-074-v1` | reviewed unsigned build coordinator | Strict fixed-source/tree/cleanliness before-and-after assertions, exact compiler/Cargo and command; only the isolated protected signing lane may treat them as trusted builder observations. | Unknown/duplicate/version/source/tree/cleanliness/compiler/command drift or oversized document refuse. Assertions from an arbitrary caller are not compilation proof; this format alone grants no install or execution authority. |
| Local-only 0.74 unsigned build content inspection | `diagnostic-unsigned-content-inspection-074-v1` | operator `audit-local-build`, no signing or execution | Bounded ELF/Cargo/lock/four-P0 content audit reports byte digests and explicit false attestation/source/install/execution/admission flags. It cannot construct VerifiedBuild. | Empty, over-budget, malformed, input/hash/profile/feature drift and nonregular leaves refuse. Unix linked leaves refuse. This is not safe ancestry, atomic file, trusted build or installed-executable proof. Existing signed receipt/campaign formats and production enrollment are unchanged. |
| Local-only 0.74 Linux diagnostic spool manifest | `1; temporary filesystem fixtures only` | local spool library, no production caller | Canonical manifest binds fixed cell, checked build, caller terminal, original file stamps/observed sizes, bounded prefix digests and complete inner byte packet or overflow-without-packet. Exactly three read-only files publish by exclusive staging and Linux no-replace rename with sync barriers. | Unknown/duplicate/noncanonical fields, external digest mismatch, unsafe metadata, changed original descriptors or conflicting publication fail loud. Pre-rename pending is retained and blocks replay; post-rename exact replay revalidates/syncs but never overwrites or executes work. Overflow hashes prefixes, not whole streams. Fixture ownership/cleanup and software fsync are not production ancestry, power-loss or live cgroup proof. Old receipt/campaign formats, installation and admission are unchanged. |
| Local-only 0.74 raw diagnostic receipt packet | `1; byte-only, no production route` | local verifier and synthetic fixtures only | Canonical JSON binds exact cell intent, externally verified build receipt digest, caller terminal summary and original bounded stdout/stderr hashes/sizes. Strict mirrors of the unchanged observer schema reconcile fixed P0 samples, CPU quality, HDR projection and RESP wire evidence. | Malformed/duplicate/unknown/missing fields or inconsistent successful content produce retained invalid decisions, never repaired successes. Offline replay recomputes the whole packet from independent inputs. Overflow refuses a complete packet. A packet is not durable spool or authenticated live cgroup proof; neither production execution nor admission is enabled. Existing campaign/build/observer formats and old packets are unchanged. |
| Local-only 0.74 signed diagnostic build statement | `1; no production builder enrollment` | synthetic local fixtures only in this slice | Canonical bounded `build-receipt-v1.json` uses a separate Ed25519 build domain and externally supplied builder key/repository policy. Strict source/tree/toolchain/features/allocator/command and binary/log/lock/four fixed P0 config hashes and sizes bind the diagnostic identity. Linux inspection owns read-only bundle/file descriptors; fixture inspection cannot certify production ancestry. | Unknown/duplicate/noncanonical fields, signature or fixed-input drift, unsafe modes/owners/links/special files and unexpected entries refuse verification. Signature is an attestation, not independent compilation proof. Revalidation detects drift but does not close later pathname-exec races. Old unsigned preparation seals remain inspection-only and cannot substitute. No builder key, installation, live execution or admission is enabled; rollback must retain existing diagnostic cleanup/history obligations. |
| Local-only 0.74 diagnostic lease envelope and terminal receipt | `1; no production IPC/CLI` | supervisor diagnostic library, local fixtures only | Canonical bounded JSON with state SHA-256; readers require schema 1, strict fields, exact lease/boot/monotonic identity and legal state. `active-diagnostic.json` coordinates with `active-campaign` under the same short host transaction lock. | Corruption, links, pending crash files, conflicting markers and incomplete cgroup cleanup retain/refuse reservation; no retry or admission. Unknown schema fails loud. Old supervisors ignore the new marker: do not downgrade or run one concurrently until the exact diagnostic cgroup is empty, terminal receipt durable and marker released. Host installation remains unauthorized. |
| Local-only 0.74 signed diagnostic request, unsigned response and request ledger | `1; no production route` | diagnostic library and temporary local socket fixtures only | Ed25519 signs the entire strict request under a separate diagnostic domain, including nonce, expected revision and controller attempt. Fixed P0 only. Schema-1 canonical bounded `diagnostic-requests-v1.json` binds original controller/lease and cached response under the shared host fence. | Changed request-id/nonce reuse, stale revision, principal drift, expired authorization, malformed/linked/pending ledger or exhausted 128-entry/256-KiB capacity refuse execution. Exact completed replay returns historical receipt without extending liveness. Uncertain intent never reexecutes; no eviction or automatic reconciliation. Owner tampering is outside the digest threat model. Older readers do not know this ledger: no downgrade with an unresolved intent or active diagnostic; first prove exact empty-tree cleanup and retain terminal/history evidence. No installation or production enrollment authorized. |
| 0.74 read-only diagnostic manager helper scope and snapshot | `1; local Linux only` | same supervisor executable in parent/worker modes; no campaign protocol change | Canonical bounded JSON scope fixes lease, boot and one of four surfaces. Snapshot binds root/PID-1 unique systemd owner and bounded unit identity; it is an observation, not cleanup or admission authority. | Future/unknown/duplicate/noncanonical fields, principal/scope/boot drift, foreign unit/cgroup, timeout, combined output overflow or nonzero helper exit refuse. Partial output is retained in the runtime failure; caller must archive it. No retries or unit mutation. Unknown helper mode on older executables fails; deploy matched parent/worker together. No persistent schema migration or host installation is authorized. |
| Opt-in Redis RESP client-certificate TLS | `0.74 opt-in; RESP2/3 unchanged` | `hydracache-server` Redis listener | `redis_api.mtls_client_ca_path` / `HYDRACACHE_REDIS_MTLS_CLIENT_CA_PATH` requires rediss, server TLS, required AUTH and an enabled listener. An omitted option preserves old TOML and TLS/plaintext defaults. The dedicated inbound CA is not inherited from global `tls.ca_path`; a verified certificate does not bypass AUTH or select a tenant. | Invalid/dormant configuration or empty/malformed/over-budget/unreadable CA refuses startup. Missing/invalid client certificate refuses TLS before RESP dispatch. Only this mode has a five-second handshake deadline, 128 owned connection ceiling, and abort/join on listener shutdown. Older binaries do not enforce the new option: block listener traffic before rollback; never reuse an mTLS policy with an older binary or silently remove the option to downgrade it. |
| `CacheInvalidationFrame` | `1` | `hydracache` invalidation bus publishers | Readers accept version `1` only. Unknown versions are rejected before apply. | Decode error is reported and the receiver continues. |
| `hydracache_invalidation_outbox` schema | `1` | `hydracache-db` outbox writers or application SQL writers | Workers accept schema version `1`. Unknown future versions must fail loud before draining. | Worker refuses to start; intent is left durable and pending. |
| `hydracache_hook_schema` schema | `1` | `hydracache-db` generated hook installers | Reconciliation expects version `1` for installed hook plans. Missing or mismatched rows report drift. | Staging/release gates can fail before silently trusting disabled or stale hooks. |
| `RaftLogStore` in-memory format | `1` | `hydracache-cluster-raft` metadata runtime | 0.41 tests cover append/replay, snapshot recovery, suffix truncation, and compaction guard semantics. Future durable engines must register their own format before rollout. | Runtime fails loud on store errors; unknown future durable formats must refuse startup. |
| HTTP replication/peer encoded-value transport | `1` | `hydracache-cluster-transport-axum` clients | Strict routes require `x-hydracache-wire-version: 1`; mismatches are rejected before payload apply. | Route returns upgrade-required style safe rejection; counters can record wire-version failures. |
| `DurableRaftLogStore` format | `1` | `hydracache-cluster-raft` durable-log feature | Readers accept format `1` and refuse unknown future versions before opening a store. | Store open fails loud; no committed command is acknowledged from an unknown format. |
| `SledRaftLogStore` snapshot envelope | `1` | `hydracache-cluster-raft` `sled-log-store` feature; enabled by `hydracache-server` for `storage_dir/raft-log` | New snapshot writes use a magic/version/length/checksum envelope around the raft protobuf payload. Readers still accept legacy raw protobuf snapshots for backward compatibility, but enveloped snapshots verify length and checksum before install. Standalone members reopen the same log before constructing `RawNode`, so persisted `HardState.commit` is never paired with an empty replacement log after process restart. | Bit-flipped or truncated snapshot bytes fail loud during store open; valid snapshots restored into the wrong metadata identity are rejected by the runtime identity contract before apply. A missing/inconsistent retained log refuses startup instead of accepting a leader commit over empty storage. |
| Raft metadata snapshot payload | `1` | `hydracache-cluster-raft` metadata runtime | Raft `Snapshot.data` may carry an internal `HCMETA01` JSON payload with the applied metadata command list and applied index. Snapshot install restores that state-machine payload before applying the committed tail. Empty legacy snapshot payloads remain log-level only and do not claim metadata recovery. | Unsupported versions, malformed payloads, cluster mismatches, duplicate command ids, or inconsistent applied indexes fail loud before serving restored membership. |
| Raft metadata golden-vector corpus | `1` | `hydracache-cluster-raft` tests/vectors | `0.62.0` records command envelopes, wire-message payloads, `ConfState`, and snapshot `ConfState` byte vectors. Readers must keep decoding the corpus with the checked expectations; intentional codec changes must regenerate the corpus in the same change and document the compatibility decision. | Golden-vector tests fail before a rolling-upgrade compatibility claim is made. |
| `node-identity.json` member identity file | `1` | `hydracache-server` member startup | Readers accept format `1` only. The file records `cluster`, stable `node_id`, and derived `raft_node_id` so a member keeps the same raft identity across address changes. A configured `node_id` must match the persisted identity once the file exists. | Unknown future identity formats, cluster mismatches, configured-id conflicts, and node-id/raft-id mismatches refuse member startup before opening the networked grid. |
| `ReplicatedValueRecord` durable format | `1` | `hydracache` durable-values feature | Readers accept format `1`; records carry partition, version, epoch, and value/tombstone state. | Unknown future formats must refuse startup before serving replicated values. |
| `ChecksummedReplicatedValueRecord` durable envelope | `1` | `hydracache` scrubber/checksum helpers | Readers accept envelope format `1`; the envelope stores a deterministic checksum over `ReplicatedValueRecord` payload fields. Scrubbers verify before serving and may repair from valid peer copies. | Checksum mismatch is reported; unrepairable corruption is not served. Unknown future envelope formats fail closed. |
| `DurableValueStore` on-disk value-store format | `1` | `hydracache` `durable-value-store` feature | Readers accept store format `1` only. Records are length-prefixed binary envelopes containing the cache key, `ReplicatedValueRecord`, tombstone/value state, and checksum metadata. The value engine is separate from the raft log engine. | Store open refuses unknown future formats. Corrupt or mismatched records fail loud and are not served. |
| `DurabilitySnapshotManifest` format | `1` | `hydracache` durability write path | Readers accept manifest format `1` only. The manifest records namespace, snapshot scheduler time, interval, and the covered `(partition, version, epoch)` watermark with a deterministic checksum. | Unknown future manifest formats or checksum mismatches fail loud before recovery trusts the snapshot watermark. |
| `ClusterCheckpointManifest` format | `1` | `hydracache` checkpoint coordinator | Readers accept manifest format `1` only. The manifest records a cluster-wide checkpoint id, authority epoch, per-partition barrier watermarks, per-node `DurabilitySnapshotManifest` evidence, and a deterministic checksum. | Partial/torn cuts, stale partition watermarks, checksum mismatches, older-than-authority checkpoints, and unknown future formats fail loud before recovery or rescale trusts the cut. |
| CRDT value encoding | `1` | `hydracache` active-active CRDT helpers | The 0.45 serde shape for `GCounter`, `PnCounter`, `OrSet`, `LwwRegister`, and OR-set tags is the registered CRDT value encoding version `1`. Durable or replicated adapters must keep this shape stable or introduce a new explicit version before emitting a changed encoding. | Unknown future CRDT value encodings must fail closed before merge/apply so stale readers do not converge on different metadata. |
| WAN `RegionLink` replication batch frame | `1` | `hydracache` active-active region-link helpers | The 0.45 serde shape for `GeoBatch`, `GeoWrite`, `IdempotencyKey`, and `GeoBatchApplyReport` is the registered WAN batch frame version `1`. Readers must preserve idempotency-key pairing and HLC/epoch/version fields. | Unknown future WAN frames must be rejected before apply; mismatched entry/idempotency vectors are already refused. |
| Cross-region anti-entropy digest exchange | `1` | `hydracache` region-link anti-entropy helpers | The 0.45 serde shape for `PartitionDigest`, `VersionSummary`, and CRDT metadata GC confirmation is the registered digest exchange version `1`. Readers compare `(version, epoch)` summaries only within the matching partition. | Unknown future digest versions must fail loud before diffing so repair cannot silently skip or over-apply keys. |
| Replayable invalidation stream snapshot | `1` | `hydracache` invalidation-ring durable adapters | The 0.46 `InvalidationRingSnapshot` shape is the registered retained-window format version `1`: partition, capacity, head sequence, next sequence, and retained invalidation events. Durable adapters must keep sequence semantics stable. | Unknown future stream snapshots must refuse restore before serving; subscribers outside retention fall back to clear-partition semantics. |
| Hinted-handoff record format | `1` | `hydracache` hinted-handoff stores | The 0.46 serde shape for `Hint`, `HintBudget`, `HintOutcome`, `HintReplayDecision`, and `InMemoryHintStore` is the registered handoff record format version `1`. Readers must retain target, key, partition, version, epoch, sealed bytes, and creation time. | Unknown future hint records must be rejected before replay; over-budget or expired hints fall back to Merkle repair rather than silent loss. |
| Merkle repair exchange format | `1` | `hydracache` Merkle repair helpers | The 0.46 serde shape for `MerkleTree`, `KeyRange`, `RepairToken`, `RepairSession`, and `RepairReport` is the registered repair exchange version `1`. Readers interpret ranges as inclusive key ranges and watermarks as incremental repaired-key cursors. | Unknown future repair exchanges must fail loud before applying foreground or scheduled repair. |
| Session token wire format | `1` | `hydracache` causal/session helpers and client context carriers | The 0.47 serde shape for `SessionToken`, `SessionId`, `SessionRequest`, `SessionLifecycleDecision`, and failover recovery reports is the registered session-token wire format version `1`. Tokens carry a bounded watermark, nonce, issue time, and MAC. | Unknown future token formats, forged MACs, wrong-session tokens, replays, and expired tokens fail loud or downgrade through the documented sessionless rebuild path. |
| Session watermark format | `1` | `hydracache` causal/session helpers | The 0.47 serde shape for `SessionWatermark`, `PartitionKey`, and `VersionStamp` is the registered watermark format version `1`. Readers preserve bounded coarsening semantics and compare `(version, epoch, HLC)` stamps. | Unknown future watermark formats must fail closed before using them to satisfy read-your-writes, monotonic-read, or causal-read guarantees. |
| Single-key conditional/fenced-lock engine state | `2` | `hydracache::SingleKeyConditionalStore` and future applied lock-command paths | `0.52.0` extends fenced-lock state from token-only holds to session-bound `LockHold { owner, fence, holds, lease_deadline }` with logical-time lease expiry and new `ConditionalError` variants `LeaseExpired`, `NotOwner`, and `ReentrancyLimit`. Fence tokens are assigned only by the applied deterministic state machine. | Readers or services that cannot understand version `2` lock state or the new error variants must fail loud before serving fenced-lock/CAS operations; stale owners, expired leases, non-owners, and weak consistency levels are rejected and counted. |
| `ControlPlaneSnapshot` format | `1` | `hydracache` self-heal snapshot helpers | Readers accept format `1` and refuse unknown future versions before restore. | Restore fails loud before rebuilding topology from an unsupported snapshot. |
| `BackupManifest` helper format | `1` | caller-supplied `hydracache` backup/PITR helper datasets | Helper readers accept manifest format `1`, verify object length/checksum, and refuse unknown future manifest versions before helper restore/PITR replay. The server does not currently populate this dataset from its live value plane. | Helper restore fails loud; this is not evidence that `/admin/backup` created a durable artifact or authoritative restore point. |
| External client HTTP route boundary | `1` | `hydracache-client-transport-axum` | Public clients use `/client/v1/*`; internal member routes remain under `/cluster/*` and are not part of the public client compatibility surface. Unknown future client route versions are refused instead of falling through to member handlers. | Unauthenticated, oversized, malformed, or wrong-route requests are rejected before protocol dispatch or state mutation. |
| HydraCache external client protocol | `4` | `hydracache-client-protocol` and `hydracache-client-transport-axum` | Protocols `1`, `2`, `3`, and `4` use custom length-prefixed binary frames over HTTP/2; the request payload is a typed HydraCache message carrying `protocol_version`, request id, context, deadline, idempotency key, stable operations, stable errors, and B1 watermark fields. Protocol `2` keeps the same framing and adds the 0.52 IMap/Fenced Lock operation family: `TryLock`, `Unlock`, `RenewLockLease`, `ForceUnlock`, `GetLockOwnership`, `CompareAndSet`, `RemoveIfValue`, `SubscribeEntryEvents` and responses `LockAcquired`, `LockBusy`, `LockReleased`, `LockLeaseRenewed`, `LockOwnership`, `CasApplied`, `CasMismatch`. Protocol `3` keeps v2 framing and adds additive TTL/expiry operations and response metadata needed by the Redis RESP edge: write expiry, mutate expiry, clear expiry, and read remaining TTL without changing value bytes. Protocol `4` keeps v3 framing and adds Redis-lock conditional value operations: `ConditionalPut`, `CompareValueAndInvalidate`, `CompareValueAndExpire`, `ConditionalStored`, and `CompareValueApplied`; `CompareValueAndExpire` carries an expiry mode for replace, replace-if-expiring, or add-to-remaining so redis-py `Lock.extend` does not shorten a lock when `replace_ttl=False`. Entry-listener projections are cache signals, not business event logs. v2-only operations must be gated on negotiated version `2` or newer; v3-only TTL metadata/expiry operations must be gated on negotiated version `3` or newer; v4-only Redis-lock conditional operations must be gated on negotiated version `4` or newer. See ADR `docs/adr/0007-client-wire-framing.md`. | Out-of-window versions, malformed/truncated frames, oversized frames, v2-only operations on v1 envelopes, v3-only TTL metadata/expiry operations on v1/v2 envelopes, v4-only Redis-lock conditional operations on v1/v2/v3 envelopes, and unknown future protocol versions are refused loud before mutation. Region-scoped subscriptions narrow delivery, not correctness. |
| Hibernate L2 provider contract | `1` | `hydracache-client-protocol::hibernate` and external `hydracache-hibernate` providers | Hibernate ORM 6.x providers map regions to HydraCache namespaces, access strategies to stable L2 consistency labels, and query cache regions to timestamp/bulk invalidation. See `docs/integrations/hibernate.md` and ADR `docs/adr/0006-why-not-clone-hibernate-hikaricp.md`. | Unsupported Hibernate versions, unsupported query-cache mode, or unknown future mapping versions must fail loud at provider bootstrap. HydraCache never joins JVM transactions. |
| Client SDK conformance manifest | `1` | `hydracache-client` and `sdks/python` | `crates/hydracache-client/tests/fixtures/conformance/client_v1.json` is the language-agnostic scenario set for protocol-v1 SDKs. Rust and Python SDKs are supported only when their runners pass this manifest. | SDKs that do not pass the manifest are not claimed as supported. Manifest major changes require a new compatibility entry and SDK semver mapping. |
| HC/2 generation and cross-SDK conformance manifest | `generation 6 / manifest 1` | `hydracache-client-hc2`, `sdks/java`, and `sdks/python/hydracache-client-hc2` | The protobuf contract and generated Rust/Java/Python metadata are checked by `client-schema-check`. `docs/testing/hc2-client-conformance-v1.json` binds the stable Rust/Java/Python set to named executable proofs for generation, data, mutation, cancellation, listener repair, fencing, session loss, and clean close. Generation 5 remains a retained compatibility reader; generation 6 is the preferred writer/client generation. | Unknown generations, dirty/nondeterministic generation, a missing SDK proof, semantic row drift, or silent fenced-session replay fail closed before package/release admission. |
| Java migration toolkit contract | `2` | `hydracache-client-protocol::java_migration` and external JVM artifacts | Java/Spring/Hibernate migration artifacts use the protocol-v1/v2 client contract, stable Java exception mapping, safe codec/schema registration, Spring Cache `native`/`jcache`/`none` modes, Java lock operation mapping, Java IMap conditional write mapping (`replace`, `replace-if-present`, `remove-if-value`), Java IMap entry-listener projection mapping, and the checked-in Hazelcast API manifest at `crates/hydracache-client-protocol/manifests/unsupported_hazelcast_apis.txt`. See `docs/integrations/java-migration.md`. | Unknown future contract/manifest versions, ambiguous codec ids, reflective or Java-native serializers, member-mode app JVM topology, unsupported Hazelcast APIs, non-admin `ForceUnlock`, unaudited admin force-unlock attempts, and attempts to treat entry listeners as `Ringbuffer`/`ReliableTopic` business logs fail loud with migration hints. |
| `ResidencyPolicy` control-plane format | `1` | `hydracache` residency governance | Policies are committed at a control-plane epoch per namespace with optional per-key overrides. Readers accept format `1`, enforce allowed regions at placement, WAN value movement, read serving, and include-value invalidation decisions, and report the enforced epoch. | Unknown future policy formats are rejected before commit. Unsatisfiable in-policy RF, forbidden boundary crossing, stale policy epochs, and forbidden-region reads fail loud and emit audit-ready events. |
| Tenant status JSON schema | `1` | `hydracache-observability` and `/client/v1/status` | `TenantStatus` is scoped to the verified caller tenant and includes schema version, namespace usage/quota, rate/fair-share state, and near-cache/subscription health. | Unknown future status schema versions must be treated as incompatible by strict clients. Servers must not include other tenants in a caller-scoped status response. |
| Consumer audit event schema | `1` | `hydracache-observability` audit recorders/sinks | Audit envelopes carry schema version `1` plus redacted governance/security/admin events. Keys are omitted or hashed; values are never logged. | Mandatory governance/security event sink failures fail closed. Future schema versions require an explicit compatibility entry before operator log readers accept them. |
| Server admin status JSON shape | `1` | `hydracache-server` `/admin/status` | `0.57.0` adds the required `source` field (`live` or `modeled`) to the existing operator status shape (`leader`, `term`, `quorum_ok`, `members`, `reshard_phase`, `draining`). `0.66.0` additively exposes sorted `member_ids` and numeric `voter_ids`, allowing strict operator proofs to reject equal-count ghost/missing-member substitutions. Management Center consumers must carry the source tag through every rendered status; existing readers that ignore unknown fields remain compatible. | Missing, unknown, or untrusted `source` values must be treated as `modeled`/not-live. Identity-sensitive consumers must require exact `member_ids`/`voter_ids` sets rather than inferring them from counts; consumers must fail loud or degrade honestly rather than painting modeled or partial data as live. |
| HydraCache Redis RESP edge surface | `RESP2/RESP3 subset v1` | `hydracache-redis-compat` and opt-in `hydracache-server` Redis listener lifecycle | `0.63.0` registers an off-by-default RESP2/RESP3 cache-subset facade for one selected RESP endpoint. It translates supported Redis cache commands into HydraCache client protocol operations. The expanded 0.63 scope depends on protocol v3 TTL metadata/expiry operations for `SET EX/PX`, `SETEX`/`PSETEX`, `EXPIRE`/`PEXPIRE`, `PERSIST`, `TTL`, and `PTTL`; protocol v4 Redis lock-conditional operations for single-endpoint `SET NX PX/EX`, token-safe lock release, and token-safe lock extension; atomic client-surface batch apply for `MSET`; Redis `AUTH`/`HELLO 2 AUTH`/`HELLO 3 AUTH` semantics for auth-required listeners; native `rediss://` transport security backed by server TLS certificate/key material; RESP3 negotiation for the same cache subset; a single logical Redis database where `SELECT 0` is a no-op compatibility command; minimal honest `INFO`; cache-subset `TYPE` (`string`/`none`); reviewed lock-script allowlist behavior for redis-py `Lock` and Node `redlock@5.0.0-beta.2` single-resource scripts on that selected endpoint; and admin-disabled `CONFIG`/`FLUSHDB`/`FLUSHALL` behavior that never fabricates config or mutates keys by default. RESP state and Redis lock state are node-local to the daemon/listener in 0.63. | Unsupported commands, admin-disabled commands, unsupported RESP3 aggregate command frames, oversized frames, malformed/truncated frames, unauthenticated data commands, wrong tenant scope, plaintext on a TLS-enabled RESP listener, wrong CA, wrong Redis credentials, non-zero `SELECT`/multi-db attempts, invalid DB indexes, Redis `ROLE`/`DBSIZE`/`SCAN`, candidate tag commands, cross-endpoint Redis key visibility, multi-endpoint Redis lock mutual exclusion, non-atomic `MSET`, missing TTL metadata, unknown/general Lua scripts, non-allowlisted lock-library scripts, Redisson full-lock traces, Redlock quorum/multi-key lock attempts, and ambiguous TLS config fail loud before mutation. No `MOVED`/`ASK`/Redis Cluster redirects are emitted. |
| `HydraCacheCluster` status schema | `v1alpha1` | `hydracache-operator` status reconciler | `0.61.0` adds optional `status.bootstrapReplicas` to preserve the original bootstrap voter cohort for deterministic StatefulSet pod start mode. Existing objects with the field absent are initialized from the existing StatefulSet current/spec replica count, not from a later desired scale target. | The operator must preserve the field once recorded; losing or recomputing it from desired replicas can make scale-up pods self-bootstrap instead of joining the existing raft cluster. |
| Prometheus metric name catalog | `1` | `hydracache-observability` `PrometheusExporter` / `registered_metric_names()` | `0.57.0` emits every registered cache, admission, cluster-grid descriptor, and topology metric name (`hydracache_cluster_*`, `hydracache_backup_age_seconds`) with stable Prometheus HELP/TYPE headers. Topology metrics carry `source="live|modeled"` so modeled status is never consumed as live. Backup age is supplied by a separate trusted checkpoint observation; `/admin/backup` request acceptance does not update it. | Future metric renames/removals require a new compatibility entry. Missing registered metrics, duplicate HELP/TYPE headers, request acceptance mislabeled as backup completion, or high-cardinality labels such as keys/request ids must fail gates before dashboard or alert readers trust the scrape. |
| ClusterOverview JSON shape | `1` | `hydracache-server` `/cluster/overview` | `0.57.0` introduces a read-only Management Center overview with stable fields `source`, `members`, `leader`, `partitions`, `consistency`, `backup_age_seconds`, and `lifecycle`. `source` is `live` or `modeled`; modeled views must carry `leader:null`. Consistency is represented as `configured_default` plus `op_counts_by_level`, never as a single fabricated current level. `backup_age_seconds` is nullable trusted observation, not an `/admin/backup` completion signal. | Unknown future overview shapes must be treated as incompatible by strict console readers. Missing or untrusted `source`, stale leaders during elections, omitted unreachable members, or `backup_age_seconds:0` for never-backed-up namespaces must fail tests or degrade honestly before the console trusts the view. |
| Server HydraCache actuator mount | `1` | `hydracache-server` `/actuator/hydracache/*` | `0.57.0` mounts the existing read-only `hydracache-actuator-axum` JSON routes on the internal admin listener. The daemon registers its cache as `server`; `/actuator/hydracache/health`, `/caches`, `/caches/server/diagnostics`, `/caches/server/stats`, `/cluster/staging-health`, `/cluster/pilot-report`, and `/correctness` are granular diagnostics that complement `/cluster/overview`. | The mount must remain read-only, available during drain, and absent from the public client surface. Unknown cache names return `404`; missing admin mount or accidental client-port exposure fails server actuator mount tests. |
| Simulator snapshot JSON schema | `5` | `hydracache-sim` and `hydracache-sim-wasm` | Readers accept the current schema only and reject unknown future versions. Version `1` carried seed, step, logical time, nodes, links, sampled keys, real invariant verdict, and progress. Version `2` adds the 0.53 W1 election/formation fields: `formation_phase`, `election_source`, `election_disclosure`, and per-node `vote_state`, `voted_for`, `votes_received`. Version `3` adds typed `in_flight` network/election messages plus `over_budget.in_flight_summarized` for bounded rendering. Version `4` adds manual-mode `clients`, `subscribers`, and `sync_progress`. Version `5` adds topology/mode state: per-node `disabled`, top-level `rebalance`, `mode`, `active_scenario`, and `intervention_count`. | Strict readers reject unknown future schema versions before rendering or replaying a shared seed. The demo must not synthesize a green verdict outside `InvariantChecker`; `election_source = "sim-model"` must be presented as a teaching model, not a production consensus claim. In-flight render summaries and subscriber buffer drops must fail loud through over-budget/drop counters instead of silently dropping evidence. |
| `ReplayScriptV1` simulator control artifact | `1` | `hydracache-sim`, `hydracache-sim-wasm`, sandbox `/sim/control`, and browser share/replay surfaces | Version `1` carries `seed`, `mode`, optional `scenario`, and ordered `ControlActionV1` actions (`step`, topology verbs, `push_event`, `subscribe`, `mode_change`) with logical `at_step`. Readers accept version `1` only and reject oversized scripts above `MAX_REPLAY_ACTIONS`. | Unknown future replay versions and over-budget action lists are rejected before execution. Topology actions not implemented by the current work item fail loud rather than being ignored or emulated. |

## Upgrade Rules

The 0.74 pinned diagnostic start material is a runtime-only, descriptor-owning
object, not a new persisted or helper-wire artifact. The fixed install reader
requires an externally checked builder policy; historical raw-trust fixture
inspection remains fixture-only. Build receipt, policy, lease, request ledger,
observer and manager helper schemas are unchanged. Start-only unit construction
caps runtime by the latest validated lease observation, without changing the
running-state model intent or the frozen 60/300-second ceilings. Neither the
runtime object nor a matching observation authorizes execution or rollback.

- Writers may not emit a newer durable or wire artifact until readers in the
  deployment explicitly support it.
- Unknown future schema versions fail closed. A worker must not silently drain a
  table it does not understand.
- Unknown wire versions are treated as decode errors, not panics.
- Forward-only migrations must be idempotent: applying the same migration twice
  leaves the artifact at the same version.

## Server Configuration Reader Policy

`ServerConfig` TOML is an additive, human-authored startup document rather than a
versioned durable or wire artifact. Readers ignore unknown fields so a newer
configuration can be inspected or rolled back by an older binary, but every
recognized field and every TLS, authentication, listener, and role combination
is validated before any listener is materialized. Environment loading starts
from secure defaults, applies StatefulSet-derived identity, and gives an
explicit `HYDRACACHE_CLUSTER_START` value precedence over ordinal inference.
HydraCache does not merge a TOML file and environment variables implicitly;
callers select one source. Credential files are read only for validation/runtime
construction, and credential bytes must never appear in config serialization,
`Debug`, startup errors, or metric labels.

## 0.37 Baseline

`0.37.0` starts this register with the existing invalidation frame and the new
database invalidation outbox schema. Later cluster releases should append raft
log format, replicated value record format, and public client protocols here
before claiming rolling-upgrade compatibility.

## 0.38 Correctness Automation

`0.38.0` adds hook-schema compatibility tracking and reconciliation drift
reports. These reports are assisted-mode guardrails: they make missing hook
schema rows, mismatched hook versions, outbox backlog, and dead-lettered rows
visible to CI/staging gates. They do not make HydraCache a transparent DB proxy
and do not remove the need to install hooks/outbox migrations in the database.

## 0.41 Grid Slice

`0.41.0` registers the first distributed-grid durable and wire-visible seams:
`RaftLogStore` format version `1` for the metadata log seam and HTTP wire
version `1` for encoded replicated/peer value transport. The release ships an
in-memory store and feature-gated example path only; production durable engine
selection remains future hardening work and must add its concrete on-disk format
to this register.

## 0.42 Grid Hardening

`0.42.0` registers the supported durable raft-log format version `1` and the
replicated value-record format version `1`. The durable raft seam refuses unknown
future format versions before opening a store. Replicated value records persist
sealed bytes plus `(partition, version, epoch)` and tombstone state so restart and
anti-entropy can converge without resurrecting deleted keys.

## 0.43 Geo-Distribution And Elasticity

`0.43.0` registers control-plane snapshot format version `1` for operational
self-healing backup/restore. Upgrade checks keep the 0.42 -> 0.43 rolling window
bounded to raft-log format `1`, replicated value-record format `1`, and
invalidation wire frame version `1`; incompatible jumps fail loud before a mixed
cluster step is accepted.

## 0.44 Deterministic Simulation And Scrubbing

`0.44.0` adds checksummed replicated-value envelopes and a scrubber gate for
durable value records. The underlying `ReplicatedValueRecord` payload format
remains `1`; the new `ChecksummedReplicatedValueRecord` envelope format `1`
detects corruption before serving and can repair a corrupt primary copy from a
valid peer copy. Unknown future envelope formats fail closed.

## 0.45 Active-Active Multiregion

`0.45.0` registers the first active-active cross-region data shapes as
compatibility version `1`. CRDT values (`GCounter`, `PnCounter`, `OrSet`,
`LwwRegister`) converge only if all durable and replicated adapters preserve the
same metadata shape. WAN batches (`GeoBatch` / `GeoWrite`) carry HLC,
origin-region, epoch/version, sealed value bytes, and one idempotency key per
entry. Anti-entropy digests (`PartitionDigest` / `VersionSummary`) compare
per-key `(version, epoch)` summaries for one partition.

The 0.45 code exposes these as serde/runtime shapes rather than standalone
codec wrappers. That shape is now registered as format `1`; any future external
codec, durable adapter, or wire transport that changes it must introduce an
explicit versioned envelope and reject unknown future versions before
merge/apply/diff.

## 0.46 Cluster Resilience And Coordination

`0.46.0` registers the resilience artifacts that can survive a transient outage
or cross-process repair path. The replayable invalidation stream snapshot
retains partition, capacity, sequence window, and invalidation events. Hinted
handoff records retain target, key, partition, version, epoch, sealed bytes, and
creation time so replay can suppress stale writes and tombstone resurrection.
Merkle repair exchanges carry inclusive key ranges, tree summaries, repair mode,
and incremental repaired-key watermarks.

The 0.46 shapes are registered as version `1`. Durable adapters must not restore
unknown future invalidation-ring snapshots or hint records silently. Repair
peers must reject unknown future exchange shapes before using them to decide
which keys to exchange.

## 0.47 Cross-Region Session Consistency

`0.47.0` registers the client-visible session consistency formats as version
`1`. Session tokens carry a stable session id, bounded watermark, nonce, logical
issue time, and tamper-evident MAC. Session watermarks carry bounded
partition/region stamps using `(version, epoch, HLC)` ordering and explicit
coarsening accounting.

These formats are part of the causal/read-your-writes contract. Unknown future
token or watermark formats must fail closed before the reader treats them as
evidence for session guarantees. Forged, replayed, wrong-session, or expired
tokens already take the loud error or documented sessionless-rebuild path.

## 0.48 Production Deployment And Security

`0.48.0` registers `BackupManifest` helper format version `1`. The helper
validates manifest version, object length, and checksums before rebuilding the
caller-supplied `BackupDataset`, and refuses unknown future manifest formats
before replaying PITR records. The dataset is not sourced from
`ClientSurfaceState` or the live cluster value plane. In 0.66,
`/admin/backup` remains request-only and reports both
`durable_artifact_created: false` and `restore_point_available: false`.

A live backup/restore compatibility claim is deferred until the server owns a
production backup source, durable object-store adapter, restore sink,
authority/fencing protocol, and key provider. Encrypted live restore and key
rotation are therefore outside the 0.66 compatibility surface.

## 0.49 Ecosystem And External Consumers

`0.49.0` reserves the external client HTTP route boundary at `/client/v1/*`.
The boundary is intentionally separate from internal `/cluster/*` member routes:
public clients cannot hit member handlers by path confusion, and anonymous or
oversized client requests are rejected before protocol dispatch or cache state
mutation. The stable client wire protocol itself is registered as its own artifact
when W1 publishes protocol version `1`.

W1 registers the HydraCache external client protocol version `1`. Framing is a
custom `u32 body_len | u16 protocol_version | postcard payload` binary envelope
over the existing HTTP/2 route boundary, not gRPC. Readers reject unknown future
protocol versions loud. Region-scoped `SubscribeInvalidations` is a dissemination
filter: it may narrow delivery, but it never hides correctness-relevant
cross-region invalidations, and `include_value` is residency-gated.

W2 registers Hibernate L2 provider contract version `1`. The provider contract is
not a Hibernate clone: an external Java `hydracache-hibernate` artifact (planned,
not yet in-repo — see `technical-debt/TD-0005-…`) *would* implement Hibernate ORM
6.x SPI and use the W1 protocol. Regions map to namespaces,
`read-only` maps to `strong-immutable`, `nonstrict-read-write` maps to
`best-effort-invalidate`, and `read-write` / `transactional` map to
`invalidate-on-commit`. Query cache support is timestamp/bulk invalidation or a
loud bootstrap refusal.

W3 registers the client SDK conformance manifest version `1`. The Rust reference
SDK (`hydracache-client`) and the Python SDK (`sdks/python`) use the same
language-agnostic scenarios for protocol negotiation, get/put/invalidate,
near-cache repair, deadlines/retry/idempotency, and stable W4/W5 error contracts.
An SDK is not considered supported unless its runner passes the manifest.

W4 binds public client identities to a bounded tenant roster before namespace
ownership, quota accounting, or tenant-labelled metrics. Over-quota and over-rate
conditions use the existing protocol-v1 stable errors `TenantQuota` and
`RateLimited` with retry-after hints; unknown tenants are refused before dispatch.

W5 registers residency policy format version `1` for authoritative
namespace/key governance. Residency is distinct from performance placement: it
filters legal regions first, then asks placement to satisfy RF inside that set.
WAN links refuse to ship pinned value bytes over forbidden boundaries, reads from
forbidden regions fail closed even if a stale local copy exists, and
policy-narrowing reports explicit eviction/degraded remediation actions.

W6 registers tenant status JSON schema `1` and consumer audit event schema `1`.
`GET /client/v1/status` is read-only and scoped to the verified tenant identity.
Audit events are append-only, redacted, and mandatory for governance/security
refusals: if the configured mandatory sink is unavailable, the guarded operation
fails closed rather than proceeding without an audit trail.

W7 registers Java migration toolkit contract version `1`. The contract is
cache-focused: typed Java clients, Spring Boot 2/3/4 starters, Spring Cache
`native`/`jcache`/`none`, Hibernate L2 delegation, listeners, Micrometer, and
Actuator probes are allowed to build on protocol-v1 behavior. The toolkit must
not imply Hazelcast wire or API compatibility. Unsupported Hazelcast APIs are
listed in `unsupported_hazelcast_apis.txt` and must fail loud with migration
hints instead of becoming silent no-ops.

## 0.50 Interactive Simulator Demo

`0.50.0` registers simulator snapshot JSON schema version `1`. The schema is a
communication surface for the interactive demo and sandbox simulator routes, not
a correctness gate. It serializes the real deterministic `SimWorld` state and
the real `InvariantChecker` verdict. Unknown future schema versions are rejected
loudly by strict readers before rendering, seed sharing, or route consumers treat
the payload as compatible.

## 0.51 Configurable Persistence

`0.51.0` registers `DurableValueStore` on-disk value-store format version `1`
for the feature-gated value-plane persistence backend. The store persists sealed
replicated value records and tombstones in length-prefixed binary envelopes and
verifies checksums before serving. It is intentionally a separate engine from
the raft log store. Unknown future store formats refuse startup; corrupt or
mismatched records fail loud and are not served.

W4 registers `DurabilitySnapshotManifest` format version `1`. Snapshot manifests
record the namespace and covered write watermark `(partition, version, epoch)` so
recovery can fail loud instead of guessing what a scheduled snapshot contained.
Unknown future manifest formats or checksum mismatches are rejected before the
manifest is trusted.

## 0.52 IMap And Fenced Lock Java Surface

`0.52.0` bumps the HydraCache external client protocol to version `2` while
keeping the protocol `1` reader window open. The frame shape stays the ADR-0007
custom length-prefixed binary envelope over HTTP/2; v2 is an operation-family
extension, not a transport swap. Ordinary protocol-v1 cache requests continue to
receive protocol-v1 response frames/envelopes. Any v2-only IMap/Fenced Lock/CAS
operation sent on a v1 envelope is rejected with `IncompatibleVersion` before
dispatch so older clients never observe a v2-only response shape.

W3/W5 add the v2 fenced-lock and IMap CAS request/response family: `TryLock { ns, key,
lease_ms, wait_ms, level }`, `Unlock { ns, key, fence }`, `RenewLockLease { ns,
key, fence, lease_ms }`, `ForceUnlock { ns, key }`, `GetLockOwnership { ns, key }`,
`CompareAndSet { ns, key, expected, new_value, level }`, and
`RemoveIfValue { ns, key, expected, level }`, and
`SubscribeEntryEvents { ns, region, from, include_value, projection }`; responses include
`LockAcquired { fence }`, `LockBusy`, `LockReleased`, `LockLeaseRenewed`,
`LockOwnership { fence, locked }`, `CasApplied { new_version }`, and
`CasMismatch { current }`. `CasExpectation::Present` is the replace-if-present
form and must mismatch absent/tombstoned keys rather than inserting. Weak
consistency levels are rejected as a stable conflict envelope; non-leader
lock/CAS endpoints return a retryable backend unavailable envelope with a leader
hint instead of applying locally.

W6 registers `EntryEventProjection`, `EntryEventSource`, `EntryEventKind`,
`EntryEvent`, and `EntryListenerContract`. The only supported listener contract
is a bounded, coalesced cache signal with lag/drop counters; it must not be
interpreted as a durable business event log. Sources that do not prove
`Removed` or `Evicted` project to `Invalidated`, and ordinary writes project to
`Upserted` rather than fabricating separate `Added`/`Updated` transitions.

The single-key conditional/fenced-lock engine state remains registered as
version `2`: session-bound lock ownership, logical leases, reentrancy count, and
the conditional tombstone used by remove-if-value are applied through the
deterministic state machine. Readers or services that do not understand those
states or error variants must fail loud before serving the 0.52 lock/CAS surface.

## 0.53 Interactive Cluster Lab

W1 bumps the simulator snapshot JSON schema to version `2`. The new fields make
the previously implicit cluster-formation/election model visible to browser,
WASM, and sandbox consumers: top-level `formation_phase`, `election_source`, and
`election_disclosure`, plus per-node `vote_state`, `voted_for`, and
`votes_received`. The current W1 election path is explicitly `sim-model`; it is a
deterministic teaching model over the simulator FSM and must not be described as
the production consensus implementation. Unknown future snapshot versions
continue to fail closed before rendering.

W2 bumps the simulator snapshot JSON schema to version `3`. The new `in_flight`
array carries typed network and election messages (`heartbeat`, replication,
ack, vote request/response, and leader heartbeat), with deterministic ids,
source/destination nodes, optional keys, sequence/term, and logical delivery
timing. The array is capped by `MAX_IN_FLIGHT_RENDERED`; omitted records are
reported through `over_budget.in_flight_summarized` so UI and replay consumers
can detect summarized evidence instead of mistaking it for an empty network.

W3 bumps the simulator snapshot JSON schema to version `4` and registers
`ReplayScriptV1` format `1`. The new manual-mode fields expose client actors,
namespace subscribers, bus-carried subscriber events, lag/drop counters, and
per-node sync progress. `ControlActionV1` is the shared control surface for
native tests, WASM, and sandbox routes; actions outside the current implemented
work item fail loud until their owning work item lands.

W4/W5 bump the simulator snapshot JSON schema to version `5`. Topology controls
(`isolate`, `rejoin`, `disable`, `enable`, `add_node`) now execute through
`ControlActionV1`; snapshots expose disabled nodes, deterministic rebalance
progress, current lab mode, active scripted scenario, and replay-visible
intervention count. The W4 add-node/reshard path is a simulator model for the
lab and does not claim production ownership movement.

## 0.55 Durable Store Hardening

W4 registers `ClusterCheckpointManifest` format version `1` for coordinated
cluster-wide consistent cuts. The manifest is separate from
`DurabilitySnapshotManifest`: node snapshots prove durable local coverage, while
the cluster checkpoint records the controller barrier watermarks that define the
cut. Restore and rescale readers accept only format `1`, verify checksum and
per-partition coverage, reject partial/torn cuts, and fence checkpoints older
than the current authority epoch before serving or redistributing data.

## 0.60 Networked Grid Hardening

W3 registers the `node-identity.json` member identity file at format version `1`.
The file lives in the member `storage_dir` beside the raft log and pins
`node_id` plus its derived `raft_node_id` to the durable member state. Readers
accept only version `1`, refuse unknown future versions, and fail startup when an
explicitly configured `node_id` conflicts with the persisted identity. Address
changes without an explicit conflicting id keep the persisted identity so the
raft log is not orphaned by a listen-address change.

W4 persists raft `ConfState` through the existing durable raft log store so voter
add/remove survives restart. This does not introduce a separate public manifest
or file-format version; compatibility remains governed by the existing raft log
store and the pinned raft/protobuf dependency pair.

## 0.62 Cluster Correctness Test Hardening

`0.62.0` enables raft pre-vote by default in the metadata runtime. The mixed
window is intentionally one release: pre-vote and non-pre-vote nodes can still
exchange the same raft protocol messages, but new rolling upgrades should move
the whole cluster to the pre-vote default before relying on partition-rejoin
stability evidence.

The release also tightens the networked daemon raft sender mapping. Wire sender
ids are now resolved through the same stable node-id to raft-id mapping used by
the outbound sink; unknown senders fail loud instead of falling through a
parse-first shortcut for integer-like node ids.

The raft metadata golden-vector corpus is registered as compatibility version
`1`. It covers command envelopes, wire-message payloads, `ConfState`, and
snapshot `ConfState` bytes. Any future codec change must either preserve these
vectors or regenerate them with an explicit compatibility note in the same PR.

The `test-failpoints` feature and `fail` dependency are test-only. They must not
appear in the default server/operator/raft release graphs; `cargo xtask
verify-no-test-features` enforces that boundary.

## 0.65 Redis Debt Safety Net

`0.65.0` does not widen the public Redis command set or distributed deployment
claim. The RESP listener remains a standalone, node-local facade, and the
stable debts `resp-cross-endpoint-key-visibility` and
`resp-cross-endpoint-lock-safety` keep that limitation executable through
real-process flip-sentinels.

The native client protocol keeps the published v1 minimum surface and accepts published v2 and frozen
v3 frames. Post-v1 operation families are rejected at the version-aware codec boundary.
Fixture provenance and digests bind those bytes to their historical source, and
compatibility tests require v1/v2/v3 requests to complete without receiving v4
conditional-result shapes. Envelope and inner-frame versions must agree before
any mutation. Protocol v4 remains the first version that exposes conditional
put/lock result shapes; the new Redis characterization does not backport them.

The Redis conformance manifest itself is now a compatibility artifact: command
routes, stable case ids, current/target debt claims, structured test layers, and
real Rust test references are checked by `cargo xtask doc-check`. A future
backend refactor may change implementation ownership, but it must preserve the
client-surface and RESP characterization layers and explicitly invert the
deployment sentinels when it pays a documented debt.
## 0.69 executable migration evidence

The Java facade claim is bounded by
`docs/integrations/hazelcast_borrowed_suite.json`: it is source-compatible for the listed adapted
expectations only, not a Hazelcast interface, runtime, or wire implementation. Default Hazelcast
lock reentrancy remains a documented divergence; server-side entry processing remains unsupported.
The gated proof executes the facade through the real Java HC/2 client and production daemon. Lock
leases expire in the production state machine; Java `FencedSession` loss is separately proved and
does not imply that facade locks are session-bound.

The DB conformance claim covers SQLite and PostgreSQL transactional outbox enqueue, worker drain,
and commit-scoped `InvalidationWait` behavior. A receipt waits only for rows with its namespace and
commit position; later pending rows do not block it. It does not promote `NoWait` into a
read-after-write guarantee.

This is a source-level change for custom `InvalidationOutbox` implementors: 0.69 requires the new
`status_for_commit` method. Returning namespace-wide status from that method violates the receipt
contract even if it compiles; adapters must filter by both namespace and exact commit position.
`crates/hydracache-db/tests/custom_outbox_contract.rs` is the compiling downstream-style migration
fixture and can be copied as the adapter skeleton.

The SQLx PostgreSQL claim is exercised against the digest-pinned 16.4 floor and current major 18.
This matrix is the declared 0.69 evidence boundary; other PostgreSQL majors are not implied by it.

HC/1 client compatibility is executed for the library commits behind `v0.62.0`, `v0.62.1`, and
`v0.63.0`. HC/2 is not reimplemented in 0.69: the complete retained nine-row 0.68 compatibility
artifact remains a required prerequisite.

## 0.72 Management Center 2.0

The read-only `/management/v1` JSON contract starts at schema version `1`. The first registered
runtime DTOs are `ClusterFormationSnapshot`, `PlacementDecisionTrace`, `DurableRecoveryStatus`,
and `ConsensusProgressSnapshot` from `hydracache-observability`. They are observations only and do
not admit members, choose/commit placement, repair storage, or advance Raft state.

Readers accept schema version `1` and reject any other top-level schema version through
`validate()` before publication or rendering. Unknown future enum/reason values deserialize to an
explicit `unknown` variant and must remain unavailable/partial; they never fall through to serving,
applied, clean, repaired, or PASS. Cross-field validation rejects discovery-as-authority,
serving without live authenticated/admitted/current state, unstable placement order, placement
progress contradictions, clean recovery with loss/corruption/partial evidence, and Raft progress
where snapshot/applied/commit/catch-up positions are incoherent.

Candidate, reason, label, identity, and selected-peer collections have exported hard limits.
Changing a limit or field meaning is a compatibility change and requires new goldens plus a
documented reader window. These DTOs are not persisted in 0.72; if a later work item retains
recovery or placement status on disk, that artifact receives a separate format registration and
crash-recovery contract before use.

The first server-backed routes are `/management/v1/capabilities`, `/management/v1/dashboard`, `/management/v1/formation`,
`/management/v1/consensus/progress`, and `/management/v1/persistence/recovery`. They are available
only on the internal admin listener and currently reuse its verified privileged identity boundary.
The routes accept GET and body-free HEAD; mutation methods are not registered. Formation reads the
committed roster and Raft progress through `GridControlPlaneHandle`, hashes node identities before
serialization, and joins remote progress only through the authenticated
`/cluster/management/v1/snapshot` route. Recovery deliberately returns
`unknown/status-not-retained` until a real retained source is connected; process liveness is never
upgraded to a clean recovery claim.

The W3 cluster snapshot RPC uses schema version `1` and is unavailable on public client listeners.
Targets are resolved only from the current committed roster; browser URLs and discovery-only
candidates are never inputs. Collection is capped at 100 members, concurrency 8, 500 ms per peer,
1.5 seconds per refresh, 256 KiB per peer response and one coalesced in-flight refresh. Its
one-second immutable cache rejects authority-epoch or observation-sequence regression. Missing,
late, incompatible, duplicate and identity-mismatched observations remain bounded partial evidence;
they are never zero-filled or retried until shown green.

List responses are capped at 100 items and 256 KiB. Continuations are 30-second opaque server-side
cursors bound to route class, authority epoch, and observation sequence; a changed snapshot returns
`snapshot-changed` instead of merging pages. The cursor registry retains at most 1,024 records.
These limits and source ownership are mirrored in the machine-checked 0.72 source and bounds
registries under `docs/testing/management-center/0.72`.

The W4 dashboard consumes typed management snapshots and does not parse the Prometheus endpoint.
Its charts are explicitly browser-local history since the page opened: at most 24 series, 360
points per series, 4,320 points overall and 256 KiB of encoded ring state. Oldest samples are
evicted, hidden/offline tabs pause collection, obsolete fetches are aborted, and retry delay is
bounded with jitter. A new authority epoch resets all series; a counter reset produces a gap while
gauges remain point values. Missing, non-finite, partial, stale and modeled observations are never
coerced to zero or live. The dashboard currently reports CPU, RSS, retained bytes, uptime and TTL
backlog as unavailable because no authoritative source is connected; later work items may fill
those fields without changing their null/unavailable meaning.

W5 adds `/management/v1/cluster/members`, `/management/v1/cluster/formation`,
`/management/v1/cluster/partitions`, and the non-enumerable
`/management/v1/cluster/placement-traces/{opaque_id}` resource. The original
`/management/v1/formation` path remains a compatibility alias. Member and ownership rows are bound
to one authority epoch, observation sequence, and node generation. Only the local member exposes
proved process/cache facts; unavailable remote CPU, RSS, file descriptor, task, client, and
partition measurements remain null. Configuration is represented by a non-reversible digest over
an explicit secret-free field allowlist.

Partition ownership and placement traces are published only from a validated runtime topology
observation matching the request's exact epoch (and, for ownership, sequence). In runtimes without
that source, real repair/reshard counters remain visible while assigned, unassigned, distribution,
and trace link stay unavailable. Placement candidates have deterministic selected-first order and
stable reason order. `committed` and `applied` are distinct outcomes with distinct progress indexes;
unknown, malformed, and stale opaque trace identifiers return the same 404 response.

W6 adds the admin-only `/management/v1/clients` schema version `1`. It reports a fixed, sorted
protocol domain (`hc1`, `hc2`, `resp`) from node-local runtime accounting. HC/2 and RESP retain
process-lifetime accepted/closed counters and live-owner gauges; where lifecycle dimensions are
available, `accepted - closed = active` is a validated invariant. HC/1 exposes only counters its
request-oriented transport can prove, so connection totals remain null and the response remains
partial. Missing buffered-byte, reconnect, slow-client, cleanup-lag, or quota sources are null,
never zero-filled. Per-client detail is not advertised until a bounded process-scoped opaque
registry exists, preventing remote address, identity, certificate, token, tenant, key, payload,
and session-token disclosure.

W7 adds caller-scoped `/management/v1/namespaces` and
`/management/v1/namespaces/{namespace}/caches` resources. The verified client/tenant headers are
resolved against the server-side tenant roster before totals are computed. An identity/tenant
mismatch returns 403; both a hidden namespace and a nonexistent namespace return the same 404 on
the detail route. The DTO never repeats tenant identity and never exposes keys, values, tags,
loader errors, raw policy, request payloads, or listener data.

Namespace entry/logical-byte usage and quota values come from the exact 0.71 tenant accounting.
Physical retained bytes, hit/miss/load, TTL backlog, tag/index, conditional, idempotency, audit,
backup, and breaker values remain null until a tenant-scoped owner source can prove them. The
single `client-surface` cache row is therefore partial and explicitly quality-tagged. Subscription
status for isolated tenants no longer reuses the global process gauge, preventing a cross-tenant
count oracle.

W8 adds the admin-only `/management/v1/healthchecks` catalogue. Stable check IDs, status/category
enums, remediation codes, evidence codes, threshold units and evaluation version are compatibility
artifacts. The server evaluates checks from immutable typed inputs; the browser only filters and
renders returned verdicts. Missing, partial, modeled, mixed-epoch, incoherent, or required truncated
evidence is `UNKNOWN`, never `PASS`; `DISABLED` requires explicit configuration. The headline is
the worst known status while UNKNOWN remains a separate count. Recovery improvement is accepted
only from a strictly newer coherent observation in the current authority epoch, so stale `Clean`
cannot overwrite newer `Degraded` evidence.

W9 adds the opt-in `/management/v1/history` fixed-query adapter. The browser can select only the
registered `replication_success`, `replication_failure`, `cache_entries`, or
`admission_queue_depth` query
ID plus bounded start/end/step values; PromQL, origin, headers, redirects, and credentials are not
wire fields. The configured origin contains scheme/host/port only. HTTPS is required unless HTTP
is explicitly allowed, and private/loopback destinations require a separate explicit allowance.
Every DNS answer is checked before a socket opens and the reviewed address is pinned for the
request. Range, point, series, response-byte, concurrency, token-file and deadline limits are
compatibility-visible. Labels and upstream error bodies are discarded. Disabled or failed history
never replaces the bounded browser-local W4 ring, and remote/local series are not spliced.

W10 adds authenticated GET-only `/management/v1/persistence`, `/management/v1/operations`, and
`/management/v1/audit` resources. Persistence configuration never exposes its destination and a
backup-age observation is not a verified artifact. Verified backup/restore identity, size and
capacity remain null until a durable owner retains that proof. Recovery remains
`unknown/status-not-retained` when no `RecoveryReport` survives startup.

The operation journal retains at most 128 newest records for the current process generation;
audit metadata retains at most 256 transitions. Eviction and generation are explicit. `requested`,
`accepted`, `running`, `completed`, `failed`, and `unknown` are distinct wire states; only legal
forward transitions are accepted and terminal records cannot change. The audit view covers only
these management transitions and contains no keys, values, paths, credentials, tenant/client
identity, or raw error text. The console issues only GET requests for all three views.

W11 changes the management authorization contract from the legacy write-admin marker to a distinct
`management.read` capability on every `/management/v1/**` route. Write-admin identities continue to
imply read; a tenant-scoped reader does not. This is an internal-admin-listener header contract, not
a public bearer credential: any remote reverse proxy must authenticate the caller, strip inbound
`x-hydracache-*` headers and install verified identity/capability headers. The console now emits
`x-hydracache-management-read: true` and no write-admin header.

Management reads have a separate frozen concurrency ceiling of 16 and fail fast with HTTP 429 when
saturated. Static and JSON responses add restrictive CSP/no-sniff/no-referrer/no-store headers;
assets remain same-origin with no runtime CDN dependency. These protections and the exact npm/SBOM
inputs are package-visible behavior for 0.72 and may only be relaxed through a reviewed compatibility
change with replacement security evidence.

W13 freezes five separately named compatibility artifacts in
`docs/testing/management-center/0.72/compatibility.toml`: formation, placement trace, durable
recovery, health catalogue/evaluation, and consensus progress, all at schema version `1`. They are
read models and are not persisted. A 0.72 reader accepts only the registered schema, maps supported
future enum values to explicit `unknown`, and rejects an unknown top-level schema before rendering.

Before a 0.72 member sends `/cluster/management/v1/snapshot`, it authenticates and reads the bounded
`/cluster/management/capabilities` endpoint. A 0.71 peer has no such endpoint, so it is recorded as
`incompatible`/partial and never receives the unsupported snapshot POST. This is intentionally not
zero-filled: apply lag, recovery cleanliness, membership admission, and serving state remain
unknown. The capability is part of the aggregation cache key, so a peer upgrade invalidates the
cached partial observation.

`HYDRACACHE_MANAGEMENT_API_ENABLED=false` removes both `/management/v1/**` and the embedded
`/console` assets while preserving `/cluster/overview`. This is the rollback and old-bookmark
window; callers must treat 404 as absence of the 0.72 capability, not as an empty healthy cluster.
The exact console, its server-embedded copy, source registries, and CycloneDX SBOM are packaged in
one deterministic bundle whose manifest records every file SHA-256, a set digest, and the exact
40-character candidate commit. Verification rejects dirty-tree release packaging, an altered
file, a rebuilt substitute, or a source-commit mismatch.
