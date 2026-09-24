# Optimization Starts With Measurement: Reading Allocations, RSS, and Runtime Together

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

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

Optimization work often starts with a profiler screenshot and a promising line of code.

That is usually too late.

Before changing the implementation, we need to decide what kind of cost we are trying to remove.
A feature can leave total runtime unchanged while increasing allocation churn. It can reduce average
latency while increasing resident memory. It can make one isolated operation cheaper by transferring
work to cleanup, eviction, or a background task.

These are different outcomes. A single benchmark number cannot distinguish them.

While preparing HydraCache 0.73, we ran a small local experiment to measure the cost of production
memory instrumentation. The experiment did not produce a release claim. It did something more useful
at that stage: it found a concrete blocker before we paid for long dedicated-host runs.

This article uses that experiment to explain:

- how to compare an instrumented path with its uninstrumented control;
- what gross allocation and resident set size (RSS) measurements actually mean;
- why a faster elapsed time does not automatically clear an optimization;
- how local screening can eliminate bad candidates before expensive qualification;
- how to turn an observed regression into a testable ownership hypothesis.

## The experiment: same binary, two modes

The comparison used one optimized release binary in two runtime modes:

- `off`: memory-footprint instrumentation did not update its production counters;
- `production`: bounded counters and the associated eviction accounting were enabled.

Each pair launched two independent processes. Three pairs were executed with alternating order:

```text
pair 1: off        -> production
pair 2: production -> off
pair 3: off        -> production
```

Alternating the order does not eliminate thermal drift, background activity, or cache effects, but
it prevents one mode from always receiving the colder or quieter position. The run also bound the
exact source commit, binary digest, host fingerprint, seed, attempt order, receipts, and raw resource
series.

The phrase “release pair” here means that the binary was compiled with the optimized release profile.
It does **not** mean that three local pairs are release-grade performance evidence.

## What we observed

The table shows medians across the three local pairs. Regression is calculated as:

```text
(production - off) / off
```

| Metric | Instrumentation off | Production instrumentation | Change |
| --- | ---: | ---: | ---: |
| Total elapsed time | 35.64 ms | 34.46 ms | -3.3% |
| Fill allocation per operation | 2,425 B | 3,075 B | +26.8% |
| Steady-read allocation per operation | 315 B | 315 B | 0.0% |
| Expire/delete allocation per operation | 2,267 B | 3,139 B | +38.5% |
| Refill allocation per operation | 1,121 B | 1,300 B | +15.9% |
| Post-idle RSS growth from cold | 512 KiB | 700 KiB | +36.7% |
| Peak RSS growth from cold | 648 KiB | 796 KiB | +22.8% |

The tempting headline would be that production instrumentation was 3.3% faster.

That would be the wrong conclusion.

Three short local runs cannot distinguish a small speed improvement from scheduler noise, timer
resolution, process startup variance, or background activity. The responsible interpretation is:

> This screen found no elapsed-time regression large enough to see locally.

The allocation result is different. Reads were unchanged, while mutation phases repeatedly showed
large directional deltas. That pattern is more useful than the aggregate elapsed number because it
identifies where additional work enters the system.

## Gross allocation is not retained memory

“Allocated 3,139 bytes per delete” does not mean that every delete permanently increases memory by
3,139 bytes.

Gross allocation counts the sizes of successful allocation requests made while the operation runs.
If the process allocates 1 KiB and frees it immediately, the full 1 KiB is still counted. A
reallocation can also count the complete new allocation size rather than only the capacity delta.

Gross allocation therefore measures **allocator traffic**, not live memory.

It is useful because allocator traffic can imply:

- additional allocator and synchronization work;
- more temporary objects on a hot path;
- increased cache and memory-bandwidth pressure;
- higher sensitivity to concurrency and allocator choice;
- future RSS growth when freed blocks remain in allocator arenas.

It cannot tell us, by itself, how much memory remains live after the operation. For that we need live
object accounting, allocator active/resident metrics, and process-level memory observations.

## RSS is not the same as owned data

RSS is the amount of a process currently resident in physical memory. It includes more than the
cache entries we intended to measure:

- application heaps and allocator arenas;
- thread stacks;
- executable and shared-library pages;
- runtime and networking structures;
- file-backed mappings;
- measurement machinery itself.

Absolute RSS is especially difficult to compare across independent short-lived processes. In this
screen we used two relative values inside each process:

