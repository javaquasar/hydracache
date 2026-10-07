# HydraCache 0.74 RESP/native performance evidence

This directory began W0/W1 from the exact frozen 0.73 product candidate
`16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b`. Release 0.73 is now published: annotated tag
`v0.73.0` points to `d1db9937e61295341ac95f289bace275641b1650`, whose product runtime is
verified unchanged from the measured candidate. `B73` deliberately pins both identities plus the
immutable confirmation archive commit `570a5bcb6959ecc7f01f8c80d0fc32b719832ad9`; W0 is closed,
but local 0.74 numbers remain non-promotable.

The scenario matrix keeps RESP, HC/1, HC/2, direct `ClientSurfaceState`, raw embedded
`HydraCache`, and typed embedded `HydraCache` results separate. The three execution tiers are
also separate: `local-quick` is a developer smoke, `local-attribution` is non-promotable D1
evidence, and `release-qualification` requires explicit authorization on an admitted host.

Run the structural gate locally with:

```text
cargo xtask performance-contract-check --release 0.74
```

The gate validates the frozen dimensions, native non-regression rules, workload-equivalence
fields, proposal isolation, required metrics, and the published predecessor identities. It must
still fail if asked for ship admission while the 0.74 candidate, Redis provenance, dedicated-host
qualification, or release evidence is incomplete.

The first non-promotable local attribution and the rejected W2/W3 results are documented in
[`w1-local-attribution.md`](w1-local-attribution.md). Raw identity-bound receipts are retained under
`local-runs/`.

The W9e allocator deferral is now resolved by a dedicated-Linux owner profile. The counterbalanced
20-attempt system/mimalloc/jemalloc matrix found real churn-phase CPU reductions, but every
alternative failed unchanged memory, binary-size or CPU guards; no allocator or purge candidate was
admitted and the system allocator remains default. See
[`w9e-linux-allocator-attribution.md`](w9e-linux-allocator-attribution.md).

The W9c kernel gate is also closed. A paired untraced/`strace` Linux matrix reconciled every RESP
byte and confirmed one server write syscall per reply even at pipeline 10, while finding no server
short-write, EAGAIN, queue-pressure or separately tunable scheduler owner. It admits no new product
candidate and leaves platform defaults unchanged. See
[`w9c-linux-kernel-attribution.md`](w9c-linux-kernel-attribution.md).

The standalone `tools/resp-stage-profile-074` tool isolates decode, translation-context,
command/translation and response/encode allocation owners. Its control stages permit only local
incremental attribution; they are not interchangeable with end-to-end product receipts.

The focused WSL2 portability result is recorded in
[`local-linux-sanity.md`](local-linux-sanity.md). It is a non-promotable `local-quick` sub-tier and
does not substitute for the admitted Linux release host.

The controller-resilience contract is frozen in
[`long-run-controller-resilience-contract.toml`](long-run-controller-resilience-contract.toml).
The first local implementation slice, `tools/long-run-supervisor-074`, writes and independently
verifies canonical hash-chained checkpoint envelopes, rejects identity/timestamp/hash drift, and
recovers at most one incomplete trailing line. A later local slice adds atomic revision-checked
`state.json` replacement, `state.previous.json`, exclusive campaign locks and the first integrated
0.74 harness component. That component refuses to append a checkpoint until the phase-aware
watchdog accepts it, and it cannot reuse an existing or torn journal as a new process lifetime.

The Linux executable now also provides a real `SOCK_SEQPACKET` transport and authenticated
read-only status and host-observation services. Packet boundaries and the 65,536-byte limit are
enforced by the socket layer; `SO_PEERCRED` plus `/proc/<pid>/status` supply uid, primary gid and
supplemental groups.
The wire envelope requires a strict Ed25519 authorization document for mutating operations, while
both read-only operations still check the admitted peer, repository and actor. Host observation is
additionally revision-zero, pathless and fixed to sentinel scope. The client
verifies the response digest and request/campaign binding before printing it. End-to-end WSL tests
round-trip the exact durable state and reject stale revisions.

At that implementation stage this was not yet the complete release supervisor. The initial
revision-zero `start`
authenticates, imports immutable evidence, obtains the host-wide claim, advances I74 and dispatches
through the production-form systemd backend. After an exact `I74_SEALED` revision, a second
pathless `start` reuses that immutable evidence and advances C74 with independent spawn evidence;
it cannot skip the seal or use a stale revision. `seal` and `abort` still fail with stable internal
error 11.
`attach` executes the
implemented journal, manifest, systemd, `/proc`, cpuset, checkpoint, admitted-host and lease guards
and durably records an exact response. There is no unconditional rejection left in the attach path:
a lease can be granted only when every guard matches. Production provisioning, bounded diagnostics,
an accepted attach rehearsal and real host fault rehearsals were still incomplete at that point;
the later evidence below records their isolated closure while release admission remains closed.

The next local slice adds the repository-side service definition, sysusers/tmpfiles definitions,
a deliberately invalid configuration template and `Type=notify` readiness support. The socket is
changed to the exact configured client-group gid before `listen`, avoiding a nominal `0660` socket
that is still unusable by the runner. Static tests require the confinement properties and reject a
shell, sudo, `systemd-run` or inherited environment file. These are reviewed provisioning inputs,
not evidence that they have been installed on an admitted host; `systemd_confinement_complete`
therefore remains false.

Linux process identity inspection is also implemented independently of PID liveness. It parses
`/proc/<pid>/stat` after the final command-name parenthesis, reads the single unified cgroup row,
binds the cgroup inode and boot id, and compares start ticks, process group, cgroup path/inode and
unit component. Tests cover command names containing spaces/parentheses and report all concurrent
identity mismatches. Checkpoint head verification and durable request replay are now implemented
as separate fail-closed components and are composed with exact systemd unit state and immutable
manifest evidence in the live request path. Full host-receipt revalidation is now a separate strict
component rather than an unimplemented rejection.

The supervisor event journal uses a distinct hash domain, canonical JSON lines, fdatasync before
head replacement, a strict 64 MiB local bound and a durable request-id index. An identical request
returns the recorded signed response without appending; reuse of an id with different canonical
request bytes fails closed. Accepted attach events carry the complete post-mutation state, so the
journal remains authoritative if the service crashes after syncing the event but before replacing
`state.json`. Recovery completes exactly that one compare-and-swap window; same-revision drift,
snapshot-ahead state and larger revision gaps are treated as corruption rather than repaired from
`state.previous.json`.

Checkpoint admission now verifies the bounded regular JSONL file, exact head file, campaign, role,
sequence, head digest and both process identities against durable state. A syntactically complete
JSON object without its terminal newline is treated as a torn final record; this closes the case
where a later append could otherwise join two objects on one line. Future useful-progress times are
also rejected instead of extending the frozen 180-second deadline.

Persistent manifest admission reconstructs the durable frozen identity from the canonical
`campaign-start.json`: tooling, both source/tree/lock identities, installed binary metadata,
workload/statistics inputs, phase/cadence/limit policy and host identity are reduced through
deterministic domain-specific bundles and compared with state. The separate manifest digest file,
regular-file/link constraints and original request digest must all agree. This prevents a locally
self-consistent state snapshot from substituting for the immutable start evidence.

The systemd adapter talks directly to the system bus and resolves only names in the fixed
`hydracache-performance-074-*.service` namespace. It binds `ActiveState=active`,
`SubState=running`, `MainPID`, `ControlGroup` and `Result=success` to both durable process
identities. A separate local WSL test read those properties from a real loaded service over D-Bus;
it did not create, stop or restart a unit. Live attach also re-reads both `/proc` identities and
their `Cpus_allowed_list`, verifies the checkpoint evidence and runs the pure lease predicates in
one locked transaction. Rejections name all independently discoverable failures, are hash-chained,
and replay byte-exactly for the same request id.

The admitted-host receipt closes the last deliberately unimplemented attach guard. A root-only
`collect-host-receipt` operation writes create-new `0400` canonical evidence and its digest sidecar.
The receipt binds the existing reference-host freeze digest, machine and boot ids, kernel release
and command-line digest, the exact campaign mount selected from `/proc/self/mountinfo`, online /
isolated / housekeeping CPU partition, every online CPU governor, the seven frozen kernel tunables,
and the root-owned installed supervisor binary's content and inode metadata. Attach re-collects the
same observation and requires byte-semantic equality with the persistent receipt, manifest and
durable state. Missing cpufreq, empty isolation, a non-root collector, WSL, a changed mount option or
any tuning/binary drift fails closed. This is local implementation evidence; no admitted host was
changed or exercised.

Offline packet verification is independently implemented in `xtask` (it does not call the
supervisor library). The strict packet and raw-manifest schemas bind the canonical campaign
manifest identity, the exact sorted raw file set, every file size and SHA-256, required guard
evidence, role results, process identities, and each journal's byte length, SHA-256, record count,
first record and hash-chain head. Unlisted files, traversal, symlink or hardlink substitution,
duplicate roles/guards, missing evidence, torn promotable journals and manifest/content drift are
rejected:

```text
cargo xtask long-run-campaign-check --release 0.74 --manifest <packet-manifest.json>
cargo xtask long-run-campaign-check --release 0.74 --manifest <final-packet-manifest.json> --continuation-manifest <i74-packet-manifest.json>
```

A one-role I74 packet must carry a null continuation digest. Every two-role final packet carries the
SHA-256 of the exact canonical I74 packet manifest, and the final offline command requires that
manifest as a separate input. It recomputes the digest, campaign/manifest identity and the complete
I74 role manifest; a valid-looking digest, a different continuation, or recaptured I74 journal is
rejected.

The supervisor library also creates the final archive without trusting directory iteration order or
filesystem metadata. It sorts bytewise-normalized paths, emits fixed uid/gid/mode/mtime tar
headers, uses deterministic single-stream zstd compression, writes an external SHA-256, enforces
the frozen file/byte limits, and refuses create-overwrite, nested outputs, symlinks, hardlinks and
non-regular files. Tests compare byte-identical archives built from differently ordered trees and
inspect every tar header.

Packet assembly now precedes that archive step. The supervisor copies an explicit bounded raw file
set through a create-new staging directory, derives every file size and digest, computes the
canonical raw-set hash, derives role record counts/first/head hashes and process identities from
the copied checkpoint journals, binds guard evidence by its copied digest, writes canonical
`raw-manifest.json` and `packet-manifest.json`, fsyncs the tree and atomically renames it. Complete
packets cannot contain an incomplete role or failed required guard, and promotable packets require
both I74 and C74. The independent `xtask` verifier accepts both a generated I74 continuation and a
generated two-role promotable packet. Two builds from separate roots produce identical manifests
and archive SHA-256. Published packets and archives are re-opened and verified before they are
returned. Archive publication now uses its own synced create-new staging directory and atomic
rename, so a size-limit or write failure cannot expose a final directory containing partial bytes.

The initial supervisor package passed all 30 targeted tests under local WSL2 Ubuntu at exact source
`37566d71`. The live-status slice at `1a29aef1` passed 43 supervisor tests plus three durable
checkpoint-writer tests. The provisioning/process-identity slice at `882b3ed2` passes 51 supervisor
tests plus the same three checkpoint-writer tests under WSL2. The durable
event/recovery/checkpoint slice at `35b2bc3f` passes 65 supervisor tests plus the same three
checkpoint-writer tests under WSL2. The persistent-manifest slice at `6759b322` passes 66
supervisor tests plus the same three checkpoint-writer tests. The composed attach-guard slice at
`1a484f6c` passes 72 ordinary supervisor tests, one explicit real-system-bus test and the same three
checkpoint-writer tests. The checked-in receipts remain explicitly local and
non-promotable: no service was installed, no product process was started and no admitted-host fault
rehearsal ran.

The host-receipt slice at `93710227` raises the local WSL supervisor total to 78 ordinary tests.
Windows formatting, tests and `clippy -D warnings` are also green. The real collector is
intentionally not promotable on WSL and no accepted lease claim is made until the same binary and
receipt are rehearsed on the admitted bare-metal host.

Start evidence preparation is now implemented separately from process launch. The importer accepts
only a revision-zero `start`, requires the staging campaign to be an exact direct child, rejects
symlink/hardlink/oversize/digest substitution, validates the manifest and host receipt together,
then writes create-new `0400` evidence into a private directory and atomically publishes the final
campaign directory after fsync. Repeating the import cannot overwrite the first result. At source
`5343ca1f`, this closed the immutable evidence-copy portion and raised the local Linux total to 82
ordinary supervisor tests.

The next start-lifecycle slice removes the remaining placeholder-state shortcut without invoking a
real process. PREPARED and `*_STARTING` snapshots contain no fabricated PID or checkpoint. A
host-wide flock plus persistent active-campaign marker admits recovery only for the same campaign.

The October 6 non-product recovery rehearsal also closed the terminal identity-mismatch cleanup
gap. Campaign `83b19a98…` recovered from revision 7 `CORRUPT_QUARANTINED` through two signed,
journal-bound abort authorizations and finished at revision 10 `ABORTED_INCOMPLETE`. The failed
empty C74 transient unit was reset and unloaded, the active marker was released by the state
machine, the supervisor recorded zero restarts, and the Actions runner was returned offline. See
`local-runs/w11-quarantine-recovery-83b19a98.json`; this is non-promotable resilience evidence,
not a product or throughput qualification result.

