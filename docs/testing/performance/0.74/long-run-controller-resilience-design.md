# HydraCache 0.74 long-run controller resilience design

Status: implementation specification; no 0.74 candidate evidence is admitted by this document.

This document turns the W11 controller-resilience requirements into an implementation contract.
Its purpose is narrow: loss of a GitHub Actions controller, network session, or runner service must
not discard a valid six-hour or 24-hour role that is still executing on the admitted host. It does
not make a dead process resumable and does not permit statistical continuation across process,
binary, host, boot, workload, or lease identity changes.

The W0 file long-run-controller-resilience-contract.toml freezes all values marked contract value
below before candidate data is observed. Examples and defaults in this document are not authority
until copied into that reviewed contract.

## Decisions and non-goals

The implementation uses one Rust workspace package at tools/long-run-supervisor-074. It builds one
static-shape executable named hydracache-long-run-supervisor-074. The installed, root-owned copy
provides serve mode. GitHub invokes an independently hash-verified copy of the same binary as an
unprivileged Unix-socket client. Python remains only at the workflow orchestration boundary and
cannot own a measured process.

The design deliberately does not:

- restart or reconstruct a harness, product daemon, sampler, durable companion, or load generator;
- concatenate measurements from different processes or boots;
- allow arbitrary commands, environment variables, paths, shell fragments, systemd properties, or
  cgroups from a workflow;
- treat a PID alone, a GitHub heartbeat, stdout activity, or an unverified state file as progress;
- upload mutable host state as green evidence before the role and packet are sealed;
- auto-retry a rejected campaign or start a replacement under the same campaign id.

## Trust and ownership boundary

There are four owners:

| Owner | May do | Must not do |
| --- | --- | --- |
| protected GitHub workflow | authorize a frozen manifest, request typed operations, observe state, upload sealed artifacts | own process lifetime, change frozen fields, submit commands |
| provisioned supervisor | validate manifests, create and inspect measured systemd units, persist journals, enforce progress and lease, seal packets | choose product identities or thresholds, declare release pass |
| measured harness | execute the frozen scenario, emit phase progress and raw measurements | admit attach, mutate campaign identity, verify itself |
| offline xtask verifier | parse schemas, recompute all hashes/guards, admit evidence | query mutable supervisor state as proof, repair evidence |

The supervisor is a pre-installed root system service because it creates tightly constrained
transient systemd units. Its socket API is the only runner-facing privilege boundary. Measured
processes run as the dedicated unprivileged hydracache-perf user. The workflow runner is only in the
hydracache-perf-client group.

## Repository and installed file map

Implementation adds these repository paths:

| Path | Responsibility |
| --- | --- |
| tools/long-run-supervisor-074/Cargo.toml | non-published Rust package and pinned dependencies |
| tools/long-run-supervisor-074/src/main.rs | CLI parsing and exit-code mapping |
| tools/long-run-supervisor-074/src/protocol.rs | versioned request/response types and size limits |
| tools/long-run-supervisor-074/src/auth.rs | peer credentials, role authorization, request replay protection |
| tools/long-run-supervisor-074/src/manifest.rs | strict campaign manifest parsing and allowlist validation |
| tools/long-run-supervisor-074/src/host_receipt.rs | canonical admitted-host collection and live revalidation |
| tools/long-run-supervisor-074/src/start_evidence.rs | create-new staging validation and immutable campaign evidence import |
| tools/long-run-supervisor-074/src/state.rs and state_store.rs | phase-aware state model, campaign locks and atomic snapshots |
| tools/long-run-supervisor-074/src/host_execution.rs | host-wide active-campaign claim retained across supervisor restart |
| tools/long-run-supervisor-074/src/event.rs and mutation.rs | request/lifecycle event chain and event-ahead snapshot recovery |
| tools/long-run-supervisor-074/src/spawn.rs and start_lifecycle.rs | durable spawn intent/result and crash-safe I74/C74 start coordinator |
| tools/long-run-supervisor-074/src/systemd_unit.rs and process_identity.rs | fixed-namespace unit and process identity inspection |
| tools/long-run-supervisor-074/src/checkpoint_evidence.rs and watchdog.rs | chain verification and phase-aware progress |
| tools/long-run-supervisor-074/src/diagnostics.rs | bounded allowlisted diagnostic capture |
| tools/long-run-supervisor-074/src/artifact.rs and archive.rs | canonical packet assembly, deterministic archive and artifact limits |
| tools/long-run-supervisor-074/tests/ | protocol, state, crash, fault and security fixtures |
| scripts/perf/long-run-supervisor-074/ | provisioning, systemd, sysusers, tmpfiles and admission scripts |
| scripts/perf/performance_long_run_074.py | frozen manifest builder and workflow adapter |
| scripts/ci/monitor-long-run-campaign-074.py | disposable status/attach monitor |
| crates/xtask/src/long_run_campaign.rs | offline independent verifier |
| .github/workflows/performance-long-run-qualification-074.yml | reusable protected operation workflow |
| docs/testing/performance/0.74/ | frozen contract, schemas, fixtures and attempt receipts |

