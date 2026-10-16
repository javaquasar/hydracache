# Worker proof reuse second screen results

The second, independently preregistered local W11 screen passes its fixed
practical-effect rule. Source is cca2cb68a6d00e8fa4af6f08f9b59cdf199956a6;
baseline source is 73f71f07, whose selected runtime/fixture/instrumentation bytes
equal 78d8d9fc. The [new preregistration](../../diagnostic-worker-proof-reuse-v2-plan-20261016.md)
was committed as ab1a3ee7 before implementation and observations. The
[first stopped screen](../worker-proof-reuse-20261016/README.md), its failure and
rollback remain immutable; none of its observations enter this estimator.

## Local diagnostic effect

All ten invocations pass once in B/C, C/B, B/C, C/B, B/C order. Ratios below are
candidate/baseline elapsed time, not throughput. The point is the geometric mean
of five paired ratios; intervals use the preregistered 10000-resample percentile
bootstrap, xorshift32 seed 0x0740c0a0 and linear quantiles. Both upper bounds are
below the unchanged 0.80 floor.

| Stage | Baseline seconds range | Candidate seconds range | C/B ratio | 95% interval |
| --- | ---: | ---: | ---: | --- |
| Reserved publication | 23.4451–24.0242 | 5.3115–5.6794 | 0.232739 | 0.228305–0.236526 |
| Heartbeat binding | 5.4761–5.8288 | 1.2861–1.4007 | 0.236497 | 0.228309–0.243068 |

This is unoptimized Rust 1.94 debug-fixture elapsed time on local WSL, including
test instrumentation and scheduler effects. It is not production CPU/op,
allocation/retained-memory measurement, native/RESP goodput, MSRV timing
qualification or release D3/D4. Signature duration nests inside request
authentication and is not added to it. No isolated percentages are composed.

Request authentication and request signature counts match exactly in every pair:
11/8/96/32/272/66 across Setup/Fence/Retain/Observe/Publish/Bind. Full worker
document verification falls from 99/72/908/308/2696/652 to 2/0/0/0/0/0. Both
independent constructors still fully verify. No global cache merges their authority.
Every source read, current epoch/context/peer/kernel observation and signed clock
predicate remains unchanged, with the same 60/10-second windows.

The one substitution reuses proof of an unchanged authenticated document inside
its original checked owner. Trust/key/body pin/epoch and host context must still
match, the signed-envelope SHA256 must remain original, and the retained canonical
projection must match the original body pin. Constructor, retained fields/buffers,
refusal ordering and lifetime stay unchanged. This relies on the existing SHA256
integrity assumptions; the enclosing owners continue to check live authority.
See the [design](../../diagnostic-worker-proof-reuse-design.md).

## Tests and retained development failures

Before implementation the single-owner and two-owner mechanism tests fail at
10 versus 1 and 18 versus 2 full verifications. After implementation all five
portable tests pass on Linux/Windows, Rust 1.94/1.88. The exact previous method
remains a private test-only differential oracle; tests include 128 seeded
input/trust/context/error-priority cases, sixteen top-level projection mutations
and four concurrent independent owners.

All eight affected-package all-target/all-feature check/strict-Clippy lanes pass.
Both Linux compilers pass 99 original-input tests with four registered manual
ignores, 29 kernel tests with one prior manual ignore, the five context/projection/
IPC/file/policy targets (including 30 projection and 24 IPC tests), 15 credential
tests, the unchanged causal four-case manual proof and 28 compile-fail examples.
No old test is skipped, serialized or retried. The two new green IPC runs do not
explain the retained old Host(Busy) occurrences, historical Time/Kernel(Invalid)
or arbitrary shared-load robustness.

All 177 static contracts, formatting, documentation/registry and local
non-promotable performance checks pass; ship admission remains expected-red.
The first formatting check requires only the standard multiline assert layout;
its failure is retained. A pre-screen hash command mistakenly interpolates a
receipt object as a path and fails on "[object Object]". Its failed receipt is
retained separately; the corrected command verifies the prescribed baseline and
fresh candidate before the first pair. No benchmark invocation was retried.

## Evidence identity and scope

The baseline is the separately retained libtest from the first proof experiment,
SHA256 7e8247610919a8dc39f3f6b7f6644944084a36a78cf949045fe9a0eaa7a946ad.
It is not claimed freshly rebuilt for this screen. A fresh baseline preflight
passes before mutation; its timings are not a sixth pair. Candidate SHA256 is
65efc5332917079da35262bd9814099944bfd9551f8c5c33e753ae83d367f32e. Both before/after
hash receipts agree. No build or whole-workspace formatting overlaps the real-clock
regressions or pairs. Selected source remains unchanged afterward.

Run powershell -NoProfile -File ./verify.ps1 from any directory. It checks eight
source blobs/tree, the nine-path candidate inventory, 33 unchanged inputs,
preregistration identity, 55 captures and 57 hashed artifacts, all six stage rows in every
invocation, counter/workload guards and independent bootstrap reproduction.
Log line endings are LF, with one newline for empty output. expected_exit_code
retains negatives; it does not waive a release gate. The unsigned packet and binary
hashes prove consistency/identity, not trusted execution or clean source-to-binary
attestation.

Read-only in-memory canaries refuse server activation, fabricated ratio statistics
and a changed worker-count row. The on-disk verifier also passes from the parent
workspace directory; these checks mutate no packet, source or benchmark sample.

This accepts only the local tooling effect. Full release verify, actual enrollment,
Starting authority handoff, authenticated original-start, all-writer cessation,
completion/later cells, new SIGKILL boundaries and host rehearsal remain separate.
No product candidate, server workload, qualification, threshold change or push
is admitted. Diagnose the historical IPC lock lifetime separately; do not change
live revocation semantics to pursue the next remaining cost.
