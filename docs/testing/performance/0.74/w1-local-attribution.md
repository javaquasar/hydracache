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

### W3 real-TCP attribution and bounded-batch retry

The v2 profiler added a loopback `TcpListener`/`TcpStream` transport with `TCP_NODELAY`, while
preserving the exact command generator, response validator and workload digest used by the
in-process duplex transport. The v3 profiler then wrapped only the accepted server socket and
counted `AsyncWrite::poll_write`/`poll_flush` attempts, ready/pending outcomes, requested/written
bytes and short writes. These are socket-layer polls, not kernel syscall counts.

Three baseline processes per cell used 200,000 operations, 10,000 warmup operations, payload 256,
key space 4,096, seed 7,407 and c1. The loopback TCP medians were:

| Cell | Goodput op/s | CPU ns/op | p99 us | RESP writes/op | Socket write polls/op |
| --- | ---: | ---: | ---: | ---: | ---: |
| GET pipeline 1 | 39,417 | 25,078 | 65 | 1.0 | 1.0 |
| GET pipeline 10 | 86,574 | 11,406 | 309 | 1.0 | 1.0 |
| SET pipeline 1 | 38,600 | 25,000 | 72 | 1.0 | 1.0 |
| SET pipeline 10 | 85,112 | 11,328 | 337 | 1.0 | 1.0 |

Every TCP write poll completed ready, no short write was observed, and the server issued one
explicit flush poll per reply. Pipeline 10 therefore batched client requests but did not batch
server replies. This confirmed the W3 owner at the Tokio socket boundary. It did not establish how
many kernel `send` calls occurred: Windows Performance Recorder refused the local Network profile
with `0xc5585011` (`Failed to enable the policy to profile system performance`). No elevated or
policy-bypassing retry was attempted, and no trace was produced.

The previously isolated 256-reply/1-MiB response batch was replayed unchanged as a new candidate.
All focused byte-identity, partial-write, malformed-frame, slow-reader, oversized-response,
disconnect/churn and boundary tests passed. The first three-process screen reduced both RESP calls
and server socket write polls from 1.0 to 0.1 per operation at pipeline 10. Against the v3 baseline,
TCP pipeline-10 goodput increased 81.24% for GET and 88.72% for SET, with no allocation change.
TCP GET pipeline 1, however, lost 5.09% goodput and increased p99 by 12.31%, crossing frozen guards.

A longer counterbalanced check then ran five A/B pairs per TCP cell with 500,000 operations and
20,000 warmup operations. The candidate and baseline executables were preserved separately; order
alternated by pair, workload hashes matched and all results were exact:

| TCP cell | Median paired goodput | Observed goodput range | Median CPU/op | Median p99 | Pairs crossing a frozen guard |
| --- | ---: | ---: | ---: | ---: | ---: |
| GET pipeline 1 | -2.78% | -5.21% to -0.29% | +3.73% | +3.12% | 4/5 |
| GET pipeline 10 | +80.90% | +69.81% to +85.76% | -45.03% | -49.57% | 0/5 |
| SET pipeline 1 | -0.22% | -2.86% to +2.20% | +1.27% | +8.20% | 4/5 |
| SET pipeline 10 | +83.01% | +80.64% to +90.97% | -44.44% | -48.13% | 0/5 |

The deep-pipeline mechanism is real and materially larger over loopback TCP than in duplex, but
the exact candidate still fails the shallow-request tail/CPU contract. The favorable deep cells
are not selected in isolation. The batch and its candidate-only tests were reverted again; v2/v3
baseline, candidate and paired receipts are retained. A future W3 design must avoid adding the
batch state/decision cost to pipeline 1, rather than weakening its guard.

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

### W6a candidate and negative result

The candidate at `79b0ceb3639d01d3eb043525c8d39afcf621de71` replaced the store mutex with
an `RwLock`. Only unisolated single-key GET used shared ownership. Tenant-isolated requests and every
mutation retained the canonical exclusive path. An expired read released its shared owner, acquired
the write owner, and rechecked the current entry; a deterministic test proved that a live
replacement installed after the stale observation survived and was returned.