Provisioning installs:

| Installed path | Ownership and mode |
| --- | --- |
| /opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 | root:root 0755, digest in host receipt |
| /etc/hydracache-perf/supervisor-074.toml | root:root 0644, no secrets |
| /etc/systemd/system/hydracache-performance-supervisor-074.service | root:root 0644 |
| /run/hydracache-perf/supervisor-v1.sock | root:hydracache-perf-client 0660 |
| /var/lib/hydracache-performance/campaigns | hydracache-perf:hydracache-perf 0750 |
| /var/lib/hydracache-performance/staging | hydracache-perf:hydracache-perf 0750 |

The provisioning receipt binds file hashes, uid/gid values, service unit properties, systemd and
kernel versions, state mount device/UUID/options, free-space minimum and admitted cgroup version.
Host admission rejects any difference.

## Service confinement and process lifetime

The main unit is enabled before a campaign and is not a child of actions.runner. It uses:

~~~ini
[Service]
Type=notify
ExecStart=/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 serve
User=root
Group=root
Restart=on-failure
RestartSec=2s
RuntimeDirectory=hydracache-perf
RuntimeDirectoryMode=0750
StateDirectory=hydracache-performance
StateDirectoryMode=0750
UMask=0027
NoNewPrivileges=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectSystem=strict
ProtectHome=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
RestrictRealtime=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
ReadWritePaths=/var/lib/hydracache-performance /run/hydracache-perf
SystemCallArchitectures=native
KillMode=process
Environment=RUST_BACKTRACE=0
~~~

W0 freezes the exact additional syscall/address-family allowlist after a host rehearsal. The unit
must retain only the privilege needed for systemd StartTransientUnit and read-only proc/cgroup
identity checks. If the host cannot express that narrow privilege safely, controller resilience is
not admitted on that host.

Each role runs in its own transient service named:

~~~text
hydracache-perf-<first-24-hex-of-campaign>-<i74|c74>.service
~~~

The supervisor constructs the complete transient unit itself. The request cannot supply properties.
The unit has User=hydracache-perf, Group=hydracache-perf, Restart=no, RemainAfterExit=yes,
KillMode=control-group,
Delegate=no, a contract-frozen CPUAffinity and memory/FD/process limits, WorkingDirectory inside the
campaign staging directory, RuntimeMaxSec equal to the frozen role duration plus diagnostic grace,
and StandardOutput/StandardError append targets inside that campaign. ExecStart is an argv array
selected from one compiled operation template; no shell is involved.

Only an allowlisted environment is passed: campaign/role identifiers, locale/timezone, evidence
directory, frozen scenario path and explicitly named non-secret tuning values. CI variables,
especially RUNNER_TRACKING_ID, GITHUB_*, CI, SSH_AUTH_SOCK, tokens and inherited PATH, are removed.
PATH is a fixed service value. Secret material is delivered only through root-created credential
files referenced by reviewed secret identifiers; secret bytes never enter a manifest or artifact.

Controller cancellation therefore kills the client/monitor only. It cannot kill the transient role.
A supervisor restart also must not kill a role: KillMode=process leaves the transient unit owned by
systemd. At startup the supervisor recovers durable campaign state, queries systemd and revalidates
the unit, PID, start ticks, boot id and cgroup before accepting another request.

## Socket protocol and authentication

The server listens on one Unix SOCK_SEQPACKET socket. A message is UTF-8 JSON no larger than 65,536
bytes. Trailing bytes, unknown fields, duplicate JSON keys, floats where an integer is required, and
more than one packet per request are rejected. SOCK_SEQPACKET avoids partial framing ambiguity.

The common request envelope is:

~~~json
{
  "schema_version": 1,
  "request_id": "UUIDv4",
  "operation": "start",
  "campaign_id": "64 lowercase hex",
  "expected_state_revision": 0,
  "manifest_path": "/var/lib/hydracache-performance/staging/<campaign>/campaign-start.json",
  "manifest_sha256": "64 lowercase hex",
  "controller": {
    "github_repository_id": 0,
    "github_run_id": 0,
    "github_run_attempt": 0,
    "github_job": "string",
    "actor_id": 0
  }
}
~~~

The initial revision-zero I74 start includes `manifest_path`. The later C74 start addresses the
existing campaign with the same immutable manifest digest, omits the path and must name the exact
`I74_SEALED` revision. Attach, status, seal and abort also address an existing campaign without a
manifest path. Abort additionally requires a frozen reason enum and an approval nonce created by
the protected environment. Status is read-only and does not require a controller lease. Unknown
operations fail closed.

