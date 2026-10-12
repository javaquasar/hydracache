# Production readiness and dedicated-server launch boundaries

Current continuation: original authority composes with signed expected intent.
The canonical Linux production campaign path now inspects fixed diagnostic
storage under its original common flock. Any entry blocks acquire, recovery
and short legacy diagnostic lock transactions, including after a prior short
guard drops. Original objects span pre/post marker checks; late refusal retains
the marker. Missing/unsafe storage is not absence. The original Reserved reader
and diagnostic fence still require trusted private campaign ownership; the
deployed worker-owned layout is not silently enrolled or migrated.
Original authority/intent/state observation is now composed locally under the
original common fence, with distinct fixed and fixture APIs and cross-input
sticky refusal. This is read-only consistency, not trusted publication or
authenticated peer/clock admission. A trusted state publisher, fixed intent
publisher and diagnostic start route remain open. See the
[composition design](diagnostic-fenced-intent-local-design.md).
Initial reservation also has a separate
[original empty fence](diagnostic-empty-reservation-fence-local-design.md):
the same lock can inspect empty fixed storage without requiring an already
published Reserved file. This removes a circular prerequisite, not the publisher
gap. It still requires trusted private campaigns ownership and is not host
enrollment, a write capability or a durable reservation.
The Linux storage preflight is ready for a controlled storage check after a fresh
reviewed installation. See the [intent composition design](diagnostic-authority-intent-local-design.md)
and [storage design](diagnostic-storage-preflight-design.md).

The [fixed admission design](diagnostic-fixed-admission-local-design.md) closes
the read-side gap demonstrated by the [original fence](diagnostic-original-fence-local-design.md).
Empty storage permits legacy admission, not diagnostic enrollment. Before
publication, coordinated deployment must exclude older supervisor binaries,
and quiescent trusted ownership enrollment must cover every campaign user.
No installed-host change or complete publisher serialization is proved here.
Holding an external lock across a workload cannot replace those requirements.

This is a repository audit on 2026-10-11, not a server inspection, installation
approval or release acceptance. The original signed leader mount composition is
implemented at `1a7d946ed542908ca04c1aa3d4c5572973d90c4d`, with reviewed early
inner-refusal hardening at `7cd8b43061638a66b10a3df431a642cd95f19cb3`. Its
[design](diagnostic-signed-mount-local-design.md) and
[local evidence](local-runs/diagnostic-signed-mount-20261011/manifest.json) cover
read-only diagnostic consistency. They do not enroll a production worker.

## Two different launches

The dedicated-host diagnostic pilot is not a HydraCache product deployment.
The former must authenticate a fixed observer and retain trustworthy measurements;
the latter also needs an accepted exact product candidate and release receipts.
The published 0.73 product remains the rollback baseline. The workspace package
version remains 0.73 and the 0.74 admission contract has zero accepted product
candidates. Installing current tooling cannot turn it into a 0.74 product release.

## Repository findings and gates

| Requirement | Current evidence | Remaining before production |
| --- | --- | --- |
| Original signed context, account files, leader credentials and mount | Fixed/fixture types; original-process-only join; sticky first failure; local tests | Controlled positive fixed-file enrollment on the exact host; no fixture promotion |
| Every worker task | Bounded original-task reader plus exclusive original signed mount composition; exact signed numeric/user/mount pins, membership-churn refusal and whole-chain latch | Controlled positive fixed-host checks; sequential observations are not continuous confinement |
| Worker policy authority | Signature verification against external key/digest/epoch; bounded typed journals; original signed-task runtime bridge and fixture-only interrupted-intent refusal persist only to safe exact-bound storage | Controlled positive root-path enrollment; authenticated production intent and reboot lifecycle; real operator-selected issuer/pins; no whole-host snapshot rollback guarantee |
| Production output creation | `prepare_fixture_outputs` rejects production roots | Root-owned fenced preparer, exact reserved cell, exclusive creation, retained ancestry, ownership, crash/incomplete-cell refusal; no automatic repair |
| Original diagnostic start | Read-only identity/execution guards; private model seams | Signed diagnostic intent tied to original material, environment, streams, worker, unit/cgroup and observed process; no pathname or process replacement |
| Failure and cleanup | Lease/IPC/spool models and separate historical campaign rehearsals | Production diagnostic backend, autonomous watchdog, writer revocation, retained uncertain intent, exact owned empty-cgroup proof before release |
| Product candidate and qualification | W10 has zero accepted candidates; ship refuses | Exact C74 D4, independent native nonregression, same-box Redis and controller overhead evidence; separately authorized 6h/24h |
| Release supply chain | Local contract/registry checks; dependency-policy check | Exact final-source packages, SBOM, advisory/license, supported targets, immutable archive and nested hash verification |

