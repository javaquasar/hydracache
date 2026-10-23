# HydraCache Compatibility Register

The opt-in joined original collector output layout v1 creates
`diagnostic-collector-output-074-<sha256 of original Reserved LF bytes>/collector-attempt-v1/{collector-stdout.bin,collector-stderr.bin}`
under the original retained `/var/lib/hydracache-performance` parent. Both
directories are exclusive root:root 0700; empty sinks are exclusive root:root
0600 regular single-link files. File and containing-directory fsync precede
runtime acknowledgement. Partial or complete context is never adopted,
overwritten, repaired or automatically removed. Old three-document intent
history and signed collector projections stay unchanged. Retained paths cannot
recover an owner, execution permission, completion or a replay right. No
installed consumer, migration or downgrade is provided. The consuming entry is
Linux x86_64 only; exact F_GETFL includes its kernel-forced large-file bit.
Nonroot modeled tests use only separately retained temporary ancestry.
This layout is not an installed production execution capability.

The opt-in collector start-consent byte artifact uses body
`diagnostic-collector-start-consent-074-v1`, envelope
`diagnostic-signed-collector-start-consent-074-v1` and Ed25519 domain
`hydracache-diagnostic-collector-start-consent-074-v1`, NUL and canonical LF JSON.
Body/envelope ceilings are 4096/8192 bytes. It binds first-cell Starting revision
two, original request/reservation/state/challenge/response and worker/unit hashes,
policy epoch, controller/source/preset and a positive signing window at most
60 seconds. Only matched opt-in library signers/inspectors understand it; old
request, collector-intent and challenge readers refuse rather than reinterpret
it. Unknown, duplicate, future, partial, noncanonical, oversized or wrong-signature
input refuses. Signature inspection with a supplied public key returns untrusted
data, not freshness, enrolled trust, replay, durable lifecycle or start authority.
No installed consumer, migration or automatic downgrade is provided.

The binary-private version1 `HC074RL1` vocabulary additionally selects cuts11/12
for latched-before-send and sent-before-recv-resume. Mode3 is permitted only for
the two new no-EXITKILL controls, which continue untraced and must never be
accepted as authenticated handoff. Old cut/mode pairs keep their exact meaning;
old readers refuse these additions. Frame16, original setup2 FDs, G/H gates,
kernel credentials and packet limits are unchanged. A required ECONNRESET prefix
is confined to the new sent-loss socket disposition; it is not EOF, and actual
zero/HUP plus the unchanged EOF guard remain mandatory. No product wire or
durable schema changes.

Fixed `root-transfer-{loss,healthy}-*` and two `root-transfer-no-exitkill-*`
diagnostics extend the binary-private version1 `HC074RL1` selected-cut vocabulary
with cuts5..10: installed, ready, receive-entry, delivered, acked and closed.
Only installed/delivered admit the no-EXITKILL mode. Old cuts0..4 remain unchanged;
old readers refuse new cuts rather than interpret them as old states. Exact
selected frames, stage, reserved byte, two setup FDs, credential checks and G/H
gates keep their existing shape and limits. For these socket-only cuts, the
monitor creates the original pair before its fixture creator fork, checks each
endpoint's distinct identity before G and discards its child-end duplicate.
These public-dummy diagnostics are not a credential or production-start API.

The binary-private `root-handoff-*` profile uses an unnamed seqpacket at FD3,
not a product listener or credential API. It reuses version1 exact 16-byte
`HC074-READY-v001`, `HC074-DUMMY-v001`, `HC074-ACK---v001` records plus the existing
loaded/installed markers. The fixed original native trace binds each message
to its stack bytes and syscall; READY is not authorization. One attempted send
is latched before dispatch, including failure. Unknown/short/oversized/duplicate
records, wrong calls/FD/flags/pointers, lost ACK and replay refuse without resend.
No ancillary operation is dispatched. Exact close3, stdio-only inventory, zero
exit, reap/ECHILD and EOF precede successful cleanup. Old pipe modes remain
separate; no persistent schema, public process API or product protocol changes.

Fixed `root-loss-*` modes add only a binary-private local seqpacket diagnostic,
not a product wire protocol. Version1 frame `HC074RL1` is exactly16 bytes: magic8,
positive original child PID LE i32, fixed cut byte0..4, mode0..2, stage0..1,
reserved0. Readers match the selected fixed mode/stage exactly. Setup carries
two SCM_RIGHTS descriptors (original child pidfd and pipe reader); cut carries
none. One-byte gates G/H carry no descriptors. Kernel SCM_CREDENTIALS PID/UID/GID
and received CLOEXEC are mandatory; malformed/unknown/truncated input refuses.
Four packets per direction and128 ancillary bytes are hard bounds. No caller PID,
path, command or FD override exists. This is not authentication for credentials,
worker start or authority. No durable/product compatibility change.

The separate fixed `diagnostic_signer_root_probe` modes `root-native`, `root-x32`
and `root-i386` compose the selected root with the existing 64-instruction policy.
Binary-private child modes accept no extra input. A shared private installer adds
no public process-mutating API; the old ordinary entry retains its proc checks.
The root parent checks actual TSYNC/header/BPF, successful SET_DUMPABLE(0), one
new kernel filter and a strict syscall sequence. Direct GET_DUMPABLE remains
forbidden by the unchanged policy, not added as an exception.
The version1 private pipe messages are distinct fixed 16-byte loaded
`HC074ROOTEXECv1\n`, installed `HC074ROOTGUARD1\n` and native-done
`HC074ROOTDONEv1\n`; none is authentication or authority. Native completion is
exit0; alternate ABI must be coreless SIGSYS without a done marker. The fixed
`root-kill-installed` and `root-drop-installed` diagnostics prove original cleanup
from the installed stop, not creator loss. Every accepted path requires reap,
ECHILD, EOF and original root removal; unknown input refuses. `root-exec` still
kills at loaded stop without installing policy. No credentials, protected handoff,
worker start, product protocol, qualification or ship admission changes.

`diagnostic_signer_root_probe root-exec` adds a fixed local empty-environment
diagnostic, not a public launcher or signer service. The binary-private
`--root-child` emits only the 16-byte `HC074ROOTEXECv1\n` marker on its fixed
nonblocking write pipe, then stops and exits 93 if unexpectedly resumed; invalid
context exits 92. Marker version1 is not an authority or authentication token.
The parent validates the original selected exec and kernel VMA identities, kills
without resuming the loaded stop, reaps and proves EOF before root cleanup.
Unknown/extra input and unsupported platforms refuse. `prepare` is unchanged.
The normal procfs-dependent restriction installer is not composed with this root;
no credentials, signing, product wire, qualification inputs or admission change.

`diagnostic_signer_owned_probe` adds a fixed nonproduct self-exec diagnostic,
not a service or public launcher API. Public native/x32/i386 modes require an
empty environment and a private readonly 0555 self image. Internal child modes
only enter the existing fixed probe after a PDEATHSIG precondition; they grant
no authority. Unknown input/unsupported platform refuses. No product defaults,
wire/domain/receipt/config/dependency/qualification inputs change; see the
[original-owner design](testing/performance/0.74/diagnostic-signer-owned-probe-design.md).

The additive `diagnostic_signer_isolation_probe` binary is a closed nonproduct
Linux x86_64 child probe, not a configured service or signer. Only native/x32/i386
arguments are accepted. Unsupported platforms and unknown input refuse; the
library entry terminates its current process and must only run in a fresh owned
child. The fixed policy equals the old fixture and changes no product defaults,
wire schema, signing domain, receipt, dependency or qualification input. No
server/controller consumer is connected; see the [child-entry design](testing/performance/0.74/diagnostic-signer-child-entry-design.md).