The server authenticates SO_PEERCRED uid/gid and then checks supplemental group membership from the
OS, not request JSON. Start, attach, seal and abort also require a short-lived signed authorization
document created by the protected GitHub environment. The document binds request_id, operation,
campaign_id, manifest digest, repository id, run id, actor id, issue time and expiry. W0 freezes the
public verification key; private signing material never reaches the host.

Every response has schema_version, request_id, campaign_id, ok, state_revision, server_time,
result/error_code and response_sha256. The response digest covers RFC 8785 canonical JSON excluding
response_sha256 and is also written to the event journal before reply.

CLI exit codes are stable:

| Code | Meaning |
| --- | --- |
| 0 | operation completed or idempotently already completed |
| 2 | local CLI usage or schema error |
| 3 | authentication/authorization failure |
| 4 | campaign not found |
| 5 | identity, revision or duplicate-start conflict |
| 6 | frozen invariant or attach predicate rejected |
| 7 | campaign is terminal failed/incomplete |
| 8 | lease expired or outside allowed time |
| 9 | journal, checkpoint or artifact corruption |
| 10 | host/campaign lock busy; no mutation occurred |
| 11 | supervisor/systemd/internal failure; incomplete receipt retained |

The client prints exactly one response JSON object to stdout. Human diagnostics go to stderr with
secrets and credential paths redacted.

## Campaign identity and manifest

The protected start job generates a 256-bit random nonce before contacting the server. It computes
the campaign id exactly as specified by W11 and writes the start manifest to a new staging directory
created with O_EXCL semantics. The manifest uses schema version 1 and contains:

- campaign id, nonce digest, repository id and protected-environment authorization identity;
- contract, scenario, host receipt, lease and tooling SHA-256;
- full I74/C74 source commits, tree hashes, Cargo.lock hashes and dirty=false assertions;
- absolute installed binary paths, file SHA-256, size, inode, device, uid/gid and modes;
- exact argv templates and canonical environment digest with secret identifiers only;
- role order, phase durations, seed, workload, offered load, estimator and threshold digests;
- host/machine/boot/mount identities, isolated and housekeeping cpusets;
- checkpoint cadence, progress warning/failure gaps, diagnostic grace and lease deadline;
- stdout/stderr/diagnostic/final artifact byte and file-count limits;
- expected output schemas and all required final guard identifiers.

RFC 8785 canonical JSON is used anywhere this design says canonical JSON. Schema validation precedes
hashing. Manifest numbers are bounded integers in explicit units; NaN, infinity and floating-point
duration/resource values are forbidden.

The server opens the staging directory with openat2 RESOLVE_BENEATH, RESOLVE_NO_SYMLINKS and
RESOLVE_NO_MAGICLINKS. It rejects symlinks, hard-linked writable inputs, group/world-writable
parents, wrong owners/modes, device changes, non-regular inputs, hash/size changes and paths outside
the configured staging root. It copies accepted inputs into an immutable campaign input directory,
fsyncs files and directories, then verifies them again immediately before StartTransientUnit.

The local importer now implements the create-new, bounded regular-file, single-link, canonical
digest and direct-child portions of this boundary. It validates the start manifest and host receipt
together, writes `0400` copies into a private temporary campaign directory, fsyncs them and
atomically renames that directory to the final campaign id. A pre-existing final or interrupted
temporary directory is a rejection. Linux ownership/mode admission for externally created staging
parents and the later `openat2` descriptor walk remain part of the production-host start slice.

## Persistent layout

Each campaign directory contains:

~~~text
campaign-start.json
campaign-start.sha256
host-observation.json
host-observation.sha256
state.json
state.previous.json
events.jsonl
events.head
lock
i74-spawn-intent.json
i74-spawn-intent.sha256
i74-spawn-result.json
i74-spawn-result.sha256
inputs/
roles/i74/{checkpoints.jsonl,checkpoints.head,stdout.log,stderr.log,diagnostics/,raw/,seal/}
roles/c74/{checkpoints.jsonl,checkpoints.head,stdout.log,stderr.log,diagnostics/,raw/,seal/}
final/seal/
~~~

Files are never reused by another campaign. Mutable log limits are enforced while writing. A
campaign whose limit is reached is stopped and sealed incomplete; truncating an over-limit stream
and continuing is forbidden.

The campaign-root directory separately contains `.host-execution.lock` and `active-campaign`.
The flock serializes a live mutation owner, while the create-new marker survives supervisor restart
and prevents a different campaign from using the host until a later reviewed terminal cleanup
slice releases it. C74 uses the corresponding `c74-spawn-*` evidence names.