- post-idle RSS minus the cold-process RSS;
- peak RSS minus the cold-process RSS.

That subtraction removes part of the process floor, but it does not transform RSS into an ownership
measurement. The result is still a local diagnostic. On a qualification host we would also separate
anonymous and file-backed pages, allocator active/resident/retained bytes, cgroup charges, and final
live cardinality.

## Why the combined reading matters

The experiment produced four distinct signals:

1. Total elapsed time did not expose a regression.
2. Read allocation was unchanged.
3. Mutation allocation increased materially.
4. Post-idle and peak RSS growth moved in the same direction as mutation allocation.

Together, these signals suggest a mutation-specific instrumentation cost. They do not prove its
owner, but they narrow the search much more effectively than an overall throughput number.

Source inspection supplied the next hypothesis. Production memory instrumentation enables an
asynchronous eviction listener used for eviction accounting and tag-index cleanup. The listener is
absent when instrumentation is off. Its lifecycle and queued asynchronous work are plausible owners
for additional allocations during fill, expiry, deletion, reset, and refill.

## The isolation experiment we actually built

We did not immediately rewrite the listener or raise the budget. First, we added a deliberately
narrow development-only seam that could remove exactly one factor from the experiment: listener
registration.

The normal public builder and server configuration were left unchanged. The seam was enabled only
in the unpublished load generator and produced a new diagnostic profile. In that profile:

- production atomic counters and retained-byte estimation stayed enabled;
- the async eviction listener was not attached to the underlying cache;
- the same eight workload phases and resource probes were used;
- control and treatment still came from one optimized binary;
- subprocess order remained counterbalanced;
- every receipt declared `diagnostic_only: true` and
  `counter_correctness_eligible: false`.

That last marker matters. Without the listener, automatic eviction and removal accounting is
incomplete. The treatment is therefore not a candidate implementation. It is an ablation: an
intentionally incomplete configuration used to answer one ownership question.

We built the release binary once, captured a source- and host-bound local context, and ran three new
independent `off`/`production-counters-without-listener` pairs. Across those pairs, the median
changes versus instrumentation-off were:

| Metric | Counters without listener: change |
| --- | ---: |
| Fill allocation per operation | 0.0% |
| Steady-read allocation per operation | 0.0% |
| Expire/delete allocation per operation | 0.0% |
| Refill allocation per operation | +3.3% |
| Post-idle RSS growth from cold | +1.4% |
| Peak RSS growth from cold | -1.8% |

The elapsed median moved +7.4%, but one production sample was much faster than the other two. Three
short local pairs cannot turn that distribution into a timing claim. We recorded the elapsed result
as noisy rather than choosing the two convenient samples or reporting an apparent regression.

Allocation and RSS tell the useful story. Removing listener registration eliminated the large fill
and expire/delete allocation deltas and nearly eliminated the RSS deltas. A small refill residual
remained, so we cannot claim that every byte of production-instrumentation cost belongs to the
listener. We can say that the listener path owns the large effect that blocked the baseline.

### The fill phase was the strongest clue

The most informative result was not the delete phase. It was fill.

The fill phase inserted 128 small entries into an empty cache whose capacity was far larger than the
test dataset. It performed no explicit deletes and should not have needed capacity eviction. Yet the
original production mode allocated 26.8% more bytes per operation during fill. When we retained the
counters but did not register the listener, that delta fell to zero.

This changes the hypothesis. The cost cannot be explained only as useful work performed after an
entry is removed. Listener registration changes the backend's mutation machinery even on a phase
that does not expect removal callbacks. Source inspection supports that interpretation: the future
cache routes eviction notifications through listener/notifier infrastructure and represents the
callback as a boxed future. Our callback also clones shared state and awaits tag-index cleanup when
it is invoked.

The first ablation did not yet split the cost among:

- enabling the backend removal-notification machinery;
- constructing and scheduling boxed listener futures;
- cloning the counter and tag-index handles;
- acquiring the tag-index lock and deleting memberships;
- computing and subtracting the retained-byte estimate;
- draining background maintenance before a phase ends.

We therefore added a second ablation. This time the backend listener remained registered, but its
HydraCache callback was empty: no counter subtraction, no retained-byte calculation, no shared-state
clones in our closure, and no tag-index cleanup. The result still reproduced most of the original
cost:

