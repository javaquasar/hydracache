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

## Making the long test as strict as the benchmark

The release work exposed a second kind of performance problem: a six-hour or 24-hour measurement
is not useful if losing the CI controller also loses the only trustworthy account of what ran. The
solution was not to make a dead benchmark resumable. It was to move process lifetime and evidence
ownership onto a narrowly constrained host supervisor, while keeping identity, workload, seed,
duration and thresholds frozen outside it.

That distinction matters. An attach may observe the original process, but it may never spawn a
replacement and call the combined samples one run. Durable checkpoint records form a hash chain
and retain boot, PID-start, process-group, cgroup, harness and daemon identity. Terminal admission
requires the original retained systemd unit, a terminal checkpoint and consistent process
identity. A crash between an event append, state update, packet build and response can be replayed;
a different request or different evidence cannot be substituted.

I74 now seals into an immutable continuation packet. The final C74 packet records the SHA-256 of
that exact canonical I74 manifest. Offline verification takes both manifests, recomputes the
digest, checks campaign identity and compares the complete I74 role manifest. A 64-character value
that merely looks like a digest is insufficient, and recapturing the I74 journal after it changed
does not produce a valid final result.

The packet input boundary is explicit too. Each role writes a canonical
`seal-input-inventory.json` naming its journal, every raw input and one evidence file for every
guard frozen before the run. The supervisor does not sweep a directory and hope that everything
inside is evidence. Unknown fields, unlisted files, cross-role paths, traversal, symlinks,
hardlinks and limit overflow fail closed. Before C74 can reuse I74, the resolver reopens the
read-only sealed packet and compares the original input bytes with the sealed copies.

Finally, packet and archive publication use create-new staging directories, sync their contents,
apply read-only Unix modes and atomically rename the completed trees. Recovery can adopt an exact
completed artifact or finish a verified rename; it cannot overwrite a conflicting artifact. These
mechanics do not make the local throughput numbers promotable, but they make it possible for a
future admitted-host run to survive controller loss without quietly changing the experiment.

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

Archive construction is now deterministic as well. The supervisor sorts normalized paths,
normalizes tar uid/gid/mode/mtime, compresses through a fixed single-stream zstd configuration and
writes an external digest over the exact archive bytes. Two trees created in different orders must
produce byte-identical archives; tests open the result and inspect every header. File and byte
limits are checked before output creation, and existing destinations, nested outputs, symlinks,
hardlinks and non-regular files fail closed. This is still library-level sealing: the live service
has not yet connected it to a terminal campaign state on the admitted Linux filesystem.

To avoid treating Windows-only behavior as proof, the complete supervisor package was then run
under local WSL2 Ubuntu at exact source `37566d71`. All 30 targeted tests passed, including Unix
directory sync, hardlink rejection, archive determinism, authorization, checkpoint recovery,
manifest parsing, protocol, state-machine and watchdog cases. The receipt is deliberately marked
local and non-promotable: no service or product process ran, and WSL is not the admitted host.

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

This was deliberately not described as a completed resilient qualification system. At that point
the live Unix service, OS peer checks, systemd ownership, provisioning, diagnostics, sealing and
controller-loss rehearsal were still missing. The next local slice reduced that list without
pretending to finish it.

The durable state is no longer only a pure transition model. A campaign store now holds an
exclusive file lock, parses strict bounded `state.json`, applies revision compare-and-swap, saves
`state.previous.json`, and replaces both snapshots through create-new temporary files, file sync,
rename and parent-directory sync. Stale revisions and manifest-identity drift leave the current
state byte-for-byte unchanged. A separate integrated-0.74 component writes the checkpoint chain:
it first asks a cloned phase watchdog to admit progress, persists and syncs the record, and only
then advances its in-memory sequence. A rejected sample therefore cannot create a gap or poison a
retry.

The socket boundary now exists as real Linux code rather than a schema test. It uses Unix
`SOCK_SEQPACKET`, rejects empty or oversized packets, preserves message boundaries, obtains
`SO_PEERCRED`, reads supplemental groups from the peer's `/proc` status, and removes its socket on
shutdown only if device and inode still match the socket it created. Mutating envelopes require a
canonical signed authorization bound to the request. Read-only status requires no signature but
still checks the exact uid/group, repository and actor admission.

The same binary now has `serve`, `request` and `verify` modes. The live server can return an exact
durable status snapshot and rejects stale revisions; the client independently checks the response
digest and request/campaign identity. WSL2 ran 43 supervisor tests and three integrated checkpoint
tests at source `1a29aef1`. This is still local, non-promotable evidence. The mutating operations
deliberately return error 11 after authentication because no systemd/process-identity adapter has
yet proved that they can spawn or attach exactly once. A production key, installed service,
diagnostics, sealing orchestration and controller-loss rehearsal remain unresolved. No six-hour or
24-hour 0.74 run has started.

