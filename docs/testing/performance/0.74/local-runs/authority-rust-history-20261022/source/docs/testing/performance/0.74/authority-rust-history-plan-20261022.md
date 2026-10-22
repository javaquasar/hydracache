# Historical Rust guard IO separation plan

Keep historical source assertions separate from admission of current code.
The prior owned-storage artifact is guarded but unapplied. This step changes
test IO only, not supervisor/runtime, checks, deadlines or qualification.
Inherits RULES R-3, R-5, R-7, R-8 and R-10.

## Historical IO and complete assertion preservation

Use exact commit `33c54b0cda8c330b115009cdf092ad002077cedf` for the existing
thirty source-canary binaries. Some inspect the fixture directly; others compare
the complete src/tests tree. Adapt both reads and Git differences explicitly.
Do not filter unexpected changed paths out of an assertion to make it pass.

The only allowed edits to each original guard are one module declaration,
replacement of std::fs::read_to_string by the historical reader, and replacement
of Git Command construction by the historical command adapter. Derive expected
whole files from the frozen originals with exact replacement counts, then
compare complete token streams. Preserve all test names, assertions, mutation
controls, normalization functions and support files. No ignored tests or wrappers
that skip bodies. Git show/ls-tree retain their original operands; diff gains
only the fixed historical right-hand revision before its original path separator.
Reject unsupported commands, argument shapes, unsafe paths and alternate cwd;
never fall back to working bytes or a mutable HEAD.

## Independent current tree gate

Historical success cannot admit current drift. Separately check every modified
pre-existing tools/crates source against the same baseline. Permit only the
exact thirty IO substitutions. Reject changed prior model inputs, normalization
support, runtime files, manifests, contract TOML and qualification. Deny new
tracked or untracked runtime src/tests files. Newly added unrelated tooling is
not a historical assertion input; the prior owned-storage source controls still
check the complete candidate and observer artifacts independently.

Runtime fixture and observer remain baseline-only at this stage. Mutation tests
must show historical results remain fixed while a changed current fixture,
observer or unrelated source is rejected by the independent gate. The current
owned-storage allowlist is not replaced by historical success.

## Cohort and retained evidence

Run the migrated thirty binaries, new IO/mutation/current-tree controls, the
eight existing candidate-source controls, format/check/strict clippy and local
documentation/registry/non-promotable performance contracts. Verify the frozen
Python reader view and receipt controls. Old raw Python audit entry points still
pin original guard bytes: run their historical reader view, not those scripts
against newly adapted current files, and do not edit their pins or old packets.
This is not a new execution of their complete old unit-control cohort.

Freeze the method commit before the first no-clobber captured cohort. Retain raw
logs, exact source snapshots and inventory, including first failures separately.
No old measured model, live/fault/original-owner image, WSL, server, installation,
supervisor, product workload, clock or service action. Keep 300/60/10 windows,
first Causal refusals 10042390247/10010835739 ns and qualification digest unchanged.
After this gate, minimal runtime integration and Linux filesystem/typechecking
remain separate work; no server admission follows from historical source checks.
