# Repaired GNU package review and bounded gate

This separate controller reviews the fixed test source
`fe4b3a0f9060615185895337181780a6f3f55439`. It does not replay any consumed
V6–V9 or earlier GNU output. `review.py` checks 391 complete Rust bodies, 58
auxiliary inputs, contextual module closure and all 155 default package targets.
The fixed input SHA256 is
`59f8d00f06e3468626b686201957615fa2b1fbb7bfec4020b63bb863aa48ec8a`.
Eight changed and five added source bodies receive a separate manual review;
378 complete source/AST rows retain the inherited review unchanged. This is
source-only review, not compiler expansion or a test verdict.

The independent local nonqualification checkout is advanced from the verified
previous package source to this source under a new transfer request. Its path
still contains the old source identifier; current HEAD, complete tree and refs
are checked against the new request, not inferred from the directory name.
The old bundle, requests, first receipts and committed evidence remain intact.
Reusing the same native location retains Cargo's existing build context and
avoids both another complete history/evidence copy and a cold-path rebuild.
No qualification/main worktree or server is modified.

The four existing ignored scratch outputs are explicitly checked for ownership,
names and symlinks. Actual Git config, Cargo config ancestry, inherited overrides,
fixed GNU toolchain and executable resolution are checked before and after each
command. Public API `--check` and synthetic provider normalization keep their
original arguments and assertions. Raw WSL PATH need not resolve Cargo: successful
execution of the real memory compatibility test independently checks Cargo's
actual test-child environment. No alias, PATH workaround or script change is
installed to manufacture a pass.

`runner.py` defaults to displaying its contract without execution. Each explicit
mode has one exclusive output and admits no timeout/filter/retry option. Full
mode requires the exact green native controller checks and focused cohort.
It invokes the entire original default package command with 780 seconds including
compilation, four threads and the physical/native 4 GiB floor. CI containment
reuses the exact pidfd/subreaper primitive, separate streams, five-second
retirement budget and kernel ECHILD; this is not production all-writer authority.
The unchanged transcript parser requires complete target/case output.

```text
# Review only, in the existing nonroot local WSL context:
python3 tools/package-repaired-gnu-074/review.py
# Display only, never dispatch:
python3 tools/package-repaired-gnu-074/runner.py --mode full
```

Do not dispatch a consumed mode again. Retain the first outcome if a mode
refuses, fails, reaches its deadline or loses resources. A green focused result
is explicitly not a full package result, and a green full GNU package is not
workspace/security/Windows/release admission. No product, service, clock, server,
installation or qualification authority is added. Detailed results belong in
the [GNU repair report](../../docs/testing/performance/0.74/repaired-gnu-package-result-20261027.md).
