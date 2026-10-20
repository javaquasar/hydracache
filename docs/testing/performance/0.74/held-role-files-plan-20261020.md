# Retained sealed role files before original admission

Baseline is 773e6bf0. Inherits docs/RULES.md. The static held receipt currently
compares caller-provided bytes, while FixedPlacement retains the original owner,
root guardians and fixed slice. Neither supplies the other's missing proof.

Implement the next bounded prerequisite: consume the checked held receipt and
two already-open Linux role files. Require distinct regular, read-only, CLOEXEC,
non-O_PATH descriptors with exactly the four immutable memfd seals. Verify bounded
ELF/content against the receipt, lock and raw Cargo log, then retain descriptors
and kernel stamps. Revalidation uses positional reads without changing shared
offsets, compares exact role expectations and is sticky after any refusal.
Expose no raw FD, pathname reopening, serialization, clone, execution or release.

This package does not authenticate the creator of the descriptors, their install
ancestry or running process identity. It does not connect an asserted policy epoch
to the live original authority. The complete consumed FixedPlacement and original
policy/key/epoch/frame join remains the following package, before any projection
or delivery. Do not replace that join with matching hashes or a caller callback.

First retain baseline static controls and a missing-method compile refusal. Add
Linux tests for sealed acceptance, swapped/aliased roles, wrong bytes, unsealed,
writable, O_PATH, pipe/directory, missing seals, lock/log mismatch, unchanged shared
offsets, sticky refusal and seeded content corruption. Synthetic keys/ELF/logs
remain nonproduct fixtures. Run GNU 1.94/MSRV 1.88 targeted controls, Windows old
controls, strict lint/fmt, exact additive source guards, registry/docs and local
contract. Preserve all 440 existing runtime files except the single module hook,
old formats, qualification digest and all sealed evidence. No new wire format.

No original send/start, watchdog arming, all-writer retirement, lifecycle completion,
server access/installation, supervisor operation, production enrollment or product
workload is opened. Existing 46 live brackets and 300/60/10 origins stay unchanged.

## Implemented content retention

Implementation 32ec9525 adds the Linux-only role_files module and PinnedRoles.
The checked receipt and both files are consumed; full content hashing uses a
fixed 64 KiB buffer and positional reads. Exact seals prevent writable aliases
from modifying content. Kernel stamps also detect observed metadata changes.
A refusal cannot be cleared by later restoring correct expectations or mode.
No FD extraction or execution method is exposed. This is immutable content
custody, not authenticated creator/process custody or a live epoch join.

Ten Linux integration tests extend the existing sixteen static controls. They
include a writable alias of the same retained file, concurrent shared-offset
changes and 32 seeded corruptions (0x0074_f11e). Source guards reconstruct all
440 preceding files after removing only the new hook; the new named source is
the only addition. The [local packet](local-runs/held-role-files-20261020/README.md)
separates development selections from accepted source/image/recipe cohorts.

The first recipe's compile-fail selection reveals a GNU 1.94 diagnostic-renderer
panic that rustdoc counts as success. It is not accepted. Multi-line formatting
alone reproduces it. The FD-extraction control instead requires the absent
IntoRawFd trait (E0277); the start control requires E0599. The final recipe also
rejects compiler panic text.
An initial xtask strict-lint refusal is retained: the predecessor bridge imports
only the normalizer, so its two unused source-list controls get narrowly scoped
test-helper annotations, not a whole-module warning exemption.

The next join must consume the same FixedPlacement and bind its original signed
claims, controller, independently admitted held-build policy and live authority
epoch before and after the unchanged complete frame. A caller-supplied hash,
ExpectedTrust epoch or callback cannot implement that admission. Protected role
receipt production and bootstrap/process binding remain separate prerequisites;
the local helper cannot yet receive an original release.

Rehash cost on real role images remains unattributed: fixtures have short ELF
headers, not release-sized binaries. Measure that preparation cost before joining
the original frame. Exhausting an original deadline must refuse, not renew it;
seal/content retention cannot substitute for a required live authority check.

Selected source 82570db8 passes two 26-test rounds and four capability docs per
GNU/MSRV compiler; Windows passes two 16-test rounds and two old docs. Twelve
source guards, three predecessor bridge controls, strict lint, registry (186
registered gates), doc-check and local contract pass. A silent formatter exposes
a missing-log bug in the final recipe wrapper; the retained recovery recipe
rechecks formatting and source freeze and supplies an explicitly bound completion
receipt. The packet verifier also refuses 34 in-memory metadata mutations.
