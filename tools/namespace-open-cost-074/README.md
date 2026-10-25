# Namespace fixed open experiment

This test-only candidate replaces the conversion performed by `OpenOptions`
for the four fixed Namespace paths with GNU `open64` on terminated literals.
The original path projection still executes once; unknown names retain the
original `OpenOptions` fallback. No validation result, FD or kernel identity is
cached. Production `namespace()` is unchanged at the baseline stage.

The pinned Rust 1.94 GNU implementation uses `open64` with atomic `O_CLOEXEC`
and repeats an interrupted open. The candidate preserves those semantics,
including immediate errno capture and the unused mode argument. See the
[pinned filesystem source](https://github.com/rust-lang/rust/blob/1.94.0/library/std/src/sys/fs/unix.rs)
and [Unix error conversion](https://github.com/rust-lang/rust/blob/1.94.0/library/std/src/sys/pal/unix/mod.rs).

## Controls and measurement

The four ordinary tests in `diagnostic_namespace_open_tests.rs` compare FD
identity and flags, missing/forbidden/non-directory paths, symlinks, dynamic
fallback and invalid inputs. Scripted interruptions cover the first success and
terminal errno. A real FIFO control directs SIGUSR1 only at the blocked test
thread in its own child process, observes re-entry into open, then releases the
FIFO. It does not signal existing processes or change services.

`protocol.json` freezes the whole-Namespace experiment before measurement:
eight fresh processes, CPU 5, UID 1000, four cases, four balanced arms, sixteen
samples and 1024 calls per sample. Each invocation freshly opens the namespace,
checks nsfs and FD identity, and retires the FD. The operation observer is
inactive; its overhead is not subtracted. The reference is the baseline whole
method, not a standalone syscall loop.

Both reference controls must have process-bootstrap 95% intervals inside
0.97–1.03. The candidate/reference upper bound must be at most 0.98 for **all
four** cases. A negative first cohort ends this candidate without integration
or retry-to-pass. A positive prototype requires a separate runtime change,
strict package gates, and the installed-image cohort before a new candidate's
full preparation may be attempted. Neither microbenchmark grants admission.

Run the pure reader controls locally with:

```powershell
C:/Python313/python.exe tools/namespace-open-cost-074/test_review.py
```

The manual Rust selectors are registered as non-shipping gates. Invocation
requires the protected-source/image coordinator and the phase's exact test
selector; do not run them as an unbounded preparation retry. The 300/60/10 live
windows, original checks and first preparation refusals are unchanged.

The [first result](../../docs/testing/performance/0.74/namespace-open-cost-result-20261025.md)
is negative: all four baseline calibrations pass, but no Namespace case meets
the frozen 2% floor. The candidate stays test-only and is not integrated.