The optional `diagnostic-signer-bootstrap-context-074-v1` inner document uses
canonical sorted-key JSON plus one LF and strict unknown/future/type/duplicate
refusal. Stronger portable/Linux APIs require it in the unchanged signer receipt's
bootstrap-context slot; legacy opaque build/runtime readers remain compatible.
No new signing domain, config loader, wire negotiation, installed workflow or
default behavior changes. Rust-only stronger results cannot deserialize or
extract weaker guards; see the
[context design](testing/performance/0.74/diagnostic-signer-bootstrap-context-design.md).

The opt-in Linux original GNU runtime join uses the unchanged
`diagnostic-signer-build-074-v1` receipt/domain. For this path only, the existing
runtime-manifest slot must contain canonical sorted-key JSON plus one LF of
`diagnostic-signer-gnu-declarations-074-v1`, regenerated from original files;
opaque, alternate encoding or other-file catalogs refuse. The legacy model's
opaque-slot behavior remains compatible. No stored configuration reader,
migration, default, public wire protocol or server admission changes. The new
Rust-only owner cannot be deserialized or negotiated; see the
[join design](testing/performance/0.74/diagnostic-signer-runtime-join-design.md).

The opt-in offline `diagnostic-signer-gnu-declarations-074-v1` catalog is
serialize-only inventory data: four logical roles with byte digests and ELF
declarations, with all authority flags false. There is no runtime trust reader,
negotiation or migration from a signer-build receipt or old fixture result.
Existing signing/policy domains and old readers stay unchanged; unknown catalog
shapes cannot enter any configuration/receipt/start reader. See the
[shared catalog design](testing/performance/0.74/diagnostic-signer-gnu-catalog-design.md).

The new Linux root-owned input guard is Rust-only original-descriptor inventory,
not a persisted/wire artifact or authenticated bundle. Its observed system-file
profile allows root-owned 0444/0555/0644/0755 leaves under 0555/0755 ancestors;
the old bundle's exact 0555 rule and schemas stay unchanged. Missing required
statx support or original context/path/content drift refuses without fallback
or refresh. The local example prints diagnostic inventory only and exposes no
server reader or negotiation. See the
[root-owned input design](testing/performance/0.74/diagnostic-root-owned-input-design.md).

The opt-in local `diagnostic-prebuilt-copy-local-074-v1` filesystem layout reserves
an absent `attempt-<32 lowercase hex>` directory with one `diagnostic_controller`
file. Success leaves both at 0555 and the file single-linked; no marker, receipt
or signed production artifact is minted. Older readers have no new negotiation
or recovery path. Existing/partial/complete attempts refuse reuse, and unknown
names or extra children refuse inventory. The returned Rust-only guard is not
persisted/deserializable or convertible to old bundle/launch authority. Existing
signer schemas, pins and bundle readers are unchanged. See the
[copy design](testing/performance/0.74/diagnostic-prebuilt-copy-design.md).

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
FDs and kernel credentials. The unchanged report body uses gate-retention setup
revision2: one descriptor-free progress byte, exactly `G` with one original gate
peer FD, then the report. Seven healthy progress records give ten packets total;
the shared parser's five-FD/32-byte/128-control-byte bounds stay unchanged. The
private child gate admits one fixed cut token and one release byte. The monitor
holds its peer until original tracer reap; EOF alone is not lifetime proof.
Unknown setup/fields/length/descriptor set/peer/ancestry refuses. The old prefix
and new prefix reject each other; no negotiation or fallback is offered between
private libtest revisions. Older binaries have no production consumer. It is
neither persisted nor connected to a
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

The separate opt-in Linux collector requirement projection registers
`diagnostic-collector-intent-074-v1`,
`diagnostic-signed-collector-intent-074-v1`, and
`diagnostic-collector-unit-projection-074-v1`. Body fields reuse the unverified
intent layout, but require the collector body schema. The signature covers
`hydracache-diagnostic-collector-intent-074-v1`, NUL and canonical body JSON plus
LF, with unchanged 4096/8192-byte and exclusive 60-second bounds. The unit hash
covers canonical JSON plus LF for `{schema_version, worker, output}`. Worker
properties retain typed ordering, fixed argv/environment and isolation/budget
controls, but omit both regular-file append properties and use WorkingDirectory
`/`, empty ReadWritePaths and LimitFSIZE zero. Output commits ordered semantic
stdout/stderr pipe slots, exclusive root-private collector sinks, byte ceilings
and the required original-start/revocation/EOF/sync/close obligations. It contains
no raw FD numbers and is not a D-Bus unit specification.

Legacy and collector envelopes, domains and unit hashes are not interchangeable.
Old readers reject collector schemas; the new verifier accepts no legacy intent
or checked guard. Unknown/duplicate/future fields, noncanonical bytes, wrong
signatures, pins or windows refuse. No migration, persisted record, start,
completion, replay prevention or production enrollment is added; signed
requirements are not evidence of their fulfillment. See the
[collector intent design](testing/performance/0.74/diagnostic-collector-intent-design.md).

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
| Separate 0.74 collector Starting byte record | `diagnostic-collector-starting-074-v1` | opt-in Linux byte encoder only; no publisher or installed consumer | Canonical LF JSON, at most 16384 bytes; eventual fixed name `diagnostic-collector-starting-074.json`. Binds exact controller-attempt/collector-Reserved LF digests, canonical signed-consent LF digest and the identical signed Starting state; reuses strict prior-history and adjacent-transition grammar. | Unknown/future/duplicate/partial/noncanonical/oversized/mixed bindings refuse. Byte consistency does not verify signatures, inspect actual clocks/credentials or recover acknowledgement, replay ownership or start rights. Old two-file storage and legacy readers remain unchanged and reject a third record. No migration, overwrite, repair, publication, output, execution or cleanup authority. |
| Separate 0.74 original collector Starting publisher | `diagnostic-collector-starting-074-v1` | opt-in consuming Linux original frame only, not an installed entry | Same byte contract and fixed name; private three-document phase retains original descriptors and fence. Exclusive 0600 create, bounded writes, seal 0400, file/directory fsync and live guards precede runtime acknowledgement. | Default two-file and legacy readers still reject retained Starting history. Partial or complete retained bytes cannot recover acknowledgement or a runtime owner; any uncertainty latches refusal and retains descriptors/context. No repair, retry, migration, process/output start, writer revocation, completion or server admission. |
| Separate 0.74 collector progress challenge | `diagnostic-collector-progress-challenge-074-v1` | opt-in Linux original-owner library only; no installed consumer | Canonical LF JSON, at most 4096 bytes; 32 system-random bytes, original controller-attempt/Reserved/canonical-state hashes, identity, revision 1 and original-kernel issuance BOOTTIME. SHA256 of `hydracache-diagnostic-collector-progress-challenge-074-v1`, NUL and the complete LF document is echoed as nonce_sha256 in an otherwise unchanged signed DiagnosticRequest v1 Heartbeat. | Unknown/future/duplicate/noncanonical/oversized/partial documents refuse. Old clients cannot parse or downgrade this challenge; deploy a matched consumer. Byte inspection computes a digest, never an owner or acknowledgement. Exact owned nonce, original live inputs and exclusive challenge deadline are required; no regeneration, retry, recovered permission, durable progress, Starting or execution authority. |
| Separate 0.74 original collector Reserved | `diagnostic-collector-reserved-074-v1` | opt-in Linux library only; no production consumer | Canonical LF JSON, at most 16384 bytes, in exclusive `diagnostic-collector-reserved-074.json` beside the retained controller attempt. Binds its full-byte SHA256, frozen receiver-observed initial state and separately signed collector envelope. Same original authority and fence remain owned; file and directory sync precede runtime acknowledgement. | Unknown/future/noncanonical/partial/mixed/extra context refuses. The bounded paired byte inspector does not verify signatures or reconstruct acknowledgement. Old readers reject the new inventory; no downgrade, overwrite, repair, deletion, legacy publisher conversion or adoption with unresolved context. Starting, outputs, execution, reboot and global nonce protection are not supplied. |
| Separate 0.74 original controller attempt | `diagnostic-controller-attempt-074-v1` | opt-in Linux library only; no production consumer | Strict canonical LF JSON, at most 16384 bytes, in exclusively created `diagnostic-controller-attempt-074.json` under the original fixed intent leaf. Binds original Reserve request/hash, token digest (never token), build/config digests and original active policy/epoch/authority revision. The same original common fence and authority borrow remain held. | Unknown/future/duplicate/noncanonical/oversized/inconsistent fields refuse. Partial and complete retained context block fresh reservation; bytes never recover acknowledgement or start rights. Legacy intent bytes/names stay unchanged; controller documents cannot transfer into their publisher. No downgrade, deletion, repair or migration with unresolved context. Whole-host rollback and global nonce protection are outside this local contract. |
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