One more local step made the service artifacts internally coherent. A `0660` socket is useless to
the runner if it remains `root:root`, so the listener now changes the socket to the exact admitted
client-group gid before listening and tests the inode's real gid. The staged systemd unit uses
`Type=notify`; the binary emits one bounded readiness datagram and supports both filesystem and
abstract Linux notify sockets. Sysusers and tmpfiles definitions create the unprivileged measured
account, client group and bounded state directories. The checked-in configuration template keeps
the repository id, uid/gid allowlists and production Ed25519 key deliberately invalid, so copying
the template cannot accidentally authorize a campaign.

Attach also needs more than “the PID still exists.” The Linux identity reader now parses start
ticks and process group from `/proc/<pid>/stat`, including command names with spaces and closing
parentheses, binds the host boot id, reads the single cgroup-v2 path and its inode, and verifies the
expected systemd unit component. A test changes every field at once and requires all mismatches to
be reported. At `882b3ed2`, 51 supervisor tests and three integrated checkpoint tests pass under
WSL2. The service files were not installed, systemd did not own a measured unit, and attach remains
closed until unit-state inspection, checkpoint validation and durable request replay are combined.

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

The next W11 slice addressed a different kind of false confidence: treating a durable state file as
proof that a controller mutation happened exactly once. The supervisor now records accepted and
rejected requests in a separate canonical hash chain and indexes stable request ids by the digest
of the canonical request. A retry with the same id and bytes receives the original signed response;
the same id with different bytes is rejected. The event is synced before the state snapshot is
replaced, and an accepted attach event contains the complete post-mutation state. A local crash
fixture stops in that exact window and proves that restart completes only the recorded
compare-and-swap. It also proves that same-revision content drift is not silently "repaired" from
a convenient copy.

Checkpoint admission was tightened at the same boundary. Attach evidence must now match the
campaign, active role, sequence, head digest, harness identity and daemon identity in durable state.
Both the journal and head must be bounded regular files. A complete JSON object without the final
newline is still classified as a torn write; accepting it would let the next append concatenate two
objects and corrupt the chain. Useful-progress timestamps from the future are rejected rather than
allowed to postpone the frozen deadline.

At source `35b2bc3f`, 65 supervisor tests and three integrated checkpoint-writer tests pass under
local WSL2. This is implementation evidence, not throughput evidence: no product process, rented
host or expensive qualification campaign ran. Live attach remains closed until exact systemd unit
state and the immutable host/manifest checks are wired into the same transaction.

The immutable-manifest half of that remaining admission is now concrete as well. The supervisor
re-parses the persistent canonical start manifest, verifies its separate digest file, and
deterministically reconstructs the frozen state identity from tooling, source/tree/lock, installed
binary, workload/statistics, phase/limit and host fields. At source `6759b322`, the WSL total is 66
supervisor tests plus three checkpoint-writer tests. Systemd unit state and live wiring remain the
deliberate fail-closed boundary.

That systemd boundary is now read-only executable code. A blocking D-Bus adapter resolves only the
fixed HydraCache unit namespace and binds active/running state, main PID, cgroup and service result
to the durable harness and daemon identities. During the first real local system-bus check, the
test exposed an incorrect assumption that `ControlGroup` belonged to the generic Unit interface;
on systemd 255 it is read from the Service interface. The adapter was corrected and the same real
test passed without creating, stopping or restarting a unit.

The live attach request path now composes replay recovery, persistent manifest, systemd, `/proc`
start identity, live cpuset, checkpoint chain and lease predicates under the campaign lock. A
rejected signed request is persisted and an identical retry returns the exact original response.
Lease admission deliberately remains impossible: every attempt includes the named
`host:full-receipt-revalidation-unimplemented` failure until mount, tuning and housekeeping state
can be re-derived from the admitted host receipt. At source `1a484f6c`, 72 ordinary supervisor
tests, one explicit real-system-bus test and three checkpoint-writer tests pass locally under WSL2.

The next slice replaced that named placeholder with evidence, not with a weaker predicate. A
root-only collector writes a canonical host observation and digest sidecar without overwrite. It
binds the existing reference-host freeze receipt; machine, boot, kernel and command-line identity;
the exact campaign mount and options from mountinfo; the full online/isolated/housekeeping CPU
partition; every online CPU governor; the seven frozen kernel tunables; and the installed
root-owned supervisor binary's digest and inode metadata. Attach re-collects the same shape and
requires exact agreement with the persistent receipt, immutable start manifest and durable state.

That change makes successful attach representable without making WSL look like an admitted host.
Missing CPU-frequency state, empty isolation, a non-root collector or any mount, tuning, binary,
machine or boot drift rejects the request. At source `93710227`, 78 ordinary supervisor tests pass
locally under WSL2, with Windows tests and strict clippy also green. This proves the local admission
mechanism, not a release campaign: no service or product process was started, and accepted attach,
controller-loss and reboot rehearsals still require the admitted bare-metal host.

The following start-path slice keeps evidence publication separate from execution. A revision-zero
start can import only the exact direct-child staging directory for its campaign. The importer rejects
symlink, hardlink, size, digest, manifest/receipt binding and pre-existing destination conflicts;
writes create-new `0400` copies into a private directory; fsyncs them; and atomically publishes the
campaign directory. An identical retry cannot overwrite the first publication. At source
`5343ca1f`, 82 ordinary supervisor tests pass locally under WSL2. That slice alone did not claim
exactly-once spawn; it established the immutable input boundary for the work that followed.

