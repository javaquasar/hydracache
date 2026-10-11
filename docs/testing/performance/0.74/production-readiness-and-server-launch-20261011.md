# Production readiness and dedicated-server launch boundaries

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
| Worker policy authority | Signature verification against externally supplied key/digest/epoch | Real operator-selected issuer and pins; durable monotonic epoch/refusal lifecycle; reboot/rollback threat model |
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
the real issuer, durable authority storage and host rollback policy remain separate
choices. Recommended next decision: operator-managed issuer key/digest/epoch plus
a root-owned crash-durable registry, explicitly excluding whole-host/root rollback
from that guarantee. If host rollback must be detected, select a separate external
monotonic authority before implementing enrollment. No test key may be enrolled.

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
Durable authority still needs the operator choice described there; no test issuer
or synthetic epoch may be promoted to replace it.

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
