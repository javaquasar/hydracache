# Sealed child-entry binding under the original owner

This component materializes a non-delivered 1024-byte assertion record and
extends the bootstrap owner's census to 27 descriptors / 15 semantic roles.
It does not implement a child executable, authenticate a child, deliver an FD,
remap descriptors, call execveat or admit original start/server operation.

## Baseline and hypothesis

Baseline eedd826c closed the three executable images into 26 descriptors, but
could not bind a future child entry to all original attempt data. The
[plan](bootstrap-entry-binding-plan-20261023.md) separates this materialization
from the later execution boundary. A sealed record alone cannot authenticate
who created it: a caller can seal fabricated assertions just as easily.

The parent therefore generates expected bytes from its own current original
Context, already verified signed bootstrap documents, actual sealed images,
legacy material and closed census. It compares every byte on both existing
inspection rounds, under the same three complete frames. Nothing removes,
merges, caches or reorders the original frames; 300/60/10 origins and absolute
bounds are retained. No new time source or refreshed window is introduced.

## What is bound

The additive [COMPAT format](../../../COMPAT.md) fixes magic, size, padding and
every field offset. It binds owner credentials/thread, origins/bounds, attempt
and original context, root identity/deadline, serial, manager names, seven
document digests, six image identities, streams, guardians, image digests,
legacy bootstrap digest, parent census fingerprint and canonical state digest.
Names hash UTF-8 bytes; state hashes canonical newline JSON. Structural decode
does not authenticate any of these values. Shape-valid mutations decode but
must differ from the parent's complete expected record.

The config identity is that of the actual sealed runtime copy, NOT the original
config source inode. The private legacy contribution checks its exact existing
256-byte record against the same private token/bindings; it returns only values
and a digest, not the token or descriptors. Old complete frames still establish
creator/guardian/live authority; this value leaf does not replace those checks.

The old sealed constructor creates the new memfd once, closes the writable alias,
and retains a readonly/CLOEXEC/exactly-four-sealed File with initial offset zero.
Subsequent inspections compare contents, metadata/flags/offset and disjointness
from every prior descriptor and inode. A missing entry after retention refuses
instead of recreating it. Entry role 14 is a parent semantic slot, not an
inherited-FD destination. The parent-local census fingerprint is not a remapping
table, signature or transferable capability.

The original owner remains the first field; explicit refusal and Drop retain
the preceding original-retirement order before added descriptors close. New
failures use the same shared sticky refusal, not a recoverable execution handle.
Exact additive source views reconstruct entire preceding owner/kernel/census/
build bodies and old COMPAT bytes. Hook mutation/duplication and unrelated
drift refuse. No activation, old launcher, guardian, manager or lockfile code
changes. The frozen qualification manifest remains byte-identical.

## Evidence and limits

The [first result](bootstrap-entry-binding-first-result-20261023.json) preserves
the absent-format source failure and a later E0283 test compilation failure.
They are explicitly captured-tool summaries, not raw fixed-runner receipts.
The type correction changes only an ambiguous digest conversion in a test.

The first fixed host lane passed tests/check but refused Clippy duplicate_mod:
two preceding guards both loaded the new helper. The exact source snapshot and
raw refusal remain retained. Loading the helper once per integration-test crate
fixes that lint without suppression or runtime changes. Both initial Linux
lanes passed; final lanes were kept separately after the helper/tooling change.
No original live or measurement attempt was repeated.

All final fixed lanes passed on stable source:

- Windows: 34 selected source-contract tests, two entry codec tests and five
  signed-build tests; all-target check, strict all-feature Clippy and formatting.
- GNU 1.94 and MSRV 1.88 each: seven entry codec/factory/private-FD tests and
  thirty prior image/build/owner-model/source regressions, 37 selected tests
  per compiler; all-target check and strict all-feature Clippy. Default-feature
  check retains the preceding unused Reading::Reference support warning.
- Three offline runner/retention controls, doc-check, 186 registered gates,
  local non-promotable performance-contract-check and diff check.

Codec and modeled context/census tests are not original live admission. Private
memfd/pipe/socket checks exercise real kernel objects, but their old-prefix/
creator fixtures deliberately do not claim an authenticated original process.
No child executable is launched. The original owner fault models still check
each existing failure cut, closing-frame order and sticky refusal.

The [exclusive packet](local-runs/bootstrap-entry-binding-20261023/audit.json)
checks source commit `41bf7567`; evidence commit is `cfd16395`. Its
[inventory](local-runs/bootstrap-entry-binding-20261023/inventory.json) lists 333
members excluding itself, SHA256
`59cd95b450a03a435743571decf05744eec50ca58305255b8905557aea12debd`.
Raw initial/final commands, outputs, source hashes and initial source snapshot
are separate from the earlier captured-tool summaries. Before evidence commit,
an exact staged-Git-blob audit verified all 334 files including inventory against
recorded hashes, sizes and worktree bytes. Plan commit is `6f3b6fab`; scoped
tooling commits are `00e8f7c6` and `f1d891e5`.

Qualification SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

New hashing, positional reads and descriptor metadata queries have unmeasured
cost. Unit/compile elapsed time is not CPU/op, allocation, latency or original
deadline evidence. No repeated measurement or original live retry, clock/service
change, privileged mount, server action or product workload occurs.

## Remaining execution boundary

Implement the separately enrolled bootstrap executable and bounded child-reader.
Define and authenticate its exact arguments/environment, inherited FD roles,
credentials, original root, manager and guardians before one-shot execveat of
the same sealed launcher, without a pathname/copy fallback. Then prove real
original entry within unchanged windows, writer cessation and SIGKILL/lost-ACK
boundaries before completion/later cells, reconnect or server admission.