The following slices add a host-wide flock plus a persistent active-campaign marker, role-specific
spawn intent/result documents and lifecycle records in the same event chain. PREPARED and STARTING
contain no invented PID. The intent is durable before the backend side effect; once it exists,
recovery can only observe the deterministic unit name. An absent unit fails incomplete, an exact
unit is adopted, and identity mismatch or multiple executors quarantines the campaign. The signed
accepted response is also journalled before it is sent, so a lost reply is replayed exactly without
another backend call.

Finally, the normal live server path now owns a production-form systemd backend. Its unit policy is
not supplied by the request: it derives exact argv, a digest-bound minimal environment, CPU mask,
runtime and memory/FD/task limits, output paths and hardening properties from the admitted manifest
and compiled policy, then uses `StartTransientUnit` with mode `fail`. Observation requires one
MainPID and exactly one daemon with the original boot id, process group, cgroup path/inode and
cpuset. A second signed start is now accepted only at the exact `I74_SEALED` revision, reuses the
immutable manifest evidence without accepting a new staging path, and records independent C74
intent/result files. Its replay and lost-response paths cannot issue a second backend start. At
source `286863e4`, 114 ordinary supervisor tests pass locally under WSL2; Windows strict clippy
also passes.

No real transient unit or HydraCache process was launched for that result. The admitted-host
account/directory permissions, controller and supervisor restart, controller-loss reattachment,
overhead budget, seal/abort/diagnostics and lease-expiry termination remain unproved or
unfinished. The point is the same as for the performance candidates: implemented mechanics and
release evidence are different claims.

The next local sealing slice deliberately stopped before claiming a live seal. It made successful
termination provable from two sources: a final `Terminal` checkpoint with the original process
identities, and a retained systemd `active/exited` unit with zero MainPID, success result and exact
unit/cgroup identity. It also corrected controller-lease ownership: later operations compare the
repository/run/actor principals covered by the signature, not the inevitably different digest of
an earlier attach request. Finally, the host claim can be released only from a clean
`COMPLETE_SEALED` state. At source `f35e276c`, 117 ordinary WSL supervisor tests pass. Packet
assembly and the live seal route remain unfinished, so these predicates do not yet authorize an
artifact or release claim.

Packet assembly is now implemented as another separate boundary. It copies only explicit safe
relative sources into a create-new staging tree, derives the raw file set and guard digests from the
copied bytes, derives role chain metadata from the copied journals, writes canonical manifests,
syncs the tree and publishes it with one rename. A complete packet cannot hide a failed guard or an
incomplete role; a promotable packet must contain I74 and C74. Independent `xtask` verification
accepts generated continuation and final fixtures, while two clean builds produce the same packet
and compressed archive digests. At source `daf738cf`, the 117-test supervisor suite and six packet
verifier tests pass under WSL. The remaining distinction is deliberate: deterministic bytes exist,
but live `seal` still must make terminal observation, packet publication, response replay and host
claim release one recoverable transaction.

That distinction led to another isolated slice rather than a premature server route. Packet and
archive readers now verify their own published output, and the archive is exposed only by a synced
staging-directory rename. A durable seal-artifact intent binds the signed request identity, role,
packet plan and resource limits before copying begins. After a crash, an exact retry can adopt an
already verified final artifact, finish the rename of a complete `.building` artifact, or recover a
synced canonical JSON document whose digest sidecar was not yet written. It cannot reuse a different
request or plan, and every replay re-hashes the packet and archive before returning the original
result. At source `75e7df26`, 121 ordinary supervisor tests pass under WSL, including explicit
tamper, conflicting-intent and crash-window tests; the one real-system-bus test remains manual. This
is evidence for recoverable artifact mechanics, not yet for live sealing: terminal state/event CAS,
the protocol response, read-only publication and host-claim release still need one ordered server
transaction and admitted-host rehearsal.

The next slice supplied that ordered coordinator without hiding the remaining adapter. On Linux it
first proves the terminal checkpoint chain and retained successful unit snapshot, commits terminal
event before state, creates or recovers the exact packet/archive, commits sealed event before state,
then journals the signed response. A replay is accepted only for the same request digest and
re-hashes the artifacts. Final C74 sealing clears process/checkpoint/lease state before the response
and removes the host marker afterward; both state reconciliation and marker removal tolerate the
corresponding lost-response windows. I74 continuation and two-role final fixtures, unit drift,
request conflict and event-ahead-of-state recovery raise the WSL suite to 125 passing ordinary tests
at source `cba4a336`, with one real-system-bus test still manual. The claim remains deliberately
narrow: the Unix server does not yet derive PacketPlan from the frozen evidence or dispatch `seal`,
and no real D-Bus/systemd rehearsal has run.

Artifact immutability then became an executable invariant rather than a prose promise. On Unix the
staging tree is converted to `0400` files and `0500` directories, synced, atomically renamed and
immediately re-opened by the replay verifier. A later mode change is evidence drift even when bytes
still hash correctly. Privileged-tamper tests change permissions explicitly, alter the artifact and
confirm fail-closed replay. At source `9b4d93a7`, all 125 ordinary WSL supervisor tests and six
independent packet-verifier tests pass. This proves the local mode/hash boundary, not production
uid/gid or mount enforcement; those still belong to admitted-host rehearsal.

