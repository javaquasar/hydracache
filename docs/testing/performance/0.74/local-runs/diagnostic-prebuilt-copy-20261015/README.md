# Unsigned copy preparation and original preparer process loss

The new preparer creates a separate file with exact asserted bytes, closes its
own writer and retains read-only inventory. It neither enrolls protected ancestry
nor authenticates the prebuilt artifact/runtime/job. Source and output are different
inodes; the output is not silently substituted for the original Cargo executable.

## Source and verification

Implementation source `6148aa9b11d5768c74bd31e4175c3332d3715ada`, tree
`3d7348be70e7c7f4171ee29ff4e6ea36998bbce4`, baseline
`a7819987b3a7f5b4efea7925abeec1b29b9995a3`. Preregistrations `e903d52e`
and `c191621e` precede implementation. All new production-facing code is opt-in;
controller/server sources, pins, dependencies and frozen qualification are unchanged.

The baseline has 10 passing bundle cases. Missing-module E0432 is expected red.
Final Linux vectors are 16/16/16 local cases, three copy integration cases,
six issuer cases, 18 old bundle/executable cases, three compile-fail API cases,
then 16 local and three integration cases on MSRV 1.88. Each 16-case suite
includes the child-only helper entry and seven actual owned-child SIGKILL cuts.
The new tests do not execute either actual controller artifact. Existing executable
regressions run only their own disposable helpers.

Current/MSRV all-target checks, strict current lint and the Windows issuer suite
pass. Windows does not run Linux-only copy tests. 151 source contracts, 170
registered gates, doc-check, local non-promotable performance contract and scoped
format pass. require-ship remains expected red even at the clean implementation.

Three development failures are retained, not counted as successful final gates:
an unregistered Linux target, module declaration formatting, and a metadata-only
source-mutation check that passed two repeats and failed the third (15 passed/1 failed).
The source mutation was real; the corrected implementation rehashes original
source content after copying without timestamp delays. Those precommit captures
are development observations, not independently identified clean build candidates.

## One actual local copy

The input is the previous unsigned controller built from source
`16e9420313ef842e19183c7f6f189eb2e10207cf`, not the new preparer executable.
Its 1,922,976 bytes hash to
`ca7bc17ad21d9495c48945430289e6626540cffb511ffc206fda42a1b86659ba`.
One clean-source invocation of the new local example, without credentials in its
environment, creates:

`/home/artur/.cache/hydracache-074-prebuilt-copy-20261015.Ruo6eQvc/attempt-00000000000000000000000000000074/diagnostic_controller`.

The source remains device/inode 2096/262375, mode 0755, UID/GID 1000/1000 and two
links. The copy is 2096/251371, mode 0555 and one link; its directory is 0555.
Size and digest match. Source metadata/hash and the preparer binary hash match
before/after. The preparer is a local Rust 1.94 debug example built during final
checks; its observed SHA256 is
`aa6e84b79e8fde4aafb6d2d6dd10647fbbf0126c2c1ff14e6c78b1add31d7847`.
This is one observed preparation, not reproducible compilation or signed provenance.
Both actual controller files stay unsigned/unexecuted outside Git. The retained
copy is local cache inventory, not a release distribution or admitted bundle.

The invocation uses `env -i HOME=/home/artur PATH=/usr/bin:/bin` and
`/home/artur/.cache/hydracache-074-194-check/debug/examples/prepare_unsigned_prebuilt`,
then the original source file, private parent, attempt name,
asserted SHA256 and byte count as five arguments. Build with
`cargo +1.94.0 build -p hydracache-long-run-supervisor-074 --example prepare_unsigned_prebuilt --locked`.
Any further local repetition requires a newly created owner-only parent and a
fresh attempt; do not rerun against this retained destination. No output cleanup
or mode repair is part of the invocation.

## Remaining boundary

The negative control retains a foreign writable descriptor during creation.
Inventory initially passes while that writer still exists, and the descriptor
writes after chmod. Revalidation then refuses permanently, including after byte
restoration. Closing this function's writer does not revoke every other writer.

Protected ancestry/account/runtime/job selection, authenticated receipt acquisition,
actual signer custody and token coordination remain before production signing.
Worker original-start, all P2/P3 writers, completion cells 2–4, server rehearsal
and qualification remain closed. No product workload or performance claim follows.

Run `python verify_packet.py` and `python test_verify_packet.py` in this directory.
All nine validation/mutation cases pass on Windows and WSL Linux. The manifest
binds 12 source files, 20 unchanged inputs and 12 raw captures, including the
development failures with their original nonzero exits.
The verifier reads only repository evidence/source bytes and Git objects, never
private keys or the actual binary. `--committed <SHA>` additionally checks exact
packet bytes against Git. Counts/sizes here describe correctness and inventory.