The existing `server.rs::admit_start` authenticates and reserves a **campaign**.
It does not call the new signed mount binder or production diagnostic preparer.
The diagnostic unit property builder constructs data, not an enrolled production
dispatcher. Campaign lifecycle evidence must not be relabeled as positive evidence
for the new diagnostic route. Local owned cat processes test real proc/nsfs objects,
but are neither production service ancestry nor authenticated original starts.

Worker policy verification currently compares caller-provided trust pins. A guard
latches refusal in memory; creating a new guard after restoring old external pins
is not durable revocation. The fixed-local-files account model has been approved;
the real issuer remains an operator choice. The continuation selects the local
registry rollback boundary: operator-managed issuer key/digest/epoch plus a
root-owned crash-durable registry as the target, excluding whole-host snapshot
rollback and root/owner tampering. The
[local journal slice](diagnostic-authority-registry-local-design.md) originally
implemented only its canonical core and nonroot temporary-fixture storage; it
selected neither the real issuer nor the root backend. If host rollback must be
detected, select a separate external monotonic authority before enrollment.
No test key may be enrolled.

The subsequent [fixed backend slice](diagnostic-authority-fixed-local-design.md)
now implements fixed root-owned storage without changing the record format or
installer. Its API has no path/owner selector and retains every original ancestor;
local modeled-tree checks are not actual root-path enrollment. The original
journal slice's fixture-only statement above describes that earlier boundary.
The [runtime bridge](diagnostic-signed-authority-local-design.md) subsequently
implements exclusive original signed-guard composition and safe refusal
publication, with typed uncertainty and no retry. It does not cover crash before
publication or turn an unsafe-storage in-memory latch into durable refusal.
The [fixture recovery slice](diagnostic-fixture-recovery-local-design.md) now
detects existing pending state/IPC intent and interrupted stages under the
original cooperative fixture host fence. Its fixture-only bridge persists safe
original-authority refusal without replay, cleanup or adoption. Quiet observations
are not launch permission. Authenticated production intent, crash-before-any-intent
and reboot lifecycle remain required, followed by controlled enrollment under
real external pins. No fixed-host diagnostic start is introduced by this slice.
No effective-root check authenticates the initial host namespace.

The subsequent [signed intent projection](diagnostic-intent-projection-local-design.md)
implements only a bounded canonical signed byte contract. It binds the exact
asserted Reserved state, original worker policy digest/epoch/boot and expected
fixed unit specification under separate external controller pins. Checked
intent drift/expiry is sticky; original worker failures survive wrapper drop.
The [packet](local-runs/diagnostic-intent-projection-20261011/README.md) uses
synthetic issuer/controller keys and does not create a production route.
Its state, peer and clocks are still assertions; signatures do not attest kernel
objects, stream/executable identities, actual environment, replay history or
durable fenced intent. All production rows above remain open. Next, bind this
contract to durable reservation/one-start recovery and the original material,
output and runtime guards, without enrolling a test key or reusing an old receipt.

## Extended local checks completed

