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

That boundary now has a producer as well as a consumer. The measured process cannot announce an
inventory until its checkpoint chain is terminal. It must provide exactly the guard IDs frozen in
the campaign manifest, and each guard must point to durable role-owned source evidence. The writer
emits a canonical guard-result document, includes the document and its sources in the exact raw
set, checks the frozen limits, and makes the inventory visible last. Its create-new/link protocol
can recover the two local crash windows without overwriting conflicting bytes; the published file
is accepted only after the temporary link is removed and the final inode has a single link. A
complete result containing a failed guard and any cross-role evidence are rejected before the
inventory appears. The positive test then sends those exact producer bytes through the independent
supervisor resolver, closing a subtle gap where a strict consumer existed but no equally strict
way to generate its input did.

The same principle applies to stopping a bad run. The local abort transaction now records an
`ABORT_REQUESTED` state before invoking any external effect and retains the exact process identity
needed for recovery. An idempotent backend boundary is then responsible for diagnostics and stop.
Only after it succeeds does `ABORT_COMPLETED` clear process/checkpoint/lease state, persist the
signed response and release the host claim. If the backend fails after the intent commit, the
caller sees the advanced revision and can resend the identical signed request; a different request
cannot inherit that intent. Tests prove that exact replay does not repeat the backend effect.

The production adapter then narrows the external effect itself. It accepts only the exact
campaign-bound 0.74 unit, verifies retained process identity through D-Bus, and atomically publishes
a bounded canonical snapshot of frozen state and allowlisted unit properties. Only then does it
call `StopUnit`, accepting success after removal or `inactive/dead` with no MainPID. Recovery from
an already absent unit requires the exact diagnostic bytes; a conflicting pre-existing file fails
closed. This completes the code path, but not its host proof: no live unit was stopped during local
development, and system-bus permissions and timing still need the admitted-host rehearsal.

Product-lease expiry now uses the same ordering without relying on a surviving controller. A
bounded listener wake-up checks only the host-locked active campaign. Once the immutable deadline
is past, the supervisor records `LEASE_EXPIRY_REQUESTED` with the original process and checkpoint
identity still present, writes a diagnostic bound to the campaign, lease id and deadline, and asks
systemd to stop the exact retained unit. It records `LEASE_EXPIRY_COMPLETED` and releases the host
claim only after that effect succeeds. The automatic retry path therefore cannot manufacture a
replacement process or forget which unit it was terminating.

This implementation also exposed a useful test-harness lesson. The first full parallel suite found
that some older server fixtures reused a wall-clock second captured before the server wrote its own
event. Crossing a second boundary made a later fixture event look time-reversed. The fix was not to
relax journal monotonicity; fixtures now continue from the durable event clock. Five parallel
repetitions and the complete 147-test local WSL suite then passed. That is strong local recovery
evidence, but still not proof of real system-bus authorization or stop latency on the admitted host.

The same separation applies to controller authorization. The private signing key is consumed only
by a protected GitHub-hosted job. A narrow Rust builder signs the exact operation, campaign,
manifest, principals and five-minute window, verifies its own packet, and publishes it create-new.
The self-hosted performance machine receives the signed packet, not the key, sends exactly one
typed operation, and then takes a best-effort read-only status snapshot. The workflow is
manual-only, host-serialized and has cancellation disabled. It has been structurally tested but
not dispatched; candidate identity, admitted host and expensive qualification remain intentionally
unresolved.

The next gap looked deceptively like a file-copy step. It was not: production staging is owned by
the performance account with mode `0750`, while the workflow runner intentionally has neither
write permission, `sudo` nor arbitrary DBus access. Giving the job permission to copy files would
have erased the security boundary the supervisor was built to provide. Instead, the local builder
now creates a transport artifact before that boundary. It binds the start manifest to the admitted
host receipt, inventories the exact four nested files by digest and size, publishes all six files
create-new, and re-hashes them after transport. Drift in machine, boot, mount or cpuset identity,
tampering, hardlinks and overwrite attempts all fail in 10/10 Windows and 10/10 WSL tests at
source `fd51e8a6`.

