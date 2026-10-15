# Conditional diagnostics and optional example build evidence

This local package closes the three conditional-error Rust1.88 lint failures
left by the [macro cleanup](../msrv-macro-diagnostics-20261015/README.md). A
separate build fix declares an existing example feature requirement. The wider
MSRV tooling gate remains failed; neither change admits a product or server run.

## Source and baseline

The baseline is `2f500651380ce9c130e07c0199a92a6610399820`. Its wider Windows
MSRV lint reproduces three `uninlined_format_args` errors in `conditional.rs`.
The original conditional suite passes six tests with one existing ignored chaos
marker. Three added exact-message tests pass on the old implementation before
changing any `write!`; their diff and full output are retained. They cover all
consistency enum labels as formatter inputs, key counts including `usize::MAX`,
owner absence/presence, Unicode keys, escaped sessions and `u64::MAX`. Synthetic
error values are not claims that strong consistency levels are rejected.

Diagnostic source commit: `ab882d4d3f82063b771a5a1e2f5269ad9873e057`. It changes only
three interpolations in `crates/hydracache/src/grid/conditional.rs` and adds the
three tests in `crates/hydracache/tests/conditional_writes.rs`. Error bytes,
decision branches, consistency rules, serialization and fencing are unchanged.

The initial all-target checks then fail because `memory_compat_driver_071`
imports optional durable types without declaring a required feature. This is
retained separately from the lint failures. Build source commit
`d0ab5c4986dfe3a33826bc6812557592b60ce466` adds only the four-line Cargo example
declaration with `required-features = ["durable-value-store"]`. Cargo metadata
before/after confirms that declaration and unchanged empty default features.
The example source and the compatibility script/workload are not modified.

The final source tree is `3c158e384633faffd68be4b31160ee44a34219b5`. Initial checks run
before these commits on the corresponding changed files; `post-*` logs identify
the separate repetitions after both source commits. Documentation/evidence is
dirty during checks; this is not signed or clean-checkout execution attestation.

## Focused checks

Each Windows/Linux and Rust1.88/1.94 lane passes 40 focused tests: conditional
writes, conditional tombstones, lock leases, reentrancy and consistency levels.
One original ignored chaos marker stays ignored in every lane. Separate
post-commit Windows MSRV and Linux current repetitions also pass 40 each.

All four lanes pass the package default all-target check and strict Clippy.
All four also check and strictly lint the explicitly selected durable-enabled
example. Without that feature an explicit example request is refused by Cargo
with exit101 and the required-feature message. Default all-target checks skip
that incompatible target through standard Cargo metadata; they are not evidence
that the durable example compiled. The paired durable lane supplies that proof.
The example is compiled/linted, never executed. Allocator-specific or exhaustive
feature-matrix coverage is not claimed. As documented in `docs/TESTING.md`,
HydraCache's mutually exclusive allocator features preclude blanket
`--all-features` for this package.

The missing Linux Rust1.88 Clippy component is installed through the existing
rustup and its version/components are retained. Focused library/test Clippy
passes on both platforms/toolchains. Windows package formatting and Linux
whole-workspace formatting pass. Wider Rust1.94 supervisor/xtask strict Clippy
passes on Windows and Linux after both source commits.

The 165 performance-contract tests and 175-gate registry check pass. The local
0.74 performance contract passes as non-promotable; `--require-ship` still
refuses with exit1 because candidate identity and qualification are incomplete.
These are local contract checks, not a product workload or a full release gate.
The release documentation consistency check also passes.

## Remaining strict MSRV failures

The wider Windows strict invocation fails next on loadgen format expressions;
the Linux strict invocation first fails on the root-handoff diagnostic branch.
Separate warning-mode inventories continue far enough to enumerate all eight
unique source locations across these selected packages/targets:

- `crates/hydracache-loadgen/src/metrics_honesty.rs:2543`
- `crates/hydracache-loadgen/src/resp_external.rs:2055`
- `crates/hydracache-loadgen/src/tiers/client_surface.rs:718`
- `crates/xtask/src/canary_check.rs:423` and `:429`
- `crates/xtask/src/fast_suite.rs:106`
- `crates/xtask/tests/perf_bootstrap_0671.rs:61`
- Linux-only `tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_handoff.rs:364`

Windows observes the seven format locations; Linux also observes the nested
else/if. Duplicate lib/lib-test diagnostics are counted once. An inventory's
exit0 is non-gating: it omits `-D warnings` only to reveal downstream warnings.
The strict gates retain `-D warnings` and their failures are preserved. No
repository lint configuration or threshold is weakened. This is a complete
observed inventory for these invocations, not every workspace/feature/platform.

## Evidence scope

[manifest.json](manifest.json) records commands, statuses, capture hashes,
source blobs/tree and frozen inputs. Captures preserve tool-returned text and
ANSI controls, normalized to UTF-8/LF with one final newline for nonempty logs;
empty logs remain empty. Unsigned digest consistency does not attest execution.
Earlier packets and failed outputs remain unchanged.
The separate `verification.txt`, outside the capture manifest, records the local
consistency check of all 52 captures, source blobs/tree/parent chain and frozen
hashes. It explicitly preserves the failed strict MSRV status and non-gating
classification of the inventories.

Cargo.lock and the 0.74 qualification manifest retain their prior SHA-256 values.
No performance measurements, product workloads, qualification, server/workflow
execution, merge/tag or push occur here. Trusted runtime/issuer/job custody,
authenticated original-start, P2/P3 cessation of every writer, completion/later
cells and server admission remain open. The next code-quality package should
close the enumerated MSRV sites with separate diagnostic/trace-control evidence.
