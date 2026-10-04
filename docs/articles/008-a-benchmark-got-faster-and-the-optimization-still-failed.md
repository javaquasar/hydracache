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

A later same-binary A/A falsifier made that distinction concrete. Both labels used the same source,
executable, workload hash and exact-result checks, yet 12 of 20 c8 pairs still crossed at least one
frozen native guard. Client PUT showed a median 3.40% goodput difference between identical binaries;
raw and typed GET pairs had even wider ranges. This did not turn the rejected patch into an accepted
one. It showed that the local Windows posture could reject but could not reliably assign causality
at those thresholds. The next batch candidate needs a stable admitted host or a preregistered
lower-noise method—not a threshold widened after the result.

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

That attribution later separated two owners. Raw embedded values already preserve the same
immutable `Bytes` allocation across PUT and GET, so there is no raw zero-copy patch to invent.
Typed access necessarily owns codec output. The direct client surface, however, moves its request
`Vec` into storage on PUT and clones the complete payload on every GET. With tenant isolation
enabled, repeated roster validation also became a contended c8 owner. These observations authorize
two experiments—not one combined fast path: generation-fenced identity reuse and, separately, an
immutable client-value representation.

Real loopback TCP later made the output-coalescing lesson sharper. The baseline performed one
server `poll_write` and one explicit `poll_flush` per reply even at pipeline 10. The bounded batch
reduced those socket-layer calls by 90% and produced median paired TCP goodput gains of 80.90% for
GET and 83.01% for SET, while roughly halving CPU/op and p99. Yet the same binary failed the
pipeline-1 contract in four of five GET pairs and four of five SET pairs: GET lost 2.78% median
goodput with 3.73% more CPU/op, and SET p99 rose 8.20%. The candidate was reverted again. The next
W3 design has a precise requirement: retain the deep-pipeline batch without charging shallow
requests for it.

Those counters end at Tokio's socket boundary. An attempted Windows network trace failed before
capture because local policy denied system-performance profiling. We recorded that limitation
rather than relabeling socket polls as syscalls or escalating privileges. Kernel syscall claims
remain open for an admitted Linux or ETW-capable host.

The verified-session experiment repeated the pattern at the native boundary. Under c8 tenant
isolation it removed the second roster lock and every repeated identity validation while retaining
protocol validation, admission, quota and store ownership. Median paired goodput rose 46.35% for a
64-byte GET and 56.77% for a 64-byte PUT; CPU/op fell 70.00% and 66.25%. But the GET p99 median rose
8.84%, and a c1 4-KiB GET crossed the CPU guard at +3.49%. The full semantic suite passed, yet the
candidate was reverted because owner removal is not the same as release acceptance.

Value ownership also produced a useful non-change. Raw embedded values already share immutable
`Bytes` backing. Direct ClientSurface GET copies because the stable protocol response owns a
`Vec<u8>`. Removing that copy locally would either change the protocol ownership contract or fork
the canonical request core, so the investigation recorded the boundary instead of inventing a
zero-copy wrapper.

## What the local campaign actually delivered

The final local branch contains attribution tools, exact receipts, regression coverage and the
decision record—not an optimization left enabled merely because one benchmark became faster. Both
the response batch and verified session were committed as isolated candidates, measured, and then
removed with targeted revert commits. The resulting product path is the clean baseline; the
instrumentation and evidence remain reproducible.

The semantic checks covered more than happy-path GET and SET:

- fragmented RESP2 and RESP3 frames and partial pipelines;
- scripted partial writes, `Pending` transitions and response ordering;
- half-open connections, disconnects, churn and bounded recovery;
- slow readers, subscription lag and backpressure;
- oversized input and large legal responses;
- expiration versus concurrent overwrite;
- tenant quota release and isolation;
- batch prevalidation, duplicate-key behavior and multi-key atomicity;
- exact response bytes and result validation in every performance receipt.

The complete client-surface run passed 72 active tests; two million-operation retention soaks
remained deliberately scheduled rather than being smuggled into a quick local gate. The focused
multitenancy suite passed 15 tests, RESP boundaries 11, connection chaos 4, and pipeline
attribution 2. Both local profilers passed their unit and clippy gates after the product candidates
were reverted.

