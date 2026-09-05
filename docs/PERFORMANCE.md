# HydraCache Performance Evidence

This document defines the performance methodology delivered by release `0.67.0`, the surfaces it
can measure, and the narrow claim boundary for the reviewed `0.67.1` dedicated reference contract.

> **Current status (2026-09-05): reference bootstrap reviewed and activated; frozen candidate
> pending.** The 0.67 tooling release remains claim-free. The `0.67.1` contract now contains one
> independently reviewed five-sample bare-metal anchor and baseline, resolving
> [`TD-0013`](technical-debt/TD-0013-dedicated-performance-runner-and-baseline-bootstrap.md).
> W7 must still pass from the exact activation merge SHA before `0.67.1` may ship or publish a final
> reference verdict.

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
| `reference-v1` | Manual serialized execution on protected `hydracache-perf-v1` bare metal | Deferred for 0.67; independently reviewed and activated for 0.67.1, with W7 still pending |

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
The activation preserves candidate self-baseline prevention. A fully green frozen-candidate
reference pipeline is still required for the 0.67.1 release verdict.

Release 0.67.1 uses an explicit two-campaign protocol. The completed pre-activation bootstrap SHA
contributed exactly five non-ship samples; deterministic W5 automation derived a median-based
contract from all five and a separate identity reviewed the exact bytes. Those activated bytes are
the immutable input to a new exact `main` SHA's frozen-candidate ship gate. The underlying report
and scenario schema remains the 0.67 measurement contract. Operational details are in
[`testing/PERF_REFERENCE_0_67_1_REVIEW_AND_ACTIVATION.md`](testing/PERF_REFERENCE_0_67_1_REVIEW_AND_ACTIVATION.md).

## Quotation rule

No numerical 0.67 release claim is permitted. The reviewed 0.67.1 bootstrap values are a
pre-candidate contract, not a final release verdict. They may be described only with their exact
report, scenario, fingerprint, profile, commit, method, and host scope, and never as portable sizing
advice or universal comparative performance. Final 0.67.1 claims additionally require green W7
frozen-candidate evidence.