The fresh two-role rehearsal then closed the live-seal path at exact source `7a6b5b15`. Signed
provisioning run `37437254815` installed that source, and bundle run `37437612895` constructed
campaign `57c731a1…`, manifest `65c28087…` and immutable bundle `83301e4d…` from a new root-bound
host observation. I74 run `37437854001` performed signed start, waited for the exact successful
terminal unit, attached terminal evidence and sealed revision 5 `I74_SEALED`. C74 run
`37440450794` continued only from that exact revision and repeated the same sequence to revision 10
`COMPLETE_SEALED`. Both retained units were `active/exited`, `MainPID=0`, `Result=success`; all
failure, corruption and duplicate-executor flags were false. Final seal released the active marker,
the supervisor stayed active with zero restarts, and the runner was returned inactive and disabled.
The complete artifact-bound receipt is
`local-runs/w11-i74-c74-complete-seal-7a6b5b15.json`.

This closes the admitted-host live `start`, `attach`, `seal` and previously rehearsed `abort`
operations for the non-product fixture.

The next exact-source rehearsal closed the live-role reboot gate without running a product
candidate. Provisioning run `37453559280` installed source `b83fd1b7`; bundle run `37453956080`
created campaign `d5175725…`, manifest `b98e0468…` and bundle `a566374b…`; signed start run
`37454199519` reached revision 2 `I74_RUNNING` with the exact fixture pair and a durable checkpoint
chain. The Actions runner was stopped and left disabled before the host reboot. Boot ID changed from
`47f3c763…` to `e81671d6…`; the enabled supervisor returned under a new PID and classified the
retained identity as `host-identity-drift`. It committed `MEASUREMENT_LOSS_REQUESTED` and
`MEASUREMENT_LOSS_COMPLETED`, ending at revision 4 `FAILED_INCOMPLETE`, with `recorded_failure=true`.
The diagnostic is cause-bound, all process/checkpoint/lease identities are clear, the transient
role unit and process are absent, and the active marker was released. No replacement workload was
started. See `local-runs/w11-live-role-reboot-b83fd1b7.json`.

This is fail-closed reboot evidence, not support for resuming a measurement across reboot. It does
not close role-level overhead, product I74/C74 execution, the six-hour qualification or the
separately authorized 24-hour confirmation. `live_service_complete`, `host_rehearsal_complete` and
`release_admission_allowed` therefore remain false.

Role-specific canonical `spawn-intent` and `spawn-result` documents have create-new SHA-256
sidecars and deterministic unit names. The intent is durable before the backend call; after that
boundary every retry observes or adopts the exact unit and can never call start again. An absent
unit becomes FAILED_INCOMPLETE, identity mismatch is quarantined, and multiple executors set the
duplicate-executor guard.

Internal PREPARED, STARTING and spawn-outcome records now share the supervisor hash-chain with
external request records without consuming request replay identity. The I74 coordinator recovers a
missing or one-revision-stale state snapshot from an event-ahead crash window and verifies durable
spawn evidence on replay. Windows and WSL fake-backend tests cover normal start, repeated start,
lost response, crash before and after the side effect, missing/stale state, absent unit,
symlink/hardlink substitution and cross-campaign exclusion.

The subsequent start slice connects the authenticated socket request to that coordinator. It
revalidates already imported evidence on retry and journals the accepted response before reply, so
an identical request returns the exact recorded response without touching the backend. The
transient-unit policy contains an exact argv array with no shell, a digest-bound minimal
environment, unprivileged user/group, CPU affinity, runtime and resource limits, output paths and
hardening properties. The real backend issues `StartTransientUnit` in `fail` mode and observes the
original MainPID plus exactly one daemon in the unit cgroup; boot, group, cgroup, cpuset and
cardinality drift fail closed. The C74 coordinator now accepts only the exact `I74_SEALED`
revision, writes separate `c74-spawn-*` evidence, applies the same no-respawn recovery rule and
journals a byte-exact accepted response before reply. At source `286863e4`, the full local WSL
suite passes 114 ordinary supervisor tests with only the explicitly manual real-system-bus
inspection ignored. Windows `clippy -D warnings` is green.

The first sealing prerequisite slice keeps successful exit independently observable. Transient
roles use `RemainAfterExit=true`; terminal admission accepts only the retained
`active/exited`, zero-MainPID, `Result=success` unit with the original unit/cgroup identity and a
checkpoint chain whose final phase is `Terminal`. A running unit or a merely dead PID cannot
impersonate completion. Controller leases now retain the repository/run/actor values covered by
the authorization signature, so a later signed `seal` or `abort` can prove the same controller
principal even though its request digest is necessarily different. The active-host marker can be
removed only after a clean `COMPLETE_SEALED` state with no retained process, checkpoint or lease.
At source `f35e276c`, 117 ordinary supervisor tests pass under WSL; the real-system-bus test remains
manual and ignored.

The next local slice makes packet/archive creation recoverable across supervisor crashes without
claiming a live `seal` route. A canonical seal intent binds request id/digest, role, packet plan and
limits before artifact work begins. Canonical intent/result documents have synced SHA-256 sidecars;
a retry verifies and adopts an exact completed packet/archive, completes an exact `.building`
rename, repairs the narrow document-before-sidecar crash window, or rejects conflicts and tampering.
The result binds the packet/raw-manifest digests, raw set, file/byte counts and archive digest/size,
and exact replay re-verifies both published directories before returning it. At source `75e7df26`,
121 ordinary supervisor tests pass under WSL; one real-system-bus test remains manual and ignored.

The local Linux seal coordinator now orders terminal checkpoint/unit verification, terminal
lifecycle event, state CAS, recoverable artifact creation, sealed lifecycle event, state CAS and
durable signed response. It reconciles an event committed immediately before a crash, binds
recovery to the exact request digest, re-verifies artifacts on response replay, clears retained
process/checkpoint/lease state only at the sealed transition, and releases the host marker only
after a durable `COMPLETE_SEALED` response. Its I74 and C74 tests cover continuation and promotable
packets, request-id conflict, unit drift, event-ahead-of-state recovery and replay after host-marker
release. At source `cba4a336`, 125 ordinary WSL supervisor tests pass; the real-system-bus test is
still manual and ignored.

Sealed packet and archive publication is now immutable at the local Unix filesystem boundary.
Before the final rename, ordinary files are changed to `0400` and directories to `0500`; those modes
are synced and then required by every replay verification. Both builders re-open the final renamed
directory and compare all hashes/counts before returning a receipt. Tests deliberately restore
write permission only to model privileged tampering, after which verification fails closed. At
source `9b4d93a7`, the same 125-test WSL suite and six independent packet-verifier tests pass.

The next local seal-input slice removes directory discovery from the server adapter. Each measured
role must close a canonical `roles/<role>/seal-input-inventory.json` that explicitly names its
journal, exact raw files and one evidence file for every guard frozen in the campaign manifest.
Unknown fields, cross-role paths, missing self/campaign/journal entries, duplicate guards, symlink
or hardlink inputs and byte/file-limit overflow fail closed. C74 planning re-opens the durable I74
seal result, verifies the published read-only packet against the original I74 inventory and checks
that every still-present I74 source byte equals the sealed copy before it can combine the two
inventories. Unlisted campaign files are never swept into an artifact. The independent final-packet
verifier also compares the complete I74 role manifest with the supplied continuation rather than
accepting a matching top-level digest alone.
At source `3e39644d`, 129 ordinary WSL supervisor tests, seven packet/inventory verifier tests and
16 performance-contract tests pass; the real-system-bus test remains manual and ignored. The
machine-readable local receipt is
`local-runs/w11-seal-input-resolver-wsl-20261005.json`.

This remains local non-promotable evidence. No supervisor service, transient measured unit or
HydraCache process was started. Production account/directory ownership, real controller and
supervisor restart, controller loss and bounded overhead still require the admitted Linux host.
At that source the live server still lacked the abort route, diagnostics and lease-expiry
termination; the later slices below close those local code paths. The signed live `seal` route now
revalidates persistent manifest/host evidence, observes the retained unit through the production
D-Bus path, derives the exact inventory-bound plan and invokes the recoverable coordinator. A
dedicated root-owned seal root prevents the unprivileged measured role from publishing artifacts.
Local socket integration covers rejection before host claim, terminal-state commit followed by an
artifact failure, same-request recovery, durable I74 seal and exact response replay. Production
uid/gid ownership and the real system bus still need admitted-host rehearsal; no real service claim
is made until that rehearsal exists.
At source `8d673777`, the full WSL suite passes 132 ordinary tests with one manual system-bus test
ignored. See `local-runs/w11-live-seal-dispatch-wsl-20261005.json`.

The measured-process side now closes the remaining local producer gap. After verifying the
terminal checkpoint chain, `tools/performance-integrated-074` binds the on-disk campaign manifest,
role and campaign directory; requires exactly the guards frozen before the run; and writes one
canonical guard-result document for each guard. Every guard result names at least one already
durable role-owned evidence file. The writer constructs the exact sorted raw set, applies the
frozen file/byte limits and publishes `seal-input-inventory.json` last. Publication uses a synced
create-new pending file plus an atomic same-filesystem link, so recovery can finish either the
pre-link or post-link/pre-cleanup crash window but can never replace divergent bytes. Cross-role,
traversal, symlink and hardlink evidence, a failed guard in a complete result and manifest drift
all fail before an inventory becomes visible. The positive integration feeds the published bytes
directly into the independent strict resolver. At source `44f9270b`, nine Windows tests, the same
nine WSL tests and strict Windows clippy pass. See
`local-runs/w11-terminal-inventory-writer-wsl-20261005.json`.

This completes the local producer-to-seal evidence path, not the admitted-host claim. The actual
qualification harness still has to call this library with its real semantic, native
non-regression and retention evidence, and production ownership/system-bus behavior remains a
host rehearsal item.

The explicit abort path now has a recoverable local lifecycle and authenticated socket dispatch.
An admitted request must carry the frozen reason, approval nonce, exact revision and the same live
controller principals. The supervisor commits `ABORT_REQUESTED` while retaining the process
identities, then calls one narrow idempotent backend boundary for bounded diagnostics and stopping
the exact unit. Only after that call succeeds does it commit `ABORT_COMPLETED`, clear the retained
process/checkpoint/lease state, record the signed response and release the host claim. A backend
failure after the first commit returns the reconciled revision; the byte-identical request resumes
from that intent and a later replay does not call the backend. Stale or unauthorized requests leave
no event and perform no effect. At source `108b96bb`, the local lifecycle and socket tests cover
normal completion, exact replay and the failure-after-intent window. See
`local-runs/w11-abort-dispatch-wsl-20261005.json`.

The ordinary service route now supplies a production abort backend. It accepts only the exact
0.74 I74/C74 transient-unit name bound to the campaign, verifies the retained process pair against
the current D-Bus snapshot, and publishes a canonical diagnostic containing only frozen state and
the allowlisted unit snapshot. The manifest diagnostic-byte limit is enforced. Publication is
create-new, synced and recoverable across both hard-link crash windows. Only after that evidence is
durable does the backend issue `StopUnit`; it accepts completion only after the unit disappears or
becomes `inactive/dead` with no MainPID. A retry with an already absent unit requires the exact
existing diagnostic. At source `879012c8`, local diagnostic replay/limit/tamper tests and namespace
rejection pass; see `local-runs/w11-systemd-abort-backend-wsl-20261005.json`.

No live unit was stopped. The real system-bus call, root/performance-account ownership and timing
behavior still require the admitted-host rehearsal, so this remains local non-promotable evidence.

Product-lease expiry is now driven by the supervisor itself rather than by a controller request.
The `SOCK_SEQPACKET` listener wakes at a bounded one-second interval, recovers only the campaign
named by the host-locked `active-campaign` marker, and compares the current time with the immutable
deadline. At the first second beyond that deadline it commits `LEASE_EXPIRY_REQUESTED` while
retaining process and checkpoint identity, publishes a cause-bound bounded diagnostic through the
same exact-unit systemd backend, stops the unit, then commits `LEASE_EXPIRY_COMPLETED`, clears the
retained execution state and releases the host claim. Failure after the first commit leaves enough
identity for an identical automatic retry; a restart after the completion commit can finish marker
release without repeating the external effect.

The expired path reopens the original canonical manifest by its stored digest and reconstructs the
complete frozen identity. It does not weaken validation by pretending the already expired deadline
must still be in the future. The expiry diagnostic is independently bound to campaign, lease id,
deadline, role, durable state and the allowlisted unit snapshot; an absent unit is accepted only
when those exact bytes already exist. Local WSL coverage now totals 147 passing ordinary supervisor
tests with one intentionally ignored manual real-system-bus test. The server integration test
proves no effect at the exact deadline and completion on the following second without any incoming
request; the parallel server suite also passed five consecutive repetitions. See
`local-runs/w11-lease-expiry-heartbeat-wsl-20261005.json`.

This closes the local implementation flag, not the host qualification. No real `StopUnit` was
issued, no product process was started, and system-bus authorization, account ownership, actual
one-second wake-up timing and diagnostic-grace behavior remain admitted-host rehearsal work.

The controller boundary now has a production-form request builder and a staged protected workflow.
`build-request` accepts only start/attach/seal/abort, an already strict unsigned request, a private
single-link Ed25519 key file and a maximum ten-minute authorization window. It replaces only the
required all-zero authorization placeholder, signs the exact request identity, re-parses and
cryptographically verifies the completed packet, and publishes it create-new with mode `0600`.
Existing output, an insecure or hardlinked key, a non-mutating operation and an oversized time
window fail closed. The private key path and bytes never enter the packet or command output.