## The 0.75 dispatcher and the cost of a clock read

HydraCache 0.75 revisited the same rule when it moved the local cache behind an asynchronous
dispatcher shared by HC/1, HC/2, and RESP. On the first paired screen against the published
0.74 source, static dispatch lost 7.07% of direct-native `GET` goodput. The boxed future lost
9.91% and increased allocated bytes per `GET` by 38.87%. Both variants still returned the
expected responses. The timing and allocation checks exposed a cost that conformance tests
could not see.

The static path began returning its existing response envelope and skipped audit copies for
unisolated reads and ordinary writes. Local cache hits became one tree lookup. RESP passed the
translated request and reply bytes by ownership. On the same Linux host, the follow-up screen
put direct-native `GET` at +22.35% goodput, RESP pipeline-10 `GET` at +0.85%, and RESP
pipeline-10 `SET` at -1.19%. All three passed the frozen goodput, CPU, p99, and allocation
limits. These figures describe local release builds and RESP over loopback TCP.

Direct-native `PUT` still needed its own decision. We ran fifteen same-binary A/A pairs and
fifteen baseline/candidate pairs, with three million measured writes per process. The goodput
estimate was -1.89%, with a 95% interval of -2.39% to -1.25%. The p99 estimate was +1.69%,
with an interval of +0.85% to +3.39%. The point estimates sat inside the -2% and +3% budgets.
Both intervals crossed a limit, so the D1 gate stayed open.

The next code-level hypothesis came from tracing a plain write. The dispatcher had already
sampled time for expiry maintenance. The write path sampled again even when there was no TTL
or idempotency key to timestamp. The focused change keeps the maintenance read and takes the
second sample only for writes that need it. A test recorded one clock read for a plain write
and two when a TTL or idempotency key was present. It also checked the stored bytes, expiry,
mutation event, and replay outcome.

The first AX42 comparison at source `89ec8970c` put direct-native `PUT` goodput at +5.87%.
One candidate process spent 5.21 ms waiting to run, above the 5 ms scheduler threshold. We
retained the entire block and repeated all its pairs. In the new fifteen A/A and fifteen A/B
pairs, `PUT` passed the four limits. Goodput was +5.20% (95% interval +4.19% to +5.69%), CPU/op
was -4.93%, and p99 was -5.08% (95% interval -5.88% to -3.47%).
The full D1 screen stayed open on that same source.
For raw embedded `GET`, the B74-versus-B74 p99 interval ran from -3.91% to +1.73%,
beyond its ±3% noise budget. The same-binary control also exceeded the goodput and CPU/op
noise budgets for typed embedded `PUT`.

We then tested CPU placement with the same B74 executable on both sides and the same
arguments for every attempt. On the original two cores, the worker migrated and the cores
received NIC interrupts. On a single isolated core we saw no worker migrations or NIC
interrupts, yet the main thread and the Tokio worker accumulated up to
15.93 ms in the run queue. Pinning those threads to different isolated physical cores brought
the maximum down to 3.19 ms for raw `GET` and 1.95 ms for typed `PUT` in the short pilot.

A separate diagnostic build recorded the latency histogram every 500,000 operations. One
raw `GET` process on an isolated core moved from a 2.10 µs p99 in the first window to about
2.19 µs in the remaining windows. The share of requests above 2.15 µs moved from 0.42% to
roughly 1.28%, crossing the 1% boundary that determines p99. The diagnostic histograms show
why a longer measurement and a fresh same-binary control matter even after CPU placement
improves.

B74's Moka version can run read-log maintenance on the lookup path at a threshold of 64
queued reads. One diagnostic run spent about 1.6% of its calls above 2.15 µs, close to one in
64. That makes the maintenance path a specific hypothesis for operation-level profiling.

The confirmation used twenty fresh A/A pairs per cell with the original release executable.
Every attempt cleared the scheduler limit. The typed `PUT` control passed all four bands.
The raw `GET` p99 interval ran from -1.39% to +3.04%, just above the frozen +3% limit.
That calibration outcome kept D1 open and stopped the new campaign before A/B.

We kept the release executable, placement, warmup and operation count per attempt, then
preregistered forty fresh raw `GET` pairs. The A/A p99 interval narrowed to -2.15% to +0.50%.
The typed `PUT` control cleared its bands as well. Both focus workloads therefore admitted
the candidate comparison on that profile.

The full matrix cleared the raw embedded `GET` p99 bound. Its candidate interval ran from
-2.30% to +0.12%. Nine cells passed all four limits. The remaining cell was raw embedded `PUT`.
Its goodput estimate was -1.90%, with an interval of -3.06% to -1.37% against a -2% floor.
CPU per operation was +1.93%, with an interval of +1.39% to +3.19% against a +3% ceiling. The
new placement and extra pairs resolved the read calibration. The write intervals kept D1 open.