| Metric | Registered no-op listener: change |
| --- | ---: |
| Fill allocation per operation | +23.9% |
| Steady-read allocation per operation | 0.0% |
| Expire/delete allocation per operation | +25.8% |
| Refill allocation per operation | +30.5% |
| Post-idle RSS growth from cold | +27.4% |
| Peak RSS growth from cold | +21.3% |

The refill result moved more than in the earlier screen, so three local pairs are not enough to
decompose that phase numerically across separate source identities. The broader pattern is clear:
an empty callback did not make listener-enabled mutations cheap. Fill remained close to the
original +26.8%, and the RSS deltas remained material.

This narrows the owner again. Most of the blocker belongs to enabling the future cache's removal
notification path, not to the business logic inside our callback. The callback still adds work,
especially when a removal actually occurs, but a callback-only rewrite cannot remove the fill cost
and is therefore not a sufficient production fix.

Together, the two ablations rule out a much broader and less useful explanation such as “atomic
counters are generally expensive.” Steady reads stayed unchanged in every experiment. Mutation
overhead disappeared when listener registration was removed while counters remained, then returned
when an empty listener was registered. That is a causal sequence, not merely a hot-looking source
line.

Source inspection then explained the shape of the result. In the Moka future cache used by
HydraCache, enabling a removal notifier also enables a per-key lock path. An insert attempts to
acquire that optional lock before the backend knows whether it is creating or replacing an entry.
Update and invalidation paths additionally protect listener delivery with shared boxed futures and
cancellation guards. This is useful for immediate, ordered notification semantics, but it means an
empty callback is not a free callback.

We also checked the next Moka patch release rather than assuming an upgrade would solve the problem.
Its public future-cache API still exposes synchronous and asynchronous eviction listeners, but no
nonblocking post-removal observer. An opportunistic dependency bump therefore cannot be presented as
the fix; an upstream seam, a reviewed backend change, or a different exact ownership design would
each be a separate proposal.

We then tested the cheapest-looking architectural shortcut: whether Moka's synchronous cache made
the notification machinery cheap enough to justify a backend migration. This was deliberately a
small allocation probe, not a HydraCache benchmark. It compared future and sync caches with the
listener disabled and with a no-op listener, used 1,024 inserts or removals per case, repeated every
case three times, and alternated `off/noop` order to reduce first-position bias.

| Backend | Operation | Listener off, B/op | No-op listener, B/op | Increase |
| --- | --- | ---: | ---: | ---: |
| Moka future | Insert | 398.4 | 756.7 | +89.9% |
| Moka future | Remove | 2,281.3 | 3,041.1 | +33.3% |
| Moka sync | Insert | 372.7 | 644.0 | +72.8% |
| Moka sync | Remove | 2,246.7 | 2,502.7 | +11.4% |

The sync backend reduced the listener's incremental allocation by 24.3% on insert and 66.3% on
remove. That is useful attribution, but not a solution. The no-op listener still added 271 bytes per
insert and a 72.8% relative penalty. More importantly, replacing the future cache with the sync
cache changes the backend and its asynchronous interaction model; the microprobe did not exercise
HydraCache correctness, concurrency, expiry, tag cleanup, or production load.

So the negative result saved a much more expensive experiment. We rejected “migrate to sync” as
the instrumentation fix before building a product candidate or reserving a qualification host. The
remaining design space is narrower: an exact nonblocking removal-observation seam or a replacement
ownership design that avoids enabling listener-backed mutation locking. Either still needs review,
pre-frozen allocation/RSS limits, and the full removal-correctness matrix before D2.

### Turning the result into an implementable observer

The chosen direction is a separate post-removal observer, not a faster implementation of the same
async listener. The distinction matters. The observer must run only after a logical removal wins,
must not return a future, acquire the listener-enabled per-key lock, allocate a boxed future, or
spawn one task per removal. Its synchronous work is limited to atomic accounting and publication
into a preallocated bounded cleanup channel.

Removing the await point creates an ordering problem that the old listener previously hid. Suppose
entry version 41 is removed, version 42 is inserted under the same key, and cleanup for 41 runs
late. A key-only cleanup could delete version 42's tag membership. The replacement design therefore
gives every entry an immutable version and stores that version with each tag membership. Deferred
cleanup becomes `unregister_if_version(key, removed_version)`: a late notification can clean its own
state but cannot mutate its successor.