`.github/workflows/performance-long-run-qualification-074.yml` is manual-only and serialized by
the admitted host id with cancellation disabled. A GitHub-hosted protected job builds the signed
packet; the self-hosted job receives only that immutable packet, sends one typed operation to the
Unix socket, and always attempts a separate read-only status snapshot. The workflow has no retry
loop and no `continue-on-error`. Its structural contract has 17/17 passing tests, and request
builder tests pass on Windows and WSL. See
`local-runs/w11-controller-workflow-local-20261005.json`.

This workflow is staged, not authorized or executed. It deliberately does not resolve the release
verification key, candidate SHA, admitted host, product build/staging, qualification runner or
expensive phase commands in `qualification-manifest.toml`. Those remain red boundaries rather than
placeholders that could accidentally start a campaign.

The same local crate now contains the pure campaign state machine, request replay map and attach
predicate evaluator. Deterministic tests prove that attach changes only controller lease/revision,
cannot spawn or restart a role, and rejects host/boot, PID start, cgroup, checkpoint, lease,
revision, duplicate-executor, recorded-failure and durable-history drift.

The phase-aware watchdog is also implemented as a deterministic supervisor component. A new
checkpoint sequence or live PID alone does not reset its deadline: measured work must advance
completed operations, per-surface accounting and process CPU time together; drain must reduce the
backlog; post-work idle must advance telemetry without changing operation counts; and
reconciliation must advance its milestone, epoch or owner state. Identity drift, skipped phases,
counter regression and the frozen 90/180-second warning/rejection boundaries fail closed. Wiring
this evaluator to a live systemd-owned process still remains host work.

The versioned request/response protocol is also locally implemented with the frozen 65,536-byte
limit, strict unknown/duplicate-field rejection, UUIDv4 and lowercase-digest validation,
operation-specific fields, the single exact staging path for `start`, and canonical response
digests. It intentionally exposes no argv, environment, shell, DBus or arbitrary-path field.

The Ed25519 authorization verifier binds the signed document and its raw SHA-256 to request,
operation, campaign, manifest, repository, run and actor identities and enforces a ten-minute
maximum lifetime with bounded clock skew. The production verification key remains explicitly
`UNRESOLVED`; local test keys cannot authorize a campaign and release admission stays closed.

The Rust supervisor foundation also independently parses the create-new start manifest. It
requires canonical strict JSON, binds the raw manifest digest, campaign and repository to the
request, freezes timing/limit fields, rejects plaintext-looking secret identifiers, and refuses
expired leases. The full design manifest is now represented by a strict schema and independently
validated by the Python builder and Rust parser: source/tree/lock identities, clean-tree assertions,
installed binary metadata, argv/environment identity, role order, phase durations, workload and
statistics digests, output limits, output-schema digests and required final guards all fail closed.
This completes the manifest data contract only; it does not implement process launch or prove the
installed-file metadata against a Linux host.

`scripts/perf/performance_long_run_074.py --prepare` builds only a create-new start manifest. It
uses the frozen campaign-id component order, removes the raw nonce after deriving its digest,
rejects unknown, nested-schema, floating-point, dirty-tree, path/argv-binding, oversized-output and
secret-bearing fields, fsyncs both files, and has no process execution mode.

The same tool now assembles and independently re-verifies an immutable start bundle before any
privileged host staging occurs. The bundle contains the manifest, admitted host observation and
their exact digest sidecars plus a canonical inventory binding every transported byte and size.
Assembly checks the manifest-to-receipt digest and machine/boot/mount/cpuset identity, rejects
non-regular, symlink/hardlink, oversized, non-canonical, changed or pre-existing inputs, publishes
all files create-new and syncs the directory on Unix. Re-verification re-hashes both the outer
inventory and all four nested files after transport. Windows and WSL2 each pass 10/10 builder tests,
including host drift, tamper, hardlink and overwrite failures. This closes the unprivileged
controller artifact boundary. See
`local-runs/w11-start-bundle-local-20261005.json`.

The supervisor-owned delivery adapter is now implemented without granting the runner filesystem,
`sudo` or DBus authority. For a revision-zero start, the client revalidates the transported bundle
and sends the signed request, manifest and host receipt as separate bounded `SOCK_SEQPACKET`
messages. Only after peer and signature admission does the root supervisor accept the two evidence
messages under a five-second-per-message timeout, re-parse their canonical schemas and nested
bindings, write root-owned `0400` files in a `0750` create-new temporary directory, sync and rename
it, and then enter the existing immutable import/start transaction. Exact completed or interrupted
uploads are recoverable; conflicting bytes, modes, owners, links, paths or digests fail before
spawn. The protected workflow now requires an exact same-repository artifact/run identity for
revision zero, verifies the six-file bundle after download, and invokes the typed `request-start`
path. Attach/seal/abort still receive no bundle and cannot use this path.

The complete WSL supervisor suite passes 153 ordinary tests with one manual system-bus test
ignored; the final upload-focused server and transport suites pass 9/9 and 8/8. The workflow was
not dispatched and the service was not installed, so the distinct admitted-host ownership/socket
rehearsal flag remains false. See `local-runs/w11-start-upload-local-20261005.json`.

A subsequent disconnect regression closes the service-loop edge around multipart upload. Transport
failure on one accepted connection is now scoped to that connection; mutation, serialization,
lease-maintenance and listener failures still remain fatal. A client that sends the signed request
and manifest but disconnects before the receipt produces no staging directory and no backend call;
the same server instance then accepts the complete request and starts exactly once. The WSL total is
now 154 passing ordinary tests plus the one ignored manual system-bus test, with the focused server
suite at 10/10. See `local-runs/w11-controller-disconnect-local-20261005.json`.

Start admission now also closes the gap between accepting an uploaded receipt and launching the
role. For every new I74 or C74 start, the privileged path collects a fresh host observation and
requires exact equality with the admitted receipt before creating the host claim, durable state or
spawn intent. An exact completed-request replay is resolved from the verified event journal first,
so later host drift cannot replace the original signed response or repeat the spawn. The injected
regression proves drift rejection before all three side effects, successful retry after convergence
and replay without another observation/backend call. The complete WSL suite is now 155 ordinary
passes plus one ignored manual system-bus test; the focused server suite is 11/11. This is local
composition evidence, not the outstanding root/systemd admitted-host rehearsal. See
`local-runs/w11-start-live-host-revalidation-local-20261005.json`.

The detached maintenance loop now consumes the harness-owned checkpoint chain directly, so loss of
the GitHub monitor does not freeze the supervisor's view at the last attach. Each checkpoint hashes
both its observation time and the last phase-qualified useful-progress time; telemetry-only
measured records cannot manufacture progress. The supervisor verifies the journal, head, campaign,
role and original process identities on every due check, refreshes attach state from that same
evidence, and ignores a valid terminal checkpoint so a later controller can seal it. A nonterminal
gap greater than the frozen 180 seconds commits `ProgressLossRequested` before bounded diagnostic
capture/stop, commits `ProgressLossCompleted`, clears retained process identity and then releases
the host claim. Backend interruption retains the exact cause for retry without restart.

At that source, the complete WSL supervisor suite passed 160 ordinary tests plus one ignored manual
system-bus test; the focused server and progress-loss suites passed 12/12 and 3/3. The integrated
writer passed 10 tests and the offline long-run verifier passed 7. Absence of the first startup
checkpoint was still not timed from the start event, and real root/systemd diagnostic-stop behavior
remained an admitted-host rehearsal. See
`local-runs/w11-progress-loss-maintenance-local-20261005.json`.

The startup gap is now closed locally without treating controller traffic as product progress. If
the role has not published a first checkpoint, the supervisor verifies the durable event journal,
requires the current running state and role-specific `Started` or `Adopted` lifecycle record, and
uses that record's timestamp as the immutable progress anchor. A missing journal and an empty
pre-publication journal share the same frozen 180-second deadline. Missing evidence is not accepted
as startup absence once durable state already names a checkpoint. When the deadline expires, the
supervisor records a startup-specific, hash-bound progress-loss cause before diagnostic/stop; a
restart recovers the timestamp and digest from the lifecycle journal and cannot invent a later
deadline or launch a replacement.

At source `8f15f3c2`, the complete WSL supervisor suite passes 162 ordinary tests plus the one
ignored manual system-bus test; the focused server and progress-loss suites pass 13/13 and 4/4.
The integrated writer remains 10/10, the offline verifier 7/7, and the performance-contract suite
17/17. Strict clippy passes for the supervisor, writer and `xtask`; the qualification dry-run
accepted the new contract digest and executed zero commands. Real root-owned service installation,
D-Bus diagnostic/stop timing and filesystem ownership are still admitted-host rehearsal work. See
`local-runs/w11-startup-progress-loss-local-20261005.json`.

The W11 expected-red proof now activates every forbidden weak admission family named by the frozen
plan in one registry-owned mutant: PID-only liveness, restart from a checkpoint, PID reuse,
sequence-only acceptance of a broken checkpoint hash, process-only mixing of attempt identities,
ignored lease expiry and ignored duplicate executors. Each weak predicate is first shown to accept
its constructed defect; the production predicate must reject it before the canary emits its
expected failure marker. On clean source `4b5f9812`, the normal seven-test state-machine suite
passes and the dynamic canary sweep records one expected-red proof with exit code 101. This is a
local model proof, not the still-required same-host controller-loss and systemd fault rehearsal.
See `local-runs/w11-controller-canaries-local-20261005.json`.

Detached maintenance now classifies loss of the measured process separately from loss of useful
progress. Before applying the progress deadline, the supervisor revalidates the frozen host
receipt, retained systemd unit, harness/daemon `/proc` identities and isolated cpuset. It records
one of four hash-bound causes: host identity drift, missing unit, unit identity drift or process
identity drift. A retained successful `active/exited` unit is still terminal evidence for sealing,
not a measurement failure. An actual loss commits `MeasurementLossRequested`, retains the exact
process/checkpoint cause across restart, captures a bounded create-new diagnostic, commits
`MeasurementLossCompleted`, clears execution identity and only then releases the host claim.

The diagnostic backend will stop a still-present unit only for process-identity loss when its unit
name, cgroup and MainPID still bind the retained campaign. Missing, replaced, cross-boot or otherwise
ambiguous units are never stopped as if they were owned; an unsafe surviving unit keeps recovery
failed closed for operator inspection. A committed failure intent also takes precedence over later
lease expiry, so restart cannot relabel progress or measurement loss. The read-only monitor now
accepts the intentionally cleared execution fields of completed terminal states and reports a
completed recorded failure as `measurement-loss` rather than `invalid-state`.

At source `9e7409d0`, the complete WSL supervisor suite passes 169 ordinary tests plus one ignored
manual system-bus test; measurement-loss and focused server suites pass 4/4 and 15/15. The monitor
passes 7/7, the integrated writer 10/10, the offline verifier 7/7 and the performance-contract suite
17/17. The no-execution qualification dry-run accepted contract digest
`4e2a2fce55058f2d9fc8aee6f6f831d60ad2ce13d48b4bcc83174171cfa7cfc9` and executed zero
commands. Production D-Bus observation/diagnostic/stop and root-owned filesystem behavior remain
admitted-host rehearsal work; this result contains no product throughput measurement. See
`local-runs/w11-measurement-loss-maintenance-local-20261005.json`.

A read-only probe of the serialized Linux runner at source `68a36e93` now narrows that external
boundary. PID 1 is systemd, the system manager bus and cgroup v2 controllers are available, and the
state path would reside on ext4. The host is not provisioned: the frozen service, binary, config,
accounts/groups, state directories and Unix socket are all absent, and the runner is not a member
of the required client group. No mutation or qualification command ran. Protected provisioning
with the real verification key is required before the same-host controller-loss rehearsal; see
`local-runs/w11-host-capability-68a36e93.json`.

Protected provisioning is now complete at source `04de1615`. The successful manual workflow run
`37322301775` derived only the Ed25519 public key in the hosted preparation job, signed the exact
bundle manifest, and admitted that bundle through a fixed root-owned installer. The self-hosted
runner receives neither the private key nor a general sudo/shell surface. Its only passwordless
root command verifies the installed trust root, bundle signature, closed file set, source identity,
configuration and active-install drift before mutation. A read-only receipt export is the only
additional operation.

The installed service is active. The supervisor account is UID 986, the client group is GID 987,
the Actions runner has effective groups `[987, 1001]`, and the Unix socket is `0660 root:987`.
Protected state directories remain intentionally unreadable to the runner. The post-install probe
reports `provisioned=true`, `runner_client_socket_access=true` and
`host_rehearsal_ready=true`; see `local-runs/w11-host-provisioning-04de1615.json`.

This closes host provisioning, not W11. No measured transient unit, fixture campaign, controller
loss, supervisor restart, hang, lease expiry, reboot, I74/C74 workload, six-hour qualification or
24-hour confirmation ran. `systemd_confinement_complete`, the host-rehearsal flags and release
admission therefore remain false.

The next bounded slice is now complete at source `9e8e47ae`. Run `37328774683` started only the
fixed `/usr/bin/sleep 1` fixture in a five-second, 64 MiB, 16-task transient service owned by
`hydracache-perf`; it did not execute a product candidate. The receipt observed a nonzero MainPID
in `/system.slice/hydracache-performance-074-systemd-smoke.service`, then `active/exited`, zero
MainPID and `Result=success`, followed by removal after `StopUnit`. The parallel read-only
capability probe also passed. See `local-runs/w11-systemd-smoke-9e8e47ae.json`.

The retained first attempt, run `37327128699`, was useful negative evidence. systemd rejected the
transient property array because `ProtectHome` was serialized as a D-Bus boolean even though the
manager interface requires the string value `yes`. Host introspection isolated that exact type;
the regression test and both fixture and production unit policies were corrected without changing
the command or granting new authority. The repeated signed install and smoke then passed.