The raw `PUT` follow-up measured roughly the same instruction count per operation on the two
binaries. On the split-core profile, the median pair showed about 1.5% more cycles per operation
for the candidate, at nearly the same effective frequency. The preregistered confirmation kept
that profile and the original binaries. In forty-eight fresh A/A and forty-eight A/B pairs,
goodput was -1.46% with an interval of -1.88% to -1.08%. CPU per operation was +1.48% with an
interval of +1.11% to +1.93%. p99 and allocation also cleared their limits. The nine matching
cells in the previous matrix and this confirmation together admitted static dispatch for D1.

## The first D2 change gives a map entry a binary identity

D1 put HC/1, HC/2, and RESP behind one dispatcher and gave the local backend ownership of
values, expiry maintenance, and retained idempotency outcomes. That boundary gives the next
backend a single place to establish which entry a request addresses.

D2 starts with the key codec. The Rust runtime now owns the provisional binary framing that
the test kit used. Each frame carries length-delimited tenant and namespace names, a namespace
generation, and the original key bytes. For example, tenant `ab` with namespace `c` and tenant
`a` with namespace `bc` stay distinct even when the key bytes match. A namespace recreated at a
new generation also produces a different frame for the same key.

The decoder reads borrowed slices while it checks lengths, UTF-8 names, generation, and the end
of the frame. It copies the fields only after those checks and avoids encoding the decoded key
a second time. Rust and the independent Java codec compare their output with the existing golden
bytes. The next decisions are to register the production format and hash, then bind reads to a
quorum-confirmed owner and applied prefix. The local D1 measurements cover the dispatcher and its
local backend. The distributed read path will have its own correctness and performance gates.

## Where a distributed read may look up the key

The next D2 change captures the epoch, member generations, and Raft progress under the same lock
that applies membership changes. With two separate reads, a restart could leave a caller with the
epoch from one state and the member's generation from another. Placement declines to route until
local apply catches up to the commit index. If applying a Raft Ready batch fails, the runtime withholds
the placement view until recovery.

The staged GET path issues a fresh Raft ReadIndex for every invocation. When the partition owner
is a Raft follower, the leader confirms the read with a quorum. The follower matches the response
to its own request and waits until it has applied the confirmed index. It then holds the value lane
and the Raft apply lock through lookup. A promotion that reaches Raft before that check invalidates
the old owner's proof. The new owner also needs the required value prefix before it can read.

We sent the requests over HC/1 HTTP and HC/2 gRPC with mTLS to that backend. Three Raft runtimes
ran behind a controllable message transport. Cutting off the follower that owned the partition
left its value intact, but the request expired with zero lookups. Delaying the Raft append let
the quorum response arrive ahead of the follower's applied index. The same invocation reached
the value once the append was delivered. We also moved ownership after quorum confirmation and
checked that the former owner never reached the store.

The next run replaced the instrumented lane with preinstalled durable replicas. Three separate
member processes on AX42 served RESP and exchanged Raft messages over HTTP. The binary value
remained on disk when the owner's Raft traffic was cut. RESP returned an authority error, and
the record lookup counter stayed flat. Healing the cut restored the read. After killing and
restarting the owner, a new topology epoch alone was insufficient. The restarted process refused
the previous replica image and an image one entry short of the required prefix. It served the
binary value again only after the full prefix was installed. The three AX42 runs recorded zero
store lookups in every rejected case. That read test used preinstalled replicas.

## A write becomes readable after the replica applies it

W10c started the members with empty value journals and wrote the value through RESP. The owner
flushed the pending record, and the required backup returned a typed acknowledgement after its
own flush. The acknowledgement included the replica generation and a digest of the mutation.
The owner then committed a certificate to Raft and returned success once the certificate was
applied locally. GET uses that certificate to decide which local record can be read.

On AX42, we blocked only the value message to the required backup. SET returned an error.
Raft remained reachable, and GET continued to return the value at prefix 1. A kill and restart
of the owner left the certified value readable from its disk. After replica delivery resumed,
SET advanced the prefix to 2 and an invalidation advanced it to 3. This was the staged
synchronous profile with same-disk recovery. Repair of a missing replica and the cost of the
extra network round trip and flushes remain separate release gates.

## A deadline needs both a clock bound and a replica

W10d records the TTL deadline in the mutation the owner sends to the backup. The backup stores
that deadline unchanged. The owner uses the upper bound on time to assign the deadline and waits
for the lower bound to reach it before proposing a tombstone. The expiry worker checks the
original entry version again, so a delayed timer cannot erase a replacement. Conditional writes
also inspect the certified value on the owner lane. In the concurrent put-if-absent test,
exactly one request stored a value.

On AX42, controlled clock feeds placed a read on each side of the deadline. We cut delivery to
the required backup while the expiry tombstone was due. The owner refused GET, and the Raft
prefix stayed at 1. After we restarted the owner and restored clock and replica authority, the
tombstone advanced the prefix to 2. A rewind of the test clock could no longer make the value
readable. These were deterministic clock observations. The production time provider still
requires OD-2 qualification.

