# Census caller integration with independent static references

The real census test-source caller now uses the bounded, independently pinned
reader and legacy panic facade. Its allowlist, public signatures and conditional
Git read order are preserved. **No reference-preparation Git commands run inside
the guard.** Windows and local GNU/WSL controls pass on Rust 1.94 and MSRV 1.88.
This is a source-integrity integration, not batching or a measured speedup.

Code commit: `ae508a57220fe95b11887b81f16ff05263f1dee9`.
The [evidence index](local-runs/historical-blob-census-integration-20261024/index.json)
retains captured candidates, first refusals, exact sources, native inputs/outputs
and local gates. Earlier [Linux/panic](historical-blob-census-linux-panic-result-20261024.md)
and [environment-contract](historical-blob-census-contract-result-20261024.md)
results remain historical evidence, not rewritten passes for this integration.

## Runtime boundary

`crates/xtask/tests/support/census_provider_input.rs` retains the original
two-revision assertion and its `original`, `accepts` and `view` signatures. Only
historical body loading delegates to `census_caller::read` and `panic_api::frozen`.
Inherited environment, explicit replacement disabling, successful-stderr handling,
status-before-UTF8 ordering and CRLF-only normalization remain. The helper is
still read and checked at its original point; body predicates and old rejection
messages remain.

An unrelated view returns before deadline creation or source reads. A selected
view creates one 30-second deadline, checks the current helper, reads BASE and
then REVIEWED, and runs the body contract only after exact normalized equality.
Both historical reads receive that same deadline without renewal. Standalone
original/acceptance calls each have their own scope. Original 300/60/10 windows,
observation frequency/order, the old 780-second package bound, 16 MiB/64 KiB caps
and five-second cleanup remain. Additional pin/budget/deadline refusals stay
explicit; arbitrary old unbounded execution is not declared equivalent.

`census_reference.rs` contains two fixed blob metadata rows. BASE is 34850 bytes
at object `2ca72f8386735580ee4cd82ced9a1b07648c9df0`, bound to the existing
contract SHA256. REVIEWED is 35108 bytes at object
`e4111fdbbb90e0c06a271617da8b3781aa25d58e`, bound to the earlier inventory SHA256.
The factory refuses other revisions or paths. It performs no Git, archive or
current-file reads; observed output cannot refresh a row. `mod.rs` shares the
checked Rust components without a new crate or dependency change.

The real consumer adds only this support namespace and a signature-only test.
The test takes function pointers without calling them or reading Git. Its
exact-source regression still compares the entire original consumer after
precisely enumerated additive changes; no original assertion is removed.

## Independent provenance and comparison

`caller_integration.py` verifies the pre-integration sealed contract packet at
SHA256 `b20b3b825033dca9cb0313da64d0dbfc67c7879c9a29303980615e3510ab50fa`.
Its previously recorded producer result supplies metadata already bound to the
independent BASE contract and REVIEWED inventory anchors. The entire current
descriptor must match a fixed source template built from those rows. The caller
must match an exact rewrite of its earlier archived source; unknown allowlist,
predicate, deadline, extra-effect or reference edits refuse before diagnostic
Git preparation.

Legacy comparison preserves the archived function bodies. Its sole import
rewrite re-exports the unchanged, whole-file-pinned contract instead of compiling
it twice. That import delta is audited explicitly; no legacy check or command
changes. Five profiles compare legacy, typed adapter, panic facade and connected
caller: clean, trace, valid config, malformed config and invalid repository.
Each exercises five operations. Trace2 verifies identical literal argv and
reached read order; panic text/type match. Healthy original/acceptance/view/
unrelated/mismatched-acceptance calls perform 1/1/2/0/1 Git reads, with identical
early stops on failure. There is no eager batch, cache, fallback or retry.
Producer Git reads remain diagnostic-only. The strict-batch counterexample is
a separately scoped frozen-legacy canary, not the connected-caller verdict.

## First results and final gates

The first Windows candidate passes consumer/caller cases but its exact-source
regression refuses: the expected import was prepended whereas rustfmt placed it
after the build-guard import. The corrected expectation names that exact insertion
and retains full-file equality. The first strict Clippy run refuses duplicate
contract-module loading; the audited re-export removes it without a lint exception.
MSRV initially warns about standalone APIs after view stops renewing their
deadlines; the zero-read signature test preserves their surface and removes the
warnings. All first sources/results remain retained.

Final Windows cohorts pass 37 cases each: four consumer, nine decoder, eleven
caller, five fault, six transport and two reader regressions. GNU/WSL passes
33 cases per toolchain in the five-target fixture: it exercises the actual caller
and checks the complete consumer source as data, but does not compile/run the
four consumer cases themselves. Both native toolchains preserve the independent
six-case/54-result corpus. Forty-five Python controls, current all-target check,
all-target/all-feature strict Clippy, formatting, Python syntax/indentation,
doc-check and local non-promotable performance-contract-check pass.

Four native source/receipt candidates are retained. Each records stable boot and
input identities, no remaining owned groups and removal of only its temporary
root. Final receipts mark the connected caller; earlier prototype flags remain
unchanged. WSL clock disagreement is not original clock authority or host qualification.

## Remaining gates

Next replay the pending complete package within its original bound and attribute
the full old/new caller cost, including hashing, reference construction and
file-backed capture. Unchanged Git count supplies no process-amortization claim;
batching requires a genuinely preplanned unconditional site or separately approved
semantics. The old incomplete package/release gates remain open. Production
trust/enrollment, original delivery and all-writer retirement do not follow from
this local test-source integration.

Cargo manifests/lockfile, dependency policy, qualification manifest and frozen
0.73 artifacts remain unchanged. No clocks, services, server, product workload,
original-owner execution, installation, qualification or push occurs.
