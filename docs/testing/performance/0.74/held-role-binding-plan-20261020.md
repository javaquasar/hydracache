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