This proves the reviewed D-Bus property shape and basic real-unit lifecycle only. It does not close
the production spawn-backend, systemd-confinement or complete host-rehearsal flags, because no
campaign manifest, durable start transaction, controller loss, supervisor restart, diagnostic-stop
path or product workload was exercised. The next safe step is a bounded non-promotable
controller-loss rehearsal through the production start lifecycle with fixture binaries and
identities.

An isolated controller-loss primitive is now also proven at source `8d05954b`. Run `37332481302`
used two separate root command processes. The first started only `/usr/bin/sleep 30`, recorded the
fixture PID, start ticks, process group, cgroup inode and fixed unit name in a root-owned `0600`
context, then exited. Two seconds later a new process proved the original controller identity was
gone and the fixture still had the exact stored process and cgroup identity before issuing
`StopUnit`. The context was removed, the transient unit unloaded and the supervisor remained
active. See `local-runs/w11-controller-loss-smoke-8d05954b.json`.

This closes only `bounded_controller_loss_fixture_complete`. It deliberately does not claim the
production start backend or full controller-loss rehearsal: the fixture is one process, bypasses
the signed socket request and campaign manifest, and creates no durable campaign state or
checkpoint chain. The next slice must supply a bounded two-process non-promotable campaign fixture
and drive it through the real upload, start lifecycle, systemd spawn backend and replay path.

That campaign-shaped slice is now complete at source `d2e3e3ad`. Run `37352537768` used a strict
non-product manifest and the production `drive_i74_start_request` and `SystemdSpawnBackend` to
start a harness/daemon pair in the exact deterministic unit. The first controller recorded revision
2 and checkpoint sequence 1, then exited. A new controller recovered the exact PIDs 1509228 and
1509229, replayed the accepted start response with zero backend spawn calls, attached the observed
checkpoint at revision 3, and drove the real abort backend to `ABORTED_INCOMPLETE` revision 5. The
unit stopped, all execution fields and the root-owned handoff context were cleared, and the active
host claim was released. After cleanup the supervisor was `active/running` with zero restarts. See
`local-runs/w11-campaign-lifecycle-smoke-d2e3e3ad.json`.

The failed rehearsals are retained as attribution rather than discarded. They separately exposed
an accidental dependency on the production host-freeze path, incorrect D-Bus append-output
properties, a `StateDirectory` ownership conflict that caused `200/CHDIR`, and a handoff context
placed below a runtime directory intentionally removed with the stopped supervisor. Each retry
changed only its identified boundary; no product candidate or qualification workload ran.

This closes the real systemd spawn-backend and campaign-shaped controller-loss rehearsal, not all
of W11. The fixed root command invoked the coordinator directly, so signed Unix-socket upload/start
admission and loss/restart of the supervisor service itself remain unproven. Admitted-host
progress-loss, measurement-loss, lease-expiry, reboot, live seal, product I74/C74 execution,
overhead budgets, six-hour qualification and 24-hour confirmation also remain open. Consequently
`host_rehearsal_complete` and `release_admission_allowed` stay false.

The admitted-host progress-loss slice is now complete at source `1de46cac`. Run `37358000444`
started the same bounded harness/daemon fixture through the production start lifecycle while the
production supervisor was explicitly inactive. An independent live snapshot observed exactly the
two retained PIDs in the expected active systemd cgroup with zero restarts. The replacement
controller waited for the frozen 180-second useful-progress gap: useful progress was timestamped
at `1791225792`, the rejection deadline was `1791225972`, and maintenance classified the loss at
`1791225975`. It then traversed the production diagnostic and stop backends, published the
2,890-byte diagnostic with SHA-256
`90ce2ce38790aa8663c2f46765a00dd4525396684e261b799026b35632c28eb9`, committed revision 4
`FAILED_INCOMPLETE`, cleared every execution field, stopped the exact unit and released the host
claim. The supervisor was restored `active/running` with zero restarts. See
`local-runs/w11-campaign-progress-loss-smoke-1de46cac.json`.

Three rejected paths remain evidence rather than being hidden by the successful retry. The first
bundle attempted to extend the immutable root entrypoint and was refused before installation. The
next provisioning attempt refused to replace a differing active installation until an exact
no-active-campaign check allowed a controlled retry. Most importantly, the first live progress
fixture overlapped the production supervisor: that supervisor correctly interpreted the fixture
receipt as production host-identity drift and entered its recovery loop. Exact cancellation,
identity-bound unit cleanup and restart recovered durable `FAILED_INCOMPLETE` state and released
the host claim. The fix did not weaken production receipt verification; instead both coordinator
and workflow now fail closed unless the production supervisor is exactly inactive for this
fixture-only maintenance path.

This closes `progress_loss_host_rehearsal_complete`, including the real 180-second deadline,
create-new diagnostic, systemd stop and durable release effects. It does not close full host or
release admission. Signed socket start, supervisor restart during a live production-shaped
campaign, lease-expiry, reboot, live seal, product I74/C74 execution, overhead
budgets, six-hour qualification and 24-hour confirmation remain open. No product candidate or
expensive workload ran.

The admitted-host measurement-loss slice is complete at source `8d786d8f`. Run `37362383128`
started the same bounded non-product pair and retained the harness after the fixture daemon exited.
At observation second `1791228108`, 212 seconds after checkpoint sequence 1, the original harness,
unit and cgroup still matched while the daemon `/proc` identity was absent. The coordinator
classified only `process-identity-drift`, published a 3,662-byte cause-bound diagnostic with
SHA-256 `c4a752521d8c77f248fa4d5721fe9857959e25c68b9dcfa6923dda5d5ee55cce`, stopped the exact unit,
committed revision 4 `FAILED_INCOMPLETE` with `recorded_failure=true`, cleared execution fields and
released the host claim. The supervisor was restored `active/running` with zero restarts. See
`local-runs/w11-campaign-measurement-loss-smoke-8d786d8f.json`.

The first run, `37360782049`, is retained as negative attribution. It correctly reached the
one-process fault shape but expected an attached checkpoint in durable state. A fresh start has a
verified live checkpoint chain before any controller has attached its head, so the resume command
failed before recording measurement intent. Recovery produced the same safe terminal revision and
released marker. The fix preserved the optional checkpoint in the durable measurement cause and
used `observe_live_checkpoint_evidence` only for the receipt's sequence assertion; a regression
test now distinguishes observation of a live unattached head from attached-head verification.

This closes `measurement_loss_host_rehearsal_complete` for process identity drift and its real
diagnostic/stop/release effects. It does not generalize one proven fault into all faults: host drift,
unit absence and unit identity drift remain fail-closed classifications that intentionally refuse
to stop ambiguous units. Signed socket start, supervisor restart, reboot, live seal, product
execution and expensive qualification remain open.

The admitted-host lease-expiry slice is complete at source `187b63fb`. Signed provisioning run
`37364363138` installed the exact source after GitHub-hosted runner recovery, and bounded run
`37380414415` started only the non-product two-process fixture with the production supervisor
explicitly inactive. After the configured transient cpuset-drift window had ended, the replacement
controller observed the exact original pair and checkpoint sequence 1. It resumed maintenance at
Unix second `1791238330`, three seconds after frozen lease deadline `1791238327`, published the
2,853-byte cause-bound diagnostic with SHA-256
`5d7f3cee490d9ec358d218dfc9b8390e0c57ef27ea036ab7e71c3d833ef2be53`, stopped the exact unit,
committed revision 4 `LEASE_EXPIRED_INCOMPLETE`, cleared execution fields and released the host
claim. The production supervisor was restored `active/running` with zero restarts. See
`local-runs/w11-campaign-lease-expiry-smoke-187b63fb.json`.

The two cancelled provisioning attempts are retained as external negative attribution. Both waited
fifteen minutes for `ubuntu-latest` during GitHub's hosted-runner incident and executed zero steps;
neither touched the host. Once Actions reported normal operation, only failed jobs of the original
run were retried. No duplicate provisioning workflow and no unsigned/manual installation bypass
were used. The final receipt proves exact identity after the configured drift window; it does not
independently sample the temporary cpuset mismatch itself, so that narrower observation remains a
test-backed schedule property rather than a host-measured claim.

This closes `lease_expiry_host_rehearsal_complete` and the real diagnostic/systemd-stop/durable
release effects. It does not close full host or release admission. Signed socket start, supervisor
restart during a live campaign, reboot, live seal, product I74/C74 execution, overhead budgets,
six-hour qualification and 24-hour confirmation remain open. No product candidate or expensive
workload ran.

The first admitted-host overhead slice is retained at tooling source `404c4622`, against the exact
installed supervisor source `187b63fb`. Run `37386694639` observed the already-active supervisor
for 30.000073206 seconds without sending a socket request, starting a workload, restarting a unit
or mutating host state. Across 61 samples the same PID/start ticks/cgroup/cpuset remained stable and
`NRestarts` remained zero. Idle CPU was 0.00754665% against the frozen 0.5% ceiling; maximum RSS was
4,538,368 bytes against 67,108,864 bytes. The service used one process/thread, accumulated 30
voluntary and one involuntary context switches, and recorded zero cgroup CPU-pressure time. The
cgroup memory-current maximum was 819,200 bytes; the lifetime cgroup peak was 1,343,488 bytes. See
`local-runs/w11-supervisor-idle-overhead-404c4622.json`.

This is a confirmed partial observation, not a completed overhead budget. The service cgroup does
not expose `io.stat`, and the unprivileged `github-runner` correctly cannot read the root-owned
supervisor's `/proc/<pid>/io`. The receipt therefore records the I/O metric and conjunction as
unevaluated instead of substituting a proxy or treating a missing counter as zero. The three
pre-success runs are retained as negative attribution: fail-closed discovery of the absent I/O
counter, a diagnostic preflight receipt, and selection of `/proc/<pid>/status` as the exact cpuset
source when the cgroup did not expose `cpuset.cpus.effective`. None mutated the service.

Consequently `supervisor_idle_cpu_rss_screen_complete=true`, while
`supervisor_idle_io_screen_complete`, `idle_overhead_budget_complete` and
`role_overhead_qualification_complete` remain false. A reviewed exact I/O counter source and the
role-level checkpoint/supervisor A/B are still required; the frozen ceilings were not changed. The
next isolated instrumentation commit therefore staged systemd `IOAccounting` for the supervisor
and fixed transient units. The root cgroup-v2 capability receipt included the `io` controller; at
that point the new unit policy had not yet been signed, installed or restart-rehearsed, so the
partial receipt correctly left its completion flags false.

That instrumentation is now signed and host-rehearsed at exact source `1bc4823b`. Provisioning run
`37387670191` installed the bundle after rechecking that no active campaign marker or fixture
context existed and stopping only the previous supervisor. The replacement service came up as PID
1538885 with zero restarts and `IOAccounting=yes`; its cgroup `io.stat` is readable by the
unprivileged runner. The fixed sudo surface did not change. See
`local-runs/w11-host-provisioning-io-accounting-1bc4823b.json`.

Read-only run `37388038717` then sampled that exact installed source for 30.000063773 seconds. All
61 samples retained the same PID, start ticks, control group and `0,5-15` cpuset. CPU was
0.00662332%, maximum RSS was 4,415,488 bytes, and exact cgroup I/O was 0 bytes/second; each stayed
below the frozen 0.5%, 67,108,864-byte and 1,048,576-byte/second idle ceilings. CPU pressure stayed
zero, memory-current peaked at 819,200 bytes, lifetime cgroup memory at 868,352 bytes, and the
single thread recorded 30 voluntary plus one involuntary context switches. See
`local-runs/w11-supervisor-idle-overhead-1bc4823b.json`.

This supersedes the telemetry blocker and closes `supervisor_idle_overhead_screen_complete`, but
not the W11 overhead budget. No checkpoint writer or I74/C74 role ran, so checkpoint I/O, paired
role timing/asymmetry and role-level overhead remain unevaluated. Accordingly
`idle_overhead_budget_complete`, `role_overhead_qualification_complete` and release admission stay
false.

The local role-overhead analyzer is now staged before collecting any role-level numbers. Its strict
attempt-set schema requires 20 independent attempts: five counterbalanced control/instrumented
pairs for I74 and five for C74. Every attempt binds source, binary, workload, payload, host receipt,
seed, operation count, warm-up and cpuset; control attempts must write zero checkpoint bytes. The
analyzer rejects order, identity, guard or operation-count drift, checks the frozen supervisor CPU,
RSS and combined supervisor/checkpoint I/O ceilings on every instrumented attempt, and evaluates
elapsed-time and role-CPU overhead asymmetry separately so one role cannot be pooled away. See
`schemas/role-overhead-attempt-set.schema.json` and
`scripts/perf/performance_long_run_role_overhead_074.py`.

This analyzer is deliberately non-promotable. Even a passing synthetic attempt set leaves
`role_overhead_qualification_complete=false` and `release_admission_allowed=false`.

The fixed same-host collector/executor is now staged as a second, separate slice. The first version
at `57b3e56d` attempted to add a no-argument `--role-overhead-smoke` verb to the root-owned
provisioning entrypoint. Signed provisioning run `37461600977` rejected the bundle before mutation
with `bundle installer differs from the root-owned entrypoint`. That is the required bootstrap
invariant, not a transient failure: signed payloads may replace the installed product binary but
cannot enlarge the command that authorizes their own installation. No host file changed, the old
supervisor was restored active with zero restarts, and the runner was returned inactive/disabled.
See `local-runs/w11-role-overhead-root-entrypoint-rejected-57b3e56d.json`.