That work deliberately stops one step short of production staging. The contract still marks the
privileged staging adapter incomplete. Closing it requires a supervisor-owned, typed delivery path
and admitted-host ownership rehearsal; it does not justify broadening the GitHub runner's
permissions. This is another instance of the release's main theme: completing the safe artifact is
useful progress, but it is not evidence that the live campaign path has run.

The typed delivery path came next. A revision-zero client now sends the signed request and the two
bounded evidence documents as separate seqpackets; the service authenticates the peer and signature
before accepting either document. The root supervisor repeats the canonical manifest/receipt checks,
writes `0400` files into a `0750` create-new directory, syncs it and publishes it by rename before
the existing start transaction can reach spawn. Each receive is bounded to five seconds, an exact
interrupted upload is recoverable, and tampered bytes fail before either staging or backend start.
The workflow can download only an explicitly named same-repository run artifact, checks its fixed
six-file inventory against the dispatch campaign and manifest, and uses the narrow `request-start`
command. It still gets no signing key, filesystem write access, `sudo`, shell injection field or
DBus capability.

At source `e1882dfb`, 153 ordinary supervisor tests pass under WSL with one manual system-bus test
ignored; the final focused server and transport suites pass 9/9 and 8/8. This closes the local code
path, not the host proof. The workflow was not dispatched, no service was installed, and the
admitted-host uid/gid, socket and mount behavior remains a release-blocking rehearsal.

Multipart transport added one more failure window: the controller can disappear after the signed
request and manifest but before the host receipt. That disconnect now terminates only the accepted
connection, not the supervisor loop. The partial request publishes no staging directory and calls
no spawn backend; a complete request on the same server instance then succeeds exactly once. At
source `af0bff6d`, the focused server suite is 10/10 and the complete WSL count is 154 ordinary
passes plus the one intentionally ignored real-system-bus test. Internal mutation and lease
maintenance failures remain fatal, so “survive the client” does not become “ignore corruption.”

The next local slice closed a subtler time-of-check gap. A correctly transported receipt proves
what the controller admitted, but it does not prove that CPU partitioning, kernel tuning, boot,
mount or installed supervisor identity still match when the role is about to start. New I74 and
C74 requests now recollect the host observation inside the privileged service and compare the full
receipt before creating the host claim, durable state or spawn intent. The ordering matters:
completed request IDs are replayed from the verified event journal first, so a later host change
cannot turn a successful retry into a new decision or repeat the process launch. Injected drift
fails closed with no claim, state or backend call; convergence permits one launch; a retry returns
the byte-identical signed response without another observation. At source `c7cfbc9f`, the focused
server suite is 11/11 and the complete WSL suite is 155 ordinary passes plus the intentionally
ignored real-system-bus test. This validates local composition, not the still-missing privileged
host rehearsal.

A controller-resilient run also needs a negative answer to a harder question: who notices a hang
when the controller is gone? The phase-aware watchdog already distinguished useful work from a
heartbeat, but that distinction previously lived only inside the writer. The checkpoint envelope
now hashes the observation time and the last useful-progress time into every record. The detached
supervisor reads and verifies that chain itself, so a new useful checkpoint during controller loss
extends the deadline while a measured telemetry-only checkpoint does not. Attach also refreshes its
checkpoint snapshot from this evidence instead of depending on an older controller snapshot.

After the frozen 180-second nonterminal gap, the supervisor first records a checkpoint-bound
failure intent, then captures bounded diagnostics and stops the exact retained unit, and only then
records completion and releases the host claim. An interrupted backend call retries the same cause;
it cannot launch a replacement. A terminal checkpoint is deliberately exempt because controller
loss after successful work must allow later sealing. At source `8141a90f`, 160 ordinary supervisor
tests pass in WSL plus one ignored real-system-bus test; the focused server/progress suites are
12/12 and 3/3, the integrated writer is 10/10 and the offline verifier is 7/7. The first-checkpoint
startup deadline and real root/systemd rehearsal remain open, so this is not yet a host-proof claim.