## Private diagnostic guardian notices

The internal 0.74 guardian fork notice protocol, guardian-fork-notice-074-v1,
uses one inherited SOCK_SEQPACKET connection within the same executable image.
A packet is exactly one byte: 1 is readiness; 2 isolation failure; 3 observation
failure; 4 revocation failure; 5 read failure; 6 write failure; 7 overflow;
8 fsync failure; 9 close failure; 10 expiry. Unknown, truncated or oversized packets
are not readiness. The private parent must not interpret disconnect or a missing
notice as successful closure, start authority, retry or recovered acknowledgement.

There is no named listener, external producer, CLI or independently versioned
reader. No migration or durable artifact is introduced. Failure notification is
best effort after receiver loss; the guardian retains the fence regardless.
This protocol does not replace any signed lifecycle record or manager ACK.

## Private original barred custody notices

The separate original-barred-custody-notice-074-v1 protocol uses one inherited
SOCK_SEQPACKET within the same executable image, with no external producer,
listener, CLI or attach API. Each notice is exactly one byte: ASCII P means
pending isolation only, E original-alarm expiry, F isolation/observation failure,
and L unexpected peer activity or loss. P is accepted exactly once during initial
pending construction; duplicate, unknown, empty, oversized or truncated notices
refuse. A later notice, missing notice, disconnect or terminal child cannot
authorize readiness, execution, collector release or recovered acknowledgement.

The child accepts no command and never consumes/rearms the original alarm. Its
failure notices are best effort; expiry or peer loss retains the fence regardless
of delivery. Cancellation uses the atomically created child pidfd while the
parent retains its exclusive original-owner borrow, not a protocol release or
completion command. Deploy only the same-image matched parent/child; old guardian
notice readers are separate and must not reinterpret this vocabulary. No durable
artifact, migration, public wire protocol or production start admission is added.

## Private original custody owner loss fixture

The hc074-pending-owner-test-v1 packet exists only in the opt-in Linux libtest
fault fixture, never an ordinary-build listener or installed command. A private
owned Unix datagram carries bounded JSON (schema, owner, cut, deadline, origins,
closing_authentication) and exactly one SCM_RIGHTS atomically created custodian
pidfd. The test driver requires exact fields, the expected subprocess PID and
kernel SCM_CREDENTIALS UID/GID/PID, and rejects truncated or ambiguous ancillary
data. It never receives the original fence or private custodian notice peer.
Only matched fixture and driver versions are supported; unknown fields/versions
fail the test. This packet is neither a start permit nor a durable receipt.

## Private original barred collector notices

The original-barred-collector-notice-074-v1 protocol is a separate, same-image
inherited SOCK_SEQPACKET, not the custodian or generic guardian vocabulary.
Exactly one byte B denotes barred original descriptor isolation only; E denotes
original-alarm expiry, L peer activity/loss and F isolation/observation/close
failure. Initial observation accepts B once; missing, duplicate, unknown, empty,
oversized or truncated notices and terminal children refuse. No notice grants
activation, original send/start, release, writer cessation or completion.

The child accepts no command and never reads/rearms the original alarm. Before
best-effort E/L/F it attempts to close both original readers and both original
O_RDWR sinks, then parks retaining its original fence alias. A failed close emits
F and cannot prove quiescence. The ordinary owner keeps its exclusive original
borrow through collector cancellation/reaping and subsequent custodian cleanup.
Matched image only; unknown notices fail closed. No public wire or durable format
is introduced, and the generic O_WRONLY engine remains unchanged.

## Private combined original enrollment owner fixture

The hc074-enrollment-owner-test-v1 datagram is cfg(test) only and matched-image
only. A private credential-checked Unix datagram carries exactly two SCM_RIGHTS
pidfds, in collector/custodian order, never original I/O, fence or notice peers.
Closed JSON fields are schema, owner, roles, cut, deadline, origins and
frame_closed. The before-frame cut is after real P/B isolation but before the
closing round; after-frame is after all final observations. frame_closed is a
boolean matching that cut, not an authentication count or execution permit.
Reject extra/missing/duplicate keys, wrong types, credentials, roles, truncation
or ambiguous rights. These test messages confer no start, release or completion
authority. The ordinary P/B child protocols and absolute deadlines are unchanged.

## Private one shot dispatch exchange mechanics

The private dispatch exchange consumes existing D-Bus version 1 messages and
introduces no new public or durable schema. Client/manager unique-name claims
are data, not authenticated connection or PID 1 evidence. Preparation rejects
well-known routing, malformed envelope/body signature and missing, aliased or
nonblocking/non-CLOEXEC pipe writers. It does not check signed unit settings or
original ancestry; those remain obligations of the future original join.

Arming closes retry before transport entry. A reply must match the original
serial, unique manager sender and client destination, with no returned FDs or
unexpected headers/flags. A canonical nonzero u32 job path is observation data
only; missing/malformed/error/duplicate replies never authorize retry, release,
start or completion. Error bodies are not diagnostics. Message ownership survives
preparation failure and uncertain outcomes. No message/FD export API is provided.
Socket-pair SCM_RIGHTS fixtures carry byte-identical zbus-built messages and
verify real local credentials/FDs; they are not system-bus authentication or
original execution. Unknown shapes fail closed. No ordinary sender is connected.

## Private fixed manager channel observations

This private owner uses standard D-Bus version 1 EXTERNAL authentication and
only Hello plus GetNameOwner, GetConnectionUnixUser and GetConnectionUnixProcessID
on the fixed root-selected system bus. It adds no public/durable codec. Actual
SO_PEERCRED and atomic SO_PEERPIDFD, retained procfs/nsfs identity and PID/user
namespace equality precede trust in namespace-relative PID 1/UID 0 replies.
Its METHOD_RETURN flags are exactly 0 or NO_REPLY_EXPECTED; that flag has no
reply semantics under D-Bus. Other known flags, wrong types and correlations
still refuse. The older exchange/request predicates remain unchanged.
The daemon's actual UID need not be zero; root-selected socket ancestry and the
retained kernel peer bind its provenance. Missing kernel access fails closed.

Client name, manager name, server GUID, boot, socket ancestry and original pidfds
remain owned by the live connection. A changed/closed/refused channel cannot
heal. The absolute BOOTTIME two-second observation cap is not an admission lease
and cannot renew any original window. There is no connection/name/FD export,
snapshot constructor, fixture promotion, message sender or execution authority.
Original frame/message joining and FD negotiation/delivery proof remain separate.

## Private dispatch method return interoperability

