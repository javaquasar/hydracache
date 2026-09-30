# A Benchmark Got 49% Faster—and the Optimization Still Failed

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Draft.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- [Part 2: Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d)
- [Part 3: TTL Is Not Enough](https://medium.com/@artur.buzov/ttl-is-not-enough-ec4e96d89546)
- [Part 4: Local-first Distributed Invalidation](https://medium.com/@artur.buzov/local-first-distributed-invalidation-87bf0249e935)
- [Part 5: Typed Query Caching in Rust](https://medium.com/@artur.buzov/typed-query-caching-in-rust-aac4352599f0)
- [Part 6: How to Measure Cache Performance Without Measuring Noise](https://medium.com/@artur.buzov/how-to-measure-cache-performance-without-measuring-noise-926c10d713f5)
- Draft: Optimization Starts With Measurement: Reading Allocations, RSS, and Runtime Together
- Draft: A Benchmark Got 49% Faster—and the Optimization Still Failed

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

An eight-key `DEL` benchmark got about 49% faster.

CPU time per operation fell by roughly a third. Gross allocation fell by nearly half. The p99 tail
improved. A logical operation that previously crossed the request dispatcher and acquired the store
lock nine times now did each once.

We reverted the change.

That decision sounds irrational only if an optimization is defined as a patch that makes its own
benchmark faster. A product optimization has a stricter definition: it must remove the intended
cost, preserve semantics, and keep the rest of the product inside its predeclared regression budget.

The `DEL` candidate passed the first two conditions and failed the third. Independent native API
controls regressed on the exact candidate binary. The code paths did not call the new batch method,
but users run binaries, not arguments about which functions ought to be unaffected.

This article describes the local investigation behind HydraCache 0.74: which RESP pipeline costs
were real, which attractive ideas were too small to matter, where the largest measured opportunity
remains, and why the strongest focused result was still a negative product result.

The numbers below are local development evidence. They are not release claims, capacity guidance,
or a comparison with Redis.

## Define failure before measuring success

HydraCache exposes several performance surfaces that share implementation layers but do not measure
the same work:

- RESP adds parsing, translation, response encoding, buffering, and transport behavior;
- the direct client surface adds admission, authorization, request dispatch, and store ownership;
- the typed embedded API adds codec work;
- the raw embedded cache is the narrowest local data-path control.

If we measured only RESP, we could make the protocol path look better by moving work into a shared
layer and never see the transfer. If we measured only the embedded cache, we would miss the costs
that motivated the release.

The comparison matrix therefore kept these surfaces separate. It also froze the workload before
candidate results existed: operation mix, payload, key set, seed, duration, concurrency, pipeline
depth, exact response validation, and semantic expectations all had to match. Candidate and
baseline processes ran in counterbalanced order, and every receipt carried source, binary, and
workload identities.

The local contract had two kinds of gates:

1. An affected cell had to improve enough to justify the complexity. Many candidates needed at
   least a 20% gain in their registered primary metric.
2. Unaffected cells could not lose more than the frozen goodput, CPU, allocation, or tail-latency
   budgets.

The second gate is why an impressive benchmark can still fail.

## First find the owner

Before changing code, we added local profiles for `GET`, `SET`, `MGET`, `MSET`, `DEL`, `EXISTS`,
pipeline depths 1 and 10, and several concurrency levels. The tools measured goodput, p50/p95/p99,
CPU per operation, gross allocation, copied bytes, high-level writes and flushes, dispatcher and
store-lock acquisitions, lock wait and hold time, RSS, and retained memory where the platform could
observe them honestly.

The first RESP attribution found two obvious mechanical costs in pipeline-10:

| Operation | Moved input bytes/op | Writes/op | Flushes/op |
| --- | ---: | ---: | ---: |
| GET | 265.5 | 1.0 | 1.0 |
| SET | 1,453.5 | 1.0 | 1.0 |

After every decoded command, the input buffer moved its unread suffix. Every reply also made one
high-level write and one explicit flush. These are real costs, but identifying a cost does not tell
us how much end-to-end performance it owns.

That distinction drove every experiment that followed.

## Removing all input copies was not enough

The first candidate replaced repeated input-buffer compaction with a cursor. For complete
pipeline-10 batches, moved suffix bytes fell to zero and compactions fell from one per command to
zero. Mechanically, the hypothesis worked perfectly.

The product result was small:

| Cell | Goodput change | CPU/op change | p99 change |
| --- | ---: | ---: | ---: |
| GET, pipeline 1 | -0.78% | +4.30% | 0.00% |
| SET, pipeline 1 | -2.45% | 0.00% | 0.00% |
| GET, pipeline 10 | +2.34% | 0.00% | -7.55% |
| SET, pipeline 10 | +1.18% | 0.00% | -1.04% |

The affected deep-pipeline cells gained only about 1–2%. Meanwhile, the unaffected pipeline-1
control crossed the frozen goodput or CPU budget. We did not reinterpret a 100% reduction in the
mechanical counter as an end-to-end win. The candidate was reverted.

The useful finding was not that cursor parsing is bad. It was that suffix movement is not the
dominant cost in the measured product path. A future cursor design would need a cheaper state
transition, but even a perfect implementation has shown a modest local ceiling for this workload.

## Ten replies became one write—and only SET cared

The next candidate accumulated a bounded connection-local response batch. It flushed on protocol
boundaries, errors, `QUIT`, subscription transitions, input exhaustion, or explicit reply and byte
limits. Tests covered exact bytes, ordering, partial writes, malformed input, slow readers, and an
oversized legal reply.

For pipeline-10, writes and flushes fell from 1.0 to 0.1 per reply: exactly the intended 90%
reduction.

The four product cells told four different stories:

| Cell | Goodput change | CPU/op change | p99 change |
| --- | ---: | ---: | ---: |
| GET, pipeline 1 | +13.44% | -5.84% | -18.52% |
| SET, pipeline 1 | -1.29% | +1.47% | +5.56% |
| GET, pipeline 10 | +3.06% | -1.26% | -11.74% |
| SET, pipeline 10 | +28.27% | -32.20% | -31.85% |

Deep-pipeline `SET` clearly benefited in the local screen. Deep-pipeline `GET` did not reach the 20%
floor, and pipeline-1 `SET` exceeded the p99 regression budget. The repeat ranges were also wide,
which is a reason to narrow a claim, not select the favorable samples.

The fixture used an in-process duplex transport. It proved fewer high-level write and flush calls;
it did not prove fewer kernel syscalls on a real TCP server. The bounded batch was therefore
reverted rather than promoted as a network optimization.

This idea still earned a better follow-up. Unlike unconditional coalescing, an adaptive policy can
target deep write-heavy pipelines while leaving shallow traffic alone. But it must first show the
same effect at the syscall layer and preserve fairness for slow readers.

## Estimate the ceiling before writing the patch

Several plausible ideas never became product candidates because stage attribution showed that they
could not clear the registered improvement floor.

For a pipeline-1 `GET`, response encoding allocated 264 bytes per operation—only 6.67% of the matched
end-to-end allocation. For `SET`, encoding allocated 8 bytes, or 0.19%. Translation owned roughly
10% in both cells. Even removing these allocations perfectly could not produce the required 20%
isolated result.

Key representation looked more dramatic. A 28-byte binary RESP key became a 72-byte canonical
segment, and materializing its stable lookup form allocated another 72 bytes. That is a 157%
representation expansion, but the materialization itself was only 1.82% of end-to-end `GET`
allocation and 1.74% of `SET` allocation.

Changing canonical key identity would affect compatibility, persistence, rollback, and possibly
tenant isolation. A large percentage attached to a small owner does not justify that migration.
HydraCache kept the existing representation for 0.74.

This was not “no result.” It saved two risky implementations:

- a direct encoder whose measured ceiling was too low;
- a binary key migration whose product-wide risk was much larger than its hot-path share.

Profiling is valuable when it tells us where not to work.

## A read lock that made concurrent reads worse

The native profile found a more promising owner. At one client, a live `GET` waited about 35
nanoseconds for the client-surface store mutex. At eight clients, aggregate wait rose to about 14.8
microseconds per operation while the measured critical section held the lock for about 1.0
microsecond.

That observation authorized one narrow experiment: replace the mutex with an `RwLock`, let live
unisolated reads take shared ownership, and retain exclusive ownership for expiration cleanup and
every mutation.

Single-client `GET` improved by 16.45%. Eight-client `GET` lost 3.74% goodput and consumed 234% more
CPU per operation. Eight-client expired `GET` lost 31.62% goodput because it paid for shared
ownership, released it, acquired write ownership, and revalidated the entry. Native controls also
crossed their guards.

The source-level story—“reads are shared, therefore read concurrency improves”—was weaker than the
measurement. The candidate was reverted. Store sharding was not automatically authorized; it is a
different architecture with different ownership, expiration, quota, and consistency risks.

## The 49% win

Multi-key attribution finally found a large, specific owner.

`MGET`, `MSET`, and `EXISTS` already crossed the client surface as one batch request and acquired the
store once. An eight-key `DEL` did not. It performed one batch lookup and then eight individual
invalidations: nine dispatcher crossings and nine store acquisitions for one logical operation.

The candidate added a bounded in-process batch-invalidation seam. RESP validated and deduplicated
the keys, then the client surface admitted identity and namespace once, removed live or expired
entries under one store lock, released tenant quota, and emitted ordered per-key events. The wire
protocol and semantic result did not change.

Five counterbalanced local pairs per cell used 20,000 measured operations, 2,000 warmup operations,
eight unique keys per command, a 256-byte payload, a 160,000-key space, seed 7,407, and exact
response validation.

| Eight-key DEL | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| Pipeline 1 goodput | 38,188 op/s | 56,350 op/s | +47.56% |
| Pipeline 1 CPU/op | 26,562 ns | 18,750 ns | -29.41% |
| Pipeline 1 allocation | 26,229 B | 13,483 B | -48.60% |
| Pipeline 1 p99 | 63 us | 43 us | -31.75% |
| Pipeline 10 goodput | 35,920 op/s | 53,622 op/s | +49.28% |
| Pipeline 10 CPU/op | 29,688 ns | 20,312 ns | -31.58% |
| Pipeline 10 allocation | 28,748 B | 16,002 B | -44.34% |
| Pipeline 10 p99 | 569 us | 393 us | -30.93% |

Dispatches and store acquisitions fell from nine to one. The primary metric cleared its floor by a
wide margin. Semantics and focused tests were green.

Then the independent controls rejected the binary.

## Why we reverted it

The first short native screen was too coarse for Windows process CPU accounting, so we increased it
to 500,000 operations with 20,000 warmup operations and ran five counterbalanced pairs per cell with
instrumentation disabled. The workload hashes and exact-result validation matched.

The native contract required at least 98% of baseline goodput and tight CPU, allocation, and p99
bounds. Among the failures:

| Native control | Goodput change | CPU/op change | p99 change |
| --- | ---: | ---: | ---: |
| Client-surface PUT, 8 clients | -3.95% | +3.00% | +159.81% |
| Raw embedded GET, 8 clients | -1.04% | 0.00% | +21.05% |
| Typed embedded GET, 8 clients | -7.10% | +2.10% | +92.31% |

Those operations did not call batch invalidation. That makes the result surprising; it does not
make the result disposable. The exact built artifact failed the predeclared release contract.

We retained the favorable `DEL` receipts, the unfavorable native receipts, and the explanation. We
did not relax the thresholds, combine hypotheses, or discard controls as “noise.” Then we reverted
only the batch seam and its candidate-specific tests.

The result should not be read as proof that batch invalidation caused every native delta. Local
scheduler and binary-layout effects remain possible. It is proof of something narrower and
operationally sufficient: this exact candidate was not stable enough to accept under the frozen
method.

## Which ideas survived the investigation

No product optimization from this sequence has been accepted yet. But the work did produce a much
smaller and better-supported design space.

Three mechanisms earned another iteration:

1. **Batch multi-key deletion is the strongest measured owner.** Removing repeated framework and
   store acquisition work produced the largest focused gain. The next design must isolate the batch
   seam more carefully and repeat the native controls on the exact binary.
2. **Output coalescing is valuable for deep write-heavy pipelines.** It removed 90% of high-level
   write and flush calls and materially improved pipeline-10 `SET`. The follow-up should be adaptive,
   should preserve shallow-request latency, and must measure real TCP syscalls.
3. **Identity, session, and value ownership deserve attribution.** They cross protocol and native
   paths and may own enough repeated work to matter. They should be measured separately before a
   verified-session or zero-copy candidate is authorized.

Four attractive directions did not earn more implementation work in this cycle:

- unconditional cursor parsing, because copy removal barely moved end-to-end throughput;
- direct response encoding, because its measured allocation ceiling was too low;
- canonical key migration, because its end-to-end share was small and compatibility risk was high;
- a general `RwLock` or immediate store sharding, because the narrow read split regressed the cells
  it was intended to improve.

This ranking is more valuable than a bag of speculative patches. It says where a future gain is
likely to come from and which ideas should stay closed until the workload changes.

## The practical rule

For every performance candidate, preserve four separate statements:

1. **Owner observation:** a measured component consumes a specific resource.
2. **Mechanical result:** the candidate reduced that component.
3. **Product result:** the affected workload improved under matched conditions.
4. **Release result:** independent surfaces, semantics, and long-run guards also passed.

The cursor candidate reached statement two. Coalescing reached three for one important cell. Batch
`DEL` reached three convincingly. None reached four.

Collapsing those statements is how local observations become exaggerated release claims.

## Conclusion

The most promising HydraCache 0.74 gain is no longer vague. It is repeated request-framework and
store-ownership work in multi-key deletion, followed by write/flush amplification in deep `SET`
pipelines. The measurements also tell us what is unlikely to pay off: key representation churn,
direct encoding in isolation, and a generic shared read lock.

The 49% result was not wasted because its code was reverted. It identified a real owner, validated
the size of the opportunity, exercised the semantic test surface, and exposed the need for stronger
isolation from native paths.

An optimization campaign succeeds when it narrows uncertainty without moving the rules. Sometimes
the most useful outcome is a faster benchmark and a rejected patch.
