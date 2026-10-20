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
