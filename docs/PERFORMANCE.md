# HydraCache Performance Evidence

This document defines what HydraCache performance results mean, which execution
surfaces release `0.67.0` actually measures, and when a result is eligible to
support a release statement.

> **Current status (2026-07-21): implementation closure reached, release not shipped.**
> The W0-W10 measurement and governance implementation is present, but no
> `0.67.0` performance number is release-qualified yet. The annotated `v0.66.0`
> predecessor is present and ancestral; the W7 `reference-v1` anchor and rolling
> baseline remain `unbootstrapped`. Shipping requires at least five eligible dedicated
> `main` runs from the same qualified runner family, an independently reviewed
> anchor/budget payload, and a fresh exact-candidate receipt set.

## Measured surfaces and claim boundaries

| Surface | Execution boundary | Eligible statement | Explicitly not measured |
| --- | --- | --- | --- |
| Embedded local cache | Real process-local cache API | Sustainable throughput-at-SLO and overload behavior for the named embedded scenario | Network or daemon capacity |
| Client surface | Real `AxumClientSurface` router via in-process dispatch | In-process router cost for the named workload | A mounted `/client/v1/*` daemon listener, socket cost, or native-wire capacity |
| RESP | Real loopback TCP to one selected prebuilt daemon | Capacity and availability of that selected node-local RESP endpoint | Distributed RESP values, cross-node failover, or summed cluster throughput |
| Control plane | Real 3/5/7-daemon admin/control-plane wire | Admin-read cost and committed-metadata event/convergence latency | Distributed value-grid capacity or live value reshard throughput |
| Grid primitives | Exported library/model helpers in-process | Cost of the exact consistency/session/replication primitive named by the report | End-to-end daemon-grid or replicated-value performance |
| Redis comparison (W8) | Same host, same pinned `redis-benchmark`, alternating order, pinned Redis image | A reproducible comparison of Redis and one HydraCache node-local RESP endpoint under that exact method | A general Redis replacement, universal superiority, or marketing benchmark claim |
| Metrics honesty (W9) | Existing daemon `/metrics` output bracketed by an independent observer interval | Agreement only for fields the daemon already exports and whose semantics are comparable | Invented metrics, in-process metrics relabeled as exporter evidence, or server service time presented as queue-inclusive latency |

W9 is deliberately **exported-only**. A missing metric is recorded as
`not_available`; the characterization release does not add a product metric to
make its own comparison pass.

## Measurement contract

- Capacity means the highest sustainable offered rate that satisfies the full
  SLO predicate: latency, achieved/offered rate, errors, timeouts, rejections,
  and bounded backlog drain. It is not peak burst throughput.
- Capacity-bearing measurements use a fixed-rate open loop and measure latency
  from scheduled send time. Closed-loop `redis-benchmark` output is supplemental
  except for the explicitly paired W8 comparison.
- Every report binds the scenario, source commit, prebuilt binary identities,
  runner fingerprint, state scope, network boundary, warm-up, repeats, and raw
  spread. Compilation and image pulls are outside the measurement window.
- Unstable spread, a shared or mismatched runner, missing tools, stale artifacts,
  or an incomplete predecessor receipt makes a run non-evidence. It is never
  repaired by silently widening a budget or averaging incompatible hosts.
- Results from different surface semantics are shown separately. They are not
  divided into a protocol ratio or combined into an aggregate cluster number.

The committed scenarios live under
[`docs/testing/perf-scenarios/0.67`](testing/perf-scenarios/0.67), while runner,
budget, and baseline contracts live under `docs/testing/perf-profiles`,
`docs/testing/perf-budgets/0.67`, and `docs/testing/perf-baselines/0.67`.

## Shared CI versus dedicated release evidence

| Lane | Purpose | May satisfy a performance ship gate? |
| --- | --- | --- |
| `ci-shared` | Broad-tolerance regression tripwire on a declared hosted-runner class; structural/unit receipts remain useful | No |
| `reference-v1` | Serialized scheduled/manual execution on a dedicated `hydracache-perf-v1` runner with exact prebuild, affinity/quota/governor and fingerprint checks | Yes, after bootstrap and only through registered gates |

Shared CI variability is expected. A shared result may warn about a regression,
but it cannot establish a capacity floor, create the release anchor, or satisfy
`--require-ship`.

The enforcing W7 decision requires both:

1. an immutable reviewed release anchor, preventing gradual ratcheting; and
2. an eligible rolling `main` window from the same runner/contract family,
   detecting recent regressions.

The current committed `reference-v1` budget and baseline are intentionally
`unbootstrapped`. Bootstrap is allowed only after at least five eligible,
successful, stable, clean-commit dedicated `main` runs are available. The exact
anchor/window payload and budget change require an independent approver; the
candidate cannot baseline or approve itself.

## Reproduction and evidence

Direct `hydracache-loadgen` commands are useful for development, but their JSON
files are not ship receipts. Release evidence must run the registered commands
through `evidence-run`, beginning with the exact-candidate prebuild:

```text
tool.perf-prebuild-067
env.hydracache-run-067-perf-core
env.hydracache-run-067-perf-resp
env.hydracache-run-067-perf-control-plane
tool.perf-budget-check-067
```

The RESP lane owns the sealed W3 reports, W8 same-box comparison, node-local
brownout/overload reports, and `metrics-resp.json`. The control-plane lane owns
the separate 3/5/7 reports, control-plane brownout report, and
`metrics-control-plane.json`. Exact PowerShell and Bash commands are documented
in [`TESTING.md`](TESTING.md); the gate-to-CI map is in [`GATES.md`](GATES.md).

## Current no-ship decision

Implementation closure is not release closure. As of 2026-07-21:

- the annotated `v0.66.0` predecessor is present and satisfies the ancestry
  prerequisite;
- the W7 anchor, rolling baseline, and all numerical budgets remain
  unbootstrapped pending at least five eligible dedicated `main` runs and
  independent review;
- final dedicated core, RESP/Redis, and control-plane artifacts have not been
  accepted as one frozen exact-candidate set; and
- the complete canary/gate receipts have not made
  `release-evidence --release 0.67 --require-ship` green.

Therefore this repository makes no shipped `0.67.0` capacity, Redis-comparison,
or metrics-agreement claim yet. Any quoted number must identify its report,
scenario, runner fingerprint, profile, source commit, method, and claim scope;
otherwise it is an exploratory measurement, not a HydraCache release result.
