# Selected issuer and unsigned local prebuilt inventory

The operator selected the previously generated signer-build public fingerprint.
The code pin has no caller key override. An explicitly asserted build-selection
factory still returns the old diagnostic-only model; it does not authenticate
expected statements, original prebuilt files, runtime or job custody. The old
generation record remains unchanged, with its historical unapproved flags.

## Source and tests

Source `16e9420313ef842e19183c7f6f189eb2e10207cf`, tree
`9fc430ed1c09934eaa4ebdb61dc282233e63a91b`, baseline
`08dc602d5ea6cbcc17ef3865e0aeb2e14bf7d1b4`. The preregistration precedes code.
The missing-module E0432 capture is expected red, not a passing implementation.
Three six-case Linux repeats, one compile-fail override test, existing trust 8
and signer 9 tests, MSRV six-case tests/checks and strict current lint/check pass.
Windows has six-case current/MSRV tests and the compile-fail guard. The unchanged
Python helper's 24 tests pass. 150 source contracts, 169 registered gates, doc-check,
local non-promotable contract and scoped format pass; require-ship stays red.

The synthetic receipt control is deliberately accepted by the old caller-key
model and rejected by the selected-key factory despite identical expected body
and supplied bytes. The original public selfcheck and all 64 signature mutations
use public material only. No private key or actual signer-build receipt is used.

## One actual unsigned build

An exact clean source produced one local diagnostic_controller release artifact
using Rust 1.94, System and x86_64-unknown-linux-gnu with the existing locked command.
The isolated build environment contains only HOME, PATH, LANG, its separate target
directory and CARGO_BUILD_JOBS=2. There are no user Cargo config overrides; the
repo config contains only its existing xtask alias. Before/after Windows Git
source/tree/clean and raw root-lock hashes match. This is one observed build,
not independently authenticated compilation or bit-for-bit reproducibility.

The artifact remains outside Git at
`/home/artur/.cache/hydracache-074-signer-prebuilt-194/x86_64-unknown-linux-gnu/release/diagnostic_controller`.
Its 1,922,976 bytes hash to
`ca7bc17ad21d9495c48945430289e6626540cffb511ffc206fda42a1b86659ba`.
It was not executed or signed. Readelf 2.42 observed ELF64/x86_64 PIE, interpreter
`/lib64/ld-linux-x86-64.so.2`, direct NEEDED declarations `libgcc_s.so.1`,
`libc.so.6`, `ld-linux-x86-64.so.2`, and no RPATH/RUNPATH. Version needs include
GLIBC_2.34. These are static declarations, not a loaded-closure or confinement proof.
The hash is checked before and after inspection. No ldd or dynamic loader runs.

The actual Cargo artifact is writable by its owner (0755) and has two hardlinks.
The existing original bundle-reader requires 0555 and one link. This artifact is
therefore NOT an admitted bundle input. No chmod, link deletion or copy disguised
as custody was performed. Safe preparation must account for original file identity,
all retained writers, protected ancestry and independently selected runtime/job
policy; merely changing mode bits does not establish those properties. The binary
is a rebuildable local cache artifact, not a retained signed release distribution.

## Verification and next boundary

The manifest/verifier bind 11 source files, 19 unchanged inputs, 11 raw captures,
exact suite vectors, the selected public key and still-closed admission fields.
All nine packet validation/mutation cases pass on Windows and WSL Linux.
Run `python verify_packet.py` and `python test_verify_packet.py` here.
`--committed <evidence-SHA>` also checks all packet bytes against Git objects.
No private paths are read by the evidence verifier. Numeric sizes/counts here
describe inventory and correctness tests, not cache performance claims.

Next implement and review actual prebuilt preparation/acquisition under the
selected issuer and supported runtime profile. The controller CLI does not yet
consume this new factory, and a production launcher/key/token handoff is not
implemented by the new getter. Runtime/root/job policy, authenticated replay/
common fence, original worker start, every P2/P3 writer's cessation, completion
cells 2–4 and server/product qualification remain separate and closed.
