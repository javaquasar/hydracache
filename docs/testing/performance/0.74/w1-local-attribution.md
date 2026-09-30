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

## W3 candidate and negative result

The W3 candidate at `ad8e2a09982eb17dd93df4361f0ee8bcb4f048c3` added a connection-local
response batch bounded by 256 replies or 1 MiB. It flushed before subscription transitions, on
QUIT, before protocol errors, when no complete command remained, and at either size boundary. A
response larger than the byte bound made one-item progress without being copied into a second
retained batch. Focused scripted-I/O tests covered byte identity, partial writes, malformed input,
slow readers, oversized legal replies and the reply quantum; the full redis-compat suite remained
green.

The owner counter moved exactly as intended. In matched pipeline-10 GET and SET controls, high-level
write and flush calls fell from 1.0 to 0.1 per reply, a 90% reduction against the preregistered 80%
floor. The output high-water bound was 2,640 bytes for ten 256-byte GET replies and 50 bytes for ten
SET replies. Exact response validation and the workload hashes matched in every pair.

Five one-million-operation counterbalanced pairs nevertheless failed the complete acceptance
contract:

| Cell | Median paired goodput delta | Observed goodput range | Median paired CPU/op delta | Median paired p99 delta | Median allocation delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| GET, pipeline 1 | +13.44% | -4.48% to +24.42% | -5.84% | -18.52% | 0.00% |
| SET, pipeline 1 | -1.29% | -10.89% to +2.23% | +1.47% | +5.56% | 0.00% |
| GET, pipeline 10 | +3.06% | -1.72% to +12.19% | -1.26% | -11.74% | -0.00% |
| SET, pipeline 10 | +28.27% | +10.15% to +34.59% | -32.20% | -31.85% | -0.00% |

SET pipeline-10 cleared the 20% goodput floor, but GET pipeline-10 did not. SET pipeline-1 also
exceeded the general 3% p99 non-regression guard. The wide local ranges reinforce that this is
screening evidence, not a portable performance claim; they do not authorize selecting only the
favorable SET cell. All 44 raw W3 receipts are retained. No sample was discarded or retried, no
threshold changed, and the candidate is rejected for targeted rollback.

## W4 stage attribution and no-candidate decision

The committed `resp-stage-profile-074` tool runs deterministic allocation and timing passes for
decode, translation-context creation, command construction plus translation, and response
construction plus encoding. It validates the complete input command, the one-request execution-plan
shape, and exact output bytes before measuring. Incremental translation and encoding allocations
are differences from their deterministic construction controls and are explicitly not end-to-end
claims.

Against the median gross allocation of the five pre-W3 pipeline-1 control processes, the first
stage receipts report:

| Operation | End-to-end B/op | Decode B/op | Context B/op | Incremental translate B/op | Incremental encode B/op |
| --- | ---: | ---: | ---: | ---: | ---: |
| GET | 3,957.30 | 306 (7.73%) | 23 (0.58%) | 399 (10.08%) | 264 (6.67%) |
| SET | 4,139.30 | 818 (19.76%) | 23 (0.56%) | 399 (9.64%) | 8 (0.19%) |

W4c's direct encoder is therefore not authorized: even perfect removal of its measured allocation
owner cannot clear the preregistered 20% floor in either cell. The same is true for isolated context
or translation work. W4a is absent from these GET/SET hot cells because the W1 counters record zero
script-cache entries cloned. This is a profile-backed no-candidate decision, not evidence that the
stages are free. Combining them into one candidate merely to cross the threshold would violate the
one-hypothesis rule.

The stage CPU fields include counting-allocator overhead and coarse Windows process-time
resolution, so they are retained for owner ranking only. The allocation counts are exact gross
allocator requests in a quiescent process. Both receipts are source/binary/workload-hash bound and
non-promotable.

## W5 key ownership and compatibility decision

The v2 stage tool adds exact source/binary-bound key measurements. The 28-byte deterministic RESP
key becomes a 72-byte `redis-binary-v1-<hex>` canonical segment, a 157.14% representation
expansion. Calling `StructuredKey::stable_key()` on the one-segment translated key allocates 72
bytes per lookup. It used 46.875 ns/op in the GET receipt and 31.25 ns/op in the SET receipt, but
Windows process-time granularity makes those CPU values owner-ranking observations only.

The materialization is only 1.82% of the matched GET allocation and 1.74% of SET. Even the entire
399 B/op incremental translation stage—an intentionally generous upper bound containing non-key
work—is 10.08% and 9.64%. W5 therefore cannot clear the 20% floor in these affected cells without
being merged with other hypotheses. ADR-0022 selects the plan's defer option: keep the exact
canonical identity and avoid a binary variant, intern table, hash handle, durable migration or
rollback bridge in 0.74.

This does not claim the expansion is free. It records a measured ceiling and preserves the
compatibility debt for a workload where long or repeatedly reused keys become a material owner.

## W6a live-read and expiry attribution

The v2 native profiler adds an `expired-get` workload and opt-in counters that classify single-key
GET as a live hit, a direct expired removal, or an ordinary miss. A focused test freezes the
classification boundary with a deterministic TTL=1 fixture. Three independent local processes per
cell produced these medians with instrumentation enabled:

| Cell | Goodput op/s | p99 us | CPU ns/op | Allocation B/op | Store wait ns/op | Store hold ns/op |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Live GET, c1 | 717,375 | 2 | 1,328 | 735.9 | 35 | 600 |
| Live GET, c8 | 448,922 | 109 | 3,516 | 740.8 | 14,771 | 1,006 |
| Expired GET, c1 | 941,429 | 2 | 1,562 | 235.5 | 33 | 435 |
| Expired GET, c8 | 546,296 | 139 | 2,344 | 285.2 | 11,091 | 965 |

Every live receipt classified all 200,000 requests as live hits. Every expired receipt classified
all 20,000 requests as direct expired removals, retained zero store entries, and recorded neither a
plain miss nor a claimed background sweep. This makes the expiry comparison exact rather than a
mostly-miss workload.

At c8, aggregate mutex wait is roughly 14.8 microseconds per live operation while the critical
section itself is roughly 1.0 microsecond. The result authorizes only W6a: a read/read concurrency
split for live hits with the current write-locked expiry cleanup retained as the fallback. It does
not authorize sharding, a new expiry index, quota changes, or a representation change. Product
performance must be measured with instrumentation off against a v2-tool baseline.

## Next step and open risks

W6a is the next isolated product candidate. It must keep tenant admission and quota paths on their
canonical locking order, release read ownership before any expiry mutation, and prove that delayed
cleanup cannot remove a live replacement. W6b sharding remains unauthorized until the W6a result
still shows material wait and a separate proposal clears the high-concurrency floor. Raw and typed
embedded paths are mandatory unaffected controls.

W0, syscall-level attribution, final instrumentation overhead, HC/1 and HC/2 process controls,
Redis same-box comparison, persistence cells, and all expensive release qualification remain open.