Duplicate accounting is also tied to the entry rather than to an ever-growing event set. Clones of
one entry share a single atomic removal-accounted flag. The first observer call owns the decrement;
repeated delivery becomes a no-op. This keeps duplicate detection bounded by live notification
state instead of accumulating identifiers for the lifetime of the process.

The cleanup channel is bounded and never fails open. Saturation or closure marks the current
observer epoch dirty. Lightweight snapshots may disclose that state, while an exact snapshot must
either wait for every accepted cleanup, rebuild from an authoritative quiescent entry list, or
return an error. It must never label incomplete counters or tag ownership as exact. Reconciliation
also drains stale queued work before rebuilding, so an old ticket cannot mutate the rebuilt index.

We first implemented these rules as a development-only reference model rather than altering the
production cache. Its falsifiers cover delayed old-version cleanup, duplicate delivery, queue
saturation, pending cleanup at an exact barrier, cancellation, and explicit, replacement, expiry,
and capacity-removal causes. This separates proof of the lifecycle contract from the later Moka API
spike and ensures that a low allocation number cannot excuse incorrect cleanup.

### The result changed governance, not just code direction

At this point the responsible next action was not to start editing the production cache. We recorded
the owner as D1-classified and explicitly left D2 unauthorized. The proposal registry now prevents
candidate measurements and product mutation until four things exist:

- a lab-only feasibility result for the viable backend designs (the sync shortcut is now rejected,
  while the exact nonblocking designs remain to be evaluated);
- an independent reviewer for the selected proposal;
- allocation and RSS rejection limits frozen before candidate data;
- exact correctness tests for every automatic and explicit removal path.

We also froze the parts of the measurement method that were already inherited and defensible: at
least five independently started admitted pairs, a 95% interval, Hodges-Lehmann paired estimates,
moving-block bootstrap, Holm correction across primary claims, and the existing 2% goodput and 3%
CPU/p99 regression guards. Allocation and RSS limits remained named blockers instead of being
chosen from the numbers we had just observed.

Finally, we created an unadmitted host-profile template. It describes the immutable and mutable host
probes, serialized lease, CPU/NUMA/power policies, and pre/post calibration required for later
qualification. It does not contain a convenient local-machine fingerprint and cannot authorize a
release claim. This separation lets local work continue without allowing local evidence to promote
itself.

### Why the ablation must not become the fix

It would be easy to stop here and ship production counters without the listener. That would make the
benchmark green by deleting required work.

The listener currently observes removals that are not all initiated by the public `remove` method:
capacity eviction, expiry, replacement, invalidation, and backend maintenance can all affect live
ownership. It also participates in tag-index cleanup. A cheaper design is acceptable only if it
continues to account for every removal path and releases every secondary owner.

The production replacement therefore needs deterministic proofs for at least:

- explicit remove, overwrite, invalidate, flush, TTL expiry, and capacity eviction;
- exact counter reconciliation at a quiescent barrier;
- complete tag-membership cleanup and bounded generation state;
- concurrent mutation snapshots being marked non-atomic rather than presented as exact;
- cancellation and shutdown draining all acknowledged work;
- unchanged public configuration, capacity semantics, and default behavior.

Only after those tests pass should the full production profile be rerun locally. If the large
allocation and RSS deltas remain gone, the candidate earns an expensive dedicated-host comparison.
If they return, the design goes back to local attribution instead of consuming a qualification run.

### What this step taught us about profiling

An ablation does not have to be a valid product configuration to be a valid diagnostic. It must,
however, state exactly which guarantees it breaks. Here, disabling the listener answered an
ownership question while the receipt explicitly prohibited counter-correctness and release claims.

The sequence was more valuable than a profiler screenshot alone:

1. Measure the complete production behavior against an off control.
2. Find the phases where the cost appears.
3. Form an owner hypothesis from phase behavior and source inspection.
4. Remove one factor without pretending the result is shippable.
5. Re-run the same process-level comparison and retain the negative evidence.
6. Turn the result into correctness constraints for the real redesign.

This is how cheap local work protects expensive performance work. The first screen stopped us from
freezing an instrumented baseline with a hidden cost. The second screen stopped us from optimizing
the atomic counters that were not the main owner. Neither screen produced a release number, but both
removed a large amount of uncertainty before dedicated-host qualification.

## Do not move the threshold after seeing the result

The observed allocation regressions were larger than expected. One easy response would be to set the
allowed allocation overhead above 38.5% and declare the instrumentation acceptable.