The first-checkpoint gap could not be closed with a controller heartbeat: that would make a dead
product look healthy whenever its observer remained alive. The accepted anchor is instead the
hash-verified `Started` or `Adopted` lifecycle timestamp already owned by the detached supervisor.
If no first record appears within the same frozen 180-second gap, the supervisor commits a distinct
startup progress-loss cause, captures diagnostics, stops the retained unit and releases the host
claim only after durable completion. A missing journal and an empty journal are handled as the same
pre-publication state; malformed evidence remains an error, and a state that once named a checkpoint
cannot fall back to the startup path.

The cause includes the campaign, lifecycle timestamp, gap and exact deadline. After a crash, the
timestamp is recovered from the durable event identity and its digest is verified before the stop
effect can resume. Diagnostic publication is keyed by that cause digest, so the no-checkpoint path
does not weaken create-new or exact-replay rules. At source `8f15f3c2`, 162 ordinary supervisor
tests pass in WSL plus one ignored real-system-bus test; the focused server/progress suites are
13/13 and 4/4, strict clippy is clean, and the no-execution qualification dry-run accepts the updated
contract. This closes the local startup liveness hole, not the real root/systemd host rehearsal and
not any product-throughput qualification claim.

Fail-closed recovery also needs proof that the rejected shortcuts stay rejected. The W11 dynamic
canary now activates all seven forbidden weak policies from the plan: PID-only liveness,
restart-from-checkpoint, PID reuse, sequence-only checkpoint hashes, mixed-attempt identity,
ignored lease expiry and ignored duplicate executors. For each case the test demonstrates that the
weak predicate would accept the defect and that the production predicate rejects it; only then does
the canary fail with its expected marker. On clean source `4b5f9812`, the ordinary state-machine
suite is green and the canary sweep records one expected-red proof. That is useful local
falsification, not a substitute for killing real processes and controllers on the admitted host.

That distinction exposed another false-positive class: a dead or replaced measured process can
also stop producing checkpoints, but calling it a progress stall loses the owner of the failure.
The detached supervisor now checks measurement identity first. It distinguishes frozen-host drift,
an absent systemd unit, unit-identity drift and `/proc` or cpuset drift from an otherwise healthy
role whose useful-work deadline expired. A retained successful terminal unit remains sealable; it
is not converted into failure merely because no more checkpoints arrive.

Measurement loss has its own canonical cause and two durable lifecycle records. The request record
precedes diagnostic/stop, so a service restart recovers the exact reason, timestamp, processes and
checkpoint rather than sampling a new story. Completion clears retained execution identity and
releases the host claim. The lease-expiry loop will not steal a previously committed failure and
relabel it. The stop rule is intentionally asymmetric: only a process-identity loss with the same
unit, cgroup and MainPID may stop the unit. Missing, cross-boot, replaced or otherwise ambiguous
identity stays failed closed for an operator instead of risking a foreign process.

At source `9e7409d0`, 169 ordinary supervisor tests pass under WSL plus the intentionally ignored
real-system-bus test. Four measurement-loss lifecycle tests cover normal completion, interruption,
exact recovery and future-observation rejection; server tests cover healthy/terminal separation and
lease-overlap recovery. The read-only monitor accepts cleared completed-failure state, and the local
qualification dry-run executes nothing. This is resilience attribution, not performance evidence:
real root/systemd timing and the admitted host still have to prove the production path.

Finally, packet and archive publication use create-new staging directories, sync their contents,
apply read-only Unix modes and atomically rename the completed trees. Recovery can adopt an exact
completed artifact or finish a verified rename; it cannot overwrite a conflicting artifact. These
mechanics do not make the local throughput numbers promotable, but they make it possible for a
future admitted-host run to survive controller loss without quietly changing the experiment.
The signed socket `seal` path now connects those pieces: it revalidates frozen host evidence,
observes the retained systemd unit, derives the inventory-bound packet plan and drives the durable
seal transaction. A local injected observation boundary tests failure-after-terminal and exact
retry without pretending that it proves the production D-Bus or uid/gid setup.

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