The one-shot private dispatch exchange now accepts METHOD_RETURN flags 0 or
NO_REPLY_EXPECTED only. D-Bus assigns that bit no reply semantics. All other
known or unknown bits refuse; request flags and ERROR classification stay exact.
Original serial/sender/destination, size/body/FD checks and refusal-before-parse
remain unchanged. A permitted reply remains job-path data, not start, cessation,
completion, retry or release authority. No public/durable format or sender is added.
This supersedes only the old exchange's empty METHOD_RETURN flag predicate;
the fixed read-only manager producer and every execution/admission limit stay exact.

## Private original manager context join

The private consuming join retains the same original enrollment frame and one
fixed live manager channel. Actual retained receiver Origin and typed nsfs
user/mount/PID/time objects are compared with the channel view; caller context
claims cannot construct the production owner. Its light channel observations
enclose one unchanged complete frame, retaining every old live check, absolute
alarm and original deadline. Failure latches and owns the original frame,
children, writers and custody; no retry or former-owner getter exists.

Only new private runtime types are added, not a public/durable codec or start
operation. Separate test channels exercise shared frame ordering without
constructing a fixed owner. Actual read-only channel/context tests do not prove
production trust, message/FD delivery, start, all-writer cessation or completion.

## Private original uniquely addressed message

The private consuming message owner retains the original live manager join,
unchanged legacy message and complete frame. A second unsent message uses only
that connection's retained unique manager/client binding, canonical original
body and serial, and two distinct pinned writer aliases for the original pipes.
The extra aliases close before original custody. The constructor consumes the
original frame directly, retaining the same fixed channel and receiver binding.
Pure preparation occurs inside one complete frame
with channel/context checks on both sides; later checks use that same round.
All original live predicates, origins, alarms and deadlines remain. Failure
retains and poisons the same owner, with no extraction, retry, reconstruction,
send or start capability.

No public/durable format changes. Same-process SCM_RIGHTS fixture delivery is
not negotiated system-bus delivery, production enrollment, active start,
whole-writer cessation, completion or server admission.

## Two phase diagnostic launch requirements

The new data-only schemas diagnostic-two-phase-launch-requirements-074-v2,
diagnostic-two-phase-launch-074-v2 and diagnostic-signed-two-phase-launch-074-v2
use the distinct hydracache-diagnostic-two-phase-launch-074-v2 signature domain
with a NUL separator and canonical JSON plus one newline. Body/envelope caps are
16384/32768 bytes; all nested objects deny unknown fields. Unknown versions,
noncanonical/duplicate input, malformed hashes/signatures and a mismatched
requirements digest refuse. Readers accept only this exact version; old v1
consent and collector-unit readers remain unchanged and reject the new envelope.

The outer signature binds the original v1 consent claims and three artifact
roles to expected held-launch semantics. The requirements projection is not a
TransientUnitSpec and does not reinterpret the old executable projection.
Supplied-key inspection proves no enrollment, live peer/time, armed watchdog,
original delivery, execution, whole-cgroup revocation, completion or fence
release. Signed timestamps do not renew the original 300/60/10-second windows.
The private owned-held-barrier-mechanics-074-v0 prototype uses one ephemeral
AF_UNIX SOCK_SEQPACKET pair and exactly 32 expected opaque nonce bytes. It
accepts no rights; receive flags are zero or the echoed MSG_CMSG_CLOEXEC policy
only. Other shapes/flags/nonces refuse. Its reader window is that original
socketpair lifetime, with no adoption, reconnect, downgrade or migration.
A future production barrier needs a distinct reviewed protocol; this prototype
cannot construct an original execution owner.

## Private prearmed guardian enrollment

hc074-prearmed-guardian-v1 uses one ephemeral AF_UNIX SOCK_SEQPACKET pair.
Packets are exactly 16 bytes: HCPG0741, kind byte (1 Pending, 2 Enroll, 3 Armed,
4 Refused, 5 Revoking), then seven zero bytes. Enroll requires one atomic
CLOEXEC pidfd and exactly one kernel SCM_CREDENTIALS record for the creating
PID/UID/GID. Notices require no rights and the guardian's actual credentials.
Only the echoed MSG_CMSG_CLOEXEC flag is allowed. Unknown versions, reserved
bytes, kinds, shapes, extra/truncated rights or credentials refuse. The parent
also retains its creating thread; SCM_CREDENTIALS itself does not prove a TID.

The guardian accepts one live direct child under local procfs inspection,
retains its pidfd before Armed, and treats a second message as revocation.
Revoking means signal attempted, not cessation. No ACK, terminal pidfd or EOF
unlocks the fence. The reader window is the original socketpair lifetime;
there is no reconnect, adoption, downgrade, migration or production reader.
This is not the historical original owner, trusted kernel host context,
whole-cgroup admission, signed launch or all-writer completion proof.

The private hc074-prearmed-owner-test-v1 carrier sends the fixed fixture JSON
schema with creating-owner credentials and one guardian pidfd, optionally a
target pidfd/read pipe and hidden write alias (at most four rights). A one-byte
D without rights marks a completed harness command. Only the owning nonroot
subreaper understands it during its temporary-directory lifetime; it grants no
production identity or authority. The old v0 barrier and signed v2 requirements
remain byte-identical.

## Private redundant guardian test carrier

hc074-redundant-owner-test-v1 is an owned nonroot test carrier, not a production
reader. Its fixed JSON contains schema, case, creating owner PID, selected
absolute BOOTTIME deadline, Pending and Armed. Actual kernel credentials must
name that creator. Exactly four CLOEXEC rights carry two guardian pidfds, the
target pidfd and read pipe; one explicit hidden write alias raises that to five.
Only the echoed MSG_CMSG_CLOEXEC flag is accepted; truncation, unknown schema,
extra fields/rights or foreign credentials refuse. D without rights marks one
completed harness command. Reader lifetime is the original private temporary
socket/directory, with no reconnect, adoption, downgrade or production consumer.
Both guardians still use unchanged hc074-prearmed-guardian-v1. This composition
adds no production protocol, original authority, completion or unlock operation.


## Attempt cgroup root enrollment packet

hc074-cgroup-root-enrollment-v2 uses exactly 104 bytes: HCRG0742 magic,
one kind byte (Enroll=1, Armed=2, Refused=3, Revoking=4), seven zero reserved
bytes, 32-byte attempt digest, little-endian root device/inode (8 bytes each),
little-endian absolute BOOTTIME deadline (8 bytes), and 32-byte original-context
digest. Digests must be nonzero, root IDs positive and deadline in 1..=i64::MAX.
Unknown version/kind, reserved bytes, invalid fields or any size change refuse.
The reader window is this version only; old guardian-v1 packets cannot upgrade
or downgrade. There is no migration, reconnect or new durable replay slot.

This package defines data-only bytes and private pinned cgroup2 mechanics, not
an authenticated transport, Armed capability or original-start permit. The
future Enroll transport must require exact creator credentials and three
atomic-CLOEXEC rights in directory/kill/events order, validate those roles and
retain the root before Armed. Replies carry no rights and must echo every bound
field. A model sender consumes its one send before the syscall boundary;
missing or malformed ACK cannot reset it. Context/deadline bytes are claims,
not live observations or permission to renew the original 300/60/10 bounds.
Existing v1 guardian, signed v2 launch and original worker projections remain
unchanged. Root kill submission or empty-at-read never authorizes completion,
fence release or server execution, including when external pipe aliases remain.

## Local cgroup root descriptor transport

The private ordinary-process reader of hc074-cgroup-root-enrollment-v2 accepts
only the existing 104-byte Enroll packet and exactly directory/kill/events
SCM_RIGHTS on an unnamed UNIX seqpacket pair. SO_PASSCRED is required before
send; exactly one SCM_CREDENTIALS must match the actual pair creator PID and
real UID/GID. No numeric caller credentials or old guardian-v1 fallback exist.
All received rights require atomic MSG_CMSG_CLOEXEC, exact access modes and
matching retained/named root-owned cgroup2 objects. Unknown ancillary data,
truncation, extra/missing/substituted rights, binding or owner drift refuse.
Every installed rejected right is closed. Both sides consume their single
operation before its syscall; uncertainty or absent ACK never permits retry.