The hash experiment used three sets of 100,000 keys over 271 partitions. The same FNV-1a function
produced 365–373 entries per partition for binary integers and 321–428 for prefixed decimal keys.
That difference matters when choosing a placement policy. A Rust/Java golden vector also covers
unsigned remainder when the high bit of the hash is set.

## A backup can read before it can accept new writes

W10e changes the owner of one partition through a Raft decision. The candidate first proves
that its durable journal covers the exact committed prefix. Raft compares that prefix again
when it applies the decision and advances the epoch. A delayed certificate from the previous
epoch cannot authorize a new value. Reads on the former owner still need their own safe
ReadIndex, including when its client port remains reachable.

The new owner can read and replay a certified outcome as soon as its journal and the applied
Raft view agree. To accept a new write it must also get a prefix proof from each configured
synchronous backup. That distinction matters to retries. A certified compare-and-set can
return its original result and version. If the previous owner may have applied a request
locally and its outcome was never certified, the client gets an explicit unknown outcome.
Evaluating the condition again could change which request wins.

On AX42, three separate processes ran the same scenario four times. The client left the
reply to a certified CAS unread. Then we cut delivery of a second CAS to the backup after
the old owner had flushed it, and isolated the old owner from the quorum while its client
port stayed open. The old owner refused the read with zero record lookups. The promoted
backup returned version 2 for the certified retry and reported the second CAS outcome as
unknown. A fresh CAS reached local apply only after the configured backup could prove its
prefix again. Restarting that backup under a different generation blocked further writes
while the promoted owner continued to serve the certified value.

## Replacing a backup while the old node is down

After the W10e cutover, a partition could keep serving reads while its only synchronous
backup was unavailable. W10f lets the current owner copy the certified image to a third member.
The copy includes the TTL and tombstone state and the outcomes needed for a caller to retry
an earlier request. The receiver flushes each chunk with a cursor and checks the full image
before it can be installed. It still needs a Raft decision to join the synchronous replica set.

On AX42 we stopped the original owner and dropped the second of two chunks on the way to
the third member. The promoted owner still served GET. A fresh write was refused before
local apply. Once the link was restored, the transfer resumed at the saved cursor and Raft
added the new backup. A new CAS advanced the certified version from 2 to 3 exactly once.
We then promoted the repaired replica and retried the original CAS there. It returned its
stored result at version 2 while GET returned the value at version 3. The original member
stayed stopped throughout all four runs.

## The owner keeps writing while the backup catches up

A fixed snapshot covers the first part of a live repair. W10f now saves later mutations
in the same durable batch as the owner apply. Writes still go through the current backup.
The replacement resumes from its saved cursor and applies the versions in order. The source
checks the chain against a fresh Raft certificate, so a locally flushed write with an unknown
outcome never becomes part of the handoff without certification.

The owner pauses its writer for a bounded final gap after the replacement has caught up.
The replacement must have the same final certificate in its Raft view and its journal.
Only then does Raft switch the backup and advance the epoch.

On AX42, we interrupted a snapshot at version 2 and resumed it on the third member. The
owner committed 15 CAS operations after the snapshot, including 12 while delta transfer
was in progress. Raft switched the backup at version 17. We stopped the previous backup
and committed version 18 with the replacement. After promoting the replacement, it
returned the original outcome for each of the 15 retries and served the value at version
18. The single-partition scenario passed four times.

## Moving backups for several partitions under write load

The owner can now decide where to send the next backup. It ranks the available members by
their certified backup assignments and asks the selected process to confirm its generation,
scope, and applied epoch. A periodic worker also checks the assigned backup directly. Two
misses for the same process incarnation can start a repair when discovery has yet to report
the failure.

Each replacement still needs the final prefix proof and a Raft decision. The next transfer
uses the new epoch and updated assignment counts. That allows a drain to spread several
partitions across available members while the current owners keep accepting writes.

On AX42, four member processes exercised a planned drain and a real process stop. During
the drain, the owner accepted SET requests for all three moving partitions and sent each
destination the changes after its snapshot. The planner split their backups two to one
across the two available members. We then stopped the former backup. The periodic worker
repaired a fourth partition while writes continued to the first three. After recovery,
CAS and its retry returned the expected results on all four partitions. We repeated the
same scenario four times on one binary.

## Moving primaries across owners

Changing one partition's assignment advances a shared epoch, so two uncoordinated
handoffs can interrupt each other's copy. Raft now holds an ordered list of primary
moves. The owner of the first partition does the work and the next task becomes eligible
when the first handoff commits.

A new destination first receives the snapshot and the later certified changes. An
existing synchronous backup can prove the final prefix directly. At the handoff the
current owner briefly holds the writer, and the new owner must prove the exact
certificate before Raft changes the assignment. The previous owner becomes a backup.
Cancelling the wave advances the epoch and fences a copy still in flight.

On AX42 we queued two primaries on different members and blocked the first destination.
Both original owners continued to accept writes. The second move stayed queued even
though its destination already held the partition as a backup. Once the first copy
caught up, the handoffs committed in order. The two new owners started accepting writes
and returned the recorded outcomes of earlier CAS requests. We then cancelled another
wave with a snapshot chunk in flight. The current owner kept the value it had just
written. The four-process scenario passed four times with the same binary.

