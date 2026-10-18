# Boot buffer plain worker and preparation gate

This packet compares the fixed boot-document buffer with the old reader on the
actual retained worker validator, then exercises the existing complete
preparation healthy/fault gate. It follows the
[preregistered protocol](../../boot-buffer-plain-gate-plan-20261018.md) and the
separate [allocation proof](../boot-buffer-20261018/README.md). Neither family
is a HydraCache product throughput or long-run qualification result.

## Measured result

The single host cohort completes on first dispatch at
`/tmp/hc074-boot-buffer-plain-20261018.9DAHSh`. All twenty plain invocations
pass, covering 2,560 validations in ten independent pairs, not 2,560 independent
latency samples. Both local criteria pass separately for each compiler:

| GNU compiler | Geometric mean candidate/baseline wall ratio | Paired 95 percent interval | Lower wall cost |
| --- | ---: | ---: | ---: |
| 1.94.0 | 0.95153 | 0.94119–0.96199 | 4.85% |
| 1.88.0 MSRV | 0.95820 | 0.94156–0.97513 | 4.18% |

These ratios compare batch-average whole-validator wall time. They are not
p50/p99, CPU/op or product throughput. Affinity and host tuning are not changed
by this diagnostic; the result is scoped to this host/cohort.

All six preparation invocations also pass: six healthy fixtures with twelve
34-authentication/34-signature/two-channel-check rounds, six same-process rights
transfers, forty sticky live refusals and two real nonroot fixed-channel
refusals. Original origins, 46 effect brackets, journals, descriptor inventories
and 300/60/10-second windows remain exact. Maximum recorded successful challenge
age is 9,548,534,663 ns, leaving 451,465,337 ns in the unchanged ten-second
window. This observed margin is not a worst-case guarantee or a paired estimate
of total preparation improvement.

Candidate source `8e7e5da1` is accepted for this nonexecuting preparation gate,
not authenticated start or release admission. Independent postflight retains
boot `e81671d6-bf23-4ebf-b8fe-5dad1f45866f`, supervisor PID 8839 with zero
restarts, exact installed binary/receipt hashes, absent campaign/authority
markers and no registered fixture executable. Uploaded images/scripts/bundle
hashes match before and after. No installation or service action occurs.

Local gates pass: Windows affected check/strict Clippy and 44 portable tests;
GNU current/MSRV all-target check/strict all-feature Clippy, focused kernel,
origin, worker, manager, exchange, pipe and tracing suites plus allocation
controls; 251 source contracts, format, the 184-discovered-gate registry, docs
and the non-promotable performance contract. The full milestone
`cargo xtask verify` is not claimed.

## Exact comparison boundary

Source `8e7e5da13b4cf4219c8f9cd15d4fd72e2899dc45` adds one ignored plain test,
its registration and source guards; the runtime candidate remains `dc00ebf9`.
Four GNU release libtest images use the same source/build path and harness.
Baseline temporarily substitutes only the exact old `boot` function from
`3b8e73fe`; unused candidate helpers/tests remain in both images. The worktree
is restored to the committed candidate before any source acceptance checks.
367 selected inputs match within each compiler role; only kernel origin
differs between roles. This conservative census includes five binary-only Rust
sources not linked into the libtest image and is not a complete compiler or
registry dependency attestation.

There are five fresh pairs per compiler, in AB/BA/AB/BA/AB order. Each invocation
creates an independent original fixture and times exactly 128 retained worker
validations between full context checks. Input bytes stay unchanged within
each fixture. Fresh live time and temporary authority identities are created
by the same unchanged constructor; byte identity across independently created
fixtures is not claimed. Timing contains no capture, counting allocator,
per-iteration timer or output. Dormant test hooks remain in both images, so
these are uncaptured libtest observations rather than production-binary timings.

The estimator uses five paired log ratios and the preregistered two-sided
95-percent Student interval, df=4. Its normal-log-ratio assumption and small
sample size limit extrapolation. The local lower-cost criterion is interval
upper bound <=0.99 in each compiler cohort; non-regression is upper bound
<=1.03. Neither criterion changes product/native qualification thresholds.

## Safety and reproduction

`build.ps1` records before/after selected inputs, origin capsules, Cargo
artifacts and immutable image copies for each role/compiler. `compare-inputs.ps1`
requires exactly one differing selected input. `check.ps1` runs local package,
source, registry and documentation gates. `run-host.ps1` refuses duplicate
dispatch, uploads exact checksums and invokes only fixed nonroot fixtures.
The installed service/binary/receipt and absence of campaigns/authority markers
are checked before and after invocations and in an independent postflight.

The runner is an owned 0700 directory, not a filesystem sandbox. Existing
strict direct-/tmp authority leaves and RAII temporary directories remain;
there is no complete per-path write census or single-parent containment proof.
The bundle executable is read-only fixture data and is never run as a product.

The first local source-equality check detected an extra registry newline; its
failure is retained alongside the corrected green result. No old assertion
was weakened. Synthetic parser controls cover nine refusals and a known
constant-ratio interval; their 0.9 ratio is test data, not an observed result.
The independent verifier recomputes observed units and intervals separately,
checks all selected Git inputs, the exact baseline substitution, uploaded
image/script identities and the fixed preparation inventory.

Real authenticated bus delivery/start, all-writer cessation, durable completion
and later cells, their new SIGKILL/lost-ack matrix, same-attempt reconnect,
production enrollment and approved installation remain separate gates.
Same-process SCM_RIGHTS controls cannot establish system-bus FD negotiation.
