# Native inclusive CPU in real short Git calls

Short ordinary Git calls now have a birth-to-terminal descendant CPU diagnostic,
without stdin holds, native-image bypass or Git suspension. One changed-source
control completes four reference and four accounted `git show` calls with exact
independent body bytes, empty stderr and exit zero. This establishes accounting
coverage for that control, not an optimization or a complete helper attribution.
V6 is not repeated and its timeout verdict stays unchanged.

## Implementation and preserved behavior

The [design](../../../../tools/xtask-package-attribution-074/git-job-cpu-design.md)
and [protocol](../../../../tools/xtask-package-attribution-074/git-job-cpu-protocol.toml)
fix the ordinary command, independent body, finite schedule and 120/5-second
run/cleanup windows. The
[Rust diagnostic worker](../../../../crates/xtask/tests/git_descendant_cpu_074.rs)
uses the original PATH, repository cwd, environment, `git show SPEC` argv and
unchanged output/pipe helper. Its exact CLI gate prevents ordinary package tests
or listing from activating diagnostic Git calls. No source-reader integration,
cache, batch, extra Git flags, native invocation or fallback is installed.

The [accounting collector](../../../../tools/xtask-package-attribution-074/git_job_cpu.py)
creates a private unlimited Windows Job. The existing CI primitive places only
the diagnostic worker in it before execution; it is unchanged. The new subclass
does not enable its CI kill-on-close policy. The worker's ordinary Git children
inherit membership from birth, avoiding late discovery of short-lived children.
Job membership is an explicit observer delta, not invisible instrumentation or
authenticated production ownership. Healthy completion never terminates a Job.

Original worker identity is checked before its existing single resume. Terminal
collection uses that same held handle, checks PID/creation continuity and
monotonic counters, and requires two equal terminal Job snapshots with exactly
nine lifetime members and no active process. Subtract whole worker and four
whole launcher lifetimes separately from Job user/kernel CPU. Underflow, changed
identity, incomplete census or unstable counters refuse without clamping.

## First outcomes and fault controls

Instrumentation/protocol commit `e3f560e69a8d1deda20999a0b1c9b7e31a75f11d`
precedes the first actual control. Four reference calls pass their byte/status
checks, then `QueryFullProcessImageNameW` on the terminal worker returns Win32
error 31. The accounted worker never starts, cleanup has no errors and that
attempt supplies no accepted CPU result.

Separate fix `bd1b3cf2e6fc4a252037caf69603b0df6571aab4` moves that path query
to the existing pre-resume retained-child publication. It preserves the original
handle through terminal PID/creation checks rather than dropping identity or
reopening a PID. No CI creation code, Git call or time window changes. One
separate control on this committed changed source succeeds. The original
refusal, source and logs remain alongside it; no unchanged retry occurs.

Twenty accounting/ownership models and sixty unchanged process/lineage models
pass. Controls cover wrong census, live descendants, unstable Job counters,
missing or duplicate launchers, underflow, changed identity, incomplete protocol,
pre-resume query failure retaining custody, forbidden owner replacement, partial
spawn cleanup, close/terminate/wait failures and late cleanup witnesses. Both
compilers pass the new four-case target, including two inactive diagnostic child
entries under ordinary argv. Strict all-target/all-feature xtask Clippy, Rust
formatting and Python syntax/indentation checks pass.

The old five-case pipe target returns four passes and one failure on both
Rust 1.94 and MSRV 1.88 before the new target is reached in the combined command.
Its large-output assertion compares the complete libtest stdout, including
printed elapsed time. Offline replay of both first failures finds equal stream
lengths of 1,048,719 bytes and exactly one differing byte at offset 1,048,715:
`0.01s` versus `0.00s`, reversed between compilers. No payload-byte difference is
present in those saved stdout values. The old test/source and both failures stay
unchanged, without rerun or a claim that this compatibility gate is green.
The new target is checked separately; it does not replace those verdicts.

## Terminal CPU accounting

The successful accounted worker records these whole-lifetime values, in ms:

| Scope over four accounted calls | User CPU | Kernel CPU | Total CPU |
| --- | ---: | ---: | ---: |
| All nine Job members | 187.500 | 1625.000 | 1812.500 |
| Original diagnostic worker | 0.000 | 796.875 | 796.875 |
| Four original direct Git launchers | 78.125 | 718.750 | 796.875 |
| Non-launcher descendants after subtraction | 109.375 | 109.375 | 218.750 |

The residual covers native descendants from birth, including work that a late
attachment could miss. Job totals alone do not attest each descendant's mapped
image or assign native CPU per call. Independent cmd/native file hashes match
before/after; that is on-disk pinning, not mapped-image attestation or ABA
protection. The observed counter values also retain their OS granularity; a
100-ns representation is not a claim of 100-ns measurement precision.

This one control is neither an overhead cohort nor a timing comparison. Its
numbers cannot be applied to V6, the old 96-call cohort, startup alone, pipe wait
or throughput. Worker/reference time is not an accepted observer cost estimate.
The next attribution needs observer-overhead separation before selecting a
cost-reduction candidate. The old pipe-fixture's elapsed-output comparison needs
a separate test-boundary repair with useful bytes/status assertions retained.

## Evidence and unchanged release gates

The [sealed 97-member packet](local-runs/git-job-cpu-20261026/README.md) includes
all first outcomes and exact source copies. Its offline verifier recomputes CPU
residuals and the two elapsed-only fixture differences without launching Git or
importing retained collection code. Four integrity controls cover index, missing
objects and corruption of counters, raw logs, first failures and source.
Index SHA256:
`c12ee4c7515ce6cd94744b7ce1754e172bbf7b2c7eba8f82893e4a3ab5defa41`.
The packet's separate final-checks index retains passing doc-check, local
non-promotable performance-contract, verifier syntax and four integrity checks;
it does not alter the native evidence index or supply package admission.

All live predicates, observation frequency, 300/60/10 origins, 780-second package
guard and first results are preserved. No V6 retry, server action, installation,
service/clock change, dependency update or product workload occurs. The
[release status](release-closure-status-20261025.md) remains unresolved: no C74
freeze, accepted performance candidate or production admission. Qualification
manifest SHA256 stays
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
