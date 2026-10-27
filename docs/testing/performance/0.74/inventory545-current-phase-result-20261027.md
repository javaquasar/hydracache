# Inventory after the approved Tokio update

The first two inventory failures in the retained V8 package have a specific
cause: the live source index contains the approved Tokio successor, while the
test inventory still stops at the earlier 544-file phase. This continuation adds
an exact 545-file phase and preserves the independent older inventories and
physical checks. The product runtime, dependency graph and execution admission
do not change.

Implementation commit: `17edd6b83fd2960e9c4351d5f0c880dc9d44bbdc`.

## Baseline and fixed source delta

The [original V8 report](roots-lock-view-package-v8-result-20261027.md) retains
`source index drift` in `artifact_inventory_phase_544_074` and
`artifact_inventory_reader_074`. Its 780-second timeout, 36 failures and 67
unstarted targets remain the first full-package result, not amended coverage.

The independent source pair is predecessor
`958b3135a15c00b9f6aad41522cef9d1f6608d3a` and approved Tokio producer
`2a4a771d48ed6ad54288fd15ea7c75b56f4bd80d`. Exactly one old file changes:
`diagnostic_manager_channel.rs`, with the owned AsyncFd registration repair.
Exactly one file is added: `diagnostic_manager_alarm_tests.rs`. There are no
removals or other changes among the 544 predecessor files.

The separately SHA-pinned descriptor records original modes, Git blobs and
canonical SHA256 bodies for both sides. Its SHA256 is
`1f41899edf172c4ae0a29f17a5fd8a1cee0fd8bf1047c5da5c4ea94b1dca962b`.
The read-only generator must reproduce this exact descriptor from the fixed
trees. It cannot learn pins from the live index or bless unrelated changes.

## Validation and preservation

The current ledger extends the existing 544 phase with the exact changed/added
metadata and checks an exact inverse roundtrip. Fresh validation still checks
the complete current names, Git mode/blob/stage, physical filesystem inventory,
symlinks/reparse points/types and canonical contents. Names reach an older phase
only after every current body passes, through `545 -> 544 -> 530 -> 446`.

The 530 ledger and 544 descriptor/module remain unchanged. The entire physical
implementation beginning with `artifact_inventory_input::root` and all five
original reader tests remain byte-identical to their fixed predecessors. The
544 test target now proves its old complete tree through the inverse map and
observes all 544 names plus the new name once per successful current round.
No successful observation is cached or reused after a later failure.

Seven new controls compare all 545 mode/blob/digest entries with the independent
producer, perform repeated physical reads, refuse stale 544 names/index, mutate
new and changed body reads, corrupt index fields/presence, mutate inverse
metadata and check byte preservation. Historical Git archive bytes are data,
never extracted, compiled or executed.

## First outcomes and local checks

The first new-target baseline compiles and fails all six initial cases against
the old ledger. Five expose the absent 545 phase. The sixth exposes a test defect:
it names the older inventory-reader commit for a descriptor that was introduced
later. The corrected independent descriptor producer is
`77ccbe7d57203d1c5f263a4046aee0b81e06357e`; the original physical-tail predecessor
stays `2b83dff9d7445aa8a14890b51ae5d4b276b048f7`.

The first implementation passes all nine 544-phase cases and five of the six new
cases, but its archive oracle rejects Git's global PAX commit header as a
nonregular source entry. The correction accepts only the one exact
`pax_global_header` with `comment=<fixed producer>`; all source entries still
must be regular and all 545 digests must match. The first sources and both
failure logs remain retained. A seventh inverse-metadata refusal control is
then added before final validation.

The final 21-case cohort passes separately on Windows Rust 1.94, Windows MSRV
1.88 and GNU/Linux Rust 1.94 in local WSL. GNU uses the existing explicit
GIT_DIR/GIT_WORK_TREE test context and existing target cache; no config or
worktree pointers are rewritten. All-targets check and strict all-targets/
all-features Clippy pass. Formatting, doc-check and the local structural
performance contract are checked separately. These are semantic/source gates,
not CPU or throughput measurements.

## Source and effect review

The scoped review covers the four changed/added Rust bodies, descriptor,
attributes and read-only generator. The two new Rust bodies are a pure metadata
phase and an ordinary test target using fixed Git/tree/archive reads, bounded
maps, local source reads and injected in-memory refusals. The existing physical
IO body is unchanged. There is no new service, manager/worker launch, bootstrap,
environment opt-in, live clock or execution authority. Only immutable compiled
ledger data is cached. This scope review is not the next complete package AST/
effect review and does not extend V8's old source clearance to changed code.

The [separate evidence packet](local-runs/inventory545-current-phase-20261027/index.json)
retains all first/corrected checks, first source bodies and the reviewed current
scope. No full package, product workload or measurement is replayed. The frozen
qualification manifest stays SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`;
runtime live predicates, observation frequency and original 300/60/10
origins/windows remain unchanged. No clock/service/server/installation changes
occur.

Next close the terminal/authority historical-view mismatch from the saved V8
logs, then the remaining lock/document/frontier/held-role boundaries. The full
package and release remain unclosed until a separately reviewed changed-source
gate completes under the same original command, threads, floor and timeout.