## Choosing a new primary

For a planned drain, the planner counts certified primaries already assigned to each
healthy candidate. It prefers the least loaded destination, breaks a tie in favour of
an existing synchronous copy, and adjusts the counts as it fills the wave. The Raft
epoch binds the choice to the placement view used to make it. The node that starts
the wave can also be a destination if its attached generation matches that view.

After an RF=2 owner failure, its only synchronous backup can take over if its durable
journal matches the exact prefix certified by Raft. The backup checks that prefix again
at a fresh ReadIndex, then Raft commits the new assignment. Reads can resume at the new
owner. Fresh writes need a certified synchronous copy, which the repair worker rebuilds
on a surviving member. The old owner's process generation stays fenced when it returns.

On AX42, three member processes moved two primaries during a planned drain. We
killed one of the new owners while RESP writes continued on the other partition.
The surviving backup promoted on the original certificate, and the third member
took the new synchronous slot. The new owner returned the recorded result of the
CAS acknowledged before the kill and accepted a fresh CAS. The restarted member
refused both GET and SET for the partition it had lost. Four runs of the same
optimized binary produced this result. The six-hour production campaign through
packaged HC/2 clients and the 24-hour confirmation are still pending.
[Release plan](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#w10f-implement-bounded-rebalance-and-replica-repair)

## Putting the replicated path behind HC/2

A public HC/2 request can now enter through any member in the explicit RF=2 profile.
The accepting member derives the partition from the admitted tenant, namespace and
binary key. If the primary is elsewhere, the receiver forwards the request once.
The owner checks the original principal and the current placement before it uses
ReadIndex or applies a mutation. The daemon attaches the durable value journal at
startup and enables automatic recovery.

A continuous diagnostic keeps the same three-member cluster through foreground traffic,
an owner kill, recovery, restart and readback. It records a separate ledger of
acknowledged control-partition writes and the readbacks that verify them. A confirmation
uses the same code and binary digests as its six-hour parent run. Production admission
also needs clock-qualified expiry, cluster-wide listener and bulk behavior, and a
larger campaign through both packaged SDKs.

The 30-second release diagnostic on AX42 verified 386 acknowledged mutations by
readback and recorded three outcomes as unknown. The promoted owner replayed the
CAS that had been acknowledged before the kill and accepted a fresh write. A
read through the restarted member confirmed that the key was absent after deletion. The production
six-hour and 24-hour campaigns remain pending.
[Release plan](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#w10i-build-the-correctness-and-compatibility-safety-net)

## Partial batches and cluster listeners

One HC/2 batch can reach several owners. HydraCache validates the whole request and
sends one bounded request to each remote owner. Each owner validates its group against
current placement and executes its items in order. The public response restores the
original order and carries each item's partition and applied version when the owner
can prove it. If an owner disappears after applying some items, losing its reply leaves
those outcomes unknown. Replies from surviving owners remain available. Retrying the
original batch under the same idempotency key can recover a certified result with its
original version, including after promotion.

A subscriber can attach to any member. The receiving process polls the current
owners and advances separate cursors for their partitions. An owner publishes a change
only when Raft has certified the version. A duplicate request can return an earlier
outcome without publishing another event. When ownership changes, the subscriber gets
a gap before further delivery. The frame carries a reason and the affected partitions
when the server can identify them. A reconnect that supplies only the old stream
watermark requires stream-wide repair. Overwritten journal history identifies the
partition whose continuity was lost.

On AX42, the test waited for a certified event from the first mutation in a 64-write
owner group, then killed that process before its reply. A surviving owner answered
its part of the same request. The replay through the new owner recovered the original
certificate. In a separate test, blocking event polls across 24 writes produced a
gap for one partition. Running the packaged Java facade against three mTLS endpoints
exercised both partial results and listener recovery. All three release repetitions
passed these scenarios. After closing the subscriptions, all three daemons reported
zero active registrations.
[W10g release evidence](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#w10g-make-bulk-operations-and-listeners-cluster-wide)

## What it takes to drain a replica

A Raft quorum can be healthy while a value partition is missing its backup. The two checks
answer different questions. HydraCache takes a bounded observation of the applied topology and
the reachable value replicas for `/readyz`. The management endpoint adds paged evidence about
which certified partitions are behind, moving or under-replicated. Prometheus groups partitions
by state. It also records how long successful replica acknowledgements take and how often they
fail.

A drain closes admission before it changes membership. A backup may already have flushed and
acknowledged a write while the owner is still waiting to certify it through Raft. The departing
backup waits until the prefix is committed or a newer epoch fences the old acknowledgement.
Raft then coordinates the transfer of primaries and backups. The process can leave once all
certified data placements have surviving replicas and the local subscriptions have closed. If a
spare is unavailable and the drain runs out of time, the node stays in the roster with admission
sealed and the admin endpoint up for a retry.

On AX42, three release repetitions exercised the drain with separate member processes and an
active writer. Each repetition preserved the transferred values and the original bulk mutation
receipt, and left zero live subscriptions on the survivors. In a separate failure scenario,
killing the spare made the drain time out while the departing member stayed in the roster. The
retry completed after the spare returned, and both seeded values were still readable.

The cluster management view preserves the source of each observation. Every member reports its
own view of the same 271 partitions. Adding the counts would count many partitions twice. The
collector displays an opaque member identifier, shows missing peers explicitly, and includes time
spent collecting or caching an observation in its reported age. An expired observation loses its
partition counts and readiness claim.

The published 0.74.0 daemon could still collect the original Raft management snapshot after
replacing one member on the same volume before the first certified IMap write. In that mixed
cluster, the new management view reported an incompatible peer and HC/2 refused writes. The
test restored the 0.75 binary and verified a new write through a different endpoint. All three
optimized repetitions passed this sequence.
[W10h release plan](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#w10h-add-distributed-observability-admission-readiness-and-operational-controls)

## Where rollback stops being safe

An IMap write leaves a certificate in Raft as well as data on its replicas. A member can have an
empty local value journal and still carry Raft commands that the old binary cannot decode.
Compaction moves those commands from the log into the recovery snapshot. A rollback check has
to inspect both representations, plus any pending state in the local value journal.

The offline preflight copies the locked stores to scratch before opening them. Recovery can then
inspect the copies while leaving the stopped member's original files intact.

On AX42, the published 0.74.0 daemon refused a cold copy containing the certificate in the log and
refused another copy after compaction moved it into the snapshot. The preflight detected both
cases. Starting 0.75 on the original volume again preserved the value, the deletion and the
original idempotent batch receipt.
[W10i rollback preflight](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#offline-rollback-preflight-for-b74)

## Measuring traffic during a live drain

The W10i diagnostic keeps one workload running across an owner kill and a later planned drain.
It seeds a separate partition on the departing member. After drain, both survivors report the
same certified version at the new placement epoch and return the retained value through HC/2.
The departing member rejoins from its retained volume.

The controller checks both timestamps for every workload round. A round counts toward drain
progress only when its writes and readbacks started and finished during the drain. It also
requires verified progress after the third member returns. On AX42, both 60-second runs passed.
The ledgers verified 37 and 42 mutations during drain and 333 and 339 after return. The reports
kept unknown outcomes in a separate count.
[W10i qualification plan](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#w10i-build-the-correctness-and-compatibility-safety-net)

## Proving that a copy resumed

A transfer can retry and still hide a full restart. The W10i diagnostic drops the second
snapshot chunk after the destination has acknowledged the first. Three member observations
check that the old primary and backup still own the certified version while the copy is paused.
The controller counts traffic on a separate control partition during that pause.

The worker then lets the second chunk through and blocks any attempt to send the first again.
The drain must finish under that rule. It verifies the new primary and backup at a later epoch
and reads the complete value through the survivors and the returning member. On AX42, the two
60-second runs each verified 15 mutations on the control partition while the copy was blocked.
[Interrupted copy diagnostic](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#interrupted-copy-before-primary-cutover)

## Comparing the replicas at cutover

An HC/2 readback checks the client path. W10i also asks each replica for a digest of its local
journal at the same Raft-certified prefix. Each member checks the epoch and its role before
and after computing the digest. The image includes values, tombstones, expiry deadlines and
retained mutation decisions. A certified write gives the same image before and after the journal
materializes it.

The worker compares the primary and backup before drain, then compares the two survivors after
cutover. The controller checks all four digests, the full prefix and the member identities in the
receipt. This makes the diagnostic sensitive to a divergent backup even if the HC/2 readback
succeeds. On AX42, the four digests and counts matched in each of three 60-second runs, including
two with an interrupted snapshot copy. The production campaign still needs these checks at each
fault checkpoint across the mixed workload.
[W10i replica reconciliation](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#replica-journal-reconciliation-at-cutover)

## Following the certified prefix after a process kill

The owner-kill diagnostic compares the primary and backup before the fault, then compares the
promoted backup and its new synchronous replica after an idempotent retry. The same certified
prefix and image must survive the change of owners. A fresh conditional write advances the prefix
while the old owner is still offline. The two active replicas must preserve that version when the
old owner rejoins with a new generation. A final conditional removal advances the prefix again,
and the two journals must agree on the tombstone while the returned HC/2 endpoint reports absence.
On AX42, all five replica comparisons passed in each of the three diagnostic profiles. Across the
profiles, 1,892 control-partition acknowledgements were verified by readback.
[W10i owner-kill reconciliation](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#replica-journal-reconciliation-after-owner-kill)

## Reading the other SDK's writes after failover

The W10i retained cohort gives Rust and Java separate keys and has each SDK read what the other
wrote before the owner is killed, while it is offline, and after it returns. Java writes through
the packaged facade and checks each bulk receipt. At the last checkpoint both SDKs read all 13
retained keys through the returning member. A second run deliberately stores the wrong Rust
value and requires the Java probe to stop before it marks the phase complete.
Both checks passed on Java 17 and 21 on AX42. The separate Rust control stream verified 96 and
84 mutations, respectively, with each complete round contained inside the owner outage.
[W10i cross-SDK cohort](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#cross-sdk-retained-cohort-through-an-owner-outage)

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