`host-observation.json` is create-new root evidence with mode `0400`. It binds the pre-existing
reference-host freeze receipt, machine/boot/kernel identity, command line, campaign mount
device/options, complete online/isolated/housekeeping CPU partition, per-CPU governors, the frozen
sysctl set and the exact installed supervisor binary. Attach re-collects this shape and compares it
to the persistent receipt, start manifest and durable state; an unavailable probe is a rejection,
not an omitted field.

The supervisor serializes mutations with:

1. one nonblocking host execution flock allowing at most one active campaign on the admitted host;
2. one per-campaign exclusive flock for state changes;
3. a monotonic state_revision compare-and-swap from the request;
4. a request replay map keyed by request_id and request digest.

Concurrent status readers use a shared campaign lock. The same request id plus same digest returns
the recorded response. Reusing an id with different bytes is an authorization failure. A repeated
start for an existing manifest is idempotent only if no spawn boundary was crossed twice; a start
for any other active campaign returns conflict before systemd is called.

## State machine

The campaign state enum is:

~~~text
PREPARED
I74_STARTING -> I74_RUNNING -> I74_TERMINAL -> I74_SEALED
              \-> FAILED_INCOMPLETE
I74_SEALED -> C74_STARTING -> C74_RUNNING -> C74_TERMINAL -> COMPLETE_SEALED
                              \-> FAILED_INCOMPLETE
any nonsealed state -> ABORTED_INCOMPLETE
any live state -> LEASE_EXPIRED_INCOMPLETE
any state with corrupt durable history -> CORRUPT_QUARANTINED
~~~

Attach and status are observations, not workload state transitions. A controller lease subrecord
changes independently and contains holder request id, authorization digest, attach time, expiry and
last observed state revision. The durable lease also retains the signed repository, run and actor
principals; a later mutation cannot compare authorization-document digests because every operation
has distinct signed bytes. `github.run_attempt` is not a lease principal because the authorization
body does not sign it. Losing a controller lease does not stop a healthy role and never starts one.
Seal and abort require an unexpired controller lease for those exact signed principals.

Allowed commands are:

| Current state | Operation | Preconditions | Durable effect |
| --- | --- | --- | --- |
| absent | start | valid new manifest, host lock free, active product lease | PREPARED then I74_STARTING; exactly one systemd call |
| I74_SEALED | start | exact revision and unchanged imported evidence | C74_STARTING; exactly one independent systemd call |
| live/terminal | status | authenticated peer and exact manifest digest | append observation event only when requested by contract |
| any non-corrupt | attach | every attach guard exact, no competing mutable controller | append controller event and grant/renew controller lease |
| I74_TERMINAL | seal | process exited successfully, checkpoints/final guards valid | deterministic I74 seal then I74_SEALED |
| C74_TERMINAL | seal | both role packets and final guards valid | deterministic final seal then COMPLETE_SEALED |
| nonsealed | abort | protected approval and exact revision | terminate unit, diagnostics, ABORTED_INCOMPLETE |
| failed/incomplete | seal | terminal evidence internally consistent | immutable incomplete seal; never green |

Before spawning, the supervisor durably records a spawn_intent containing the transient unit name,
manifest digest and nonce. It then calls systemd and records spawn_result. Recovery handles the
crash window by querying exactly that unit name. If no unit exists, the attempt becomes
FAILED_INCOMPLETE; recovery never calls StartTransientUnit. If one unit exists and all identities
match, it is adopted. More than one matching process quarantines the campaign.

The local coordinator implements these ordering rules behind a typed spawn backend. It appends
PREPARED, I74_STARTING and C74_STARTING lifecycle events before their state snapshots, writes each
role's independent intent before the backend side effect, and writes the result before the final
lifecycle transition. C74 is admitted only from the exact sealed I74 revision with no retained
process/checkpoint identity. Recovery can repair a missing or one-revision-stale snapshot from the
authoritative event chain. Once an intent exists, the coordinator calls only `observe`; it never
calls `start_once` again. An absent unit is retained as FAILED_INCOMPLETE, an exact unit is adopted,
and identity mismatch or multiple executors is quarantined. Local fake-backend and Unix-socket
tests exercise normal start, replay, stale/unsealed refusal and lost-response adoption for C74. The production-form
backend now constructs the complete transient-unit property set from the immutable manifest,
verifies the digest of a fixed role environment, calls systemd `StartTransientUnit` with mode
`fail`, and observes the exact MainPID plus one daemon through the unit cgroup. It rejects boot,
process-group, cgroup inode/path or cpuset drift and classifies a third process as a duplicate
executor. The ordinary live `start` dispatch uses this backend; the injected backend remains only
for deterministic socket tests. This closes the local implementation boundary, not the host proof:
no service or measured unit was installed or started, account ownership has not been rehearsed on
the admitted host, and the controller/supervisor-loss fault matrix and overhead budget remain
unexecuted.

