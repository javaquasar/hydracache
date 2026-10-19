# Attempt cgroup root enrollment evidence

Baseline source is `7fb289c5e2b80f4f0ea2f68da12f61a8afb44125`; its closed
[audit regressions](../original-arming-boundary-audit-20261019/README.md) retain
the unchanged GNU/MSRV images, 22 ordinary controls per compiler and protected
source/qualification checks. The [approved local plan](../../cgroup-root-enrollment-plan-20261019.md)
separates new protocol/root mechanics from future original enrollment and
autonomous guardian integration. No server or product operation is part of
this packet.

## Selected source and scope

Implementation source is `593e51481d78cfd3a917b81d22f46b153ae1c57a`.
The ordinary build exposes only expected packet claims. Private kernel handles
have no public constructor or original-owner join. The
[contract](../../cgroup-root-enrollment-contract.toml) keeps authenticated
transport, autonomous guardians, Armed, held-unit delivery, completion, fence
release, production enrollment and server/product/qualification admission false.

The protocol is a new 104-byte version, not an extension of guardian-v1. The
one-shot claim exchange is per instance and in memory; it is not durable replay
exclusion. Deadline bytes are claims, not clock observations. Kernel root pinning
requires actual cgroup2, root ownership, original named/retained descriptor
identities and an empty unfrozen domain. Here "original descriptors" means
objects opened by this private root handle, not the admitted original frame.
That frame and its 300/60/10-second origins are unchanged.

## Current and MSRV cohorts

`gnu194-selected` and `gnu188-selected` each retain all-target check, strict
all-feature Clippy, seven new ordinary controls, the broad two-phase regression
(29 passed, four ignored), thirty old exchange controls (one ignored), the
unchanged seventeen-case guardian driver, and three eleven-case root matrices.
This is 66 selected root-scenario executions and 24 actual SIGKILL cuts of the
private kill caller across both compilers. It is not a product timing sample.

The GNU image hashes are:

- 1.94.0: `e1998ce44611d72f1564bedc6bcb5ab91b3ba93db940f9488325ac53ecb3e2f7`.
- 1.88.0: `3709240c8b8cf3958c2b47fc08d891c90d5134191f7165062825716c6ced0827`.

Each receipt binds committed source, unchanged before/after image, recipe hash,
nonroot build UID 1000 and local fixture UID 0. Workers are owned `/bin/cat`
helpers with UID/GID 65534, not HydraCache binaries. Actual process cgroup
membership and SIGKILL status are checked. Tests kill only exact owned pidfds or
the fresh fixture's pinned cgroup. Exact child cleanup precedes nonrecursive
cgroup removal. The independent fixture fence remains busy throughout cleanup;
it is not the production original fence. No test cgroups remain at the recorded
post-checks. Local WSL timesyncd is observed active before and after each cohort;
no clock or service is changed and no dedicated server is accessed.

## Eleven kernel scenarios

| Scenario | Required observation |
| --- | --- |
| Empty root and late population | Initial kill can leave it empty; later population is observed and needs another revocation |
| Recursive nonroot writers | Root and nested targets die by actual SIGKILL; a nonempty root cannot enroll as empty |
| External writer alias | Empty cgroup does not give EOF until the outside pipe alias closes |
| Replaced root | Retained handle refuses; replacement target is not signaled |
| Removed root | Observation and kill submission refuse |
| Permission drift | Restoring permissions cannot clear refusal |
| Threaded root | Domain enrollment refuses |
| Kill caller SIGKILL before write | Target remains live until owned fixture cleanup; no autonomous guardian is implied |
| Kill caller SIGKILL after write | Original target dies without a returned confirmation |
| Kill caller SIGKILL after API return | Target dies but no application confirmation or recovered success is fabricated |
| Kill caller SIGKILL after empty observation | Empty-at-read still grants no attempt completion |

The four cuts concern the private kill operation. Instrumentation notices select
the cuts, not application ACKs. No new enrollment socket exchange, original
delivery, release, durable completion or full-frame crash matrix is claimed.
The two-guardian common-mode and hidden-writer negatives in the old driver remain
unchanged and expected. New full original-frame gates are not rerun here; the
exact source guards protect every old body before the future join.

## Preliminary and negative receipts

`red-missing-module.log` retains one actual expected failing source control.
`red-exact-filter.log` ran zero tests due to an over-specific filter and is not
red or passing proof. `gnu194-first.log` retains a new test-only private
checkpoint visibility error, corrected to parent-only visibility without
changing old code or predicates. Preliminary broad `root_enrollment` filters
also matched one old unrelated authority control: their eight passes are not
eight new controls. Selected cohorts use the exact module prefix and pass seven.
The first kernel screen predates the added fixture-fence assertion; only the
committed-source selected cohorts include that assertion.

Windows passes 54 ordinary and eighteen compile-fail controls; strict affected
Clippy passes. The runner's existing-cohort, path-escape and unsupported-compiler
guards all refuse before a new cohort or cgroup is created. Raw logs are retained
without trimming, prefix removal or newline normalization.

The complete source suite passes 277 controls, preserving all 381 prior
supervisor Rust bodies through exact additive adapters. The new COMPAT entry
and manual registry block are checked exactly, including mutation controls.
Old packet integrity verification passes again: 94 current Pending files and
182 unchanged files across four independently audited historical packets.
Documentation, registry, scoped formatting and the local non-promotable
performance contract pass. The Windows registry reports 184 discovered gates,
all registered; that discovery count is not evidence that this Linux/root-only
manual fixture ran. Its six explicit matrix receipts supply that proof.

## Reproduction and integrity

Use `run.ps1 -Compiler 1.94.0 -Label <fresh-label>` and then the same command
with `1.88.0`. It requires clean selected sources, active local time service,
the expected WSL build account and explicit local-root fixtures. It never
deletes/adopts pre-existing fixture roots, overwrites a cohort, changes a clock,
installs a unit or stops a supervisor. Preserve this closed packet: for a new
run copy the recipe to a fresh sibling evidence directory at the same depth
and retain new receipts there. Do not append runs to the sealed inventory.

`verify.ps1 -SelfTest` checks the closed file set, exact raw byte lengths/SHA256,
both source/image/recipe/scope receipts, all six kernel matrices and eight
in-memory inventory mutations. The per-packet attributes preserve raw bytes in
Git. These checksums are not signatures or production enrollment authority.