Five 500,000-operation counterbalanced pairs, plus smaller exact-expiry cells, rejected the
candidate:

| Cell | Median paired goodput delta | Median CPU/op delta | Median p99 delta | Allocation delta |
| --- | ---: | ---: | ---: | ---: |
| Client GET, c1 | +16.45% | -11.43% | 0.00% | 0.00% |
| Client GET, c8 | -3.74% | +234.07% | -6.25% | 0.00% |
| Client PUT, c1 | -0.63% | +3.23% | 0.00% | 0.00% |
| Client PUT, c8 | -2.19% | +2.86% | +9.43% | 0.00% |
| Expired GET, c1 | -6.61% | 0.00% | 0.00% | 0.00% |
| Expired GET, c8 | -31.62% | +383.33% | -16.78% | 0.00% |
| Raw embedded GET, c1 | -2.42% | -5.26% | 0.00% | 0.00% |
| Raw embedded GET, c8 | -3.78% | +2.17% | +5.26% | +0.31% |
| Typed embedded GET, c1 | +0.17% | 0.00% | 0.00% | 0.00% |
| Typed embedded GET, c8 | +1.45% | +3.33% | +4.17% | +0.01% |

The shared read path improved the single-client cell but missed the 20% high-concurrency floor and
made c8 CPU substantially worse. The expiry fallback necessarily paid read plus write acquisition,
and write/native controls also crossed frozen guards. All 100 exact-response receipts are retained;
no sample or unfavorable control was removed. W6a is rejected and W6b remains unauthorized rather
than treating the result as permission for a more invasive shard table.

## W7 multi-key attribution

The RESP profile tool now emits real batch-eight DEL and EXISTS commands. Its DEL guard requires
`key_space >= max(operations, warmup) * batch_size`, and preload uses MSET, so every measured delete
has exactly eight unique hits. Three instrumentation-on owner processes per cell report:

| Cell | Goodput op/s | CPU ns/op | Allocation B/op | Dispatches/op | Store locks/op |
| --- | ---: | ---: | ---: | ---: | ---: |
| MGET b8, p1 | 49,899 | 20,312 | 32,979 | 1 | 1 |
| MGET b8, p10 | 46,160 | 24,219 | 39,947 | 1 | 1 |
| MSET b8, p1 | 50,197 | 21,094 | 34,879 | 1 | 1 |
| MSET b8, p10 | 46,357 | 22,656 | 44,189 | 1 | 1 |
| DEL b8, p1 | 37,816 | 27,344 | 26,229 | 9 | 9 |
| DEL b8, p10 | 35,485 | 27,344 | 28,748 | 9 | 9 |
| EXISTS b8, p1 | 56,304 | 19,531 | 15,761 | 1 | 1 |
| EXISTS b8, p10 | 52,594 | 20,312 | 18,299 | 1 | 1 |

MGET, MSET and EXISTS already cross the client surface as one batch request and are closed as
already vectorized for this proposal. DEL performs one BatchGet followed by one Invalidate request
for every existing key. Reducing the eight-key fixture from nine dispatch/lock acquisitions to two
has a 77.8% mechanical ceiling and is the only authorized W7 candidate. The numbers are owner
attribution, not a throughput claim; candidate goodput still must clear the 20% multi-key floor.

### W7 DEL candidate result

The candidate added a bounded in-process batch-invalidation seam without changing the HC wire
protocol. RESP DEL delegated its deduplicated canonical keys to that seam, which admitted identity
and namespace once, removed live/expired entries under one store lock, released tenant quota, and
published ordered per-key events. Five counterbalanced baseline/candidate processes per cell used
20,000 operations, 2,000 warmup operations, batch size 8, payload 256, key space 160,000, seed
7,407, concurrency 1, instrumentation on, and exact response validation. Each baseline/candidate
pair has one workload hash.

