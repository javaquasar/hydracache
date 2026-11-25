# Performance

HydraCache publishes performance numbers only together with the exact host, commit, scenario, and method that produced them. This page explains what has been measured, what the numbers mean, and — just as important — what they do not mean.

> **Read this first.** HydraCache is not pitched as a Redis throughput replacement. Its value is correctness and DB-aware invalidation inside your process. The numbers below are a reference point for one machine, not sizing advice.

## How Numbers Are Produced

- **Capacity means sustainable, not peak.** A scenario's capacity is the highest fixed offered rate that still meets its latency SLO, achieved-vs-offered rate, error, timeout, and backlog-drain limits. Burst peaks are not reported as capacity.
- **Open-loop load.** Capacity measurements send at a fixed rate and measure latency from the *scheduled* send time, so a stalled server cannot hide its own queueing delay.
- **Repeats and spread.** Each result needs a minimum number of repeats and fails if the spread between repeats exceeds 15%.
- **Bare metal, fully identified.** Reference runs execute serially on a protected, non-virtualized host. Each report records the commit, binary identities, host fingerprint, warm-up, repeats, raw spread, and artifact digests.
- **Separate surfaces stay separate.** The embedded cache, the in-process client surface, a node-local RESP endpoint, the control plane, and grid primitives are measured independently and are never summed into one "cluster throughput" figure.
- **Hosted CI is a tripwire, not a benchmark.** Shared CI runners only catch large regressions.

## Reference Result (0.67.1)

Host: AMD Ryzen 7 PRO 8700GE (8 cores, SMT off), 64 GiB RAM, NVMe RAID1, Ubuntu 24.04, no virtualization. The final campaign passed all 19 numerical budget checks.

| Surface | Sustainable capacity | p99 at SLO |
| --- | ---: | ---: |
| Embedded local cache | 20,000 ops/s | 1,917 µs |
| In-process client surface | 24,989 ops/s | 2,085 µs |
| Node-local RESP endpoint | 49,948 ops/s | 3,769 µs |

These are the capacities of the named scenarios at their SLOs on this host — not the maximum speed of a cache lookup, and not the capacity of a multi-node cluster.

### Same-Host Redis Observation

On the same host, with a pinned method and five alternating-order repeats:

| Operation | Pipeline | HydraCache RESP | Redis | Ratio |
| --- | ---: | ---: | ---: | ---: |
| GET | 1 | 59,630 req/s | 87,796 req/s | 67.9% |
| SET | 1 | 59,382 req/s | 87,719 req/s | 67.6% |
| GET | 10 | 136,426 req/s | 757,576 req/s | 18.1% |
| SET | 10 | 132,979 req/s | 746,269 req/s | 18.0% |

The honest conclusion: HydraCache's node-local RESP path gains much less from pipelining than Redis does, and that is a known optimization target. The table is one paired observation on one host. It is not a portable ratio and not a distributed-cache comparison.

## Efficiency Work Since Then

Release 0.73 focused on measured efficiency rather than headline throughput: it removed redundant key copies in expiry scans, shared immutable event bytes and tags between subscribers, dropped a redundant RESP output-frame copy, and bounded the HC/2 application queue. Each change was qualified with paired local measurements and published with its own scope; no portable throughput claim follows from them. See the [0.73.0 release notes](releases/0.73.0.md).

## Measuring Your Own Workload

The reference result tells you how carefully numbers are produced, not how fast *your* service will be. Your hit ratio, value sizes, loader latency, and key cardinality matter far more. To measure:

1. Use the cache's own counters — hits, misses, loader calls, single-flight joins — from [Diagnostics and Events](guides/diagnostics-and-events.md).
2. Size memory with [Memory Accounting](guides/memory-accounting.md) and [Memory Sizing](guides/memory-sizing.md).
3. Load-test with a fixed offered rate and report latency from scheduled send time, as above.

## Sources

- `docs/PERFORMANCE.md` — methodology, surfaces, and quotation rules.
- `docs/testing/perf-scenarios/0.67/results/ax42-reference-0.67.1-20260911.md` — the full anonymized reference report.
- `docs/articles/006-measuring-cache-performance-on-bare-metal.md` — how the host was prepared and how noise was controlled.
