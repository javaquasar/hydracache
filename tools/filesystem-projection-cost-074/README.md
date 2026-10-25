# Fresh filesystem output projection experiment

This test-only candidate removes the output-buffer zeroing before `fstatfs`.
It retains one fresh libc query, the original nonzero-return error handling,
no EINTR retry, and exactly the original filesystem-type comparison. It allocates
the full aligned writable `statfs` buffer but reads only `f_type` after success;
it never assumes that the entire object is initialized. No result is cached.
Production `filesystem()` is unchanged at the baseline stage.

The [Linux interface](https://man7.org/linux/man-pages/man2/statfs.2.html)
defines filesystem type in the output. The
[Linux 6.8 native conversion](https://github.com/torvalds/linux/blob/v6.8/fs/statfs.c)
copies that field into the user buffer before returning success. The candidate
is restricted to GNU x86_64 tests; this is not an ABI migration or a syscall
replacement. Failed calls never permit reading the output.

## Frozen controls and comparison

Three ordinary Rust tests cover real proc/nsfs/files/devices/sockets and a bad
FD, unchanged FD state, 2048 seeded success/mismatch cases initializing only
the output field, ten nonzero-return/errno combinations without output writes,
and full Boot/Namespace values, refusals and observation counts. They do not
exercise original delivery or the all-writer lifecycle matrix.

`protocol.json` freezes seven cases: proc/nsfs leaf checks, full Boot and four
whole Namespace paths. Eight fresh UID 1000 processes run on CPU 5, with four
balanced arms, sixteen samples and 1024 calls per sample. Every Boot invocation
retains four fresh opens/filesystem checks, read, validation and descriptor
retirement; every Namespace invocation retains one open/filesystem/identity
observation. Expected identities are setup inputs, not extra timed queries.
The previous rejected Namespace-open candidate is not used.

All baseline-control intervals must fit 0.97–1.03. Candidate/reference CPU upper
bounds must be at most 0.98 for all seven cases. The process-level estimator and
fixed-seed bootstrap are shared with the preceding independent experiments.
The operation observer is inactive, and nothing is subtracted or filtered.

A negative first cohort ends this candidate without integration or retry-to-pass.
A positive prototype still needs separate runtime integration, focused fault/check/
strict-lint gates and a separately verified installed-image cohort. Original live
frequency, order, 300/60/10 windows and first preparation refusals stay fixed.

Pure reader controls:

```powershell
C:/Python313/python.exe tools/filesystem-projection-cost-074/test_review.py
```

Both Rust measurement selectors are manual non-shipping gates. Use the exact
protected source/image coordinator, not an unbounded preparation retry.

The [first result](../../docs/testing/performance/0.74/filesystem-projection-cost-result-20261025.md)
rejects this candidate: all calibration controls pass, but no case meets the
frozen 2% floor. Codegen confirms the zero stores disappeared without the
required CPU benefit. The change stays test-only.