This work also established a strict measurement boundary. The TCP profiler observes Tokio
`AsyncWrite` polls, ready/pending outcomes, short writes, bytes and explicit flush polls. It does
not observe kernel `send` calls. Windows Performance Recorder refused the local network profile
with error `0xc5585011`, so no trace existed to reinterpret. Expensive infrastructure, the rented
server and the frozen 0.73 confirmation run were left untouched.

For reproducibility, the main checkpoints were:

| Checkpoint | Commit |
| --- | --- |
| Real TCP transport | `a7942461` |
| Server socket-poll attribution | `34168994` |
| W3 TCP evidence and rejection | `ddfabc79` |
| Verified-session candidate and profiler | `4f0beed2`, `7204599c` |
| Verified-session targeted rollback | `39d2d242`, `2471f84a` |
| W8a evidence and final local decision | `81a63257` |

No expensive qualification number should be inferred from those local medians. The next W3 or
W8a attempt needs a stable admitted Linux or ETW-capable host. W8a additionally needs a real
production policy-generation and revocation lifecycle; W8b needs an explicit decision about
whether changing the stable `ClientResponse::Value` ownership contract is acceptable. Until those
decisions are made, widening thresholds or combining the session, value and RESP hypotheses would
reduce evidence quality rather than advance the release.

Four attractive directions did not earn more implementation work in this cycle:

- unconditional cursor parsing, because copy removal barely moved end-to-end throughput;
- direct response encoding, because its measured allocation ceiling was too low;
- canonical key migration, because its end-to-end share was small and compatibility risk was high;
- a general `RwLock` or immediate store sharding, because the narrow read split regressed the cells
  it was intended to improve.

This ranking is more valuable than a bag of speculative patches. It says where a future gain is
likely to come from and which ideas should stay closed until the workload changes.

## What we completed while waiting for 0.73

Waiting for the predecessor confirmation did not justify another speculative product patch. It
did leave seven useful local tasks, all of which could be completed without touching the frozen
0.73 candidate or expensive infrastructure.

First, the local measurement harness now enforces five independent ABBA pairs, mandatory warm-up,
process affinity, process priority and a frozen background-CPU ceiling. Same-binary variation is
used to derive a minimum detectable effect; an observed delta below that value is inconclusive.
The first real run demonstrated the fail-loud behavior: all ten processes completed and placement
was applied, but background CPU was 28.91-69.53% against a 20% ceiling. The series was invalidated,
no A/A number was accepted, and the ceiling was not widened after seeing the result.

Second, the native profiler now measures the previously open local owners. In the c8 GET receipt,
audit context copied 26.89 bytes per operation, request/expiry clocks consumed 364.26 observed
nanoseconds per operation, and canonical store-key construction copied 66 identity bytes. An
eight-item Batch GET copied 528 canonical key bytes but still used one dispatch and one store lock
for the whole batch. This last result matters: the native client surface already has a vectorized
batch core, so building another one would not remove the measured owner.

The profiler also counts the `SeqCst` operations by purpose. A GET performs one dispatch-counter
operation, two monotonic clock fences and one expiry-gate operation. Batch PUT replaces the second
clock fence with a mutation-counter operation. These counts establish frequency, not the cost of a
different memory ordering; no atomic was relaxed without a happens-before proof.

Third, adversarial transport coverage no longer depends on the reverted coalescing candidate. A
retained baseline test stream fragments input to one byte, forces `Pending`, performs one-byte
writes, blocks one connection behind a closed write gate, disconnects during a partial reply and
returns a 64-KiB value through repeated short writes. The blocked connection applies backpressure
after its first committed command while a second connection continues to make progress.

Fourth, ADR-0023 records the two W8 architecture boundaries. Reusable verification requires an
authoritative policy generation whose publication ordering is part of the isolation API;
connection lifetime is not revocation. A zero-copy native GET must be additive or versioned around
immutable `Bytes`; the existing `Vec<u8>` response cannot silently change ownership. The ADR is a
proposal and does not resurrect W8a or authorize W8b.

Fifth, the focused tests were repeated under local Ubuntu WSL2 with a separate target directory.
The native attribution suite passed 3/3 and the adversarial transport suite passed 4/4. WSL2 had no
`perf` or `strace`, and its nightly toolchain did not match the frozen release toolchain, so this is
portability evidence only—not a Linux performance or syscall claim.