No real transient unit or HydraCache process was launched for that result. At that source the
admitted-host account/directory permissions, controller and supervisor restart,
controller-loss reattachment, overhead budget, seal/abort/diagnostics and lease-expiry
termination remained unproved or unfinished; later local slices close the code paths but not the
host proof. The point is the same as for the performance candidates: implemented mechanics and
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

## A capable host is not a provisioned measurement host

The same distinction applies to infrastructure. A read-only probe found a healthy systemd PID 1,
the system manager bus, unified cgroup v2 with CPU, cpuset, memory, I/O and PID controllers, and an
ext4 filesystem for the planned persistent state. None of that made the runner ready for the W11
controller-loss rehearsal.

The dedicated supervisor user and client group did not exist. Neither did the reviewed service,
binary, production config, state directories or Unix socket, and the Actions runner had no client
group membership. The probe changed nothing and started no qualification process.

This is not a generic “CI is unavailable” excuse. It identifies one exact operational transition:
protected provisioning must install the reviewed artifacts, bind the real verification key, create
the least-privilege identities and admit the runner to the socket group. Only then can a rehearsal
test real D-Bus timing, root-owned modes, detached unit identity and reattachment after controller
loss. A machine with systemd is capability evidence; it is not admission evidence.

Provisioning later made that transition concrete, and the failures on the way were part of the
result. The first protected run stopped before bundle construction because the signing secret had
not been configured. The next derived and transported only the public key but stopped because the
runner had no general passwordless sudo. We did not solve that by granting a shell. A one-command,
root-owned entrypoint now accepts only a closed provisioning bundle whose SHA-256 manifest is signed
by the protected Ed25519 key. The installed supervisor binary verifies that signature and the
already-installed public trust root before any replacement. The private seed never reaches the
self-hosted machine.

The first post-install probe then found a genuine permission bug. A `0660` group-owned Unix socket
was still unreachable through a `0750 root:root` runtime directory. Changing only the directory to
`0711` preserved non-listability while allowing group-authorized traversal. The same probe had also
treated `permission-denied` on protected campaign directories as if those directories were missing;
that was corrected without weakening their modes. A later rerun exposed an artifact identity race,
and the final collection exposed that a world-readable receipt below a `0750` parent is not actually
readable by the runner. Both were retained as negative results and fixed at their exact boundary.

The final run `37322301775` is green. The service is active, the runner has the client GID, the socket
is `0660 root:hydracache-perf-client`, the signed bundle is idempotent, and the capability verdict is
`host_rehearsal_ready=true`. This still is not performance evidence: no product workload or measured
transient unit ran. It authorizes the next bounded fixture rehearsal; it does not authorize a release
claim or silently turn on six-hour and 24-hour qualification.

## The allocator hypothesis: faster churn, larger memory footprint

W9e finally moved the allocator discussion from a deferred idea to a dedicated-Linux measurement.
The run did not reuse the 0.73 Windows receipts: it built new system, mimalloc and jemalloc binaries
from source `707abde4`, differing only by the mutually exclusive allocator feature. A
counterbalanced schedule produced five no-purge repeats for each allocator and five separate
mimalloc purge repeats. All 20 attempts passed the exact seed, key, payload, phase, binary and host
identity guards.

Mimalloc found a real CPU owner. Relative to the system allocator, median fill CPU/op fell 51.26%,
delete CPU/op 41.54% and refill CPU/op 8.55%; each direction held in all five repeats. That still did
not make it an acceptable release change. Whole-trace elapsed time, including the fixed two-second
idle phase, improved only 3.08%, while post-idle PSS rose 14.24%, RSS rose 14.04% and executable
size rose 9.92%. The isolated CPU owners clear the owner floor, but the unchanged memory and binary
guards still forbid admission. The hypothesis reduced allocator work by buying more resident
memory and a larger binary. It moved cost rather than removing it.

Jemalloc was a different version of the same lesson. Delete CPU/op fell 29.31% in every repeat, but
total elapsed changed by only -0.23%; refill CPU/op regressed 5.10% and executable size grew 34.12%.
Its post-idle PSS increase was smaller at 1.69%, but the unchanged guard evaluates the complete
trade, not the most favorable row.

