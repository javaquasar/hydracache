# Pipe compatibility repaired and observer cost still unresolved

The pipe compatibility test now passes on current/MSRV without comparing the
variable libtest elapsed-time footer. A separate fixed-Job attribution completes
all sixty-four ordinary Git calls, but its mixed CPU/wall differences do not
justify accepting small observer overhead. Both previous first fixture failures
remain immutable, V6 is not repeated, and no product optimization is admitted.

## Exact stream assertions without a variable footer

The [previous first results](git-job-cpu-result-20261026.md) fail on both
compilers only because libtest prints `0.00s` in one child and `0.01s` in the
other. Their 1,048,719-byte stdout values differ at one byte, after the useful
payload. The old source and both complete first logs are retained again by
digest; they are not overwritten, normalized or rerun.

Commit `11557ac94c412226b5e13af2ce9713298800e079` changes only the synthetic
child/test in
[`direct_output_cost_074.rs`](../../../../crates/xtask/tests/direct_output_cost_074.rs).
Successful fixture modes explicitly flush stdout/stderr and exit zero before
libtest appends the footer. Nonzero mode keeps exit seven and original binary
payloads. The fixed harness prefix remains part of the exact expected stdout;
the empty mode means empty useful payload, not literally zero raw stdout bytes.

Original status/stdout/stderr equality assertions, stdin EOF, one-MiB stream
checks, retained-process identity and missing-program error assertions stay.
An additional independent oracle checks exact expected prefix/payload/stderr and
exit code for both original and observed paths. Its pure regression rejects a
changed byte, extra stderr, wrong/missing status, unknown mode and either elapsed
footer; it never strips captured data. The original pipe helper and Git worker
remain byte-identical. The two first changed-source target checks pass 6/6 each
on Rust 1.94 and MSRV 1.88. All-target check, strict all-target/all-feature xtask
Clippy and formatting also pass. The old 96-call timing cohort is not activated.

## Fixed Job attribution method

Commit `20247dd65f821aff33946b9643032c7773f0b146` precedes the sole actual
attribution. Its [protocol and design](../../../../tools/xtask-package-attribution-074/git-cpu-overhead-design.md)
fix four alternating ABBA/BAAB blocks of the unchanged reference/profile worker,
with four fresh `git show` calls each: sixteen workers, sixty-four calls total.
Both paths inherit the same unlimited private accounting Job and unchanged
environment, argv, cwd, null stdin and separate pipes. The independent body and
Git image pins remain fixed.

Job membership and pre/post worker observations are common to both arms. The
difference includes the installed per-call profile path and phase serialization,
not just two counter API calls. It excludes any estimate of the Job's own cost.
Whole-lifetime Job CPU includes native descendants; worker and external
coordinator CPU stay separate. Reference has no direct-launcher counters, so
no native-only CPU is invented for it. Existing profile residuals keep their
non-launcher-descendant scope, not per-image or per-call attestation.

One common 120-second origin covers the cohort; cleanup retains five seconds
per refused owned Job. Original worker handle/PID/creation checks, two equal
terminal Job snapshots, nine lifetime members, zero active processes, exact
Git bytes/status, free-space/log bounds and source/HEAD/image postflight stay.
The first fault would stop the cohort without a successful-subset estimate.
Sixteen new deterministic controls and twenty original accounting controls pass,
including partial-spawn custody, first-refusal/cleanup separation, late frames,
unrenewed origin, signed differences and zero-counter ratio refusal labels.

## First attribution result

All sixteen workers pass. All sixty-four Git calls return the independent
23,589-byte body, empty stderr and exit zero. Each Job's terminal census is
complete and all cleanup results are clean. The summed sixteen worker-frame
wall observations are 12,057.1393 ms; that sum is not the entire outer session
duration because preflight, per-sample result appends and final sealing are
outside those frames.

Each row below sums two workers, or eight Git calls, **per arm**. Ratios are
profile/reference, rounded for display; raw integer numerators/denominators
remain in the packet.

| Block | Reference Job CPU ms | Profile Job CPU ms | Job CPU ratio | Wall ratio |
| --- | ---: | ---: | ---: | ---: |
| ABBA 1 | 750.000 | 781.250 | 1.042 | 0.931 |
| BAAB 1 | 718.750 | 734.375 | 1.022 | 0.680 |
| ABBA 2 | 1734.375 | 1484.375 | 0.856 | 0.908 |
| BAAB 2 | 718.750 | 687.500 | 0.957 | 1.872 |

Job CPU changes sign between blocks; wall ratios span 0.680–1.872. Worker CPU
profile/reference ratios span about 0.778–5.500, and external coordinator CPU
differences include both minus and plus 500 ms. Those separate CPU owners
matter: adding a single launcher counter or calling a negative whole-Job delta
negative overhead would misclassify the observations. The saved CPU values are
coarsely quantized; their nanosecond representation does not provide nanosecond
accuracy. These aggregate frame observations do not identify the source of the
variance or establish a pure counter-call cost.

The preregistered estimator retains all samples and four signed block deltas,
without warm-up exclusion, outlier trimming, retries or a confidence claim. The
result is inconclusive for small instrumentation overhead; acceptance remains
false rather than deriving a floor from the observed data. No acceleration,
low-overhead instrument or changed source-loader candidate is established.

## Evidence and next gate

The [108-member packet](local-runs/pipe-fixture-git-overhead-20261026/README.md)
retains exact source, protocol/build, all eleven check receipts, old first
failures and every worker log/result. Offline replay recomputes CPU and block
arithmetic without executing retained code. Four integrity controls cover index,
missing objects and corruption of old failures, metrics, raw streams and source.
Index SHA256:
`1ba276dcf62f97801f69806ea5efba138c7d47425f30d2563874aa393681fcff`.
The separate final-checks index retains passing doc-check, local non-promotable
performance-contract, verifier syntax and all four integrity checks. Those
receipts are not a package or release-qualification verdict.

Next, attribute the observed variance offline before another measurement or
cost-reduction candidate. Job-membership overhead itself remains unmeasured.
The fixture boundary is repaired on changed source, not retroactively green in
V6. Its [full package result](package-closure-v6-result-20261026.md) stays red;
the 780-second guard, all live predicates/frequencies and 300/60/10 origins stay
unchanged. No server, service, clock, dependency, release installation or product
load is used. C74, security milestone and production admission remain unresolved.
Qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
