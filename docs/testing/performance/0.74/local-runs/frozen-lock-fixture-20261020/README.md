# Synthetic frozen lock fixture evidence

Baseline is 23c57b77523ad1cc16b1a79fc90b02a125f8d9b8. The
[preregistered plan](../../frozen-lock-fixture-plan-20261020.md) repairs only the
test input; the production hash, runtime and admission boundaries remain fixed.
Selected source is 84fee2cce1d139d4361ae5cccc09d147eba357ab. Windows ordinary
package checks and both selected Linux artifact/receipt targets pass. These
results neither build a production executable nor open original admission.

## Original failure and exact input repair

baseline-red.log reproduces the original failed root-lock comparison before
implementation; baseline.json binds its source and nonzero exit. The two new
controls first fail in new-regressions-red.log on the same old helper.
The exact worker-source Git blob is copied into the test fixture, 175300 bytes,
SHA256 be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19.
Its Git blob identity is 59c78f536f3b8ad1d0b13be0f8f1d16b8c892b16.
The -text attribute preserves the original LF bytes on Windows and Linux.

The existing hash assertion remains. Thirty-two seeded corruptions fail the
existing content verifier; the moving workspace lock and even re-signed metadata
with its digest remain rejected. The fixture needs no Git history or network at
test runtime. It is not a workspace lock update or a new production pin.

The first broader Windows invocation then reveals a second unchanged consumer
in diagnostic_receipts. windows-package.log/json preserve its eight failures,
and receipts-baseline-red.log/json reproduce one at the identical baseline file.
Only that consumer's include path is subsequently changed to the same fixture.
All eight targeted Windows receipt controls then pass. Both integration files
reconstruct exactly after reversing only the helper/module hook and this path;
no old assertion, signature, ownership, report or input-bound check is removed.

source-scope.log preserves an initial new guard's unsupported LowerHex formatting
error. The corrected guard uses the existing byte-wise hex convention. Initial
GNU/MSRV lint refusals in gnu194/gnu188 preserve the uneven seed digit grouping;
294f1f57 changes its spelling without changing its numeric value. The initial
recipe is retained as linux-initial.ps1. initial-linux-refusal.json is explicitly
recorded after refusal, not a fabricated before-execution image receipt.

## Selected checks and runner correction

Four source-scope controls preserve all 437 prior runtime files, Cargo.lock,
COMPAT, registry, qualification and the external creator harness. They check the
exact fixture hash/blob and -text behavior, restore both old integration files
byte-for-byte and reject missing/duplicate hooks and unrelated assertion drift.
The previous complete 321-check suite is not rerun; runtime equality and these
four specific controls bound this test-only change. No new runtime module exists.

Both first selected Linux runs actually complete all 31 artifact and nine receipt
tests, then the runner refuses its own wrong eight-case receipt oracle. Those
results remain in gnu194-confirmed/gnu188-confirmed without success receipts.
Their recorded recipe hash matches linux-eight-case-oracle.ps1. The extra case
is Linux-only synthetic spool/offline verification, not a new product test.
This is a harness selection error, not a failed product predicate or a changed
performance threshold. The corrected runner lists each actual image and requires
31 and nine cases before one complete repeat on the same source.

gnu194-final and gnu188-final each pass both targets with zero failed/ignored
tests. Strict all-target/all-feature Clippy passes for both compilers. Their
identities precede execution; individual image hashes, target listings, raw
results and post-execution hash/clean-source checks bind the receipts.

windows-confirmed completes the entire ordinary monitor package: 296 ordinary
tests and twenty doc tests, zero failures or ignored. It retains 79 test-image
hashes and 80 result summaries, including zero-case platform-gated targets.
The source and image set are checked again after completion. This closes the
ordinary Windows package gate on this source, not the full Linux library suite,
workspace-wide testing, live original-frame matrix or release qualification.

Strict Windows monitor/xtask lint, scoped format, registry, docs and the local
non-promotable performance contract pass. Registry discovery stays at 184.
The preceding fixed-slice packet still verifies its 63 hashed files and 22
mutation refusals. Frozen runtime/qualification checks are in frozen-inputs.json;
local-state.json records unchanged active clock service and format success.
No server, supervisor, installation, original send or product workload is used.

## Reproduction

Use a separate checkout at the selected source and copy the final recipes into
the same packet-relative path without retained output directories. Keep compiler
targets separate. Existing output labels are immutable and must not be reused.

```powershell
./windows.ps1 -Label windows-confirmed
./linux.ps1 -Compiler 1.94.0 -Label gnu194-final
./linux.ps1 -Compiler 1.88.0 -Label gnu188-final
./verify.ps1 -SelfTest
```

The verifier checks raw bytes, source/image/recipe bindings, complete target
selection and the preserved failures. Mutation controls must not promote test
fixture acceptance into runtime changes, production trust, fixed-root/artifact
admission, held projection, original delivery, completion or server readiness.
