# Linux census caller and legacy panic compatibility

The isolated census adapter and its new panic facade pass the supported local
Linux and Windows controls on Rust 1.94 and MSRV 1.88. Old process/helper/provider
messages and actual UTF8/IO unwrap payloads are preserved without changing the
conditional read schedule. **The real caller is not switched.** The bounded
adapter's additional pin, budget, deadline and cleanup refusals remain explicit.

Code commit: `fc15b0c5d25cbba08e768f47c843f8b5048cc45f`.
The [evidence index](local-runs/historical-blob-census-linux-panic-20261024/index.json)
retains all attempts, exact input sources, native receipts, first refusals and
local gates. The preceding [environment contract](historical-blob-census-contract-result-20261024.md)
and its first invalid-original failure remain unchanged historical results.

## Lossless errors and the panic facade

`transport::run_owned_with_io` preserves the original `std::io::Error` beside
the existing refusal. The old `run_owned` entry point still returns exactly
`Refusal`; no deadline check, stream cap, polling order or cleanup effect moves.
The census adapter retains the IO cause instead of reconstructing it from a
Display string. Batch stderr admission and fallback output/ABA validation remain
strict, and the original successful-stderr counterexample is still exercised.

`census_caller::panic_api` unwraps original IO and UTF8 results through std's own
panic boundary. This preserves the Debug payload rather than formatting a guessed
replacement. Process failure remains `missing exact material fixture source`;
helper mismatch remains `unreviewed fixture control`; body rejection remains
`unreviewed provider body`. Status still precedes UTF8 and normalization changes
only CRLF. New adapter refusals panic explicitly as `census adapter refusal`,
without retry, fallback or conversion into an old successful result.

The lazy facade uses the same helper, BASE, REVIEWED and body predicates as the
typed adapter. Helper/input errors stop before the next read; unrelated views
read nothing; mismatched acceptance does not execute the body predicate. Pure
controls cover each boundary, invalid UTF8, absent/nonzero status, original OS
errors, actual invalid-current-directory spawning and explicit new refusals.

Five real-Git profiles compare the old caller, typed adapter and panic facade:
clean, trace diagnostics, valid config, malformed config and invalid repository
override. Each profile exercises five operations. Trace2 confirms identical
literal argv and reached read order for all three paths; caught panic text and
payload type match the old caller. This establishes finite-profile compatibility,
not identical backtrace locations or arbitrary environment equivalence.

## Local Linux fixture and first refusals

Only the already-running Ubuntu WSL is used, as UID 1000 on kernel 6.6.87.2.
The stopped diagnostic distro is not started. One private temporary fixture per
attempt receives exact source bytes, the old root lock and sixteen existing Git
objects: the two commits, required path trees and blobs. The new repository is
initialized only inside that temporary root; no mounted real Git directory is
used or changed. The existing caller and pin producer run unmodified against it.
The producer verifies its earlier sealed anchors; its Git reads are diagnostic
preparation, not new authorized reads inside a real guard.

The separate fixture manifest pins only existing root-lock versions, enables the
serde/std context needed by xtask, and disables automatic discovery of a copied
source file used as data. Offline metadata resolves 33 package identities, each
checked by version/source/checksum against the existing lock. No dependency is
downloaded, installed or updated in the workspace baseline.

The first native build refuses because the initial minimal fixture omitted
serde/std and therefore cannot serialize `Result` in its receipt. The corrected
manifest uses the same existing serde version. The second attempt compiles but
the old fault target refuses `Environment`: the common fixture injected
`GIT_CONFIG_*` variables forbidden by its original preflight. The correction
removes those common overrides, not the guard; named caller overrides remain
child-local. Both first refusals and their inputs are retained.

The third native attempt passes all tests. The first strict Clippy run then
refuses literal `unwrap(Err(...))` expressions used to preserve the old panic.
A shared generic legacy unwrap boundary removes that lint issue without an
allow/expect exception or manual panic-text reconstruction. The fourth native
attempt tests that final Rust source and passes. These are distinct source/fixture
candidates; no unchanged failed candidate is retried until it passes.

All four native receipts record unchanged boot and input-source identities,
no active owned process groups and removal of only their own temporary roots.
Raw clocks remain in the evidence. The existing WSL MONOTONIC/RAW disagreement
is not waived: passing byte/process/API controls does not admit original clock
authority or qualify a production host.

## Gates and remaining integration work

Windows and GNU/WSL each pass 32 cases on both toolchains: nine decoder, five
fault, six transport, ten caller and two old census regressions. Both native
toolchains retain the independent six-profile/54-result corpus. Five caller
profiles compare 25 operation triples per platform/toolchain. Forty Python
controls, current all-target check, all-target/all-feature strict Clippy, focused
rustfmt, Python compilation/tabnanny, doc-check and the local non-promotable
performance-contract-check pass. These are correctness counts, not timing claims.

The Linux finite-profile and old-panic compatibility gates are green. Integration
must still preserve the old two-revision allowlist and exact allowed source delta,
consume independent anchors without adding producer Git reads to the guard, and
admit the bounded adapter's extra refusals explicitly. No eager batching, cache,
new source authority or measured process-amortization gain is established here.
The old caller and its provenance checker remain unchanged; production enrollment,
original delivery/writer retirement and the incomplete package/release gates are
not advanced by this test-only facade.

All original live-check order/frequency and 300/60/10 windows, the old 780-second
package bound, the reader's shared 30-second bound, 16 MiB/64 KiB stream caps and
five-second cleanup bound remain. The native wrapper keeps its 300-second build,
90-second test and 1100-second outer bounds; a 1000-second census session cutoff
can only shorten a phase to leave cleanup/export time. None of the completed
native phases needs that shortening. Cargo manifests/lockfile, deny policy,
qualification manifest and frozen 0.73 artifacts are unchanged. No clocks,
services, server, product workload, original-owner execution, installation,
qualification or push occurs.

The subsequent [caller integration](historical-blob-census-integration-result-20261024.md)
connects the narrow source reader with static references and no extra guard Git
reads. This earlier pre-integration result and its first refusals remain;
production and performance admission are separate.