This reader retains descriptors but emits no Armed or other acknowledgement.
It is not a post-clone-safe guardian, original-owner constructor or runtime
admission API. Replies, guardian credentials, original fence/timer, full-frame
join and durable completion remain separate gates. Reader compatibility is
only v2 as above, with no migration, reconnect or durable replay slot. Existing
packet claims, v1 direct-child guardian and original projections are unchanged.

## Active local cgroup root guardian reader

The private local guardian reader uses hc074-cgroup-root-enrollment-v2 without
changing its 104-byte HCRG0742 packet. It accepts one creator-authenticated Enroll
with three CLOEXEC directory/kill/events rights. Actual root-owned cgroup2
retained/named identities and empty unfrozen domain must match the inherited
snapshot before a rights-free Armed reply. The ordinary parent requires the
exact clone3 child PID, live pidfd, real UID/GID, binding and original timer.
Both requests precede waiting for replies. Missing/extra rights, malformed or
unknown bytes, truncation, substituted credentials/binding or replay refuse.
No Pending packet, downgrade, reconnect, migration or durable replay slot exists.

Each isolated child retains its root and the same original fence/timer OFDs.
Each additionally opens its own read-only events watch OFD on the bound inode;
notification consumption must not be shared between guardians. Owner pidfd,
timer or channel termination and observed root drift enter permanent revocation.
Only the retained original kill FD is used, including for late population.
Refused and Revoking replies echo the binding with no rights and are best effort
not completion receipts. No unlock, all-writer cessation or original Armed
capability follows. This local reader is not a production owner constructor,
held-unit projection, trust enrollment or server execution API. Original v1
direct-child predicates, original projections and old packet claims stay intact.

## Local root guardian creator cut carrier

The external local-only root fixture uses a test-only 104-byte HCRGCUT1 carrier:
eight magic bytes, one stage byte 0 through 5 and 95 zero reserved bytes. Stage
zero transfers exactly two guardian pidfds; later stages transfer no rights.
The harness checks exact creator SCM_CREDENTIALS, atomic CLOEXEC rights, packet
length, reserved bytes and truncation. Unknown versions or stages refuse; no
migration, durable replay, reconnect or production reader exists. Only an
explicit ignored libtest victim and owning local subreaper use this carrier.
This does not change HCRG0742, its reader, or the original guardian v1 protocol.

## Original frame binding for local root guardians

The internal HCRG0742 context digest uses the domain
hc074-original-root-context-v1 and a serde JSON tuple of actual owner IDs,
original state, origins, bounds, ten descriptor roles, collector/custodian
pidfd roles, fixed manager/client names, original serial, addressed writer
aliases and pinned root device/inode. The attempt digest uses
hc074-original-root-attempt-v1 and the original diagnostic identity.
These are opaque same-build bindings, not credentials or a new trust reader.
No persisted migration or cross-build replay is admitted.

The exclusive original context supplies its own fence and existing timer;
callers cannot provide a replacement deadline, context digest or owner PID.
Preparation and enrollment retain the unchanged complete validation rounds.
The crate-private pinned root verifies kernel objects but does not enroll a
production host or authorize a held unit. Local Armed still grants no execution,
completion or fence release. Existing v1 predicates and all source deadlines
remain unchanged. Root kernel controls and nonroot original frame controls are
separate gates, not proof of an end-to-end original privileged manager channel.

## Local owned original manager creator fixture carrier

The private local-only hc074-owned-root-creator-v1 JSON carrier is bounded below
16384 bytes and uses an unnamed Unix seqpacket pair. The preparing phase carries
no rights and inventories six root-owned fresh temporary scopes, a relay scope,
the original fence path and fresh cgroup device/inode. The ready phase repeats
these fields, adds four original origins/bounds and transfers exactly thirteen
CLOEXEC rights: stdout/stderr readers, two guardian pidfds, collector/custodian
pidfds, the controller relay pidfd, three dummy writer pidfds and three held
stdin writers (preventing natural EOF exit). Only the outside-alias negative
adds a fourteenth stdout writer. Exact creator kernel
credentials, fields, version, phase, rights and truncation are checked; unknown
values refuse. No migration, persisted replay or production reader exists.
This ignored fixture carrier is not original delivery, host enrollment,
completion, unlock or a new guardian protocol.

## Signed expected held placement v3

The additive diagnostic-held-placement-requirements-074-v3,
diagnostic-held-placement-074-v3 and diagnostic-signed-held-placement-074-v3
schemas bind complete historical v2 launch claims and the exact lowercase hex
encoding of the 104-byte HCRG0742 Enroll packet. Its attempt digest must equal
the unchanged hc074-original-root-attempt-v1 serde JSON identity tuple digest.
Armed, Refused and Revoking packets are not placement requests. No v1/v2 reader,
signature, packet or requirement changes; no upgrade or fallback exists.

Signing bytes are hydracache-diagnostic-held-placement-074-v3, one NUL byte and
the canonical newline-terminated body. The body action is project-held-cell-one.
The requirements digest covers canonical newline-terminated requirements JSON;
its launch_signing_message_sha256 covers the complete existing v2 signing
message, including its domain and NUL. Bodies/envelopes retain 16384/32768-byte
limits. Unknown/duplicate fields, noncanonical JSON, wrong domains/keys, changed
bindings and non-lowercase hex refuse. Inspection uses only a supplied key.

Expected placement derives system-hc074 followed by the full attempt digest and
.slice directly beneath system.slice, with the existing fixed cell-one unit as
its child. No caller supplies arbitrary paths, commands or a renewed deadline.
These are data-only expectations, not a live root, trusted issuer, held unit,
systemd request or original-owner capability. Actual placement, complete live
frame, issuer enrollment, barrier, writer cessation and completion stay separate.
All six runtime admission fields and renewal are false. No send/start, host
creation, completion or fence release follows from this new signature.

<!-- BEGIN HELD BUILD AUTHORITY V1 -->
## Purpose-bound held runtime build v1

The diagnostic-held-build-policy-074-v1 policy pins the exact source commit,
tree and root lock, repository 1217101761, builder identity and a separate key,
controller and expected epoch. Its independently supplied digest and the legacy
worker policy's independent digest are mandatory. Both policies must name the
same controller; the held builder, worker builder and controller keys differ.
This is static reviewed configuration, not live issuer enrollment or epoch proof.

The diagnostic-held-build-074-v1 body and diagnostic-signed-held-build-074-v1
envelope have the purpose original-host-and-held-launcher. The signature covers
hydracache-diagnostic-held-build-074-v1, one NUL and canonical newline-terminated
body JSON. Policy/envelope limits are 4096/65536 bytes; unknown/duplicate fields,
noncanonical JSON, unknown versions, invalid keys/signatures and mismatched pins,
epochs or recipe refuse. Hex is lowercase. There is no migration, fallback or
upgrade from the frozen worker receipt or any placement/launch v1/v2/v3 format.

The fixed Cargo 1.94.0 Linux GNU release recipe selects only the supervisor host
and diagnostic_held_launcher binaries, locked and without features. Observation
assertions bind clean before/after source/tree, exact tools and isolated build.
Signed digests/lengths bind two distinct ELF images, root lock and raw Cargo log.
Binary/lock/log bounds are 128 MiB/1 MiB/16 MiB; log records are at most 1 MiB.
Require the two exact package/target/source/output/profile records and one final
successful event; missing, duplicate or foreign binaries and trailing events
refuse. Other Cargo metadata is retained through the complete log digest.