The local terminal predicate requires both independent sources: a checkpoint chain ending in the
`Terminal` phase with the original harness/daemon identities, and the retained systemd unit in
`active/exited` with zero MainPID, `Result=success`, and the original unit/cgroup identity. The
transient policy uses `RemainAfterExit` so this proof is not replaced by PID absence or a
garbage-collected unit. The host-wide marker is releasable only after `COMPLETE_SEALED` has cleared
process, checkpoint and controller-lease state. Packet construction and the live seal transaction
still have to join these predicates before the route can return success.

State snapshots include state_revision, campaign state, role/phase, unit name, MainPID, harness and
daemon PIDs, /proc start ticks, process group, cgroup path and inode, machine id, boot id, binary
device/inode/hash, checkpoint head/sequence, useful-progress time, lease, exit status, journal head
and all controller events. State transitions append an event before atomically replacing state.json.

## Durable write and recovery algorithm

Each event or checkpoint is an envelope:

~~~json
{
  "schema_version": 1,
  "sequence": 1,
  "previous_record_sha256": "64 lowercase hex or genesis zeros",
  "payload": {},
  "payload_sha256": "64 lowercase hex",
  "record_sha256": "64 lowercase hex"
}
~~~

payload_sha256 is SHA-256 of RFC 8785 canonical payload bytes. record_sha256 is SHA-256 of the ASCII
domain string hydracache-long-run-record-v1 followed by a zero byte, unsigned sequence in
big-endian u64, previous hash bytes and payload hash bytes. The envelope is serialized as one
canonical JSON line.

The durable append sequence is:

1. validate the next sequence and previous hash in memory;
2. append the complete line with one write loop;
3. fdatasync the journal;
4. write the new head file to a same-directory O_EXCL temporary file;
5. fsync the temporary file, rename over the head and fsync its parent;
6. write the next state snapshot the same way to state.json;
7. only then send a success response.

state.previous.json retains the last valid snapshot for diagnosis but cannot override the journal.
On recovery, the journal is authoritative. One incomplete trailing line may be truncated after its
bytes are copied into diagnostics. A parse error elsewhere, sequence gap/duplicate, previous-hash
break, digest mismatch, timestamp reversal, identity drift or snapshot ahead of journal produces
CORRUPT_QUARANTINED. Recovery replays valid events to reconstruct state, compares the stored
snapshot, revalidates live systemd/proc identity and writes a recovery event.

Host reboot is detected by boot id and always invalidates a live role. PID identity is the tuple
(boot id, PID, /proc start ticks, cgroup path, cgroup inode, unit name). A pidfd is retained while
the supervisor process is alive but is not a durable identity across supervisor restart.

## Phase progress and hang classification

Every checkpoint contains campaign/role/phase, phase epoch, sequence, monotonic elapsed nanoseconds,
UTC timestamp, completed/failed/rejected/timeout operations, per-surface counters, bytes,
resource/owner counters, outstanding work, harness/daemon process identity and explicit milestone.
The writer emits on the contract cadence from warmup through final reconciliation, including
intentional post-work idle.

Useful progress is phase-specific:

| Phase | Progress predicate |
| --- | --- |
| startup/warmup | startup milestone or completed warmup operations strictly advances |
| measured | sequence and completed operations advance; surface totals reconcile; process CPU may not regress |
| drain | outstanding count decreases or reaches zero while sequence advances |
| durable companion | reviewed durability milestone advances |
| post-work idle | checkpoint and telemetry sample sequences advance while operation totals remain fixed |
| reconciliation | reconciliation milestone advances and owner deltas converge to exact terminal values |
| terminal | process exit, final checkpoint and required raw-file digests all agree |

The supervisor polls checkpoint heads without parsing unbounded logs. At the frozen warning gap it
emits a warning event. At the frozen rejection gap it captures one bounded diagnostic bundle,
requests a graceful stop, kills the entire role cgroup after diagnostic grace, and seals
FAILED_INCOMPLETE. Client heartbeat traffic cannot update useful_progress_at.

A controller_loss classification requires a live exact role plus valid fresh progress and absence of
an attached observer. A progress_loss classification requires live identities but an expired useful
progress deadline. Measurement_loss covers process exit/replacement, cgroup drift, reboot, missing
unit or invalid measurement chain. Host_loss is recorded by the next admitted controller from the
last durable state and can never pass.

## Attach admission

Attach performs these checks in order and writes a rejection receipt naming the first and all
independently discoverable failures:

1. peer and signed operation authorization;
2. campaign id, manifest digest, schema and request freshness;
3. journal/head/state integrity and non-quarantined state;
4. exact contract/scenario/tooling/source/tree/lock/binary/overlay hashes;
5. exact workload, seed, durations, estimator, thresholds, cadence and limits;
6. lease id/deadline and controller lease conflict;
7. machine id, boot id, host receipt, mount device/options, tuning and cpuset;
8. systemd unit state plus PID/start-ticks/process-group/cgroup identity;
9. command and allowlisted environment digest;
10. checkpoint chain, latest sequence, phase and useful-progress deadline;
11. no failure, abort, expiry, duplicate executor, replacement or prior corrupt event.

