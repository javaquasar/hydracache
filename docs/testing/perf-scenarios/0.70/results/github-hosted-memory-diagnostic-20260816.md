# GitHub-hosted HydraCache / Redis memory diagnostic — 2026-08-16

> Exploratory characterization only. This run is not qualification, bootstrap, SLO, capacity,
> release-ranking, or ship evidence.

## Run identity

- Successful measurement job: [GitHub Actions run 31915839965](https://github.com/javaquasar/hydracache/actions/runs/31915839965), `Memory Diagnostic (GitHub Hosted)`.
- Raw artifact: `memory-diagnostic-31915839965-1`, artifact id `9255333401`, retained by GitHub for 30 days.
- Source: `e627b05699b82b13a4c6536d65b63614dadd8fd6` on
  `feat/0.70-allocation-retention-audit`; source tree was clean.
- HydraCache binary SHA-256:
  `b5d7eccad183422909a5d3e65d041aac7779ebb52705132606f350f7c5083d4d`.
- Redis image: `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e`;
  workload tool: checksum-built `redis-benchmark 7.2.5`.
- Runner: GitHub-hosted Ubuntu 24.04 VM, Linux `6.17.0-1022-azure`, 4 vCPU on an
  AMD EPYC 7763 host, 16 GiB RAM, Docker 28.0.4. Target and load tool were pinned to CPU 0.
- Workload: 256-byte values, 10 clients, pipeline 1, 10,000 requests per batch, three cycles,
  one-second telemetry and nominal 180-second observation windows.
- Claim boundary recorded by the runner: `ship_evidence_eligible=false` with bare-metal and IRQ
  isolation checks explicitly not applicable.

The Actions run as a whole is red because the newly inserted Redis checksum recipe exposed an
order-dependent 0.67 governance canary; the measurement job itself completed successfully and
uploaded all ten rows. The canary was subsequently made job-specific. The red aggregate status is
not a product or measurement failure.

## Completeness and known coverage limit

All five HydraCache and five Redis status rows are `complete`, with 180 timestamped samples per
row. Native HydraCache reset responses independently proved zero retained client owners after each
of the three reset cycles. Redis reset returned exact `OK` and `DBSIZE == 0`; those predicates are
required for the row to become complete.

Analysis found one instrumentation limitation after the run: the fixed-duration collector expired
before the final TTL checkpoint because the 10,000 sequential TTL commands add wall-clock time on
top of the nominal sleep budget. The last TTL checkpoint was about 106 seconds newer than the last
sample for both products. Therefore the TTL row below is useful for footprint order only and is not
accepted as complete three-cycle expiry-reclamation evidence. The runner now keeps the collector
alive through workload overhead and fails a row whose final checkpoint is not covered.

## Process footprint

Values are medians and maxima over the timestamped process samples. PSS-anon is the strongest
available process-private allocation signal. Hydra/Redis is a descriptive ratio for this VM and
workload only.

| Scenario | Hydra RSS median / max MiB | Redis RSS median / max MiB | RSS Hydra/Redis | Hydra PSS-anon median MiB | Redis PSS-anon median MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| fixed 10k-key space | 12.91 / 13.42 | 18.25 / 18.55 | 0.71x | 5.95 | 10.19 |
| TTL expiry (partial coverage) | 11.53 / 11.54 | 15.18 / 15.25 | 0.76x | 4.41 | 7.29 |
| three load/reset cycles | 13.69 / 14.03 | 16.67 / 18.08 | 0.82x | 5.57 | 8.79 |
| three fresh-process restarts | 12.56 / 13.05 | 18.17 / 18.31 | 0.69x | 5.45 | 9.92 |
| post-load idle | 11.77 / 11.77 | 17.34 / 17.44 | 0.68x | 4.66 | 9.30 |
| all process samples | **11.77 / 14.03** | **17.34 / 18.55** | **0.68x** | **4.66** | **9.30** |

For this small node-local workload HydraCache used about 32% less median RSS and about 50% less
median PSS-anon than Redis. HydraCache remained below Redis in every scenario, but the margin is
not a universal product comparison: persistence was disabled for Redis, the Hydra local role had
its diagnostic Admin API enabled, and the VM was not isolated bare metal.

Process cgroup totals are intentionally excluded from the comparison. HydraCache ran directly in
the Actions job cgroup, which also contained build/runtime pages, while Redis had its own Docker
cgroup. Absolute cgroup values therefore have different accounting boundaries.

## Growth and reclamation signals

The original whole-window regression includes the initial warm-up step. The last-60-sample slope
is more useful for detecting continuing growth after the footprint has formed.

| Scenario | Whole-window RSS slope Hydra / Redis MiB/min | Tail RSS slope Hydra / Redis MiB/min | Interpretation |
| --- | ---: | ---: | --- |
| fixed key space | 0.845 / 0.529 | 0.073 / 0.034 | warm-up followed by plateau |
| TTL expiry (partial) | 0.592 / -0.007 | 0.003 / 0.004 | sampled portion is flat; final TTL checkpoint missing |
| load/reset | 0.623 / 0.141 | 0.158 / -0.190 | logical state cleared; allocator high-water remains |
| restart soak | 0.656 / 0.312 | 0.398 / 0.144 | concatenated fresh-process warm-ups, not one retained process |
| idle fragmentation | 0.042 / 0.010 | 0.000 / 0.000 | no continuing idle growth observed |

No scenario showed a continuing order-of-MiB-per-minute process RSS slope after warm-up. Threads
and descriptors also stayed bounded: HydraCache held 2 threads and 13 steady-state FDs; Redis held
5 threads and 8 steady-state FDs. Startup maxima were 23 and 18 FDs respectively and returned to
their steady values.

## Reset attribution

HydraCache started the reset case at 8.70 MiB RSS. The three verified post-reset checkpoints were
11.70, 13.67 and 14.03 MiB. Before reset the native snapshots contained 6,345, 7,896 and 8,519
entries; after every reset, store entries, value bytes, identity bytes, idempotency outcomes,
conditional records, locks and session heartbeats were exactly zero.

Redis started the same case at 15.40 MiB RSS and reached 17.29, 17.73 and 16.73 MiB after the three
verified resets. Both products therefore retain allocator/runtime high-water after logical cleanup.
HydraCache's final cold-to-reset RSS delta was larger in this short run (about +5.33 MiB versus
+1.33 MiB), although its absolute final footprint remained about 2.70 MiB below Redis. This is a
useful allocator/reuse candidate for release 0.71, not evidence of live HydraCache owners in 0.70.

## Work rate sanity check

The stored `redis-benchmark` summaries are not a formal throughput suite, but they verify that both
targets performed comparable work. Across the 13 recorded SET/GET batch summaries, HydraCache's
median was 37,175 requests/s and Redis's was 38,023 requests/s on the single pinned CPU. The ratio
is approximately 0.98x, so the footprint difference was not obtained by HydraCache doing an order
of magnitude less work.

## Decision

1. The GitHub-hosted data is sufficient for an order-of-magnitude statement: both products are in
   the tens-of-MiB class for this small RESP workload, with HydraCache smaller in this run.
2. No process-level unbounded growth was observed in the fixed-keyspace or idle tails.
3. The verified-zero reset snapshots move HydraCache's remaining post-reset RSS to the
   allocator/runtime high-water category for 0.71 rather than retained application state in 0.70.
4. The TTL row must be repeated with the corrected coverage guard. Public sizing, universal
   Redis-ranking, allocator selection and capacity claims still require at least three fresh runs
   on the reviewed dedicated host.