The fixed role map is launcher=held_launcher and watchdog=collector=original_host.
It attests the trusted build job's assertion, not independent reproducibility,
actual role-file identity, an active collector, original owner or execution.
Checked output only compares content/expected digests. No start/send/renewal,
completion or unlock capability follows; all existing live checks and deadlines
remain separate. The launcher target is a reserved recipe name, not an installed
executable or permission to replace the original worker ExecStart.
<!-- END HELD BUILD AUTHORITY V1 -->

<!-- BEGIN LOCAL HELD LAUNCHER V1 -->
## Local inherited descriptor held launcher v1

HCHL0741 bootstrap is exactly 256 little-endian bytes in a read-only memfd with
exact SEAL, GROW, SHRINK and WRITE seals. Offsets: deadline u64 at 8; owner
PID/UID/GID u32 at 16/20/24; guardian PIDs u32 at 28/32; token32 at 40;
worker/config SHA256 bytes at 72/104; worker/config device/inode/length u64
triples at 136/160; three stdio device/inode pairs at 184/200/216; control pair
at 232. Bytes 36..40 and 248..256 are zero. Token is nonzero; process IDs are
distinct positive non-self IDs. Worker/config bounds are 128 MiB/64 KiB.

HCLR0741 Ready=1 and Release=2 are exactly 80 bytes: magic8, kind at 8, zero
bytes 9..16, token32 at 16 and bootstrap SHA256 bytes at 48. Unknown versions,
kinds, reserved bits, lengths, changed bindings, ancillary rights/truncation,
wrong owner credentials and queued extra packets refuse. Require one exact
SCM_CREDENTIALS matching the fixed AF_UNIX SEQPACKET peer and bootstrap owner.
Ready is not watchdog Armed, execution ACK or a completion assertion. EOF,
failed send and lost confirmation are uncertain and never permit a resend.

The new local diagnostic_held_launcher target uses only inherited descriptors
0..10: stdio, control3, worker4, config5, owner6, guardians7/8, alarm9 and
bootstrap10. Only stdio/config survive its one execveat with AT_EMPTY_PATH;
there is no pathname/command fallback. Worker argv is fixed --run with
/proc/self/fd/5, expected digest and frozen worker source. Environment is empty.
Helper refuses root/set-ID, capabilities or extra threads and sets no_new_privs.
Config is sealed; worker must be sealed or root-owned executable without group/
world write permission. Root is trusted for the latter, not independently fenced.
The inherited absolute CLOCK_BOOTTIME one-shot alarm is revalidated against the
unchanged supplied deadline; it is never consumed or rearmed by the helper.

HCLF0741 is a fixture-only eight-byte carrier with exactly four SCM_RIGHTS:
held pidfd, two guardian pidfds and stdout reader. Its one SCM_CREDENTIALS must
identify the externally launched fixture owner. An owned subreaper harness uses
it for real owner SIGKILL and exact orphan cleanup; it is not production IPC.

These formats have no migration, fallback or upgrade from worker, signed launch,
placement or guardian protocols. Existing formats/live checks/deadlines remain.
They add a local target to the previously reserved recipe, not a production unit,
installed path, enrolled authority or original-owner/full-frame join. Guardian
liveness is not arming/all-writer cessation. No Running/completion/unlock or server
admission follows from helper success or this local nonproduct fixture.
<!-- END LOCAL HELD LAUNCHER V1 -->

<!-- BEGIN HELD ROLE BINDING V4 -->
## Held role policy and receipt binding v4

diagnostic-held-role-binding-074-v4 and
diagnostic-signed-held-role-binding-074-v4 are separate canonical newline JSON
body/envelope formats, bounded to 40960/49152 bytes. The controller signature
covers hydracache-diagnostic-held-role-binding-074-v4, one NUL and body bytes.
The fixed action is bind-held-build-to-placement. Hex is lowercase; future
versions, unknown/duplicate fields, noncanonical documents, malformed signatures
and over-budget body/envelope/receipt inputs refuse.

The body contains the complete signed v3 placement, held_policy_sha256,
held_receipt_sha256 and authority_epoch. Both controller signatures must verify
under the same controller selected by the independently checked held policy.
The canonical signed held receipt digest, its policy binding and signature must
agree. Its exact launcher/host role hashes must match all placement expectations;
the held policy, receipt and placement consent epochs must agree, with the same
repository 1217101761. The retained v3 bytes bind the original attempt, consent,
root context and absolute deadline. No new replay slot or renewed deadline exists.

This is static cross-document inspection. It neither enrolls the supplied
policy/pin/epoch nor authenticates an original live owner, host, process, FD,
manager or collector. Exact placement-byte/content checks export no originals,
start/send/arm/release, Running/completion or fence-unlock capability. There is
no migration, fallback or upgrade from any worker, v1/v2/v3 or held-build signature.
Old schemas and signed bytes remain unchanged. Original trust/frame/object join
and independent held-key enrollment remain separate gates.
<!-- END HELD ROLE BINDING V4 -->

<!-- BEGIN HELD RUNTIME TRUST V1 -->
## Fixed held runtime trust input v1

diagnostic-held-runtime-trust-074-v1 is canonical newline JSON, at most 2048
bytes, read only from /etc/hydracache-perf/held-runtime-trust-074.json. Its sole
public opener requires root UID/GID and offers no path/key/pin override. Root,
ancestors and regular single-link leaf are retained with no-follow descriptors;
unsafe ownership/modes, replacement, metadata/content drift or malformed input
cause sticky refusal. There is no writer, default held key/pin or enrollment.

Fields are schema_version, repository_id, controller_key_hex,
held_builder_key_hex, held_policy_sha256, worker_builder_policy_sha256,
worker_policy_sha256 and policy_epoch. Hashes/keys are lowercase, epoch nonzero,
repository 1217101761. Unknown/duplicate/missing/null fields and noncanonical or
future documents refuse. The protected file independently selects the held key
and policy digest; held key must differ from controller, old worker builder and
worker issuer. The latter three keys and old builder-policy pin remain reviewed
constants. Incoming v4/policy documents cannot choose these trust selectors.

OriginalHeldTrust borrows the same SignedControllerInput and held source. Fresh
observations retain the actual controller key, original request digest and active
worker policy digest/epoch/revision/trust snapshot. Both sources and the original
kernel/authority chain are revalidated around policy inspection, with sticky
refusal of both borrows on any error. Optional v4 inspection also matches the
original reservation request, controller, identity, source, preset and actual
worker authority. This early read-only guard exports only static Inspection,
not a controller, policy/FD original, start/send/replay or fence-unlock capability.

This is not late consuming placement/role-object/full-frame admission, consent
receive, durable intent, root/process/Armed proof, production enrollment or server
admission. No migration/fallback or changes to old signed formats, live checks,
46 brackets or original 300/60/10 origins/deadlines are introduced.
<!-- END HELD RUNTIME TRUST V1 -->

<!-- BEGIN HELD ACTIVATION INGRESS V1 -->
## Explicit held activation environment ingress v1

The diagnostic_held_launcher argument --systemd-activation-v1 selects only a
metadata adapter. It requires exactly LISTEN_PID equal to its canonical decimal
current PID, LISTEN_FDS equal to 8 and LISTEN_FDNAMES equal to
release:worker:config:creator:root-a:root-b:alarm:bootstrap. The eight extra
descriptors correspond to inherited slots 3 through 10; stdio is separate.
Missing, duplicate, unknown, non-UTF8 or noncanonical fields/values and future
arguments refuse with exit 2; no legacy-entry retry or automatic detection.

The fresh single-threaded adapter removes only these three validated variables,
checks that the environment is empty and enters the unchanged HCHL0741 core.
The no-argument entry still rejects any environment. All original privilege,
content, FD, creator/guardian, alarm/deadline, peer and release checks remain;
no role descriptor is opened, duplicated, remapped or sent by metadata adaptation.
This adds no timer, renewed deadline or migration of HCHL0741/HCLR0741 bytes.