Provider-native counters were retained without pretending their names were interchangeable.
Glibc `mallinfo2`, mimalloc JSON and jemalloc `mallctl` expose different definitions of allocated,
committed, resident and retained memory. Missing native resident or live counters remained missing;
RSS/PSS was reported separately and was never inserted as a substitute. That distinction matters:
a superficially comparable "retained" number can mean free heap bytes, virtual reservation or
retained allocator mappings depending on the provider.

The reviewed APIs also did not expose a comparable scalar thread-cache count. That missing field
would block acceptance even if the other rows passed; it does not block rejection after independent
memory, binary-size and CPU guards have already failed.

Explicit mimalloc purge did not rescue the proposal. Five of five runs recorded three purge calls
and 655,360 provider-reported purged bytes, but process PSS/RSS fell by only 339,968 bytes, roughly
0.22% of pre-purge PSS. The second refill remained correct, yet the mechanical benefit was too small
to justify a policy candidate.

So the allocation hypotheses were tested when they had a dedicated Linux owner profiler, and they
were tested in two stages: first provider-native ownership plus process memory/CPU, then—only if the
frozen owner and regression floors passed—an isolated product candidate followed by the unchanged
RESP and native matrices. Neither allocator reached the second stage. The system allocator remains
the default, and no 6-hour or 24-hour qualification slot is spent on a candidate that already moved
cost across a frozen guard.

## The syscall boundary confirmed an owner without reopening its rejected fix

The remaining response-path uncertainty was below Tokio. We knew that pipeline 10 accepted ten
requests together while the server still issued one `AsyncWrite` and one flush per reply, but local
Windows policy denied kernel network tracing. It was still possible that Tokio or the kernel merged
those writes, or that short writes, EAGAIN and socket queues pointed at a different platform-level
owner.

The dedicated Linux host could not expose tracefs or even software `perf` counters, but it could
trace a child with `strace` and inspect sockets with `ss`. We added a measurement gate after preload
and warmup, then ran GET and SET at pipeline 1/concurrency 1, pipeline 10/concurrency 1, and pipeline
10/concurrency 8. Five counterbalanced repeats yielded 30 attempts.

The first campaign failed for a useful reason. The validator assumed that a profiler with a durable
output path would keep stdout empty. The profiler deliberately writes the same JSON to both. All 30
attempts were retained and classified invalid. The corrected validator parses both copies and
requires exact equality instead of deleting the inconvenient campaign.

That correction exposed a second measurement trap. A process stopped at every traced syscall has
an observer-created scheduling pattern: the median traced/untraced context-switch ratio was about
6,121. Treating traced `getrusage` as scheduler evidence would have produced a precise but false
story. The final contract therefore uses two independent processes per attempt. The untraced
companion supplies measurement-window context switches, page faults and PID-owned socket queues;
the traced process supplies syscall and endpoint counts. Workload digests and deterministic
application counters must match across them. All 60 processes and 30 pairs passed.

The result closes the kernel question. Pipeline 10 reduced client writes and server reads from
roughly 1.0 to 0.1 per operation, but server writes remained exactly 1.0 per operation in every
cell. All request and response bytes reconciled. Across the entire matrix, server read/write EAGAIN,
client write EAGAIN, application pending writes and short writes were zero. The largest median
socket send/receive queue high-water was only 1,300 bytes.

This confirms a real write-syscall owner, but it does not authorize a candidate. The earlier bounded
batch already reduced response writes to 0.1 per operation and won strongly in deep pipelines; it
was rejected by pipeline-1 CPU/tail guards. Its adaptive successor failed the frozen backpressure
semantics before measurement. The syscall trace supplies no Nagle, buffer-size, scheduler or runtime
knob that avoids those failures, and task-level wakeup timing remains unavailable.

W9c therefore closes as measured-no-new-candidate. Platform defaults stay unchanged. Reopening W3
requires a semantically different shallow-free batching design, not a reinterpretation of the same
one-write-per-reply evidence.

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