| DEL b8 cell | Baseline median | Candidate median | Delta |
| --- | ---: | ---: | ---: |
| p1 goodput op/s | 38,188 | 56,350 | +47.56% |
| p1 CPU ns/op | 26,562 | 18,750 | -29.41% |
| p1 allocation B/op | 26,229 | 13,483 | -48.60% |
| p1 p99 us | 63 | 43 | -31.75% |
| p10 goodput op/s | 35,920 | 53,622 | +49.28% |
| p10 CPU ns/op | 29,688 | 20,312 | -31.58% |
| p10 allocation B/op | 28,748 | 16,002 | -44.34% |
| p10 p99 us | 569 | 393 | -30.93% |

The mechanical result was exact: logical dispatches and store acquisitions fell from 9/op to 1/op.
This clears the W7 isolated 20% floor, but it is not sufficient for acceptance because W7 changes
the client-surface crate and therefore owes the frozen native controls.

The first 20,000-operation native screen is retained as `w7-native-control-*` but is explicitly
non-decisional: Windows process CPU quantization was too coarse. The guard was rerun with 500,000
operations, 20,000 warmup operations, instrumentation off, and five counterbalanced pairs per
cell. Workload hashes and exact-result validation match in every pair. Median deltas were:

| Native control | Goodput | CPU/op | Allocation/op | p99 |
| --- | ---: | ---: | ---: | ---: |
| Client GET c1 | +0.32% | +2.78% | 0.00% | 0.00% |
| Client GET c8 | -0.54% | -5.07% | 0.00% | +4.04% |
| Client PUT c1 | -1.27% | -6.45% | 0.00% | 0.00% |
| Client PUT c8 | -3.95% | +3.00% | 0.00% | +159.81% |
| Raw embedded GET c1 | +0.37% | 0.00% | 0.00% | 0.00% |
| Raw embedded GET c8 | -1.04% | 0.00% | -0.18% | +21.05% |
| Typed embedded GET c1 | +0.03% | 0.00% | 0.00% | 0.00% |
| Typed embedded GET c8 | -7.10% | +2.10% | +0.15% | +92.31% |

Client PUT c8 and typed embedded GET c8 cross the 98% goodput floor; several c8 p99 controls also
cross the 3% bound. The affected operations do not invoke the new seam, but the release contract
guards the built candidate, not only source-level intent. W7 is therefore rejected. All favorable
and unfavorable receipts are retained, thresholds are unchanged, and only the candidate seam and
its API-specific tests are reverted.

### W7 same-binary A/A falsifier

After the targeted revert, the v5 native profiler ran five counterbalanced `A/B` pairs where both
labels invoked the same executable, source, workload and binary digest. Each process used 500,000
operations, 20,000 warmup operations, payload 256, key space 4,096, seed 7,407, c8 and
instrumentation off. This is a harness/host falsifier, not a candidate comparison.

| Same-binary cell | Median B-vs-A goodput | Observed goodput range | Median CPU/op | Maximum p99 delta | Pairs crossing a frozen guard |
| --- | ---: | ---: | ---: | ---: | ---: |
| Client GET c8 | -0.66% | -3.10% to +1.47% | +14.42% | +3.37% | 3/5 |
| Client PUT c8 | -3.40% | -6.35% to +3.57% | +4.85% | +150.94% | 3/5 |
| Raw embedded GET c8 | +1.74% | -7.53% to +37.47% | +4.76% | +42.86% | 4/5 |
| Typed embedded GET c8 | +1.44% | -8.45% to +15.80% | +0.95% | +73.33% | 2/5 |

Every A/B workload hash, binary hash and exact-result check matches. The same binary therefore
crosses the 2% goodput or 3% CPU/allocation/p99 guards in 12 of 20 pairs on this Windows host. This
does not retroactively accept the rejected W7 artifact: the frozen method correctly rejected the
observed candidate comparison. It does show that this local host/tool posture cannot causally assign
the independent native deltas to the batch seam. A second W7 candidate is blocked until the native
guard is resolvable on a stable admitted host or by a preregistered lower-noise measurement method;
the thresholds are not widened after observing this result.

## W8 identity, session and value ownership attribution