The corrected collector is entirely unprivileged and the root entrypoint is byte-identical to its
pre-candidate blob. `performance_long_run_role_overhead_collect_074.py` runs exactly 20 short
non-product attempts in ABBA order, selects one CPU from the runner's allowed set and invokes only
`taskset`, `nice` and the installed supervisor binary's hidden bounded fixture with an argv list and
`shell=False`. Fixed rlimits cap address space, open files, output size and core dumps. The caller
cannot supply a command, operation count, warm-up, delay, checkpoint size or schedule.

Each attempt binds the installed source/binary and root-emitted provisioning-receipt digest,
verifies unchanged boot and supervisor PID/start-time, exact argv, cgroup and cpuset identities, and records role elapsed/CPU/RSS/write
bytes plus exact supervisor cgroup-v2 CPU/I/O and process peak RSS. Control writes zero checkpoint
bytes; instrumented writes and fsyncs exactly 4,096 bytes. Because the Actions user intentionally
cannot traverse the root/service-group campaign directory, the collector does not infer absence
from an inaccessible path. Instead it brackets every attempt with the existing typed read-only
host-observation request. The root supervisor validates the marker before and after observation and
returns `active_campaign_absent=true` inside its response digest; any active, malformed or
unreadable claim fails closed. The workflow still requires an active zero-restart supervisor, runs
the frozen analyzer and retains raw attempts, fixture receipts and analysis. Before the admitted
host run, `non_product_role_overhead_rehearsal_complete` remained false. Product I74/C74 pairs are
a later, separate gate; no product candidate or expensive qualification is started by this mode.

The first unprivileged host run, `37465569900` at `02a1123d`, failed closed before attempt one:
Linux ptrace policy denied the Actions user access to `/proc/5083/exe`. The retained artifact proves
that source/provisioning binding and all earlier guards passed, no checkpoint was written, and the
supervisor remained active with zero restarts. The corrected identity check uses the exact readable
`cmdline`, unified `cgroup`, PID/start ticks and cpuset, while the root host-observation response
continues to bind the installed/live binary digest. It adds no privilege and does not weaken binary
identity. See `local-runs/w11-role-overhead-runner-proc-guard-rejected-02a1123d.json`.

The next signed run, `37466656900` at `2815c72c`, passed that corrected process-identity guard but
stopped before attempt one because the collector treated an empty, readable cgroup-v2 `io.stat` as
missing telemetry. On this freshly restarted `IOAccounting=yes` service, the zero-length file is
the kernel's valid zero state: no block-device row exists until the cgroup issues block I/O. The
correction accepts only that readable empty file as zero; a missing or unreadable file, a malformed
row, or a row without `rbytes`/`wbytes` still fails closed. The supervisor remained active at PID
6982 with zero restarts and no campaign context. See
`local-runs/w11-role-overhead-empty-io-stat-rejected-2815c72c.json`.

The corrected exact-source run `37467960862` at `543108f1` completed all 20 attempts. Each role had
five ABBA control/instrumented pairs with seed 740074, 100,000,000 measured operations and 1,000,000
warm-up operations per attempt on CPU 0. I74 median elapsed overhead was 1.271387% and median role
CPU overhead was 0.043175%; C74 measured 1.291742% and 0.045724%. The cross-role differences were
0.020355 and 0.002548 percentage points, below the frozen 1-point asymmetry ceiling. Maximum
supervisor CPU was 0%, peak supervisor RSS was 5,242,880 bytes, and maximum combined checkpoint
plus supervisor I/O rate was 9,039.898 bytes/s, all below their frozen ceilings. Every identity,
placement, operation-count and campaign guard passed; control attempts wrote zero checkpoint bytes
and instrumented attempts wrote exactly 4,096 bytes.

This closes only `non_product_role_overhead_rehearsal_complete`. The fixture is synthetic, so
`idle_overhead_budget_complete`, `role_overhead_qualification_complete`, `host_rehearsal_complete`
and release admission remain false. The runner was returned inactive/disabled and the supervisor
remained active at PID 8839 with zero restarts. See
`local-runs/w11-role-overhead-rehearsal-543108f1.json`.

The exact-source reference-host preparation is now rehearsed at `e756a41e`. The runner was proven
idle, disabled and stopped; rootless and rootful Docker were absent; all campaign markers and
fixture contexts were absent; and the profile-driven quiet-service policy was applied into a fresh
state directory. Before reboot, the generated normal GRUB entries contained the reviewed
`nosmt`, `isolcpus`, `nohz_full`, `rcu_nocbs` and `irqaffinity` arguments exactly once and no active
entry contained `pci=nomsi`. The host returned through ordinary SSH with a new boot id, all three
RAID1 arrays still `[UU]`, CPUs `0-7` online, CPUs `1-4` isolated/nohz-full and SMT off. Rescue was
not needed.

The post-boot early IRQ layout preflight passed across 68 IRQ files without mutating affinity, and
the full host verifier admitted eight dormant, unmapped, zero-count NVMe queues under the existing
exception. A fresh canonical freeze was then written at the production collector's exact
`/var/lib/hydracache-perf/host-tuning-v1` path. Its SHA-256 is
`1e0e8e5ec784faf590617fcec379b834cc71174a5dd0c2b5a61656e7c6bf0628`; an immediate drift check
passed. Signed provisioning run `37389564746` subsequently reinstalled the exact `e756a41e` source,
after stopping only the supervisor and temporarily enabling the otherwise-disabled runner. The
runner was returned offline, the supervisor was active as PID 7606 with zero restarts and
`IOAccounting=yes`, and the same frozen-state check passed again. See
`local-runs/w11-reference-host-freeze-e756a41e.json`.

This closes reference-host freeze preparation and the controlled pre-admission reboot only. It is
not the expected-red reboot test for a live role: no campaign process existed across this reboot.
Signed socket start, supervisor restart with an exact live non-product pair, live-role reboot,
live seal, role-level overhead and product qualification remain open. The next implementation must
export the production host observation through the fixed root-owned entrypoint; a manual writable
receipt or expanded arbitrary-path sudo rule is not an acceptable substitute.

The first export candidate at `d7a69d9e` tested whether that fixed entrypoint could grow one
no-argument read-only verb. Hosted preparation and all tests passed and produced a signed bundle,
but run `37390392244` was rejected before installation with `bundle installer differs from the
root-owned entrypoint`. That is intentional bootstrap immutability: the already-installed command
cannot authorize a replacement for itself, even when the replacement is inside a newly signed
bundle. No host file was copied. The runner was returned offline, the original supervisor was
restored active with zero restarts, and `check-frozen` passed. The candidate was reverted by
`7be1a442`; see `local-runs/w11-host-observation-export-rejected-d7a69d9e.json`.

The next design therefore keeps sudoers and the root entrypoint byte-identical. Host-observation
export must be a typed read-only request served by the already-root supervisor over its protected
Unix socket, with peer admission and bounded canonical output. That uses the installed trust
boundary instead of attempting to enlarge the bootstrap boundary.

That corrected path is now rehearsed on the admitted host at exact source `127ebc6c`. Signed
provisioning run `37394441292` installed a supervisor whose SHA-256 is
`7f8f61bc80fb48c0fa5559189aa362b0387cc5ba069f4d10a94d3e5f78175409`. Before the self-hosted
install, the active-campaign marker and fixture context were absent and only the prior supervisor
was stopped. After installation, the runner was returned offline and the service was active as PID
25488 with zero restarts. The canonical freeze was regenerated for that exact installed source and
has SHA-256 `e15e5b218c1fee33f842ea8e697507629fab8a6ca4bc362e1bef1d0fe787d763`;
`check-frozen` passed.

Read-only observation run `37394832233` then used the existing root supervisor without stopping or
restarting it. The request was revision zero, pathless, unsigned and fixed to the sentinel campaign
and manifest identities; peer UID/GID, repository and actor admission still applied. The root
server read only the fixed root-owned provisioning receipt, required its binary digest to equal the
live observed supervisor digest, and returned the installed source commit with the canonical host
receipt. Independent artifact verification reproduced request digest `792d808a...`, response
digest `39dbaa24...` and host-receipt digest `c8041142...`. The retained artifact is
`long-run-074-host-observation-127ebc6cb24191d567772966586bdb827a0ae380-37394832233-1`, GitHub
artifact `11382701858`, digest `sha256:e9417fe6dea7f7445b013e24b9ee0f63b579c8952fd20abebc33b193662205ad`.
No arbitrary output path was accepted, no campaign state was created, no product candidate ran and
the result is explicitly non-promotable. See
`local-runs/w11-host-observation-127ebc6c.json`.

Two preceding failures sharpened the boundary rather than being discarded. Run `37393137026`
proved that the unprivileged runner cannot read the root-owned provisioning receipt through the
state-directory permissions; those permissions were not relaxed. Run `37393593771` disproved
cross-home Rust binary byte equality as source proof: the hosted installation and self-hosted
rebuild embedded different absolute Cargo registry paths. The accepted design therefore lets the
root supervisor validate the fixed receipt and bind its `source_commit` to the digest of the binary
that collected the observation. It does not trust runner-readable state or claim reproducible
builds where they were not established.

This closes `host_observation_socket_export_complete` and
`installed_source_receipt_binding_complete`, not signed start. The next safe slice is to assemble
one exact non-product start bundle from this admitted receipt and rehearse protected authorization,
supervisor-owned upload and start. Supervisor-restart survival, live-role reboot, live seal,
role-level overhead and product qualification remain open.

That protected socket slice is now complete at exact source `56ff0819`. Signed provisioning run
`37405246561` installed binary
`d4eb5c6b36520af361404c754ac276cb21e125c15d0ef0d5fa760770dc71e178` after the active claim and
fixture context were proven absent. Bundle run `37405722464` bound the installed source, canonical
host receipt, fixture identity and a 900-second non-product role limit into campaign `3b1fa143...`.
The lease remained 3,600 seconds; no product binary was selected.

One top-level protected run, `37405915773`, then signed and executed start, attach and abort with
three distinct request UUIDs but the same GitHub run principal. Start accepted the exact
harness/daemon pair at revision 2. Attach accepted checkpoint sequence 6 at revision 3 and bound
the controller lease to that same run. The recurring writer reached 11 checkpoints before signed
abort committed revision 5 `ABORTED_INCOMPLETE`, stopped the exact transient unit, cleared process,
checkpoint and controller-lease fields, and released the host claim. The runner was returned
offline; the supervisor retained PID 27610 and `NRestarts=0`. NVMe interrupt counters on isolated
CPUs 1-4 remained zero while housekeeping CPUs owned the evidence I/O. See
`local-runs/w11-protected-start-attach-abort-56ff0819.json`.

The immediately preceding run `37402969310` remains checked in as negative attribution. It exposed
a real read/write race: the checkpoint writer had fdatasynced a new JSONL record but had not yet
atomically replaced `checkpoints.head`, so maintenance treated a valid commit window as stale
evidence and exited. The restart then changed the service mount-namespace identity and correctly
failed closed rather than stopping an ambiguous unit. A separate bound mismatch also gave the
900-second rehearsal daemon only 360 seconds of `RuntimeMaxSec`. The fix reads journal/head as a
bounded stable snapshot, covers both complete-record and partial-tail windows, and makes the
fixture unit limit exactly 900 seconds. Permanent chain corruption is still rejected. See
`local-runs/w11-protected-start-attach-abort-negative-29675f17.json`.

This closes privileged start-bundle staging and the signed start/attach/abort host rehearsal. It
does not close all mutating operations or W11.

The supervisor-restart slice is now also complete at exact source `9178f3ab`. Signed provisioning
run `37410275153` installed binary
`1dfb472f54998e0d12cf68fa050b8819cc277629368d5f8378d22ba7b2f9815b`; bundle run
`37410539986` bound campaign `6689c9cd...` to the installed receipt. Combined protected run
`37410711175` started the non-product pair, after which only the supervisor was restarted. Its PID
changed from 65781 to 74207 while the campaign state digest, fixture PIDs 73941/73942 and both
start-tick identities remained unchanged. No duplicate executor appeared. Signed attach in that
same run accepted checkpoint sequence 7 at revision 3, and the same controller principal then
aborted to revision 5 `ABORTED_INCOMPLETE`. Thirteen checkpoint records were retained; the unit,
fixture processes and host claim were absent afterward, the runner was offline and the restored
supervisor remained active with `NRestarts=0`. See
`local-runs/w11-supervisor-restart-9178f3ab.json`.

The failed predecessor is part of the result. A service restart creates a new private mount
namespace, so its numeric mount ID changed from 324 to 430 although the device, root, mount point,
filesystem, source and mount options were identical. The durable identity now excludes only that
namespace-local number while the full receipt still records it; every semantic mount field remains
fail-closed. The same attempt also proved that controller lease ownership requires attach and abort
to share one workflow run, and that systemd may release the cgroup of the exact retained
`active/exited`, `MainPID=0`, successful unit. That empty terminal cgroup is now admitted only for
that exact terminal shape; a non-empty foreign cgroup remains identity drift. See
`local-runs/w11-supervisor-restart-negative-671c93d6.json`.

This rehearsal closed supervisor-restart survival, same-process adoption and post-restart
attach/abort only. At that stage, live seal, reboot while a role was active, role-level overhead,
product I74/C74 execution, six-hour qualification and the separately authorized 24-hour
confirmation were still open; the later receipts above close the two lifecycle gates only.
`host_rehearsal_complete` and `release_admission_allowed` therefore stay false. No product
candidate, expensive qualification or isolated-CPU interrupt measurement was performed in this
slice.