Sixth, W11 now has a content-addressed qualification manifest and a dry-run validator. It checks
phase order, workload-contract hashes, same-box Redis requirements and artifact naming while
keeping every expensive phase `not-run`. The tool deliberately has no execution mode until the
exact 0.74 candidate, admitted host, Redis binary identities, qualification runner and explicit
authorization are available. The 0.73 tag and confirmation are no longer blockers.

Finally, the semantic suite gained concurrent batch, expiry and authorization regressions. Batch
readers observe either all old values or all new values, never a torn mix. Sixteen simultaneous
readers remove a due batch exactly once and see only misses. Authorized and forged callers sharing
the same client id cannot leak tenant authorization between requests; every forged call is denied
and audited.

These additions do not change the release conclusion. They improve the ability to reject a noisy
series, locate a native owner, reproduce a transport edge case and start qualification safely. The
product path still contains no accepted 0.74 optimization.

## What changed after 0.73 shipped

Publication closed one identity question without changing the measured baseline. The annotated
`v0.73.0` tag points to release commit `d1db9937`, while the confirmed product runtime remains
byte-equivalent to measured candidate `16d2e98b`. W0 now pins both identities, the successful
24-hour confirmation run and immutable archive commit `570a5bcb`; it does not pretend the tag
commit and measured commit are the same object.

That closure allowed the remaining local follow-ups to proceed, but it did not improve the signal
quality of this Windows host. The first completed follow-up was the W9a atomic-ordering audit. We
classified every `SeqCst` operation in the measured RESP and direct client paths. Lifecycle,
message publication, monotonic time-floor and expiry-claim atomics kept their stronger ordering.
Only independent diagnostic counters were eligible for a `Relaxed` screen.

The preregistered screen used seven counterbalanced pairs and required at least 2% lower CPU/op
with six of seven pairs favoring `Relaxed`. At concurrency 1 both medians were exactly 7.03125
ns/op and zero pairs favored `Relaxed`. At concurrency 8 both medians were 142.578125 ns/op and
only four pairs favored it. The screen failed before product mutation, so no atomic was changed.
This is another useful negative result: a weaker ordering can be semantically defensible and still
not be worth carrying.

The other post-publication work improved the qualification control plane rather than the product
hot path. The long-run resilience contract now distinguishes controller loss from progress loss
and measurement loss. A new Rust foundation writes canonical hash-chained checkpoints, syncs the
journal before atomically replacing its head, and rejects sequence gaps, middle corruption,
digest breaks, identity drift and timestamp reversal. Recovery tolerates at most one incomplete
trailing JSON record; appending after that tail is forbidden until explicit recovery archives it.

A separate `xtask` verifier reimplements the digest calculation instead of trusting the writer.
It now validates strict packet and raw-manifest schemas, canonical campaign identity, the exact
sorted raw file set, every file size and SHA-256, required guard evidence, role/process identities,
and each journal's byte length, raw SHA-256, record count, first record and hash-chain head.
Unlisted files, manifest disagreement, duplicate roles or guards, missing evidence, parent
traversal, symlink or hardlink substitution and any promotable packet without complete I74 and C74
journals are rejected. The pure supervisor state machine separately proves that attach can update
only controller lease and revision; host/boot, PID-start, cgroup, cpuset, checkpoint, lease,
failure, duplicate-executor or durable-history drift all fail closed. Its typed 64-KiB protocol
rejects unknown and duplicate fields, malformed packets and cross-operation fields, and exposes no
arbitrary command or environment surface.

The disposable GitHub-side monitor is also implemented as a strictly read-only observer. It
validates the full durable-state shape plus campaign and manifest identities, reports controller
loss separately from stale useful progress, measurement loss and evidence corruption, and writes
a create-new non-promotable receipt. Future progress timestamps are invalid rather than clamped to
zero. The observer has no start, attach, seal or abort action, so replacing it cannot change the
measured process lifetime.

Two security layers were then added without inventing production credentials. The Ed25519 verifier
hash-binds the raw authorization document to the request and checks operation, campaign, manifest,
repository, workflow run, actor, issue time, expiry, maximum ten-minute lifetime and clock skew.
The Rust manifest parser independently requires canonical strict JSON, checks its raw digest and
request identity, rejects unknown fields and plaintext-looking secret values, and freezes cadence,
progress, diagnostic, lease and artifact limits. The complete start-manifest data contract is now
implemented on both sides rather than left as a prose-only list. A strict JSON schema, the Python
builder and the independent Rust parser cover source commits, tree and Cargo.lock identities,
clean-tree assertions, installed binary path/hash/inode/device/owner/mode metadata, exact argv and
environment identity, ordered roles and phase durations, workload/statistics digests, output
limits, output-schema identities and final guard identifiers. Nested unknown fields, binary/argv
drift, dirty sources, duplicate guards and limits outside the campaign envelope are negative tests.
This narrows a future host failure to provisioning or execution rather than ambiguity in the
manifest format. The production public key remains explicitly `UNRESOLVED`; the test key can prove
verification behavior but cannot authorize a campaign.