The v3 ownership screen is retained as falsified exploratory evidence. Its GET validator rebuilt the
expected payload as a fresh `Vec` inside the allocation window, adding one artificial payload-sized
allocation to every surface. No v3 allocation number is used below. The v4 tool validates the same
deterministic bytes without allocating an expected value; 72 exact-response receipts cover raw
embedded, typed embedded and direct client-surface GET/PUT at 0 B, 64 B, 4 KiB and 1 MiB, with three
independent processes per cell. The 1 MiB cells use 500 operations and remain owner-ranking data,
not timing claims.

Median gross allocation per operation was:

| Operation | Payload | Raw embedded B/op | Typed embedded B/op | Client surface B/op |
| --- | ---: | ---: | ---: | ---: |
| GET | 0 B | 104.24 | 104.24 | 227.40 |
| GET | 64 B | 104.24 | 168.24 | 291.40 |
| GET | 4 KiB | 104.24 | 4,200.24 | 4,323.40 |
| GET | 1 MiB | 663.29 | 1,049,239.29 | 1,049,358.42 |
| PUT | 0 B | 2,443.61 | 2,475.61 | 227.40 |
| PUT | 64 B | 2,531.61 | 2,779.61 | 291.40 |
| PUT | 4 KiB | 6,563.61 | 22,939.61 | 4,323.40 |
| PUT | 1 MiB | 1,051,604.44 | 5,245,900.44 | 1,049,358.42 |

Pointer fixtures at all four payload sizes establish the ownership boundary. Raw embedded
`put_encoded` moves an immutable `Bytes` owner into the cache and `get_encoded` returns a cheap
clone with the same backing allocation. Direct ClientSurface PUT also moves its request `Vec` into
the store without copying. Direct ClientSurface GET clones the complete stored `Vec`; its exact
`get_value_bytes_cloned` counter equals the payload in every measured operation, and the returned
value remains valid after replacement. Typed embedded GET necessarily owns a decoded `Vec`, while
typed PUT additionally pays codec serialization growth. W8b is therefore `measured-no-win` for raw
embedded, expected codec ownership for typed embedded, and an authorized isolated owner for direct
ClientSurface GET. Any candidate must preserve immutable response lifetime and cannot claim typed
or network zero copy.

The v5 identity screen enables real `ConsumerIsolation` with a bounded tenant roster and namespace
quota. Three processes per cell cover GET/PUT, 64 B/4 KiB and c1/c8. The profile times the existing
identity and protocol validation without bypassing admission:

| Cell | Identity validation ns/op | Protocol validation ns/op | Audit context B/op |
| --- | ---: | ---: | ---: |
| GET 64 B, c1 | 88.49 | 34.27 | 26.78 |
| GET 64 B, c8 | 12,820.68 | 53.65 | 26.78 |
| GET 4 KiB, c1 | 89.14 | 33.91 | 26.78 |
| GET 4 KiB, c8 | 15,933.37 | 54.36 | 26.78 |
| PUT 64 B, c1 | 88.32 | 33.95 | 26.78 |
| PUT 64 B, c8 | 19,131.84 | 52.77 | 26.78 |
| PUT 4 KiB, c1 | 86.87 | 33.61 | 26.78 |
| PUT 4 KiB, c8 | 21,514.22 | 52.69 | 26.78 |

The c8 values are aggregate waiter time and include contention on the isolation mutex; they are not
wall-clock latency. At c1, identity plus protocol validation is below the 20% CPU floor. At c8,
repeated roster resolution is a material contended owner and authorizes only the W8a
generation-fenced session experiment. Per-request quota, admission, audit, deadlines, request ids
and mutation publication remain outside the session. W8a and the ClientSurface value-ownership
candidate remain separate hypotheses and must be benchmarked independently.

All v4 receipts bind profile `w1-w8-native-api-profile-074-v4` to source
`49075afbaa057d36af0186890b81b95ec7d7fbb6`; all isolated v5 receipts bind
`w1-w8-native-api-profile-074-v5` to source
`b4ea64ce6f1ade0c21615866f714bef19d087828`. Every receipt is local, non-promotable and carries
exact-result validation.