`scripts/ci/monitor-long-run-campaign-074.py` is a replaceable read-only observer. It strictly
validates the complete durable-state shape and expected campaign/manifest identities, reports
controller loss separately from stale useful progress and measurement loss, rejects future
progress timestamps, and writes a create-new non-promotable receipt. It never starts, attaches,
seals, aborts, or otherwise mutates a campaign.

W11 is staged, but disabled, by `qualification-manifest.toml`. Its contract inputs are
content-addressed and its expensive phases remain `not-run`. The only currently supported action
is a no-execution validation:

```text
python scripts/perf/performance_qualification_dry_run_074.py --dry-run \
  --manifest docs/testing/performance/0.74/qualification-manifest.toml \
  --output target/performance-evidence/0.74/qualification-dry-run.json
```

The dry-run refuses digest drift, reordered phases, hidden blockers, or admission of an expensive
phase. It deliberately has no execution mode before the 0.74 candidate identity, admitted host,
Redis binary identities, qualification runner and explicit authorization are available. The
predecessor tag and confirmation are no longer blockers.

Numerical receipts must never be hand-edited into claims. Every receipt binds the trace, payload
and key corpora, seed, warmup, duration, offered schedule, concurrency, pipeline depth, security,
persistence and final-state digest. Errors, timeouts, rejections, late operations and incomplete
operations stay in the goodput denominator.

Local paired work uses `local-harness.toml` and
`scripts/perf/performance_local_pairing_074.py`. The runner fixes a five-pair ABBA order, requires
warm-up, applies affinity and process priority, rejects attempts that start above the frozen
background-CPU ceiling, and derives a minimum detectable effect from same-binary A/A deltas. Its
output is always non-promotable; a result below that A/A-derived floor is `inconclusive`, not a
product win. Unit-test the runner with:

```text
python -m unittest scripts/perf/test_performance_local_pairing_074.py
```

W10's checked-in `composition-ledger.toml` currently records zero accepted product candidates.
It therefore forbids a synthetic C74 freeze or addition of isolated percentage gains. Tooling and
evidence work can continue, but composition remains a no-op until an isolated proposal actually
passes its native, semantic and local performance gates.

## W12 pre-admission boundary

`release-admission-contract.toml` now makes the current release state machine-readable. C74 remains
`UNRESOLVED`; product candidate runs, expensive qualification, six-hour qualification and 24-hour
confirmation remain disabled; every product, portable, distributed-capacity and Redis-superiority
claim remains forbidden. The contract lists the exact unresolved identity, qualification,
supply-chain and immutable-archive receipts and is itself a content-addressed input to
`qualification-manifest.toml`.

The draft release note at `docs/releases/0.74.0.md` records the negative proposal decisions,
activation/default state, compatibility boundary, rollback path and remaining gates without
inventing an accepted product candidate. The W12 expected-red canary mutates the expensive-run
flag and must fail with `HC-CANARY-RED:PERF74-W12`; a green mutant would mean the admission checker
could silently authorize unresolved work. `local-runs/w12-pre-admission-local-20261006.json`
records this preparation only and is explicitly non-promotable.

This closes local W12 contract and documentation preparation, not W12 release admission. Exact C74
D4 evidence, product qualification, packages, SBOM, advisory/license results, supported-target
receipts, and the independently verifiable immutable archive remain absent. Both `--require-ship`
checks must stay red until those inputs exist on the same frozen candidate.

The W2-W9 terminal outcomes are also normalized in `terminal-disposition-ledger.toml`. Its 23 rows
bind every parent/sub-item to retained files and require `terminal=true` plus
`accepted_product_change=false`. The release-evidence report therefore has zero `Planned` rows and
28 `Implemented` evidence contracts, but still zero `FastGreen`, `GatedGreen`, or `ShipReady` rows.
Here, `Implemented` means the negative/deferred disposition is machine-readable; it does not mean
the proposed optimization was shipped. The remaining transition requires real gate receipts, not
a relabeling of terminal local evidence.

All 28 work items now name the existing `fast.workspace-nextest` gate. Its command remains `cargo
nextest run --workspace --profile ci --locked`; its 2,400-second timeout, 840-second budget and the
1,680-second aggregate registry budget are unchanged. The additional W4/W6/W8/W9 sub-item labels
only make coverage explicit. No narrow evidence-only test substitutes for the workspace gate.
Until that exact command produces a clean exact-commit receipt, every item remains `Implemented`
rather than `FastGreen`. The non-promotable registration receipt is
`local-runs/w12-fast-gate-registration-local-20261006.json`.

The first exact-source W11 canary refresh on Windows stopped in its green guard before the canary
ran. The supervisor binary referenced the Linux-only `campaign_fixture_harness` from one unguarded
CLI match arm, producing compiler error E0425. The narrow repair adds the same Linux cfg guard used
by the adjacent fixture commands; it does not alter Linux execution. The failed guard receipt is
retained as `local-runs/w11-windows-canary-guard-compile-rejected-ae32f62c.json` and cannot count as
expected-red evidence.

The first clean exact-source execution of `fast.workspace-nextest` used the evidence runner's
isolated Cargo target and exhausted the unchanged 2,400-second hard timeout while the workspace was
still compiling. It also exceeded the unchanged 840-second cadence budget. No test started, the
required JUnit file was absent, and no product or test failure was observed. The result is retained
as `local-runs/w12-workspace-fast-gate-cold-timeout-05e7c6e9.json`; it is non-promotable and leaves
all work items short of `FastGreen`. One same-command warm-cache retry may distinguish cold-build
capacity from a repeatable gate failure, but cannot erase this result or relax either limit.

That one warm-cache retry completed compilation in 11 minutes 24 seconds and started 3,655 tests
across 477 binaries, with 82 tests skipped by the profile. The overall command still timed out:
the evidence receipt records 2,626,352 ms including 226,352 ms spent returning after the nominal
hard-timeout boundary. No failure was reported before termination, but the suite did not complete
and JUnit was not produced, so absence of an observed failure is not a pass. The non-promotable
record is `local-runs/w12-workspace-fast-gate-warm-timeout-415b01fc.json`. There will be no third
local retry without a newly identified root cause; the unchanged full gate remains the blocker.

The follow-up investigation reproduced a separate evidence-runner defect with a short Windows
fixture: a parent exited while its descendant retained stdout/stderr, and a one-second gate
incorrectly returned `Pass` after 4,070 ms. This does not establish the exact cause of the earlier
workspace overrun. The baseline is retained in
`local-runs/w12-windows-inherited-pipe-baseline-5bc712f1.json`.

The Windows runner now creates the registered command suspended, assigns it to a private
kill-on-close Job Object, and resumes its initial thread only after successful assignment. `Pass`
requires the parent's successful exit, zero live job processes and completed output capture within
the command deadline. Timeout terminates that owned job and uses a bounded two-second cleanup
poll, so it no longer waits on an external `taskkill` or unbounded pipe joins. A containment,
resume, accounting or capture failure fails closed.

On clean source `a4b15edf`, all 15 evidence-runner tests passed. The inherited-pipe, null-pipe and
three-generation fixtures returned `Timeout` in 1,071, 1,057 and 1,106 ms respectively with a
one-second limit, and no survivor marker appeared after four seconds. A separate fixture retained
128 KiB on each stream and a Unicode tail. These diagnostic timings and the local Linux compile
check and 12/12 Linux runner tests are recorded in
`local-runs/w12-windows-evidence-job-timeout-a4b15edf.json`. The full workspace
gate was not repeated; its existing negative receipts and release-admission boundary still apply.

For an ordinary `ci.yml` dispatch with `candidate_release=0.74`, the existing hosted fast-evidence
lane now runs the 0.74 contract and runner tests plus both expected-red canaries before the
unchanged workspace gate. It then assembles a non-ship 0.74 report from those same-source receipts,
and uploads that report with the existing receipts and JUnit artifact. This conditional wiring
does not select any nightly, performance, rented-host or release-qualification job.

The ordinary CI audit also registered 12 existing Linux-only supervisor test targets and added the
eight existing 0.74 workflows to the repository-wide topology catalog. Existing 0.73 entries,
the sole publication producer and existing CI artifact budgets remain unchanged. Reviewed expiring
exceptions describe the existing signed-request and content-addressed artifact identities; those
names and their consumers were not changed. Three previously unbounded sensitive steps now have
explicit deadlines, and start-bundle assembly has its own run/source/host concurrency group, distinct
from its child observation workflow. The touched kernel-attribution and host-capability jobs require
explicit dispatch, so a metadata push cannot start those host jobs. The other host workflows were
only inventoried, not dispatched.

Local preflight passed topology, 17 governance checks, 33 performance-contract tests, six gated-registry
tests, 23 governance regression tests and the topology unit test. The affected Python modules ran 13
tests with one Windows FIFO skip. On local WSL, the supervisor package passed 206 tests; its one
opt-in systemd user-bus test stayed ignored, and no real system service was mutated. A single Windows
workspace fmt invocation hit OS error 206 (command-line length); checking all 36 workspace packages
individually then passed without changing files. Performance-contract lint passed `clippy::all`, with
the existing supervisor dependency dead-code warning unchanged. The source identities,
dirty-documentation scope and non-promotable boundary are retained in
`local-runs/w12-ordinary-ci-preflight-94368d8a.json`. These checks do not replace full workspace evidence.

The first ordinary hosted dispatch is run `37515470936` on clean `9bf80b4b`, with all expensive,
nightly and qualification inputs disabled. Both push-triggered touched host workflows skipped every
job, confirming the new explicit-dispatch boundary. Local W11/W12 canaries on that exact source
returned expected red. While CI was still running, two completed jobs exposed missing metadata:
the existing draft lacked its mdBook wrapper/navigation, and the conservative memory scanner found
the already implemented `ProfiledStoreGuard` without a reviewed registry disposition. Both failures
were reproduced locally. The wrapper now includes the canonical draft, clearly labeled as draft;
no 0.73 publication record was rewritten. The guard borrows the map already accounted under
`ClientSurfaceState`, so its exact registry entry describes inline metadata and avoids double counting.
A new test proves normal-drop and unwind lock release for profiling on/off, with poisoning preserved.
Five ownership tests, inventory closure, strict transport lint, three release-doc checker tests,
documentation synchronization, links and mdBook passed locally. The timestamped partial CI record is
`local-runs/w12-ordinary-ci-first-dispatch-9bf80b4b.json`; it does not claim the final run outcome or
validate the subsequent repairs as hosted evidence. No replacement CI run is dispatched concurrently.

The same run later rejected the two internal workspace tools at dependency policy because their
manifests omitted license metadata; downstream Docker admission then rejected the failed Rust result
before interop execution. `cargo deny --locked check licenses` and a new inheritance regression both
reproduced the omission. The tools now inherit the existing workspace `Apache-2.0` metadata. No
license allowlist, exception, confidence threshold or lockfile changed. License checking and all
34 performance-contract tests pass locally; the original CI rejection remains retained, not rerun
against another source under its old identity.

The original run has now completed with failure. Its unchanged full workspace gate finished in
607,210 ms, including compilation, without timeout or missing JUnit: 3,756 tests ran, 3,754 passed,
two failed and 85 were skipped. This shows one hosted execution fits the existing 840-second
cadence; it is neither green evidence nor a product performance measurement. The two failures were
the missing guard review on the original source and a frozen 0.71 scenario digest mismatch. MSRV
independently rejected the same frozen input. The final negative record and downloaded receipt/JUnit
digests are retained in `local-runs/w12-ordinary-ci-final-failure-9bf80b4b.json`.

The digest failure exposed an error in our earlier inventory update: the topology catalog and owner
registry under `memory/0.71/` are frozen baseline inputs, not live extension points. Both snapshots
are restored exactly, without changing baseline identities or refreshing their hashes. The current
workflow catalog is now `docs/testing/ci-topology.json`; reviewed new symbols are composed from
`docs/testing/memory/ownership-registry-additions.toml`. Composition cannot replace or duplicate old
reviews, change schema/release/discovery scope, or bypass the existing production scanner, stale
review checks, expiry rules and focused-test resolution. A new regression checks all twelve frozen
prerequisite digests, not only the two files that exposed the mistake.

On clean repair source `83042ecc`, Windows passed eight baseline, five ownership, 35 performance
contract, seven governance and two merge tests; W11/W12 mutations returned expected red. The two
merge fixtures also passed on Linux WSL as development diagnostics. Format, strict library lint,
live topology and governance checks passed. Exact scopes and identities are recorded in
`local-runs/w12-frozen-registry-separation-83042ecc.json`. No third local workspace retry or expensive
qualification ran. A new exact-source ordinary CI run must validate all repairs together; the old
negative run remains negative.

While that new hosted run was in progress, a separate W12 audit found that the generic evidence
aggregator did not inherit the closed 0.74 admission boundary. An all-green ordinary receipt with
empty gated lists could otherwise advance a row to `ShipReady`. A new report regression failed
before repair. The aggregator now shares the performance-contract CLI's closed boundary, records
it explicitly in report reasons, rejects `--require-ship` for both `0.74` and `0.74.0`, and caps
ordinary-green rows at `FastGreen`. A synthetic all-green stage fixture verifies this ceiling while
preserving older releases' progression; it is not a fabricated measured receipt.

Clean source `d6294ca7` passed 109 xtask unit tests, 36 performance-contract tests and 11 aggregator
tests. Both CLI aliases rejected ship admission and W11/W12 canaries returned expected red. The
stage fixture also passed on Linux WSL. The prior registry checks passed in WSL after scoped Git
metadata environment overrides resolved the Windows gitdir path, without rewriting `.git`. A broader
Windows test-target lint still encounters the existing supervisor non-Linux dead-code warning;
the changed-library strict lint passes. These scopes, including the failed diagnostic attempts, are
retained in `local-runs/w12-ship-report-boundary-d6294ca7.json`.