Attach never calls systemd StartTransientUnit and never executes a campaign binary. A successful
attach records the new controller provenance and returns current state plus the checkpoint head.
The monitor then uses status with expected state revisions. A dropped response is retried with the
same request id; a new request id is used only for a new observation.

After I74_SEALED, an attach may proceed to C74 only on the same machine and boot inside the same
product lease, after offline verification of the I74 continuation and a fresh frozen calibration.
No I74 process needs to remain alive after its valid seal. After C74_STARTING, only the original
live C74 unit can continue; otherwise the pair is incomplete.

## Sealing and artifact format

Seal first validates terminal state and checkpoint chain, then builds a deterministic packet in a
new seal directory. Entries are sorted bytewise; timestamps, uid/gid and modes are normalized;
symlinks, hardlinks, devices and paths containing parent traversal are forbidden. raw-manifest.json
lists every relative path, size and SHA-256. packet-manifest.json binds the campaign manifest,
journal head, checkpoint head/tail, process identities, role result, guard results and raw manifest.
The packet digest covers canonical packet-manifest.json. A tar.zst is produced only after all files
are closed and synced. outer-sha256.txt covers the exact archive bytes.

I74 sealing produces an immutable continuation packet. Final sealing includes that exact digest and
the C74 packet. Offline verification requires the separately downloaded I74 manifest, recomputes
its digest and requires the final I74 role manifest to be byte-semantically identical. An incomplete
seal has the same structural integrity but result=incomplete and an
enumerated terminal reason. Artifact byte/file limits are checked before copy, during copy and
before rename. Sealed directories are read-only and never modified; a repeated seal returns the
existing digest.

The local packet builder implements the content boundary before the live transaction. Raw
sources are explicit safe relative paths under the campaign directory; every component is checked
before a create-new copy. The builder derives journal metadata from the copied bytes, not from a
mutable source read, and atomically publishes canonical raw and packet manifests only after all
files and directories are synced. The independently implemented xtask verifier accepts generated
one-role continuation and two-role promotable fixtures, and separately built archives are
byte-identical. Archive publication also uses a synced create-new staging directory and atomic
rename, followed by exact directory/digest verification.

PacketPlan derivation is also explicit. A canonical
`roles/<role>/seal-input-inventory.json` names the role journal, exact bounded raw file set and
evidence path/result for every frozen guard. The resolver never scans the campaign directory,
requires every non-manifest path to stay under the active role root, and rejects unlisted or
cross-role data. C74 resolution combines the two explicit inventories, obtains the continuation
digest only from the durable I74 seal result, re-verifies the published read-only I74 packet and
compares every original I74 input with its sealed copy before final assembly. Thus mutable source
drift is rejected before a final artifact is created, not deferred to release review.

The corresponding producer is part of the measured-process library rather than the controller.
Only after the checkpoint journal verifies as terminal does it accept the frozen campaign manifest
and exact guard outcomes. Each outcome points to at least one pre-existing role-owned evidence file;
the producer writes a canonical result document under `roles/<role>/guards/`, includes both the
result and its source evidence in the raw set, checks the frozen file/byte limits and publishes the
inventory last. Guard-result and inventory names are deterministic. Publication uses a synced
create-new pending file followed by an atomic hard link into the final name and pending-link
cleanup. On replay, identical pre-link and linked-but-not-cleaned windows are completed, while any
different existing byte string fails closed. The final surviving file must have one link. The
positive integration passes the resulting inventory through the supervisor resolver; negative
coverage rejects a complete result with a failed guard and cross-role evidence without publishing
an inventory.

The durable artifact subtransaction now persists a canonical request/role/plan/limit-bound intent
before building. On retry it verifies and adopts exact final packet/archive directories, recovers a
fully verified `.building` directory through the missing rename, and repairs only the narrow synced
JSON-before-sidecar window. A canonical result binds all nested manifest hashes, the raw set and
file/byte counts, and the archive hash/size; exact replay re-verifies the published artifacts, while
conflicting intent or any drift fails closed.

