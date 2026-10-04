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
read-only status service. Packet boundaries and the 65,536-byte limit are enforced by the socket
layer; `SO_PEERCRED` plus `/proc/<pid>/status` supply uid, primary gid and supplemental groups.
The wire envelope requires a strict Ed25519 authorization document for mutating operations, while
status remains read-only but still checks the admitted peer, repository and actor. The client
verifies the response digest and request/campaign binding before printing it. End-to-end WSL tests
round-trip the exact durable state and reject stale revisions.

This is still not the complete systemd supervisor. `start`, `seal` and `abort` authenticate and
then fail with stable internal error 11 at the live server boundary. `attach` now executes the
implemented journal, manifest, systemd, `/proc`, cpuset, checkpoint, admitted-host and lease guards
and durably records an exact response. There is no unconditional rejection left in the attach path:
a lease can be granted only when every guard matches. Production provisioning, bounded diagnostics,
an accepted attach rehearsal and real host fault rehearsals remain incomplete, so release admission
stays closed.

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
```

The supervisor library also creates the final archive without trusting directory iteration order or
filesystem metadata. It sorts bytewise-normalized paths, emits fixed uid/gid/mode/mtime tar
headers, uses deterministic single-stream zstd compression, writes an external SHA-256, enforces
the frozen file/byte limits, and refuses create-overwrite, nested outputs, symlinks, hardlinks and
non-regular files. Tests compare byte-identical archives built from differently ordered trees and
inspect every tar header. The live sealing state transition and Linux filesystem rehearsal remain
incomplete.

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
symlink/hardlink substitution and cross-campaign exclusion. This remains local non-promotable
evidence: the real StartTransientUnit adapter, accepted live `start` response, terminal marker
release, C74 orchestration, sealing/abort/diagnostics and admitted-host rehearsal are still open.
The full local WSL suite now passes 101 ordinary supervisor tests with only the explicitly manual
real-system-bus inspection ignored; the Windows-portable subset passes 77 tests, and
`clippy -D warnings` is green.

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