Hosted run `37521028035` is bound to earlier `b795e58e`, not this admission repair. Its ordinary
test results remain useful for that source, but any generic `ShipReady` labels it might produce
must not be interpreted as 0.74 release authorization. No candidate, host qualification, Redis
comparison, six-hour run or 24-hour confirmation has been admitted by these changes.

That second run has now completed with failure. On its exact source `b795e58e`, all 3,761 executed
workspace tests passed, with 85 skipped across 477 binaries. JUnit reports 423.370 seconds of test
execution; the executor receipt covers compilation too and records 856,663 ms. This exceeds the
unchanged 840-second fast-suite budget by 16,663 ms, despite staying below the 2,400-second hard
timeout. Successful execution is not cadence compliance. The old report's 28 `ShipReady` rows are
invalid for 0.74 admission; they precede the closed-admission and cadence repairs. The immutable
negative result, successful jobs and artifact digests are retained in
`local-runs/w12-ordinary-ci-second-failure-b795e58e.json`.

The independent Rust failure was strict Linux supervisor lint, not the nonfatal cache symlink
warnings. Local WSL with pinned Rust 1.94.0 reproduced 24 production diagnostics plus four test-only
diagnostics. Mechanical fixes propagate response results directly, remove an unnecessary borrowed
slice and final option reborrow, and limit a Linux helper to Linux or tests. Private admission and
observation enums now use inline `ControlFlow` and `Option`; no `Box`, new allocation, wire-format
change or lock-lifetime change is introduced. This does not claim smaller stack layout or measured
allocation savings. Checkpoint writer arguments are grouped into an owned context with the same
values and channels. Existing fixture timing assertions are kept as runtime checks; no duration,
lease, authorization rule or lint threshold changed. The added observation regression proves that
an incomplete process pair cannot hide a terminal foreign-unit mismatch. The local supervisor
baseline passed 206 tests; after repair it passed 207, with the same opt-in systemd test ignored.

A separate new aggregator regression first failed on a receipt one millisecond above its budget.
The aggregator now enforces the already registered fast-suite cadence, accepting the boundary
and rejecting its first overrun. The 0.74 ordinary CI lane also explicitly runs `fast-suite-check`
on the generated receipts. Applying that checker to the downloaded second-run receipt returns the
expected failure for 856,663 ms. Hard timeout 2,400 seconds, workspace cadence 840 seconds and
aggregate budget 1,680 seconds remain unchanged. These checks repair tooling correctness, not
product throughput. The next proof is ordinary CI on one exact source containing all repairs,
with every costly lane disabled. Candidate identity remains unresolved and ship admission closed.

Clean source `a2ce6e5e` passed the complete strict Linux Clippy command matrix from ordinary CI on
local WSL with pinned Rust 1.94.0: workspace all-target/all-feature lint, HydraCache no-allocator
and system-allocator all-target lint, and jemalloc/mimalloc library lint. Windows strict xtask
library/test-target lint also passed. Final targeted checks passed 109 xtask unit, seven fast-suite,
36 performance-contract, 12 aggregator and 207 Linux supervisor tests, with one opt-in systemd
test ignored. Both admission aliases remained rejected, both canaries returned expected red and
the report had zero ship-ready rows. Exact scopes, earlier negative diagnostics and unchanged
budgets are retained in `local-runs/w12-linux-lint-and-cadence-preflight-a2ce6e5e.json`.

### Exact-source ordinary CI closes the reproduced tooling failures