The Linux seal lifecycle coordinator now implements that ordering around an internally supplied
PacketPlan and already observed terminal UnitSnapshot. It commits each lifecycle event before its
state CAS so reconciliation can finish a lost state write, binds terminal/sealed recovery to the
same request digest, writes the signed response only after the artifact and sealed state exist, and
re-verifies artifacts on response replay. `COMPLETE_SEALED` clears the retained process, checkpoint
and controller lease before the durable response; only then is host-marker removal attempted, and
that removal is idempotent across a lost response. Local I74/C74 tests exercise continuation and
promotable packets plus event/state and marker-release crash windows. The strict inventory resolver
now derives PacketPlan from frozen on-disk evidence. The signed socket `seal` dispatch composes
persistent manifest and host checks, production D-Bus observation, the resolver and coordinator;
it reports the reconciled revision if artifact work fails after the terminal transition and permits
same-request recovery without spawning. The configured seal root is a distinct root-owned
directory. Production system-bus and uid/gid behavior still require admitted-host rehearsal before
this becomes a live-service claim.

Abort uses a separate two-transition transaction. `ABORT_REQUESTED` changes the campaign to
`ABORTED_INCOMPLETE` but deliberately retains the exact harness, daemon, checkpoint and controller
lease so an interrupted supervisor can finish the same request. An idempotent backend boundary
must capture bounded allowlisted diagnostics and stop that retained unit without spawning.
`ABORT_COMPLETED` then clears those retained fields; only after its event and snapshot are durable
does the supervisor append the accepted request response and release the host-wide claim. The
socket dispatcher revalidates manifest/host evidence first and reports the reconciled revision
when the backend fails after the intent commit. Same-request recovery and response replay are
locally exercised. The production diagnostics/systemd backend is still absent, so the ordinary
service route cannot yet execute abort and fails without mutation.

Publication modes are now part of the verified artifact contract on Unix. Packet and archive
staging trees are recursively changed to `0400` files and `0500` directories, metadata is synced,
and only then is the tree renamed. The builders immediately re-open the final location through the
same replay verifier used after a crash. Missing read-only modes, unexpected entries, digest drift,
hardlinks or symlinks all reject the artifact. This closes local immutability mechanics; the
provisioned account/group ownership and mount behavior remain host-rehearsal evidence.

The workflow uploads:

~~~text
long-run-074-start-<campaign_id>
long-run-074-i74-<campaign_id>-<packet_digest>
long-run-074-final-<campaign_id>-<packet_digest>
long-run-074-incomplete-<campaign_id>-<packet_digest>
~~~

Upload success is transport provenance only. The release gate downloads the archive into a fresh
directory and runs cargo xtask long-run-campaign-verify without access to the live supervisor.

## GitHub workflow graph

The protected entry workflow uses workflow_dispatch inputs operation, campaign_id,
manifest_sha256 and expected_state_revision. Start preparation creates and uploads the small start
manifest artifact before requesting start, so its identity survives cancellation immediately after
spawn. The graph is:

~~~text
authorize -> admit-host -> build-and-stage -> start-or-attach
          -> monitor-current-role -> seal-role -> upload
          -> fresh-calibration -> start-or-attach-next-role
          -> monitor -> final-seal -> upload -> offline-verify
~~~

Each mutating job uses the protected environment, explicit permissions, timeout-minutes slightly
above its control-plane task only, and concurrency group long-run-074-<host-id> with
cancel-in-progress false. Long measurement duration is enforced by the host lease/unit, not by a
single irreplaceable GitHub job. Monitor cancellation is expected to be recoverable.

The workflow has no automatic start retry. A replacement workflow must be dispatched with
operation=attach and the existing campaign id. It downloads the original start manifest, verifies
its digest, asks status, attaches, and resumes observation. If the role is already terminal it asks
seal; it never starts it again. The always path requests a best-effort status snapshot but the
supervisor remains authoritative when Actions cancellation prevents final steps.

GitHub run id/attempt are controller provenance only. They are not campaign identity and cannot
change frozen inputs. Workflow permissions do not include host sudo or arbitrary service control.

## Lease, cleanup and operator recovery

The product/server lease is authoritative and longer than the planned campaign by a frozen safety
margin. The supervisor checks it at start, periodically, before attach, before a next role and before
seal. On expiry it captures diagnostics, terminates the role cgroup and writes
LEASE_EXPIRED_INCOMPLETE. No controller can renew the product lease through this API.

The operator runbook is:

1. obtain campaign id and manifest digest from the pre-spawn start artifact;
2. dispatch operation=status to inspect without mutation;
3. if state is live and attach predicates pass, dispatch operation=attach;
4. if terminal-success, dispatch operation=seal;
5. if stuck or intentionally abandoned, obtain protected abort approval and dispatch abort;
6. download the sealed packet and verify offline.

Host cleanup is a separate provisioned command unavailable to workflows. It deletes only campaigns
that are sealed, past the contract retention period, outside every active lease and recorded in the
external evidence/archive index. It refuses an active flock, live unit, unsealed directory,
unverified upload or unknown path. Deletion targets are resolved beneath the campaign root and
logged with the final packet digest. Disk-pressure admission refuses a new campaign before deleting
or truncating evidence.

