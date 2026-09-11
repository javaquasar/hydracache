# HydraCache Performance Evidence

This document defines the performance methodology delivered by release `0.67.0`, the surfaces it
can measure, and the narrow claim boundary for the reviewed `0.67.1` dedicated reference contract.

> **Current status (2026-09-11): reference bootstrap and frozen candidate complete.** The 0.67
> tooling release remains claim-free. The `0.67.1` contract contains one
> independently reviewed five-sample bare-metal anchor and baseline, resolving
> [`TD-0013`](technical-debt/TD-0013-dedicated-performance-runner-and-baseline-bootstrap.md).
> W7 passed from an exact post-activation `main` SHA; the anonymized final verdict is
> [`testing/perf-scenarios/0.67/results/ax42-reference-0.67.1-20260911.md`](testing/perf-scenarios/0.67/results/ax42-reference-0.67.1-20260911.md).

## Measured surfaces and claim boundaries

| Surface | Execution boundary | A future reviewed report may describe | Explicitly not measured |
| --- | --- | --- | --- |
| Embedded local cache | Real process-local cache API | Sustainable throughput-at-SLO and overload behavior for the named scenario | Network or daemon capacity |
| Client surface | Real `AxumClientSurface` router via in-process dispatch | In-process router cost | A mounted `/client/v1/*` daemon listener, socket cost, or native-wire capacity |
| RESP | Real loopback TCP to one selected prebuilt daemon | That selected node-local endpoint | Distributed RESP values, cross-node failover, or summed cluster throughput |
| Control plane | Real 3/5/7-daemon admin/control-plane wire | Admin-read cost and committed-metadata event/convergence latency | Distributed value-grid capacity or live value reshard throughput |
| Grid primitives | Exported library/model helpers in-process | Cost of the exact named consistency/session/replication primitive | End-to-end daemon-grid performance |
| Redis comparison | Same host and pinned method in alternating order | The exact paired observation | A Redis replacement or universal superiority claim |
| Metrics honesty | Existing daemon `/metrics` bracketed by an independent observer | Agreement for already-exported comparable fields | Invented metrics or service time relabeled as queue-inclusive latency |

Missing exported metrics remain `not_available`; release 0.67 does not add product metrics to make its own evidence pass.

## Measurement contract

- Capacity means the highest sustainable offered rate satisfying latency, achieved/offered rate, zero-error or declared error limits, timeouts, rejections, and bounded backlog drain. It is not peak burst throughput.
- Capacity-bearing measurements use fixed-rate open loop and latency from scheduled send time. Closed-loop output is supplemental except for the explicitly paired comparison method.
- Every report binds scenario, source commit, prebuilt binary identities, runner fingerprint, state scope, network boundary, warm-up, repeats, raw spread, and artifact digests.
- The committed minimum repeats, SLOs, zero-error rules, and 15% spread limit are not weakened by this deferral.
- Unstable spread, shared or mismatched hardware, missing tools, stale artifacts, or incomplete predecessor evidence remains fail-closed.
- Results with different surface semantics remain separate and are never combined into a protocol ratio or aggregate cluster number.

Scenarios live under [`testing/perf-scenarios/0.67`](testing/perf-scenarios/0.67). The activated
profile lives under `docs/testing/perf-profiles`; reviewed `0.67.1` payloads live under
`docs/testing/perf-anchors/0.67.1`, `docs/testing/perf-budgets/0.67.1`,
`docs/testing/perf-baselines/0.67.1`, and `docs/testing/perf-reviews/0.67.1`. Historical `0.67`
budget and baseline files remain unchanged.

## Hosted tripwire versus deferred reference evidence

| Lane | Purpose | 0.67 ship role |
| --- | --- | --- |
| `ci-shared` | Broad-tolerance hosted regression tripwire plus structural/unit receipts | Non-numerical regression signal only |
| `reference-v1` | Manual serialized execution on protected `hydracache-perf-v1` bare metal | Deferred for 0.67; independently reviewed, activated, and passed by the separate 0.67.1 W7 frozen candidate |

The protected workflow and these registered gates are retained unchanged in method:

```text
tool.perf-prebuild-067
env.hydracache-run-067-perf-core
env.hydracache-run-067-perf-resp
env.hydracache-run-067-perf-control-plane
tool.perf-budget-check-067
```

They remain fail-closed on missing capability, runner mismatch, unstable spread, stale/mixed
evidence, or unbootstrapped budgets. They were not listed as 0.67 ship-mandatory receipts; 0.67.1
adds its own reviewed activation and frozen-candidate gates without rewriting that historical ship
manifest.

The committed `reference-v1` profile, anchor, budgets, and baseline are now `bootstrapped` from five
eligible, stable, successful pre-candidate `main` runs from one fingerprint and contract family.
The activation preserves candidate self-baseline prevention. The separate frozen-candidate
reference pipeline passed and did not become a member of its own baseline.

Release 0.67.1 uses an explicit two-campaign protocol. The completed pre-activation bootstrap SHA
contributed exactly five non-ship samples; deterministic W5 automation derived a median-based
contract from all five and a separate identity reviewed the exact bytes. Those activated bytes are
the immutable input to a new exact `main` SHA's frozen-candidate ship gate. The underlying report
and scenario schema remains the 0.67 measurement contract. Operational details are in
[`testing/PERF_REFERENCE_0_67_1_REVIEW_AND_ACTIVATION.md`](testing/PERF_REFERENCE_0_67_1_REVIEW_AND_ACTIVATION.md).

## Published 0.67.1 reference result

The final exact-SHA AX42 campaign passed all 19 numerical budget checks and aggregated W0-W7 as
`ship-ready`. Its selected capacity and p99 results were:

| Surface | Capacity | p99 at SLO | Effective release boundary |
| --- | ---: | ---: | --- |
| Embedded local cache | 20,000 ops/s | 1,917 us | at least 18,000 ops/s; at most 2,137.3 us |
| In-process client surface | 24,989.1 ops/s | 2,085 us | at least 22,490.6 ops/s; at most 2,269.3 us |
| Node-local RESP endpoint | 49,948.1 ops/s | 3,769 us | at least 44,954.9 ops/s; at most 3,890.7 us |

The pinned same-host Redis comparison used five alternating-order repeats and produced these stable
medians:

| Operation | Pipeline | HydraCache | Redis | HydraCache / Redis |
| --- | ---: | ---: | ---: | ---: |
| GET | 1 | 59,630 req/s | 87,796 req/s | 67.9% |
| SET | 1 | 59,382 req/s | 87,719 req/s | 67.6% |
| GET | 10 | 136,426 req/s | 757,576 req/s | 18.1% |
| SET | 10 | 132,979 req/s | 746,269 req/s | 18.0% |

The pipeline-10 rows identify pipelined RESP processing as a measured optimization opportunity; they
do not establish general Redis-relative performance. Full identity, host, budget, artifact, and
claim-boundary details are in the
[`0.67.1` AX42 report](testing/perf-scenarios/0.67/results/ax42-reference-0.67.1-20260911.md).
The reusable host-preparation and low-noise measurement lessons are explained in
[`How to Measure Cache Performance Without Measuring Noise`](articles/006-measuring-cache-performance-on-bare-metal.md).

## Quotation rule

No numerical 0.67 release claim is permitted. The reviewed 0.67.1 bootstrap values remain the
pre-candidate contract, and the separate green W7 result is the final narrowly scoped verdict.
Numbers may be described only with their exact report, scenario, fingerprint, profile, commit,
method, and host scope, and never as portable sizing advice or universal comparative performance.
