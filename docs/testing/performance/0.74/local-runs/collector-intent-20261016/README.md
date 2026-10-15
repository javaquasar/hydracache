# Signed collector requirement verification

The [design](../../diagnostic-collector-intent-design.md) implements the
[preregistered prerequisite](../../diagnostic-collector-intent-plan-20261016.md)
without starting a process or advancing lifecycle. Runtime is `9e936fcf` and the
independent static guard is `9303bb7e3746fdeddaf2171515a66718ddc9fc15`, tree
`26285e32a712b4d185ebc82fe8605c6417e2f685`. Baseline is
`46f721e9ad98d5404f3682819b11be550c36f566`; only documentation/evidence
separates it from the preregistered original runtime baseline `c34add1b`.

## Scope and results

The exact old integration target passes 16 tests before the missing-API probe.
The probe then fails with E0432 for the absent collector module. Its same retained
patch from the [blocked packet](../collector-intent-prerequisite-20261016/README.md)
now reaches Rust; that packet's WSL process-creation refusal remains historical,
not relabeled as a compiler test. The first implementation passes 29 tests.
A final output-field hashing/repeated-verification regression raises the group
to 30: three current-Rust repetitions and one Rust 1.88 repetition all pass.

The group contains 16 existing and 14 new tests. It verifies all four modeled
cells, exact retained typed worker command/properties, every fixed output field,
ordered pipe slots/finalization requirements, both old/new signature/domain/unit
digest refusal directions, strict canonical/schema/budget/principal/window rules
and original state/peer/envelope/configuration/worker refusal. All ten output
fields are hashed; two array reversals change the digest. The deterministic
state matrix uses seed 0x0740c097 for 64 valid mutations and four independent
simultaneous guards. One guard's refusal does not affect another. Re-verification
of bytes is explicitly allowed and is not a replay ledger.

Each compiler passes the two existing typed-projection unit tests and five
compile-fail API tests. The latter print expected compiler errors: cloning,
legacy promotion, deserialization, start and completion remain unavailable.
Their native exit status is zero because the negative examples must not compile.
The eight final Linux/Windows, Rust 1.94/1.88, check/strict-Clippy lanes pass
for both supervisor and xtask, all targets/features, locked offline dependencies.
The full static contract suite passes 170; the automatic registry stays at 176.
Workspace formatting, docs and local non-promotable performance contracts pass;
require-ship refuses as expected because candidate/qualification are incomplete.

## What remains unproved

These signatures authenticate requirements, not their fulfillment. No actual
pipe/sink/root ownership, loaded executable/environment, manager invocation,
original process/cgroup, watchdog or writer revocation is authenticated here.
No filesystem/process/service/journal/start route or lifecycle conversion is
added. Production trust/pins, original-start, completion, later cells and server
admission remain open; the historical Kernel(Invalid) remains unexplained.
There is no product workload, qualification, performance result, full workspace
milestone verification or push in this packet.

The old verifier differs only by the new module registration, and the old unit
builder, collector writer, Starting/request primitives, dependency lockfile and
qualification manifest are unchanged. COMPAT has an explicit additive byte-format
entry, not a rewritten old contract. Historical packets are immutable: their
verifiers that pin the previous entire COMPAT file remain scoped to that old
source, rather than being rerun against the intentionally extended register.

## Environment recovery and provenance

After observing less than 60 MB free, Cargo's package/profile/target-scoped dry
run identified 5.8 GiB of reproducible Windows supervisor dev artifacts. Its
matching clean removed 22,693 files and reported 5.8 GiB; source, evidence and
release profile were outside that operation. Both Ubuntu and docker-desktop
were observed Stopped before resetting the failed WSL VM. Ubuntu then launched
successfully with its root mounted rw, and the baseline/probe could run. This
does not establish a complete filesystem-health or clean-build attestation.

The baseline and missing-API logs also retain Cargo's corrupt-incremental-artifact
warnings for builder/projection caches after the environment failure. Cargo says
those artifacts are ignored/deleted; final captures do not repeat them. The warnings are not suppressed
or reclassified as the intended E0432. Dependencies and caches are reused; only
source-object/byte consistency and the stated test results are claimed.

The [manifest](manifest.json) inventories 26 UTF-8/LF command captures and hashes
all packet files. Run `./verify.ps1` to verify source/frozen/artifact identities
and result summaries. This is unsigned local consistency, not signed execution
attestation, binary reproducibility or a clean-tree build claim. No final poll's
wall-time field is represented as a total command duration.
