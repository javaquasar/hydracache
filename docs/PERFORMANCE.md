# HydraCache Performance Evidence

This document defines the performance methodology delivered by release `0.67.0`, the surfaces it can measure, and the claim boundary while dedicated reference evidence is deferred.

> **Current status (2026-07-26): tooling scope implemented; numerical evidence deferred.**
> The W0-W10 measurement and governance implementation is present. The 0.67 code release may ship that infrastructure without capacity, sizing, Redis-comparison, metrics-agreement, or numerical baseline claims. Dedicated `reference-v1` bootstrap is tracked by [`TD-0013`](technical-debt/TD-0013-dedicated-performance-runner-and-baseline-bootstrap.md).

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

Scenarios live under [`testing/perf-scenarios/0.67`](testing/perf-scenarios/0.67); profiles, budgets, and baseline contracts live under `docs/testing/perf-profiles`, `docs/testing/perf-budgets/0.67`, and `docs/testing/perf-baselines/0.67`.

## Hosted tripwire versus deferred reference evidence

| Lane | Purpose | 0.67 ship role |
| --- | --- | --- |
| `ci-shared` | Broad-tolerance hosted regression tripwire plus structural/unit receipts | Non-numerical regression signal only |
| `reference-v1` | Manual serialized execution on protected `hydracache-perf-v1` bare metal | Deferred by TD-0013; no 0.67 numerical claim |

The protected workflow and these registered gates are retained unchanged in method:

```text
tool.perf-prebuild-067
env.hydracache-run-067-perf-core
env.hydracache-run-067-perf-resp
env.hydracache-run-067-perf-control-plane
tool.perf-budget-check-067
```

They remain fail-closed on missing capability, runner mismatch, unstable spread, stale/mixed evidence, or unbootstrapped budgets. They are not listed as 0.67 ship-mandatory receipts while TD-0013 is open.

The committed `reference-v1` profile, anchor, budgets, and baseline stay `unbootstrapped`. Closure requires the protected runner, at least five eligible stable successful `main` runs from one fingerprint/contract family, independent anchor/baseline/budget review, activation without candidate self-baselining, and one fully green frozen-candidate reference pipeline.

Release 0.67.1 prepares that closure as an explicit two-campaign protocol. The pre-activation
bootstrap SHA contributes exactly five non-ship samples; deterministic W5 automation derives a
median-based contract from all five and a separate identity reviews the exact bytes. Only after
those bytes are committed may a new exact `main` SHA run the full frozen-candidate ship gate. The
canonical activated budget/baseline paths are under `0.67.1`; the underlying report and scenario
schema remains the 0.67 measurement contract. Preparation alone does not activate a numerical
claim or resolve TD-0013. Operational details are in
[`testing/PERF_REFERENCE_0_67_1_REVIEW_AND_ACTIVATION.md`](testing/PERF_REFERENCE_0_67_1_REVIEW_AND_ACTIVATION.md).

## Quotation rule

No numerical 0.67 release claim is currently permitted. Exploratory output must be labeled exploratory and must not appear as capacity floors, sizing advice, comparative claims, or release baselines. After TD-0013 closes, any quoted number must identify its report, scenario, fingerprint, profile, commit, method, and claim scope.
