# Census caller environment and error contract

The isolated census adapter preserves the old successful-stderr interpretation
and conditional read schedule in five local Windows profiles on Rust 1.94 and
1.88. **It is not connected to the real caller or the native batch reader.**
The bounded adapter has additional explicit refusal conditions; it is not an
equivalent replacement for the old unbounded, panic-based API in every environment.

Code commit: `a9095e56fb052dff2cab169bf6195b0eb7884d6f`.
The [evidence index](local-runs/historical-blob-census-contract-20261024/index.json)
retains the first failed fixture, corrected source, raw child output, Git Trace2
and check receipts. The [earlier caller and Linux audit](historical-blob-caller-linux-result-20261024.md)
remains the authority for the two test-source anchors and the batch/stderr
counterexample; neither its source nor its verdict is rewritten.

## Environment and output interpretation

`tools/historical-blob-batch-074/census_caller.rs` issues one literal
`git --no-replace-objects show <revision>:<path>` at each reached read. It inherits
PATH, Git environment and system/global/local configuration at spawn, like the
old command. It neither filters environment variables nor switches replacement
policy. There is no native attempt followed by an old-loader retry, no new
fallback and no prefetch. The actual caller's two-revision allowlist stays in
the unchanged original implementation; a constructed reference is not authority
to replace that allowlist or enroll a new source.

The adapter checks status before interpreting UTF8, then normalizes only CRLF.
Successful stderr remains available as raw bytes and does not invalidate census
stdout. Nonzero or absent status is `Process`, even with invalid stdout; successful
invalid UTF8 is `Utf8`. `Failure` retains completed command stdout/stderr/status.
Spawn, budget, deadline and cleanup refusals remain distinct `Transport` errors.
An unavailable completed output is `None`, not an invented empty capture.

An independent raw blob reference binds the literal query, expected byte count
and SHA256 before execution. Query swaps refuse before Git; body drift refuses
as `Content`, retaining the observed output without refreshing the pin. The
producer still verifies the earlier sealed source anchors before preparing pins.
The generic transport and decoder are unchanged, including strict batch stderr
admission, fallback output bindings and ABA controls.

The inherited environment is a compatibility boundary, not a hardened production
environment or a local-only Git policy. In these disposable child controls,
Git overrides are first cleared and each named profile is installed only in the
child; both old and candidate commands inherit `GIT_NO_LAZY_FETCH=1` and Trace2.
No repository, global configuration or parent environment is changed. The adapter
itself does not inject those test restrictions. Arbitrary inherited hooks,
helpers, network behavior or diagnostic destinations are not admitted.

## Conditional reads and errors

The lazy `Inputs` interface preserves the schedule rather than constructing a
two-object batch:

- An unrelated view returns immediately without helper or historical reads.
- The selected view reads the current helper, checks it, reads BASE, then reads
  REVIEWED. The body contract runs only after exact normalized REVIEWED equality.
- Standalone acceptance reads only REVIEWED before the equality/body decision.
- A helper, BASE or REVIEWED failure stops at that point; no later input is read.

Injected controls cover every stop and body mismatch. Real-caller comparisons
capture exact Git argv/order with Trace2 for `original`, `accepts`, selected view,
unrelated view and mismatched acceptance. The original helper/hash/body checks
are reused, not replaced by the adapter's byte pins. The actual old source and
its existing regression tests remain unchanged.

Five profiles cover clean reads, successful `GIT_TRACE=1` stderr, a valid
`GIT_CONFIG_COUNT` override, malformed config, and an invalid `GIT_DIR`.
In the two failure profiles the old panic retains its original message and the
candidate reports the matching process-failure class. This is an explicit
comparison of outcomes, not proof that typed `Result` errors are the old panic
API. A future switch must preserve that boundary deliberately.

The shared diagnostic bound stays at 30 seconds without renewal between reached
reads, stdout/stderr caps stay at 16 MiB/64 KiB, and owned-child cleanup stays at
five seconds. File-backed capture differs from the old piped/unbounded command;
deadline/budget/pin failures are additional refusals. No original 300/60/10-second
window, live check/frequency/order or old 780-second package bound changes.

## First result and checks

The first version passes seven caller cases and fails its new clean profile.
Its fixture supplied `"unused"` as the original source to a contract that requires
the canonical BASE body. The first source and four successful Git start records
locate the failure at acceptance; its overly broad child panic hook did not
capture the inner panic payload. That missing payload is not reconstructed as
evidence. The separate corrected fixture uses the independently pinned BASE
input and restores the hook immediately after each expected old-caller panic.
No reader validation or deadline is changed to obtain the later pass.

The corrected target passes eight cases on each toolchain, including its two
child entries and the retained generic-batch tracing counterexample. Both old
role-census regression cases pass on each toolchain. Current all-target check,
all-target/all-feature strict Clippy, focused rustfmt and doc-check pass.
The five profiles compare 25 operation pairs per toolchain; this is semantic
coverage, not a performance measurement.

## Remaining admission boundaries

This closes the isolated finite-profile environment/error and lazy-schedule
contract gate. It does not admit production pin authority, Linux behavior of
this new caller adapter, arbitrary environment equivalence, a panic-API switch,
batched reads or a speedup. Next prove the supported Linux caller boundary and
the intended error facade before connecting any existing source guard. Process
amortization still requires a separately justified observation schedule; this
adapter intentionally retains the old number of Git reads.

Cargo manifests/lockfile, dependency policy, qualification manifest and frozen
0.73 artifacts remain unchanged. The old incomplete full-package and release
gates remain open. No clocks, services, server, original-owner execution,
product workload, qualification, installation or push occurs.
