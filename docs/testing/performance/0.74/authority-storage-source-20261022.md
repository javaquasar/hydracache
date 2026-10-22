# Unapplied owned authority storage source candidate

The fixture journal now has a separately retained owned-scratch source candidate
and a versioned mutable observer. Complete-file patch allowlists and independent
body restoration pass locally. Neither file is installed in the supervisor:
the current fixture, old observer and existing source guards remain unchanged.

Method commit: `f193fabae86c9bd7a2e778a9463a43984834deb5`.
Baseline commit: `33c54b0cda8c330b115009cdf092ad002077cedf`.
See the [preregistered plan](authority-storage-source-plan-20261022.md),
[candidate](../../../../tools/authority-storage-source-074/candidate.rs),
[observer](../../../../tools/authority-storage-source-074/observer.rs) and
[source tests](../../../../crates/xtask/tests/authority_storage_source_074.rs).

## Owned storage without replacing fresh validation

Each Leaf gains one optional scratch Vec. Its constructor still executes the
original allocating read and starts with None. Fresh validation opens the named
file again, checks regular-file ownership, mode, link count, device and budget,
then takes a slot only when its capacity exactly matches the observed length.
It clears the Vec length before the original bounded EOF read. The after-read
metadata, byte length, retained stamp and expected bytes are still checked.

Retention happens only after both original comparisons, with exact length
capacity and the existing budget bound. The fresh File remains alive through
comparison and retention. A Leaf error clears its slot. Journal validation,
binding, transition and publication errors now call the existing refusal latch;
that latch also clears every record and marker slot. Original descriptors,
stamps, bytes and history remain retained. Four internal failure paths and the
external latch account for all five cleanup boundaries identified by the audit.

The hypothesis is that repeated successful fresh reads can reuse allocation
storage. This step measures neither savings nor retained memory. Real allocator
behavior, layout, RSS and filesystem performance still need separate evidence;
the earlier synthetic model results do not become actual-engine measurements.

## Reopen does not transfer a cache

New Leaf constructors begin empty. Reopen reconstructs entries from files and
keeps its original final journal revalidation, which may populate their new
slots before returning. No scratch is transferred from a previous owner. An
assertion that every returned reopened journal has empty scratch would be wrong
for this candidate; removing the final validation to achieve it would change the
original schedule. Neither change is made.

## Source checks and their limits

The contract derives expected complete files from independently pinned baseline
sources through count-checked replacements. Full Rust token comparison rejects
unreviewed helpers, fields, attributes and effects; syntax parsing rejects invalid
Rust. Independent restoration compares complete fresh-read and validation bodies
after removing only the allowed storage substitution. Record-before-marker order,
directory/inventory brackets, reopen and publication bodies retain their original
checks, effects and checkpoint ordering.

The mutable observer keeps the old allocating reference, site enum and fixture
phases. Its new cached read has the same six ordered observation sites, and the
held-metadata observation remains before that read. Unwrapping exact expression
hooks reproduces the candidate's allocating read, reusable read, validation and
refusal wrapper bodies. Mutable fixture references are part of the whole-file
allowlist, not inferred from the old immutable observer.

Eight source controls include per-occurrence mutation rejection for storage and
validation checks, every internal refusal site, missing or duplicate observations,
extra effects, receiver drift, duplicated methods and altered baseline anchors.
The checker is a fixed patch allowlist, not general semantic equivalence or a
Linux typecheck. The candidate and observer artifacts are parsed, not compiled
as supervisor modules. No actual filesystem, cache behavior or crash safety is
claimed from these controls.

## Development refusals and final cohort

Three development failures are retained in a
[contemporaneous transcript](../../../../tools/authority-storage-source-074/development-refusals.txt).
The first failed before running tests because the digest array lacked LowerHex.
The next ran seven controls with three refusals: a rustfmt-added trailing argument
comma, an extractor incorrectly expecting one registry impl instead of the
baseline's two, and the corresponding observer token mismatch. A misplaced mut
in the repair then caused a compile refusal. The original terminal outputs are
in the chat record; the transcript is not presented as redirected raw logs.

Repairs format digest bytes, account for both existing registry impls while
requiring unique selected methods, and handle a trailing comma only in recognized
helper calls. Whole-file comparison still rejects extra blocks. Acceptance
predicates, pinned baseline and runtime checks are not relaxed. The final source
adds an independent storage-only body restoration control before method freeze.

The first captured final cohort uses the committed method and a no-clobber output
directory. Its raw logs and source snapshot are retained in
[the evidence packet](local-runs/authority-storage-source-20261022/).
The snapshot preserves raw Git archive bytes; it is not substituted for the
working runtime's independent raw-byte identity check.

All 27 first final steps passed on MSVC Rust 1.94: eight new source controls,
79 unchanged source canaries in 30 binaries, 129 offline controls, formatting,
package check/strict clippy, documentation, 186 registered gates and the local
non-promotable performance contract. The subsequent documentation check also
passed. The preceding historical packet seal and all 55 of its members were
verified unchanged; the historical Python readers were not modified.

Evidence commit: `bee77c949c6b6929e0ec30de96004b8896c8394e`.
The inventory covers 40 members, excluding inventory.json, with SHA-256
`4b409fcbe7428d3dd6e7b3f8e8097cb5e8c62668512f56b5b262ae5645c8cd1e`.
All 41 packet files have identical raw working and staged bytes.

## Remaining integration gates

Historical Rust guard wiring still needs an explicit frozen-source path alongside
current-candidate validation. Do not redirect every guard to history and infer
current safety from archived success. After that separation, integrate the minimal
runtime patch and compile the mutable call sites on Linux, then check actual
temporary-filesystem success/refusal, whole-journal eviction and reopen behavior.

Original validation count/order, 300/60/10 windows and first Causal refusals
10042390247/10010835739 ns remain unchanged. Qualification raw SHA-256 remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
No WSL, live/original-owner or old measurement image, server, installation,
supervisor, product workload, clock or service action occurred. Publication,
all-writer termination, full SIGKILL/lost-ACK, reconnect and server admission are
still separate gates.