## Fault, security and conformance tests

Local/CI tests use fake proc, cgroup and systemd adapters plus filesystem fault injection. A
privileged same-host rehearsal is still mandatory because mocks cannot prove process-tree survival.

| Requirement | Automated coverage | Required evidence |
| --- | --- | --- |
| protocol framing/schema | oversized, truncated, unknown/duplicate field, invalid UTF-8 and replay fixtures | client/server conformance receipt |
| authorization | wrong uid/group/key/repository/actor, expired signature, replay and unauthorized abort | expected-red authorization matrix |
| no arbitrary execution | argv/env/property/path fuzzing, shell metacharacters and DBus denial | negative security receipt |
| exactly-once spawn | concurrent starts, lost response, crash before/after spawn_intent and systemd call | one unit/PID plus event chain |
| controller survival | kill monitor/job shell, stop Actions runner and cut network in every phase | same PID/start ticks/cgroup and uninterrupted chain |
| supervisor recovery | restart supervisor in every write/spawn window | adopted exact unit or retained incomplete, never respawn |
| hang detection | freeze harness, daemon, writer, sampler and monitor separately | bounded diagnosis and no restart |
| identity guards | PID reuse, exec substitution, cgroup move, reboot, cpuset/governor/mount drift | attach rejection receipts |
| journal durability | torn tail, middle corruption, reorder, duplicate, gap, hash/timestamp reversal | quarantine except one torn tail |
| phase semantics | idle with live telemetry, frozen idle collector, drain/reconciliation stalls | correct useful-progress decisions |
| sealing | symlink/hardlink/path traversal, limit overflow, interrupted seal, repeated seal | deterministic digest or incomplete |
| I74 handoff | cancellation before/after seal and calibration failure | exact continuation reuse or rejection |
| offline verification | mutate every manifest, raw file, nested digest and guard | xtask expected-red matrix |
| resource neutrality | supervisor/checkpoint CPU, memory, I/O, scheduler and timing A/B | overhead below frozen asymmetric-safe budget |

Property tests generate valid state-machine traces and prove that no command reaches a disallowed
state. Loom tests cover Rust in-process lock/replay races. Filesystem fixtures run each durable-write
step with an injected crash. The same-host rehearsal uses real systemd and:

- kills the GitHub-side monitor during startup, measured work, drain, idle, reconciliation and after
  process exit;
- stops/restarts the Actions runner service and removes network access;
- restarts the supervisor while the transient role remains alive;
- proves exactly one harness/product tree and exact checkpoint continuity;
- repeats a bounded uninterrupted control and compares final accounting/digests where deterministic.

All dangerous positive canaries are expected red: PID-only acceptance, replacement spawn under the
same campaign, broken-chain acceptance, cross-boot attach, mixed-attempt identity, ignored lease
expiry, duplicate executor, writable sealed packet and arbitrary command execution.

## Rollout, gates and rollback

Rollout proceeds in this order:

1. implement schemas, protocol, state machine and offline verifier with deterministic fixtures;
2. run unprivileged unit/property/crash/security tests in normal CI;
3. provision a disposable host and run a short systemd smoke with no product candidate;
4. run a bounded controller-loss rehearsal against non-promotable I74/C74 fixtures;
5. freeze the W0 contract and host receipt;
6. run the six-hour qualification and independently verify it;
7. authorize the 24-hour confirmation separately only after the six-hour packet is green.

The feature is release-blocking until every conformance row is green and the overhead budget passes
for both roles. A controller-resilient result is promotable only when the original measured process
survived, all attach predicates stayed exact, the final archive passed offline verification and all
ordinary 0.74 performance/resource/correctness gates passed.

Rollback disables admission of new resilient campaigns, waits for or explicitly aborts any live
campaign under its lease, archives incomplete evidence, stops the supervisor and restores the prior
host provisioning receipt. It never silently falls back to an Actions-owned long process for release
evidence. Removing the binary/service is allowed only after no live unit or unarchived campaign
remains.

## Implementation completion checklist

- Rust package, schemas and fixtures compile with the repository MSRV and locked dependencies.
- Installed and client binary digests are identical and bound by host admission.
- All state transitions, exit codes and recovery windows have deterministic tests.
- Real systemd rehearsal proves controller and supervisor restart survival without duplicate spawn.
- Attach validates every frozen identity and is incapable of spawning.
- Checkpoint/event journals survive allowed torn-tail cases and quarantine all other corruption.
- Lease expiry, hang and artifact overflow produce bounded immutable incomplete packets.
- Xtask verifies packets from scratch without socket, state directory or supervisor trust.
- The release plan, contract, workflow, runbook and evidence registry reference the same schema
  versions and filenames.
- No 0.74 candidate measurement begins until this design, W0 contract and expected-red suite are
  reviewed and frozen.