[Run 37538251858](https://github.com/javaquasar/hydracache/actions/runs/37538251858), attempt one,
completed successfully on clean source `3ce743508a283a2a0e5612852e9331d9cc65381b` at
2026-10-06 22:42:29 UTC. All thirteen executed ordinary jobs passed, including pinned Linux strict
Clippy, public API checks, MSRV 1.88.0, memory contracts, migration conformance, docs and HC/2
admission. Every expensive qualification, reference, diagnostic, nightly and soak input remained
disabled. No rented-host service or product campaign was touched.

The complete workspace gate passed 3,765 tests with zero failures and 85 skips across 477 binaries.
Its executor receipt records 444,319 ms including compilation, within the unchanged 840-second
cadence and 2,400-second hard deadline. JUnit separately records 229.848 seconds of test execution.
The aggregate limit stays 1,680 seconds. This is one valid hosted cadence execution, not a stable
timing distribution: the fast-suite registry still has zero measured baselines. Different source
identities, test sets, caches and runners also prevent treating the earlier 856,663 ms result and
this execution as a paired performance improvement.

The downloaded artifact ZIP digest independently matches GitHub's digest. Nested JUnit bytes and
SHA-256 independently match the receipt. On the same clean `3ce74350` source, the receipt-aware
fast-suite checker passed and both `--require-ship` aliases rejected admission. The hosted and
independently regenerated reports agree: 28 `FastGreen`, zero `GatedGreen`, zero `ShipReady`, with
the explicit closed-admission reason. Both exact-source W11/W12 mutations returned expected red.
The first local verification attempt correctly rejected missing canonical JUnit and older local
canaries; verification succeeded only after materializing the actual verified downloaded files
and backing up prior generated canaries, without rewriting the original receipts or report.

The immutable summary, run/job identities, artifact/receipt/JUnit/report digests and verification
scopes are retained in `local-runs/w12-ordinary-ci-exact-source-green-3ce74350.json`. Earlier negative
runs remain negative. This closes their reproduced ordinary-CI defects on one later source; it
does not combine old-source results with local repairs, bind receipts to a later documentation
commit, or promote rejected proposal dispositions into product wins.

Next, W11/W12 still require an admitted C74 identity, candidate-specific native/reference and
long-run qualification, package/SBOM/advisory/license and supported-target proofs, and the final
immutable archive. Accepted product candidates remain zero, C74 remains `UNRESOLVED`, and ship
admission stays closed. Those prerequisites are not waived by ordinary CI, and expensive runs
are not started by this evidence update.

### Release-wide reasons remain visible outside JSON

A follow-up W10/W12 audit reproduced a reporting omission on `20194d5f`: JSON retained the closed
ship-admission reason, but CLI and Markdown showed only row-local reasons. Admission still rejected
`--require-ship`; the defect did not allow a release. It obscured why even ordinary-green rows could
not become ship-ready.

Two new regressions failed before repair. The CLI fixture uses an isolated clean synthetic Git
repository without work-item failures, so the release-wide reason alone must explain rejection.
It covers `0.74` and `0.74.0`, with and without `--require-ship`. The Markdown fixture preserves
every global reason before the work-item table, verifies existing pipe escaping and compares the
JSON contents unchanged. A separate empty-reason legacy fixture passed before and after repair.
These fixtures are not measured receipts. Temporary Git commands and CLI children discard
ambient worktree/index overrides and cannot mutate the real checkout.

The repair emits `release-evidence: release-wide: ...` and adds a nonempty-only Markdown
`Release-wide reasons` section. JSON schema, admission decisions, stages, counts, exit codes and
all budgets are unchanged. Local checks passed 111 xtask unit, 13 aggregator, 36 performance-contract
and seven fast-suite tests, all-target `cargo check`, all-target/all-feature strict Clippy and format.
On clean source `2056c151886869865e0f86b6421de81e13b75115`, all four real CLI combinations preserved
the same explicit reason in CLI/Markdown/JSON; normal generation exited zero, ship requests exited
one, and ship-ready rows stayed zero. W11/W12 canaries returned expected red and 17 governance
checks passed. Exact scopes and generated digests are retained in
`local-runs/w12-report-reasons-local-2056c151.json`.

No new full-workspace receipt was created or borrowed. The earlier successful hosted proof remains
historical evidence for `3ce74350`, not this reporting change. W10 still has zero accepted product
proposals and no C74 to compose. The kernel study confirms the W3 write-amplification owner, but
explicitly does not reopen its rejected implementations. Any future product retry needs a genuinely
different design, preregistered backpressure behavior and the same shallow/native/tail guards.
The plan's no-win outcome is to retain measurements and defer changes, not manufacture a candidate
from documentation or relax thresholds. No product retry, qualification or release was started.

### W3 feasibility: partial progress is not the mutation frontier

The next local step screens a proposed shallow-free W3 mechanism before product code. On baseline
`8bf4eb26`, the existing seven adversarial/adaptive tests passed. Source inspection shows that the
generic connection loop requires the previous response's entire `write_all` and `flush` to succeed
before executing another command. Completion refers to the supplied `AsyncWrite` boundary, not
TCP acknowledgement or peer receipt. Readiness or a first accepted byte cannot replace it.

Two deterministic integration guards now supply both SETs and QUIT in one read. One enumerates
every accepted prefix, zero through five bytes, of `+OK\r\n`; the serving future is explicitly
polled to its pending boundary. A separate connection sees the first SET and a miss for the second
at every cut, including a complete first reply with pending flush. Opening the gate permits the
second SET and three ordered replies. The other guard injects a failed flush after all five bytes:
the server fails loudly without executing the second SET or automatically retrying. Correctness
uses exact bytes/state, not sleeps; completion timeouts apply only after release.

The review therefore rejects readiness/progress-gated coalescing of already-executed SET responses
as a new mechanism. Completing each previous write and flush preserves semantics but supplies no
elimination of that previous write. This narrow result does not prove all transport optimizations
impossible. GET-only purity is unproven, and staged execution needs an explicit separate authority,
visibility, quota, audit and acknowledgement design; neither is authorized by this investigation.

Clean source `849fea3a2d802611191307996fb0dbd443aeb250` passed 152 redis-compat tests with 23 existing
opt-in cases ignored, and 37 performance-contract tests. Both affected packages passed all-target
checking, strict all-target/all-feature lint and format during development. The local performance
contract passed and W11/W12 canaries remained expected red. Exact scope, earlier metadata/format
diagnostics and outcomes live in `local-runs/w3-delivery-frontier-review-849fea3a.json`; the design
and frozen review policy are `w3-delivery-frontier-review.md` and `.toml`. W3 release-evidence now
requires these artifacts and all three new regression selectors alongside its original evidence.

No product code, dependency, native/shared hot path, frozen 0.73 artifact, qualification input or
numerical threshold changed. No new goodput, latency, CPU, allocation or memory measurement exists.
The older hosted receipt remains proof for `3ce74350` only. Previous negative candidates remain
negative, accepted product proposals stay zero, C74 is unresolved and release admission is closed.

### W3 staged-execution assessment: an overlay does not make the result stable

An explicitly authorized local architecture review now examines staged execution rather than
assuming that a private overlay resolves the previous mutation-frontier failure. The compatible
path still completes the preceding response write and flush, validates current state/time/admission,
commits the next command, and only then writes its actual response. This remains **not distributed
transactions** and introduces no executor, public API, durable/wire artifact or new store ownership.

Four new deterministic integration guards enumerate all six cuts of the first SET reply. Direct
native invalidation changes a queued `SET NX PX` from failure to success; native replacement changes
a queued GET's value; exact injected-clock expiry changes NX and its replacement TTL origin; native
PUT consumes remaining quota before a queued SET, which must return the exact error and one audit
without a third mutation. These 24 controlled traces prove the enumerated state/progress outcomes,
not numerical native performance or exhaustive authorization/event/idempotency/multi-key semantics.

A finite ordering model enumerates six permutations for a fixed pre-encoded two-reply batch. Its
three chronological shared-write schedules cannot both delay SET 2 until flush 1 and commit SET 2
before exposing its response. The ordinary separate-write sequence is a passing positive control.
This checks the model's stated generic-writer assumptions, not all possible transport designs.

The design screens overlays, early/late commit, quota/store-lock reservation, rollback and post-flush
revalidation. The first variants change semantics or native admission/progress; revalidation can
preserve the boundary but supplies no fixed-batch write reduction. No reviewed staged design is
admitted for implementation. Dynamic transport-specific work remains a separate unproven mechanism,
not permission to relabel several kernel writes as one application call.

Clean source `4caa3b69049b666906ff136c974971b5c3f9b588` passed 157 redis-compat tests with 23 opt-in
cases ignored, and 90 targeted xtask tests (38 contract, 13 evidence, 23 governance, 16 doc-check).
Both affected packages passed all-target checking, strict all-target/all-feature lint and format
during development; the local contract passed and W11/W12 canaries remained expected red. The
design/policy are `w3-staged-execution-review.md` and `.toml`; source-bound scope, negative fixture
diagnostics and outcomes are in `local-runs/w3-staged-execution-review-4caa3b69.json`. W3 manifest
selectors retain all earlier evidence and require the four guards, model and policy regression.

Runtime, native/shared paths, dependencies, numerical thresholds and qualification inputs/digests
are unchanged. There is no new throughput, latency, CPU or allocation measurement, no rented-host
workload, and no reused CI receipt for this source. The recommendation is to retain W3's negative
disposition; accepted product proposals remain zero, C74 unresolved and ship admission closed.

### W9b large-GET response encoding: a local D1 owner, not a product win

The separate serial-scratch hypothesis now has preregistered attribution on clean
source `cc0b2fbfc0b7d9c4c9f37e96089f37134fdf546f`. The v3 stage profiler compares
independently initialized canonical execution against the same execution plus
the existing encoder. Exact payload/frame validation performs no allocations;
dispatches, mutations, checksums, final value and cardinality are reconciled.
Three fresh-process repeats of all five cells ran in rotating order, without
retries or selected-best samples. All 15 raw receipts are retained and hash-checked.

GET encoding increments are 264 B/op at 256 B, 4,105 B/op at 4 KiB and
1,048,588.168 B/op at 1 MiB: 16.97%, 31.39% and 33.325% of the **measured
execution-plus-encode response path**, respectively. SET encoding is 8 B/op at
both 256 B and 4 KiB. Every allocation total repeats identically. At 1 MiB the
canonical subtraction exceeds isolated encoding by 84 bytes per 500-operation
window; this residual remains unassigned, not attributed to the encoder.

This is not complete RESP/network allocation, CPU, latency or goodput evidence.
Large GET supplies an owner for D2 serial scratch review, not acceptance or proof
of the unchanged 20% end-to-end floor. Previous W3/W4 rejections remain unchanged.
The next isolated design must retain previous complete write/flush before the next
execution, canonical pipeline-one behavior, bounded scratch lifetime before the
next read, no cross-connection pool and no larger idle/output bound. Native,
ClientSurfaceState and embedded remain separate D3 non-regression controls.

See [the assessment](w9b-response-buffer-attribution.md),
`w9b-response-buffer-attribution-contract.toml` and
`local-runs/w9b-serial-encoder-owner-cc0b2fbf.json`. Source-bound checks passed six
profiler tests and 91 xtask tests (39 contract, 13 evidence, 23 governance, 16 docs);
the evidence stage adds a 40th contract regression verifying all archived cells.
Affected format, check, strict lint and non-promotable contract passed; W11/W12
canaries remained expected red. Validator isolation retains the original failed
75-byte parallel-test diagnostic without relaxing the zero-allocation requirement.
Runtime, root dependencies, qualification manifest/pinned digests, frozen 0.73 and
thresholds are unchanged. No rented-host or expensive workload ran; C74 remains
unresolved with zero accepted product proposals.

### W9b D2: serial GET scratch implemented, feature off, D3 not started

The new large-response follow-up is preregistered at `4c5b16d7` and implemented
at `b8cc7c6c32afb6e4f7fa1efaf92e14846f7102b1` under the default-off feature
`experimental-resp-serial-scratch-074`. This is not W3 batching or a reopened
negative candidate. Only consecutive equal-sized successful GET bulk replies
from 4 KiB to 1 MiB within one received read are eligible. First reply is
canonical, second allocates scratch, third/later reuse its address. Every reply
still completes one separate write and flush before the next command executes.

Capacity is bounded at 1,048,588 bytes and dropped on class/command/error boundaries
and before another read. Unit/property checks prove exact RESP2/RESP3 output and
pointer reuse; this is not an end-to-end allocation or RSS measurement. Scratch
overlap with the next reducer may increase peak-live memory; no memory regression
is waived. AUTH/subscription/malformed/QUIT/cancellation and native progress,
exact expiry and quota interleavings are explicitly exercised after actual reuse.

Clean source passed 166 feature-off and 171 feature-on RESP tests (23 existing
opt-in cases ignored per build), 18 feature-on server lifecycle cases and 94
xtask tests. Format, affected/direct-server check and strict lint, non-promotable
contract, governance and W11/W12 expected-red canaries pass. A Windows executable
lock during parallel validation is retained; the affected checks subsequently
passed sequentially from the same clean source, with no performance attempt retry.

See [the design and remaining gates](w9b-serial-scratch-design.md),
`w9b-serial-scratch-proposal.toml` and
`local-runs/w9b-serial-scratch-semantic-b8cc7c6c.json`. The registry lists the new
follow-up separately; terminal negatives and qualification/pinned input digests
remain unchanged. Default runtime path, public codecs, native/shared/embedded
sources, root lockfile, time/expiry/reducer and durability are unchanged.
Experimental source is now present, but no default activation or release win is
claimed. No rented-host or expensive workload ran. D3 must first seal source/
feature/binary identities, counterbalanced pairs, per-native controls and peak/
idle/RSS guards; feature-on CI proof is still required before promotion.

### W9b D3a: gross allocation win rejected on active requested-live peak

The contract/instrumentation was sealed at `3171d02a` before any numerical sample.
All 140 fresh-process attempts completed without failure/retry: seven cells,
five AA and five counterbalanced AB pairs each. Source/feature/binary/tool-lock/
toolchain/workload identities and exact results/cardinality reconcile. This is
scripted RESP2 plaintext IO with a tool-only System requested-layout allocator,
not socket, scheduled-latency, RSS-retention or release qualification evidence.

GET 4 KiB/pipeline 50 reduces gross bytes/op by 25.97% but raises window-live peak
above start by 16.33% (25,134 to 29,239 bytes). GET 1 MiB/pipeline 10 reduces gross
by 26.65% but raises peak by 49.60% (2,114,091 to 3,162,679 bytes). All five paired
values repeat; AA and unaffected allocation/live-peak ratios are 1.00. Extra peak
equals one encoded frame, retained during the next canonical reduction. Next-read
and post-close owner increments have no increase; no leak is claimed or needed
to reject a higher live peak. Gross floor passes, memory ceiling fails.

The candidate is selectively removed: feature/helper/writer no longer exist,
runtime source/manifest match the legacy default baseline. Adversarial integration
guards remain. The standalone tool's retired flag fails before workload instead
of silently benchmarking a feature-on no-op. Historical experimental reproduction
requires the exact sealed source, not a new attempt presented as the old one.

Full raw/attempt/seal/summary evidence: `local-runs/w9b-d3a-3171d02a/`;
[decision and scope](w9b-serial-scratch-d3-result.md). Earlier terminal negatives,
root lock, native/embedded code and qualification/pinned digests are unchanged.
No rented host or expensive run executed. The broader native/timing/retention/
secure-concurrent/CI controls were not run for a red candidate, not waived.
Accepted product proposals remain zero; C74 unresolved and ship admission closed.

### Separate D1: canonical GET response reduction owns another payload copy

After selectively removing serial scratch, a new independent owner screen was
preregistered at `2562f6f7`. All 18 fresh-process attempts succeeded: six fixed
cells, three rotating repeats, no retry. The tool measures dispatch, borrowed
reduction and their combined execution using public canonical APIs, not an
experimental product branch. Runtime/native/store sources remain unchanged.

Nonempty GET reduction adds one successful allocation and exactly 64 / 4,096 /
1,048,576 requested bytes per operation; empty hits, misses and SET add zero.
For 4 KiB GET, dispatch/combined gross bytes are 4,432 / 8,528 and window-live
peaks are 4,432 / 8,294. For 1 MiB they are 1,048,912 / 2,097,488 gross bytes and
1,048,912 / 2,097,254 peak bytes. Every repeat agrees and each window releases its
temporary owners. Checkpoint deltas are one payload; peak deltas are not additive.

This is fixed-plan, injected-clock, requested-layout attribution, excluding
decode/translation/encoding/IO and timing. It is not an end-to-end candidate gain,
RSS-retention proof or native numerical guard. Full packet:
`local-runs/response-owner-2562f6f7/`; [method, results and next boundary](response-reduction-attribution-result.md).
A future private consuming-GET reducer requires a new D2 contract and D3; no
scratch rescue or earlier rejection is reopened, no accepted proposal is added.

### Separate D2: private GET response owner transfer, default off

The D1 follow-up is now preregistered (`bf42f776`, capacity amendment `d07c35f0`)
and implemented at `2379698d` as `experimental-resp-get-owner-074`. It transfers
one successful nonempty, length-sized response Vec in the private executor only;
every other case uses the unchanged public borrowed reducer. No native/store,
encoder, pooling, batching or write/flush-frontier change is combined with it.

Both builds passed 178 RESP tests and 18 server lifecycle cases; the seeded
128-case property oracle explicitly covers transfer and spare-capacity fallback.
The original failed property assumption and its correction are retained. Ordinary
feature-on CI is enrolled at `b3f9d2c4`, not claimed hosted-green by enrollment.
See [implementation, semantic evidence and next boundary](get-response-owner-design.md)
and `local-runs/get-response-owner-semantic-2379698d.json`.

This is D2 semantic evidence, not a new numerical comparison. D3 must separately
seal the finite attempt set, A/A controls, at least five counterbalanced pairs,
20% end-to-end large-GET allocation floor and unchanged live-memory/native guards.
Accepted proposals stay zero; C74 unresolved. No costly workload or qualification
manifest change is part of this step.

The local verification follow-up retains clean-`3f13f1d1` W11/W12 ExpectedRed
receipts and exact scope in `local-runs/get-response-owner-checks-3f13f1d1.json`.
All 99 targeted xtask, 16 replay and three docs-script tests passed. Local evidence
aggregation still has zero fast-green/gated-green/ship-ready rows; structural
mapping is not full workspace/hosted qualification or accepted product changes.

### Private GET owner D3a: allocation/memory early screen passed

Source `213e9e0a` seals the separate contract/tool before any sample. All 180
attempts (nine cells, five A/A and five counterbalanced A/B pairs each) succeeded
without retry. Full connection-path gross allocation falls 26.8697% for GET
4 KiB/p50 and 33.3159% for GET 1 MiB/p10, exactly one payload allocation/op.
Requested-live peak is non-increasing, owner increments match, and SET/empty/miss
controls are unchanged. Every A/A ratio is 1.00; all five counts repeat.

All 362 raw/attempt/seal/summary files are retained in
`local-runs/get-owner-d3a-213e9e0a/` and replayed with raw SHA checks. See
[result, denominators and remaining guards](get-response-owner-d3-result.md).
This is scripted RESP2 plaintext requested-layout evidence, not socket throughput,
CPU/p99, RSS retention or native numerical nonregression. Phase B needs a separate
sealed contract; no expensive run is authorized. Feature remains off, accepted
proposals zero, C74 unresolved and qualification manifest unchanged.

Verification follow-up retains clean-`d32fbb18` W11/W12 ExpectedRed receipts and
all-362-file committed-blob/raw-byte identity in
`local-runs/get-owner-d3a-verification-d32fbb18.json`. Local structural aggregation
still has zero fast-green/gated-green/ship-ready rows; no full qualification is
implied by archive verification or passing local guards.

### Private GET owner phase B0: independent unprofiled controls

The separately preregistered local B0 contract and `get-owner-controls-074`
standalone tool separate default-allocator timing from requested-layout counting.
Old native/RESP profilers cannot certify unprofiled timing merely by turning off
product instrumentation: their loadgen allocator remains active. They and all
historical packets are left untouched.

Ten embedded/ClientSurfaceState/real-loopback-TCP cells at logical concurrency
1/8 run 400 finite fresh processes, with five A/A and counterbalanced A/B pairs
per lane/cell. Placement precedes warmup via READY/GO. Exact payload/trace/counts,
closed-loop operation vs whole-batch latency, CPU precision and unchanged native
floors are enforced; a noisy/invalid attempt is retained without retry/tuning.
This is a preregistration/instrumentation step, not a numerical result or full D3.
See [scope, methodology and required follow-ups](get-response-owner-phase-b0-design.md).
HC1/HC2, matched mTLS/RESP3, higher concurrency, scheduled per-operation latency,
retention/refill and current-source hosted CI remain required. Feature stays off,
accepted proposals zero, integrated C74 unresolved, qualification unchanged.

The first sealed B0 launch (`18ee8cdc`) was refused before a benchmark process
started: 57.8125% background CPU versus the unchanged 10% ceiling. One refused
attempt, zero numerical pairs, no native/throughput result or retry. All three
raw files and an offline no-spawn audit survive; see
[invalidation, exact boundary and next work](get-response-owner-phase-b0-result.md).

### Scheduled latency and independent HC1/HC2 instrumentation

The bounded extension of the canonical loadgen calendar now retains each original
offer, queue/execution boundary, terminal class and censored incomplete owner.
Separate scheduled/service histograms and all-offer goodput accounting prevent
a service-only or successful-response-only tail from silently hiding overload.
HC1 production HTTP routes and HC2 production gRPC/mTLS listeners have independent
real-loopback GET/PUT byte-oracle controls, including binary/1 MiB values, missing
keys, tenant separation and anonymous/foreign-CA rejection.

These are local semantic fixtures, not measured native floors or a retry of B0.
No numerical CLI/series exists; real transport 32/128, scheduled RESP per-response
adapter, security-matched cohorts, retention/refill and hosted CI are still open.
See [methodology and remaining proofs](get-response-owner-scheduled-controls-design.md).
The private feature remains off and qualification/integrated admission unchanged.
Focused checks at clean `a6047895`: 19 tool tests per variant and 103 xtask tests,
strict lint/check, docs/book and 17 governance checks passed. The
[check summary](local-runs/get-owner-scheduled-instrumentation-checks-a6047895.json)
is local functional evidence only, not native nonregression or hosted CI.

The subsequent semantic extension adds a bounded RESP2 GET per-frame sidecar on
one real TCP connection at outstanding ceilings 1/10/50. It keeps cancelled FIFO
owners, supports partial writes while reading replies, validates wire identities
and timestamps, and includes wire-owner drain in elapsed accounting. Frame
observation is distinct from verified-operation latency; no batch average or late
reply can become a successful operation. Native GET/PUT controls now also test
barrier-synchronized 32/128 slots and HC2 live connections/cleanup. These are
functional prerequisites only; full measured/nonregression requirements remain
open, and the earlier check summary remains bound to its original source.

Clean-source verification at `a8440175`: 29 tests per tool variant (12 scheduled,
eight native, nine RESP), 104 focused xtask tests, strict lint/check, scoped
formatting, docs/book and 17 governance checks passed. The separate
[RESP FIFO check summary](local-runs/get-owner-resp-fifo-checks-a8440175.json)
records exact source identities; this is not a benchmark, B0 retry, full workspace
verification or hosted receipt. No numerical performance conclusion is added.
