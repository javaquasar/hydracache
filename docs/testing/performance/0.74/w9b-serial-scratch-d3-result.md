# W9b D3a: allocation hypothesis confirmed, candidate rejected on live peak

## Decision

`p74-w9b-serial-get-scratch-v1` fails the preregistered requested-live-memory
guard. Retain the complete screen, then remove only this product candidate. Do
not increase the memory ceiling, trim pairs, restart the experiment, or combine
a reducer/value-ownership optimization to rescue it. Acceptance remains zero,
C74 `UNRESOLVED`, and release admission closed. Still not distributed transactions.

Source `3171d02a8ebb77111d819ebd845516ea797c996d` includes instrumentation and
the unchanged D2 implementation `b8cc7c6c`. Both release binaries came from
that same clean source: feature off/on, not an old baseline binary relabeled.
The off product path corresponds to legacy default baseline `4d733e31`.
The source/feature/binary/lock/toolchain seal precedes every numerical sample.

All evidence is in `local-runs/w9b-d3a-3171d02a/`: `seal.json`, `summary.json`,
140 raw receipts and 140 attempt records. No attempt failed or was retried.
Each of seven registered cells has five independent AA and five counterbalanced
AB pairs. AA uses the same feature-off binary in both role positions; AB is
feature off/on. Every pair reconciles seed, corpus, workload digest, warm-up,
operation budget, read chunks and exact final values/cardinality. Duration is a
fixed operation budget, not a timed throughput/SLO qualification.

## Measurements and attribution

Only gross allocation, allocation calls and outstanding requested-layout bytes
are instrumented. Immutable request/expected corpus is built before counting.
The measured canonical connection window includes decoding, dispatch, reduction,
encoding, immediately-ready scripted writes/flushes and close. These are not
isolated encoder-only ratios. No output collector or expected payload construction
is charged in the window. There is no competing task in the current-thread runtime.

| RESP2 plaintext scripted cell | Gross bytes/op off → on | Gross change | Window peak above start off → on | Peak change |
| --- | ---: | ---: | ---: | ---: |
| GET 4 KiB, pipeline 50, 2,000 operations | 15,176.952 → 11,236.152 | −25.97% | 25,134 → 29,239 bytes | +16.33% |
| GET 1 MiB, pipeline 10, 200 operations | 3,147,349.520 → 2,308,479.120 | −26.65% | 2,114,091 → 3,162,679 bytes | +49.60% |

These paired values repeat exactly in all five AB pairs. All AA allocation and
window-peak ratios equal 1.00. The fixed mean-log-ratio t(4) interval therefore
collapses to the observed ratio; that reflects this deterministic scripted lane,
not certainty about real sockets, scheduler latency, RSS or allocator page behavior.

The gross-allocation floor of 20% passes in both affected cells. The zero-increase
peak ceiling fails in both. The extra peak is exactly one encoded frame: 4,105
bytes and 1,048,588 bytes. Scratch remains alive while the next canonical command
creates/reduces its response. Reusing its address removes repeated allocations
but adds one simultaneous owner at that next response's peak. This confirms the
risk named before D2/D3, without inventing a new cause from RSS fluctuations.

Pipeline-one GET, 256-byte GET, 4 KiB SET, alternating 4 KiB/256-byte GET, and
one-byte fragmented GET controls all have allocation and window-peak ratios 1.00.
Next-read and post-close live **increments above each process's starting owners**
have zero candidate increase in every pair. Baseline/feature binary filenames
have a one-byte outside-window owner difference; absolute process totals must
not be mistaken for workload increments. Idle owner release passes, but it
cannot excuse the higher active peak. Every response still emits one write and
one flush, with exact bytes. No syscall reduction exists here.

## Scope limits and rollback

This is an early **rejection** screen, not complete D3 performance qualification.
Requested layouts exclude allocator metadata and hidden System realloc overlap.
RSS snapshots and lifetime peaks are retained only as supplemental observations,
not window peaks or a timed idle retention proof. Profiling atomics change cost;
no CPU, goodput, p50/p95/p99, real-network or RSS improvement is claimed.

The five separate native/transport/timing/retention/CI obligations listed in the
summary remain unexecuted, not waived or reported as passed. A conclusively red
early memory guard makes broader candidate qualification unnecessary. Embedded,
ClientSurfaceState, HC1 and HC2 do not gain numerical non-regression receipts by
having unchanged source. No rented server, GitHub dispatch or expensive product
workload ran. Frozen 0.73, root dependencies, qualification manifest and thresholds
are unchanged.

The selective rollback removes the experimental product feature, scratch helper
and connection-writer branch. Keep the canonical adversarial integration guards,
baseline owner evidence, D2 semantic record and complete D3a negative receipts.
The historical feature-on implementation can be inspected at `b8cc7c6c` and the
sealed measurement source at `3171d02a`; it is not a current release capability.
Future owners require separate D1 attribution and a new proposal, not a quiet
retry of this candidate with looser memory bounds.
