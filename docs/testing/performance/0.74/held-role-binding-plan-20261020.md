# Signed role build binding before original admission

Baseline is 697a911aae11cab00b67046c80b42713d3d9e3e7. Inherits docs/RULES.md and
the approved minimal [artifact contract](held-artifact-boundary-audit-20261020.md).
The existing v3 placement authenticates role hashes and attempt/root/deadline,
but does not name the new held policy or receipt. PinnedRoles retains immutable
content, not a controller authorization of that provenance for this attempt.

Implement the already planned separate v4 signed binding. Its body retains the
complete signed v3 placement and names the held policy digest, canonical signed
receipt digest and epoch. Validate both controller signatures using the same
controller named by the independently checked held policy. Require that policy,
receipt, placement consent epoch and repository agree, and that the receipt's
fixed role map matches all signed placement hashes. Do not reuse the worker
builder key, select a new production pin, upgrade an old signature or add a replay
slot. External signing bytes remain distinct from an enrolled original owner.

The new inspector returns opaque static checked data with exact placement-byte
comparison and content verification only. No File/FD, clock, original owner,
manager, start/send/arm/release/completion/unlock method. Register bounded
canonical schemas and domain in COMPAT; preserve old signed bytes and all 441
preceding runtime files except one exact additive module hook. Add one named
runtime source and exact adapter controls, preserving qualification and sealed
packets. No old live predicate or 300/60/10 origin changes.

First retain baseline tests and a missing-module refusal. Add portable canonical,
signature/domain/key, independent-pin, cross-epoch/receipt/role/attempt binding,
re-signed invalid data, nested unknown/duplicate/malformed fields, budget and
seeded mutation controls. Extend existing synthetic fixtures, not production
keys. Run current GNU/MSRV/Windows targeted tests, capability compile failures,
source/adapter guards, strict lint/fmt, registry/docs and local contract checks.

This closes the static cross-document contract only. Before an original join,
an independently admitted held trust source still must exist; the reviewed pins
currently cover only the old worker builder/controller/worker issuer. Then
consume the same FixedPlacement and role files, select original controller and
live authority epoch before/after unchanged full frames, and measure real-image
rehash cost. No server, installation, supervisor, system-bus/original send,
production enrollment, actual role receipt production or product workload.

## Audit findings and implementation boundary

KernelWorkerPolicyInput::controller_authority_binding reads the retained active
authority registry and revalidates the same input. The original validation frame
compares its observed policy/epoch with the prepared consent and checks original
clocks. In contrast, the portable v4 inspector can compare signed consent claims
only. Equal epochs in its output do not replace that journal/kernel observation.

diagnostic_reviewed_trust pins the existing controller, worker builder/policy and
worker issuer. It has no reviewed held-runtime builder or held-policy digest.
Do not select either from an incoming v4 document, reuse the frozen worker key
for the new purpose or add a trusted default merely to make a join constructible.
The new inspector accepts only the existing opaque CheckedPolicy, whose external
inputs remain reviewed assertions until an original trust owner admits them.

The implemented v4 body carries the complete signed v3 envelope, not merely its
image hashes or a decoded projection. The new signature and old placement
signature both use the policy's independently selected controller. The held
receipt retains its separate builder signature. Exact receipt/policy digests,
epoch/repository and all three expected roles must agree. Another correctly
signed attempt can inspect successfully as static data but cannot match the
retained original placement bytes. This comparison is not replay authority.

The local recipe rejects compiler panics and records silent successful commands
as explicit zero-exit receipts. It does not edit or reuse the preceding packet's
failed wrapper or sealed recovery. Real-image hashing attribution, fixed trust
admission, the consumed FixedPlacement/role-file join and its full live/fault
matrix remain subsequent gates.

## Accepted local gate

Implementation b479d5a0 and selected source c24f460a pass two rounds of all 28
tests and four capability examples on Windows, and two rounds of all 38 tests
plus six examples on each GNU 1.94/MSRV 1.88 compiler. The twelve new portable
controls include 64 seeded corruptions per round (0x0074_b14d), correctly re-signed
mismatches and rejection of old/new signature substitution. Fourteen source
guards, three predecessor bridge controls, strict lint, scoped fmt, registry
(186 registered), docs and local contract pass. The explicit silent format receipt
records exit zero; no wrapper recovery or previous log rewriting is needed.

The [new packet](local-runs/held-role-binding-20261020/README.md) binds exact
source/compiler/recipe/test images and rejects 34 in-memory metadata mutations.
The preceding role-file packet and qualification digest remain fixed. These are
static binding/format results, not an original live join or performance attribution.