LISTEN metadata is not manager authentication, executable addressing or original
owner proof. Manager defaults must be suppressed by a separately admitted unit,
not silently cleared by this adapter. Complete pre-exec environment admission,
retained launcher addressing, original manager delivery, all-writer cessation
and server readiness are independent gates. No installation or service start is
enabled by accepting this metadata.
<!-- END HELD ACTIVATION INGRESS V1 -->

<!-- BEGIN BOOTSTRAP ENROLLMENT V1 -->
## Independent protected bootstrap selection v1

diagnostic-bootstrap-enrollment-074-v1 is canonical newline JSON, at most 2048
bytes, read only from /etc/hydracache-perf/held-bootstrap-enrollment-074.json
under the same retained root/ancestor safety checks as HeldTrustInput. Fields
are schema_version, repository_id, bootstrap_builder_key_hex,
bootstrap_policy_sha256, bootstrap_sha256, held_policy_sha256 and policy_epoch.
The repository is 1217101761; hashes and keys are lowercase. Bootstrap key must
be nonweak and distinct from controller, worker builder/issuer and held builder.
Digests are nonzero; held policy and nonzero epoch match the original protected
held source. Missing/duplicate/unknown/noncanonical/future input refuses.

Fresh held-source validation brackets protected snapshot comparison; unsafe
ownership/mode/link count, changed bytes or inode/ancestors cause sticky refusal
of both borrows. No path/key override, default pin, writer or key generation.
There is no migration, fallback or alteration of the held runtime trust v1.

This is independent protected selection, not a bootstrap build receipt, installed
executable, actual manager/process/FD or original-owner admission. No configuration
is installed or generated. Purpose-bound policy/receipt, original consuming join,
bootstrap execveat, writer cessation and server enrollment remain separate gates.
No start/send/release/retry/completion/unlock capability is exported.
<!-- END BOOTSTRAP ENROLLMENT V1 -->

<!-- BEGIN BOOTSTRAP BUILD V1 -->
## Signed bootstrap policy and build receipt v1

The independently pinned bootstrap selector selects the complete canonical
newline JSON signed-policy bytes, not an incoming key. The policy envelope
diagnostic-signed-bootstrap-policy-074-v1 contains schema_version, policy and
signature_hex. The policy body diagnostic-bootstrap-build-policy-074-v1 contains
schema_version, purpose, repository_id, builder_id, builder_key_hex,
controller_key_hex, authority_epoch, source (commit, tree, root_lock),
bootstrap_sha256, held_policy_sha256, held_receipt_sha256,
original_host_sha256 and held_launcher_sha256. Its domain is
hydracache-diagnostic-bootstrap-policy-074-v1 followed by NUL and canonical
newline body bytes. Maximum signed policy size is 4096 bytes.

The receipt envelope diagnostic-signed-bootstrap-build-074-v1 contains
schema_version, statement and signature_hex. The statement
diagnostic-bootstrap-build-074-v1 contains schema_version, purpose,
policy_sha256, authority_epoch, observation, bootstrap, root_lock and build_log.
Observation uses the held build v1 field shape with the exact separate bootstrap
build command. Digests contain sha256 and bytes. Its domain is
hydracache-diagnostic-bootstrap-build-074-v1 followed by NUL and canonical newline
statement bytes. Maximum receipt size is 65536 bytes. Signatures use strict
Ed25519 and lowercase 128-character hex. Purpose is
original-sealed-launcher-bootstrap; builder is hydracache-held-bootstrap-074-v1.

Both schemas reject unknown/future/duplicate/missing/noncanonical fields,
wrong signatures/domains/pins/keys/epoch and zero or over-budget digests.
Older readers do not accept these new artifacts; no old schema is reinterpreted.
There is no migration, default key, installed configuration or execution right.
Original-owner join is runtime-only: it consumes the existing material owner,
retains the protected source and sealed image, checks original held policy/receipt
and host/launcher digests inside unchanged complete frames, and retires that same
owner on refusal. Static assertions and model tests do not admit original delivery,
child credentials, execveat, all-writer cessation, completion or server operation.
<!-- END BOOTSTRAP BUILD V1 -->

<!-- BEGIN BOOTSTRAP ENTRY V1 -->
## Bootstrap child-entry assertions v1 (not enabled for delivery)

HCBT0741 is exactly 1024 little-endian bytes, size u32 at 8, zero padding at
12..16 and 220..224. Offsets: owner[6] u64 at 16, origins[4] at 64, bounds[4]
at 96; attempt/context SHA256 at 128/160; root device/inode/deadline at 192;
nonzero serial u32 at 216; name digests[2] at 224; document digests[7] at 288
(five original documents then bootstrap policy/receipt); image identities[6]
at 512 (host, launcher, bootstrap, worker, sealed config, legacy256); stream
identities[4] at 656 (stdin, stdout, stderr, child release); guardian pid/dev/ino
triples[2] at 720; image digests[5] at 768 (host, launcher, bootstrap, worker,
config); legacy digest at 928, parent-local census fingerprint at 960 and
canonical newline DiagnosticState digest at 992. Names hash UTF-8 bytes without
newline. All digests are raw SHA256. Identities are dev/ino/size u64 triples,
stream pairs dev/ino. Deadline is the original minimum, never a new window.

Wrong length/magic/size/padding, invalid integer shape, zero serial, mismatched
minimum/root deadline or legacy size refuse. Structural decode is NOT
authentication; shape-valid mutations remain assertions. The parent compares
every byte against its own freshly validated original sources and retains the
same readonly/CLOEXEC/four-sealed inode and offset inside unchanged full frames.
The parent fingerprint binds its descriptor values/roles/image stamps; it is
NOT a child FD remapping table, signature or transferable permit.

This is an additive, currently non-delivered format, not a reinterpretation of
HCHL0741. Old readers must refuse it; no migration/default/fallback. It grants
no child credential, manager/guardian authentication, execveat, original start,
writer cessation, completion or server admission. No child-reader/binary or
transport is enabled by this schema.
<!-- END BOOTSTRAP ENTRY V1 -->

<!-- BEGIN BOOTSTRAP CHILD READER V1 -->
## Bootstrap entry reader v1 with authentication closed

The separate diagnostic_held_bootstrap binary accepts exactly one argument,
--systemd-bootstrap-entry-v1, and only LISTEN_PID (its canonical decimal PID),
LISTEN_FDS=12 and LISTEN_FDNAMES=release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry.
These metadata fields are assertions, not manager or FD authentication. Proposed
slots 0..10 preserve held launcher semantics; host/launcher/bootstrap-image/entry
are 11/12/13/14. No parent remapping or transport is enabled. Only entry FD 14
is inspected in this implementation, not the complete inherited-role census.

Entry FD must be regular, readonly/non-O_PATH, exactly four-sealed and 1024 bytes,
with descriptor flags zero as inherited across exec and initial offset zero.
Bounded positional reads compare metadata/status/seals/offset before and after;
HCBT0741 structural decoding returns assertions only. A second fresh observation
must match. Unknown/missing/duplicate/noncanonical environment, extra/future
arguments, missing FD, wrong shape/size/access/seals or metadata drift refuse.
No environment, flag, deadline or descriptor repair, reopen, fallback or retry.

All paths, INCLUDING canonical forged records, exit 2 without start/send/execveat;
successful reading reaches explicit original-authentication-closed refusal.
Unsupported targets also refuse. Old readers reject this new mode; no migration
or implicit activation. Actual credentials, manager/guardians/root and every
inherited role, signed build enrollment and original owner connection remain
required before delivery/execution or server admission. Reader success is not
an authenticated entry, handoff receipt, completion or all-writer cessation.
<!-- END BOOTSTRAP CHILD READER V1 -->