The separate [readiness packet](local-runs/production-readiness-local-20261011/README.md)
retains the terminal native Windows verifier exit 0. Its 532 successful harness
rows total 3,982 passes and 82 existing ignored executions, not unique tests.
Workspace/xtask tests, doctests, strict clippy/allocator lanes, dependency policy,
workspace rustdoc and budget contract pass. The built-in console gate explicitly
skips missing native `npm`; separate `npm.cmd` runs pass 16 unit, six package and
46 browser tests twice. After a test-only nested-copy review, 16 unit and seven
package tests pass. External DB, scheduled/chaos and Linux-only cases skipped by
the Windows workspace run are not covered by its success.

Linux supervisor all-targets check additionally passes on Rust 1.88 MSRV;
Linux rustdoc passes on 1.94 with warnings denied. The original scoped packet
retains 370 Linux passes, one existing ignored and 43 portable Windows passes.
The full verifier spanned working-tree changes during development. It is not
an immutable exact-candidate receipt or `fast.workspace-nextest` success;
840-second cadence and 2400-second hard timeout are unchanged.

Full npm audit initially finds two high-severity dev dependencies. Minimal
compatible lock updates fix `brace-expansion` 5.0.9 to 5.0.12 and `source-map-js`
1.2.1 to 1.2.2, with test-first regressions and nested-copy/missing-package canary.
Repeated full audit reports zero vulnerabilities at capture time, not a future
guarantee. References: [brace parsing advisories](https://github.com/advisories/GHSA-q2hr-2g5m-vwhr)
and [indexed source-map offsets](https://github.com/advisories/GHSA-68fv-2mgg-jv7q).
The retained 182-component console SBOM is local package-0.73 metadata, not the
missing final 0.74 source-bound receipt. HTML input LF normalization removes
incidental generated/embedded content changes; no asset blob is changed.

These completed local checks do not finish any missing production row above.
Durable production authority still needs controlled fixed-backend enrollment,
original-guard composition and real operator issuer/pins; no test issuer or synthetic epoch may
be promoted to replace them. The selected local rollback scope is not enrollment.

The subsequent [original-task slice](diagnostic-task-set-local-design.md) now
retains up to 128 original task objects and refuses inventory or per-task drift.
Owned unprivileged helpers exercise stable multi-thread operation, real additions,
removals and exit; private mutations cover credential and namespace checks.
This primitive accepts explicit numeric assertions, not an original signed reader.
It therefore does not close signed worker enrollment or the dedicated-host gate.

The next [signed-task composition](diagnostic-signed-tasks-local-design.md) derives
that primitive privately from the original signed mount reader. It accepts no
replacement process, task set or policy. Its fixed/fixture APIs retain distinct
origins; observed membership or mapping drift refuses every original borrowed
guard, including after wrapper drop. This closes the local composition gap, not
the remaining controlled-host enrollment, durable authority or production start.

## Required order on the dedicated host

The subsequent authority-journal audit also identifies an inherited qualification
input mismatch. The plan last changed at `8e16117b`, while the manifest retains
its earlier SHA-256. The current plan's committed LF digest is
`cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828`;
the declared digest is
`6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d`.
Windows checkout CRLF additionally changes its raw file digest. The genuine
content mismatch exists in the baseline commit, not just checkout normalization.
The qualification dry-run refuses. Local structural checks do not replace that
input check. No plan or frozen manifest is altered by this slice; an explicit
reviewed reconciliation is required before qualification, not a silent digest
refresh to pass a guard.

This is a launch checklist, not an executable runbook: production diagnostic
preparation/start entry points do not yet exist. Do not substitute fixture CLI
commands or change admission booleans to get a launch.

1. Finish and review the missing local components above. Freeze the exact tooling
   source, lockfiles, features, toolchain, Linux binary/config digests and signed
   build/provisioning statements. Obtain operator authority pins independently
   of the policy envelope. Rebuild after source changes; never reuse an old receipt.
2. With separate host approval, perform read-only preflight: installed source
   receipt and signature/key binding, machine/boot/context, supported systemd and
   cgroup-v2 properties, fixed paths/backing filesystem, available disk/inodes,
   bounded spool storage, CPU placement/governor, expected unit/binary ownership,
   and no active campaign, diagnostic marker, pending intent or lifecycle fixture.
   A busy or uncertain host remains reserved; do not remove markers to proceed.
3. Verify the actual allocated nonroot worker UID/GID and complete bounded group
   set against the separately approved signed policy and fixed account files.
   The sysusers `-` allocation request and root supervisor identity do not specify
   this mapping. Bind admitted user/mount context and reboot revocation rules;
   equality with the observer or PID 1 is not initial-host attestation.
4. Install only the reviewed signed diagnostic bundle through the approved path,
   verify the installed receipt and every artifact, and use the supervisor-owned
   lease/fence. Any maintenance stop must target only the exact stable
   `hydracache-performance-supervisor-074.service` after absence checks, with
   explicit restoration and verification. Do not hold the legacy host lock as a
   substitute reservation or interrupt an unrelated campaign.
5. Run a separately reviewed **non-product** positive lifecycle rehearsal, then
   refusal/fault cases: bad signatures/pins, revoked epoch, reboot, task churn,
   changed credentials/namespaces, stale process/unit generation, output/executable
   replacement, partial/overflow receipts, slow or missing manager, controller/
   supervisor loss, uncertain journal and incomplete cleanup. Retain every attempt;
   prove writer cessation and exact empty owned cgroup before sealing/releasing.
6. Verify the stable supervisor is restored and health/receipt/marker checks pass.
   Only after the diagnostic trust chain and overhead guards pass may an operator
   separately admit a preregistered finite numerical pilot. Freeze offered calendars,
   seed, payload, keys, concurrency, duration, estimator and thresholds beforehand;
   measure RESP, native, ClientSurfaceState and embedded independently.
7. Product release is a later boundary: accept the complete candidate contract,
   freeze B73/I74/C74 identities and Redis versions/placement, obtain explicit
   qualification authorization, run the required 6h and 24h stages without hiding
   failures, validate final supply-chain/target/archive receipts, and require green
   exact-candidate ship admission. Deployment then needs its own application
   configuration, tenant/auth/TLS policy, persistence/freshness settings, resource
   limits, health/metrics, backup/restore and rollback smoke tests.

No server, workload or qualification operation is performed by this audit.
The [admission contract](release-admission-contract.toml) and frozen qualification
inputs remain unchanged. Passing local tests does not complete the host or release
rows above; no throughput, allocation, CPU or memory improvement is claimed.

## Subsequent local intent journal result

The [fixture journal](diagnostic-intent-journal-local-design.md) completes local
durable consumption of exact signed intent before any future execution. It
retains four fixed cells under the existing fence, reauthenticates history and
refuses duplicate cells, partial pairs, stale boot/controller bindings and
original file replacement. Lost acknowledgment after a complete commit does not
permit retry. Nineteen new unit tests pass three repeats; the
[packet](local-runs/diagnostic-intent-journal-20261011/README.md) separates this
result from modeled fault checkpoints and the ineffective early regression.

This does not change the launch checklist into a runbook. A fixed root-owned
production intent backend bound to the original authority is still absent;
fixture path selection is not global replay protection. Live kernel peer/clock,
original executable/output/runtime composition, Starting/watchdog/owned cleanup
and controlled real issuer/host enrollment remain required. Existing frozen
qualification inputs and their plan-digest blocker are unchanged. No admission,
worker start or dedicated-host operation is added by this result.

## Fixed intent ancestry prerequisite

The [fixed storage inspector](diagnostic-intent-fixed-storage-local-design.md)
now retains and rechecks every original directory up to a prospective root-owned
0700 intent leaf. No caller-selected path, owner or descriptor is exposed; errors
are sticky. It does not create storage, publish records, acquire the host fence,
join authority or start a worker. Modeled nonroot positive tests are not fixed-host
enrollment. The [packet](local-runs/diagnostic-intent-fixed-storage-20261011/README.md)
retains the local proof and unchanged admission boundary.

Current provisioning makes campaigns worker-owned and the common parent
root:hydracache-perf. Campaigns therefore cannot hold trusted root authority; the
older fixed authority backend's root:root ancestor rule also differs from the
bundle. Resolve that ownership policy in a preregistered composition contract
before deployment, rather than silently changing permissions or a reader predicate.
The real fixed publisher, lease retention, original authority/fence/state join
and production lifecycle remain open. Frozen qualification inputs are unchanged.

## Read only server storage check

The preregistered ownership amendment now resolves the parent-GID conflict above.
Both readers accept only the root-owned nonwritable common parent and retain its
exact original GID. Other ancestors and both 0700 private leaves stay root:root.
Campaign ownership is unchanged. Before installation mutation, the signed
installer refuses unsafe existing ancestry/leaves rather than allowing tmpfiles
to repair metadata. Safe existing leaf contents are not journal validation.

The new code command, executed through a separately approved root operator path,
is:

```sh
/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 diagnostic-storage-preflight
```

It takes no arguments and returns one JSON line on success. Require exit 0,
schema `diagnostic-storage-preflight-074-v1`, scope `storage-ancestry-only` and
`storage_ancestry_verified=true`. Every other proof/permission field must be false.
Failure exits nonzero; the CLI creates no directory, journal, lock or worker.
No new sudoers permission or provisioning-entrypoint mode is introduced.

Before this check, obtain approval for a fresh signed bundle installation and
verify its source receipt. Confirm the host has no active campaign or lifecycle
fixture, follow the exact stable-supervisor maintenance boundary above if needed,
and restore/verify that service afterward. Retain the installed source identity,
stdout/stderr and exit code. Do not run an old binary and attribute its result to
these changes. Positive fixed-root execution remains a host test, not a local
nonroot model result.

The [local packet](local-runs/diagnostic-storage-preflight-20261011/README.md)
covers both modeled leaves, substitutions, missing paths, metadata drift, sticky
refusal, report scope, CLI refusal, provisioning invariants and toolchain checks.
The next code package remains the fixed publisher with original authority,
fence and state composition, then production output/start/watchdog/owned cleanup.
Real worker/controller issuer pins and initial-host admission remain operator
prerequisites. The inherited plan-digest mismatch and closed ship admission are
unchanged. Passing the storage check authorizes none of those later operations.

## Original authority and expected intent composition

The [new guard](diagnostic-authority-intent-local-design.md) exclusively borrows
the original authority chain and checks authority, signed expected intent and
authority again. It does not reconstruct worker policy to bypass borrowing.
The existing signed projection formats and old standalone API remain unchanged.
Invalid packets before binding cannot revoke a healthy worker; after binding,
envelope/state/controller/peer drift or expiry refuses the original chain.

Only safe storage retaining the exact original authority pins receives a Policy
refusal record. Partial or late-unacknowledged publication preserves the intent
failure and typed uncertainty; restoring bytes or permissions cannot refresh the
same guard. The [packet](local-runs/diagnostic-authority-intent-20261011/README.md)
tests those boundaries locally, not under enrolled root storage or real issuer
keys. A private fixed-dispatch model is not a public fixture conversion.

The next code package must join the original supervisor host fence and trusted
persisted Reserved state with that guard, then implement publication through the
original fixed intent storage. Caller-supplied state, peer and time remain
assertions until their trusted owners are connected. Production output creation,
original executable/streams/runtime binding, dispatch, watchdog and exact owned
cleanup still precede a diagnostic pilot. This slice changes no installer,
server command, frozen qualification input or ship admission.

## Existing root owned Reserved state reader

The [fixed reader](diagnostic-reserved-state-local-design.md) requires an existing
`active-diagnostic.json` under the original root-owned intent leaf. It validates
the existing canonical schema-1 state envelope, full expected Reserved snapshot,
private root ownership, single link, same device and bounded bytes. It retains
the original file rather than adopting a newly opened replacement, and bound
failures latch the original storage. Same-leaf pending state/IPC, active campaign
or unhandled IPC ledger refuses.

This does not create that document. Never copy the worker-owned campaign state
into the root leaf as an enrollment workaround. Its trusted reservation publisher
and shared supervisor-owned host fence must be implemented together with the
legacy transaction boundary. A new independent lock would not prevent a legacy
campaign from running concurrently. The [local packet](local-runs/diagnostic-reserved-state-20261011/README.md)
uses private nonroot modeled storage and is not a positive fixed-host check.

After the original authority/intent/state/fence join and fixed consumption
publisher, implement output/start/watchdog/owned cleanup and enroll real issuer
and initial-host pins. Current installation can still support only the separately
approved storage-ancestry preflight, not a diagnostic worker pilot. All eight
qualification inputs and the inherited plan-digest refusal remain unchanged.

## Signed intent and empty fence before initial publication

The [original empty-fence join](diagnostic-empty-fenced-intent-local-design.md)
now binds the existing authority/signed-intent guard to empty fixed storage
under its retained original common lock. Full asserted state must match intent
pins before I/O. Fence and intent observations are interleaved; bound failure
latches both chains, and Policy refusal reaches only independently exact-safe
original authority. This is read-only composition, not a trusted reservation
or authenticated peer/time source.

The next implementation needs an explicit no-overwrite publication transition
that keeps this same original fence and retains the published object's identity.
Do not write a state and call the empty guard again, or drop/reacquire the lock
to refresh eligibility. Partial/crash uncertainty must fail closed. Existing
worker-owned host layout still needs coordinated reviewed trusted ownership
and binary rollout; no automatic repair or enrollment is introduced.
The [local packet](local-runs/diagnostic-empty-fenced-intent-20261012/README.md)
does not admit a server pilot, product workload, qualification or shipping.

## Initial state publication retains the original common fence

The [fixed-only library publisher](diagnostic-reservation-publication-local-design.md)
now creates the first Reserved state without overwrite or a lock gap. Runtime
acknowledgement follows file/directory sync and original inode, canonical bytes,
inventory and signed-intent checks. Errors retain created context and the
original lock until drop, cross-latch the original chains and preserve uncertain
authority refusal. Existing state cannot be adopted as a successful new attempt.

This closes the local initial-state publication dependency only. The service is
not wired to the publisher; peer and time remain assertions, and deployed
worker-owned storage is not trusted enrollment. The [local evidence](local-runs/diagnostic-reservation-publication-20261012/README.md)
models interruption boundaries, not machine power loss or a positive installed
host. Next specify exact intent-consumption publication and restart ambiguity,
then authenticated peer/clock and coordinated reviewed ownership/binary rollout.
Original output/start/watchdog/owned cleanup remains before any diagnostic pilot.
No product workload, qualification, server operation or ship admission follows.

## Initial cell-one consumption retains runtime ownership

The [initial consumption library](diagnostic-initial-consumption-local-design.md)
now consumes the runtime-acknowledged fixed publisher under its uninterrupted
original common fence. It creates only cell-one retained JSON and digest-commit
objects, seals new owned files 0400 and acknowledges after file/directory sync
and original state, object, namespace, authority and intent checks. Errors own the
same lock until drop, retain partial bytes and fail closed. No existing history
or recovered Reserved record can become a runtime acknowledgement.

The [local packet](local-runs/diagnostic-initial-consumption-20261012/README.md)
covers logical interruption boundaries, not actual process kill, power loss or
positive host enrollment. This closes initial-cell retention only: cells two
through four, state advancement and actual owned-process interruption tests are
not complete. Peer and time remain assertions; the service has no consumption
entry or worker-start conversion. Trusted coordinated ownership/binary rollout
and original output/start/watchdog/owned cleanup remain before a diagnostic pilot.
Frozen qualification inputs, the inherited digest refusal and ship gates stay
unchanged. No server operation or product workload is performed.