### W8a generation-fenced session candidate and rollback

The isolated W8a candidate added a `VerifiedClientSession` bound to one identity, tenant,
namespace and protocol version. Session creation validated that binding once. Matching requests
compared an acquire-loaded policy generation and then entered the existing canonical request core;
deadlines, idempotency, admission, quota, audit, protocol operation checks, request ids, store
ownership and mutation publication remained per request. A generation change forced revalidation,
while another namespace/version fell back to ordinary dispatch. Test-only policy replacement
proved revocation and restoration at the next request boundary, and `Send + Sync`, audit/result
parity, cancellation, quota, expiration and concurrency suites passed.

An instrumented c8/64-B GET owner check reconciled the mechanism exactly: ordinary ClientSurface
performed 100,000 identity validations, 100,000 protocol validations and 100,000 store
acquisitions; the session performed 0, 100,000 and 100,000 respectively. Identity time fell from
13,930.23 ns/op to zero in the steady-state window. This did not bypass request admission or the
store lock.

The v6 screen used one exact binary and compared ordinary ClientSurface with VerifiedSession under
the same tenant roster, operation, payload, key space, seed and result validator. A longer run then
used five counterbalanced pairs per cell, 500,000 operations, 20,000 warmup operations, c1/c8 and
64-B/4-KiB payloads with instrumentation off:

| Cell | Median paired goodput | Observed goodput range | Median CPU/op | Median p99 | Allocation |
| --- | ---: | ---: | ---: | ---: | ---: |
| GET c1, 64 B | +3.36% | -8.57% to +4.60% | -4.65% | 0.00% | -3.21% |
| GET c1, 4 KiB | -0.55% | -5.00% to -0.32% | +3.49% | 0.00% | -0.25% |
| GET c8, 64 B | +46.35% | +12.74% to +62.02% | -70.00% | +8.84% | -3.19% |
| GET c8, 4 KiB | +24.88% | +13.98% to +34.32% | -37.08% | -5.45% | -0.25% |
| PUT c1, 64 B | +3.40% | +2.50% to +4.49% | -3.33% | 0.00% | -2.32% |
| PUT c1, 4 KiB | +1.99% | -9.29% to +3.09% | +1.23% | 0.00% | -0.24% |
| PUT c8, 64 B | +56.77% | +44.31% to +68.19% | -66.25% | -21.39% | -2.31% |
| PUT c8, 4 KiB | +26.35% | -3.13% to +36.41% | -51.44% | -23.47% | -0.24% |

W8a clears the 20% CPU/op floor in every c8 median and confirms that the second isolation lock is a
real owner. The exact candidate still fails the complete local contract: c8 GET 64 B has +8.84%
median p99, and c1 GET 4 KiB has +3.49% CPU/op. Local same-binary tail noise is already known, but
the guard is not weakened after observation. The candidate and its profiler surface were reverted;
all 130 exact v6 receipts remain as non-promotable evidence. A future session candidate needs a
stable admitted host and a production policy-generation lifecycle before acceptance.

W8b remains separate. Raw embedded storage is already immutable `Bytes` with shared backing, and
typed/network responses necessarily own encoded bytes. Direct ClientSurface GET still clones its
`Vec`, but eliminating that clone would change the stable `ClientResponse::Value<Option<Vec<u8>>>`
protocol ownership contract or create a second operation core. Neither is authorized by the local
owner data, so no W8b product mutation was manufactured.

## Next step and open risks

W3, W7 and the first W8a candidate have settled as negative product results despite confirmed
owners. W8a requires a stable admitted host plus a real policy-generation lifecycle before another
candidate. W8b is closed as raw `measured-no-win` and direct-surface protocol-ownership debt, not
permission to fork semantics. The rejected results are not permission to combine hypotheses or
weaken native guards. W6b remains unauthorized.

W0, kernel-syscall attribution, final instrumentation overhead, HC/1 and HC/2 process controls,
Redis same-box comparison, persistence cells, and all expensive release qualification remain open.