<!-- BEGIN BOOTSTRAP CHILD CENSUS V1 -->
## Bootstrap complete role assertions v1 with authentication closed

--systemd-bootstrap-census-v1 is a separate opt-in mode of the same bootstrap.
It uses the unchanged three activation assertions and twelve role names, not
entry-mode fallback or retry. The earlier --systemd-bootstrap-entry-v1 behavior
and HCBT0741/HCHL0741 bytes remain unchanged; older binaries reject the new mode.

The census checks all slots 0..14, with inherited flags zero: EOF empty stdin,
distinct nonblocking output pipes, empty AF_UNIX/SOCK_SEQPACKET release with
SO_PASSCRED and matching SO_PEERCRED, image/config/legacy identity and bounded
content hashes, live creator/guardian pidfds using fixed procfs fdinfo, and the
same absolute BOOTTIME alarm. Regular material inodes must be distinct; pidfd
inodes may alias on older kernels. Host/launcher/bootstrap/config/legacy/entry
require exact seals; original worker also permits root-owned nonwritable ELF.
Entry revalidation brackets two complete role observations. The procfs FD set
must contain exactly 0..14 plus the enumerator itself; extra descriptors refuse.
Enumeration, fdinfo and content reads have fixed bounds and never repair inputs,
consume queued traffic, create a runtime timer, send, start or exec.

Matching claims is not original provenance: current executable/credentials,
initial namespace, manager/root/cgroup/privileged guardian and signed actual
build/policy enrollment remain unauthenticated. The parent fingerprint is opaque,
not a child remapping map. Even a complete canonical forged role set exits 2 at
authentication-closed; all mismatches also exit 2 without an execution handle.
Unsupported targets refuse. No installation, default activation, migration,
original owner transfer, writer-cessation or server admission follows this mode.
<!-- END BOOTSTRAP CHILD CENSUS V1 -->

<!-- BEGIN BOOTSTRAP CURRENT IMAGE V1 -->
## Bootstrap current executable consistency v1

--systemd-bootstrap-image-v1 is a separate opt-in refusal-only bootstrap mode.
Older binaries reject it. Entry-only/census-only modes, HCBT0741/HCHL0741 bytes,
activation assertions and role names remain unchanged. The new mode repeats
the complete census, with two current executable observations around its old
revalidation. Fixed procfs thread-self/exe lookup must identify the same sealed
readonly regular inode as bootstrap FD13 and its asserted device/inode/size.
Metadata, seals, flags and zero offsets are checked without reading executable
content again. Owned inspection FDs close before the next closed-set census.

This sequential image consistency does not enroll procfs namespaces, credentials,
signed build/policy or original manager/root/guardians. A matching forged context
still exits 2 with authentication closed; mismatches also exit 2. No send/start,
launcher handoff, role repair, default activation, installation, migration,
writer cessation or server admission is enabled. Old absolute windows remain.
<!-- END BOOTSTRAP CURRENT IMAGE V1 -->

<!-- BEGIN BOOTSTRAP INSTALLED ORIGIN V1 -->
## Parent retained installed bootstrap origin v1

The opt-in parent wrapper consumes the original BootstrapOwnedUnit and retains
/opt/hydracache-perf/bin/diagnostic-held-bootstrap-074 independently from its
unchanged sealed reference. Only the privileged parent opens that fixed path.
Root UID/GID ancestry must allow owner/other search without special/group/other-write
bits; the executable must be a single-link root-owned regular file, mode 0555
or 0755. All retained opens are readonly/CLOEXEC/nofollow/nonblocking, with
positional bounded ELF/hash reads against the same verified bootstrap receipt.
Name, inode, metadata, flags, zero offset and ancestry drift latch refusal.

Parent-local census extends the old 27-FD/15-role prefix to 32 FDs/16 roles:
four ancestry FDs and the distinct installed image; no descriptor or owner export.
Three old complete bootstrap frames bracket two installed inspections. Failure
retires the same owner before added descriptors close; missing retained source
cannot be recreated. Old absolute origins/windows and child modes/record bytes
stay unchanged. This fixed-origin contract is not an enabled child wire v2,
ExecStart, installation, child namespace/credential enrollment, release, writer
cessation, completion or server admission. Older binaries do not inspect this
additional origin; no migration, default activation or weaker fallback exists.
<!-- END BOOTSTRAP INSTALLED ORIGIN V1 -->

<!-- BEGIN BOOTSTRAP INSTALLED ENTRY V1 -->
## Installed bootstrap entry assertions v1

HCBI0741 is a separate opt-in canonical 1280-byte assertion envelope, not an
authenticated message or execution permit. It embeds unchanged HCBT0741 bytes
at 16..1040, installed device/inode/size at 1040, content SHA-256 at 1064,
fixed pathname SHA-256 at 1096, opaque parent inventory SHA-256 at 1128,
little-endian parent counts 32/16 at 1160/1164 and embedded-entry SHA-256 at
1168. Header padding 12..16 and trailing padding 1200..1280 must be zero.
The fixed pathname is /opt/hydracache-perf/bin/diagnostic-held-bootstrap-074.
Counts describe the parent inventory, not inherited child FD slots.

Installed identity components must be nonzero, size at most 128 MiB and equal
to the sealed reference size, content hash equal to that reference, and its
device/inode pair distinct from every embedded regular-image identity.
Unknown magic, length, padding, counts, path digest, embedded digest, embedded
shape or cross-image mismatch refuses. Old/new readers reject each other's
records; there is no fallback, migration or changed old child mode.

All fields remain untrusted assertions, including original owner, times and
inventory fingerprint. Canonical consistency does not establish signed policy,
filesystem/custody, credentials, namespaces or manager authentication. This
component adds no record materialization, child inventory remapping, transport,
start/send/exec/release, writer cessation, completion or server admission.
Original absolute windows, old record bytes and live validation stay unchanged.
<!-- END BOOTSTRAP INSTALLED ENTRY V1 -->

<!-- BEGIN BOOTSTRAP INSTALLED MATERIAL V1 -->
## Parent retained installed entry materialization v1

The opt-in consuming wrapper retains HCBI0741 under the same InstalledBootstrapUnit.
It accepts no caller bytes/path/FDs or replacement owner. Original HCBT0741 is
read from its retained readonly/CLOEXEC/four-sealed zero-offset object with
stamp and exact prior-census brackets. Its installed prefix must match the old
27-FD/15-role census. A separate domain-separated parent fingerprint covers
fixed-order roles/FDs and original-entry, installed-image and ancestry stamps.
This is a parent-local assertion checksum, not a signature or child remapping.

The old sealed constructor supplies one readonly/CLOEXEC/four-sealed 1280-byte
memfd with no writable alias at publication and zero retained offset. Complete
installed-owner frames bracket two new inspections; each compares exact bytes,
retained metadata/flags/offset and repeated original expectations. Parent census
extends the exact 32-FD/16-role prefix to 33/17, rejecting old FD/inode aliases.
HCBI0741 bytes still describe its input 32/16 prefix, without a self-reference.
Any error latches the same owner; missing retained resource cannot be recreated.
Constructor failure keeps the refused owner, and retirement precedes closing
published retained descriptors. Internal sealed-constructor temporaries retain
their existing cleanup behavior.

Old record formats, child modes, predicates and absolute windows are unchanged.
Older binaries lack this optional consuming join; no migration, fallback or
default activation exists. This is not durable completion, child transport or
FD export, original manager/sandbox authentication, send/start/exec/release,
writer cessation, lifecycle progression or server admission.
<!-- END BOOTSTRAP INSTALLED MATERIAL V1 -->