The local supervisor model now also distinguishes activity from useful progress. Merely receiving
another checkpoint does not keep a run alive. Measured work must advance completed operations,
surface accounting and process CPU time together; drain must reduce outstanding work; intentional
post-work idle must advance telemetry while operation counts remain fixed; and reconciliation must
change its milestone, epoch or owner state. Tests retain decreasing RSS as a valid gauge rather
than misclassifying it as counter corruption, while identity drift, phase skips and true cumulative
counter regression remain failures. This proves the watchdog decision logic locally, not its
future integration with systemd or `/proc` on the admitted host.

W12 now has a fail-closed evidence skeleton listing all 29 plan items in exact release order. It
does not promote incomplete rows: the first local report contained 23 planned items, five locally
implemented items, zero fast-green items and zero ship-ready items. A release-scoped W11 canary
proved expected-red behavior at clean commit `38a8117e`; PID-only/restart/lease/duplicate
acceptance failed with the registered signature instead of being counted as an ordinary green
test. Later commits intentionally require a fresh exact-commit canary receipt.

This is deliberately not described as a completed resilient qualification system. The live Unix
`SOCK_SEQPACKET` service, OS peer-credential and supplemental-group checks, production verification
key, systemd process ownership, installed-file verification, provisioning, bounded diagnostics,
deterministic archive creation and real controller-loss rehearsals still require implementation or
Linux-host evidence. No six-hour or 24-hour 0.74 run has started.

Finally, W10 now has an explicit zero-candidate composition ledger. Because W2, W3, W6a, W7, W8a
and W9a were rejected and the other proposals are deferred or unauthorized, there is no honest
C74 to freeze. Tooling progress is not a substitute for a product candidate, and isolated local
percentages are not added together.

After publication we also tested the most conservative-looking W3 retry: leave pipeline-one on the
old response path and activate coalescing only when a second complete frame is already present in
the current read buffer. An 18-cell pre-candidate TCP baseline confirmed the owner mechanically:
every cell performed exactly one server write and one explicit flush per response. The new shallow
and partial-frame guards passed, and ten buffered replies remained byte-exact and ordered.

The candidate still failed before performance measurement. With the first connection's write gate
closed, it committed two SET mutations before the first response made write progress; the retained
baseline guard permits one. The buffering was bounded, but it nevertheless moved backpressure
from the socket boundary to already-executed commands. The product diff was removed, no candidate
SHA was created, and no numeric win was claimed. This closes the simple "detect a deep read, then
batch" design: preserving pipeline-one latency is insufficient if slow-reader admission changes.

## The practical rule

For every performance candidate, preserve four separate statements:

1. **Owner observation:** a measured component consumes a specific resource.
2. **Mechanical result:** the candidate reduced that component.
3. **Product result:** the affected workload improved under matched conditions.
4. **Release result:** independent surfaces, semantics, and long-run guards also passed.

The cursor candidate reached statement two. Coalescing, batch `DEL`, and the verified session
reached statement three in their target cells. None reached four.

Collapsing those statements is how local observations become exaggerated release claims.

## Conclusion

The most promising HydraCache 0.74 gains are no longer vague. They are repeated request-framework
and store-ownership work in multi-key deletion, write/flush amplification in deep pipelines, and
repeated identity resolution under tenant-isolated concurrency. The measurements also tell us what
is unlikely to pay off: key representation churn, direct encoding in isolation, a generic shared
read lock, or a zero-copy wrapper that cannot preserve the stable protocol contract.

The 49%, 83%, and 57% target-cell results were not wasted because their code was reverted. They
identified real owners, bounded the opportunity, exercised the semantic surface, and exposed the
specific shallow-request, tail-latency, host-stability, and policy-lifecycle work still required.

An optimization campaign succeeds when it narrows uncertainty without moving the rules. Sometimes
the most useful outcome is a faster benchmark and a rejected patch.
