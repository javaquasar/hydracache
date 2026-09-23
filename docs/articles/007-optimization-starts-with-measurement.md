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

That remains a hypothesis until an isolating experiment separates:

- atomic counter updates;
- retained-byte estimation;
- listener registration and notification delivery;
- tag-index cleanup;
- background eviction completion.

The next experiment should change one of these factors at a time while keeping source, binary,
scenario, order, and instrumentation outputs otherwise equivalent.

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
