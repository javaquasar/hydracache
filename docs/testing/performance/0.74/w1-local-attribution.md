# 0.74 W0/W1 local attribution evidence

Status: **non-promotable local evidence**. No release or comparative product claim may be derived
from this file.

## Identities and scope

- Frozen predecessor product candidate: `16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b`.
- Instrumented pre-W2 control: `c56a0da576e401e8d6b392d0788e3f4ee46235b5`.
- First W2 cursor candidate: `2785d429c2c3be010762cb89305322fd686df104`.
- Eager-clear W2 cursor candidate: `6fc1354e5d0ce5ae787839a8cea84a6acaf3e792`.
- Host and toolchain are the unadmitted Windows local host recorded in `host-profile.toml`.
- All receipts are under `local-runs/`, report `promotable = false`, bind the full source SHA and
  executable SHA-256, bind the deterministic workload SHA-256, and validate every response or
  native result exactly.
- No rented host, GitHub Actions qualification job, Redis oracle, or 0.73 confirmation process was
  used or changed.

W0 remains open because an annotated, published `v0.73.0` identity and its final confirmation
receipts do not exist yet. These runs compare only local I74 stages and W2 candidates. They are not
a B73/I74 release comparison.

## W1: what was measured

`resp-pipeline-profile-074` exercises real `RedisRespServer::serve_connection` calls over persistent
in-process duplex streams. It covers GET, SET, MGET, MSET, DEL and EXISTS, pipeline depths 1 and 10,
and concurrency 1 and 8. Receipts contain goodput, pipeline-batch p50/p95/p99, process CPU/op,
gross allocations/op, RSS/peak RSS, retained client state, input/read/parser/compaction counters,
translation and request-id counters, output bytes, write/flush calls, and store-lock wait/hold time.
Kernel syscall counts, short writes, pending-write duration, and per-stage allocator attribution are
explicitly unavailable rather than recorded as zero.

`native-api-profile-074` keeps four N10 paths separate: raw embedded, typed embedded including codec
cost, direct `ClientSurfaceState`, and `get_or_insert_with` hit/single-flight. Subscriber-off and
subscriber-on are distinct receipts. The first single-flight fixture was rejected before evidence
because a scheduler yield did not guarantee overlapping loaders; the retained fixture uses an
explicit 1 ms loader window and validates exactly one loader per wave.

### Confirmed owner observations

For the pre-W2 pipeline-10 controls, one logical command caused one input compaction, one high-level
write and one explicit flush:

| Cell | Moved input bytes/op | Compactions/op | Writes/op | Flushes/op |
| --- | ---: | ---: | ---: | ---: |
| GET, pipeline 10 | 265.5 | 1.0 | 1.0 | 1.0 |
| SET, pipeline 10 | 1453.5 | 1.0 | 1.0 | 1.0 |

Pipeline-1 moved no suffix bytes. This isolates W2 to deep input pipelines and W3 to output replies.
The write/flush counters show that ten replies decoded from one read still make ten `write_all` and
ten `flush` calls. W3 is therefore authorized as a separate local candidate; no W3 result is claimed
yet.

The instrumentation-on comparison itself is not admitted. In the longer three-repeat pipeline-10
screen, median GET goodput was 178,497 op/s with instrumentation versus 184,579 op/s without it
(-3.3%); SET was 190,184 versus 220,002 op/s (-13.6%). The local host was variable, but both exceed
the frozen 2% overhead limit. Counters remain useful for owner selection; instrumented goodput must
not be used as a product claim, and I74 cannot freeze with this instrumentation posture.

### Preliminary native observations

Single local receipts, all subscriber-off unless stated otherwise:

| Path | Cell | Goodput op/s | p99 us | CPU ns/op | Gross allocation B/op |
| --- | --- | ---: | ---: | ---: | ---: |
| Raw embedded | GET, c1 | 1,708,027 | 4 | 625 | 357 |
| Typed embedded | GET, c1 | 1,028,557 | 5 | 938 | 613 |
| Direct client surface | GET, c1 | 675,783 | 3 | 1,406 | 737 |
| Direct client surface | GET, c8 | 374,768 | 115 | 4,062 | 747 |
| Typed embedded | PUT, c1, subscriber off | 417,663 | 17 | 2,188 | 3,737 |
| Typed embedded | PUT, c1, subscriber on | 408,076 | 17 | 2,031 | 3,797 |

These numbers are preliminary owner observations, not repeat-backed comparisons. They show why raw
embedded, typed embedded and client-surface costs must not be pooled. The c8 direct-client receipt
also records material store-lock wait, but W6 is not authorized until a repeated contention profile
and native non-regression control exist.

## W2 candidate and negative result

The first cursor candidate deferred even an empty-buffer clear until the next read. Five short and
five longer counterbalanced pairs showed a repeatable pipeline-1 warning, so that variant was not
accepted. The candidate was narrowed to clear a fully consumed buffer immediately without copying.

The eager-clear candidate met the registered primary floor: pipeline-10 GET and SET reduced suffix
bytes moved from 265.5/1453.5 B/op to zero, a 100% reduction against the required 80%, and reduced
compactions from one per command to zero for complete batches. Semantic bytes, allocations/op and
all focused tests remained unchanged.

It nevertheless failed the preregistered unaffected-cell guard in the final five-pair local block:

| Cell | Median paired goodput delta | Median paired CPU/op delta | Median paired p99 delta |
| --- | ---: | ---: | ---: |
| GET, pipeline 1 | -0.78% | +4.30% | 0.00% |
| SET, pipeline 1 | -2.45% | 0.00% | 0.00% |
| GET, pipeline 10 | +2.34% | 0.00% | -7.55% |
| SET, pipeline 10 | +1.18% | 0.00% | -1.04% |

The guard permits at most 2% goodput and 3% CPU/op regression in unaffected cells. No samples were
discarded, including a noisy fifth GET pipeline-1 pair. Because a performance candidate must be
able to fail, W2 is rejected locally and only its code is reverted. The raw negative receipts and
this explanation are retained. A future W2 attempt needs a lower-overhead buffer state transition
and a more stable admitted host; the threshold must not be changed.

## Next step and open risks

W3 is next: bounded reply coalescing must target at least 80% fewer write/flush calls and at least
20% pipeline-10 goodput improvement while keeping pipeline-1 regression within 2%. It must cover
partial writes, Pending transitions, slow readers, disconnects, large responses, QUIT, errors,
subscriptions, ordering and fairness. Native paths remain unchanged and must be rerun after any
shared hot-path mutation.

W0, syscall-level attribution, final instrumentation overhead, HC/1 and HC/2 process controls,
Redis same-box comparison, persistence cells, and all expensive release qualification remain open.