That would turn a measurement into a justification.

A release threshold must be chosen before candidate data is observed. It should represent the
largest practical cost the product is willing to accept, not the smallest value that makes the
current implementation pass.

For this experiment, throughput, CPU per request, and p99 latency already had inherited regression
ceilings. Allocation and RSS limits had intentionally remained unmeasured blockers. After seeing the
screen, they remained blockers. The result was recorded as negative, non-promotable evidence rather
than used to manufacture a permissive budget.

This distinction is central to evidence-driven optimization:

```text
measurement answers “what happened?”
policy answers “what cost is acceptable?”
```

The measurement cannot be allowed to rewrite the policy that judges it.

## A profiling ladder that avoids expensive runs

Not every development iteration needs a dedicated bare-metal campaign. A useful workflow has several
levels.

### 1. Deterministic unit and model tests

Use these to prove semantics, accounting identities, bounds, and failure behavior. They are fast and
should run on every relevant change. They do not prove performance.

Examples include:

- every attempted operation is classified as success, rejection, timeout, late, or incomplete;
- a retained-byte estimate equals the sum of its checked components;
- an instrumentation-off snapshot cannot masquerade as an available production snapshot;
- a failed attempt remains in the evidence ledger.

### 2. In-process allocation probes

A counting allocator can attribute gross allocation to a narrowly bounded future or operation. This
is useful for finding churn without relying on RSS. Run the process quiescently: unrelated tasks in
the same process can otherwise enter the allocation count.

### 3. Local counterbalanced process screening

Build the binary once, then launch independent control and treatment processes in alternating order.
Bind every attempt to exact hashes and retain raw output. This level can reject gross regressions and
validate the evidence pipeline.

It cannot establish portable capacity or a release-grade numerical improvement.

### 4. Sampling and tracing for ownership

After a metric moves, use stack sampling, allocation profiling, tracing, or subsystem counters to
find its owner. Profiling builds must remain distinct from production-mode comparison builds: a
profiler can alter scheduling, allocation, code generation, and memory layout.

### 5. Dedicated-host paired qualification

Only candidates that survive the cheaper levels need the expensive run. Use a stable admitted host,
prebuilt binaries, fixed offered load, identical traces, complete outcome accounting, pre/post host
calibration, and at least the preregistered number of independently started pairs.

### 6. Integrated and long-running confirmation

An isolated optimization may transfer cost to another subsystem. Re-run affected mixed workloads,
compatibility, rollback, and boundedness tests on the exact integrated candidate. A candidate-only
long run can show boundedness, but a comparative slope or plateau claim needs an equal-duration
baseline.

## What local screening should save

A useful local run produces more than terminal output. Retain at least:

- source commit and dirty-tree state;
- binary and scenario hashes;
- privacy-safe host and toolchain fingerprint;
- run-order seed and actual order;
- one directory per attempt;
- stdout and stderr, including failed attempts;
- raw allocation and memory series;
- complete operation outcome counts;
- a machine-readable aggregate explicitly marked non-promotable.

The output directory should be append-only. If a run fails halfway through, the correct result is a
failed attempt with its partial artifacts—not a silent retry that replaces history.

## A practical review checklist

Before accepting an optimization or an instrumentation change, ask:

1. Is the control the exact behavior we intend to compare against?
2. Are control and treatment doing equal logical work?
3. Did they use the same binary, scenario, trace, cardinality, and host?
4. Was process order counterbalanced?
5. Are failures, timeouts, rejections, late completions, and incomplete operations visible?
6. Does a lower latency result hide lower goodput or dropped work?
7. Are allocation, live ownership, allocator state, and RSS reported as separate concepts?
8. Was profile-mode evidence kept separate from production-mode evidence?
9. Were thresholds frozen before candidate observations?
10. Can the result reject the candidate, or is the harness designed only to produce green output?

The last question is the most important. A performance test that cannot say “no” is a demo.

## The lesson

Optimization is not the act of making one number smaller.

It is the process of locating a cost, assigning it to an owner, changing that owner without moving
the cost somewhere worse, and proving the result under a method chosen before the answer was known.

In this case, a small local screen prevented us from treating production instrumentation as free.
It also prevented us from wasting a dedicated-host campaign on an unresolved baseline. The screen
did not tell us what threshold to publish or what optimization to ship. It told us exactly what to
investigate next.

That is what a good profiler and a good benchmark should do.
